//! SoC clock configuration
// 其实和gpio两个模块同属GLB外设
// 时钟控制器
// The clocking in this chip is split into several peripheral sections oriented around low power modes
//
// Here is a quick overview of the peripheral sections as relates to those modes
//
// The GLB (global register) portion of the chip controls most clock enable/division circuits
//   as well as the GPIO
// The AON (always on) section is parts of the SOC that are active in all but the deepest
//   hibernate mode (HBN3). This section controls power to external high frequency crystal
// The PDS (power-down state, sleep) is the smallest level of power saving.
//   It always keeps CORE SRAM and timer power enabled.
//   Power to CPU, Wireless PHY+MAC, and digital/analog pins is optionally turned off at different pre-set levels
//   Peripherals that relate to clocking in this module: PLL
// The HBN (hibernate, deep sleep) section is the largest level of power saving.
//   It always turns off CPU, Wireless PHY+MAC, CORE SRAM and timers, and optionally sections or all of AON
//   It contains the root clock source selection (sysclk/flck)
// The L1C (level 1 cache) section maps tightly-coupled ram/cache SRAM in front of slower buses
//   (ROM, flash). It contains configuration for internal ROM access latency

use crate::pac;
use crate::gpio::ClkCfg;
use core::{num::NonZeroU32, unimplemented};
use embedded_time::rate::Hertz;
use crate::pac::Peripherals;
use embedded_hal::blocking::delay::{DelayUs};
use crate::delay::*;
pub struct Clocks {
    pll_xtal_freq: u32,
    sysclk: u32,
    uart_clk_div: u8,
}

impl Clocks {
    pub fn new() -> Self {
        Clocks {
            pll_xtal_freq: 0,
            sysclk: 32_000_000,
            uart_clk_div: 0,
        }
    }

    pub fn use_pll<F>(mut self, freq: F) -> Self
    where
        F: Into<Hertz>,
    {
        self.pll_xtal_freq = freq.into().0;
        self
    }

    pub fn sys_clk<F>(mut self, freq: F) -> Self
    where
        F: Into<Hertz>,
    {
        self.sysclk = freq.into().0;
        self
    }

    pub const fn uart_clk(&self) -> Hertz {
        Hertz(160_000_000 / self.uart_clk_div as u32)
    }

    pub fn freeze(self) -> Clocks {
        let glb = unsafe { &*pac::GLB::ptr() };
        glb.clk_cfg2.write(|w| unsafe { w
            .uart_clk_div().bits(self.uart_clk_div-1)
            .uart_clk_en().set_bit()
        });
        glb_set_system_clk(self.pll_xtal_freq, self.sysclk);
        let clocks = Clocks {
            pll_xtal_freq: self.pll_xtal_freq,
            sysclk: self.sysclk,
            uart_clk_div: self.uart_clk_div,
        };
        clocks
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
    pll_xtal_freq: Option<u32>,
    sysclk: Option<u32>,
}

impl Strict {
    /// Create a strict configurator
    pub fn new() -> Self {
        Strict {
            target_uart_clk: None,
            pll_xtal_freq: None,
            sysclk: None,
        }
    }

    /// Sets the desired frequency for the UART-CLK clock
    pub fn uart_clk(mut self, freq: impl Into<Hertz>) -> Self {
        let freq_hz = freq.into().0;
        self.target_uart_clk = NonZeroU32::new(freq_hz);
        self
    }

    pub fn use_pll(mut self, freq: impl Into<Hertz>) -> Self
    {
        self.pll_xtal_freq = Some(freq.into().0);
        self
    }

