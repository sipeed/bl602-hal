//! Pulse Width Modulation
//!
//! # Example
//!
//! ```no_run
//! # use crate::pwm;
//! # let dp = crate::pac::Peripherals::take().unwrap();
//! # let clocks = crate::clock::Clocks::new();
//! use embedded_time::duration::Milliseconds;
//!
//! let mut channels = pwm::Channels::from((dp.PWM, clocks));
//!
//! pwm.channel2.enable(&()).unwrap();
//!
//! pwm.channel2.set_period(Milleseconds::new(20)).unwrap();
//!
//! // 5% duty cycle
//! let duty = 5 * (pwm.channel2.get_max_duty().unwrap() / 100);
//! pwm.channel2.set_duty(duty).unwrap();
//!
//! // Use pin 17 as PWM channel 2's output
//! parts.pin17.into_pull_down_pwm();
//!
//! // Control PWM and its settings via the `pwm` object
//! ```

use core::convert::{Infallible, TryInto};
use embedded_hal::pwm::blocking::Pwm as PwmTrait;
use embedded_time::{
    duration::{Duration, Milliseconds, Seconds},
    fixed_point::FixedPoint,
    rate::Hertz,
};

use crate::{clock::Clocks, pac};

macro_rules! per_channel {
    ( $($channel:literal),* ) => { paste::paste!{
        /// PWM entry point
        pub struct Channels {
            $(pub [<channel $channel>]: [<Channel $channel>]),+
        }

        $(
            #[doc = concat!("PWM channel ", stringify!($channel)) ]
            pub struct [<Channel $channel>] {
                pwm: &'static pac::pwm::RegisterBlock,
                clocks: Clocks,
            }
        )+

        impl From<(pac::PWM, Clocks)> for Channels {
            fn from(other: (pac::PWM, Clocks)) -> Self {
                Self {
                    $([<channel $channel>]: [<Channel $channel>] {
                        pwm: unsafe { &*pac::PWM::ptr() },
                        clocks: other.1,
                    }),+
                }
            }
        }

        $(impl PwmTrait for [<Channel $channel>] {
            type Error = Infallible;
            type Channel = ();
            type Time = Milliseconds<u64>;
            type Duty = u16;

            fn disable(
                &mut self,
                channel: &Self::Channel,
            ) -> Result<(), Self::Error> {
                let _ = channel;

                self.pwm.[<pwm $channel _config>].write(|w|
                    w.pwm_stop_en().set_bit()
                );

                Ok(())
            }

            fn enable(
                &mut self,
                channel: &Self::Channel,
            ) -> Result<(), Self::Error> {
                let _ = channel;

                self.pwm.[<pwm $channel _config>].write(|w|
                    w.pwm_stop_en().clear_bit()
                );
                Ok(())
            }

            fn get_period(&self) -> Result<Self::Time, Self::Error> {
                todo!()
            }

            fn get_duty(
                &self,
                channel: &Self::Channel,
            ) -> Result<Self::Duty, Self::Error> {
                let _ = channel;

                Ok(self.pwm.[<pwm $channel _thre2>].read().pwm_thre2().bits())
            }

            fn get_max_duty(&self) -> Result<Self::Duty, Self::Error> {
                Ok(
                    self
                        .pwm
                        .[<pwm $channel _period>]
                        .read()
                        .pwm_period()
                        .bits()
                )
            }

            fn set_duty(
                &mut self,
                channel: &Self::Channel,
                duty: Self::Duty,
            ) -> Result<(), Self::Error> {
                let (_, duty) = (channel, duty);

                // Zero out threshold 1
                self.pwm.[<pwm $channel _thre1>].write(|w| unsafe {
                    w.pwm_thre1().bits(0)
                });

                // Set threshold 2
                self.pwm. [<pwm $channel _thre2>] .write(|w| unsafe {
                    w.pwm_thre2().bits(duty)
                });

                Ok(())
            }

            fn set_period<P>(
                &mut self,
                period: P,
            ) -> Result<(), Self::Error>
            where
                P: Into<Self::Time>
            {
                let period_time = period.into();

                // The non-whole-second portion of the desired period length
                let period_time_subsec = period_time % Seconds(1_u32);

                // The rate required by subsecond period time
                let period_rate: Hertz<u32> =
                    match period_time_subsec.to_rate() {
                        Ok(x) => x,
                        Err(_) => todo!(
                            "handle periods where period_time_subsec == 0?"
                        ),
                    };

                let clk_hz = self.clocks.sysclk();

                // Make clk_div as small as possible so Self::Duty can have a
                // usefully large range of values
                let clk_div_rem = clk_hz.integer()
                    % (u16::MAX as u32 * period_rate.integer());
                let clk_div = clk_hz.integer()
                    / (u16::MAX as u32 * period_rate.integer())
                    + if clk_div_rem == 0 { 0 } else { 1 };

                let clk_divided = clk_hz / clk_div as u32;

                // Divided clocks per period
                let period_val: u16 = (clk_divided / period_rate.integer())
                    .integer()
                    .try_into()
                    .unwrap_or(u16::max_value());

                // Use the system clock
                //
                // TODO: make this configurable
                self.pwm.[<pwm $channel _config>].write(|w| unsafe {
                    w.reg_clk_sel().bits(0b11)
                });

                // Clock divider
                self.pwm.[<pwm $channel _clkdiv>].write(|w| unsafe {
                    w.pwm_clk_div().bits(
                        clk_div
                            .try_into()
                            .unwrap_or(u16::max_value())
                    )
                });

                // Set how many divided clocks are in a period
                self.pwm.[<pwm $channel _period>].write(|w| unsafe {
                    w.pwm_period().bits(period_val)
                });

                Ok(())
            }
        })+
    }}
}

per_channel!(0, 1, 2, 3, 4);
