/*!
  # Interrupt Management
  Interrupts can be enabled, disabled and cleared.

  ## Example
  ```rust
    enable_interrupt(TimerCh0);

    // ...

    #[no_mangle]
    fn TimerCh0() {
        // ..
        clear_interrupt(TimerCh0);
    }
  ```

  ## The following functions can be implemented as interrupt handlers
  ```rust
    fn Gpio();
    fn TimerCh0();
    fn TimerCh1();
    fn Watchdog();
  ```
*/

use riscv::register::mcause;

extern "C" {
    fn Gpio(trap_frame: &mut TrapFrame);
    fn TimerCh0(trap_frame: &mut TrapFrame);
    fn TimerCh1(trap_frame: &mut TrapFrame);
    fn Watchdog(trap_frame: &mut TrapFrame);
}

// see components\bl602\bl602_std\bl602_std\RISCV\Core\Include\clic.h
// see components\hal_drv\bl602_hal\bl_irq.c
const IRQ_NUM_BASE: u32 = 16;
const CLIC_HART0_ADDR: u32 = 0x02800000;
const CLIC_INTIE: u32 = 0x400;
const CLIC_INTIP: u32 = 0x000;

const GPIO_IRQ: u32 = IRQ_NUM_BASE + 44;
const TIMER_CH0_IRQ: u32 = IRQ_NUM_BASE + 36;
const TIMER_CH1_IRQ: u32 = IRQ_NUM_BASE + 37;
const WATCHDOG_IRQ: u32 = IRQ_NUM_BASE + 38;

#[doc(hidden)]
#[no_mangle]
pub fn _setup_interrupts() {
    extern "C" {
        pub fn _start_trap_hal();
    }

    let new_mtvec = _start_trap_hal as usize;
    unsafe {
        riscv::interrupt::disable();
        riscv::register::mtvec::write(new_mtvec | 2, riscv::register::mtvec::TrapMode::Direct);
    }

    // disable all interrupts
    let e = unsafe {
        core::slice::from_raw_parts_mut((CLIC_HART0_ADDR + CLIC_INTIE) as *mut u32, 16 + 8)
    };
    let p = unsafe {
        core::slice::from_raw_parts_mut((CLIC_HART0_ADDR + CLIC_INTIP) as *mut u32, 16 + 8)
    };

    e.iter_mut().for_each(|v| *v = 0);
    p.iter_mut().for_each(|v| *v = 0);

    unsafe {
        riscv::interrupt::enable();
    }
}

/// Registers saved in trap handler
#[doc(hidden)]
#[allow(missing_docs)]
#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct TrapFrame {
    pub ra: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s0: usize,
    pub s1: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub gp: usize,
    pub tp: usize,
    pub sp: usize,
}

/// # Safety
///
/// This function is called from an assembly trap handler.
#[doc(hidden)]
#[link_section = ".trap.rust"]
#[export_name = "_start_trap_rust_hal"]
pub unsafe extern "C" fn start_trap_rust_hal(trap_frame: *mut TrapFrame) {
    extern "C" {
        pub fn _start_trap_rust(trap_frame: *const TrapFrame);
    }

    let cause = mcause::read();
    if cause.is_exception() {
        _start_trap_rust(trap_frame);
    } else {
        let code = cause.code();
        if code < IRQ_NUM_BASE as usize {
            _start_trap_rust(trap_frame);
        } else {
            let interrupt_number = (code & 0xff) as u32;
            let interrupt = Interrupt::from(interrupt_number);

            match interrupt {
                Interrupt::Unknown => _start_trap_rust(trap_frame),
                Interrupt::Gpio => Gpio(trap_frame.as_mut().unwrap()),
                Interrupt::TimerCh0 => TimerCh0(trap_frame.as_mut().unwrap()),
                Interrupt::TimerCh1 => TimerCh1(trap_frame.as_mut().unwrap()),
                Interrupt::Watchdog => TimerCh1(trap_frame.as_mut().unwrap()),
            };
        }
    }
}

/// Available interrupts
pub enum Interrupt {
    #[doc(hidden)]
    Unknown,
    /// GPIO Interrupt
    Gpio,
    /// Timer Channel 0 Interrupt
    TimerCh0,
    /// Timer Channel 1 Interrupt
    TimerCh1,
    /// Watchdog Timer Interrupt
    /// Used when WDT is configured in Interrupt mode using ConfiguredWatchdog0::set_mode()
    Watchdog,
}

impl Interrupt {
    fn to_irq(&self) -> u32 {
        match &self {
            Interrupt::Unknown => panic!("Unknown interrupt has no irq number"),
            Interrupt::Gpio => GPIO_IRQ,
            Interrupt::TimerCh0 => TIMER_CH0_IRQ,
            Interrupt::TimerCh1 => TIMER_CH1_IRQ,
            Interrupt::Watchdog => WATCHDOG_IRQ,
        }
    }

    fn from(irq: u32) -> Interrupt {
        match irq {
            GPIO_IRQ => Interrupt::Gpio,
            TIMER_CH0_IRQ => Interrupt::TimerCh0,
            TIMER_CH1_IRQ => Interrupt::TimerCh1,
            WATCHDOG_IRQ => Interrupt::Watchdog,
            _ => Interrupt::Unknown,
        }
    }
}

/// Enable the given interrupt
pub fn enable_interrupt(interrupt: Interrupt) {
    let irq = interrupt.to_irq();
    let ptr = (CLIC_HART0_ADDR + CLIC_INTIE + irq) as *mut u8;
    unsafe {
        ptr.write_volatile(1);
    }
}

/// Disable the given interrupt
pub fn disable_interrupt(interrupt: Interrupt) {
    let irq = interrupt.to_irq();
    let ptr = (CLIC_HART0_ADDR + CLIC_INTIE + irq) as *mut u8;
    unsafe {
        ptr.write_volatile(0);
    }
}

/// Clear the given interrupt.
/// Usually the interrupt needs to be cleared also on the peripheral level.
pub fn clear_interrupt(interrupt: Interrupt) {
    let irq = interrupt.to_irq();
    let ptr = (CLIC_HART0_ADDR + CLIC_INTIP + irq) as *mut u8;
    unsafe {
        ptr.write_volatile(0);
    }
}
