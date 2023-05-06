use super::{Channel, ChannelIndex, ChannelRegs, ReadTarget, WriteTarget};
use crate::typelevel::Sealed;
use core::convert::TryFrom;

/// Trait which implements low-level functionality for transfers using a single DMA channel.
pub trait SingleChannel: Sealed {
    /// Returns the registers associated with this DMA channel.
    fn ch(&self) -> &crate::pac::dma::CH;
    /// Returns the index of the DMA channel.
    fn id(&self) -> u8;

    /// Enables the terminal count signal for this channel.
    fn listen_tc_irq(&mut self) {
        self.ch().ch_config.modify(|_, w| {
            w.itc().clear_bit() // unmask terminal count interrupt
        });
    }

    /// Disables the terminal count signal for this channel.
    fn unlisten_tc_irq(&mut self) {
        self.ch().ch_config.modify(|_, w| {
            w.itc().set_bit() // mask terminal count interrupt
        });
    }

    /// Check if an interrupt is pending for this channel
    /// and clear the corresponding pending bit
    fn check_tc_irq(&mut self) -> bool {
        // Safety: ...
        unsafe {
            let status = (*crate::pac::DMA::ptr()).dma_int_tcstatus.read().bits();
            if (status & (1 << self.id())) != 0 {
                // Clear the interrupt.
                (*crate::pac::DMA::ptr())
                    .dma_int_tcclear
                    .write(|w| w.int_tcclear().bits(1 << self.id()));
                true
            } else {
                false
            }
        }
    }

    /// Enables the error signal for this channel.
    fn listen_err_irq(&mut self) {
        self.ch().ch_config.modify(|_, w| {
            w.ie().clear_bit() // unmask error interrupt
        });
    }

    /// Disables the error signal for this channel.
    fn unlisten_err_irq(&mut self) {
        self.ch().ch_config.modify(|_, w| {
            w.ie().set_bit() // mask error interrupt
        });
    }

    /// Check if an interrupt is pending for this channel
    /// and clear the corresponding pending bit
    fn check_err_irq(&mut self) -> bool {
        // Safety: ...
        unsafe {
            let status = (*crate::pac::DMA::ptr()).dma_int_error_status.read().bits();
            // TODO: figure out which bit gets set
            if (status & (1 << self.id())) != 0 {
                // Clear the interrupt.
                (*crate::pac::DMA::ptr())
                    .dma_int_err_clr
                    .write(|w| w.int_err_clr().bits(1 << self.id()));
                true
            } else {
                false
            }
        }
    }

    /// Get the number of data transfers that (still) need to be done.
    fn transfer_size(&self) -> usize {
        self.ch().ch_control.read().transfer_size().bits().into()
    }
}

impl<CH: ChannelIndex> SingleChannel for Channel<CH>
where
    Channel<CH>: ChannelRegs,
{
    fn ch(&self) -> &crate::pac::dma::CH {
        self.regs()
    }

    fn id(&self) -> u8 {
        CH::id()
    }
}

impl<CH: ChannelIndex> Sealed for Channel<CH> {}

/// The number of bits of a single element
pub enum TransferWidth {
    TW8 = 0,
    TW16 = 1,
    TW32 = 2,
}

/// The number of elements in a single transfer burst
pub enum BurstSize {
    BS1 = 0,
    BS4 = 1,
    BS8 = 2,
    BS16 = 3,
}

pub enum FlowControl {
    /// Memory to memory transfer
    M2M = 0b000,
    /// Memory to peripheral transfer
    M2P = 0b001,
    /// Peripheral to memory transfer
    P2M = 0b010,
    /// Peripheral to peripheral transfer
    P2P = 0b011,
    // TODO: other options
}

pub(crate) trait ChannelConfig {
    fn config<WORD, FROM, TO>(&mut self, from: &FROM, to: &mut TO)
    where
        FROM: ReadTarget<ReceivedWord = WORD>,
        TO: WriteTarget<TransmittedWord = WORD>;

    fn start(&mut self);

    fn is_enabled(&self) -> bool;
}

impl<CH: SingleChannel> ChannelConfig for CH {
    fn config<WORD, FROM, TO>(&mut self, from: &FROM, to: &mut TO)
    where
        FROM: ReadTarget<ReceivedWord = WORD>,
        TO: WriteTarget<TransmittedWord = WORD>,
    {
        // Configure the DMA channel.
        let (src, src_count) = from.rx_address_count();
        let src_incr = from.rx_increment();
        let (dst, dst_count) = to.tx_address_count();
        let dst_incr = to.tx_increment();
        let len: u16 = match u16::try_from(u32::min(src_count, dst_count)) {
            Ok(v) => u16::min(v, 4095), // limit to max transfer size
            Err(_) => 4095,             // use max transfer size
        };

        let (srcph, dstph, flowctrl) = match (FROM::rx_treq(), TO::tx_treq()) {
            // Memory-to-memory
            (None, None) => (0, 0, 0b000),
            // Memory-to-peripheral
            (None, Some(d)) => (0, d, 0b001),
            // Peripheral-to-memory
            (Some(s), None) => (s, 0, 0b010),
            // Peripheral-to-peripheral
            (Some(s), Some(d)) => (s, d, 0b011),
        };

        self.ch().ch_config.modify(|_, w| w.e().clear_bit());
        #[rustfmt::skip]
        self.ch().ch_control.write(|w| {
            w.transfer_size().variant(len)
             .sbsize().variant(0) // increment 1 byte
             .dbsize().variant(0) // increment 1 byte
             .swidth().variant(0) // 8-bit width
             .dwidth().variant(0) // 8-bit width
             .si().bit(src_incr)
             .di().bit(dst_incr)
             .prot().variant(0)   // TODO: when would you need this?
             .i().set_bit()
        });
        #[rustfmt::skip]
        self.ch().ch_config.modify(|_, w| {
            w.src_peripheral().variant(srcph)
             .dst_peripheral().variant(dstph)
             .flow_cntrl().variant(flowctrl)
             .itc().clear_bit() // mask terminal count interrupt
             .ie().set_bit() // mask error interrupt
        });

        // set source and destination address
        self.ch()
            .ch_src_addr
            .write(|w| unsafe { w.src_addr().bits(src) });
        self.ch()
            .ch_dst_addr
            .write(|w| unsafe { w.dst_addr().bits(dst) });

        // clear interrupt status
        let dma = unsafe { &*crate::pac::DMA::ptr() };
        dma.dma_int_tcclear
            .write(|w| w.int_tcclear().variant(1 << self.id()));
        dma.dma_int_err_clr
            .write(|w| w.int_err_clr().variant(1 << self.id()));
    }

    fn start(&mut self) {
        self.ch().ch_config.modify(|_, w| w.e().set_bit());
    }

    fn is_enabled(&self) -> bool {
        self.ch().ch_config.read().e().bit_is_set()
    }
}
