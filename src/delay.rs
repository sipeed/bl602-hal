//! Delays

use embedded_hal::timer::CountDown;
use crate::clock::Clocks;
use embedded_hal::blocking::delay::{DelayUs, DelayMs};
/// Machine mode cycle counter (`mcycle`) as a delay provider
#[derive(Copy, Clone)]
pub struct McycleDelay {
    core_frequency: u32
}
#[derive(Debug, Clone)]
pub struct DelayError;

impl McycleDelay {
    /// Constructs the delay provider based on provided core clock frequency
    pub fn new(freq: u32) -> Self {
        Self {
            core_frequency: freq
        }
    }

    pub fn get_time() -> u64 {
        riscv::register::mcycle::read64()
    }

    pub fn elapsed_us(time: u64) -> u64 {
        riscv::register::mcycle::read64().wrapping_sub(time)
    }
}

impl DelayUs<u64> for McycleDelay {
    type Error = DelayError;
    fn try_delay_us(&mut self, us: u64) -> Result<(), <Self as DelayUs<u64>>::Error>  {
        let t0 = riscv::register::mcycle::read64();
        let clocks = (us * (self.core_frequency as u64)) / 1_000_000;
        while riscv::register::mcycle::read64().wrapping_sub(t0) <= clocks { }
        Ok(())
    }
}

impl DelayMs<u64> for McycleDelay {
    type Error = DelayError;
    fn try_delay_ms(&mut self, us: u64) -> Result<(), <Self as DelayMs<u64>>::Error>  {
        let t0 = riscv::register::mcycle::read64();
        let clocks = (us * (self.core_frequency as u64)) / 1000;
        while riscv::register::mcycle::read64().wrapping_sub(t0) <= clocks { }
        Ok(())
    }
}