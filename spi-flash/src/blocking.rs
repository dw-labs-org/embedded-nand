use address::{ColumnAddress, PageAddress};
use embedded_hal::spi::{Operation, SpiDevice};
use utils::{spi_transaction, spi_transfer, spi_transfer_in_place, spi_write};

use crate::{address, ECCStatus, JedecID, SpiNand};

// #[cfg(target_feature = "defmt")]
#[derive(defmt::Format)]
pub enum SpiFlashError<SPI: SpiDevice> {
    /// Error from the SPI peripheral
    SPI(SPI::Error),
    /// Block Erase failed.
    /// This can happen if the block is protected, write is disabled or block has failed.
    EraseFailed,
    /// Program failed.
    /// This can happen if the write is disabled, block is protected or the block has failed.
    ProgramFailed,
    /// Read failed
    /// This can happen due to an ECC error
    ReadFailed,
}

pub trait SpiNandBlocking<SPI: SpiDevice, const N: usize>: SpiNand<N> {
    /// Issue a reset command to the flash device
    fn reset(&self, spi: &mut SPI) -> Result<(), SpiFlashError<SPI>> {
        spi_write(spi, &[Self::RESET_COMMAND])
    }

    /// Read the JEDEC ID of the flash device
    /// By default reads the first byte
    // TODO: #1 Read the full JEDEC ID
    fn read_jedec_id(&self, spi: &mut SPI) -> Result<JedecID, SpiFlashError<SPI>> {
        let mut buf = [0; 2];
        spi_transfer(spi, &mut buf, &[Self::JEDEC_COMMAND, 0])?;
        Ok(JedecID::new(buf[1], 1))
    }

    /// Read  status register 1
    fn read_status_register_1(&self, spi: &mut SPI) -> Result<u8, SpiFlashError<SPI>> {
        let mut buf = [Self::STATUS_REGISTER_READ_COMMAND, 0xA0, 0];
        spi_transfer_in_place(spi, &mut buf)?;
        Ok(buf[2])
    }

    /// Read status register 2
    fn read_status_register_2(&self, spi: &mut SPI) -> Result<u8, SpiFlashError<SPI>> {
        let mut buf = [Self::STATUS_REGISTER_READ_COMMAND, 0xB0, 0];
        spi_transfer_in_place(spi, &mut buf)?;
        Ok(buf[2])
    }

    /// Read status register 3
    fn read_status_register_3(&self, spi: &mut SPI) -> Result<u8, SpiFlashError<SPI>> {
        let mut buf = [Self::STATUS_REGISTER_READ_COMMAND, 0xC0, 0];
        spi_transfer_in_place(spi, &mut buf)?;
        Ok(buf[2])
    }

    /// Check if busy flag is set
    fn is_busy(&self, spi: &mut SPI) -> Result<bool, SpiFlashError<SPI>> {
        let status = self.read_status_register_3(spi)?;
        Ok((status & 0x01) != 0)
    }

    /// Read a page into the device buffer/register
    fn page_read(&self, spi: &mut SPI, address: PageAddress) -> Result<(), SpiFlashError<SPI>> {
        let pa = address.0;
        let buf = [
            Self::PAGE_READ_COMMAND,
            (pa >> 16) as u8,
            (pa >> 8) as u8,
            pa as u8,
        ];
        spi_write(spi, &buf)
    }

    /// Read bytes of a page from the device buffer/register starting from column address
    fn page_read_buffer(
        &self,
        spi: &mut SPI,
        ca: ColumnAddress,
        buf: &mut [u8],
    ) -> Result<(), SpiFlashError<SPI>> {
        spi_transaction(
            spi,
            &mut [
                Operation::Write(&[
                    Self::PAGE_READ_BUFFER_COMMAND,
                    (ca.0 >> 8) as u8,
                    ca.0 as u8,
                    0,
                ]),
                Operation::Read(buf),
            ],
        )
    }

    /// Check the ECC flags after a page read
    fn check_ecc(&self, spi: &mut SPI) -> Result<ECCStatus, SpiFlashError<SPI>> {
        let status = self.read_status_register_3(spi)? & 0x30;
        match status {
            0x00 => Ok(ECCStatus::Ok),
            0x10 => Ok(ECCStatus::Corrected),
            0x20 => Ok(ECCStatus::Failed),
            _ => Ok(ECCStatus::Failing),
        }
    }

    /// Check if the block is marked as bad
    fn block_marked_bad(
        &self,
        spi: &mut SPI,
        address: PageAddress,
    ) -> Result<bool, SpiFlashError<SPI>> {
        // Read page into the buffer
        self.page_read(spi, address)?;
        // Read the first byte of the extra data
        let mut buf = [0; 1];
        self.page_read_buffer(spi, ColumnAddress(Self::PAGE_SIZE as u16), &mut buf)?;
        Ok(buf[0] != 0xFF)
    }

    /// Enable writing to the flash device
    fn write_enable(&self, spi: &mut SPI) -> Result<(), SpiFlashError<SPI>> {
        spi_write(spi, &[Self::WRITE_ENABLE_COMMAND])
    }

