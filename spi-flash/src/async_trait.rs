use embedded_hal_async::spi::{Operation, SpiDevice};
use embedded_nand::{ColumnAddress, PageIndex};
use utils::{spi_transaction, spi_transfer, spi_transfer_in_place, spi_write};

use crate::{ECCStatus, JedecID, SpiNand};

// #[cfg(target_feature = "defmt")]
#[derive(defmt::Format)]
pub enum SpiFlashErrorASync<SPI: SpiDevice> {
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

#[allow(async_fn_in_trait)]
pub trait SpiNandAsync<SPI: SpiDevice, const N: usize>: SpiNand<N> {
    /// Issue a reset command to the flash device
    async fn reset(&self, spi: &mut SPI) -> Result<(), SpiFlashErrorASync<SPI>> {
        spi_write(spi, &[Self::RESET_COMMAND]).await
    }

    /// Read the JEDEC ID of the flash device
    /// By default reads the first byte
    // TODO: #1 Read the full JEDEC ID
    async fn read_jedec_id(&self, spi: &mut SPI) -> Result<JedecID, SpiFlashErrorASync<SPI>> {
        let mut buf = [0; 2];
        spi_transfer(spi, &mut buf, &[Self::JEDEC_COMMAND, 0]).await?;
        Ok(JedecID::new(buf[1], 1))
    }

    /// Read  status register 1
    async fn read_status_register_1(&self, spi: &mut SPI) -> Result<u8, SpiFlashErrorASync<SPI>> {
        let mut buf = [Self::STATUS_REGISTER_READ_COMMAND, 0xA0, 0];
        spi_transfer_in_place(spi, &mut buf).await?;
        Ok(buf[2])
    }

    /// Read status register 2
    async fn read_status_register_2(&self, spi: &mut SPI) -> Result<u8, SpiFlashErrorASync<SPI>> {
        let mut buf = [Self::STATUS_REGISTER_READ_COMMAND, 0xB0, 0];
        spi_transfer_in_place(spi, &mut buf).await?;
        Ok(buf[2])
    }

    /// Read status register 3
    async fn read_status_register_3(&self, spi: &mut SPI) -> Result<u8, SpiFlashErrorASync<SPI>> {
        let mut buf = [Self::STATUS_REGISTER_READ_COMMAND, 0xC0, 0];
        spi_transfer_in_place(spi, &mut buf).await?;
        Ok(buf[2])
    }

    /// Check if busy flag is set
    async fn is_busy(&self, spi: &mut SPI) -> Result<bool, SpiFlashErrorASync<SPI>> {
        let status = self.read_status_register_3(spi).await?;
        Ok((status & 0x01) != 0)
    }

    /// Read a page into the device buffer/register
    async fn page_read(
        &self,
        spi: &mut SPI,
        address: PageIndex,
    ) -> Result<(), SpiFlashErrorASync<SPI>> {
        let pa = address.as_u32();
        let buf = [
            Self::PAGE_READ_COMMAND,
            (pa >> 16) as u8,
            (pa >> 8) as u8,
            pa as u8,
        ];
        spi_write(spi, &buf).await
    }

    /// Read bytes of a page from the device buffer/register starting from column address
    async fn page_read_buffer(
        &self,
        spi: &mut SPI,
        ca: ColumnAddress,
        buf: &mut [u8],
    ) -> Result<(), SpiFlashErrorASync<SPI>> {
        let ca = ca.as_u16();
        spi_transaction(
            spi,
            &mut [
                Operation::Write(&[Self::PAGE_READ_BUFFER_COMMAND, (ca >> 8) as u8, ca as u8, 0]),
                Operation::Read(buf),
            ],
        )
        .await
    }

