#![no_std]
#![no_main]

use bl602_hal as hal;
use embedded_hal::delay::blocking::DelayMs;
use embedded_hal::digital::blocking::OutputPin;
use hal::{
    clock::{Strict, SysclkFreq, UART_PLL_FREQ},
    pac,
    prelude::*,
};
use panic_halt as _;

#[riscv_rt::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let mut parts = dp.GLB.split();

    // Set up all the clocks we need
    let clocks = Strict::new()
        .use_pll(40_000_000u32.Hz())
        .sys_clk(SysclkFreq::Pll160Mhz)
        .uart_clk(UART_PLL_FREQ.Hz())
        .freeze(&mut parts.clk_cfg);

    let mut gpio5 = parts.pin5.into_pull_down_output();

    // Create a blocking delay function based on the current cpu frequency
    let mut d = bl602_hal::delay::McycleDelay::new(clocks.sysclk().0);

    loop {
        gpio5.set_high().unwrap();
        d.delay_ms(1000).unwrap();

        gpio5.set_low().unwrap();
        d.delay_ms(1000).unwrap();
    }
}
