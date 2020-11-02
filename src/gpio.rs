//! General Purpose Input/Output (GPIO)
use std::marker::PhantomData;

/// Floating input (type state)
pub struct Floating;
/// Pulled down input (type state)
pub struct PullDown;
/// Pulled up input (type state)
pub struct PullUp;

/// Input mode (type state)
pub struct Input<MODE> {
    _mode: PhantomData<MODE>,
}

/// Output mode (type state)
pub struct Output<MODE> {
    _mode: PhantomData<MODE>,
}

/// Alternate mode (type state)
pub struct Alternate;

// todo Alternate and PullUp//PullDown?

pub trait GpioExt {
    fn split(self) -> Parts;
}

// There are Pin0 to Pin22, totally 23 pins

pub struct Parts {
    pub pin0: Pin0,
}

pub struct Pin0 {
    _ownership: PhantomData<()>,
}

