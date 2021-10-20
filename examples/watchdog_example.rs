/*

   This example code is designed to always fail to `feed()` the watchdog so you can see that it is
   working as intended. When the watchdog fails, it sets the WTS bit of the WSR register on the
   bl602 to be 1. You can read this bit on initialization of the chip to see if the watchdog has
   been triggered.

   The code below will read this bit as 0 the first time it is powered on, and then it will fail
   to `feed()` the watchdog, resulting in a watchdog reset. The next time (and every subsequent
   time) it resets via the watchdog the WTS bit value that is read should be 1. To clear the bit,
   you either have to manually clear it via the watchdog.clear_wts() function, or reset the board
   using either the reset pin or turning off the power to the board and bringing it back online.

   After the board has been reset by the watchdog timer once, the code will change the Watchdog's
   mode to call the Watchdog interrupt function instead. This interrupt function will no
   longer reset the board, but will instead toggle the red led channel of the RGB led every time
   the watchdog is triggered.
*/

#![no_std]
#![no_main]

use bl602_hal as hal;
use core::cell::RefCell;
use core::fmt::Write;
use core::ops::DerefMut;
use embedded_hal::delay::blocking::DelayMs;
use embedded_hal::digital::blocking::{OutputPin, ToggleableOutputPin};
use embedded_hal::watchdog::blocking::{Enable, Watchdog};
use embedded_time::{duration::*, rate::*};
use hal::{
    clock::{Strict, SysclkFreq, UART_PLL_FREQ},
    gpio::{Output, PullDown},
    interrupts::*,
    pac,
    prelude::*,
    serial::*,
    timer::*,
    watchdog::*,
};
use heapless::String;
use panic_halt as _;
use riscv::interrupt::Mutex;

// Setup custom types to make the code below easier to read:
type RedLedPin = hal::gpio::Pin17<Output<PullDown>>;
type WatchdogTimer = hal::watchdog::ConfiguredWatchdog0;

// Initialize global static containers for peripheral access within the interrupt function:
static G_INTERRUPT_LED_PIN_R: Mutex<RefCell<Option<RedLedPin>>> = Mutex::new(RefCell::new(None));
static G_LED_TIMER: Mutex<RefCell<Option<WatchdogTimer>>> = Mutex::new(RefCell::new(None));

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

    // Initialize the led pin to their default state:
    let mut r_led_pin = gpio_pins.pin17.into_pull_down_output();
    let _ = r_led_pin.set_low();

    // Create a blocking delay function based on the current cpu frequency
    let mut d = bl602_hal::delay::McycleDelay::new(clocks.sysclk().0);

    // Set up the watchdog timer to the slowest tick rate possible:
    let timers = dp.TIMER.split();
    let watchdog = timers
        .watchdog
        .set_clock_source(WdtClockSource::Rc32Khz, 125.Hz());

    // Before setting up and enabling the watchdog,
    // retrieve and print to serial the state of the WTS bit:
    let wts_bit_value = match watchdog.has_watchdog_reset_occurred() {
        true => 1_u8,
        false => 0_u8,
    };

    let mut debug_string = String::<2048>::from("The watchdog WTS bit reads as: ");
    let _ = write!(debug_string, "{}\r\n", wts_bit_value);
    serial.write_str(debug_string.as_str()).ok();

    // On a clean boot, the watchdog will trigger a board reset if not fed in time.
    // If the watchdog has previously reset the board, switch to interrupt mode.
    match wts_bit_value {
        0_u8 => watchdog.set_mode(WatchdogMode::Reset),
        1_u8 => {
            watchdog.set_mode(WatchdogMode::Interrupt);
            enable_interrupt(Interrupt::Watchdog);
        }
        _ => unreachable!(),
    }

    // The watchdog timer doesn't begin counting ticks until it is started. We don't need to handle
    // the error state, since the watchdog start function will never actually return an Err().
    let mut watchdog = watchdog.start(10_u32.seconds()).unwrap();

    // Move the references to their UnsafeCells once initialized, and before interrupts are enabled:
    riscv::interrupt::free(|cs| G_INTERRUPT_LED_PIN_R.borrow(cs).replace(Some(r_led_pin)));
    riscv::interrupt::free(|cs| G_LED_TIMER.borrow(cs).replace(Some(watchdog)));

    loop {
        // Since we delay for more than 10 seconds, the watchdog is only fed the first time through
        // the loop. If you want to prevent the watchdog from triggering, decrease the number of
        // milliseconds in the delay to less than 10 seconds.

        // In order to use the watchdog once it has been moved into the RefCell, you must call free():
        riscv::interrupt::free(|cs| {
            if let Some(watchdog) = G_LED_TIMER.borrow(cs).borrow_mut().deref_mut() {
                watchdog.feed().ok();
            }
        });
        d.delay_ms(20_000).ok();
    }
}

// This is the interrupt handler for the watchdog. It currently toggles the red led channel of the
// RGB led on the board every time the watchdog is triggered after it has been reset at least once.
#[allow(non_snake_case)]
#[no_mangle]
fn Watchdog(_: &mut TrapFrame) {
    disable_interrupt(Interrupt::Watchdog);
    clear_interrupt(Interrupt::Watchdog);

    //Clear the WDT interrupt flag and feed the watchdog to reset its counter:
    riscv::interrupt::free(|cs| {
        if let Some(watchdog) = G_LED_TIMER.borrow(cs).borrow_mut().deref_mut() {
            watchdog.clear_interrupt();
            watchdog.feed().ok();
        }
    });

    // Toggle the red led whenever the interrupt is triggered:
    riscv::interrupt::free(|cs| {
        if let Some(led_pin) = G_INTERRUPT_LED_PIN_R.borrow(cs).borrow_mut().deref_mut() {
            led_pin.toggle().ok();
        }
    });

    enable_interrupt(Interrupt::Watchdog);
}
