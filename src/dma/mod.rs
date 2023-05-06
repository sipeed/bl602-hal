//! Direct Memory Access
//!
//! Abstraction layer for configuring and using the DMA controller to move data
//! without intervention from the CPU core.
//! The BL602 support 4 independent channels that can transfer data to/from
//! memory and peripherals in various configurations.
//!
//! The current structure of this module has been taken from
//! [rp-hal](https://github.com/rp-rs/rp-hal) and could be subject to change in the future
//! if it needs to be tailored for BL602 specifics (e.g., when implementing the linked list mode).
use crate::typelevel::Sealed;
use core::marker::PhantomData;
use embedded_dma::{ReadBuffer, WriteBuffer};

pub mod single_buffer;
pub mod single_channel;

/// DMA unit.
pub trait DMAExt: Sealed {
    /// Splits the DMA unit into its individual channels.
    fn split(self) -> Channels;
}

pub struct DMA {
    dma: crate::pac::DMA,
}

impl DMA {
    /// Enable the DMA engine and construct a new instance
    pub fn new(dma: crate::pac::DMA) -> Self {
        dma.dma_top_config.modify(|_, w| w.e().set_bit());
        Self { dma }
    }

    /// Disable the DMA engine and free the underlying object
    pub fn free(self) -> crate::pac::DMA {
        self.dma.dma_top_config.modify(|_, w| w.e().clear_bit());
        self.dma
    }
}

impl Sealed for DMA {}

/// DMA channel.
pub struct Channel<CH: ChannelIndex> {
    _phantom: PhantomData<CH>,
}

/// DMA channel identifier.
pub trait ChannelIndex: Sealed {
    /// Numerical index of the DMA channel (0..3).
    fn id() -> u8;
}

trait ChannelRegs {
    unsafe fn ptr() -> *const crate::pac::dma::CH;
    fn regs(&self) -> &crate::pac::dma::CH;
}

macro_rules! channels {
    (
        $($CHX:ident: ($chX:ident, $x:expr),)+
    ) => {
        impl DMAExt for DMA {
            fn split(self) -> Channels {
                Channels {
                    $(
                        $chX: Channel {
                            _phantom: PhantomData,
                        },
                    )+
                }
            }
        }

        /// Set of DMA channels.
        pub struct Channels {
            $(
                /// DMA channel.
                pub $chX: Channel<$CHX>,
            )+
        }
        $(
            /// DMA channel identifier.
            pub struct $CHX;
            impl ChannelIndex for $CHX {
                fn id() -> u8 {
                    $x
                }
            }

            impl Sealed for $CHX {}

            impl ChannelRegs for Channel<$CHX> {
                unsafe fn ptr() -> *const crate::pac::dma::CH {
                    &(*crate::pac::DMA::ptr()).$chX as *const _
                }

                fn regs(&self) -> &crate::pac::dma::CH {
                    unsafe { &*Self::ptr() }
                }
            }
        )+
    }
}

channels! {
    CH0: (ch0, 0),
    CH1: (ch1, 1),
    CH2: (ch2, 2),
    CH3: (ch3, 3),
}

/// Trait which is implemented by anything that can be read via DMA.
pub trait ReadTarget {
    /// Type which is transferred in a single DMA transfer.
    type ReceivedWord;

    /// Returns the SRCPH number for this data source (`None` for memory buffers).
    fn rx_treq() -> Option<u8>;

    /// Returns the address and the maximum number of words that can be transferred from this data
    /// source in a single DMA operation.
    ///
    /// For peripherals, the count should likely be u32::MAX. If a data source implements
    /// EndlessReadTarget, it is suitable for infinite transfers from or to ring buffers. Note that
    /// ring buffers designated for endless transfers, but with a finite buffer size, should return
    /// the size of their individual buffers here.
    ///
    /// # Safety
    ///
    /// This function has the same safety guarantees as `ReadBuffer::read_buffer`.
    fn rx_address_count(&self) -> (u32, u32);

    /// Returns whether the address shall be incremented after each transfer.
    fn rx_increment(&self) -> bool;
}

/// Marker which signals that `rx_address_count()` can be called multiple times.
///
/// The DMA code will never call `rx_address_count()` to request more than two buffers to configure
/// two DMA channels. In the case of peripherals, the function can always return the same values.
pub trait EndlessReadTarget: ReadTarget {}

impl<B: ReadBuffer> ReadTarget for B {
    type ReceivedWord = <B as ReadBuffer>::Word;

    fn rx_treq() -> Option<u8> {
        None
    }

    fn rx_address_count(&self) -> (u32, u32) {
        let (ptr, len) = unsafe { self.read_buffer() };
        (ptr as u32, len as u32)
    }

    fn rx_increment(&self) -> bool {
        true
    }
}

/// Trait which is implemented by anything that can be written via DMA.
pub trait WriteTarget {
    /// Type which is transferred in a single DMA transfer.
    type TransmittedWord;

    /// Returns the DSTPH number for this data sink (`None` for memory buffers).
    fn tx_treq() -> Option<u8>;

    /// Returns the address and the maximum number of words that can be transferred from this data
    /// source in a single DMA operation.
    ///
    /// See `ReadTarget::rx_address_count` for a complete description of the semantics of this
    /// function.
    fn tx_address_count(&mut self) -> (u32, u32);

    /// Returns whether the address shall be incremented after each transfer.
    fn tx_increment(&self) -> bool;
}

/// Marker which signals that `tx_address_count()` can be called multiple times.
///
/// The DMA code will never call `tx_address_count()` to request more than two buffers to configure
/// two DMA channels. In the case of peripherals, the function can always return the same values.
pub trait EndlessWriteTarget: WriteTarget {}

impl<B: WriteBuffer> WriteTarget for B {
    type TransmittedWord = <B as WriteBuffer>::Word;

    fn tx_treq() -> Option<u8> {
        None
    }

    fn tx_address_count(&mut self) -> (u32, u32) {
        let (ptr, len) = unsafe { self.write_buffer() };
        (ptr as u32, len as u32)
    }

    fn tx_increment(&self) -> bool {
        true
    }
}
