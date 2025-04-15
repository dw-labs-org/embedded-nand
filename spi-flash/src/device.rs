use core::fmt::Debug;

use embedded_hal::spi::SpiDevice;
use embedded_nand::{
    BlockStatus, ErrorType, NandFlash, NandFlashError, NandFlashErrorKind, ReadNandFlash,
};

use crate::{
    address::{BlockAddress, ByteAddress, PageAddress},
    async_trait::SpiNandAsync,
    blocking::{SpiFlashError, SpiNandBlocking},
};

use super::{address::ColumnAddress, JedecID};

#[derive(Debug, defmt::Format)]
pub struct SpiFlash<SPI, D, const N: usize> {
    pub spi: SPI,
    pub device: D,
    // page_size: usize,
}

impl<SPI, D, const N: usize> SpiFlash<SPI, D, N> {
    pub fn new(spi: SPI, device: D) -> Self {
        SpiFlash { spi, device }
    }
}

impl<SPI: SpiDevice, D: SpiNandBlocking<SPI, N>, const N: usize> SpiFlash<SPI, D, N> {
    /// Get the Jedec ID of the flash device
    pub fn jedec_blocking(&mut self) -> Result<JedecID, SpiFlashError<SPI>> {
        self.device.read_jedec_id_cmd(&mut self.spi)
    }
    /// Reset the flash device
    pub fn reset_blocking(&mut self) -> Result<(), SpiFlashError<SPI>> {
        self.device.reset_cmd(&mut self.spi)
    }
    /// Read status register 1
    pub fn read_status_register_1_blocking(&mut self) -> Result<u8, SpiFlashError<SPI>> {
        self.device.read_status_register_1_cmd(&mut self.spi)
    }

    /// Read status register 2
    pub fn read_status_register_2_blocking(&mut self) -> Result<u8, SpiFlashError<SPI>> {
        self.device.read_status_register_2_cmd(&mut self.spi)
    }

    /// Read status register 3
    pub fn read_status_register_3_blocking(&mut self) -> Result<u8, SpiFlashError<SPI>> {
        self.device.read_status_register_3_cmd(&mut self.spi)
    }

    /// Check if the device is busy
    pub fn is_busy_blocking(&mut self) -> Result<bool, SpiFlashError<SPI>> {
        self.device.is_busy(&mut self.spi)
    }

    /// Wait until the device is ready
    pub fn wait_ready_blocking(&mut self) -> Result<(), SpiFlashError<SPI>> {
        while self.is_busy_blocking()? {}
        Ok(())
    }

    /// Read a page into the device buffer/register
    /// Wait for the device to be ready
    pub fn page_read_blocking(&mut self, address: PageAddress) -> Result<bool, SpiFlashError<SPI>> {
        self.device.page_read_cmd(&mut self.spi, address)?;
        self.wait_ready_blocking()?;
        let ecc = self.device.check_ecc(&mut self.spi)?;
        match ecc {
            super::ECCStatus::Ok => Ok(true),
            super::ECCStatus::Corrected => Ok(true),
            super::ECCStatus::Failing => Ok(false),
            super::ECCStatus::Failed => Err(SpiFlashError::ReadFailed),
        }
    }

    /// Read bytes of a page from the device buffer/register starting from column address
    pub fn page_read_buffer_blocking(
        &mut self,
        ca: ColumnAddress,
        buf: &mut [u8],
    ) -> Result<(), SpiFlashError<SPI>> {
        self.device.page_read_buffer_cmd(&mut self.spi, ca, buf)
    }

    /// Enable writing to the flash device
    pub fn write_enable_blocking(&mut self) -> Result<(), SpiFlashError<SPI>> {
        self.device.write_enable_cmd(&mut self.spi)
    }

    /// Disable writing to the flash device
    pub fn write_disable_blocking(&mut self) -> Result<(), SpiFlashError<SPI>> {
        self.device.write_disable_cmd(&mut self.spi)
    }

