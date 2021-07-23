#![no_std]
#![no_main]

use bl602_hal as hal;
use core::{convert::TryFrom, fmt::Write};
use embedded_time::{duration::Seconds, Clock};
use hal::{
    clock::{Strict, SysclkFreq, UART_PLL_FREQ},
    pac,
    prelude::*,
    rtc::Rtc,
    serial::*,
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

    // Set up uart output. Since this microcontroller has a pin matrix,
    // we need to set up both the pins and the muxs
    let pin16 = parts.pin16.into_uart_sig0();
    let pin7 = parts.pin7.into_uart_sig7();
    let mux0 = parts.uart_mux0.into_uart0_tx();
    let mux7 = parts.uart_mux7.into_uart0_rx();

    // Configure our UART to 115200Baud, and use the pins we configured above
    let mut serial = Serial::uart0(
        dp.UART,
        Config::default().baudrate(115_200.Bd()),
        ((pin16, mux0), (pin7, mux7)),
        clocks,
    );

    // Create RTC
    let rtc = Rtc::rtc(dp.HBN);

    loop {
        write!(
            serial,
            "Current millis since start of the rtc: {}\r\n",
            rtc.get_millis()
        )
        .ok();

        let seconds =
            Seconds::<u64>::try_from(rtc.try_now().unwrap().duration_since_epoch()).unwrap();
        write!(serial, "Current instant in seconds: {}\r\n", seconds).ok();

        let timer = rtc.new_timer(Seconds(1u32)).into_oneshot();
        timer.start().unwrap().wait().ok();
    }
}
