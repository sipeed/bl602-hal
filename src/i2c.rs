/*!
  # Inter-Integrated Circuit (I2C) bus
  To construct the I2C instance use the `I2c::new` function.
  The pin parameter is a tuple containing `(scl, sda)` which should be configured via `into_i2c_scl` and `into_i2c_sda`.

  ## Initialisation example
  ```rust
    let scl = parts.pin4.into_i2c_scl();
    let sda = parts.pin5.into_i2c_sda();

    let mut i2c = hal::i2c::I2c::i2c(
        dp.I2C,
        (scl, sda),
        100_000u32.Hz(),
        clocks,
    );
    ```
*/

use bl602_pac::I2C;
use embedded_hal::i2c as i2cAlpha;
use embedded_hal_zero::blocking::i2c::Read as ReadZero;
use embedded_hal_zero::blocking::i2c::Write as WriteZero;
use embedded_time::rate::Hertz;

use crate::delay::McycleDelay;
use crate::{clock::Clocks, pac};

use self::private::Sealed;

/// I2C error
#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    /// Rx overflow occurred
    RxOverflow,
    /// Rx underflow occurred
    RxUnderflow,
    /// Tx overflow occurred
    TxOverflow,
    /// Tx underflow occurred
    TxUnderflow,
    /// Timeout waiting for fifo occurred
    Timeout,
}

impl embedded_hal::i2c::Error for Error {
    fn kind(&self) -> embedded_hal::i2c::ErrorKind {
        match self {
            Self::RxOverflow => embedded_hal::i2c::ErrorKind::Overrun,
            Self::TxOverflow => embedded_hal::i2c::ErrorKind::Overrun,
            Self::RxUnderflow => embedded_hal::i2c::ErrorKind::Overrun,
            Self::TxUnderflow => embedded_hal::i2c::ErrorKind::Overrun,
            Self::Timeout => embedded_hal::i2c::ErrorKind::NoAcknowledge(
                embedded_hal::i2c::NoAcknowledgeSource::Address,
            ),
        }
    }
}

/// SDA pins
pub trait SdaPin<I2C>: Sealed {}

/// SCL pins
pub trait SclPin<I2C>: Sealed {}

/// I2C pins
pub trait Pins<I2C>: Sealed {}

impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin0<MODE> {}
impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin1<MODE> {}
impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin2<MODE> {}
impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin3<MODE> {}
impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin4<MODE> {}
impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin5<MODE> {}
impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin6<MODE> {}
impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin7<MODE> {}
impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin8<MODE> {}
impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin9<MODE> {}
impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin10<MODE> {}
impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin11<MODE> {}
impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin12<MODE> {}
impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin13<MODE> {}
impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin14<MODE> {}
impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin15<MODE> {}
impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin16<MODE> {}
impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin17<MODE> {}
impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin18<MODE> {}
impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin19<MODE> {}
impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin20<MODE> {}
impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin21<MODE> {}
impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin22<MODE> {}

impl<SCL, SDA> Pins<I2C> for (SCL, SDA)
where
    SCL: SclPin<I2C>,
    SDA: SdaPin<I2C>,
{
}

/// I2C peripheral operating in master mode supporting seven bit addressing
pub struct I2c<I2C, PINS> {
    /// i2c peripheral instance
    i2c: I2C,
    /// sda and scl pins for this i2c interface
    pins: PINS,
    /// timeout (in milliseconds)
    timeout: u16,
}

