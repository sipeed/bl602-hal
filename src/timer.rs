/*!
  # Timer
  The chip has two 32-bit counters, each of which can independently control and configure its parameters and clock frequency

  ## Example
  ```rust
    let timers = dp.TIMER.split();

    let ch0 = timers
        .channel0
        .set_clock_source(ClockSource::Clock1Khz, 1_000u32.Hz());

    ch0.enable_match0_interrupt();
    ch0.enable_match1_interrupt();
    ch0.disable_match2_interrupt();

    ch0.set_preload_value(0.milliseconds());
    ch0.set_preload(hal::timer::Preload::PreloadMatchComparator1);
    ch0.set_match0(3_u32.seconds());
    ch0.set_match1(7_000_000_000_u32.microseconds());

    ch0.enable(); // start timer
  ```
*/

use crate::{clock::Clocks, pac};
use bl602_pac::TIMER;
use core::cell::RefCell;
use embedded_time::{
    duration::*,
    rate::*,
};
use paste::paste;

/// Error for [CountDown](embedded_hal::timer::CountDown)
#[derive(Debug)]
pub enum CountDownError {
    /// Indicates that the clock wrapped during count down
    Wrapped,
}

/// Clock sources for a timer channel.
/// There are four timer clock sources available.
pub enum ClockSource<'a> {
    /// System master clock
    Fclk(&'a Clocks),
    /// 32K clock
    Rc32Khz,
    /// 1K clock (32K frequency division)
    Clock1Khz,
    /// 32M clock
    Pll32Mhz,
}

impl<'a> ClockSource<'a> {
    pub fn tccr_value(&self) -> u8 {
        match self {
            ClockSource::Fclk(_) => 0,
            ClockSource::Rc32Khz => 1,
            ClockSource::Clock1Khz => 2,
            ClockSource::Pll32Mhz => 3,
        }
    }

    pub fn hertz(&self) -> Hertz {
        match self {
            ClockSource::Fclk(clocks) => clocks.sysclk(),
            ClockSource::Rc32Khz => 32_000.Hz(),
            ClockSource::Clock1Khz => 1_000.Hz(),
            ClockSource::Pll32Mhz => 32_000_000.Hz(),
        }
    }
}

/// When to preload
pub enum Preload {
    /// No preload
    NoPreload,
    /// Preload when comparator 0 matches
    PreloadMatchComparator0,
    /// Preload when comparator 1 matches
    PreloadMatchComparator1,
    /// Preload when comparator 2 matches
    PreloadMatchComparator2,
}

impl Preload {
    fn to_prlcr(&self) -> u8 {
        match self {
            Preload::NoPreload => 0,
            Preload::PreloadMatchComparator0 => 1,
            Preload::PreloadMatchComparator1 => 2,
            Preload::PreloadMatchComparator2 => 3,
        }
    }
}

/// Timer Channel 0
pub struct TimerChannel0 {}

/// Timer Channel 1
pub struct TimerChannel1 {}

/// Watchdog Timer
pub struct TimerWatchdog {}

/// Timers obtained from [TIMER.split](bl602_pac::Peripherals::TIMER)
pub struct Timers {
    pub channel0: TimerChannel0,
    pub channel1: TimerChannel1,
    pub watchdog: TimerWatchdog,
}