    /// Erase a block of flash memory
    /// Checks the busy flag until complete
    /// Checks erase failed flag
    pub fn erase_block_blocking(
        &mut self,
        block_address: BlockAddress,
    ) -> Result<(), SpiFlashError<SPI>> {
        self.device.erase_block_cmd(&mut self.spi, block_address)?;
        self.wait_ready_blocking()?;
        if self.erase_failed_blocking()? {
            Err(SpiFlashError::EraseFailed)
        } else {
            Ok(())
        }
    }

    /// Check if writing to the device is enabled
    pub fn is_write_enabled_blocking(&mut self) -> Result<bool, SpiFlashError<SPI>> {
        self.device.is_write_enabled(&mut self.spi)
    }

    /// Check if programming failed
    /// This is only valid after a write operation
    pub fn program_failed_blocking(&mut self) -> Result<bool, SpiFlashError<SPI>> {
        self.device.program_failed(&mut self.spi)
    }

    /// Check if erasing failed
    /// This is only valid after an erase operation
    pub fn erase_failed_blocking(&mut self) -> Result<bool, SpiFlashError<SPI>> {
        self.device.erase_failed(&mut self.spi)
    }

    /// Write bytes to the device buffer/register, enable writing first.
    /// This will reset the buffer/register to 0xFF
    pub fn program_load_blocking(
        &mut self,
        ca: ColumnAddress,
        buf: &[u8],
    ) -> Result<(), SpiFlashError<SPI>> {
        self.write_enable_blocking()?;
        self.device.program_load_cmd(&mut self.spi, ca, buf)
    }

    /// Write buffer to page, wait until completes, check for program failure
    pub fn program_execute_blocking(
        &mut self,
        page_address: PageAddress,
    ) -> Result<(), SpiFlashError<SPI>> {
        self.device
            .program_execute_cmd(&mut self.spi, page_address)?;
        self.wait_ready_blocking()?;
        if self.program_failed_blocking()? {
            Err(SpiFlashError::ProgramFailed)
        } else {
            Ok(())
        }
    }

    /// Read a whole page from the device
    /// This will read the page into the device buffer/register and then read it to the buffer
    pub fn read_page_blocking(
        &mut self,
        address: PageAddress,
        buf: &mut [u8; N],
    ) -> Result<(), SpiFlashError<SPI>> {
        self.page_read_blocking(address)?;
        self.page_read_buffer_blocking(ColumnAddress(0), buf)
    }

    /// Write a whole page to the device
    /// This will write the page to the buffer/register and then write it to the page
    pub fn write_page_blocking(
        &mut self,
        address: PageAddress,
        buf: &[u8; N],
    ) -> Result<(), SpiFlashError<SPI>> {
        self.program_load_blocking(ColumnAddress(0), buf)?;
        self.program_execute_blocking(address)
    }

    /// Enter deep power down mode
    pub fn deep_power_down_blocking(&mut self) -> Result<(), SpiFlashError<SPI>> {
        self.device.deep_power_down_cmd(&mut self.spi)
    }

    /// Exit deep power down mode
    pub fn deep_power_down_exit_blocking(&mut self) -> Result<(), SpiFlashError<SPI>> {
        self.device.deep_power_down_exit_cmd(&mut self.spi)
    }
}

