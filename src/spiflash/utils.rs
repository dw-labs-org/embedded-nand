use embedded_hal::spi::{Operation, SpiDevice};

use super::SpiFlashError;

/// Wrapper around [SpiDevice::write] that maps errors
pub fn spi_write<SPI: SpiDevice>(spi: &mut SPI, buf: &[u8]) -> Result<(), SpiFlashError<SPI>> {
    spi.write(buf).map_err(|e| SpiFlashError::SPI(e))
}

/// Wrapper around [SpiDevice::read] that maps errors
pub fn spi_read<SPI: SpiDevice>(spi: &mut SPI, buf: &mut [u8]) -> Result<(), SpiFlashError<SPI>> {
    spi.read(buf).map_err(|e| SpiFlashError::SPI(e))
}

/// Wrapper around [SpiDevice::transfer] that maps errors
pub fn spi_transfer<SPI: SpiDevice>(
    spi: &mut SPI,
    read: &mut [u8],
    write: &[u8],
) -> Result<(), SpiFlashError<SPI>> {
    spi.transfer(read, write).map_err(|e| SpiFlashError::SPI(e))
}

/// Wrapper around [SpiDevice::transfer_in_place] that maps errors
pub fn spi_transfer_in_place<SPI: SpiDevice>(
    spi: &mut SPI,
    buf: &mut [u8],
) -> Result<(), SpiFlashError<SPI>> {
    spi.transfer_in_place(buf)
        .map_err(|e| SpiFlashError::SPI(e))
}

/// Wrapper around [SpiDevice::transaction] that maps errors
pub fn spi_transaction<SPI: SpiDevice>(
    spi: &mut SPI,
    operations: &mut [Operation<'_, u8>],
) -> Result<(), SpiFlashError<SPI>> {
    spi.transaction(operations)
        .map_err(|e| SpiFlashError::SPI(e))
}
