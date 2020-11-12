//! Serial communication
use embedded_time::rate::{Extensions, Baud};
use crate::pac;

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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
/// Order of the bits that are transmitted and received on the wire.
pub enum Order {
    /// Each byte is sent out LSB-first
    LsbFirst,
    /// Each byte is sent out MSB-first
    MsbFirst,
}

/// Parity
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

// todo
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
    pins: PINS
}

impl<PINS> Serial<pac::UART, PINS> { // todo: there is uart0 and uart1
    pub fn uart0(
        uart: pac::UART,
        config: Config,
        pins: PINS
    ) -> Self {
        // Bit inverse configuration; MsbFirst => 1, LsbFirst => 0
        let order_cfg = match config.order {
            Order::LsbFirst => false,
            Order::MsbFirst => true,
        };
        uart.data_config.write(|w| w
            .cr_uart_bit_inv().bit(order_cfg)
        );
        // Uart TX config
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
            Parity::ParityOdd => (true, true), // odd => 1
        };
        uart.utx_config.write(|w| unsafe { w
            .cr_utx_prt_en().bit(parity_enable)
            .cr_utx_prt_sel().bit(parity_type)
            .cr_utx_bit_cnt_d().bits(data_bits_cfg)
            .cr_utx_bit_cnt_p().bits(stop_bits_cfg) 
            .cr_utx_frm_en().set_bit() // [!] freerun on // todo
            .cr_utx_cts_en().clear_bit() // no CTS // todo
            .cr_utx_en().set_bit() // enable TX
        });
        // Uart RX config
        uart.urx_config.write(|w| unsafe { w
            .cr_urx_prt_en().bit(parity_enable)
            .cr_urx_prt_sel().bit(parity_type)
            .cr_urx_bit_cnt_d().bits(data_bits_cfg)
            .cr_urx_deg_en().clear_bit() // no rx input de-glitch // todo
            .cr_urx_rts_sw_mode().clear_bit() // no RTS // todo
            .cr_urx_en().set_bit() // enable RX
        });
        Serial { uart, pins }
    }

    // pub fn listen(&mut self, event: Event) {

    // }
}

impl<PINS> embedded_hal::serial::Write<u8> for Serial<pac::UART, PINS> {
    type Error = Error;

    fn try_write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        self.uart.uart_fifo_wdata.write(|w| unsafe {
            w.bits(word as u32)
        });
        Ok(())
    }

    fn try_flush(&mut self) -> nb::Result<(), Self::Error> {
        if self.uart.uart_fifo_config_1.read().tx_fifo_cnt().bits() < 1 {
            Err(nb::Error::WouldBlock)
        } else {
            Ok(())
        }
    }
}

impl<PINS> embedded_hal::serial::Read<u8> for Serial<pac::UART, PINS> {
    type Error = Error;

    fn try_read(&mut self) -> nb::Result<u8, Self::Error> {
        let ans = self.uart.uart_fifo_rdata.read().bits();
        Ok((ans & 0xff) as u8)
    }
}
