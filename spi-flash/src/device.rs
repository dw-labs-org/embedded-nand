use core::fmt::Debug;

use embedded_hal::spi::SpiDevice;
use embedded_nand::{
    BlockStatus, ErrorType, NandFlash, NandFlashError, NandFlashErrorKind, ReadNandFlash,
};

use crate::{
    address::{ByteAddress, PageAddress},
    blocking::{SpiFlashError, SpiNandBlocking},
};

use super::{address::ColumnAddress, JedecID};

#[derive(Debug, defmt::Format)]
pub struct SpiFlash<SPI, D> {
    pub spi: SPI,
    pub device: D,
}

impl<SPI, D> SpiFlash<SPI, D> {
    pub fn new(spi: SPI, device: D) -> Self {
        SpiFlash { spi, device }
    }
}

impl<SPI: SpiDevice, D: SpiNandBlocking<SPI>> SpiFlash<SPI, D> {
    /// Get the Jedec ID of the flash device
    pub fn jedec(&mut self) -> Result<JedecID, SpiFlashError<SPI>> {
        self.device.read_jedec_id(&mut self.spi)
    }
    /// Reset the flash device
    pub fn reset(&mut self) -> Result<(), SpiFlashError<SPI>> {
        self.device.reset(&mut self.spi)
    }
    /// Read status register 1
    pub fn read_status_register_1(&mut self) -> Result<u8, SpiFlashError<SPI>> {
        self.device.read_status_register_1(&mut self.spi)
    }

    /// Read status register 2
    pub fn read_status_register_2(&mut self) -> Result<u8, SpiFlashError<SPI>> {
        self.device.read_status_register_2(&mut self.spi)
    }

    /// Read status register 3
    pub fn read_status_register_3(&mut self) -> Result<u8, SpiFlashError<SPI>> {
        self.device.read_status_register_3(&mut self.spi)
    }

    /// Check if the device is busy
    pub fn is_busy(&mut self) -> Result<bool, SpiFlashError<SPI>> {
        self.device.is_busy(&mut self.spi)
    }

    /// Wait until the device is ready
    pub fn wait_ready(&mut self) -> Result<(), SpiFlashError<SPI>> {
        while self.is_busy()? {}
        Ok(())
    }

    /// Read a page into the device buffer/register
    /// Wait for the device to be ready
    pub fn page_read(&mut self, address: PageAddress) -> Result<bool, SpiFlashError<SPI>> {
        self.device.page_read(&mut self.spi, address)?;
        self.wait_ready()?;
        let ecc = self.device.check_ecc(&mut self.spi)?;
        match ecc {
            super::ECCStatus::Ok => Ok(true),
            super::ECCStatus::Corrected => Ok(true),
            super::ECCStatus::Failing => Ok(false),
            super::ECCStatus::Failed => Err(SpiFlashError::ReadFailed),
        }
    }

    /// Read bytes of a page from the device buffer/register starting from column address
    pub fn page_read_buffer(
        &mut self,
        ca: ColumnAddress,
        buf: &mut [u8],
    ) -> Result<(), SpiFlashError<SPI>> {
        self.device.page_read_buffer(&mut self.spi, ca, buf)
    }

    /// Enable writing to the flash device
    pub fn write_enable(&mut self) -> Result<(), SpiFlashError<SPI>> {
        self.device.write_enable(&mut self.spi)
    }

    /// Disable writing to the flash device
    pub fn write_disable(&mut self) -> Result<(), SpiFlashError<SPI>> {
        self.device.write_disable(&mut self.spi)
    }

    /// Erase a block of flash memory
    /// Checks the busy flag until complete
    /// Checks erase failed flag
    pub fn erase_block(&mut self, page_address: PageAddress) -> Result<(), SpiFlashError<SPI>> {
        self.device.erase_block(&mut self.spi, page_address)?;
        self.wait_ready()?;
        if self.erase_failed()? {
            Err(SpiFlashError::EraseFailed)
        } else {
            Ok(())
        }
    }

    /// Check if writing to the device is enabled
    pub fn is_write_enabled(&mut self) -> Result<bool, SpiFlashError<SPI>> {
        self.device.is_write_enabled(&mut self.spi)
    }

    /// Check if programming failed
    /// This is only valid after a write operation
    pub fn program_failed(&mut self) -> Result<bool, SpiFlashError<SPI>> {
        self.device.program_failed(&mut self.spi)
    }

    /// Check if erasing failed
    /// This is only valid after an erase operation
    pub fn erase_failed(&mut self) -> Result<bool, SpiFlashError<SPI>> {
        self.device.erase_failed(&mut self.spi)
    }

    /// Write bytes to the device buffer/register, enable writing first.
    /// This will reset the buffer/register to 0xFF
    pub fn program_load(
        &mut self,
        ca: ColumnAddress,
        buf: &[u8],
    ) -> Result<(), SpiFlashError<SPI>> {
        self.write_enable()?;
        self.device.program_load(&mut self.spi, ca, buf)
    }

    /// Write buffer to page, wait until completes, check for program failure
    pub fn program_execute(&mut self, page_address: PageAddress) -> Result<(), SpiFlashError<SPI>> {
        self.device.program_execute(&mut self.spi, page_address)?;
        self.wait_ready()?;
        if self.program_failed()? {
            Err(SpiFlashError::ProgramFailed)
        } else {
            Ok(())
        }
    }
}

impl<SPI: SpiDevice, D> ErrorType for SpiFlash<SPI, D> {
    type Error = SpiFlashError<SPI>;
}

impl<SPI: SpiDevice> NandFlashError for SpiFlashError<SPI> {
    fn kind(&self) -> NandFlashErrorKind {
        todo!()
    }
}
impl<SPI: SpiDevice, D: SpiNandBlocking<SPI>> ReadNandFlash for SpiFlash<SPI, D> {
    const READ_SIZE: usize = D::READ_SIZE as usize;

    fn read(&mut self, offset: u32, mut bytes: &mut [u8]) -> Result<(), Self::Error> {
        let ba = ByteAddress(offset);
        let ca = ba.as_column_address(D::PAGE_SIZE);
        let mut pa = ba.as_page_address(D::PAGE_SIZE);
        if ca.0 != 0 {
            // Not aligned to page
            // Read rest of page (or requested bytes)
            self.page_read(pa)?;
            // check if single read is enough
            let end = D::PAGE_SIZE as usize - ca.0 as usize;
            if end >= bytes.len() {
                return self.page_read_buffer(ca, bytes);
            }
            self.page_read_buffer(ca, &mut bytes[0..end])?;
            bytes = &mut bytes[end..];
        }

        for chunk in bytes.chunks_mut(D::PAGE_SIZE as usize) {
            self.page_read(pa)?;
            self.page_read_buffer(0.into(), chunk)?;
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

impl<SPI: SpiDevice, D: SpiNandBlocking<SPI>> NandFlash for SpiFlash<SPI, D> {
    const WRITE_SIZE: usize = D::PAGE_SIZE as usize;
    const ERASE_SIZE: usize = D::BLOCK_SIZE as usize;

    fn erase(&mut self, mut offset: u32, length: u32) -> Result<(), Self::Error> {
        loop {
            let pa = ByteAddress(offset).as_page_address(D::PAGE_SIZE);
            self.erase_block(pa)?;

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
            self.program_load(0.into(), chunk)?;
            self.program_execute(pa)?;
        }
        Ok(())
    }

    fn mark_bad(&mut self, address: u32) -> Result<(), Self::Error> {
        todo!()
    }
}
