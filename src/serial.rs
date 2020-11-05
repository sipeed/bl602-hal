//! Serial communication

/// Serial abstraction
pub struct Serial<UART, TX, RX> {
    uart: UART,
    tx: TX,
    rx: RX,
}

// impl<UART>