/*!
    # Watchdog
    The BL602 has a single watchdog timer. It can be configured to run from four different clock sources, which can be divided by 1-256. It has a single counter and comparator, which will determine when the watchdog is triggered. The trigger can either reset the chip or call an interrupt function.

    ## Watchdog Setup and Activation Example:
    ```rust
    use bl602_hal::{watchdog::*, interrupts::*, pac, timer::*,};

    let dp = pac::Peripherals::take().unwrap();
    let timers = dp.TIMER.split();
    let mut wd: ConfiguredWatchdog0 = timers
        .watchdog
        .set_clock_source(ClockSource::Clock1Khz, 1_000u32.Hz());
    wd.set_mode(WatchdogMode::Interrupt);
    wd.start(10u32.seconds());

    // When using the watchdog in interrupt mode, you must also enable the IRQ interrupt
    enable_interrupt(Interrupt::Watchdog);
    loop{
        // So other things in the loop, but make sure to feed the watchdog at least every 10 seconds
        wd.feed();
    }

    // ...

    // Make sure to clear the interrupt in your interrupt function or you'll never escape
    #[no_mangle]
    fn Watchdog(trap_frame: &mut TrapFrame){
        clear_interrupt(Interrupt::Watchdog);
    }

    ```
  # Units
  This library uses embedded_time::{duration::*, rate::*} for time units. You can use any supported units as long as they can be cast into Nanoseconds::<u64> for durations, or Hertz for cycles. Time can be cast into other units supported by embedded_time by explicitly typing a variable and calling .into() Note that this will round to the nearest integer in the cast units, potentially losing precision.

  ## Time Casting Example:
  ```rust
  use embedded_time::duration::*;
  // gets the current time in Nanoseconds::<u64> and casts it into milliseconds.
  let time_in_milliseconds: Milliseconds = watchdog.current_time().into();
  ```
 */

