/*!
   # Watchdog
   The BL602 has a single watchdog timer. It can be configured to run from four different clock sources, which can be divided by 1-256. It has a single counter and comparator, which will determine when the watchdog is triggered. The trigger can either reset the chip or call an interrupt function.

   ## Watchdog Setup and Activation Example:
   ```rust

   let dp = pac::Peripherals::take().unwrap();
   let timers = dp.TIMER.split();
   let mut wd: ConfiguredWatchdog0 = timers
       .watchdog
       .set_clock_source(WdtClockSource::Rc32Khz, 125.Hz());
   wd.set_mode(WatchdogMode::Interrupt);
   wd.start(10.seconds());

   // When using the watchdog in interrupt mode, you must also enable the IRQ interrupt
   enable_interrupt(Interrupt::Watchdog);
   loop{
       // Do other things in the loop, but make sure to feed the watchdog at least every 10 seconds
       wd.feed();
   }

   // ...

   // If you fail to feed the watchdog, the interrupt function `Watchdog()` will be triggered.
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

use crate::{clock::Clocks, pac, timer::TimerWatchdog};
use embedded_time::{duration::*, rate::*};

/// Clock sources for a Watchdog channel.
/// There are only three timer clock sources available.
pub enum WdtClockSource<'a> {
    /// System master clock
    Fclk(&'a Clocks),
    /// 32K clock
    Rc32Khz,
    /// 32M clock
    Pll32Mhz,
}

impl<'a> WdtClockSource<'a> {
    fn tccr_value(&self) -> u8 {
        match self {
            WdtClockSource::Fclk(_) => 0,
            WdtClockSource::Rc32Khz => 1,
            WdtClockSource::Pll32Mhz => 3,
        }
    }

    fn hertz(&self) -> Hertz {
        match self {
            WdtClockSource::Fclk(clocks) => clocks.sysclk(),
            WdtClockSource::Rc32Khz => 32_000.Hz(),
            WdtClockSource::Pll32Mhz => 32_000_000.Hz(),
        }
    }
}

/// Error for [Watchdog](embedded_hal::watchdog::blocking::Watchdog)
#[derive(Debug)]
pub enum WatchdogError {
    Infallible,
}

pub enum WatchdogMode {
    Interrupt,
    Reset,
}

pub enum WatchdogKeys {
    Wfar,
    Wsar,
}

impl WatchdogKeys {
    /// Returns the key access register values that must be written immediately before
    /// writing any of the watchdog timer's critical register values
    fn get_key(&self) -> u16 {
        match self {
            WatchdogKeys::Wfar => 0xBABA_u16,
            WatchdogKeys::Wsar => 0xEB10_u16,
        }
    }
}

/// A configured Watchdog timer ready to be enabled or `feed()`
pub struct ConfiguredWatchdog0 {
    clock: Hertz,
}

/// This sends the access codes so that we can write values to the WDT registers.
fn send_access_codes() {
    let timer = unsafe { &*pac::TIMER::ptr() };
    timer
        .wfar
        .write(|w| unsafe { w.wfar().bits(WatchdogKeys::Wfar.get_key()) });
    timer
        .wsar
        .write(|w| unsafe { w.wsar().bits(WatchdogKeys::Wsar.get_key()) });
}

impl ConfiguredWatchdog0 {
    /// Enable the watchdog counter
    pub fn enable(&self) {
        let timer = unsafe { &*pac::TIMER::ptr() };
        send_access_codes();
        timer.wcr.write(|w| w.wcr().set_bit());
        send_access_codes();
        timer.wmer.modify(|_r, w| w.we().set_bit());
    }

    /// Read the WMER register's WE bit to see if the WDT is enabled or disabled.
    pub fn is_enabled(&self) -> WatchdogMode {
        let timer = unsafe { &*pac::TIMER::ptr() };
        match timer.wmer.read().we().bit() {
            true => WatchdogMode::Reset,
            false => WatchdogMode::Interrupt,
        }
    }

    //noinspection RsSelfConvention
    /// Set the time that the watchdog timer will be triggered unless `feed()`
    pub fn set_timeout(&self, time: impl Into<Nanoseconds<u64>>) {
        let time: Nanoseconds<u64> = time.into();
        let ticks = (self.clock.0 as u64 * time.integer() / 1_000_000_000_u64) as u16;
        let timer = unsafe { &*pac::TIMER::ptr() };
        send_access_codes();
        timer.wmr.write(|w| unsafe { w.wmr().bits(ticks) });
    }

    //noinspection RsSelfConvention
    /// Determine whether the watchdog will reset the board, or trigger an interrupt
    pub fn set_mode(&self, mode: WatchdogMode) {
        let timer = unsafe { &*pac::TIMER::ptr() };
        match mode {
            WatchdogMode::Interrupt => {
                send_access_codes();
                timer.wmer.write(|w| w.wrie().clear_bit());
            }
            WatchdogMode::Reset => {
                send_access_codes();
                timer.wmer.write(|w| w.wrie().set_bit());
            }
        }
    }

    /// Check the value of the watchdog reset register (WTS) to see if a reset has occurred
    pub fn has_watchdog_reset_occurred(&self) -> bool {
        let timer = unsafe { &*pac::TIMER::ptr() };
        timer.wsr.read().wts().bit_is_set()
    }

    /// Clear the watchdog reset register (WTS)
    pub fn clear_wts(&self) {
        let timer = unsafe { &*pac::TIMER::ptr() };
        send_access_codes();
        timer.wsr.write(|w| w.wts().set_bit());
    }

    /// clears the watchdog interrupt once it has been set by the WDT activating in Interrupt mode
    pub fn clear_interrupt(&self) {
        let timer = unsafe { &*pac::TIMER::ptr() };
        send_access_codes();
        timer.wicr.write(|w| w.wiclr().set_bit());
    }

    /// Gets the value in ticks the match register is currently set to
    pub fn get_match_ticks(&self) -> u16 {
        let timer = unsafe { &*pac::TIMER::ptr() };
        timer.wmr.read().wmr().bits() as u16
    }

    /// Get the current value of the watchdog timer in nanoseconds
    pub fn get_match_time(&self) -> Nanoseconds<u64> {
        let ticks = self.get_match_ticks() as u64;
        // ticks * (1e9 nanoseconds/second) / (ticks / second) = nanoseconds
        Nanoseconds::<u64>::new((ticks * 1_000_000_000_u64) / self.clock.integer() as u64)
    }

    /// Get the current value in ticks of the watchdog timer
    pub fn get_current_ticks(&self) -> u16 {
        let timer = unsafe { &*pac::TIMER::ptr() };
        timer.wvr.read().wvr().bits() as u16
    }

    /// Get the current value of the watchdog timer in nanoseconds
    pub fn get_current_time(&self) -> Nanoseconds<u64> {
        let ticks = self.get_current_ticks() as u64;
        // ticks * (1e9 nanoseconds/second) / (ticks / second) = nanoseconds
        Nanoseconds::<u64>::new((ticks * 1_000_000_000_u64) / self.clock.integer() as u64)
    }

    /// Read the TCCR register containing the CS_WDT bits that select the clock source
    pub fn get_cs_wdt(&self) -> u8 {
        let timer = unsafe { &*pac::TIMER::ptr() };
        timer.tccr.read().cs_wdt().bits() as u8
    }

    /// Read the WMER register's WRIE bit to see if the WDT is in Reset or Interrupt mode.
    pub fn get_wrie(&self) -> WatchdogMode {
        let timer = unsafe { &*pac::TIMER::ptr() };
        match timer.wmer.read().wrie().bit() {
            true => WatchdogMode::Reset,
            false => WatchdogMode::Interrupt,
        }
    }

    /// Read the TCDR register's WCDR bits to see the clock division value.
    pub fn get_wcdr(&self) -> u8 {
        let timer = unsafe { &*pac::TIMER::ptr() };
        timer.tcdr.read().wcdr().bits()
    }
}

impl embedded_hal::watchdog::blocking::Watchdog for ConfiguredWatchdog0 {
    type Error = WatchdogError;

    /// This feeds the watchdog by resetting its counter value to 0.
    /// WCR register is write-only, no need to preserve register contents
    fn feed(&mut self) -> Result<(), Self::Error> {
        let timer = unsafe { &*pac::TIMER::ptr() };
        send_access_codes();
        timer.wcr.write(|w| w.wcr().set_bit());
        Ok(())
    }
}

impl embedded_hal::watchdog::blocking::Disable for ConfiguredWatchdog0 {
    type Error = WatchdogError;
    type Target = ConfiguredWatchdog0;

    fn disable(self) -> Result<Self::Target, Self::Error> {
        let timer = unsafe { &*pac::TIMER::ptr() };
        send_access_codes();
        timer.wmer.write(|w| w.we().clear_bit());
        Ok(self)
    }
}

impl embedded_hal::watchdog::blocking::Enable for ConfiguredWatchdog0 {
    type Error = WatchdogError;
    type Time = Nanoseconds<u64>;
    type Target = ConfiguredWatchdog0;

    fn start<T>(self, period: T) -> Result<Self::Target, Self::Error>
    where
        T: Into<Self::Time>,
    {
        self.set_timeout(period);
        self.enable();
        Ok(self)
    }
}

impl TimerWatchdog {
    //noinspection RsSelfConvention
    /// This sets up the watchdog clock source and target clock speed, and returns a ConfiguredWatchdog0.
    /// Note that when setting up the clock source, you will need to select a source and tick rate that
    /// allows for your desired timeout time to be expressed as 65535 or less ticks of the WDT.
    ///
    /// # Examples:
    ///
    ///     - Slowest possible tick rate: Clock Source of 32 kHz clock selected, with target clock of 125 Hz
    ///
    ///         - This results in the max time of 65535 ticks / 125 Hz = 524.28 seconds
    ///
    ///     - Fast 32kHz tick rate: Clock source of 32 kHz clock selected, with target clock of 32_000 Hz.
    ///
    ///         - This results in the max time of 65535 ticks / 32_000 Hz = ~2.04 seconds
    ///
    ///     - Fastest possible tick rate: Clock source of Fclk at 160 MHz clock selected, with target clock of 160_000_000 Hz.
    ///
    ///         - This results in the max time of 65535 ticks / 160_000_000 Hz = ~4.09 milliseconds
    ///
    pub fn set_clock_source(
        self,
        source: WdtClockSource,
        target_clock: impl Into<Hertz>,
    ) -> ConfiguredWatchdog0 {
        let target_clock = target_clock.into();
        let timer = unsafe { &*pac::TIMER::ptr() };
        timer
            .tccr
            .modify(|_r, w| unsafe { w.cs_wdt().bits(source.tccr_value()) });
        let divider = source.hertz().0 / target_clock.0;

        if !(1..=256).contains(&divider) {
            panic!("unreachable target clock");
        }

        timer
            .tcdr
            .modify(|_r, w| unsafe { w.wcdr().bits((divider - 1) as u8) });

        //clear interrupt bit when initializing:
        send_access_codes();
        timer.wicr.write(|w| w.wiclr().clear_bit());

        ConfiguredWatchdog0 {
            clock: target_clock,
        }
    }
}
