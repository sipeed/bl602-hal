/*

   This example code is designed to always fail to `feed()` the watchdog so you can see that it is
   working as intended. When the watchdog fails, it sets the WTS bit of the WSR register on the
   bl602 to be 1. You can read this bit on initialization of the chip to see if the watchdog has
   been triggered.

   The code below should read this bit as 0 the first time it is powered on, and then it will fail
   to `feed()` the watchdog, resulting in a watchdog reset. The next time (and every subsequent
   time) it resets via the watchdog the WTS bit value that is read should be 1. To clear the bit,
   you either have to manually clear it via the watchdog.clear_wts() function, or reset the board
   using either the reset pin or turning off the power to the board and bringing it back online.
*/

#![no_std]
#![no_main]

use bl602_hal as hal;
use core::fmt::Write;
use embedded_hal::delay::blocking::DelayMs;
use embedded_hal::watchdog::blocking::{Enable, Watchdog};
use embedded_time::{duration::*, rate::*};
use hal::{
    clock::{Strict, SysclkFreq, UART_PLL_FREQ},
    interrupts::*,
    pac,
    prelude::*,
    serial::*,
    timer::*,
    watchdog::*,
};
use heapless::String;
use panic_halt as _;

#[riscv_rt::entry]
fn main() -> ! {
    //take control of the device peripherals:
    let dp = pac::Peripherals::take().unwrap();
    let mut gpio_pins = dp.GLB.split();

    // Set up all the clocks we need
    let clocks = Strict::new()
        .use_pll(40_000_000u32.Hz())
        .sys_clk(SysclkFreq::Pll160Mhz)
        .uart_clk(UART_PLL_FREQ.Hz())
        .freeze(&mut gpio_pins.clk_cfg);

    // Set up uart output for debug printing. Since this microcontroller has a pin matrix,
    // we need to set up both the pins and the muxs
    let pin16 = gpio_pins.pin16.into_uart_sig0();
    let pin7 = gpio_pins.pin7.into_uart_sig7();
    let mux0 = gpio_pins.uart_mux0.into_uart0_tx();
    let mux7 = gpio_pins.uart_mux7.into_uart0_rx();

    // Configure our UART to 2MBaud, and use the pins we configured above
    let mut serial = Serial::uart0(
        dp.UART,
        Config::default().baudrate(2_000_000.Bd()),
        ((pin16, mux0), (pin7, mux7)),
        clocks,
    );

    // Create a blocking delay function based on the current cpu frequency
    let mut d = bl602_hal::delay::McycleDelay::new(clocks.sysclk().0);

    // Set up the watchdog timer:
    let timers = dp.TIMER.split();
    let watchdog = timers
        .watchdog
        .set_clock_source(ClockSource::Rc32Khz, 4.Hz());

    let mut debug_string = String::<2048>::from("Clock Select Bits: ");
    let _ = write!(debug_string, "{}\r\n", watchdog.get_cs_wdt());
    serial.write_str(debug_string.as_str());

    // The watchdog timer will reset the board if not fed often enough.
    watchdog.set_mode(WatchdogMode::Reset);

    // This will cause the board to reset if the watchdog is not fed at least once every 10 seconds.
    let mut watchdog = watchdog.start(100_u32.seconds()).unwrap();

    // If you were using the watchdog timer in interrupt mode instead of reset mode, you would need
    // enable the interrupt like below:
    // enable_interrupt(Interrupt::Watchdog);

    // Before entering the loop, print to serial the state of the WTS bit:
    let wts_bit_value = match watchdog.get_wts() {
        true => 1_u8,
        false => 0_u8,
    };

    let mut debug_string = String::<2048>::from("The watchdog WTS bit reads as: ");
    let _ = write!(debug_string, "{}\r\n", wts_bit_value);
    serial.write_str(debug_string.as_str());


    let mut debug_string = String::<2048>::from("Match Ticks: ");
    let _ = write!(debug_string, "{}\r\n", watchdog.get_match_ticks());
    serial.write_str(debug_string.as_str());

    loop {
        // Uncomment the 2 lines below to `feed()` the watchdog once every second. This will result
        // in the board never resetting, since the watchdog has been fed.

        // watchdog.feed();
        // d.delay_ms(250);

        let mut debug_string = String::<2048>::from("Current Ticks: ");
        let _ = write!(debug_string, "{}\r\n", watchdog.get_current_ticks());
        serial.write_str(debug_string.as_str());
        d.delay_ms(1000);
    }
}

// This is a sample interrupt handler for the watchdog. You can uncomment it to test
// WatchdogMode::Interrupt functionality.

// #[no_mangle]
// fn Watchdog(trap_frame: &mut TrapFrame) {
//     disable_interrupt(Interrupt::Watchdog);
//     clear_interrupt(Interrupt::Watchdog);
//
//     // Do something when the watchdog interrupt is triggered.
//
//     enable_interrupt(Interrupt::Watchdog);
// }
