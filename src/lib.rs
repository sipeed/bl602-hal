//! # HAL for the BL602 microcontroller
//!
//! This is an implementation of the [`embedded-hal`] traits for the BL602 microcontroller.
//!
//! [`embedded-hal`]: https://crates.io/crates/embedded-hal
//!
//! # Usage
//!
//!
//! ## Commonly used setup
//!
//! ```rust
//! // Get access to the device specific peripherals from the peripheral access crate
//! let dp = pac::Peripherals::take().unwrap();
//! let mut parts = dp.GLB.split();
//!
//! // Freeze the configuration of all the clocks in the system and store the frozen frequencies in
//! // `clocks`
//! let clocks = Strict::new().freeze(&mut parts.clk_cfg);
//! ```
//!
//!
//! To avoid the linker to complain about missing symbols please add `hal_defaults.x` to `.cargo/config` like this
//! ```toml
//! rustflags = [
//!   "-C", "link-arg=-Tmemory.x",
//!   "-C", "link-arg=-Tlink.x",
//!   "-C", "link-arg=-Thal_defaults.x",
//! ]
//! ```
//!

#![no_std]

pub use bl602_pac as pac;

pub mod checksum;
pub mod clock;
pub mod delay;
pub mod gpio;
pub mod i2c;
pub mod interrupts;
pub mod rtc;
pub mod serial;
pub mod spi;
pub mod timer;

/// HAL crate prelude
pub mod prelude {
    pub use crate::gpio::GlbExt as _bl602_hal_gpio_GlbExt;
    pub use embedded_time::rate::Extensions;
}
