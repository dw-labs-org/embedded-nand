use core::fmt::Debug;

use embedded_hal::spi::SpiDevice;
use embedded_nand::{
    check_erase, check_read, check_slice, check_write, AddressConversions, BlockIndex, BlockStatus,
    ByteAddress, ColumnAddress, ErrorType, NandFlash, PageIndex,
};

use crate::{cmd_blocking::SpiNandBlocking, error::SpiFlashError};

use super::JedecID;

/// Concrete type that implements all the flash device features.
///
/// This type is generic over the SPI peripheral and the flash device.
/// The aim is to support a wide range of SPI Nand devices by implementing the
/// defacto standard commands in the [SpiNandBlocking] and [SpiNandAsync] traits.
/// These are configurable to some extent by the [crate::SpiNand] trait, which also
/// defines the layout of the device.
///
/// For overwriting specific commands, the functions in [SpiNandBlocking] and
/// [SpiNandAsync] should be overwritten.
///
/// [SpiNandDevice] implements the [embedded_nand::NandFlash] trait, which provides
/// an abstraction for NAND flash devices which a flash translation layer (FTL) /
/// bad block management (BBM) / wear levelling algorithm or file system can use.
///
/// To use the blocking interface, the device D must implement [SpiNandBlocking]
/// and SPI must implement [embedded_hal::spi::SpiDevice].
///
/// To use the async interface, the device D must implement [SpiNandAsync]
/// and SPI must implement [embedded_hal_async::spi::SpiDevice].
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SpiNandDevice<SPI, D, const N: usize> {
    pub spi: SPI,
    pub device: D,
}
// Manually implement Debug to avoid bounds on SPI
// D must implement Debug, which should be fine as its just data
impl<SPI, D, const N: usize> Debug for SpiNandDevice<SPI, D, N>
where
    D: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SpiFlash")
            .field("device", &self.device)
            .finish()
    }
}

impl<SPI, D, const N: usize> SpiNandDevice<SPI, D, N> {
    /// Create a new [SpiNandDevice] with the given SPI peripheral and flash device.
    pub fn new(spi: SPI, device: D) -> Self {
        SpiNandDevice { spi, device }
    }
}

impl<SPI: SpiDevice, D: SpiNandBlocking<SPI, N>, const N: usize> SpiNandDevice<SPI, D, N> {
    /// Get the Jedec ID of the flash device using blocking SPI
    pub fn jedec_blocking(&mut self) -> Result<JedecID, SpiFlashError<SPI::Error>> {
        self.device.read_jedec_id_cmd(&mut self.spi)
    }
    /// Reset the flash device using blocking SPI
    pub fn reset_blocking(&mut self) -> Result<(), SpiFlashError<SPI::Error>> {
        self.device.reset_cmd(&mut self.spi)
    }
    /// Erase a block of flash memory using blocking SPI
    pub fn erase_block_blocking(
        &mut self,
        block: BlockIndex,
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        self.device.erase_block(&mut self.spi, block)
    }
    /// Read a page into the buffer using blocking SPI
    /// Checks for ECC errors
    pub fn read_page_blocking(
        &mut self,
        page_address: PageIndex,
        buf: &mut [u8; N],
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        // Read page
        self.device.read_page(&mut self.spi, page_address, buf)
        // Check ECC if enabled
    }

    /// Read a slice of a page using blocking SPI
    /// Checks for ECC errors
    pub fn read_page_slice_blocking(
        &mut self,
        page_address: PageIndex,
        column_address: ColumnAddress,
        buf: &mut [u8],
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        // Read page
        self.device
            .read_page_slice(&mut self.spi, page_address, column_address, buf)
        // Check ECC if enabled
    }

    /// Write a page to the device using blocking SPI
    /// This will overwrite the entire page
    /// The page must be erased before writing
    pub fn write_page_blocking(
        &mut self,
        page_address: PageIndex,
        buf: &[u8; N],
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        // Write page
        self.device.write_page(&mut self.spi, page_address, buf)
    }

    /// Write a slice of a page to the device using blocking SPI
    /// This will overwrite the slice of the page
    /// The state of unwritten bytes is defined by the device
    pub fn write_page_slice_blocking(
        &mut self,
        page_address: PageIndex,
        column_address: ColumnAddress,
        buf: &[u8],
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        // Write page
        self.device
            .write_page_slice(&mut self.spi, page_address, column_address, buf)
    }

    /// Copy a page to another using the device buffer
    /// TODO: This might not be supported by all devices
    pub fn copy_page_blocking(
        &mut self,
        src_page_address: PageIndex,
        dest_page_address: PageIndex,
    ) -> Result<(), SpiFlashError<SPI::Error>> {
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

    /// Mark a block as bad using blocking SPI
    /// This will mark the block as bad in the device
    /// This is device specific and not guaranteed to work on all devices
    pub fn mark_block_bad_blocking(
        &mut self,
        block: BlockIndex,
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        self.device.mark_block_bad(&mut self.spi, block)
    }
}

impl<SPI: SpiDevice, D, const N: usize> ErrorType for SpiNandDevice<SPI, D, N> {
    type Error = SpiFlashError<SPI::Error>;
}

impl<SPI: SpiDevice, D: SpiNandBlocking<SPI, N>, const N: usize> NandFlash
    for SpiNandDevice<SPI, D, N>
{
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
