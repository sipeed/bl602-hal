//! Delays

use embedded_hal::delay::DelayNs;
use embedded_hal_zero::blocking::delay::{DelayMs as DelayMsZero, DelayUs as DelayUsZero};

/// Use RISCV machine-mode cycle counter (`mcycle`) as a delay provider.
///
/// This can be used for high resolution delays for device initialization,
/// bit-banging protocols, etc
#[derive(Copy, Clone)]
pub struct McycleDelay {
    /// System clock frequency, used to convert clock cycles
    /// into real-world time values
    core_frequency: u32,
}

impl McycleDelay {
    /// Constructs the delay provider based on core clock frequency `freq`
    pub fn new(freq: u32) -> Self {
        Self {
            core_frequency: freq,
        }
    }

    /// Retrieves the cycle count for the current HART
    #[inline]
    pub fn get_cycle_count() -> u64 {
        riscv::register::mcycle::read64()
    }

    /// Returns the number of elapsed cycles since `previous_cycle_count`
    #[inline]
    pub fn cycles_since(previous_cycle_count: u64) -> u64 {
        riscv::register::mcycle::read64().wrapping_sub(previous_cycle_count)
    }

    /// Performs a busy-wait loop until the number of cycles `cycle_count` has elapsed
    #[inline]
    pub fn delay_cycles(cycle_count: u64) {
        let start_cycle_count = McycleDelay::get_cycle_count();

        while McycleDelay::cycles_since(start_cycle_count) <= cycle_count {}
    }
}

// embedded-hal 1.0 traits
impl DelayNs for McycleDelay {
    /// Performs a busy-wait loop until the number of nanoseconds `ns` has elapsed
    fn delay_ns(&mut self, ns: u32) {
        McycleDelay::delay_cycles((ns as u64 * (self.core_frequency as u64)) / 1_000_000_000);
    }
    /// Performs a busy-wait loop until the number of microseconds `us` has elapsed
    #[inline]
    fn delay_us(&mut self, us: u32) {
        McycleDelay::delay_cycles((us as u64 * (self.core_frequency as u64)) / 1_000_000);
    }
    /// Performs a busy-wait loop until the number of milliseconds `ms` has elapsed
    #[inline]
    fn delay_ms(&mut self, ms: u32) {
        McycleDelay::delay_cycles((ms as u64 * (self.core_frequency as u64)) / 1000);
    }
}

// embedded-hal 0.2 traits
impl DelayUsZero<u64> for McycleDelay {
    /// Performs a busy-wait loop until the number of microseconds `us` has elapsed
    #[inline]
    fn delay_us(&mut self, us: u64) {
        McycleDelay::delay_cycles((us * (self.core_frequency as u64)) / 1_000_000);
    }
}

// Call DelayMsZero::<u64>::delay_ms for all of u8/u16/u32/i8/i16/i32/i64
impl DelayMsZero<u8> for McycleDelay {
    /// Performs a busy-wait loop until the number of milliseconds `ms` has elapsed
    #[inline]
    fn delay_ms(&mut self, ms: u8) {
        DelayMsZero::<u64>::delay_ms(self, ms as u64);
    }
}

impl DelayMsZero<u16> for McycleDelay {
    /// Performs a busy-wait loop until the number of milliseconds `ms` has elapsed
    #[inline]
    fn delay_ms(&mut self, ms: u16) {
        DelayMsZero::<u64>::delay_ms(self, ms as u64);
    }
}

impl DelayMsZero<u32> for McycleDelay {
    /// Performs a busy-wait loop until the number of milliseconds `ms` has elapsed
    #[inline]
    fn delay_ms(&mut self, ms: u32) {
        DelayMsZero::<u64>::delay_ms(self, ms as u64);
    }
}

impl DelayMsZero<i8> for McycleDelay {
    /// Performs a busy-wait loop until the number of milliseconds `ms` has elapsed
    #[inline]
    fn delay_ms(&mut self, ms: i8) {
        DelayMsZero::<u64>::delay_ms(self, ms as u64);
    }
}

impl DelayMsZero<i16> for McycleDelay {
    /// Performs a busy-wait loop until the number of milliseconds `ms` has elapsed
    #[inline]
    fn delay_ms(&mut self, ms: i16) {
        DelayMsZero::<u64>::delay_ms(self, ms as u64);
    }
}

impl DelayMsZero<i32> for McycleDelay {
    /// Performs a busy-wait loop until the number of milliseconds `ms` has elapsed
    #[inline]
    fn delay_ms(&mut self, ms: i32) {
        DelayMsZero::<u64>::delay_ms(self, ms as u64);
    }
}

impl DelayMsZero<i64> for McycleDelay {
    /// Performs a busy-wait loop until the number of milliseconds `ms` has elapsed
    #[inline]
    fn delay_ms(&mut self, ms: i64) {
        DelayMsZero::<u64>::delay_ms(self, ms as u64);
    }
}

impl DelayMsZero<u64> for McycleDelay {
    /// Performs a busy-wait loop until the number of milliseconds `ms` has elapsed
    #[inline]
    fn delay_ms(&mut self, ms: u64) {
        McycleDelay::delay_cycles((ms * (self.core_frequency as u64)) / 1000);
    }
}