impl<SPI: embedded_hal_async::spi::SpiDevice, D: SpiNandAsync<SPI, N>, const N: usize>
    SpiFlash<SPI, D, N>
{
    /// Get the Jedec ID of the flash device
    pub async fn jedec(&mut self) -> Result<JedecID, crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.device.read_jedec_id(&mut self.spi).await
    }

    /// Reset the flash device
    pub async fn reset(&mut self) -> Result<(), crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.device.reset(&mut self.spi).await
    }

    /// Read status register 1
    pub async fn read_status_register_1(
        &mut self,
    ) -> Result<u8, crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.device.read_status_register_1(&mut self.spi).await
    }

    /// Read status register 2
    pub async fn read_status_register_2(
        &mut self,
    ) -> Result<u8, crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.device.read_status_register_2(&mut self.spi).await
    }

    /// Read status register 3
    pub async fn read_status_register_3(
        &mut self,
    ) -> Result<u8, crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.device.read_status_register_3(&mut self.spi).await
    }

    /// Check if the device is busy
    pub async fn is_busy(&mut self) -> Result<bool, crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.device.is_busy(&mut self.spi).await
    }

    /// Wait until the device is ready
    pub async fn wait_ready(&mut self) -> Result<(), crate::async_trait::SpiFlashErrorASync<SPI>> {
        while self.is_busy().await? {}
        Ok(())
    }

    /// Read a page into the device buffer/register
    pub async fn page_read(
        &mut self,
        address: PageAddress,
    ) -> Result<bool, crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.device.page_read(&mut self.spi, address).await?;
        self.wait_ready().await?;
        let ecc = self.device.check_ecc(&mut self.spi).await?;
        match ecc {
            super::ECCStatus::Ok => Ok(true),
            super::ECCStatus::Corrected => Ok(true),
            super::ECCStatus::Failing => Ok(false),
            super::ECCStatus::Failed => Err(crate::async_trait::SpiFlashErrorASync::ReadFailed),
        }
    }

    /// Read bytes of a page from the device buffer/register starting from column address
    pub async fn page_read_buffer(
        &mut self,
        ca: ColumnAddress,
        buf: &mut [u8],
    ) -> Result<(), crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.device.page_read_buffer(&mut self.spi, ca, buf).await
    }

    /// Enable writing to the flash device
    pub async fn write_enable(
        &mut self,
    ) -> Result<(), crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.device.write_enable(&mut self.spi).await
    }

    /// Disable writing to the flash device
    pub async fn write_disable(
        &mut self,
    ) -> Result<(), crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.device.write_disable(&mut self.spi).await
    }

    /// Erase a block of flash memory
    /// Checks the busy flag until complete
    /// Checks erase failed flag
    pub async fn erase_block(
        &mut self,
        page_address: PageAddress,
    ) -> Result<(), crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.device.erase_block(&mut self.spi, page_address).await?;
        self.wait_ready().await?;
        if self.erase_failed().await? {
            Err(crate::async_trait::SpiFlashErrorASync::EraseFailed)
        } else {
            Ok(())
        }
    }

    /// Check if writing to the device is enabled
    pub async fn is_write_enabled(
        &mut self,
    ) -> Result<bool, crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.device.is_write_enabled(&mut self.spi).await
    }

    /// Check if programming failed
    /// This is only valid after a write operation
    pub async fn program_failed(
        &mut self,
    ) -> Result<bool, crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.device.program_failed(&mut self.spi).await
    }

    /// Check if erasing failed
    /// This is only valid after an erase operation
    pub async fn erase_failed(
        &mut self,
    ) -> Result<bool, crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.device.erase_failed(&mut self.spi).await
    }

    /// Load bytes to the device buffer/register, enable writing first.
    /// This will reset the buffer/register to 0xFF
    pub async fn program_load(
        &mut self,
        ca: ColumnAddress,
        buf: &[u8],
    ) -> Result<(), crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.write_enable().await?;
        self.device.program_load(&mut self.spi, ca, buf).await
    }

    /// Write buffer to page, wait until completes, check for program failure
    pub async fn program_execute(
        &mut self,
        page_address: PageAddress,
    ) -> Result<(), crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.device
            .program_execute(&mut self.spi, page_address)
            .await?;
        self.wait_ready().await?;
        if self.program_failed().await? {
            Err(crate::async_trait::SpiFlashErrorASync::ProgramFailed)
        } else {
            Ok(())
        }
    }

    /// Read a whole page from the device
    /// This will read the page into the device buffer/register and then read it to the buffer
    pub async fn read_page(
        &mut self,
        address: PageAddress,
        buf: &mut [u8; N],
    ) -> Result<(), crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.page_read(address).await?;
        self.page_read_buffer(ColumnAddress(0), buf).await
    }

    /// Write a whole page to the device
    /// This will write the page to the buffer/register and then write it to the page
    pub async fn write_page(
        &mut self,
        address: PageAddress,
        buf: &[u8; N],
    ) -> Result<(), crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.program_load(ColumnAddress(0), buf).await?;
        self.program_execute(address).await
    }

    /// Disable block protection
    pub async fn disable_block_protection(
        &mut self,
    ) -> Result<(), crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.device.disable_block_protection(&mut self.spi).await
    }

    /// Enter deep power down
    pub async fn deep_power_down(
        &mut self,
    ) -> Result<(), crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.device.deep_power_down(&mut self.spi).await
    }
    /// Exit deep power down
    pub async fn deep_power_down_exit(
        &mut self,
    ) -> Result<(), crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.device.deep_power_down_exit(&mut self.spi).await
    }
}

