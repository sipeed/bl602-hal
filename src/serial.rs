//! Serial communication
use embedded_time::rate::Baud;

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
    pub parity: Parity,
    pub stopbits: StopBits,
    pub wordlength: WordLength,
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

/// Serial abstraction
pub struct Serial<UART, TX, RX> {
    uart: UART,
    tx: TX,
    rx: RX,
}

// impl<UART>