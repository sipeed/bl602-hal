//! SoC clock configuration

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
//
// Currently implemented clock tree configuration options:
//   - internal 32Mhz RC oscillator for sysclock
//   - XTAL driving PLL, sysclock frequencies of 48/80/120/160/192Mhz
//   - UART using PLL if sysclock is using PLL

use crate::delay::*;
use crate::gpio::ClkCfg;
use crate::pac;
use core::num::NonZeroU32;
use embedded_hal::blocking::delay::DelayUs;
use embedded_time::rate::{Extensions, Hertz};

/// Internal high-speed RC oscillator frequency
pub const RC32M: u32 = 32_000_000;
/// UART peripheral clock frequency when PLL selected
pub const UART_PLL_FREQ: u32 = 160_000_000;

#[derive(PartialEq, Copy, Clone)]
#[repr(u32)]
pub enum SysclkFreq {
    Rc32Mhz = 32_000_000,
    Pll48Mhz = 48_000_000,
    Pll120Mhz = 120_000_000,
    Pll160Mhz = 160_000_000,
}

/// Frozen clock frequencies
///
/// The existance of this value indicates that the clock configuration can no longer be changed
#[derive(Clone, Copy)]
pub struct Clocks {
    sysclk: Hertz,
    uart_clk: Hertz,
    spi_clk: Hertz,
    _xtal_freq: Option<Hertz>,
    pll_enable: bool,
}

impl Clocks {
    pub fn new() -> Self {
        Clocks {
            sysclk: Hertz(RC32M),
            uart_clk: Hertz(RC32M),
            spi_clk: Hertz(RC32M),
            _xtal_freq: None,
            pll_enable: false,
        }
    }

    pub fn sysclk(&self) -> Hertz {
        self.sysclk
    }

    pub fn pll_enable(&self) -> bool {
        self.pll_enable
    }

    pub const fn uart_clk(&self) -> Hertz {
        self.uart_clk
    }

    pub const fn spi_clk(&self) -> Hertz {
        self.spi_clk
    }
}

impl Default for Clocks {
    fn default() -> Self {
        Self::new()
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
    target_spi_clk: Option<NonZeroU32>,
    target_uart_clk: Option<NonZeroU32>,
    pll_xtal_freq: Option<u32>,
    sysclk: SysclkFreq,
}

impl Strict {
    /// Create a strict configurator
    pub fn new() -> Self {
        Strict {
            target_spi_clk: None,
            target_uart_clk: None,
            pll_xtal_freq: None,
            sysclk: SysclkFreq::Rc32Mhz,
        }
    }

    /// Sets the desired frequency for the SPI-CLK clock
    pub fn spi_clk(mut self, freq: impl Into<Hertz>) -> Self {
        let freq_hz = freq.into().0;

        self.target_spi_clk = NonZeroU32::new(freq_hz);

        self
    }

    /// Sets the desired frequency for the UART-CLK clock
    pub fn uart_clk(mut self, freq: impl Into<Hertz>) -> Self {
        let freq_hz = freq.into().0;

        self.target_uart_clk = NonZeroU32::new(freq_hz);

        self
    }

    /// Enables PLL clock source, using external XTAL frequency provided
    pub fn use_pll(mut self, freq: impl Into<Hertz>) -> Self {
        self.pll_xtal_freq = Some(freq.into().0);

        self
    }