impl<SPI: SpiDevice, D, const N: usize> ErrorType for SpiFlash<SPI, D, N> {
    type Error = SpiFlashError<SPI>;
}

impl<SPI: SpiDevice> NandFlashError for SpiFlashError<SPI> {
    fn kind(&self) -> NandFlashErrorKind {
        todo!()
    }
}
impl<SPI: SpiDevice, D: SpiNandBlocking<SPI, N>, const N: usize> ReadNandFlash
    for SpiFlash<SPI, D, N>
{
    const READ_SIZE: usize = D::READ_SIZE as usize;

    fn read(&mut self, offset: u32, mut bytes: &mut [u8]) -> Result<(), Self::Error> {
        let ba = ByteAddress(offset);
        let ca = ba.as_column_address(D::PAGE_SIZE);
        let mut pa = ba.as_page_address(D::PAGE_SIZE);
        if ca.0 != 0 {
            // Not aligned to page
            // Read rest of page (or requested bytes)
            self.page_read_blocking(pa)?;
            // check if single read is enough
            let end = D::PAGE_SIZE as usize - ca.0 as usize;
            if end >= bytes.len() {
                return self.page_read_buffer_blocking(ca, bytes);
            }
            self.page_read_buffer_blocking(ca, &mut bytes[0..end])?;
            bytes = &mut bytes[end..];
        }

        for chunk in bytes.chunks_mut(D::PAGE_SIZE as usize) {
            self.page_read_blocking(pa)?;
            self.page_read_buffer_blocking(0.into(), chunk)?;
            pa.0 += 1;
        }
        Ok(())
    }

    fn capacity(&self) -> u32 {
        D::CAPACITY
    }

    fn block_status(&mut self, address: u32) -> Result<BlockStatus, Self::Error> {
        if self
            .device
            .block_marked_bad(&mut self.spi, address.into())?
        {
            Ok(BlockStatus::Failed)
        } else {
            Ok(BlockStatus::Ok)
        }
    }
}

impl<SPI: SpiDevice, D: SpiNandBlocking<SPI, N>, const N: usize> NandFlash for SpiFlash<SPI, D, N> {
    const WRITE_SIZE: usize = D::PAGE_SIZE as usize;
    const ERASE_SIZE: usize = D::BLOCK_SIZE as usize;

    fn erase(&mut self, mut offset: u32, length: u32) -> Result<(), Self::Error> {
        loop {
            let pa = ByteAddress(offset).as_block_address(D::BLOCK_SIZE);
            self.erase_block_blocking(pa)?;

            offset += D::BLOCK_SIZE;
            if offset >= length {
                break;
            }
        }
        Ok(())
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        for chunk in bytes.chunks(D::PAGE_SIZE as usize) {
            let pa = ByteAddress(offset).as_page_address(D::PAGE_SIZE);
            self.program_load_blocking(0.into(), chunk)?;
            self.program_execute_blocking(pa)?;
        }
        Ok(())
    }

    fn mark_bad(&mut self, address: u32) -> Result<(), Self::Error> {
        todo!()
    }
}
