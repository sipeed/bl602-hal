//! SoC clock configuration
// 其实和gpio两个模块同属GLB外设
// 时钟控制器
use crate::pac;
use crate::gpio::ClkCfg;
use core::num::NonZeroU32;
use embedded_time::rate::Hertz;

pub struct Clocks {
    uart_clk_div: u8,
}

impl Clocks {
    pub const fn uart_clk(&self) -> Hertz {
        Hertz(160_000_000 / self.uart_clk_div as u32)
    }
}

/// Strict clock configurator
///
/// This configurator only accepts strictly accurate value. If all available frequency
/// values after configurated does not strictly equal to the desired value, the `freeze`
/// function panics. Users must be careful to ensure that the output frequency values
/// can be strictly configurated into using input frequency values and internal clock
/// frequencies.
///
/// If you need to get most precise frequenct possible (other than the stictly accutare
/// value only), use configurator `Precise` instead.
///
/// For example if 49.60MHz and 50.20MHz are able to be configurated prefectly, input
/// 50MHz into `Strict` would result in a panic when performing `freeze`; however input
/// same 50MHz into `Precise` it would not panic, but would set and freeze into
/// 50.20MHz as the frequency error is smallest.
pub struct Strict {
    target_uart_clk: Option<NonZeroU32>,
}

impl Strict {
    /// Create a strict configurator
    pub fn new() -> Self {
        Strict {
            target_uart_clk: None,
        }
    }

    /// Sets the desired frequency for the UART-CLK clock
    pub fn uart_clk(mut self, freq: impl Into<Hertz>) -> Self {
        let freq_hz = freq.into().0;
        self.target_uart_clk = NonZeroU32::new(freq_hz);
        self
    }

    /// Calculate and balance clock registers to configure into the given clock value.
    /// If accurate value is not possible, this function panics. 
    /// 
    /// Be aware that Rust's panic is sometimes not obvious on embedded devices; if your
    /// program didn't execute as expected, or the `pc` is pointing to somewhere weird
    /// (usually `abort: j abort`), it's likely that this function have panicked. 
    /// Breakpoint on `rust_begin_unwind` may help debugging.
    ///
    /// # Panics
    ///
    /// If strictly accurate value of given `ck_sys` etc. is not reachable, this function
    /// panics. 
    pub fn freeze(self, clk_cfg: &mut ClkCfg) -> Clocks {
        drop(clk_cfg); // logically use its ownership
        let uart_clk = self.target_uart_clk.map(|f| f.get()).unwrap_or(40_000_000);
        let uart_clk_div = {
            let ans = 160_000_000 / uart_clk;
            if !(ans >= 1 && ans <= 7) || ans * uart_clk != 160_000_000 {
                panic!("unreachable uart_clk")
            }
            ans as u8
        };
        let glb = unsafe { &*pac::GLB::ptr() };
        glb.clk_cfg2.write(|w| unsafe { w
            .uart_clk_div().bits(uart_clk_div)
            .uart_clk_en().set_bit()
        });
        Clocks {
            uart_clk_div
        }
    }
}