macro_rules! impl_timer_channel {
    ($name: ident, $conf_name: ident, $channel: literal, $channel_cs: literal) => {

        /// A configured timer channel ready to use.
        pub struct $conf_name {
            clock: Hertz,
            count_down_target: Option<Milliseconds>,
            last_count_down_value: Option<Milliseconds>,
            is_running: RefCell<bool>,
        }

        paste! {
            impl $conf_name {
                /// Enable interrupt for match register 0.
                pub fn enable_match0_interrupt(&self) {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.[<tier $channel>].modify(|_r, w| w.tier_0().set_bit());
                }

                /// Enable interrupt for match register 1.
                pub fn enable_match1_interrupt(&self) {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.[<tier $channel>].modify(|_r, w| w.tier_1().set_bit());
                }

                /// Enable interrupt for match register 2.
                pub fn enable_match2_interrupt(&self) {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.[<tier $channel>].modify(|_r, w| w.tier_2().set_bit());
                }

                /// Disable interrupt for match register 0.
                pub fn disable_match0_interrupt(&self) {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.[<tier $channel>].modify(|_r, w| w.tier_0().clear_bit());
                }

                /// Disable interrupt for match register 1.
                pub fn disable_match1_interrupt(&self) {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.[<tier $channel>].modify(|_r, w| w.tier_1().clear_bit());
                }

                /// Disable interrupt for match register 2.
                pub fn disable_match2_interrupt(&self) {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.[<tier $channel>].modify(|_r, w| w.tier_2().clear_bit());
                }

                /// Enable this counter
                pub fn enable(&self) {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.tcer.modify(|_r, w| w.[<timer $channel _en>]().set_bit());
                    self.is_running.replace(true);
                }

                /// Disable this counter
                pub fn disable(&self) {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.tcer.modify(|_r, w| w.[<timer $channel _en>]().set_bit());
                    self.is_running.replace(false);
                }

                /// Check if the timer is enabled / running
                pub fn is_enabled(&self) -> bool {
                    *self.is_running.borrow()
                }

                /// Clear interrupt for match register 0.
                /// TICR register is write-only, no need to preserve register contents
                pub fn clear_match0_interrupt(&self) {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.[<ticr $channel>].write(|w| w.tclr_0().set_bit());
                }

                /// Clear interrupt for match register 1.
                /// TICR register is write-only, no need to preserve register contents
                pub fn clear_match1_interrupt(&self) {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.[<ticr $channel>].write(|w| w.tclr_1().set_bit());
                }

                /// Clear interrupt for match register 2.
                /// TICR register is write-only, no need to preserve register contents
                pub fn clear_match2_interrupt(&self) {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.[<ticr $channel>].write(|w| w.tclr_2().set_bit());
                }

                /// Sets when the to preload.
                pub fn set_preload(&self, preload: Preload) {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer
                        .[<tplcr $channel>]
                        .modify(|_r, w| unsafe { w.tplcr().bits(preload.to_prlcr()) });
                }

                /// Sets match register 0
                pub fn set_match0(&self, time: impl Into<Nanoseconds::<u64>>) {
                    let time: Nanoseconds::<u64> = time.into();
                    let time = (self.clock.0 as u64 * time.integer() / 1_000_000_000_u64) as u32;
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.[<tmr $channel _0>].modify(|_r, w| unsafe { w.tmr().bits(time) });
                }

                /// Sets match register 1
                pub fn set_match1(&self, time: impl Into<Nanoseconds::<u64>>) {
                    let time: Nanoseconds::<u64> = time.into();
                    let time = (self.clock.0 as u64 * time.integer() / 1_000_000_000_u64) as u32;
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.[<tmr $channel _1>].modify(|_r, w| unsafe { w.tmr().bits(time) });
                }

                /// Sets match register 2
                pub fn set_match2(&self, time: impl Into<Nanoseconds::<u64>>) {
                    let time: Nanoseconds::<u64> = time.into();
                    let time = (self.clock.0 as u64 * time.integer() / 1_000_000_000_u64) as u32;
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.[<tmr $channel _2>].modify(|_r, w| unsafe { w.tmr().bits(time) });
                }

                // Current counter value in raw ticks.
                pub fn current_ticks(&self) -> u32 {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.[<tcr $channel>].read().bits()
                }

                // Current counter value in milliseconds.
                pub fn current_time(&self) -> Milliseconds {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    let ticks = timer.[<tcr $channel>].read().bits() as u64;
                    Milliseconds::new( (ticks as u64 * 1000u64 / self.clock.0 as u64) as u32)
                }

                /// Will only become true if `enable_match0_interrupt` is active
                pub fn is_match0(&self) -> bool {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.[<tmsr $channel>].read().tmsr_0().bit()
                }

                /// Will only become true if `enable_match2_interrupt` is active
                pub fn is_match1(&self) -> bool {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.[<tmsr $channel>].read().tmsr_1().bit()
                }

                /// Will only become true if `enable_match2_interrupt` is active
                pub fn is_match2(&self) -> bool {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.[<tmsr $channel>].read().tmsr_2().bit()
                }

                /// Set pre-load mode.
                pub fn pre_load_mode(&self) {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.tcmr.modify(|_r, w| w.[<timer $channel _mode>]().clear_bit());
                }

                /// Set free running mode.
                pub fn free_running_mode(&self) {
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.tcmr.modify(|_r, w| w.[<timer $channel _mode>]().set_bit());
                }

                /// The value which should be used for preload.
                pub fn set_preload_value(&self, time: impl Into<Nanoseconds::<u64>>) {
                    let time: Nanoseconds::<u64> = time.into();
                    let time = (self.clock.0 as u64 * time.0 / 1_000_000_000_u64) as u32;
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer.[<tplvr $channel>].modify(|_r, w| unsafe { w.bits(time) });
                }
            }
        }

        impl embedded_hal::timer::nb::CountDown for $conf_name {
            type Error = CountDownError;

            type Time = Milliseconds;

            fn start<T>(&mut self, count: T) -> Result<(), Self::Error>
            where
                T: Into<Self::Time>,
            {
                self.count_down_target = Some(Milliseconds(self.current_time().0 + count.into().0));
                self.last_count_down_value = None;
                Ok(())
            }

            fn wait(&mut self) -> nb::Result<(), Self::Error> {
                match self.count_down_target {
                    Some(millis) => {
                        let current_time = self.current_time();

                        if current_time >= millis {
                            Ok(())
                        } else {
                            match self.last_count_down_value {
                                Some(last_count_down_value) => {
                                    if current_time < last_count_down_value {
                                        Err(nb::Error::Other(CountDownError::Wrapped))
                                    } else {
                                        self.last_count_down_value = Some(current_time);
                                        Err(nb::Error::WouldBlock)
                                    }
                                }
                                None => {
                                    self.last_count_down_value = Some(current_time);
                                    Err(nb::Error::WouldBlock)
                                }
                            }
                        }
                    }
                    None => Ok(()),
                }
            }
        }

        paste! {
            impl $name {

                /// Configures the clock source and creates a configured timer channel
                pub fn set_clock_source(
                    self,
                    source: ClockSource,
                    desired_timing: impl Into<Hertz>,
                ) -> $conf_name {
                    let target_clock: Hertz = desired_timing.into();
                    let timer = unsafe { &*pac::TIMER::ptr() };
                    timer
                        .tccr
                        .modify(|_r, w| unsafe { w.[<cs_ $channel_cs>]().bits(source.tccr_value()) });

                    let divider = (source.hertz() / target_clock.0).0;

                    if !(1..=256).contains(&divider) {
                        panic!("Unreachable target clock");
                    }

                    timer
                        .tcdr
                        .modify(|_r, w| unsafe { w.[<tcdr $channel>]().bits((divider - 1) as u8) });

                    timer.tcmr.modify(|_r, w| {
                        w.[<timer $channel _mode>]().clear_bit() // pre-load mode
                    });
                    timer.[<tplvr $channel>].modify(|_r, w| unsafe {
                        w.tplvr().bits(0) // pre-load value
                    });

                    $conf_name {
                        clock: target_clock,
                        count_down_target: None,
                        last_count_down_value: None,
                        is_running: RefCell::new(false),
                    }
                }
            }
        }
    }
}

impl_timer_channel!(TimerChannel0, ConfiguredTimerChannel0, 2, 1);

impl_timer_channel!(TimerChannel1, ConfiguredTimerChannel1, 3, 2);

/// Extension trait to split TIMER peripheral into independent channels
pub trait TimerExt {
    fn split(self) -> Timers;
}

impl TimerExt for TIMER {
    fn split(self) -> Timers {
        Timers {
            channel0: TimerChannel0 {},
            channel1: TimerChannel1 {},
            watchdog: TimerWatchdog {},
        }
    }
}