use crate::{pac, timer::{ClockSource, TimerWatchdog}};
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
    const fn get_bit_value(&self) -> bool {
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
    const fn get_key_value(&self) -> u16 {
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
    pub fn set_timeout(&self, time: impl Into<Nanoseconds::<u64>>) {
        let time: Nanoseconds::<u64> = time.into();
        let ticks = (self.clock.0 as u64 * time.integer() / 1_000_000_000_u64) as u16;
        let timer = unsafe { &*pac::TIMER::ptr() };
        timer.wsar.write(|w|unsafe {w.wsar().bits(WatchdogAccessKeys::get_key_value(&WatchdogAccessKeys::Wsar))});
        timer.wfar.write(|w|unsafe {w.wfar().bits(WatchdogAccessKeys::get_key_value(&WatchdogAccessKeys::Wfar))});
        timer.wmr.write(|w| unsafe { w.wmr().bits(ticks) });
    }

    /// Gets the value in ticks the match register is currently set to
    pub fn get_match_ticks(&self) -> u16 {
        let timer = unsafe { &*pac::TIMER::ptr() };
        timer.wmr.read().wmr().bits() as u16
    }

    /// Get the current value of the watchdog timer in nanoseconds
    pub fn get_match_time(&self) -> Nanoseconds::<u64> {
        let ticks = self.get_match_ticks() as u64;
        // ticks * (1e9 nanoseconds/second) / (ticks / second) = nanoseconds
        Nanoseconds::<u64>::new((ticks * 1_000_000_000_u64) / self.clock.integer() as u64)
    }

    /// clears the watchdog interrupt once it has been set by the WDT activating in Interrupt mode
    pub fn clear_interrupt(&self) {
        let timer = unsafe { &*pac::TIMER::ptr() };
        timer.wsar.write(|w|unsafe {w.wsar().bits(WatchdogAccessKeys::get_key_value(&WatchdogAccessKeys::Wsar))});
        timer.wfar.write(|w|unsafe {w.wfar().bits(WatchdogAccessKeys::get_key_value(&WatchdogAccessKeys::Wfar))});
        timer.wicr.write(|w| w.wiclr().clear_bit());
    }

    /// Get the current value in ticks of the watchdog timer
    pub fn get_current_ticks(&self) -> u16 {
        let timer = unsafe { &*pac::TIMER::ptr() };
        timer.wvr.read().wvr().bits() as u16
    }

    /// Get the current value of the watchdog timer in nanoseconds
    pub fn get_current_time(&self) -> Nanoseconds::<u64> {
        let ticks = self.get_current_ticks() as u64;
        // ticks * (1e9 nanoseconds/second) / (ticks / second) = nanoseconds
        Nanoseconds::<u64>::new((ticks * 1_000_000_000_u64) / self.clock.integer() as u64)
    }

    /// Determine whether the watchdog will reset the board, or trigger an interrupt
    pub fn set_mode(&self, mode: WatchdogMode) {
        let timer = unsafe { &*pac::TIMER::ptr() };
        match mode {
            WatchdogMode::Interrupt => {
                timer.wsar.write(|w|unsafe {w.wsar().bits(WatchdogAccessKeys::get_key_value(&WatchdogAccessKeys::Wsar))});
                timer.wfar.write(|w|unsafe {w.wfar().bits(WatchdogAccessKeys::get_key_value(&WatchdogAccessKeys::Wfar))});
                timer.wmer.write(|w| w.wrie().clear_bit());
            }
            WatchdogMode::Reset => {
                timer.wsar.write(|w|unsafe {w.wsar().bits(WatchdogAccessKeys::get_key_value(&WatchdogAccessKeys::Wsar))});
                timer.wfar.write(|w|unsafe {w.wfar().bits(WatchdogAccessKeys::get_key_value(&WatchdogAccessKeys::Wfar))});
                timer.wmer.write(|w| w.wrie().set_bit());
            }
        }
    }

    /// Clear the watchdog reset register (WTS)
    pub fn clear_wts(&self) {
        let timer = unsafe { &*pac::TIMER::ptr() };
        timer.wsar.write(|w|unsafe {w.wsar().bits(WatchdogAccessKeys::get_key_value(&WatchdogAccessKeys::Wsar))});
        timer.wfar.write(|w|unsafe {w.wfar().bits(WatchdogAccessKeys::get_key_value(&WatchdogAccessKeys::Wfar))});
        timer.wsr.write(|w| w.wts().clear_bit());
    }

    /// Check the value of the watchdog reset register (WTS)
    pub fn get_wts(&self) -> bool {
        let timer = unsafe { &*pac::TIMER::ptr() };
        timer.wsr.read().wts().bits() as bool
    }
}

impl embedded_hal::watchdog::blocking::Watchdog for ConfiguredWatchdog0 {
    type Error = WatchdogError;

    /// This feeds the watchdog by resetting its counter value to 0.
    /// WCR register is write-only, no need to preserve register contents
    fn feed(&mut self) -> Result<(), Self::Error> {
        let timer = unsafe { &*pac::TIMER::ptr() };
        timer.wsar.write(|w|unsafe {w.wsar().bits(WatchdogAccessKeys::get_key_value(&WatchdogAccessKeys::Wsar))});
        timer.wfar.write(|w|unsafe {w.wfar().bits(WatchdogAccessKeys::get_key_value(&WatchdogAccessKeys::Wfar))});
        timer.wcr.write(|w| w.wcr().set_bit());
        Ok(())
    }
}

impl embedded_hal::watchdog::blocking::Disable for ConfiguredWatchdog0 {
    type Error = WatchdogError;
    type Target = ConfiguredWatchdog0;

    fn disable(self) -> Result<Self::Target, Self::Error> {
        let timer = unsafe { &*pac::TIMER::ptr() };
        timer.wsar.write(|w|unsafe {w.wsar().bits(WatchdogAccessKeys::get_key_value(&WatchdogAccessKeys::Wsar))});
        timer.wfar.write(|w|unsafe {w.wfar().bits(WatchdogAccessKeys::get_key_value(&WatchdogAccessKeys::Wfar))});
        timer.wmer.write(|w| w.we().clear_bit());
        self.is_running.replace(false);
        Ok(self)
    }
}

impl embedded_hal::watchdog::blocking::Enable for ConfiguredWatchdog0 {
    type Error = WatchdogError;
    type Time = Nanoseconds::<u64>;
    type Target = ConfiguredWatchdog0;

    fn start<T>(self, period: T) -> Result<Self::Target, Self::Error> where T: Into<Self::Time> {
        self.set_timeout(period);
        self.enable();
        Ok(self)
    }
}

impl TimerWatchdog {
    pub fn set_clock_source(self, source: ClockSource, target_clock: impl Into<Hertz>) -> ConfiguredWatchdog0 {
        let target_clock = target_clock.into();
        let timer = unsafe{ &*pac::TIMER::ptr() };
        timer.wsar.write(|w|unsafe {w.wsar().bits(WatchdogAccessKeys::get_key_value(&WatchdogAccessKeys::Wsar))});
        timer.wfar.write(|w|unsafe {w.wfar().bits(WatchdogAccessKeys::get_key_value(&WatchdogAccessKeys::Wfar))});
        timer.tccr.modify(|_r, w| unsafe {w.cs_wdt().bits(source.tccr_value())});
        let divider = source.hertz().0/ target_clock.0;

        if !(1..256).contains(&divider) {
            panic!("unreachable target clock");
        }
        timer.wsar.write(|w|unsafe {w.wsar().bits(WatchdogAccessKeys::get_key_value(&WatchdogAccessKeys::Wsar))});
        timer.wfar.write(|w|unsafe {w.wfar().bits(WatchdogAccessKeys::get_key_value(&WatchdogAccessKeys::Wfar))});
        timer.tcdr.modify(|_r, w| unsafe { w.wcdr().bits((divider - 1) as u8) });

        ConfiguredWatchdog0 {
            clock: target_clock.into(),
            is_running: RefCell::new(false)
        }
    }
}
