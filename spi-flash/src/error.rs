use embedded_hal::spi::SpiDevice;
use embedded_nand::NandFlashErrorKind;

#[derive(thiserror::Error)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SpiFlashError<SPI: SpiDevice> {
    /// Error from the SPI peripheral
    #[error("SpiDevice error: {0}")]
    SPI(SPI::Error),
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

impl<SPI: SpiDevice> core::fmt::Debug for SpiFlashError<SPI> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SpiFlashError::SPI(e) => write!(f, "SPI error: {:?}", e),
            SpiFlashError::EraseFailed => write!(f, "Erase failed"),
            SpiFlashError::ProgramFailed => write!(f, "Program failed"),
            SpiFlashError::ReadFailed => write!(f, "Read failed"),
            SpiFlashError::OutOfBounds => write!(f, "Out of bounds"),
            SpiFlashError::NotAligned => write!(f, "Not aligned"),
            SpiFlashError::Other => write!(f, "Other error"),
        }
    }
}

// This impl is only for the helper check bounds / alignment function for auto conversion
impl<SPI: SpiDevice> From<NandFlashErrorKind> for SpiFlashError<SPI> {
    fn from(kind: NandFlashErrorKind) -> Self {
        match kind {
            NandFlashErrorKind::NotAligned => SpiFlashError::NotAligned,
            NandFlashErrorKind::OutOfBounds => SpiFlashError::OutOfBounds,
            _ => SpiFlashError::Other,
        }
    }
}
