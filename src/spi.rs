/*!
  # Serial Peripheral Interface
  To construct the SPI instances, use the `Spi::new` function.
  The pin parameter is a tuple containing `(miso, mosi, cs, sck)` which should be configured via `into_spi_miso, into_spi_mosi, into_spi_ss, into_spi_sclk`.

  CS is optional - so you can also pass a tuple containing `(miso, mosi, sck)`
  ## Initialisation example
  ```rust
    let miso = parts.pin4.into_spi_miso();
    let mosi = parts.pin5.into_spi_mosi();
    let ss = parts.pin2.into_spi_ss();
    let sclk = parts.pin3.into_spi_sclk();

    let mut spi = hal::spi::Spi::new(
        dp.SPI,
        (miso, mosi, ss, sclk),
        embedded_hal::spi::MODE_0,
        8_000_000u32.Hz(),
        clocks,
    );
  ```
*/

use bl602_pac::SPI;
pub use embedded_hal::spi::Mode;
use embedded_hal_nb;
use embedded_hal_zero::spi::FullDuplex as FullDuplexZero;
use embedded_time::rate::Hertz;

use crate::pac;

use crate::clock::Clocks;

/// SPI error
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Rx overflow occurred
    RxOverflow,
    /// Rx underflow occurred
    RxUnderflow,
    /// Tx overflow occurred
    TxOverflow,
    /// Tx underflow occurred
    TxUnderflow,
}

impl embedded_hal_nb::spi::Error for Error {
    fn kind(&self) -> embedded_hal_nb::spi::ErrorKind {
        match self {
            Self::RxOverflow => embedded_hal_nb::spi::ErrorKind::Overrun,
            Self::TxOverflow => embedded_hal_nb::spi::ErrorKind::Overrun,
            Self::RxUnderflow => embedded_hal_nb::spi::ErrorKind::Overrun,
            Self::TxUnderflow => embedded_hal_nb::spi::ErrorKind::Overrun,
        }
    }
}

/// The bit format to send the data in
#[derive(Debug, Clone, Copy)]
pub enum SpiBitFormat {
    /// Least significant bit first
    LsbFirst,
    /// Most significant bit first
    MsbFirst,
}

/// MISO pins
pub trait MisoPin<SPI>: private::Sealed {}

/// MOSI pins
pub trait MosiPin<SPI>: private::Sealed {}

/// SS pins
pub trait SsPin<SPI>: private::Sealed {}

/// SCLK pins
pub trait SclkPin<SPI>: private::Sealed {}

/// Spi pins
pub trait Pins<SPI>: private::Sealed {}

impl<MODE> MisoPin<pac::SPI> for crate::gpio::Pin0<MODE> {}
impl<MODE> MosiPin<pac::SPI> for crate::gpio::Pin1<MODE> {}
impl<MODE> SsPin<pac::SPI> for crate::gpio::Pin2<MODE> {}
impl<MODE> SclkPin<pac::SPI> for crate::gpio::Pin3<MODE> {}
impl<MODE> MisoPin<pac::SPI> for crate::gpio::Pin4<MODE> {}
impl<MODE> MosiPin<pac::SPI> for crate::gpio::Pin5<MODE> {}
impl<MODE> SsPin<pac::SPI> for crate::gpio::Pin6<MODE> {}
impl<MODE> SclkPin<pac::SPI> for crate::gpio::Pin7<MODE> {}
impl<MODE> MisoPin<pac::SPI> for crate::gpio::Pin8<MODE> {}
impl<MODE> MosiPin<pac::SPI> for crate::gpio::Pin9<MODE> {}
impl<MODE> SsPin<pac::SPI> for crate::gpio::Pin10<MODE> {}
impl<MODE> SclkPin<pac::SPI> for crate::gpio::Pin11<MODE> {}
impl<MODE> MisoPin<pac::SPI> for crate::gpio::Pin12<MODE> {}
impl<MODE> MosiPin<pac::SPI> for crate::gpio::Pin13<MODE> {}
impl<MODE> SsPin<pac::SPI> for crate::gpio::Pin14<MODE> {}
impl<MODE> SclkPin<pac::SPI> for crate::gpio::Pin15<MODE> {}
impl<MODE> MisoPin<pac::SPI> for crate::gpio::Pin16<MODE> {}
impl<MODE> MosiPin<pac::SPI> for crate::gpio::Pin17<MODE> {}
impl<MODE> SsPin<pac::SPI> for crate::gpio::Pin18<MODE> {}
impl<MODE> SclkPin<pac::SPI> for crate::gpio::Pin19<MODE> {}
impl<MODE> MisoPin<pac::SPI> for crate::gpio::Pin20<MODE> {}
impl<MODE> MosiPin<pac::SPI> for crate::gpio::Pin21<MODE> {}
impl<MODE> SsPin<pac::SPI> for crate::gpio::Pin22<MODE> {}

