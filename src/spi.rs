/*!
  # Serial Peripheral Interface
  To construct the SPI instances, use the `Spi::spi` function.
  The pin parameter is a tuple containing `(miso, mosi, cs, sck)` which should be configured via `into_spi_miso, into_spi_mosi, into_spi_ss, into_spi_sclk`.

  CS is optional - so you can also pass a tuple containing `(miso, mosi, sck)`
  ## Initialisation example
  ```rust
    let miso = parts.pin4.into_spi_miso();
    let mosi = parts.pin5.into_spi_mosi();
    let ss = parts.pin2.into_spi_ss();
    let sclk = parts.pin3.into_spi_sclk();

    let mut spi = hal::spi::Spi::spi(
        dp.SPI,
        (miso, mosi, ss, sclk),
        embedded_hal::spi::MODE_0,
        8_000_000u32.Hz(),
        clocks,
    );
  ```
*/

use bl602_pac::SPI;
pub use embedded_hal::spi::{FullDuplex, Mode};
use embedded_time::rate::Hertz;

use crate::pac;

use crate::clock::Clocks;

/// SPI error
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Overrun occurred
    Overrun,
    /// Mode fault occurred
    ModeFault,
    /// CRC error
    Crc,
}

/// The bit format to send the data in
#[derive(Debug, Clone, Copy)]
pub enum SpiBitFormat {
    /// Least significant bit first
    LsbFirst,
    /// Most significant bit first
    MsbFirst,
}

/// MISO pins - DO NOT IMPLEMENT THIS TRAIT
pub unsafe trait MisoPin<SPI> {}

/// MOSI pins - DO NOT IMPLEMENT THIS TRAIT
pub unsafe trait MosiPin<SPI> {}

/// SS pins - DO NOT IMPLEMENT THIS TRAIT
pub unsafe trait SsPin<SPI> {}

/// SCLK pins - DO NOT IMPLEMENT THIS TRAIT
pub unsafe trait SclkPin<SPI> {}

/// Spi pins - DO NOT IMPLEMENT THIS TRAIT
pub unsafe trait Pins<SPI> {}

unsafe impl<MODE> MisoPin<pac::SPI> for crate::gpio::Pin0<MODE> {}
unsafe impl<MODE> MosiPin<pac::SPI> for crate::gpio::Pin1<MODE> {}
unsafe impl<MODE> SsPin<pac::SPI> for crate::gpio::Pin2<MODE> {}
unsafe impl<MODE> SclkPin<pac::SPI> for crate::gpio::Pin3<MODE> {}
unsafe impl<MODE> MisoPin<pac::SPI> for crate::gpio::Pin4<MODE> {}
unsafe impl<MODE> MosiPin<pac::SPI> for crate::gpio::Pin5<MODE> {}
unsafe impl<MODE> SsPin<pac::SPI> for crate::gpio::Pin6<MODE> {}
unsafe impl<MODE> SclkPin<pac::SPI> for crate::gpio::Pin7<MODE> {}
unsafe impl<MODE> MisoPin<pac::SPI> for crate::gpio::Pin8<MODE> {}
unsafe impl<MODE> MosiPin<pac::SPI> for crate::gpio::Pin9<MODE> {}
unsafe impl<MODE> SsPin<pac::SPI> for crate::gpio::Pin10<MODE> {}
unsafe impl<MODE> SclkPin<pac::SPI> for crate::gpio::Pin11<MODE> {}
unsafe impl<MODE> MisoPin<pac::SPI> for crate::gpio::Pin12<MODE> {}
unsafe impl<MODE> MosiPin<pac::SPI> for crate::gpio::Pin13<MODE> {}
unsafe impl<MODE> SsPin<pac::SPI> for crate::gpio::Pin14<MODE> {}
unsafe impl<MODE> SclkPin<pac::SPI> for crate::gpio::Pin15<MODE> {}
unsafe impl<MODE> MisoPin<pac::SPI> for crate::gpio::Pin16<MODE> {}
unsafe impl<MODE> MosiPin<pac::SPI> for crate::gpio::Pin17<MODE> {}
unsafe impl<MODE> SsPin<pac::SPI> for crate::gpio::Pin18<MODE> {}
unsafe impl<MODE> SclkPin<pac::SPI> for crate::gpio::Pin19<MODE> {}
unsafe impl<MODE> MisoPin<pac::SPI> for crate::gpio::Pin20<MODE> {}
unsafe impl<MODE> MosiPin<pac::SPI> for crate::gpio::Pin21<MODE> {}
unsafe impl<MODE> SsPin<pac::SPI> for crate::gpio::Pin22<MODE> {}

unsafe impl<MISO, MOSI, SS, SCLK> Pins<SPI> for (MISO, MOSI, SS, SCLK)
where
    MISO: MisoPin<SPI>,
    MOSI: MosiPin<SPI>,
    SS: SsPin<SPI>,
    SCLK: SclkPin<SPI>,
{
}

