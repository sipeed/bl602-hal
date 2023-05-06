//! Single-buffered or peripheral-peripheral DMA Transfers

use core::sync::atomic::{compiler_fence, Ordering};

use super::{ReadTarget, WriteTarget};
use super::single_channel::{ChannelConfig, SingleChannel};

/// Configuration for single-buffered DMA transfer
pub struct Config<CH: SingleChannel, FROM: ReadTarget, TO: WriteTarget> {
    ch: CH,
    from: FROM,
    to: TO,
}

impl<CH, FROM, TO, WORD> Config<CH, FROM, TO>
where
    CH: SingleChannel,
    FROM: ReadTarget<ReceivedWord = WORD>,
    TO: WriteTarget<TransmittedWord = WORD>,
{
    /// Create a new configuration for single-buffered DMA transfer
    pub fn new(ch: CH, from: FROM, to: TO) -> Config<CH, FROM, TO> {
        Config { ch, from, to }
    }

    /// Start the DMA transfer
    pub fn start(mut self) -> Transfer<CH, FROM, TO> {
        // TODO: Do we want to call any callbacks to configure source/sink?

        // Make sure that memory contents reflect what the user intended.
        // TODO: How much of the following is necessary?
        compiler_fence(Ordering::SeqCst);

        // Configure the DMA channel and start it.
        self.ch.config(&self.from, &mut self.to);
        self.ch.start();

        Transfer {
            ch: self.ch,
            from: self.from,
            to: self.to,
        }
    }
}

// TODO: Drop for most of these structs
/// Instance of a single-buffered DMA transfer
pub struct Transfer<CH: SingleChannel, FROM: ReadTarget, TO: WriteTarget> {
    ch: CH,
    from: FROM,
    to: TO,
}

impl<CH, FROM, TO, WORD> Transfer<CH, FROM, TO>
where
    CH: SingleChannel,
    FROM: ReadTarget<ReceivedWord = WORD>,
    TO: WriteTarget<TransmittedWord = WORD>,
{
    /// Check if an interrupt is pending for this channel
    /// and clear the corresponding pending bit
    pub fn check_tc_irq(&mut self) -> bool {
        self.ch.check_tc_irq()
    }

    /// Check if an interrupt is pending for this channel
    /// and clear the corresponding pending bit
    pub fn check_err_irq(&mut self) -> bool {
        self.ch.check_err_irq()
    }

    pub fn is_done(&self) -> bool {
        !self.ch.is_enabled()
        // let status = unsafe { (*crate::pac::DMA::ptr()).dma_int_tcstatus.read().bits() };
        // status != 0
    }

    /// Block until the transfer is complete, returning the channel and targets
    pub fn wait(self) -> (CH, FROM, TO) {
        while !self.is_done() {}

        // Make sure that memory contents reflect what the user intended.
        compiler_fence(Ordering::SeqCst);

        (self.ch, self.from, self.to)
    }
}
