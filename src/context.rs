use crate::{
    params::{Key, HasParamStorage},
    Extra, Value, ValueIter,
};

use std::marker::PhantomData;

pub struct QuickContext<C, InputFn, ParamFn> {
    ctx: C,
    input_fn: InputFn,
    param_fn: ParamFn,
}

impl<InputFn> QuickContext<(), InputFn, ()> {
    pub fn input(input_fn: InputFn) -> Self {
        Self::new((), input_fn, ())
    }
}

impl<C, InputFn, ParamFn> QuickContext<C, InputFn, ParamFn> {
    pub fn new(ctx: C, input_fn: InputFn, param_fn: ParamFn) -> Self {
        QuickContext {
            ctx,
            input_fn,
            param_fn,
        }
    }
}

// TODO: Support MIDI inputs
impl<C, InputFn, ParamFn, Spec, I> GetInput<Spec> for QuickContext<C, InputFn, ParamFn>
where
    InputFn: Fn(&C, Spec) -> Option<I>,
    I: ValueIter + Send,
{
    type Iter = I;

    fn input(&self, spec: Spec) -> Option<Self::Iter> {
        (self.input_fn)(&self.ctx, spec)
    }
}

impl<C, Spec> GetInput<Spec> for &'_ C
where
    C: GetInput<Spec>,
{
    type Iter = C::Iter;

    fn input(&self, spec: Spec) -> Option<Self::Iter> {
        C::input(*self, spec)
    }
}

pub trait ContextMeta {
    /// Samples per second
    fn samples(&self) -> usize;
}

pub struct FileId<Kind> {
    index: usize,
    _marker: PhantomData<Kind>,
}

pub trait FileAccess<Kind> {
    type ReadFile;

    // Will always read the file from the start
    fn read(&self, id: FileId<Kind>) -> Option<Self::ReadFile>;
}

pub trait GetInput<Spec> {
    type Iter: ValueIter + Send;

    // `None` means that this input is not wired
    fn input(&self, spec: Spec) -> Option<Self::Iter>;
}

pub trait GetParam<Spec, T: Key> {
    fn param(&self) -> T::Value;
}

pub trait GetRuntimeParam<Spec> {
    fn param(&self, spec: Spec) -> !;
}
