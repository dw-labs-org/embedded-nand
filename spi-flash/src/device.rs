use core::fmt::Debug;

use embedded_hal::spi::SpiDevice;
use embedded_nand::{
    check_erase, check_read, check_slice, check_write, AddressConversions, BlockIndex, BlockStatus,
    ByteAddress, ColumnAddress, ErrorType, NandFlash, NandFlashError, NandFlashErrorKind,
    PageIndex,
};

use crate::{
    async_trait::SpiNandAsync,
    blocking::{SpiFlashError, SpiNandBlocking},
};

use super::JedecID;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SpiFlash<SPI, D, const N: usize> {
    pub spi: SPI,
    pub device: D,
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
    pub fn read_page_blocking(
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

    /// Copy a page to another using the device buffer
    pub fn copy_page_blocking(
        &mut self,
        src_page_address: PageIndex,
        dest_page_address: PageIndex,
    ) -> Result<(), SpiFlashError<SPI>> {
        // Load the page into the device buffer
        self.device.page_read_cmd(&mut self.spi, src_page_address)?;
        // Write the page to the destination address
        self.device
            .program_execute_cmd(&mut self.spi, dest_page_address)?;
        // Wait until the device is ready
        while self.device.is_busy(&mut self.spi)? {}
        // Return the status of the operation
        if self.device.program_failed(&mut self.spi)? {
            return Err(SpiFlashError::ProgramFailed);
        }
        Ok(())
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
        match self {
            SpiFlashError::NotAligned => NandFlashErrorKind::NotAligned,
            SpiFlashError::OutOfBounds => NandFlashErrorKind::OutOfBounds,
            SpiFlashError::SPI(_) => NandFlashErrorKind::Other,
            SpiFlashError::EraseFailed => NandFlashErrorKind::BlockFail(None),
            SpiFlashError::ProgramFailed => NandFlashErrorKind::BlockFail(None),
            SpiFlashError::ReadFailed => NandFlashErrorKind::BlockFail(None),
            SpiFlashError::Other => NandFlashErrorKind::Other,
        }
    }
}

impl<SPI: SpiDevice, D: SpiNandBlocking<SPI, N>, const N: usize> NandFlash for SpiFlash<SPI, D, N> {
    const READ_SIZE: usize = D::READ_SIZE as usize;
    const PAGE_SIZE: usize = D::PAGE_SIZE as usize;
    const BLOCK_COUNT: usize = D::BLOCK_COUNT as usize;
    const ERASE_SIZE: usize = D::BLOCK_SIZE as usize;
    const PAGES_PER_BLOCK: usize = D::PAGES_PER_BLOCK as usize;
    const WRITE_SIZE: usize = 1;

    fn read(&mut self, offset: u32, mut bytes: &mut [u8]) -> Result<(), Self::Error> {
        trace!("Reading {} bytes from offset {}", bytes.len(), offset);
        // Check that the requested read is aligned and within bounds
        check_read(self, offset, bytes.len())?;

        // Check if the first page is whole
        let ba = ByteAddress::new(offset);
        let ca = ba.as_column_address(D::PAGE_SIZE);
        let mut pa = ba.as_page_index(D::PAGE_SIZE);
        if ca.as_u16() != 0 {
            // Not aligned to page
            // number of bytes in rest of page
            let remaining = D::PAGE_SIZE as usize - ca.as_u16() as usize;
            // number of bytes to read
            let read_len = if bytes.len() > remaining {
                remaining
            } else {
                bytes.len()
            };
            trace!(
                "Partial read {} bytes from page {} column {}",
                read_len,
                pa.as_u32(),
                ca.as_u16()
            );
            // read first page into buffer
            self.read_page_slice_blocking(pa, ca, bytes[..read_len].as_mut())?;
            // remove first non full page from bytes
            bytes = &mut bytes[read_len..];
            // increment page address
            pa.inc();
        }

        // read full pages.
        // If already read all bytes, 0 iterations
        for chunk in bytes.chunks_mut(D::PAGE_SIZE as usize) {
            // read page into buffer
            self.read_page_slice_blocking(pa, ColumnAddress::new(0), chunk)?;
            // increment page address
            pa.inc();
        }
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

    fn erase(&mut self, offset: u32, length: u32) -> Result<(), Self::Error> {
        trace!("Erasing {} bytes from offset {}", length, offset);
        // Check that the requested erase is aligned and within bounds
        check_erase(self, offset, offset + length)?;

        let start_block = Self::raw_byte_to_block_index(offset);
        let end_block = Self::raw_byte_to_block_index(offset + length);

        // Be nice to use an iterator here, but custom range is nightly only
        for block in (start_block.as_u16())..(end_block.as_u16()) {
            self.erase_block(BlockIndex::new(block))?;
        }
        Ok(())
    }

    fn write(&mut self, offset: u32, mut bytes: &[u8]) -> Result<(), Self::Error> {
        trace!("Writing {} bytes to offset {}", bytes.len(), offset);
        // Check that the requested write is aligned and within bounds
        check_write(self, offset, bytes.len())?;

        // Check if the first page is whole
        let ba = ByteAddress::new(offset);
        let ca = ba.as_column_address(D::PAGE_SIZE);
        let mut pa = ba.as_page_index(D::PAGE_SIZE);
        if ca.as_u16() != 0 {
            // Not aligned to page
            // number of bytes in rest of page
            let remaining = D::PAGE_SIZE as usize - ca.as_u16() as usize;
            // number of bytes to read
            let read_len = if bytes.len() > remaining {
                remaining
            } else {
                bytes.len()
            };
            // write first page into buffer
            self.write_page_slice_blocking(pa, ca, &bytes[..read_len])?;
            // remove first non full page from bytes
            bytes = &bytes[read_len..];
            // increment page address
            pa.inc();
        }

        // Write the remaining full and final partial/full page
        for chunk in bytes.chunks(D::PAGE_SIZE as usize) {
            // write page into buffer
            self.write_page_slice_blocking(pa, ColumnAddress::new(0), chunk)?;
            // increment page address
            pa.inc();
        }
        Ok(())
    }

    fn erase_block(&mut self, block: BlockIndex) -> Result<(), Self::Error> {
        trace!("Erasing block {}", block.as_u16());
        // check range
        if block.as_u16() >= Self::BLOCK_COUNT as u16 {
            return Err(SpiFlashError::OutOfBounds);
        }
        // erase
        self.erase_block_blocking(block)
    }

    fn copy(&mut self, src_offset: u32, dest_offset: u32, length: u32) -> Result<(), Self::Error> {
        // Check that both read and write are aligned with pages and within bounds
        check_slice(self, Self::PAGE_SIZE, src_offset, length as usize)?;
        check_slice(self, Self::PAGE_SIZE, dest_offset, length as usize)?;

        // Iterate over pages
        let n_pages = length / Self::PAGE_SIZE as u32;
        let mut src_page = Self::byte_to_page_index(ByteAddress::new(src_offset));
        let mut dest_page = Self::byte_to_page_index(ByteAddress::new(dest_offset));
        for _ in 0..n_pages {
            // Copy the page
            self.copy_page_blocking(src_page, dest_page)?;
            // Increment the page addresses
            src_page.inc();
            dest_page.inc();
        }
        Ok(())
    }

    fn mark_block_bad(&mut self, block: BlockIndex) -> Result<(), Self::Error> {
        debug!("Marking block {} as bad", block.as_u16());
        // check range
        if block.as_u16() >= Self::BLOCK_COUNT as u16 {
            return Err(SpiFlashError::OutOfBounds);
        }
        // mark bad
        self.mark_block_bad_blocking(block)
    }
}
