//! Hardware checksum engine

use bl602_pac::CKS;

/// Checksum engine abstraction
///
/// # Examples
///
/// ```no_run
/// use bl602_hal::pac;
/// use bl602_hal::checksum::{Checksum, Endianness};
///
/// fn main() -> ! {
///     let dp = pac::Peripherals::take().unwrap();
///     let checksum = Checksum::new(dp.CKS, Endianness::Little);
///
///     checksum.write(&[
///         0x45, 0x00, 0x00, 0x73, 0x00, 0x00, 0x40, 0x00, 0x40, 0x11, 0x00, 0x00, 0xc0, 0xa8, 0x00,
///         0x01, 0xc0, 0xa8, 0x00, 0xc7,
///     ]);
///
///     assert_eq!(checksum.result(), u16::from_be_bytes([0xb8, 0x61]));
///
///     loop {}
/// }
/// ```
pub struct Checksum {
    cks: CKS,
}

/// The endianness used when computing checksums.
pub enum Endianness {
    /// Big endian
    Big,
    /// Little endian
    Little,
}

impl Checksum {
    /// Resets the CKS state and returns a new `Checksum` instance that is configured to compute
    /// checksums with the given `endianness`.
    ///
    /// This takes ownership of the `CKS` peripheral to ensure that the state won't be modified or
    /// reset somewhere else
    pub fn new(cks: CKS, endianness: Endianness) -> Self {
        let checksum = Self { cks };

        checksum.reset(endianness);

        checksum
    }

    /// Resets the CKS peripheral while setting the `endianness`.
    #[inline(always)]
    pub fn reset(&self, endianness: Endianness) {
        self.cks.cks_config.write(|w| {
            // Set `cr_cks_clr` to `1` in order to clear the checksum engine state
            w.cr_cks_clr()
                .set_bit()
                // Set the `cr_cks_byte_swap` bit to 1 when big endian, 0 when little endian.
                .cr_cks_byte_swap()
                .bit(match endianness {
                    Endianness::Big => true,
                    Endianness::Little => false,
                })
        });
    }

    /// Sets the `endianness` of the checksum engine.
    #[inline(always)]
    pub fn set_endianness(&self, endianness: Endianness) {
        // Set the `cr_cks_byte_swap` bit to 1 when big endian, 0 when little endian.
        self.cks.cks_config.write(|w| {
            w.cr_cks_byte_swap().bit(match endianness {
                Endianness::Big => true,
                Endianness::Little => false,
            })
        });
    }

    /// Writes the given slice of `bytes` to the checksum engine, one at a time.
    #[inline(always)]
    pub fn write(&self, bytes: &[u8]) {
        for byte in bytes {
            self.cks.data_in.write(|w| unsafe { w.bits(*byte as u32) });
        }
    }

    /// Reads the computed 16-bit result from the checksum engine.
    #[inline(always)]
    pub fn result(&self) -> u16 {
        self.cks.cks_out.read().bits() as u16
    }

    /// Releases the checksum (`CKS`) peripheral.
    pub fn free(self) -> CKS {
        self.cks
    }
}
