#![no_std]
#![no_main]

use core::mem::MaybeUninit;
use bl602_hal as hal;
use embedded_hal::digital::blocking::StatefulOutputPin;
use embedded_hal::digital::blocking::OutputPin;
use hal::{pac, prelude::*, interrupts::*};
use panic_halt as _;

use bl602_hal::gpio::InterruptPin;


static mut GPIO3: MaybeUninit<hal::gpio::pin::Pin3<hal::gpio::Input<hal::gpio::PullDown>>> = MaybeUninit::uninit();
static mut GPIO5: MaybeUninit<hal::gpio::pin::Pin5<hal::gpio::Output<hal::gpio::PullDown>>> = MaybeUninit::uninit();

fn get_gpio3() -> &'static mut hal::gpio::pin::Pin3<hal::gpio::Input<hal::gpio::PullDown>> {
    unsafe { &mut *GPIO3.as_mut_ptr() }
}

fn get_gpio5() -> &'static mut hal::gpio::pin::Pin5<hal::gpio::Output<hal::gpio::PullDown>> {
    unsafe { &mut *GPIO5.as_mut_ptr() }
}

#[riscv_rt::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let parts = dp.GLB.split();

    let mut gpio3 = parts.pin3.into_pull_down_input();
    let mut gpio5 = parts.pin5.into_pull_down_output();

    gpio5.set_high().unwrap();

    gpio3.enable_smitter();
    gpio3.trigger_on_event(hal::gpio::Event::NegativePulse);
    gpio3.control_asynchronous();

    gpio3.enable_interrupt();

    unsafe {
        *(GPIO3.as_mut_ptr()) = gpio3;
        *(GPIO5.as_mut_ptr()) = gpio5;
    }

    enable_interrupt(Interrupt::Gpio);

    loop {
        unsafe {
            riscv::asm::wfi();
        }
    }
}

#[allow(non_snake_case)]
#[no_mangle]
fn Gpio(_trap_frame: &mut TrapFrame) {
    disable_interrupt(Interrupt::Gpio);
    clear_interrupt(Interrupt::Gpio);

    get_gpio3().disable_interrupt();
    get_gpio3().clear_interrupt_pending_bit();

    let is_on = get_gpio5().is_set_high();
    if let Ok(res) = is_on {
        if res {
            get_gpio5().set_low().unwrap();
        }
        else {
            get_gpio5().set_high().unwrap();
        }
    }

    get_gpio3().enable_interrupt();
    enable_interrupt(Interrupt::Gpio);
}