    pub fn sys_clk(mut self, freq: impl Into<Hertz>) -> Self
    {
        self.sysclk = Some(freq.into().0);
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

        // Default to not using the PLL, and selecting the internal RC oscillator if nothing selected
        // TODO: verify clocks
        let pll_xtal_freq = self.pll_xtal_freq.unwrap_or(0);
        let sysclk = self.sysclk.unwrap_or(32_000_000);

        // UART config
        // TODO: use 160_000_000 only when sourced from PLL, otherwise sysclk
        let uart_clk = self.target_uart_clk.map(|f| f.get()).unwrap_or(40_000_000);
        let uart_clk_div = {
            let ans = 160_000_000 / uart_clk;
            if !(ans >= 1 && ans <= 7) || ans * uart_clk != 160_000_000 {
                panic!("unreachable uart_clk")
            }
            ans as u8
        };


        Clocks {
            pll_xtal_freq,
            sysclk,
            uart_clk_div,
        }
    }
}

fn system_core_clock_set(value:u32){
    let hbn = unsafe { &*pac::HBN::ptr() };
    hbn.hbn_rsv2.write(|w| unsafe { w
        .bits(value)
    })
}

pub fn system_core_clock_get() -> u32 {
    let hbn = unsafe { &*pac::HBN::ptr() };
    hbn.hbn_rsv2.read().bits()
}

fn glb_set_system_clk_div(hclkdiv:u8, bclkdiv:u8){
    // recommended: fclk<=160MHz, bclk<=80MHz
    // fclk is determined by hclk_div (strange), which then feeds into bclk, hclk and uartclk
    // glb_reg_bclk_dis isn't in the SVD file, so it isn't generated through svd2rust 
    // It's only used here.
    let glb_reg_bclk_dis = 0x40000FFC as * mut u32;
    let glb = unsafe { &*pac::GLB::ptr() };
    glb.clk_cfg0.modify(|_, w| unsafe { w
        .reg_hclk_div().bits(hclkdiv)
        .reg_bclk_div().bits(bclkdiv)
    });
    unsafe { glb_reg_bclk_dis.write_volatile(1) };
    unsafe { glb_reg_bclk_dis.write_volatile(0) };
    let currclock = system_core_clock_get();
    system_core_clock_set(currclock / (hclkdiv as u32 + 1) );

    let mut delay = McycleDelay::new(system_core_clock_get());
    // This delay used to be 8 NOPS (1/4 us). Might need to be replaced again.
    delay.try_delay_us(1).unwrap();

    glb.clk_cfg0.modify(|_, w| { w
        .reg_hclk_en().set_bit()
        .reg_bclk_en().set_bit()
    });
    delay.try_delay_us(1).unwrap();
}


fn pds_select_xtal_as_pll_ref(){
    let pds = unsafe { &*pac::PDS::ptr() };
    pds.clkpll_top_ctrl.modify(|_, w| {w
        .clkpll_refclk_sel().set_bit()
        .clkpll_xtal_rc32m_sel().clear_bit()
    });
}

fn pds_power_off_pll(){
    let pds = unsafe { &*pac::PDS::ptr() };
    pds.pu_rst_clkpll.modify(|_, w| {w
        .pu_clkpll_sfreg().clear_bit()
        .pu_clkpll().clear_bit()
    });

    pds.pu_rst_clkpll.modify(|_, w| {w
        .clkpll_pu_cp().clear_bit()
        .clkpll_pu_pfd().clear_bit()
        .clkpll_pu_fbdv().clear_bit()
        .clkpll_pu_postdiv().clear_bit()
    });
}

/// Minimal implementation of power-on pll. Currently only allows external xtal
fn pds_power_on_pll(freq: u32) {
    let pds = unsafe { &*pac::PDS::ptr() };
    let mut delay = McycleDelay::new(system_core_clock_get());

    pds_select_xtal_as_pll_ref();

    // power off PLL first - this step is required
    pds_power_off_pll();

    // PLL param config
    //TODO: document + research this a bit more to work out if we can run at other frequecies
    if freq == 26_000_000 {
        pds.clkpll_cp.modify(|_, w| unsafe {w
            .clkpll_icp_1u().bits(1)
            .clkpll_icp_5u().bits(0)
            .clkpll_int_frac_sw().set_bit()
        });
        pds.clkpll_rz.modify(|_, w| unsafe {w
            .clkpll_c3().bits(2)
            .clkpll_cz().bits(2)
            .clkpll_rz().bits(5)
            .clkpll_r4_short().clear_bit()
        });
    } else {
        pds.clkpll_cp.modify(|_, w| unsafe {w
            .clkpll_icp_1u().bits(0)
            .clkpll_icp_5u().bits(2)
            .clkpll_int_frac_sw().clear_bit()
        });
        pds.clkpll_rz.modify(|_, w| unsafe {w
            .clkpll_c3().bits(3)
            .clkpll_cz().bits(1)
            .clkpll_rz().bits(1)
            .clkpll_r4_short().set_bit()
        });
    }

    pds.clkpll_top_ctrl.modify(|_, w| unsafe {w
        .clkpll_postdiv().bits(0x14)
        .clkpll_refdiv_ratio().bits(2)
    });

    pds.clkpll_sdm.modify(|_, w| unsafe {w
        .clkpll_sdmin().bits(
            match freq {
                24_000_000 =>  0x50_0000,
                32_000_000 =>  0x3C_0000,
                38_400_000 =>  0x32_0000,
                40_000_000 =>  0x30_0000,
                26_000_000 =>  0x49_D39D,
                _ => panic!()
            }
        )
    });

    pds.clkpll_fbdv.modify(|_, w| unsafe {w
        .clkpll_sel_fb_clk().bits(1)
        .clkpll_sel_sample_clk().bits(1)
    });

    /*************************/
    /* PLL power up sequence */
    /*************************/
    pds.pu_rst_clkpll.modify(|_, w| {w
        .pu_clkpll_sfreg().set_bit()
    });

    delay.try_delay_us(5).unwrap();

    pds.pu_rst_clkpll.modify(|_, w| {w
        .pu_clkpll().set_bit()
    });

    pds.pu_rst_clkpll.modify(|_, w| {w
        .clkpll_pu_cp().set_bit()
        .clkpll_pu_pfd().set_bit()
        .clkpll_pu_fbdv().set_bit()
        .clkpll_pu_postdiv().set_bit()
    });

    delay.try_delay_us(5).unwrap();

    pds.pu_rst_clkpll.modify(|_, w| {w
        .clkpll_sdm_reset().set_bit()
    });

    delay.try_delay_us(1).unwrap();

    pds.pu_rst_clkpll.modify(|_, w| {w
        .clkpll_reset_fbdv().set_bit()
    });

    delay.try_delay_us(2).unwrap();

    pds.pu_rst_clkpll.modify(|_, w| {w
        .clkpll_reset_fbdv().clear_bit()
    });

    delay.try_delay_us(1).unwrap();

    pds.pu_rst_clkpll.modify(|_, w| {w
        .clkpll_sdm_reset().clear_bit()
    });
}

fn aon_power_on_xtal() {
    let aon = unsafe { &*pac::AON::ptr() };
    aon.rf_top_aon.modify(|_, w| { w
        .pu_xtal_aon().set_bit()
        .pu_xtal_buf_aon().set_bit()
    });

    let mut delaysrc = McycleDelay::new(system_core_clock_get());
    let mut timeout:u32 = 0;
    delaysrc.try_delay_us(10).unwrap();
    while aon.tsen.read().xtal_rdy().bit_is_clear() && timeout < 120{
        delaysrc.try_delay_us(10).unwrap();
        timeout+=1;
    }
    // TODO: error out on timeout
}

fn hbn_set_root_clk_sel_pll(){
    let hbn = unsafe { &*pac::HBN::ptr() };
    hbn.hbn_glb.modify(|r,w| unsafe { w
        .hbn_root_clk_sel().bits(
            r.hbn_root_clk_sel().bits() as u8 | 0b10u8
        )
    });
}

fn hbn_set_root_clk_sel_rc32(){
    let hbn = unsafe { &*pac::HBN::ptr() };
    hbn.hbn_glb.modify(|_, w| unsafe { w
        .hbn_root_clk_sel().bits(0b00u8)
    });
}

fn pds_enable_pll_all_clks(){
    let pds = unsafe { &*pac::PDS::ptr() };
    pds.clkpll_output_en.modify(|r, w| unsafe {w
        .bits(r.bits() | 0x1FF)
    });
}

/// Set the system clock to use the internal 32Mhz RC oscillator
fn glb_set_system_clk_rc32(){
    // reg_bclk_en = reg_hclk_en = reg_fclk_en = 1, cannot be zero
    let glb = unsafe { &*pac::GLB::ptr() };
    glb.clk_cfg0.modify(|_, w| { w
        .reg_bclk_en().set_bit()
        .reg_hclk_en().set_bit()
        .reg_fclk_en().set_bit()
    });

    // Before config XTAL and PLL ,make sure root clk is from RC32M
    hbn_set_root_clk_sel_rc32();

    glb.clk_cfg0.modify(|_, w| unsafe { w
        .reg_hclk_div().bits(0)
        .reg_bclk_div().bits(0)
    });

    // Update sysclock
    system_core_clock_set(32_000_000);

    // Select PKA clock from hclk
    glb.swrst_cfg2.modify(|_, w| { w
        .pka_clk_sel().clear_bit()
    });
}

/// Original code supported a bunch of configurations for core clock
/// There are probably uses for driving PLL using RC or using XTAL direct for root clock,
/// but it complicates something that is already sufficiently complex.
/// Settling for two configuration options for now:
///   - internal 32Mhz RC oscillator for sysclock
///   - XTAL driving PLL, sysclock frequencies of 48/80/120/160/192Mhz
pub fn glb_set_system_clk(xtal_freq: u32, sysclk_freq: u32) {
    // Ensure clock is running off internal RC oscillator before changing anything else
    glb_set_system_clk_rc32();
    // if target clock is 32Mhz we don't have to do any more
    if sysclk_freq == 32_000_000{
        return
    } else {
        // Configure XTAL, PLL and select it as clock source for fclk
        glb_set_system_clk_pll(sysclk_freq, xtal_freq)
    }
}

fn glb_set_system_clk_pll(target_core_clk: u32, xtal_freq: u32) {
    let glb = unsafe { &*pac::GLB::ptr() };
    // Power up the external crystal before we start up the PLL
    aon_power_on_xtal();

    // Power up PLL and enable all PLL clock output
    pds_power_on_pll(xtal_freq);

    let mut delay = McycleDelay::new(system_core_clock_get());
    delay.try_delay_us(55).unwrap();

    pds_enable_pll_all_clks();
    
    // Enable PLL
    glb.clk_cfg0.modify(|_, w| {w
        .reg_pll_en().set_bit()
    });

    // select which pll output clock to use before 
    // selecting root clock via HBN_Set_ROOT_CLK_Sel
    // Note that 192Mhz is out of spec
    glb.clk_cfg0.modify(|_, w| unsafe {w
        .reg_pll_sel().bits(
            match target_core_clk {
                48_000_000 => 0,
                120_000_000 => 1,
                160_000_000 => 2,
                192_000_000 => 3,
                _ => {panic!()}
            }
        )
    });

    // Keep bclk <= 80MHz
    if target_core_clk > 48_000_000 {
        glb_set_system_clk_div(0, 1);
    }

    // For frequencies above 120Mhz we need 2 clocks to access internal rom
    if target_core_clk > 120_000_000 {
        let l1c = unsafe { &*pac::L1C::ptr() };
        l1c.l1c_config.modify(|_, w| {w
            .irom_2t_access().set_bit()
        });
    }

    hbn_set_root_clk_sel_pll();
    system_core_clock_set(target_core_clk);
    
    let mut delay = McycleDelay::new(system_core_clock_get());
    // This delay used to be 8 NOPS (1/4 us). (GLB_CLK_SET_DUMMY_WAIT) Might need to be replaced again.
    delay.try_delay_us(1).unwrap();

    // use 120Mhz PLL tap for PKA clock since we're using PLL
    // NOTE: This isn't documented in the datasheet!
    glb.swrst_cfg2.modify(|_, w| { w
        .pka_clk_sel().set_bit()
    });
}