    /// Check the ECC flags after a page read
    async fn check_ecc(&self, spi: &mut SPI) -> Result<ECCStatus, SpiFlashErrorASync<SPI>> {
        let status = self.read_status_register_3(spi).await? & 0x30;
        match status {
            0x00 => Ok(ECCStatus::Ok),
            0x10 => Ok(ECCStatus::Corrected),
            0x20 => Ok(ECCStatus::Failed),
            _ => Ok(ECCStatus::Failing),
        }
    }

    /// Check if the block is marked as bad
    async fn block_marked_bad(
        &self,
        spi: &mut SPI,
        address: PageIndex,
    ) -> Result<bool, SpiFlashErrorASync<SPI>> {
        // Read page into the buffer
        self.page_read(spi, address).await?;
        // Read the first byte of the extra data
        let mut buf = [0; 1];
        self.page_read_buffer(spi, ColumnAddress::new(Self::PAGE_SIZE as u16), &mut buf)
            .await?;
        Ok(buf[0] != 0xFF)
    }

    /// Enable writing to the flash device
    async fn write_enable(&self, spi: &mut SPI) -> Result<(), SpiFlashErrorASync<SPI>> {
        spi_write(spi, &[Self::WRITE_ENABLE_COMMAND]).await
    }

    /// Disable writing to the flash device
    async fn write_disable(&self, spi: &mut SPI) -> Result<(), SpiFlashErrorASync<SPI>> {
        spi_write(spi, &[Self::WRITE_DISABLE_COMMAND]).await
    }

    /// Check if write protection is enabled
    async fn is_write_enabled(&self, spi: &mut SPI) -> Result<bool, SpiFlashErrorASync<SPI>> {
        Ok((self.read_status_register_3(spi).await? & 0x02) != 0)
    }

    /// Check if programming/writing failed
    async fn program_failed(&self, spi: &mut SPI) -> Result<bool, SpiFlashErrorASync<SPI>> {
        Ok((self.read_status_register_3(spi).await? & 0x08) != 0)
    }

    /// Check if erase failed
    async fn erase_failed(&self, spi: &mut SPI) -> Result<bool, SpiFlashErrorASync<SPI>> {
        Ok((self.read_status_register_3(spi).await? & 0x04) != 0)
    }

    /// Write to status register 1
    /// This is used to set the block protection bits and status protection bits
    async fn write_status_register_1(
        &self,
        spi: &mut SPI,
        data: u8,
    ) -> Result<(), SpiFlashErrorASync<SPI>> {
        spi_write(spi, &[Self::STATUS_REGISTER_WRITE_COMMAND, 0xA0, data]).await
    }

    /// Disable block protection
    async fn disable_block_protection(&self, spi: &mut SPI) -> Result<(), SpiFlashErrorASync<SPI>> {
        let reg = self.read_status_register_1(spi).await?;
        self.write_status_register_1(spi, reg & 0b10000111).await
    }