    /// Set the system clock frequency (fclk/hclk)
    ///
    /// Supported frequencies:
    ///   `32_000_000`, `48_000_000`, `80_000_000`, `120_000_000`, `160_000_000`
    pub fn sys_clk(mut self, freq: SysclkFreq) -> Self {
        self.sysclk = freq;

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
    pub fn freeze(self, _clk_cfg: &mut ClkCfg) -> Clocks {
        // Default to not using the PLL, and selecting the internal RC oscillator if nothing selected
        let pll_xtal_freq = self.pll_xtal_freq.unwrap_or(0);
        let pll_enabled = pll_xtal_freq != 0;
        let sysclk = self.sysclk;

        // If sysclk isn't 32Mhz but PLL isn't enabled, panic
        assert!(pll_enabled || sysclk == SysclkFreq::Rc32Mhz);

        // If PLL is available we'll be using the PLL_160Mhz clock, otherwise sysclk
        let uart_clk_src = if pll_enabled {
            UART_PLL_FREQ
        } else {
            sysclk as u32
        };

        // UART config
        let uart_clk = self
            .target_uart_clk
            .map(|f| f.get())
            .unwrap_or(uart_clk_src as u32);

        let uart_clk_div = {
            let ans = uart_clk_src / uart_clk;

            if !(1..=7).contains(&ans) || ans * uart_clk != uart_clk_src {
                panic!("unreachable uart_clk")
            }

            ans as u8
        };

        // Enable system clock, PLL + crystal if required
        match sysclk {
            SysclkFreq::Rc32Mhz => glb_set_system_clk_rc32(),
            _ => glb_set_system_clk_pll(sysclk as u32, pll_xtal_freq),
        };

        // If PLL is enabled, use that for the UART base clock
        // Otherwise, use sysclk as the UART clock
        unsafe { &*pac::HBN::ptr() }
            .hbn_glb
            .modify(|_, w| w.hbn_uart_clk_sel().bit(pll_enabled));

        // Write UART clock divider
        unsafe { &*pac::GLB::ptr() }.clk_cfg2.modify(|_, w| unsafe {
            w.uart_clk_div()
                .bits(uart_clk_div - 1_u8)
                .uart_clk_en()
                .set_bit()
        });

        // SPI config
        let spi_clk = self
            .target_spi_clk
            .map(|f| f.get())
            .unwrap_or(32_000_000u32);

        // SPI Clock Divider (BUS_CLK/(N+1)), default BUS_CLK/4
        let bus_clock = calculate_bus_clock();
        let spi_clk_div = bus_clock.0 / spi_clk;

        if spi_clk_div == 0 || spi_clk_div > 0b100000 {
            panic!("Unreachable SPI_CLK");
        }

        let spi_clk_div = ((spi_clk_div - 1) & 0b11111) as u8;
        // Write SPI clock divider
        unsafe { &*pac::GLB::ptr() }
            .clk_cfg3
            .modify(|_, w| unsafe { w.spi_clk_en().set_bit().spi_clk_div().bits(spi_clk_div) });

        Clocks {
            sysclk: Hertz(sysclk as u32),
            uart_clk: Hertz(uart_clk),
            spi_clk: Hertz(spi_clk),
            _xtal_freq: Some(Hertz(pll_xtal_freq)),
            pll_enable: pll_enabled,
        }
    }
}

impl Default for Strict {
    fn default() -> Self {
        Self::new()
    }
}

/// Gets the current bus clock rate
fn calculate_bus_clock() -> Hertz {
    let root_clk_sel = unsafe { &*pac::GLB::ptr() }
        .clk_cfg0
        .read()
        .hbn_root_clk_sel()
        .bits();
    let pll_clk_sel = unsafe { &*pac::GLB::ptr() }
        .clk_cfg0
        .read()
        .reg_pll_sel()
        .bits();
    let hclk_div = unsafe { &*pac::GLB::ptr() }
        .clk_cfg0
        .read()
        .reg_hclk_div()
        .bits();
    let bclk_div = unsafe { &*pac::GLB::ptr() }
        .clk_cfg0
        .read()
        .reg_bclk_div()
        .bits();

    let root = match root_clk_sel {
        0 => 32_000_000_u32.Hz(),
        1 => 40_000_000_u32.Hz(),
        _ => match pll_clk_sel {
            0 => 48_000_000_u32.Hz(),
            1 => 120_000_000_u32.Hz(),
            2 => 160_000_000_u32.Hz(),
            _ => 192_000_000_u32.Hz(),
        },
    };

    let root = root / (hclk_div as u32 + 1) / (bclk_div as u32 + 1);
    root.into()
}

/// Sets the system clock in the (undocumented) system_core_clock register
fn system_core_clock_set(value: u32) {
    unsafe { &*pac::HBN::ptr() }
        .hbn_rsv2
        .write(|w| unsafe { w.bits(value) })
}

/// Gets the system clock in the (undocumented) system_core_clock register
fn system_core_clock_get() -> u32 {
    unsafe { &*pac::HBN::ptr() }.hbn_rsv2.read().bits()
}

fn glb_set_system_clk_div(hclkdiv: u8, bclkdiv: u8) {
    // recommended: fclk<=160MHz, bclk<=80MHz
    // fclk is determined by hclk_div (strange), which then feeds into bclk, hclk and uartclk
    // glb_reg_bclk_dis isn't in the SVD file, so it isn't generated through svd2rust
    // It's only used by this function so define it as a local variable
    let glb_reg_bclk_dis = 0x4000_0FFC as *mut u32;

    unsafe { &*pac::GLB::ptr() }
        .clk_cfg0
        .modify(|_, w| unsafe { w.reg_hclk_div().bits(hclkdiv).reg_bclk_div().bits(bclkdiv) });
    unsafe { glb_reg_bclk_dis.write_volatile(1) };
    unsafe { glb_reg_bclk_dis.write_volatile(0) };

    let currclock = system_core_clock_get();

    system_core_clock_set(currclock / (hclkdiv as u32 + 1));

    let mut delay = McycleDelay::new(system_core_clock_get());

    // This delay used to be 8 NOPS (1/4 us). Might need to be replaced again.
    delay.try_delay_us(1).unwrap();

    unsafe { &*pac::GLB::ptr() }
        .clk_cfg0
        .modify(|_, w| w.reg_hclk_en().set_bit().reg_bclk_en().set_bit());

    delay.try_delay_us(1).unwrap();
}

// This is a reference implementation of `PDS_Select_XTAL_As_PLL_Ref`.
// It is currently unused.
#[allow(dead_code)]
fn pds_select_xtal_as_pll_ref() {
    unsafe { &*pac::PDS::ptr() }.clkpll_top_ctrl.modify(|_, w| {
        w.clkpll_refclk_sel()
            .set_bit()
            .clkpll_xtal_rc32m_sel()
            .clear_bit()
    });
}

// This is a reference implementation of `PDS_Power_Off_PLL`.
// It is currently unused.
#[allow(dead_code)]
fn pds_power_off_pll() {
    unsafe { &*pac::PDS::ptr() }
        .pu_rst_clkpll
        .modify(|_, w| w.pu_clkpll_sfreg().clear_bit().pu_clkpll().clear_bit());

    unsafe { &*pac::PDS::ptr() }.pu_rst_clkpll.modify(|_, w| {
        w.clkpll_pu_cp()
            .clear_bit()
            .clkpll_pu_pfd()
            .clear_bit()
            .clkpll_pu_fbdv()
            .clear_bit()
            .clkpll_pu_postdiv()
            .clear_bit()
    });
}

/// Power up PLL using C function built into on-chip ROM
// Powering on the PLL means powering *OFF* the PLL if it's already running.
// If we're running from flash, this causes a lockup. Running from RAM or ROM is
// okay though. So there's probably some steps we need to take to do this safely,
// but the flash peripheral is not documented yet.
// The easiest solution is to use the C function built into the ROM to do the change.
#[inline]
fn pds_power_on_pll_rom(freq: u32) {
    // Lookup table for ROM function addresses is at 0x21010800
    // index in the table for RomDriver_PDS_Power_On_PLL is 88
    // each entry is a pointer, size of each entry is 4 bytes (sizeof usize)
    // we can can use core::ptr::offset to calculate the true offset for us.
    // In my ROM, the address of rom_pds_power_on_pll is 0x2101_4ACE,
    // this line is commented out in case this is not true for others
    let rom_function_table_base = (0x2101_0800) as *mut usize;
    let power_on_pll_entry = unsafe { rom_function_table_base.offset(88) };
    let power_on_pll_addr = unsafe { power_on_pll_entry.read_volatile() };
    //assert_eq!(power_on_pll_addr, 0x2101_4ACE);

    let romdriver_pds_power_on_pll = unsafe {
        core::mem::transmute::<*const (), extern "C" fn(usize) -> usize>(
            power_on_pll_addr as *const (),
        )
    };
    let xtal_src = match freq {
        24_000_000 => 1,
        32_000_000 => 2,
        38_400_000 => 3,
        40_000_000 => 4,
        26_000_000 => 5,
        _ => panic!("Unsupported PLL clock source"),
    };

    // 0 == success, 1 == failure, 2 == timeout
    let pll_success = romdriver_pds_power_on_pll(xtal_src);
    assert_eq!(pll_success, 0);
}

/// Minimal implementation of power-on pll. Currently only allows external xtal
/// Fails when running from flash - use the pds_power_on_pll_rom for now
// TODO: work out how to safely power off PLL while running from flash
//
// This is a reference implementation of `PDS_Power_On_PLL`.
// It is currently unused.
#[allow(dead_code)]
fn pds_power_on_pll(freq: u32) {
    let pds = unsafe { &*pac::PDS::ptr() };
    let mut delay = McycleDelay::new(system_core_clock_get());

    pds_select_xtal_as_pll_ref();

    // power off PLL first - this step is required
    pds_power_off_pll();

    // PLL param config
    if freq == 26_000_000 {
        pds.clkpll_cp.modify(|_, w| unsafe {
            w.clkpll_icp_1u()
                .bits(1)
                .clkpll_icp_5u()
                .bits(0)
                .clkpll_int_frac_sw()
                .set_bit()
        });

        pds.clkpll_rz.modify(|_, w| unsafe {
            w.clkpll_c3()
                .bits(2)
                .clkpll_cz()
                .bits(2)
                .clkpll_rz()
                .bits(5)
                .clkpll_r4_short()
                .clear_bit()
        });
    } else {
        pds.clkpll_cp.modify(|_, w| unsafe {
            w.clkpll_icp_1u()
                .bits(0)
                .clkpll_icp_5u()
                .bits(2)
                .clkpll_int_frac_sw()
                .clear_bit()
        });

        pds.clkpll_rz.modify(|_, w| unsafe {
            w.clkpll_c3()
                .bits(3)
                .clkpll_cz()
                .bits(1)
                .clkpll_rz()
                .bits(1)
                .clkpll_r4_short()
                .set_bit()
        });
    }

    pds.clkpll_top_ctrl
        .modify(|_, w| unsafe { w.clkpll_postdiv().bits(0x14).clkpll_refdiv_ratio().bits(2) });

    pds.clkpll_sdm.modify(|_, w| unsafe {
        w.clkpll_sdmin().bits(match freq {
            24_000_000 => 0x50_0000,
            32_000_000 => 0x3C_0000,
            38_400_000 => 0x32_0000,
            40_000_000 => 0x30_0000,
            26_000_000 => 0x49_D39D,
            _ => panic!(),
        })
    });

    pds.clkpll_fbdv.modify(|_, w| unsafe {
        w.clkpll_sel_fb_clk()
            .bits(1)
            .clkpll_sel_sample_clk()
            .bits(1)
    });

    /*************************/
    /* PLL power up sequence */
    /*************************/
    pds.pu_rst_clkpll
        .modify(|_, w| w.pu_clkpll_sfreg().set_bit());

    delay.try_delay_us(5).unwrap();

    pds.pu_rst_clkpll.modify(|_, w| w.pu_clkpll().set_bit());

    pds.pu_rst_clkpll.modify(|_, w| {
        w.clkpll_pu_cp()
            .set_bit()
            .clkpll_pu_pfd()
            .set_bit()
            .clkpll_pu_fbdv()
            .set_bit()
            .clkpll_pu_postdiv()
            .set_bit()
    });

    delay.try_delay_us(5).unwrap();

    pds.pu_rst_clkpll
        .modify(|_, w| w.clkpll_sdm_reset().set_bit());

    delay.try_delay_us(1).unwrap();

    pds.pu_rst_clkpll
        .modify(|_, w| w.clkpll_reset_fbdv().set_bit());

    delay.try_delay_us(2).unwrap();

    pds.pu_rst_clkpll
        .modify(|_, w| w.clkpll_reset_fbdv().clear_bit());

    delay.try_delay_us(1).unwrap();

    pds.pu_rst_clkpll
        .modify(|_, w| w.clkpll_sdm_reset().clear_bit());
}

fn aon_power_on_xtal() -> Result<(), &'static str> {
    unsafe { &*pac::AON::ptr() }
        .rf_top_aon
        .modify(|_, w| w.pu_xtal_aon().set_bit().pu_xtal_buf_aon().set_bit());