    /// Disable writing to the flash device
    fn write_disable(&self, spi: &mut SPI) -> Result<(), SpiFlashError<SPI>> {
        spi_write(spi, &[Self::WRITE_DISABLE_COMMAND])
    }

    /// Check if write protection is enabled
    fn is_write_enabled(&self, spi: &mut SPI) -> Result<bool, SpiFlashError<SPI>> {
        Ok((self.read_status_register_3(spi)? & 0x02) != 0)
    }

    /// Check if programming/writing failed
    fn program_failed(&self, spi: &mut SPI) -> Result<bool, SpiFlashError<SPI>> {
        Ok((self.read_status_register_3(spi)? & 0x08) != 0)
    }

    /// Check if erase failed
    fn erase_failed(&self, spi: &mut SPI) -> Result<bool, SpiFlashError<SPI>> {
        Ok((self.read_status_register_3(spi)? & 0x04) != 0)
    }

    /// Write to status register 1
    /// This is used to set the block protection bits and status protection bits
    fn write_status_register_1(&self, spi: &mut SPI, data: u8) -> Result<(), SpiFlashError<SPI>> {
        spi_write(spi, &[Self::STATUS_REGISTER_WRITE_COMMAND, 0xA0, data])
    }

    /// Disable block protection
    /// Writes bits 3 to 6 as 0 in status register 1 (after reading)
    fn disable_block_protection(&self, spi: &mut SPI) -> Result<(), SpiFlashError<SPI>> {
        let reg = self.read_status_register_1(spi)?;
        self.write_status_register_1(spi, reg & 0b10000111)
    }

    /// Erase a block of flash memory
    fn erase_block(
        &self,
        spi: &mut SPI,
        page_address: PageAddress,
    ) -> Result<(), SpiFlashError<SPI>> {
        let address = page_address.0;
        // Enable writing first
        self.write_enable(spi)?;
        spi_write(
            spi,
            &[
                Self::BLOCK_ERASE_COMMAND,
                (address >> 16) as u8,
                (address >> 8) as u8,
                address as u8,
            ],
        )
    }

    /// Write bytes to the device buffer/register
    ///
    /// This will reset the buffer/register to 0xFF
    ///
    /// Use [SpiNandWrite::write_enable] to enable writing before this command
    ///
    /// Use [SpiNandWrite::program_random_load] to write without resetting
    ///
    /// Use [SpiNandWrite::program_execute] to write the buffer/register to a page
    ///
    fn program_load(
        &self,
        spi: &mut SPI,
        ca: ColumnAddress,
        buf: &[u8],
    ) -> Result<(), SpiFlashError<SPI>> {
        let data = [Self::PROGRAM_LOAD_COMMAND, (ca.0 >> 8) as u8, ca.0 as u8];
        spi_transaction(spi, &mut [Operation::Write(&data), Operation::Write(buf)])
    }

    /// Write bytes to the device buffer/register without resetting
    ///
    /// Use [SpiNandWrite::write_enable] to enable writing before this command
    ///
    /// Use [SpiNandWrite::program_execute] to write the buffer/register to a page
    ///
    /// Use [SpiNandWrite::program_load] to write with resetting
    fn program_random_load(
        &self,
        spi: &mut SPI,
        ca: ColumnAddress,
        buf: &[u8],
    ) -> Result<(), SpiFlashError<SPI>> {
        let data = [
            Self::PROGRAM_RANDOM_LOAD_COMMAND,
            (ca.0 >> 8) as u8,
            ca.0 as u8,
        ];
        spi_transaction(spi, &mut [Operation::Write(&data), Operation::Write(buf)])
    }

    /// Write the device buffer/register to a page
    ///
    /// Use [SpiNandWrite::program_load] or [SpiNandWrite::program_random_load] to write to the buffer/register    
    ///
    /// Use [SpiNandRead::is_busy] to check when the write is complete
    ///
    /// Check [SpiNandWrite::program_failed] to see if the write failed
    fn program_execute(
        &self,
        spi: &mut SPI,
        address: PageAddress,
    ) -> Result<(), SpiFlashError<SPI>> {
        let pa = address.0;
        let data = [
            Self::PROGRAM_EXECUTE_COMMAND,
            (pa >> 16) as u8,
            (pa >> 8) as u8,
            pa as u8,
        ];
        spi_write(spi, &data)
    }
}

pub mod utils {
    use embedded_hal::spi::{Operation, SpiDevice};

    use super::SpiFlashError;

    /// Wrapper around [SpiDevice::write] that maps errors
    pub fn spi_write<SPI: SpiDevice>(spi: &mut SPI, buf: &[u8]) -> Result<(), SpiFlashError<SPI>> {
        spi.write(buf).map_err(|e| SpiFlashError::SPI(e))
    }

    /// Wrapper around [SpiDevice::read] that maps errors
    pub fn spi_read<SPI: SpiDevice>(
        spi: &mut SPI,
        buf: &mut [u8],
    ) -> Result<(), SpiFlashError<SPI>> {
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
}