    /// Erase a block of flash memory
    async fn erase_block(
        &self,
        spi: &mut SPI,
        page_address: PageIndex,
    ) -> Result<(), SpiFlashErrorASync<SPI>> {
        let address = page_address.as_u32();
        // Enable writing first
        self.write_enable(spi).await?;
        spi_write(
            spi,
            &[
                Self::BLOCK_ERASE_COMMAND,
                (address >> 16) as u8,
                (address >> 8) as u8,
                address as u8,
            ],
        )
        .await
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
    async fn program_load(
        &self,
        spi: &mut SPI,
        ca: ColumnAddress,
        buf: &[u8],
    ) -> Result<(), SpiFlashErrorASync<SPI>> {
        let ca = ca.as_u16();
        let data = [Self::PROGRAM_LOAD_COMMAND, (ca >> 8) as u8, ca as u8];
        spi_transaction(spi, &mut [Operation::Write(&data), Operation::Write(buf)]).await
    }

    /// Write bytes to the device buffer/register without resetting
    ///
    /// Use [SpiNandWrite::write_enable] to enable writing before this command
    ///
    /// Use [SpiNandWrite::program_execute] to write the buffer/register to a page
    ///
    /// Use [SpiNandWrite::program_load] to write with resetting
    async fn program_random_load(
        &self,
        spi: &mut SPI,
        ca: ColumnAddress,
        buf: &[u8],
    ) -> Result<(), SpiFlashErrorASync<SPI>> {
        let ca = ca.as_u16();
        let data = [Self::PROGRAM_RANDOM_LOAD_COMMAND, (ca >> 8) as u8, ca as u8];
        spi_transaction(spi, &mut [Operation::Write(&data), Operation::Write(buf)]).await
    }

    /// Write the device buffer/register to a page
    ///
    /// Use [SpiNandWrite::program_load] or [SpiNandWrite::program_random_load] to write to the buffer/register    
    ///
    /// Use [SpiNandRead::is_busy] to check when the write is complete
    ///
    /// Check [SpiNandWrite::program_failed] to see if the write failed
    async fn program_execute(
        &self,
        spi: &mut SPI,
        address: PageIndex,
    ) -> Result<(), SpiFlashErrorASync<SPI>> {
        let pa = address.as_u32();
        let data = [
            Self::PROGRAM_EXECUTE_COMMAND,
            (pa >> 16) as u8,
            (pa >> 8) as u8,
            pa as u8,
        ];
        spi_write(spi, &data).await
    }

    /// Enter deep power down.
    /// Call [SpiNandAsync::deep_power_down_exit] to exit
    async fn deep_power_down(&self, spi: &mut SPI) -> Result<(), SpiFlashErrorASync<SPI>> {
        spi_write(spi, &[Self::DEEP_POWER_DOWN_COMMAND]).await
    }
    /// Exit deep power down
    async fn deep_power_down_exit(&self, spi: &mut SPI) -> Result<(), SpiFlashErrorASync<SPI>> {
        spi_write(spi, &[Self::DEEP_POWER_DOWN_EXIT_COMMAND]).await
    }
}

pub mod utils {
    use embedded_hal_async::spi::{Operation, SpiDevice};

    use super::SpiFlashErrorASync;

    /// Wrapper around [SpiDevice::write] that maps errors
    pub async fn spi_write<SPI: SpiDevice>(
        spi: &mut SPI,
        buf: &[u8],
    ) -> Result<(), SpiFlashErrorASync<SPI>> {
        spi.write(buf).await.map_err(|e| SpiFlashErrorASync::SPI(e))
    }

    /// Wrapper around [SpiDevice::read] that maps errors
    pub async fn spi_read<SPI: SpiDevice>(
        spi: &mut SPI,
        buf: &mut [u8],
    ) -> Result<(), SpiFlashErrorASync<SPI>> {
        spi.read(buf).await.map_err(|e| SpiFlashErrorASync::SPI(e))
    }

    /// Wrapper around [SpiDevice::transfer] that maps errors
    pub async fn spi_transfer<SPI: SpiDevice>(
        spi: &mut SPI,
        read: &mut [u8],
        write: &[u8],
    ) -> Result<(), SpiFlashErrorASync<SPI>> {
        spi.transfer(read, write)
            .await
            .map_err(|e| SpiFlashErrorASync::SPI(e))
    }

    /// Wrapper around [SpiDevice::transfer_in_place] that maps errors
    pub async fn spi_transfer_in_place<SPI: SpiDevice>(
        spi: &mut SPI,
        buf: &mut [u8],
    ) -> Result<(), SpiFlashErrorASync<SPI>> {
        spi.transfer_in_place(buf)
            .await
            .map_err(|e| SpiFlashErrorASync::SPI(e))
    }

    /// Wrapper around [SpiDevice::transaction] that maps errors
    pub async fn spi_transaction<SPI: SpiDevice>(
        spi: &mut SPI,
        operations: &mut [Operation<'_, u8>],
    ) -> Result<(), SpiFlashErrorASync<SPI>> {
        spi.transaction(operations)
            .await
            .map_err(|e| SpiFlashErrorASync::SPI(e))
    }
}