impl<MISO, MOSI, SS, SCLK> Pins<SPI> for (MISO, MOSI, SS, SCLK)
where
    MISO: MisoPin<SPI>,
    MOSI: MosiPin<SPI>,
    SS: SsPin<SPI>,
    SCLK: SclkPin<SPI>,
{
}

impl<MISO, MOSI, SCLK> Pins<SPI> for (MISO, MOSI, SCLK)
where
    MISO: MisoPin<SPI>,
    MOSI: MosiPin<SPI>,
    SCLK: SclkPin<SPI>,
{
}

// Prevent users from implementing the SPI pin traits
mod private {
    use bl602_pac::SPI;

    use crate::gpio;

    use super::{MisoPin, MosiPin, SclkPin, SsPin};

    pub trait Sealed {}
    impl<MISO, MOSI, SCLK> Sealed for (MISO, MOSI, SCLK)
    where
        MISO: MisoPin<SPI>,
        MOSI: MosiPin<SPI>,
        SCLK: SclkPin<SPI>,
    {
    }

    impl<MISO, MOSI, SS, SCLK> Sealed for (MISO, MOSI, SS, SCLK)
    where
        MISO: MisoPin<SPI>,
        MOSI: MosiPin<SPI>,
        SS: SsPin<SPI>,
        SCLK: SclkPin<SPI>,
    {
    }

    impl<MODE> Sealed for gpio::Pin0<MODE> {}
    impl<MODE> Sealed for gpio::Pin1<MODE> {}
    impl<MODE> Sealed for gpio::Pin2<MODE> {}
    impl<MODE> Sealed for gpio::Pin3<MODE> {}
    impl<MODE> Sealed for gpio::Pin4<MODE> {}
    impl<MODE> Sealed for gpio::Pin5<MODE> {}
    impl<MODE> Sealed for gpio::Pin6<MODE> {}
    impl<MODE> Sealed for gpio::Pin7<MODE> {}
    impl<MODE> Sealed for gpio::Pin8<MODE> {}
    impl<MODE> Sealed for gpio::Pin9<MODE> {}
    impl<MODE> Sealed for gpio::Pin10<MODE> {}
    impl<MODE> Sealed for gpio::Pin11<MODE> {}
    impl<MODE> Sealed for gpio::Pin12<MODE> {}
    impl<MODE> Sealed for gpio::Pin13<MODE> {}
    impl<MODE> Sealed for gpio::Pin14<MODE> {}
    impl<MODE> Sealed for gpio::Pin15<MODE> {}
    impl<MODE> Sealed for gpio::Pin16<MODE> {}
    impl<MODE> Sealed for gpio::Pin17<MODE> {}
    impl<MODE> Sealed for gpio::Pin18<MODE> {}
    impl<MODE> Sealed for gpio::Pin19<MODE> {}
    impl<MODE> Sealed for gpio::Pin20<MODE> {}
    impl<MODE> Sealed for gpio::Pin21<MODE> {}
    impl<MODE> Sealed for gpio::Pin22<MODE> {}
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

