use core::fmt::Debug;

use embedded_hal::spi::SpiDevice;
use embedded_nand::{
    BlockIndex, BlockStatus, ByteAddress, ColumnAddress, ErrorType, NandFlash, NandFlashError,
    NandFlashErrorKind, PageIndex,
};

use crate::{
    async_trait::SpiNandAsync,
    blocking::{SpiFlashError, SpiNandBlocking},
};

use super::JedecID;

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
    /// Erase a block
    pub fn erase_block_blocking(&mut self, block: BlockIndex) -> Result<(), SpiFlashError<SPI>> {
        self.device.erase_block(&mut self.spi, block)
    }
    /// Read a page
    pub fn page_read_blocking(
        &mut self,
        page_address: PageIndex,
        buf: &mut [u8; N],
    ) -> Result<(), SpiFlashError<SPI>> {
        // Read page
        self.device.read_page(&mut self.spi, page_address, buf)
        // Check ECC if enabled
    }

    /// Read a slice of a page
    pub fn read_page_slice_blocking(
        &mut self,
        page_address: PageIndex,
        column_address: ColumnAddress,
        buf: &mut [u8],
    ) -> Result<(), SpiFlashError<SPI>> {
        // Read page
        self.device
            .read_page_slice(&mut self.spi, page_address, column_address, buf)
        // Check ECC if enabled
    }

    /// Write a page
    pub fn write_page_blocking(
        &mut self,
        page_address: PageIndex,
        buf: &[u8; N],
    ) -> Result<(), SpiFlashError<SPI>> {
        // Write page
        self.device.write_page(&mut self.spi, page_address, buf)
    }

    /// Write a slice of a page
    pub fn write_page_slice_blocking(
        &mut self,
        page_address: PageIndex,
        column_address: ColumnAddress,
        buf: &[u8],
    ) -> Result<(), SpiFlashError<SPI>> {
        // Write page
        self.device
            .write_page_slice(&mut self.spi, page_address, column_address, buf)
    }

    /// Mark a block as bad
    pub fn mark_block_bad_blocking(&mut self, block: BlockIndex) -> Result<(), SpiFlashError<SPI>> {
        self.device.mark_block_bad(&mut self.spi, block)
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
        address: PageIndex,
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
        page_address: PageIndex,
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
        page_address: PageIndex,
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
        address: PageIndex,
        buf: &mut [u8; N],
    ) -> Result<(), crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.page_read(address).await?;
        self.page_read_buffer(ColumnAddress::new(0), buf).await
    }

    /// Write a whole page to the device
    /// This will write the page to the buffer/register and then write it to the page
    pub async fn write_page(
        &mut self,
        address: PageIndex,
        buf: &[u8; N],
    ) -> Result<(), crate::async_trait::SpiFlashErrorASync<SPI>> {
        self.program_load(ColumnAddress::new(0), buf).await?;
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

impl<SPI: SpiDevice, D: SpiNandBlocking<SPI, N>, const N: usize> NandFlash for SpiFlash<SPI, D, N> {
    const READ_SIZE: usize = D::READ_SIZE as usize;
    const PAGE_SIZE: usize = D::PAGE_SIZE as usize;
    const BLOCK_COUNT: usize = D::BLOCK_COUNT as usize;
    const ERASE_SIZE: usize = D::BLOCK_SIZE as usize;
    const PAGES_PER_BLOCK: usize = D::PAGES_PER_BLOCK as usize;

    fn read(&mut self, offset: u32, mut bytes: &mut [u8]) -> Result<(), Self::Error> {
        let ba = ByteAddress::new(offset);
        let ca = ba.as_column_address(D::PAGE_SIZE);
        let mut pa = ba.as_page_index(D::PAGE_SIZE);
        // if ca.0 != 0 {
        //     // Not aligned to page
        //     // Read rest of page (or requested bytes)
        //     self.page_read_blocking(pa)?;
        //     // check if single read is enough
        //     let end = D::PAGE_SIZE as usize - ca.0 as usize;
        //     if end >= bytes.len() {
        //         return self.page_read_buffer_blocking(ca, bytes);
        //     }
        //     self.page_read_buffer_blocking(ca, &mut bytes[0..end])?;
        //     bytes = &mut bytes[end..];
        // }

        // for chunk in bytes.chunks_mut(D::PAGE_SIZE as usize) {
        //     self.page_read_blocking(pa)?;
        //     self.page_read_buffer_blocking(0.into(), chunk)?;
        //     pa.0 += 1;
        // }
        Ok(())
    }

    fn capacity(&self) -> u32 {
        D::CAPACITY
    }

    fn block_status(&mut self, block: BlockIndex) -> Result<BlockStatus, Self::Error> {
        if self.device.block_marked_bad(&mut self.spi, block)? {
            Ok(BlockStatus::Failed)
        } else {
            Ok(BlockStatus::Ok)
        }
    }
    const WRITE_SIZE: usize = D::PAGE_SIZE as usize;

    fn erase(&mut self, mut offset: u32, length: u32) -> Result<(), Self::Error> {
        loop {
            let block = ByteAddress::new(offset).as_block_index(D::BLOCK_SIZE);
            self.erase_block_blocking(block)?;

            offset += D::BLOCK_SIZE;
            if offset >= length {
                break;
            }
        }
        Ok(())
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        for chunk in bytes.chunks(D::PAGE_SIZE as usize) {
            let pa = ByteAddress::new(offset).as_page_index(D::PAGE_SIZE);
            // self.program_load_blocking(0.into(), chunk)?;
            // self.program_execute_blocking(pa)?;
        }
        Ok(())
    }

    fn erase_block(&mut self, block: BlockIndex) -> Result<(), Self::Error> {
        todo!()
    }

    fn copy(&mut self, src_offset: u32, dest_offset: u32, length: u32) -> Result<(), Self::Error> {
        todo!()
    }

    fn mark_block_bad(&mut self, block: BlockIndex) -> Result<(), Self::Error> {
        todo!()
    }
}
