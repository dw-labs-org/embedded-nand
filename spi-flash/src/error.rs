use core::fmt::Debug;
use embedded_nand::NandFlashErrorKind;

/// Error type for the SPI flash driver.
///
/// This error type is used for both blocking and async SPI flash drivers.
/// It is generic over the SPI error type (SE), which allows for different SPI implementations.
/// This is either [`embedded_hal::spi::Error`] or [`embedded_hal_async::spi::Error`].
///
///
#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SpiFlashError<SE> {
    /// Error from the SPI peripheral
    #[error("SpiDevice error: {0}")]
    SPI(SE),
    /// Block Erase failed.
    /// This can happen if the block is protected, write is disabled or block has failed.
    #[error("Erase failed")]
    EraseFailed,
    /// Program failed.
    /// This can happen if the write is disabled, block is protected or the block has failed.
    #[error("Program failed")]
    ProgramFailed,
    /// Read failed
    /// This can happen due to an ECC error
    #[error("Read failed")]
    ReadFailed,
    /// Requested bytes out of bounds
    #[error("Requested bytes out of bounds")]
    OutOfBounds,
    /// Requested bytes not aligned
    #[error("Requested bytes not aligned")]
    NotAligned,
    /// Other error
    #[error("Other error. Should not happen")]
    Other,
}

// This impl is only for the helper check bounds / alignment functions for auto conversion from errors
impl<SE> From<NandFlashErrorKind> for SpiFlashError<SE> {
    fn from(kind: NandFlashErrorKind) -> Self {
        match kind {
            NandFlashErrorKind::NotAligned => SpiFlashError::NotAligned,
            NandFlashErrorKind::OutOfBounds => SpiFlashError::OutOfBounds,
            _ => SpiFlashError::Other,
        }
    }
}
