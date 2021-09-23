/*!
  # Inter-Integrated Circuit (I2C) bus
  To construct the I2C instance use the `I2c::i2c` function.
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
use embedded_hal::i2c::blocking::Read as ReadAlpha;
use embedded_hal::i2c::blocking::Write as WriteAlpha;
use embedded_hal_zero::blocking::i2c::Read as ReadZero;
use embedded_hal_zero::blocking::i2c::Write as WriteZero;
use embedded_time::rate::Hertz;

use crate::{clock::Clocks, pac};

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

/// SDA pins - DO NOT IMPLEMENT THIS TRAIT
pub unsafe trait SdaPin<I2C> {}

/// SCL pins - DO NOT IMPLEMENT THIS TRAIT
pub unsafe trait SclPin<I2C> {}

/// I2C pins - DO NOT IMPLEMENT THIS TRAIT
pub unsafe trait Pins<I2C> {}

unsafe impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin0<MODE> {}
unsafe impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin1<MODE> {}
unsafe impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin2<MODE> {}
unsafe impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin3<MODE> {}
unsafe impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin4<MODE> {}
unsafe impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin5<MODE> {}
unsafe impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin6<MODE> {}
unsafe impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin7<MODE> {}
unsafe impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin8<MODE> {}
unsafe impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin9<MODE> {}
unsafe impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin10<MODE> {}
unsafe impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin11<MODE> {}
unsafe impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin12<MODE> {}
unsafe impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin13<MODE> {}
unsafe impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin14<MODE> {}
unsafe impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin15<MODE> {}
unsafe impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin16<MODE> {}
unsafe impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin17<MODE> {}
unsafe impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin18<MODE> {}
unsafe impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin19<MODE> {}
unsafe impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin20<MODE> {}
unsafe impl<MODE> SdaPin<pac::I2C> for crate::gpio::Pin21<MODE> {}
unsafe impl<MODE> SclPin<pac::I2C> for crate::gpio::Pin22<MODE> {}

unsafe impl<SCL, SDA> Pins<I2C> for (SCL, SDA)
where
    SCL: SclPin<I2C>,
    SDA: SdaPin<I2C>,
{
}

/// I2C peripheral operating in master mode supporting seven bit addressing
pub struct I2c<I2C, PINS> {
    i2c: I2C,
    pins: PINS,
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

    /// Set the timeout when waiting for fifo (rx and tx).
    /// It's not a time unit but the number of cycles to wait.
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

impl<PINS> ReadAlpha<i2cAlpha::SevenBitAddress> for I2c<pac::I2C, PINS>
where
    PINS: Pins<pac::I2C>,
{
    type Error = Error;

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

        for value in tmp.iter_mut() {
            let mut timeout_countdown = self.timeout;
            while self.i2c.i2c_fifo_config_1.read().rx_fifo_cnt().bits() == 0 {
                if timeout_countdown == 0 {
                    return Err(Error::Timeout);
                }
                timeout_countdown -= 1;
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
}

impl<PINS> WriteAlpha<i2cAlpha::SevenBitAddress> for I2c<pac::I2C, PINS>
where
    PINS: Pins<pac::I2C>,
{
    type Error = Error;

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

        for value in tmp.iter() {
            let mut timeout_countdown = self.timeout;
            while self.i2c.i2c_fifo_config_1.read().tx_fifo_cnt().bits() == 0 {
                if timeout_countdown == 0 {
                    return Err(Error::Timeout);
                }
                timeout_countdown -= 1;
            }
            self.i2c
                .i2c_fifo_wdata
                .write(|w| unsafe { w.i2c_fifo_wdata().bits(*value as u32) });
        }

        while self.i2c.i2c_bus_busy.read().sts_i2c_bus_busy().bit_is_set() {
            // wait for transfer to finish
        }

        self.i2c
            .i2c_config
            .modify(|_r, w| w.cr_i2c_m_en().clear_bit());

        Ok(())
    }
}

impl<PINS> ReadZero for I2c<pac::I2C, PINS>
where
    PINS: Pins<pac::I2C>,
{
    type Error = Error;

    fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        ReadAlpha::read(self, address, buffer)
    }
}

impl<PINS> WriteZero for I2c<pac::I2C, PINS>
where
    PINS: Pins<pac::I2C>,
{
    type Error = Error;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        WriteAlpha::write(self, addr, bytes)
    }
}
