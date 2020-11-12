//! General Purpose Input/Output
use core::marker::PhantomData;
use crate::pac;

/// Extension trait to split GLB peripheral into independent pins, registers and other modules
pub trait GlbExt {
    /// Splits the register block into independent pins and modules
    fn split(self) -> Parts;
}

impl GlbExt for pac::GLB {
    fn split(self) -> Parts {
        Parts {
            pin5: Pin5 { _mode: PhantomData },
        }
    }
}

/*
// todo: english
    在GPIO模式下，可以设置内部上下拉，以类型状态机模式设计
    SPI、UART、I2C等数字功能下，可以设置内部上下拉，但不会影响返回类型的状态
    ADC、DAC下，软件禁止设置内部上下拉。HAL库不会生成此类函数，以免出错。
*/

/// Hi-Z Floating pin (type state)
pub struct Floating;
/// Pulled down pin (type state)
pub struct PullDown;
/// Pulled up pin (type state)
pub struct PullUp;

/// Input mode (type state)
pub struct Input<MODE> {
    _mode: PhantomData<MODE>,
}

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
// AF11 is SwGpio, ignore
/// Alternate function 14 (type state)
pub struct AF14;

/// Alternate function 10 (type state)
pub struct Analog;

// There are Pin0 to Pin22, totally 23 pins

pub use self::pin::*;

/// Gpio parts
pub struct Parts {
    pub pin5: Pin5<Input<Floating>>,
}

/// Gpio pins
pub mod pin {
    use core::marker::PhantomData;
    use core::convert::Infallible;
    use crate::pac;
    use super::*;
    use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin, toggleable};

    /// Pin
    pub struct Pin5<MODE> {
        pub(crate) _mode: PhantomData<MODE>,
    }

    impl<MODE> Pin5<MODE> {
        /// Configures the pin to operate as a Hi-Z floating output pin.
        pub fn into_floating_output(self) -> Pin5<Output<Floating>> {
            self.into_pin_with_mode(false, false, false)
        }
        /// Configures the pin to operate as a pull-up output pin.
        pub fn into_pull_up_output(self) -> Pin5<Output<PullUp>> {
            self.into_pin_with_mode(true, false, false)
        }
        /// Configures the pin to operate as a pull-down output pin.
        pub fn into_pull_down_output(self) -> Pin5<Output<PullDown>> {
            self.into_pin_with_mode(false, true, false)
        }
        /// Configures the pin to operate as a Hi-Z floating input pin.
        pub fn into_floating_input(self) -> Pin5<Input<Floating>> {
            self.into_pin_with_mode(false, false, true)
        }
        /// Configures the pin to operate as a pull-up input pin.
        pub fn into_pull_up_input(self) -> Pin5<Input<PullUp>> {
            self.into_pin_with_mode(true, false, true)
        }
        /// Configures the pin to operate as a pull-down input pin.
        pub fn into_pull_down_input(self) -> Pin5<Input<PullDown>> {
            self.into_pin_with_mode(false, true, true)
        }
        fn into_pin_with_mode<T>(&self, pu: bool, pd: bool, ie: bool) -> Pin5<T> {
            let glb = unsafe { &*pac::GLB::ptr() };
            glb.gpio_cfgctl2.write(|w| unsafe { w
                .reg_gpio_5_func_sel().bits(11) // GPIO_FUN_SWGPIO
                .reg_gpio_5_ie().bit(ie) // output
                .reg_gpio_5_pu().bit(pu)
                .reg_gpio_5_pd().bit(pd)
                .reg_gpio_5_drv().bits(0) // disabled
                .reg_gpio_5_smt().clear_bit()
            });
            Pin5 { _mode: PhantomData }
        }
    }

    impl<MODE> Pin5<Input<MODE>> {
        /// Enable smitter GPIO input filter
        pub fn enable_smitter(&mut self) {
            let glb = unsafe { &*pac::GLB::ptr() };
            glb.gpio_cfgctl2.modify(|_, w| w.reg_gpio_5_smt().set_bit());
        }
        /// Enable smitter GPIO output filter
        pub fn disable_smitter(&mut self) {
            let glb = unsafe { &*pac::GLB::ptr() };
            glb.gpio_cfgctl2.modify(|_, w| w.reg_gpio_5_smt().clear_bit());
        }
    }

    impl<MODE> Pin5<MODE> {
        // todo: documents
        pub fn into_af1(self) -> Pin5<Alternate<AF1>> {
            todo!()
        }
        pub fn into_af2(self) -> Pin5<Alternate<AF2>> {
            todo!()
        }
        pub fn into_af4(self) -> Pin5<Alternate<AF4>> {
            todo!()
        }
        pub fn into_af6(self) -> Pin5<Alternate<AF6>> {
            todo!()
        }
        pub fn into_af7(self) -> Pin5<Alternate<AF7>> {
            todo!()
        }
        pub fn into_af8(self) -> Pin5<Alternate<AF8>> {
            todo!()
        }
        pub fn into_af9(self) -> Pin5<Alternate<AF9>> {
            todo!()
        }
        pub fn into_analog(self) -> Pin5<Analog> {
            todo!()
        }
        pub fn into_af14(self) -> Pin5<Alternate<AF14>> {
            todo!()
        }
    }
    impl<MODE> Pin5<Alternate<MODE>> {
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

    impl<MODE> InputPin for Pin5<Input<MODE>> {
        type Error = Infallible;

        fn try_is_high(&self) -> Result<bool, Self::Error> {
            let glb = unsafe { &*pac::GLB::ptr() };
            Ok(glb.gpio_cfgctl30.read().reg_gpio_5_i().bit_is_set())
        }

        fn try_is_low(&self) -> Result<bool, Self::Error> {
            let glb = unsafe { &*pac::GLB::ptr() };
            Ok(glb.gpio_cfgctl30.read().reg_gpio_5_i().bit_is_clear())
        }
    }

    impl<MODE> OutputPin for Pin5<Output<MODE>> {
        type Error = Infallible;

        fn try_set_high(&mut self) -> Result<(), Self::Error> {
            let glb = unsafe { &*pac::GLB::ptr() };
            glb.gpio_cfgctl32.modify(|_, w| w.reg_gpio_5_o().set_bit());
            Ok(())
        }

        fn try_set_low(&mut self) -> Result<(), Self::Error> {
            let glb = unsafe { &*pac::GLB::ptr() };
            glb.gpio_cfgctl32.modify(|_, w| w.reg_gpio_5_o().clear_bit());
            Ok(())
        }
    }

    impl<MODE> StatefulOutputPin for Pin5<Output<MODE>> {
        fn try_is_set_high(&self) -> Result<bool, Self::Error> {
            let glb = unsafe { &*pac::GLB::ptr() };
            Ok(glb.gpio_cfgctl32.read().reg_gpio_5_o().bit_is_set())
        }

        fn try_is_set_low(&self) -> Result<bool, Self::Error> {
            let glb = unsafe { &*pac::GLB::ptr() };
            Ok(glb.gpio_cfgctl32.read().reg_gpio_5_o().bit_is_clear())
        }
    }

    impl<MODE> toggleable::Default for Pin5<Output<MODE>> {}
}

// todo: generate macros