impl<PINS> I2c<pac::I2C, PINS>
where
    PINS: Pins<pac::I2C>,
{
    /**
      Constructs an I2C instance in master mode.
      The pin parameter tuple (scl, sda) needs to be configured accordingly.

      The frequency cannot be more than a quarter of the i2c clock frequency.

      The I2C instance supports 7 bit addressing mode.
    */
    pub fn new(i2c: I2C, pins: PINS, freq: Hertz<u32>, clocks: Clocks) -> Self
    where
        PINS: Pins<pac::I2C>,
    {
        // length of phase 0,1,2 and 3
        // needs to be divided by four
        let len = clocks.i2c_clk().0 / freq.0 / 4;
        if len > 256 || len <= 1 {
            // from the RM: Note: This value should not be set to 8’d0, adjust source
            // clock rate instead if higher I2C clock rate is required
            panic!("Cannot reach the desired I2C frequency");
        }

        let len = (len - 1) as u8;

        i2c.i2c_prd_start.modify(|_r, w| unsafe {
            w.cr_i2c_prd_s_ph_0()
                .bits(len)
                .cr_i2c_prd_s_ph_1()
                .bits(len)
                .cr_i2c_prd_s_ph_2()
                .bits(len)
                .cr_i2c_prd_s_ph_3()
                .bits(len)
        });

        i2c.i2c_prd_stop.modify(|_r, w| unsafe {
            w.cr_i2c_prd_p_ph_0()
                .bits(len)
                .cr_i2c_prd_p_ph_1()
                .bits(len)
                .cr_i2c_prd_p_ph_2()
                .bits(len)
                .cr_i2c_prd_p_ph_3()
                .bits(len)
        });

        i2c.i2c_prd_data.modify(|_r, w| unsafe {
            w.cr_i2c_prd_d_ph_0()
                .bits(len)
                .cr_i2c_prd_d_ph_1()
                .bits(len)
                .cr_i2c_prd_d_ph_2()
                .bits(len)
                .cr_i2c_prd_d_ph_3()
                .bits(len)
        });

        I2c {
            i2c,
            pins,
            timeout: 2048,
        }
    }

    pub fn release(self) -> (pac::I2C, PINS) {
        (self.i2c, self.pins)
    }

    /// Set the timeout (in milliseconds) when waiting for fifo (rx and tx).
    /// This defaults to 2048
    pub fn set_timeout(&mut self, timeout: u16) {
        self.timeout = timeout;
    }

    /// Clear FIFOs
    pub fn clear_fifo(&mut self) {
        self.i2c
            .i2c_fifo_config_0
            .write(|w| w.rx_fifo_clr().set_bit().tx_fifo_clr().set_bit());
    }
}

impl<PINS> i2cAlpha::ErrorType for I2c<pac::I2C, PINS> {
    type Error = Error;
}

