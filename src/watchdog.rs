/*!
    # Watchdog
    The BL602 has a single watchdog timer. It can be configured to run from four different clock sources, which can be divided by 1-256. It has a single counter and comparator, which will determine when the watchdog is triggered. The trigger can either reset the chip or call an interrupt function.

    # Examples
    ``` rust

    ```
*/

use crate::{clock::Clocks, pac, timer::{ClockSource, TimerWatchdog}};
use bl602_pac::TIMER;
use core::cell::RefCell;
use embedded_time::{
    duration::*,
    rate::*,
};

/// Error for [Watchdog](embedded_hal::watchdog::blocking::Watchdog)
#[derive(Debug)]
pub enum WatchdogError {
    Infallible,
}


pub enum WatchdogMode {
    Interrupt,
    Reset,
}

impl WatchdogMode {
    /// Returns the bit value to be set for the WMER WRIE bit to configure
    /// the watchdog timer to operate in reset mode or in interrupt mode
    fn get_bit_value(&self) -> bool {
        match self {
            WatchdogMode::Interrupt => false,
            WatchdogMode::Reset => true,
        }
    }
}

pub enum WatchdogAccessKeys {
    Wfar,
    Wsar,
}

impl WatchdogAccessKeys {
    /// Returns the key access register values that must be written
    /// to write to the watchdog timer's registers
    fn get_key_value(&self) -> u16 {
        match self {
            WatchdogAccessKeys::Wfar => 0xBABA_u16,
            WatchdogAccessKeys::Wsar => 0xEB10_u16,
        }
    }
}

/// A configured Watchdog timer ready to be enabled or `feed()`
pub struct ConfiguredWatchdog0 {
    clock: Hertz,
    is_running: RefCell<bool>,
}

impl ConfiguredWatchdog0 {
    /// Enable the watchdog counter
    pub fn enable(&self) {
        let timer = unsafe { &*pac::TIMER::ptr() };
        timer.wmer.modify(|_r, w| w.we().set_bit());
        self.is_running.replace(true);
    }

    /// Check if the watchdog counter is enabled / running
    pub fn is_enabled(&self) -> bool {
        *self.is_running.borrow()
    }

    /// Set the time that the watchdog timer will be triggered unless `feed()`
    pub fn set_match(&self, time: Nanoseconds::<u64>) {
        todo!()
    }

    /// Gets the value in ticks the match register is set to
    pub fn get_match_ticks(&self) -> u16 {
        todo!()
    }

    pub fn get_match(&self) -> Milliseconds {
        todo!()
    }

    pub fn clear_interrupt(&self) {
        todo!()
    }

    /// Get the current value in ticks of the watchdog timer
    pub fn current_ticks(&self) -> u16 {
        todo!()
    }

    /// Get the current value of the watchdog timer in milliseconds
    pub fn current_time(&self) -> Nanoseconds::<u64> {
        todo!()
    }

    /// Determine whether the watchdog will reset the board, or trigger an interupt
    pub fn set_mode(&self, mode: WatchdogMode) {
        todo!()
    }

    /// Clear the watchdog reset register (WTS)
    pub fn clear_wts(&self) {
        todo!()
    }

    /// Check the value of the watchdog reset register (WTS)
    pub fn get_wts(&self) -> bool {
        todo!()
    }
}

impl embedded_hal::watchdog::blocking::Watchdog for ConfiguredWatchdog0 {
    type Error = WatchdogError;

    /// This feeds the watchdog by resetting its counter value to 0.
    /// WCR register is write-only, no need to preserve register contents
    fn feed(&mut self) -> Result<(), Self::Error> {
        let timer = unsafe { &*pac::TIMER::ptr() };
        timer.wcr.write(|w| w.wcr().set_bit());
        Ok(())
    }
}

impl embedded_hal::watchdog::blocking::Disable for ConfiguredWatchdog0 {
    type Error = WatchdogError;
    type Target = ConfiguredWatchdog0;

    fn disable(self) -> Result<Self::Target, Self::Error> {
        self.is_running.replace(false);
        todo!()
    }
}

impl embedded_hal::watchdog::blocking::Enable for ConfiguredWatchdog0 {
    type Error = WatchdogError;
    type Time = Nanoseconds::<u64>;
    type Target = ConfiguredWatchdog0;

    fn start<T>(self, period: T) -> Result<Self::Target, Self::Error> where T: Into<Self::Time> {
        todo!()
    }
}

impl TimerWatchdog {
    pub fn set_clock_source(self, source: ClockSource, target_clock: impl Into<Hertz>) -> ConfiguredWatchdog0 {
        let target_clock = target_clock.into();
        let timer = unsafe{ &*pac::TIMER::ptr() };
        timer.tccr.modify(|_r, w| unsafe {w.cs_wdt().bits(source.tccr_value())});
        let divider = (source.hertz().0/ target_clock.0);

        if !(1..256).contains(&divider) {
            panic!("unreachable target clock");
        }

        timer.tcdr.modify(|_r, w| unsafe { w.wcdr().bits((divider - 1) as u8) });

        ConfiguredWatchdog0 {
            clock: target_clock.into(),
            is_running: RefCell::new(false)
        }

    }
}
