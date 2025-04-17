//!
use embedded_hal_async::spi::SpiDevice;

use crate::SpiNand;

/// Async trait for SPI NAND flash devices
///
/// Implemntation of this trait allows the use of async functions is [crate::device::SpiFlash]
pub trait SpiNandAsync<SPI: SpiDevice, const N: usize>: SpiNand<N> {}

pub mod utils {
    use embedded_hal_async::spi::{Operation, SpiDevice};

    use crate::error::SpiFlashError;

    /// Wrapper around [SpiDevice::write] that maps errors
    pub async fn spi_write<SPI: SpiDevice>(
        spi: &mut SPI,
        buf: &[u8],
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        spi.write(buf).await.map_err(|e| SpiFlashError::SPI(e))
    }

    /// Wrapper around [SpiDevice::read] that maps errors
    pub async fn spi_read<SPI: SpiDevice>(
        spi: &mut SPI,
        buf: &mut [u8],
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        spi.read(buf).await.map_err(|e| SpiFlashError::SPI(e))
    }

    /// Wrapper around [SpiDevice::transfer] that maps errors
    pub async fn spi_transfer<SPI: SpiDevice>(
        spi: &mut SPI,
        read: &mut [u8],
        write: &[u8],
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        spi.transfer(read, write)
            .await
            .map_err(|e| SpiFlashError::SPI(e))
    }

    /// Wrapper around [SpiDevice::transfer_in_place] that maps errors
    pub async fn spi_transfer_in_place<SPI: SpiDevice>(
        spi: &mut SPI,
        buf: &mut [u8],
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        spi.transfer_in_place(buf)
            .await
            .map_err(|e| SpiFlashError::SPI(e))
    }

    /// Wrapper around [SpiDevice::transaction] that maps errors
    pub async fn spi_transaction<SPI: SpiDevice>(
        spi: &mut SPI,
        operations: &mut [Operation<'_, u8>],
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        spi.transaction(operations)
            .await
            .map_err(|e| SpiFlashError::SPI(e))
    }
}