impl<PINS> i2cAlpha::I2c<i2cAlpha::SevenBitAddress> for I2c<pac::I2C, PINS>
where
    PINS: Pins<pac::I2C>,
{
    fn read(
        &mut self,
        address: i2cAlpha::SevenBitAddress,
        buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        let fifo_config = self.i2c.i2c_fifo_config_0.read();

        if fifo_config.rx_fifo_overflow().bit_is_set() {
            self.i2c
                .i2c_fifo_config_0
                .write(|w| w.rx_fifo_clr().set_bit());
            return Err(Error::RxOverflow);
        } else if fifo_config.rx_fifo_underflow().bit_is_set() {
            self.i2c
                .i2c_fifo_config_0
                .write(|w| w.rx_fifo_clr().set_bit());
            return Err(Error::RxUnderflow);
        }

        let count = buffer.len() / 4 + if buffer.len() % 4 > 0 { 1 } else { 0 };
        let mut word_buffer = [0u32; 255];
        let tmp = &mut word_buffer[..count];

        self.i2c.i2c_config.modify(|_r, w| unsafe {
            w.cr_i2c_pkt_len()
                .bits(buffer.len() as u8 - 1u8)
                .cr_i2c_slv_addr()
                .bits(address)
                .cr_i2c_sub_addr_en()
                .clear_bit()
                .cr_i2c_sub_addr_bc()
                .bits(0)
                .cr_i2c_scl_sync_en()
                .set_bit()
                .cr_i2c_pkt_dir()
                .set_bit() // = read
                .cr_i2c_m_en()
                .set_bit()
        });

        // We don't know what the CPU frequency is. Assume maximum of 192Mhz
        // This might make our timeouts longer than expected if frequency is lower.
        let mut delay = McycleDelay::new(192_000_000);
        for value in tmp.iter_mut() {
            let start_time = McycleDelay::get_cycle_count();
            while self.i2c.i2c_fifo_config_1.read().rx_fifo_cnt().bits() == 0 {
                if delay.ms_since(start_time) > self.timeout.into() {
                    return Err(Error::Timeout);
                }
            }
            *value = self.i2c.i2c_fifo_rdata.read().i2c_fifo_rdata().bits();
        }

        self.i2c
            .i2c_config
            .modify(|_r, w| w.cr_i2c_m_en().clear_bit());

        for (idx, value) in buffer.iter_mut().enumerate() {
            let shift_by = (idx % 4 * 8) as u32;
            *value = (word_buffer[idx / 4].overflowing_shr(shift_by).0 & 0xff) as u8;
        }

        Ok(())
    }

    fn write(
        &mut self,
        address: i2cAlpha::SevenBitAddress,
        buffer: &[u8],
    ) -> Result<(), Self::Error> {
        let fifo_config = self.i2c.i2c_fifo_config_0.read();

        if fifo_config.tx_fifo_overflow().bit_is_set() {
            self.i2c
                .i2c_fifo_config_0
                .write(|w| w.tx_fifo_clr().set_bit());
            return Err(Error::TxOverflow);
        } else if fifo_config.tx_fifo_underflow().bit_is_set() {
            self.i2c
                .i2c_fifo_config_0
                .write(|w| w.tx_fifo_clr().set_bit());
            return Err(Error::TxUnderflow);
        }

        let mut word_buffer = [0u32; 255];
        let count = buffer.len() / 4 + if buffer.len() % 4 > 0 { 1 } else { 0 };
        for (idx, value) in buffer.iter().enumerate() {
            let shift_by = (idx % 4 * 8) as u32;
            word_buffer[idx / 4] |= (*value as u32).overflowing_shl(shift_by).0;
        }
        let tmp = &word_buffer[..count];

        self.i2c.i2c_config.modify(|_r, w| unsafe {
            w.cr_i2c_pkt_len()
                .bits(buffer.len() as u8 - 1u8)
                .cr_i2c_slv_addr()
                .bits(address)
                .cr_i2c_sub_addr_en()
                .clear_bit()
                .cr_i2c_sub_addr_bc()
                .bits(0)
                .cr_i2c_scl_sync_en()
                .set_bit()
                .cr_i2c_pkt_dir()
                .clear_bit() // = write
                .cr_i2c_m_en()
                .set_bit()
        });

        // We don't know what the CPU frequency is. Assume maximum of 192Mhz
        // This might make our timeouts longer than expected if frequency is lower.
        let mut delay = McycleDelay::new(192_000_000);
        for value in tmp.iter() {
            let start_time = McycleDelay::get_cycle_count();
            while self.i2c.i2c_fifo_config_1.read().tx_fifo_cnt().bits() == 0 {
                if delay.ms_since(start_time) > self.timeout.into() {
                    return Err(Error::Timeout);
                }
            }
            self.i2c
                .i2c_fifo_wdata
                .write(|w| unsafe { w.i2c_fifo_wdata().bits(*value) });
        }

        let start_time = McycleDelay::get_cycle_count();
        while self.i2c.i2c_fifo_config_1.read().tx_fifo_cnt().bits() < 2 {
            // wait for write fifo to be empty
            if delay.ms_since(start_time) > self.timeout.into() {
                return Err(Error::Timeout);
            }
        }

        let start_time = McycleDelay::get_cycle_count();
        while self.i2c.i2c_bus_busy.read().sts_i2c_bus_busy().bit_is_set() {
            // wait for transfer to finish
            if delay.ms_since(start_time) > self.timeout.into() {
                return Err(Error::Timeout);
            }
        }

        self.i2c
            .i2c_config
            .modify(|_r, w| w.cr_i2c_m_en().clear_bit());

        Ok(())
    }

    /// We can't meet the conttract for transaction, leaving it as unimplemented for now.
    /// https://github.com/rust-embedded/embedded-hal/blob/bf2b8a11fde064194ae5c70642b579051de631c8/embedded-hal/src/i2c.rs#L361
    fn transaction(
        &mut self,
        _address: i2cAlpha::SevenBitAddress,
        _operations: &mut [i2cAlpha::Operation<'_>],
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }
}

impl<PINS> ReadZero for I2c<pac::I2C, PINS>
where
    PINS: Pins<pac::I2C>,
{
    type Error = Error;

    fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        i2cAlpha::I2c::read(self, address, buffer)
    }
}

impl<PINS> WriteZero for I2c<pac::I2C, PINS>
where
    PINS: Pins<pac::I2C>,
{
    type Error = Error;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        i2cAlpha::I2c::write(self, addr, bytes)
    }
}

// Prevent users from implementing the i2c pin traits
mod private {
    use super::{SclPin, SdaPin};
    use crate::gpio;
    use bl602_pac::I2C;

    pub trait Sealed {}
    impl<SCL, SDA> Sealed for (SCL, SDA)
    where
        SCL: SclPin<I2C>,
        SDA: SdaPin<I2C>,
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
