//! Serial communication
use crate::clock::Clocks;
use crate::pac;
use core::fmt;
use core::ops::Deref;
use embedded_hal_nb;
use embedded_hal_nb::serial::Write;
use embedded_time::rate::{Baud, Extensions};
use nb::block;

/// Serial error
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Framing error
    Framing,
    /// Noise error
    Noise,
    /// RX buffer overrun
    Overrun,
    /// Parity check error
    Parity,
}

/// Serial configuration
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Config {
    pub baudrate: Baud,
    pub order: Order,
    pub parity: Parity,
    pub stopbits: StopBits,
    pub wordlength: WordLength,
}

impl Config {
    /// Sets the target baudrate
    pub fn baudrate(mut self, baudrate: impl Into<Baud>) -> Self {
        self.baudrate = baudrate.into();

        self
    }

    /// Sets parity to no parity check
    pub fn parity_none(mut self) -> Self {
        self.parity = Parity::ParityNone;

        self
    }

    /// Sets parity check to even
    pub fn parity_even(mut self) -> Self {
        self.parity = Parity::ParityEven;

        self
    }

    /// Sets parity check to odd
    pub fn parity_odd(mut self) -> Self {
        self.parity = Parity::ParityOdd;

        self
    }

    /// Sets the target stopbits
    pub fn stopbits(mut self, stopbits: StopBits) -> Self {
        self.stopbits = stopbits;

        self
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            baudrate: 115_200_u32.Bd(),
            order: Order::LsbFirst,
            parity: Parity::ParityNone,
            stopbits: StopBits::STOP1,
            wordlength: WordLength::Eight,
        }
    }
}

/// Order of the bits transmitted and received on the wire
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Order {
    /// Each byte is sent out LSB-first
    LsbFirst,
    /// Each byte is sent out MSB-first
    MsbFirst,
}

/// Parity check
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Parity {
    /// No parity check
    ParityNone,
    /// Even parity bit
    ParityEven,
    /// Odd parity bit
    ParityOdd,
}

/// Stop bits
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StopBits {
    /// 1 stop bit
    STOP1,
    /// 0.5 stop bits
    STOP0P5,
    /// 2 stop bits
    STOP2,
    /// 1.5 stop bits
    STOP1P5,
}

/// Word length
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum WordLength {
    Five,
    Six,
    Seven,
    Eight,
}

/// Interrupt event
pub enum Event {
    /// UART RX FIFO error interrupt
    RxFifoError,
    /// UART TX FIFO error interrupt
    TxFifoError,
    /// UART RX parity check error interrupt
    RxParityError,
    /// UART RX Time-out interrupt
    RxTimeout,
    /// UART RX FIFO ready (rx_fifo_cnt > rx_fifo_th) interrupt
    RxFifoReady,
    /// UART TX FIFO ready (tx_fifo_cnt > tx_fifo_th) interrupt
    TxFifoReady,
    /// UART RX transfer end interrupt
    RxTransferEnd,
    /// UART TX transfer end interrupt
    TxTransferEnd,
}

/// Serial abstraction
pub struct Serial<UART, PINS> {
    uart: UART,
    pins: PINS,
}

