// This example toggles the state of the the channels of the RGB LED on the Pinecone eval board
// using timer-based interrupts as well as a normal delay loop. The code takes advantage of the
// three match interrupts provided with TimerCh0 to perform multiple different blinking patterns
// using a single timer.
// The programmed blinking pattern below results in:
//      -the interrupt-controlled blue LED being on for 1500ms, then off for 1500ms.
//      -the interrupt-controlled green LED toggling on for 500ms and then off for 1000ms.
//      -the delay-loop controlled red LED toggling on for 3000ms and off for 3000ms.
// Global access to variables inside the interrupt handler function is controlled by the
// Mutex<RefCell<>> library, as outlined in the rust embedded handbook chapter on concurrency.
// See https://docs.rust-embedded.org/book/concurrency/index.html for more info.

#![no_std]
#![no_main]

use bl602_hal as hal;
use core::cell::RefCell;
use core::ops::DerefMut;
use embedded_hal::delay::blocking::DelayMs;
use embedded_hal::digital::blocking::{OutputPin, ToggleableOutputPin};
use embedded_time::{duration::*, rate::*};
use hal::{
    clock::{Strict, SysclkFreq},
    gpio::{Output, PullDown},
    interrupts::*,
    pac,
    prelude::*,
    timer::*,
};
use panic_halt as _;
use riscv::interrupt::Mutex;

// Setup custom types to make the code below easier to read:
type BlueLedPin = hal::gpio::Pin11<Output<PullDown>>;
type GreenLedPin = hal::gpio::Pin14<Output<PullDown>>;
type LedTimer = hal::timer::ConfiguredTimerChannel0;

// Initialize global static containers for peripheral access within the interrupt function:
static G_INTERRUPT_LED_PIN_B: Mutex<RefCell<Option<BlueLedPin>>> = Mutex::new(RefCell::new(None));
static G_INTERRUPT_LED_PIN_G: Mutex<RefCell<Option<GreenLedPin>>> = Mutex::new(RefCell::new(None));
static G_LED_TIMER: Mutex<RefCell<Option<LedTimer>>> = Mutex::new(RefCell::new(None));

#[riscv_rt::entry]
fn main() -> ! {
    // Setup the device peripherals:
    let dp = pac::Peripherals::take().unwrap();
    let mut glb = dp.GLB.split();

    // Set up all the clocks we need:
    let clocks = Strict::new()
        .use_pll(40_000_000u32.Hz())
        .sys_clk(SysclkFreq::Pll160Mhz)
        .freeze(&mut glb.clk_cfg);

    // Initialize all led pins to their default state:
    let mut r_led_pin = glb.pin17.into_pull_down_output();
    let _ = r_led_pin.set_high();

    let mut b_led_pin = glb.pin11.into_pull_down_output();
    let _ = b_led_pin.set_low();

    let mut g_led_pin = glb.pin14.into_pull_down_output();
    let _ = g_led_pin.set_high();

    // Initialize TimerCh0 to increment its count at a rate of 1000Hz:
    let timers = dp.TIMER.split();
    let timer_ch0 = timers
        .channel0
        .set_clock_source(ClockSource::Fclk(&clocks), 160_000_000_u32.Hz());

    // Set up Match0 which we will use to control the blue LED:
    // Note that you can use any embedded_time::duration as a time value in these set functions.
    timer_ch0.enable_match0_interrupt();
    timer_ch0.set_match0(1500_u32.milliseconds()); //toggles blue every 1500ms

    // Then set up Match1 and Match2 which we will use to control the green LED:
    timer_ch0.enable_match1_interrupt();
    timer_ch0.set_match1(500_000_000_u32.nanoseconds()); // turns green on after 500,000,000ns of a cycle
    timer_ch0.enable_match2_interrupt();
    timer_ch0.set_match2(1_u32.seconds()); //turns green back off after 1s

    // Use the Match0 interrupt as the trigger to reset the counter value to the preload value:
    timer_ch0.set_preload_value(0.microseconds());
    timer_ch0.set_preload(hal::timer::Preload::PreloadMatchComparator0);

    // Finally, remember to enable the timer channel so the interrupts will function:
    timer_ch0.enable();

    // Move the references to their UnsafeCells once initialized, and before interrupts are enabled:
    riscv::interrupt::free(|cs| G_INTERRUPT_LED_PIN_B.borrow(cs).replace(Some(b_led_pin)));
    riscv::interrupt::free(|cs| G_INTERRUPT_LED_PIN_G.borrow(cs).replace(Some(g_led_pin)));
    riscv::interrupt::free(|cs| G_LED_TIMER.borrow(cs).replace(Some(timer_ch0)));

    // Enable the timer interrupt only after pin and timer setup and move to global references:
    // If enabled before the needed variables are globally accessible, you won't be able to use
    // them inside the interrupt function, resulting in being unable to clear the timer interrupts
    // and immediately re-triggering the interrupt function when it returns.
    enable_interrupt(Interrupt::TimerCh0);

    // Create a blocking delay function based on the current cpu frequency for the red LED control:
    let mut d = bl602_hal::delay::McycleDelay::new(clocks.sysclk().0);

    loop {
        // Toggle the red channel every 3000ms to show it is still running in the background:
        let _ = d.delay_ms(3000);
        let _ = r_led_pin.toggle();
    }
}

// This handler is called by the hal whenever any of the three match interrupts on TimerCh0 is high.
// When using multiple match interrupts, the handler will need to check which is active and decide
// what actions to perform based on that information. Any active match interrupts will need to be
// cleared or the interrupt function will be called again immediately upon returning.
#[allow(non_snake_case)]
#[no_mangle]
fn TimerCh0(_trap_frame: &mut TrapFrame) {
    disable_interrupt(Interrupt::TimerCh0);
    clear_interrupt(Interrupt::TimerCh0);

    // Create local variables to hold data on which match caused the interrupt:
    // These will be reset every time the interrupt function runs.
    let mut is_match0_interrupt: bool = false;
    let mut is_match1_interrupt: bool = false;
    let mut is_match2_interrupt: bool = false;

    // Clear the active timer interrupts and set the flags to let us decide how to handle each case:
    riscv::interrupt::free(|cs| {
        if let Some(timer) = G_LED_TIMER.borrow(cs).borrow_mut().deref_mut() {
            if timer.is_match0() {
                timer.clear_match0_interrupt();
                is_match0_interrupt = true;
            }
            if timer.is_match1() {
                timer.clear_match1_interrupt();
                is_match1_interrupt = true;
            }
            if timer.is_match2() {
                timer.clear_match2_interrupt();
                is_match2_interrupt = true;
            }
        }
    });

    // match0 controls the blue led pin:
    if is_match0_interrupt {
        riscv::interrupt::free(|cs| {
            if let Some(led_pin) = G_INTERRUPT_LED_PIN_B.borrow(cs).borrow_mut().deref_mut() {
                led_pin.toggle().ok();
            }
        });
    }

    // match1 and match2 control the green led pin:
    if is_match1_interrupt || is_match2_interrupt {
        riscv::interrupt::free(|cs| {
            if let Some(led_pin) = G_INTERRUPT_LED_PIN_G.borrow(cs).borrow_mut().deref_mut() {
                led_pin.toggle().ok();
            }
        });
    }

    // Don't forget to re-enable timer interrupt when done:
    enable_interrupt(Interrupt::TimerCh0);
}
