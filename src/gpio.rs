//! General Purpose Input/Output (GPIO)
use std::marker::PhantomData;

/// Extension trait to split a GPIO peripheral into independent pins and registers
pub trait GpioExt {
    /// Splits the GPIO block into independent pins and registers
    fn split(self) -> Parts;
}

/*
// todo: english
    在GPIO模式下，可以设置内部上下拉，以类型状态机模式设计
    SPI、UART、I2C等数字功能下，可以设置内部上下拉，但不会影响返回类型的状态
    ADC、DAC下，软件禁止设置内部上下拉。HAL库不会生成此类函数，以免出错。
*/

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

/// Open drain input or output (type state)
pub struct OpenDrain;

/// Push pull output (type state)
pub struct PushPull;

/// Output mode (type state)
pub struct Output<MODE> {
    _mode: PhantomData<MODE>,
}

/// Alternate function (type state)
pub struct Alternate<MODE> {
    _mode: PhantomData<MODE>,
}

/// Alternate function 1 (type state)
pub struct AF1;
/// Alternate function 2 (type state)
pub struct AF2;
/// Alternate function 4 (type state)
pub struct AF4;
/// Alternate function 6 (type state)
pub struct AF6;
/// Alternate function 7 (type state)
pub struct AF7;
/// Alternate function 8 (type state)
pub struct AF8;
/// Alternate function 9 (type state)
pub struct AF9;
/// Alternate function 10 (type state)
pub struct AF10;
/// Alternate function 11 (type state)
pub struct AF11;
/// Alternate function 14 (type state)
pub struct AF14;

/// Digital function that supports pull-up or pull-down
pub trait Digital {}

impl Digital for Alternate<AF1> {}
impl Digital for Alternate<AF2> {}
impl Digital for Alternate<AF4> {}
impl Digital for Alternate<AF6> {}
impl Digital for Alternate<AF7> {}
impl Digital for Alternate<AF8> {}
impl Digital for Alternate<AF9> {}
// no AF10; AF10 is Analog
impl Digital for Alternate<AF11> {}
impl Digital for Alternate<AF14> {}

// There are Pin0 to Pin22, totally 23 pins

pub use self::pin::*;

/// Gpio parts
pub struct Parts {
    pub pin0: Pin0<Input<Floating>>,
}

/// Gpio pins
pub mod pin {
    use std::marker::PhantomData;
    use super::*;

    /// Pin
    pub struct Pin0<MODE> {
        _mode: PhantomData<MODE>,
    }

    impl<MODE> Pin0<MODE> {
        /// Configures the pin to operate as a push-pull output pin.
        pub fn into_push_pull_output(self) -> Pin0<Output<PushPull>> {
            todo!()
        }
        /// Configures the pin to operate as an open-drain output pin.
        pub fn into_open_drain_output(self) -> Pin0<Output<OpenDrain>> {
            todo!()
        }
        /// Configures the pin to operate as a floating input pin.
        pub fn into_floating_input(self) -> Pin0<Input<Floating>> {
            todo!()
        }
        /// Configures the pin to operate as a pull-up input pin.
        pub fn into_pull_up_input(self) -> Pin0<Input<PullUp>> {
            todo!()
        }
        /// Configures the pin to operate as a pull-down input pin.
        pub fn into_pull_down_input(self) -> Pin0<Input<PullDown>> {
            todo!()
        }
    }

    impl<MODE> Pin0<MODE> {
        // todo: documents
        pub fn into_af1(self) -> Pin0<Alternate<AF1>> {
            todo!()
        }
        pub fn into_af2(self) -> Pin0<Alternate<AF2>> {
            todo!()
        }
        pub fn into_af4(self) -> Pin0<Alternate<AF4>> {
            todo!()
        }
        pub fn into_af6(self) -> Pin0<Alternate<AF6>> {
            todo!()
        }
        pub fn into_af7(self) -> Pin0<Alternate<AF7>> {
            todo!()
        }
        pub fn into_af8(self) -> Pin0<Alternate<AF8>> {
            todo!()
        }
        pub fn into_af9(self) -> Pin0<Alternate<AF9>> {
            todo!()
        }
        pub fn into_af10(self) -> Pin0<Alternate<AF10>> {
            todo!()
        }
        pub fn into_af11(self) -> Pin0<Alternate<AF11>> {
            todo!()
        }
        pub fn into_af14(self) -> Pin0<Alternate<AF14>> {
            todo!()
        }
    }
    impl<MODE: Digital> Pin0<Alternate<MODE>> {
        // 虽然有这些内部上下拉函数，内部上下拉很弱，大约44K，还是建议片外上拉
        // todo: english
        pub fn set_pull_up(&mut self) {
            todo!()
        }
        pub fn set_pull_down(&mut self) {
            todo!()
        }
        pub fn set_floating(&mut self) {
            todo!()
        }
    }
}

// todo: generate macros