impl<UART, PINS> Serial<UART, PINS>
where
    UART: Deref<Target = pac::uart0::RegisterBlock>,
    PINS: Pins<UART>,
{
    pub fn new(uart: UART, config: Config, pins: PINS, clocks: Clocks) -> Self {
        // Initialize clocks and baudrate
        let uart_clk = clocks.uart_clk();
        let baud = config.baudrate.0;
        let divisor = {
            // Can't possibly have a baudrate greater than uart_clock
            if baud > uart_clk.0 {
                panic!("impossible baudrate");
            }
            // If we did this calculation using integer math, it always rounds down
            // Reduce error by doing calculation using floating point, then
            // add half before converting back to integer to round nearest instead
            let ans_f = uart_clk.0 as f32 / baud as f32;
            let ans = (ans_f + 0.5) as u32;

            if !(1..=65535).contains(&ans) {
                panic!("impossible baudrate");
            }

            ans as u16
        };

        uart.uart_bit_prd.write(|w| unsafe {
            w.cr_urx_bit_prd()
                .bits(divisor - 1)
                .cr_utx_bit_prd()
                .bits(divisor - 1)
        });

        // Bit inverse configuration; MsbFirst => 1, LsbFirst => 0
        let order_cfg = match config.order {
            Order::LsbFirst => false,
            Order::MsbFirst => true,
        };

        uart.data_config
            .write(|w| w.cr_uart_bit_inv().bit(order_cfg));

        // UART TX config
        let data_bits_cfg = match config.wordlength {
            WordLength::Five => 4,
            WordLength::Six => 5,
            WordLength::Seven => 6,
            WordLength::Eight => 7,
        };
        let stop_bits_cfg = match config.stopbits {
            StopBits::STOP0P5 => 0,
            StopBits::STOP1 => 1,
            StopBits::STOP1P5 => 2,
            StopBits::STOP2 => 3,
        };
        let (parity_enable, parity_type) = match config.parity {
            Parity::ParityNone => (false, false),
            Parity::ParityEven => (true, false), // even => 0
            Parity::ParityOdd => (true, true),   // odd => 1
        };

        uart.utx_config.write(|w| unsafe {
            w.cr_utx_prt_en()
                .bit(parity_enable)
                .cr_utx_prt_sel()
                .bit(parity_type)
                .cr_utx_bit_cnt_d()
                .bits(data_bits_cfg)
                .cr_utx_bit_cnt_p()
                .bits(stop_bits_cfg)
                .cr_utx_frm_en()
                .set_bit() // [!] freerun on // todo
                .cr_utx_cts_en()
                .bit(PINS::HAS_CTS)
                .cr_utx_en()
                .bit(PINS::HAS_TX)
        });

        // UART RX config
        uart.urx_config.write(|w| unsafe {
            w.cr_urx_prt_en()
                .bit(parity_enable)
                .cr_urx_prt_sel()
                .bit(parity_type)
                .cr_urx_bit_cnt_d()
                .bits(data_bits_cfg)
                .cr_urx_deg_en()
                .clear_bit() // no rx input de-glitch // todo
                .cr_urx_rts_sw_mode()
                .clear_bit() // no RTS // todo
                .cr_urx_en()
                .bit(PINS::HAS_RX)
        });

        Serial { uart, pins }
    }

    pub fn free(self) -> (UART, PINS) {
        // todo!
        (self.uart, self.pins)
    }
}

impl<UART, PINS>  embedded_hal_nb::serial::ErrorType for Serial<UART, PINS>{
    type Error = Error;
}

impl<UART, PINS> embedded_hal_nb::serial::Write for Serial<UART, PINS>
where
    UART: Deref<Target = pac::uart0::RegisterBlock>,
{
    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        // If there's no room to write a byte or more to the FIFO, return WouldBlock
        if self.uart.uart_fifo_config_1.read().tx_fifo_cnt().bits() == 0 {
            Err(nb::Error::WouldBlock)
        } else {
            self.uart
                .uart_fifo_wdata
                .write(|w| unsafe { w.bits(word as u32) });
            Ok(())
        }
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        // If we're still transmitting or have data in our 32 byte FIFO, return WouldBlock
        if self.uart.uart_fifo_config_1.read().tx_fifo_cnt().bits() != 32
            || self.uart.uart_status.read().sts_utx_bus_busy().bit_is_set()
        {
            Err(nb::Error::WouldBlock)
        } else {
            Ok(())
        }
    }
}

impl<UART, PINS> embedded_hal_nb::serial::Read<u8> for Serial<UART, PINS>
where
    UART: Deref<Target = pac::uart0::RegisterBlock>,
{
    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        if self.uart.uart_fifo_config_1.read().rx_fifo_cnt().bits() == 0 {
            Err(nb::Error::WouldBlock)
        }
        else {
            let ans = self.uart.uart_fifo_rdata.read().bits();
            Ok((ans & 0xff) as u8)
        }
    }
}

