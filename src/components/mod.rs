pub mod anycomponent;

pub use anycomponent::{AnyComponent, AnyInputSpec, AnyOutputSpec, AnyParamSpec, Types};

use crate::{
    params::{HasParamStorage, Key},
    Context, Value, ValueType,
};
use itertools::Either;
use nom_midi::MidiEventType;
use std::fmt;

// TODO: This can probably be `u8`
pub type SpecId = usize;

pub trait RefRuntimeSpecifier: fmt::Display {
    fn id(&self) -> SpecId;
    fn value_type(&self) -> ValueType;
}

pub trait RuntimeSpecifier: RefRuntimeSpecifier + Sized {
    fn from_id(id: SpecId) -> Self;
}

pub trait EnumerateValues: RuntimeSpecifier + 'static {
    type Iter: ExactSizeIterator<Item = &'static Self> + Clone;

    fn values() -> Self::Iter;
}

impl RefRuntimeSpecifier for ! {
    fn id(&self) -> SpecId {
        unreachable!()
    }

    fn value_type(&self) -> ValueType {
        unreachable!()
    }
}

impl RuntimeSpecifier for ! {
    fn from_id(_: SpecId) -> Self {
        unreachable!()
    }
}

impl EnumerateValues for ! {
    type Iter = std::iter::Empty<&'static !>;

    fn values() -> Self::Iter {
        std::iter::empty()
    }
}

pub trait Component: Clone + Sized {
    type InputSpecifier;
    type OutputSpecifier;
    type ParamSpecifier: HasParamStorage;

    fn update<Ctx>(&self, _ctx: &Ctx) -> Self
    where
        Ctx: Context<Self>;
}

pub trait GetOutput<Spec: Key>: Component {
    type Iter: ExactSizeIterator<Item = Spec::Value>;

    fn output<Ctx>(&self, ctx: &Ctx) -> Self::Iter
    where
        Ctx: Context<Self>;
}

// TODO: Support MIDI inputs

pub trait PossiblyIter<T>: Sized {
    type Iter: ExactSizeIterator<Item = T>;

    fn try_iter(self) -> Result<Self::Iter, Self>;
}

pub enum PossiblyEither<A, B> {
    Left(A),
    Right(B),
}

impl<A, B, T> PossiblyIter<T> for PossiblyEither<A, B>
where
    A: PossiblyIter<T>,
    B: PossiblyIter<T>,
{
    type Iter = Either<A::Iter, B::Iter>;

    fn try_iter(self) -> Result<Self::Iter, Self> {
        match self {
            Self::Left(val) => val
                .try_iter()
                .map(Either::Left)
                .map_err(PossiblyEither::Left),
            Self::Right(val) => val
                .try_iter()
                .map(Either::Right)
                .map_err(PossiblyEither::Right),
        }
    }
}

/// Implementation detail since Rust doesn't take associated types into account when checking whether
/// two implementations overlap.
pub trait ValueIterImplHelper<T> {
    type AnyIter: PossiblyIter<Value> + PossiblyIter<MidiEventType>;

    fn mk_valueiter(other: T) -> Self::AnyIter;
}

pub struct NoIter<T> {
    _noconstruct: !,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Iterator for NoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        unreachable!()
    }
}

impl<T> ExactSizeIterator for NoIter<T> {
    fn len(&self) -> usize {
        unreachable!()
    }
}

impl<T: ExactSizeIterator<Item = MidiEventType>> ValueIterImplHelper<T> for MidiEventType {
    type AnyIter = AnyIter<T, NoIter<Value>>;
    fn mk_valueiter(other: T) -> Self::AnyIter {
        AnyIter(AnyIterInner::Midi(other))
    }
}

impl<T: ExactSizeIterator<Item = Value>> ValueIterImplHelper<T> for Value {
    type AnyIter = AnyIter<NoIter<MidiEventType>, T>;

    fn mk_valueiter(other: T) -> Self::AnyIter {
        AnyIter(AnyIterInner::Analog(other))
    }
}

impl<A, B, V> From<V> for AnyIter<A, B>
where
    A: ExactSizeIterator<Item = MidiEventType>,
    B: ExactSizeIterator<Item = Value>,
    V: ExactSizeIterator,
    V::Item: ValueIterImplHelper<V, AnyIter = AnyIter<A, B>>,
{
    fn from(other: V) -> AnyIter<A, B> {
        V::Item::mk_valueiter(other)
    }
}

pub struct AnyIter<A, B>(AnyIterInner<A, B>);

impl<A> Default for AnyIter<A, NoIter<Value>>
where
    A: Default,
{
    fn default() -> Self {
        AnyIter(AnyIterInner::Midi(A::default()))
    }
}

impl<B> Default for AnyIter<NoIter<MidiEventType>, B>
where
    B: Default,
{
    fn default() -> Self {
        AnyIter(AnyIterInner::Analog(B::default()))
    }
}

enum AnyIterInner<A, B> {
    Midi(A),
    Analog(B),
}

impl<A, B> PossiblyIter<MidiEventType> for AnyIter<A, B>
where
    A: ExactSizeIterator<Item = MidiEventType>,
{
    type Iter = A;

    fn try_iter(self) -> Result<Self::Iter, Self> {
        match self.0 {
            AnyIterInner::Midi(inner) => Ok(inner),
            this @ AnyIterInner::Analog(_) => Err(AnyIter(this)),
        }
    }
}

impl<A, B> PossiblyIter<Value> for AnyIter<A, B>
where
    B: ExactSizeIterator<Item = Value>,
{
    type Iter = B;

    fn try_iter(self) -> Result<Self::Iter, Self> {
        match self.0 {
            AnyIterInner::Analog(inner) => Ok(inner),
            this @ AnyIterInner::Midi(_) => Err(AnyIter(this)),
        }
    }
}

#[cfg(test)]
mod test {
    // Statically assert that an enum with `NoIter` as one variant has no tag,
    // which makes `AnyIter` zero-cost.
    const _: () = {
        enum Foo {
            A,
            B(super::NoIter<()>),
        }

        [(); 0 - std::mem::size_of::<Foo>()];
    };
}