    let mut delaysrc = McycleDelay::new(system_core_clock_get());
    let mut timeout: u32 = 0;

    delaysrc.try_delay_us(10).unwrap();

    while unsafe { &*pac::AON::ptr() }
        .tsen
        .read()
        .xtal_rdy()
        .bit_is_clear()
        && timeout < 120
    {
        delaysrc.try_delay_us(10).unwrap();
        timeout += 1;
    }

    if timeout == 120 {
        Err("timeout occured")
    } else {
        Ok(())
    }
}

fn hbn_set_root_clk_sel_pll() {
    unsafe { &*pac::HBN::ptr() }.hbn_glb.modify(|r, w| unsafe {
        w.hbn_root_clk_sel()
            .bits(r.hbn_root_clk_sel().bits() as u8 | 0b10u8)
    });
}

fn hbn_set_root_clk_sel_rc32() {
    unsafe { &*pac::HBN::ptr() }
        .hbn_glb
        .modify(|_, w| unsafe { w.hbn_root_clk_sel().bits(0b00u8) });
}

fn pds_enable_pll_all_clks() {
    unsafe { &*pac::PDS::ptr() }
        .clkpll_output_en
        .modify(|r, w| unsafe { w.bits(r.bits() | 0x1FF) });
}

/// Sets the system clock to use the internal 32Mhz RC oscillator
fn glb_set_system_clk_rc32() {
    // reg_bclk_en = reg_hclk_en = reg_fclk_en = 1, cannot be zero
    unsafe { &*pac::GLB::ptr() }.clk_cfg0.modify(|_, w| {
        w.reg_bclk_en()
            .set_bit()
            .reg_hclk_en()
            .set_bit()
            .reg_fclk_en()
            .set_bit()
    });

    // Before config XTAL and PLL, make sure root clk is from RC32M
    hbn_set_root_clk_sel_rc32();

    glb_set_system_clk_div(0, 0);

    // Update sysclock
    system_core_clock_set(RC32M);

    // Select PKA clock from hclk
    unsafe { &*pac::GLB::ptr() }
        .swrst_cfg2
        .modify(|_, w| w.pka_clk_sel().clear_bit());
}

/// Sets the system clock to use the PLL with external crystal
fn glb_set_system_clk_pll(target_core_clk: u32, xtal_freq: u32) {
    // Ensure clock is running off internal RC oscillator before changing anything else
    glb_set_system_clk_rc32();

    // Power up the external crystal before we start up the PLL
    aon_power_on_xtal().unwrap();

    // Power up PLL and enable all PLL clock output
    pds_power_on_pll_rom(xtal_freq);

    let mut delay = McycleDelay::new(system_core_clock_get());
    delay.try_delay_us(55).unwrap();

    pds_enable_pll_all_clks();

    // Enable PLL
    unsafe { &*pac::GLB::ptr() }
        .clk_cfg0
        .modify(|_, w| w.reg_pll_en().set_bit());

    // select which pll output clock to use before
    // selecting root clock via HBN_Set_ROOT_CLK_Sel
    // Note that 192Mhz is out of spec
    unsafe { &*pac::GLB::ptr() }.clk_cfg0.modify(|_, w| unsafe {
        w.reg_pll_sel().bits(match target_core_clk {
            48_000_000 => 0,
            120_000_000 => 1,
            160_000_000 => 2,
            192_000_000 => 3,
            _ => {
                panic!()
            }
        })
    });

    // Keep bclk <= 80MHz
    if target_core_clk > 48_000_000 {
        glb_set_system_clk_div(0, 1);
    }

    // For frequencies above 120Mhz we need 2 clocks to access internal rom
    if target_core_clk > 120_000_000 {
        unsafe { &*pac::L1C::ptr() }
            .l1c_config
            .modify(|_, w| w.irom_2t_access().set_bit());
    }

    hbn_set_root_clk_sel_pll();
    system_core_clock_set(target_core_clk);

    let mut delay = McycleDelay::new(system_core_clock_get());

    // This delay used to be 8 NOPS (1/4 us). (GLB_CLK_SET_DUMMY_WAIT) Might need to be replaced again.
    delay.try_delay_us(1).unwrap();

    // use 120Mhz PLL tap for PKA clock since we're using PLL
    // NOTE: This isn't documented in the datasheet!
    unsafe { &*pac::GLB::ptr() }
        .swrst_cfg2
        .modify(|_, w| w.pka_clk_sel().set_bit());
}
