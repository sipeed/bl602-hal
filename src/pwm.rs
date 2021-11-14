//! Pulse Width Modulation
//!
//! # Example
//!
//! ```no_run
//! # use crate::pwm;
//! # let dp = crate::pac::Peripherals::take().unwrap();
//! # let clocks = crate::clock::Clocks::new();
//! let mut pwm = pwm::Pwm::from((dp.PWM, clocks));
//!
//! pwm.enable(&pwm::Channel::C2).unwrap();
//! pwm.set_period(embedded_time::duration::Seconds::new(1)).unwrap();
//! // TODO: duty cycle
//!
//! parts.pin17.into_pull_down_pwm();
//!
//! // Control PWM and its settings via the `pwm` object
//! ```

use core::convert::{TryInto, Infallible};
use embedded_hal::pwm::blocking::Pwm as PwmTrait;
use embedded_time::{
    rate::Hertz, fixed_point::FixedPoint, duration::{Duration, Nanoseconds}
};

use crate::{pac, clock::Clocks};

/// Interface to the PWM peripherals
pub struct Pwm {
    pwm: pac::PWM,
    clocks: Clocks,
}

impl From<(pac::PWM, Clocks)> for Pwm {
    fn from(other: (pac::PWM, Clocks)) -> Self {
        Self {
            pwm: other.0,
            clocks: other.1,
        }
    }
}

macro_rules! per_channel {
    ( $($channel:literal),* ) => { paste::paste!{
        /// PWM channel
        pub enum Channel {
            $(
                [<C $channel>],
            )+
        }

        impl PwmTrait for Pwm {
            type Error = Infallible;
            type Channel = Channel;
            type Time = Nanoseconds<u64>;
            type Duty = u32;

            fn disable(
                &mut self,
                channel: &Self::Channel,
            ) -> Result<(), Self::Error> {
                match channel {$(
                    Channel:: [<C $channel>] => {
                        self.pwm. [<pwm $channel _config>] .write(|w|
                            w.pwm_stop_en().set_bit()
                        );
                    }
                )+}
                Ok(())
            }

            fn enable(
                &mut self,
                channel: &Self::Channel,
            ) -> Result<(), Self::Error> {
                match channel {$(
                    Channel:: [<C $channel>] => {
                        self.pwm. [<pwm $channel _config>] .write(|w|
                            w.pwm_stop_en().clear_bit()
                        );
                    }
                )+}
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
                todo!()
            }

            fn get_max_duty(&self) -> Result<Self::Duty, Self::Error> {
                todo!()
            }

            fn set_duty(
                &mut self,
                channel: &Self::Channel,
                duty: Self::Duty,
            ) -> Result<(), Self::Error> {
                let (_, _) = (channel, duty);
                todo!()
            }

            fn set_period<P>(
                &mut self,
                period: P,
            ) -> Result<(), Self::Error>
            where
                P: Into<Self::Time>
            {
                let hz = self.clocks.sysclk();
                let clk_div = 1_000_000_u32;
                let period: Hertz<u32> = hz
                    / clk_div
                    / period.into()
                        .to_rate::<Hertz<_>>()
                        .unwrap()
                        .integer();

                let duty = period / 2;
                $(
                    self.pwm. [<pwm $channel _config>] .write(|w| {
                        unsafe { w.reg_clk_sel().bits(0b11) }
                    });

                    // Clock divider
                    self.pwm. [<pwm $channel _clkdiv>] .write(|w| unsafe {
                        w.pwm_clk_div().bits(
                            clk_div.try_into().unwrap_or(u16::max_value())
                        )
                    });

                    // Period
                    self.pwm. [<pwm $channel _period>] .write(|w| unsafe {
                        w.pwm_period().bits(
                            period.integer().try_into().unwrap_or(u16::max_value())
                        )
                    });

                    // Zero out threshold 1 (should be in set duty)
                    self.pwm. [<pwm $channel _thre1>] .write(|w| unsafe {
                        w.pwm_thre1().bits(0)
                    });

                    // Set threshold 2 (should be in set duty)
                    self.pwm. [<pwm $channel _thre2>] .write(|w| unsafe {
                        w.pwm_thre2().bits(
                            duty.integer().try_into().unwrap_or(u16::max_value())
                        )
                    });
                )+

                Ok(())
            }
        }
    }}
}

per_channel!(0, 1, 2, 3, 4);