      The frequency cannot be more than half of the spi clock frequency.
    */
    pub fn new(spi: SPI, pins: PINS, mode: Mode, freq: Hertz<u32>, clocks: Clocks) -> Self
    where
        PINS: Pins<pac::SPI>,
    {
        let glb = unsafe { &*pac::GLB::ptr() };

        glb.glb_parm.modify(|_r, w| {
            w.reg_spi_0_master_mode()
                .set_bit()
                .reg_spi_0_swap()
                .set_bit()
        });

        // length of phase 0 and 1 (i.e. low / high values of SCLK)
        // needs to be divided by two
        let len = clocks.spi_clk().0 / freq.0 / 2;
        if len > 256 || len == 0 {
            panic!("Cannot reach the desired SPI frequency");
        }

        let len = (len - 1) as u8;
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
                    embedded_hal::spi::Phase::CaptureOnFirstTransition => true,
                    embedded_hal::spi::Phase::CaptureOnSecondTransition => false,
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

impl<PINS> embedded_hal_nb::spi::ErrorType for Spi<pac::SPI, PINS> {
    type Error = Error;
}

impl<PINS> embedded_hal_nb::spi::FullDuplex<u8> for Spi<pac::SPI, PINS>
where
    PINS: Pins<pac::SPI>,
{
    fn read(&mut self) -> nb::Result<u8, Error> {
        let spi_fifo_config_0 = self.spi.spi_fifo_config_0.read();

        if spi_fifo_config_0.rx_fifo_overflow().bit_is_set() {
            Err(nb::Error::Other(Error::RxOverflow))
        } else if spi_fifo_config_0.rx_fifo_underflow().bit_is_set() {
            Err(nb::Error::Other(Error::RxUnderflow))
        } else if self.spi.spi_fifo_config_1.read().rx_fifo_cnt().bits() == 0 {
            Err(nb::Error::WouldBlock)
        } else {
            Ok((self.spi.spi_fifo_rdata.read().bits() & 0xff) as u8)
        }
    }

    fn write(&mut self, data: u8) -> nb::Result<(), Self::Error> {
        let spi_fifo_config_0 = self.spi.spi_fifo_config_0.read();

        if spi_fifo_config_0.tx_fifo_overflow().bit_is_set() {
            Err(nb::Error::Other(Error::TxOverflow))
        } else if spi_fifo_config_0.tx_fifo_underflow().bit_is_set() {
            Err(nb::Error::Other(Error::TxUnderflow))
        } else if self.spi.spi_fifo_config_1.read().tx_fifo_cnt().bits() == 0 {
            Err(nb::Error::WouldBlock)
        } else {
            self.spi
                .spi_fifo_wdata
                .write(|w| unsafe { w.bits(data as u32) });

            Ok(())
        }
    }
}

impl<PINS> FullDuplexZero<u8> for Spi<pac::SPI, PINS>
where
    PINS: Pins<pac::SPI>,
{
    type Error = Error;

    fn read(&mut self) -> nb::Result<u8, Error> {
        embedded_hal_nb::spi::FullDuplex::read(self)
    }

    fn send(&mut self, data: u8) -> nb::Result<(), Self::Error> {
        embedded_hal_nb::spi::FullDuplex::write(self, data)
    }
}

//TODO: Default marker traits are removed from e-h 1.0 alpha 5, must re-implement manually.
// We can still use them for e-h 0.2 though, so that makes life easy
impl<PINS> embedded_hal_zero::blocking::spi::transfer::Default<u8> for Spi<pac::SPI, PINS> where
    PINS: Pins<pac::SPI>
{
}

impl<PINS> embedded_hal_zero::blocking::spi::write::Default<u8> for Spi<pac::SPI, PINS> where
    PINS: Pins<pac::SPI>
{
}

impl<PINS> embedded_hal_zero::blocking::spi::write_iter::Default<u8> for Spi<pac::SPI, PINS> where
    PINS: Pins<pac::SPI>
{
}
