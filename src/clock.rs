//! SoC clock configuration
// 其实和gpio两个模块同属GLB外设
// 时钟控制器
use crate::pac;
use crate::gpio::ClkCfg;
use core::{num::NonZeroU32, unimplemented};
use embedded_time::rate::Hertz;
use crate::pac::Peripherals;
use embedded_hal::blocking::delay::{DelayUs};
use crate::delay::*;
pub struct Clocks {
    target_clksrc: HbnRootClkType,
    pll_xtal: GlbPllXtalType,
    target_sys_ck: SysClk,
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

/// HBN root clock type definition
#[allow(dead_code)]
#[repr(u8)]
enum HbnRootClkType {
    Rc32m = 0,           // use RC32M as root clock
    Xtal  = 1,           // use XTAL as root clock
    Pll   = 2,           // use PLL as root clock
}

/**
 *  @brief PLL XTAL type definition
 */
 #[allow(dead_code)]
 #[repr(u8)]
 #[derive(PartialEq)]
pub enum GlbPllXtalType {
    None        = 0,     // XTAL is none
    Xtal24m    = 1,     // XTAL is 24M
    Xtal32m    = 2,     // XTAL is 32M
    Xtal38p4m  = 3,     // XTAL is 38.4M
    Xtal40m    = 4,     // XTAL is 40M
    Xtal26m    = 5,     // XTAL is 26M
    Rc32m       = 6,     // XTAL is RC32M
}
#[allow(dead_code)]
#[derive(PartialEq)]
pub enum SysClk {
    Rc32m   = 0, // use RC32M as system clock frequency
    Xtal    = 1, // use XTAL as system clock
    Pll48m  = 2, // use PLL output 48M as system clock
    Pll120m = 3, // use PLL output 120M as system clock
    Pll160m = 4, // use PLL output 160M as system clock
    Pll192m = 5, // use PLL output 192M as system clock
}

pub fn system_core_clock_set(value:u32){
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
    let glb_reg_bclk_dis = 0x40000FFC as * mut u32;
    let glb = unsafe { &*pac::GLB::ptr() };
    glb.clk_cfg0.modify(|_,w| unsafe { w
        .reg_hclk_div().bits(hclkdiv)
        .reg_bclk_div().bits(bclkdiv)
    });
    unsafe { glb_reg_bclk_dis.write_volatile(1) };
    unsafe { glb_reg_bclk_dis.write_volatile(0) };
    let currclock = system_core_clock_get();
    system_core_clock_set(currclock / (hclkdiv as u32 + 1) );

    // The original delays in this function were 8 NOP instructions. at 32mhz, this is 1/4 of a us
    // but since we just changed our clock source, we'll wait the equivalent of 1us worth
    // of clocks at 160Mhz (this *should* be much longer than necessary)
    // Might be worth switching to asm once that stabilises - seems to be okay for now
    let mut delay = McycleDelay::new(system_core_clock_get());
    delay.try_delay_us(1).unwrap();

    glb.clk_cfg0.modify(|_,w| { w
        .reg_hclk_en().set_bit()
        .reg_bclk_en().set_bit()
    });
    delay.try_delay_us(1).unwrap();
}


fn pds_select_xtal_as_pll_ref(){
    let pds = unsafe { &*pac::PDS::ptr() };
    pds.clkpll_top_ctrl.modify(|_r,w| {w
        .clkpll_refclk_sel().set_bit()
        .clkpll_xtal_rc32m_sel().clear_bit()
    });
}

fn pds_power_off_pll(){
    /* pu_clkpll_sfreg=0 */
    /* pu_clkpll=0 */
    let pds = unsafe { &*pac::PDS::ptr() };
    pds.pu_rst_clkpll.modify(|_r, w| {w
        .pu_clkpll_sfreg().clear_bit()
        .pu_clkpll().clear_bit()
    });

    /* clkpll_pu_cp=0 */
    /* clkpll_pu_pfd=0 */
    /* clkpll_pu_fbdv=0 */
    /* clkpll_pu_postdiv=0 */
    pds.pu_rst_clkpll.modify(|_r, w| {w
        .clkpll_pu_cp().clear_bit()
        .clkpll_pu_pfd().clear_bit()
        .clkpll_pu_fbdv().clear_bit()
        .clkpll_pu_postdiv().clear_bit()
    });
}

/// Minimal implementation of power-on pll. Currently only allows external xtal
fn pds_power_on_pll(xtal: GlbPllXtalType) {
    let pds = unsafe { &*pac::PDS::ptr() };
    let mut delay = McycleDelay::new(system_core_clock_get());
    let freq = match xtal{
        Xtal24m => 24_000_000,
        Xtal32m => 32_000_000,
        Xtal38p4m => 38_400_000,
        Xtal40m => 40_000_000,
        Xtal26m => 26_000_000,
        _ => panic!()
    };

    /**************************/
    /* select PLL XTAL source */
    /**************************/
    pds_select_xtal_as_pll_ref();

    /*******************************************/
    /* PLL power down first, not indispensable */
    /*******************************************/
    /* power off PLL first, this step is not indispensable */
    pds_power_off_pll();

    /********************/
    /* PLL param config */
    /********************/

    if freq == 26_000_000 {
        pds.clkpll_cp.modify(|_r, w| unsafe {w
            .clkpll_icp_1u().bits(1)
            .clkpll_icp_5u().bits(0)
            .clkpll_int_frac_sw().set_bit()
        });
        pds.clkpll_rz.modify(|_r, w| unsafe {w
            .clkpll_c3().bits(2)
            .clkpll_cz().bits(2)
            .clkpll_rz().bits(5)
            .clkpll_r4_short().clear_bit()
        });
    } else {
        pds.clkpll_cp.modify(|_r, w| unsafe {w
            .clkpll_icp_1u().bits(0)
            .clkpll_icp_5u().bits(2)
            .clkpll_int_frac_sw().clear_bit()
        });
        pds.clkpll_rz.modify(|_r, w| unsafe {w
            .clkpll_c3().bits(3)
            .clkpll_cz().bits(1)
            .clkpll_rz().bits(1)
            .clkpll_r4_short().set_bit()
        });
    }

    pds.clkpll_top_ctrl.modify(|_r, w| unsafe {w
        .clkpll_postdiv().bits(0x14)
        .clkpll_refdiv_ratio().bits(2)
    });

    pds.clkpll_sdm.modify(|_r, w| unsafe {w
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

    pds.clkpll_fbdv.modify(|_r, w| unsafe {w
        .clkpll_sel_fb_clk().bits(1)
        .clkpll_sel_sample_clk().bits(1)
    });

    /*************************/
    /* PLL power up sequence */
    /*************************/
    pds.pu_rst_clkpll.modify(|_r, w| {w
        .pu_clkpll_sfreg().set_bit()
    });

    delay.try_delay_us(5).unwrap();

    pds.pu_rst_clkpll.modify(|_r, w| {w
        .pu_clkpll().set_bit()
    });

    pds.pu_rst_clkpll.modify(|_r, w| {w
        .clkpll_pu_cp().set_bit()
        .clkpll_pu_pfd().set_bit()
        .clkpll_pu_fbdv().set_bit()
        .clkpll_pu_postdiv().set_bit()
    });

    delay.try_delay_us(5).unwrap();

    pds.pu_rst_clkpll.modify(|_r, w| {w
        .clkpll_sdm_reset().set_bit()
    });

    delay.try_delay_us(1).unwrap();

    pds.pu_rst_clkpll.modify(|_r, w| {w
        .clkpll_reset_fbdv().set_bit()
    });

    delay.try_delay_us(2).unwrap();

    pds.pu_rst_clkpll.modify(|_r, w| {w
        .clkpll_reset_fbdv().clear_bit()
    });

    delay.try_delay_us(1).unwrap();

    pds.pu_rst_clkpll.modify(|_r, w| {w
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

fn hbn_set_root_clk_sel(sel: HbnRootClkType){
    let hbn = unsafe { &*pac::HBN::ptr() };
    hbn.hbn_glb.modify(|r,w| unsafe { w
        .hbn_root_clk_sel().bits(
            match sel {
                HbnRootClkType::Rc32m => 0b00u8,
                HbnRootClkType::Xtal => 0b01u8,
                HbnRootClkType::Pll => r.hbn_root_clk_sel().bits() as u8 | 0b10u8
            }
        )
    });
}

fn pds_enable_pll_all_clks(){
    let pds = unsafe { &*pac::PDS::ptr() };
    pds.clkpll_output_en.modify(|r, w| unsafe {w
        .bits(r.bits() | 0x1FF)
    });
}

/// Setup XTAL and PLL for system clock
/// TODO: finish clock init - some parts are hard-coded for 40Mhz XTAL + 160Mhz target clock
fn glb_set_system_clk_rc32(){
    /* reg_bclk_en = reg_hclk_en = reg_fclk_en = 1, cannot be zero */
    let glb = unsafe { &*pac::GLB::ptr() };
    glb.clk_cfg0.modify(|_, w| { w
        .reg_bclk_en().set_bit()
        .reg_hclk_en().set_bit()
        .reg_fclk_en().set_bit()
    });

     /* Before config XTAL and PLL ,make sure root clk is from RC32M */
    hbn_set_root_clk_sel(HbnRootClkType::Rc32m);

    glb.clk_cfg0.modify(|_,w| unsafe { w
        .reg_hclk_div().bits(0)
        .reg_bclk_div().bits(0)
    });

    // Update sysclock
    system_core_clock_set(32_000_000);

    /* Select PKA clock from hclk */
    glb.swrst_cfg2.modify(|_,w| { w
        .pka_clk_sel().clear_bit()
    });
}

/// Original code supported a bunch of configurations for core clock
/// There are probably uses for driving PLL using RC or using XTAL direct for root clock,
/// but it complicates something that is already sufficiently complex.
/// Settling for two configuration options for now:
///   - internal 32Mhz RC oscillator for sysclock
///   - XTAL driving PLL, sysclock frequencies of 48/80/120/160/192Mhz
pub fn glb_set_system_clk(xtal: GlbPllXtalType, clk: SysClk) {
    // Ensure clock is running off internal RC oscillator before changing anything else
    glb_set_system_clk_rc32();
    if xtal == GlbPllXtalType::None && clk == SysClk::Rc32m {
        // Target clock is the same as our safe default, so we don't have to do any more
        return;
    }
    // Configure XTAL, PLL and select it as clock source for fclk
    glb_set_system_clk_pll(clk);
}

fn glb_set_system_clk_pll(clk: SysClk) {
    /* reg_bclk_en = reg_hclk_en = reg_fclk_en = 1, cannot be zero */
    let glb = unsafe { &*pac::GLB::ptr() };
    // Power up the external crystal before we start up the PLL
    aon_power_on_xtal();

    /* always power up PLL and enable all PLL clock output */
    pds_power_on_pll(GlbPllXtalType::Xtal40m);

    let mut delay = McycleDelay::new(system_core_clock_get());
    delay.try_delay_us(55).unwrap();

    pds_enable_pll_all_clks();
    
    /* reg_pll_en = 1, cannot be zero */
    glb.clk_cfg0.modify(|_, w| {w
        .reg_pll_en().set_bit()
    });

    /* select pll output clock before select root clock */
    // sets to clkFreq-GLB_SYS_CLK_PLL48M, where PLL160M is 2 more than PLL48M
    // Doing this with a match seems more Rusty
    glb.clk_cfg0.modify(|_, w| unsafe {w
        .reg_pll_sel().bits(
            match clk {
                SysClk::Pll48m => 0,
                SysClk::Pll120m => 1,
                SysClk::Pll160m => 2,
                SysClk::Pll192m => 3,
                _ => {panic!()}
            }
        )
    });

    let target_core_clk = match clk{
        SysClk::Rc32m => 0,
        SysClk::Xtal => 0,
        SysClk::Pll48m => 48_000_000,
        SysClk::Pll120m => 120_000_000,
        SysClk::Pll160m => 160_000_000,
        SysClk::Pll192m => 192_000_000,
    };

    if target_core_clk > 48_000_000 {
        glb_set_system_clk_div(0, 1);
    }

    if target_core_clk > 120_000_000 {
        let l1c = unsafe { &*pac::L1C::ptr() };
        l1c.l1c_config.modify(|r, w| {w
            .irom_2t_access().set_bit()
        });
    }
    if target_core_clk > 0 {
        hbn_set_root_clk_sel(HbnRootClkType::Pll);
        system_core_clock_set(target_core_clk);
    }

    // GLB_CLK_SET_DUMMY_WAIT;
    // This was a set of 8 NOP instructions. at 32mhz, this is 1/4 of a us
    // but since we just changed our clock source, we'll wait the equivalent of 1us worth
    // of clocks at 160Mhz (this *should* be much longer than necessary)
    let mut delay = McycleDelay::new(system_core_clock_get());
    delay.try_delay_us(1).unwrap();

    /* select PKA clock from 120M since we power up PLL */
    // NOTE: This isn't documented in the datasheet!
    // GLB_Set_PKA_CLK_Sel(GLB_PKA_CLK_PLL120M);
    glb.swrst_cfg2.write(|w| { w
        .pka_clk_sel().set_bit()
    });
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

        let target_clksrc = HbnRootClkType::Pll;
        let pll_xtal = GlbPllXtalType::Xtal40m;
        let target_sys_ck = SysClk::Pll160m;

        Clocks {
            target_clksrc,
            pll_xtal,
            target_sys_ck,
            uart_clk_div,
        }
    }
}