impl<UART, PINS> embedded_hal_zero::serial::Write<u8> for Serial<UART, PINS>
where
    UART: Deref<Target = pac::uart0::RegisterBlock>,
{
    type Error = Error;

    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        embedded_hal_nb::serial::Write::write(self, word)
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        embedded_hal_nb::serial::Write::flush(self)
    }
}

impl<UART, PINS> embedded_hal_zero::serial::Read<u8> for Serial<UART, PINS>
where
    UART: Deref<Target = pac::uart0::RegisterBlock>,
{
    type Error = Error;

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        embedded_hal_nb::serial::Read::read(self)
    }
}

impl<UART, PINS> fmt::Write for Serial<UART, PINS>
where
    Serial<UART, PINS>: embedded_hal_nb::serial::Write<u8>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        s.as_bytes()
            .iter()
            .try_for_each(|c| block!(self.write(*c)))
            .map_err(|_| fmt::Error)
    }
}

/// Serial transmit pins - DO NOT IMPLEMENT THIS TRAIT
pub unsafe trait TxPin<UART> {}
/// Serial receive pins - DO NOT IMPLEMENT THIS TRAIT
pub unsafe trait RxPin<UART> {}
/// Serial rts pins - DO NOT IMPLEMENT THIS TRAIT
pub unsafe trait RtsPin<UART> {}
/// Serial cts pins - DO NOT IMPLEMENT THIS TRAIT
pub unsafe trait CtsPin<UART> {}

macro_rules! impl_uart_pin {
    ($(($UartSigi: ident, $UartMuxi: ident),)+) => {
        use crate::gpio::*;
        $(
        unsafe impl<PIN: UartPin<$UartSigi>> TxPin<pac::UART0> for (PIN, $UartMuxi<Uart0Tx>) {}
        unsafe impl<PIN: UartPin<$UartSigi>> RxPin<pac::UART0> for (PIN, $UartMuxi<Uart0Rx>) {}
        unsafe impl<PIN: UartPin<$UartSigi>> RtsPin<pac::UART0> for (PIN, $UartMuxi<Uart0Rts>) {}
        unsafe impl<PIN: UartPin<$UartSigi>> CtsPin<pac::UART0> for (PIN, $UartMuxi<Uart0Cts>) {}

        unsafe impl<PIN: UartPin<$UartSigi>> TxPin<pac::UART1> for (PIN, $UartMuxi<Uart1Tx>) {}
        unsafe impl<PIN: UartPin<$UartSigi>> RxPin<pac::UART1> for (PIN, $UartMuxi<Uart1Rx>) {}
        unsafe impl<PIN: UartPin<$UartSigi>> RtsPin<pac::UART1> for (PIN, $UartMuxi<Uart1Rts>) {}
        unsafe impl<PIN: UartPin<$UartSigi>> CtsPin<pac::UART1> for (PIN, $UartMuxi<Uart1Cts>) {}
        )+
    };
}

impl_uart_pin!(
    (UartSig0, UartMux0),
    (UartSig1, UartMux1),
    (UartSig2, UartMux2),
    (UartSig3, UartMux3),
    (UartSig4, UartMux4),
    (UartSig5, UartMux5),
    (UartSig6, UartMux6),
    (UartSig7, UartMux7),
);

/// Serial pins - DO NOT IMPLEMENT THIS TRAIT
pub unsafe trait Pins<UART> {
    const HAS_TX: bool;
    const HAS_RX: bool;
    const HAS_RTS: bool;
    const HAS_CTS: bool;
}

unsafe impl<UART, TX, RX> Pins<UART> for (TX, RX)
where
    TX: TxPin<UART>,
    RX: RxPin<UART>,
{
    const HAS_TX: bool = true;
    const HAS_RX: bool = true;
    const HAS_RTS: bool = false;
    const HAS_CTS: bool = false;
}

unsafe impl<UART, TX, RX, RTS, CTS> Pins<UART> for (TX, RX, RTS, CTS)
where
    TX: TxPin<UART>,
    RX: RxPin<UART>,
    RTS: RxPin<UART>,
    CTS: RxPin<UART>,
{
    const HAS_TX: bool = true;
    const HAS_RX: bool = true;
    const HAS_RTS: bool = true;
    const HAS_CTS: bool = true;
}