unsafe impl<MISO, MOSI, SCLK> Pins<SPI> for (MISO, MOSI, SCLK)
where
    MISO: MisoPin<SPI>,
    MOSI: MosiPin<SPI>,
    SCLK: SclkPin<SPI>,
{
}

/// A Serial Peripheral Interface
pub struct Spi<SPI, PINS> {
    spi: SPI,
    pins: PINS,
}

impl<PINS> Spi<pac::SPI, PINS>
where
    PINS: Pins<pac::SPI>,
{
    /**
      Constructs an SPI instance in 8bit dataframe mode.
      The pin parameter tuple (miso, mosi, cs, sck) needs to be configured accordingly.
      You can also omit `cs` to have manual control over `cs`.
    */
    pub fn spi(spi: SPI, pins: PINS, mode: Mode, freq: Hertz<u32>, clocks: Clocks) -> Self
    where
        PINS: Pins<pac::SPI>,
    {
        let glb = unsafe { &*pac::GLB::ptr() };

        glb.glb_parm
            .modify(|_r, w| w.reg_spi_0_master_mode().set_bit());

        let len = clocks.spi_clk().0 / freq.0;
        if len > 0b11111 || len == 0 {
            panic!("Cannot reach the desired SPI frequency");
        }

        // TODO the measured frequency of SCLK is half of what I configure
        let len = ((len - 1) & 0b11111) as u8;
        spi.spi_prd_0.modify(|_r, w| unsafe {
            w.cr_spi_prd_s()
                .bits(len)
                .cr_spi_prd_p()
                .bits(len)
                .cr_spi_prd_d_ph_0()
                .bits(len)
                .cr_spi_prd_d_ph_1()
                .bits(len)
        });

        spi.spi_prd_1
            .modify(|_r, w| unsafe { w.cr_spi_prd_i().bits(len) });

        spi.spi_config.modify(|_, w| unsafe {
            w.cr_spi_sclk_pol()
                .bit(match mode.polarity {
                    embedded_hal::spi::Polarity::IdleLow => false,
                    embedded_hal::spi::Polarity::IdleHigh => true,
                })
                .cr_spi_sclk_ph()
                .bit(match mode.phase {
                    embedded_hal::spi::Phase::CaptureOnFirstTransition => false,
                    embedded_hal::spi::Phase::CaptureOnSecondTransition => true,
                })
                .cr_spi_m_cont_en()
                .clear_bit() // disable cont mode
                .cr_spi_frame_size()
                .bits(0) // 8 bit frames
                .cr_spi_s_en()
                .clear_bit() // not slave
                .cr_spi_m_en()
                .set_bit() // master
        });

        Spi { spi, pins }
    }

    pub fn release(self) -> (pac::SPI, PINS) {
        (self.spi, self.pins)
    }

    /// Select which frame format is used for data transfers
    pub fn bit_format(&mut self, format: SpiBitFormat) {
        match format {
            SpiBitFormat::LsbFirst => self
                .spi
                .spi_config
                .modify(|_, w| w.cr_spi_bit_inv().set_bit()),
            SpiBitFormat::MsbFirst => self
                .spi
                .spi_config
                .modify(|_, w| w.cr_spi_bit_inv().clear_bit()),
        }
    }

    /// Clear FIFOs
    pub fn clear_fifo(&mut self) {
        self.spi
            .spi_fifo_config_0
            .write(|w| w.rx_fifo_clr().set_bit().tx_fifo_clr().set_bit());
    }
}

impl<PINS> FullDuplex<u8> for Spi<pac::SPI, PINS>
where
    PINS: Pins<pac::SPI>,
{
    type Error = Error;

    fn try_read(&mut self) -> nb::Result<u8, Error> {
        if self.spi.spi_fifo_config_1.read().rx_fifo_cnt().bits() == 0 {
            Err(nb::Error::WouldBlock)
        } else {
            Ok((self.spi.spi_fifo_rdata.read().bits() & 0xff) as u8)
        }
    }

    fn try_send(&mut self, data: u8) -> nb::Result<(), Self::Error> {
        if self.spi.spi_fifo_config_1.read().tx_fifo_cnt().bits() == 0 {
            Err(nb::Error::WouldBlock)
        } else {
            self.spi
                .spi_fifo_wdata
                .write(|w| unsafe { w.bits(data as u32) });

            Ok(())
        }
    }
}

impl<PINS> embedded_hal::blocking::spi::transfer::Default<u8> for Spi<pac::SPI, PINS> where
    PINS: Pins<pac::SPI>
{
}

impl<PINS> embedded_hal::blocking::spi::write::Default<u8> for Spi<pac::SPI, PINS> where
    PINS: Pins<pac::SPI>
{
}

impl<PINS> embedded_hal::blocking::spi::write_iter::Default<u8> for Spi<pac::SPI, PINS> where
    PINS: Pins<pac::SPI>
{
}

impl<PINS> embedded_hal::blocking::spi::transactional::Default<u8> for Spi<pac::SPI, PINS> where
    PINS: Pins<pac::SPI>
{
}
