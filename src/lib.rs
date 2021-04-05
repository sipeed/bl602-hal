#![no_std]

pub use bl602_pac as pac;

pub mod checksum;
pub mod clock;
pub mod delay;
pub mod gpio;
pub mod i2c;
pub mod serial;
pub mod spi;

/// HAL crate prelude
pub mod prelude {
    pub use crate::gpio::GlbExt as _bl602_hal_gpio_GlbExt;
    pub use embedded_hal::prelude::*;
    pub use embedded_time::rate::Extensions;
}
