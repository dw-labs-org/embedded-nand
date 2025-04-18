#![allow(async_fn_in_trait)]

use embedded_hal::spi::Operation;
use embedded_hal_async::spi::SpiDevice;
use embedded_nand::{BlockIndex, ColumnAddress, PageIndex};
use utils::{spi_transaction, spi_transfer_in_place, spi_write};

use crate::{error::SpiFlashError, JedecID, SpiNand};

/// Blocking SPI NAND flash trait.
/// Contains the low level, mostly single SPI operation commands.
///
/// Some compound functions are provided including getting specific status flags,
/// and read/write/execute functions including the required write enable,
/// waiting and checking for errors.
///
/// Any changes made to these default functions must match the behaviour
///
/// The default implementations are fairly generic and should work for most SPI NAND flash devices.
/// Look to make changes to the [SpiNand] trait first to change the default behavior.
/// If this isn't possible, override the default function(s).
///
/// For async implementations, see [crate::async_trait::SpiNandAsync].
pub trait SpiNandAsync<SPI: SpiDevice, const N: usize>: SpiNand<N> {
    // ============= Commands =============

    /// Issue a reset command to the flash device
    async fn reset_cmd(&self, spi: &mut SPI) -> Result<(), SpiFlashError<SPI::Error>> {
        spi_write(spi, &[Self::RESET_COMMAND]).await
    }

    /// Issue a hard reset command to the flash device
    async fn hard_reset_cmd(&self, spi: &mut SPI) -> Result<(), SpiFlashError<SPI::Error>> {
        spi_write(spi, &[Self::RESET_ENABLE_COMMAND, Self::HARD_RESET_COMMAND]).await
    }

    /// Read the JEDEC ID of the flash device
    /// command then byte then ID then 2 device ID bytes    
    async fn read_jedec_id_cmd(&self, spi: &mut SPI) -> Result<JedecID, SpiFlashError<SPI::Error>> {
        let mut buf = [Self::JEDEC_COMMAND, 0, 0, 0, 0];
        spi_transfer_in_place(spi, &mut buf).await?;
        Ok(JedecID::new(buf[2], ((buf[3] as u16) << 8) + buf[4] as u16))
    }

    /// Read a register
    /// Warning: does not check if the register is valid
    async fn read_register_cmd(
        &self,
        spi: &mut SPI,
        register: u8,
    ) -> Result<u8, SpiFlashError<SPI::Error>> {
        let mut buf = [Self::STATUS_REGISTER_READ_COMMAND, register, 0];
        spi_transfer_in_place(spi, &mut buf).await?;
        Ok(buf[2])
    }

    /// Write a register
    /// Warning: does not check if the register is valid
    /// Warning: Some registers / bits are not writable
    async fn write_register_cmd(
        &self,
        spi: &mut SPI,
        register: u8,
        data: u8,
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        spi_write(spi, &[Self::STATUS_REGISTER_WRITE_COMMAND, register, data]).await
    }

    /// Set bits in a register
    async fn set_register_cmd(
        &self,
        spi: &mut SPI,
        register: u8,
        mask: u8,
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        let data = self.read_register_cmd(spi, register).await? | mask;
        self.write_register_cmd(spi, register, data).await
    }

    /// Clear bits in a register
    async fn clear_register_cmd(
        &self,
        spi: &mut SPI,
        register: u8,
        mask: u8,
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        let data = self.read_register_cmd(spi, register).await? & mask;
        self.write_register_cmd(spi, register, data).await
    }

    /// Read a page into the device buffer/register
    async fn page_read_cmd(
        &self,
        spi: &mut SPI,
        address: PageIndex,
    ) -> Result<(), SpiFlashError<SPI::Error>> {
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
    async fn page_read_buffer_cmd(
        &self,
        spi: &mut SPI,
        ca: ColumnAddress,
        buf: &mut [u8],
    ) -> Result<(), SpiFlashError<SPI::Error>> {
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

    /// Enable writing to the flash device
    async fn write_enable_cmd(&self, spi: &mut SPI) -> Result<(), SpiFlashError<SPI::Error>> {
        spi_write(spi, &[Self::WRITE_ENABLE_COMMAND]).await
    }

    /// Disable writing to the flash device
    async fn write_disable_cmd(&self, spi: &mut SPI) -> Result<(), SpiFlashError<SPI::Error>> {
        spi_write(spi, &[Self::WRITE_DISABLE_COMMAND]).await
    }

    /// Write to status register 1
    /// This is used to set the block protection bits and status protection bits
    async fn write_status_register_1_cmd(
        &self,
        spi: &mut SPI,
        data: u8,
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        spi_write(spi, &[Self::STATUS_REGISTER_WRITE_COMMAND, 0xA0, data]).await
    }

    /// Erase a block of flash memory
    async fn erase_block_cmd(
        &self,
        spi: &mut SPI,
        block_address: BlockIndex,
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        let address = PageIndex::from_block_address(block_address, Self::PAGES_PER_BLOCK).as_u32();
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
    /// Use [SpiNandBlocking::write_enable] to enable writing before this command
    ///
    /// Use [SpiNandBlocking::program_random_load] to write without resetting
    ///
    /// Use [SpiNandBlocking::program_execute] to write the buffer/register to a page
    ///
    async fn program_load_cmd(
        &self,
        spi: &mut SPI,
        ca: ColumnAddress,
        buf: &[u8],
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        let ca = ca.as_u16();
        let data = [Self::PROGRAM_LOAD_COMMAND, (ca >> 8) as u8, ca as u8];
        spi_transaction(spi, &mut [Operation::Write(&data), Operation::Write(buf)]).await
    }

    /// Write bytes to the device buffer/register without resetting
    ///
    /// Use [SpiNandBlocking::write_enable] to enable writing before this command
    ///
    /// Use [SpiNandBlocking::program_execute] to write the buffer/register to a page
    ///
    /// Use [SpiNandBlocking::program_load] to write with resetting
    async fn program_random_load_cmd(
        &self,
        spi: &mut SPI,
        ca: ColumnAddress,
        buf: &[u8],
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        let ca = ca.as_u16();
        let data = [Self::PROGRAM_RANDOM_LOAD_COMMAND, (ca >> 8) as u8, ca as u8];
        spi_transaction(spi, &mut [Operation::Write(&data), Operation::Write(buf)]).await
    }

    /// Write the device buffer/register to a page
    ///
    /// Use [SpiNandBlocking::program_load] or [SpiNandWrite::program_random_load] to write to the buffer/register    
    ///
    /// Use [SpiNandBlocking::is_busy] to check when the write is complete
    ///
    /// Check [SpiNandBlocking::program_failed] to see if the write failed
    async fn program_execute_cmd(
        &self,
        spi: &mut SPI,
        address: PageIndex,
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        let pa = address.as_u32();
        let data = [
            Self::PROGRAM_EXECUTE_COMMAND,
            (pa >> 16) as u8,
            (pa >> 8) as u8,
            pa as u8,
        ];
        spi_write(spi, &data).await
    }

    /// Put the device in deep power down mode
    /// Requires callling [SpiNandBlocking::deep_power_down_exit] to exit
    async fn deep_power_down_cmd(&self, spi: &mut SPI) -> Result<(), SpiFlashError<SPI::Error>> {
        spi_write(spi, &[Self::DEEP_POWER_DOWN_COMMAND]).await
    }

    /// Exit deep power down mode
    async fn deep_power_down_exit_cmd(
        &self,
        spi: &mut SPI,
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        spi_write(spi, &[Self::DEEP_POWER_DOWN_EXIT_COMMAND]).await
    }

    // ============= Status functions ============

    /// Check if write protection is enabled
    async fn is_write_enabled(&self, spi: &mut SPI) -> Result<bool, SpiFlashError<SPI::Error>> {
        Ok((self.read_register_cmd(spi, Self::STATUS_REGISTER).await? & 0x02) != 0)
    }

    /// Check if programming/writing failed
    async fn program_failed(&self, spi: &mut SPI) -> Result<bool, SpiFlashError<SPI::Error>> {
        Ok((self.read_register_cmd(spi, Self::STATUS_REGISTER).await? & 0x08) != 0)
    }

    /// Check if erase failed
    async fn erase_failed(&self, spi: &mut SPI) -> Result<bool, SpiFlashError<SPI::Error>> {
        Ok((self.read_register_cmd(spi, Self::STATUS_REGISTER).await? & 0x04) != 0)
    }

    /// Check if busy flag is set
    async fn is_busy(&self, spi: &mut SPI) -> Result<bool, SpiFlashError<SPI::Error>> {
        let status = self.read_register_cmd(spi, Self::STATUS_REGISTER).await?;
        Ok((status & 0x01) != 0)
    }

    /// Disable block protection
    /// Sets bits 3 to 6 as 0 in status register 1
    async fn disable_block_protection(
        &self,
        spi: &mut SPI,
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        self.clear_register_cmd(spi, Self::CONFIGURATION_REGISTER, 0b1111000)
            .await
    }

    // ============ Bad Block functions ============
    /// Check if the block is marked as bad
    async fn block_marked_bad(
        &self,
        spi: &mut SPI,
        block_address: BlockIndex,
    ) -> Result<bool, SpiFlashError<SPI::Error>> {
        // Read the first 2 bytes of the extra data
        let mut buf = [0; 2];
        self.read_page_slice(
            spi,
            PageIndex::from_block_address(block_address, Self::PAGES_PER_BLOCK),
            ColumnAddress::new(Self::PAGE_SIZE as u16),
            &mut buf,
        )
        .await?;
        Ok(buf[0] != 0xFF || buf[1] != 0xFF)
    }

    /// Mark a block as bad
    /// This will write 0x00 to the 2nd byte of the extra data.
    ///
    /// Returns true if sucessful
    async fn mark_block_bad(
        &self,
        spi: &mut SPI,
        block_address: BlockIndex,
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        let pa = PageIndex::from_block_address(block_address, Self::PAGES_PER_BLOCK);
        // Erase the block
        self.erase_block(spi, block_address).await?;
        // Write to the 2nd byte in the extra data
        self.write_page_slice(
            spi,
            pa,
            ColumnAddress::new(Self::PAGE_SIZE as u16 + 1),
            &[0],
        )
        .await
    }

    // ============= RWE functions =============
    /// Erase a block
    async fn erase_block(
        &self,
        spi: &mut SPI,
        block_address: BlockIndex,
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        // Enable writing
        self.write_enable_cmd(spi).await?;
        // Erase the block
        self.erase_block_cmd(spi, block_address).await?;
        // Wait for the erase to complete
        while self.is_busy(spi).await? {}
        // Check if the erase failed
        if self.erase_failed(spi).await? {
            return Err(SpiFlashError::EraseFailed);
        }
        Ok(())
    }

    /// Read a page from the device
    async fn read_page(
        &self,
        spi: &mut SPI,
        page_address: PageIndex,
        buf: &mut [u8; N],
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        // Read page into device buffer
        self.page_read_cmd(spi, page_address).await?;
        // Read the page from the device buffer
        self.page_read_buffer_cmd(spi, ColumnAddress::new(0), buf)
            .await
    }

    /// Read a slice from a page
    async fn read_page_slice(
        &self,
        spi: &mut SPI,
        page_address: PageIndex,
        column_address: ColumnAddress,
        buf: &mut [u8],
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        // Read page into device buffer
        self.page_read_cmd(spi, page_address).await?;
        // Wait for the read to complete
        while self.is_busy(spi).await? {}
        // Read the page from the device buffer
        self.page_read_buffer_cmd(spi, column_address, buf).await
    }

    /// Write a page to the device.
    ///
    /// Must use [SpiNandBlocking::block_erase] first
    async fn write_page(
        &self,
        spi: &mut SPI,
        page_address: PageIndex,
        buf: &[u8; N],
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        // Enable writing
        self.write_enable_cmd(spi).await?;
        // Write to the device buffer
        self.program_load_cmd(spi, ColumnAddress::new(0), buf)
            .await?;
        // Write the buffer to the page
        self.program_execute_cmd(spi, page_address).await?;
        // Wait for the write to complete
        while self.is_busy(spi).await? {}
        // Check if the write failed
        if self.program_failed(spi).await? {
            return Err(SpiFlashError::ProgramFailed);
        }
        Ok(())
    }

    /// Write a slice to a page
    ///
    /// Must use [SpiNandBlocking::block_erase] first
    async fn write_page_slice(
        &self,
        spi: &mut SPI,
        page_address: PageIndex,
        column_address: ColumnAddress,
        buf: &[u8],
    ) -> Result<(), SpiFlashError<SPI::Error>> {
        // Enable writing
        self.write_enable_cmd(spi).await?;
        // Write to the device buffer
        self.program_load_cmd(spi, column_address, buf).await?;
        // Write the buffer to the page
        self.program_execute_cmd(spi, page_address).await?;
        // Wait for the write to complete
        while self.is_busy(spi).await? {}
        // Check if the write failed
        if self.program_failed(spi).await? {
            return Err(SpiFlashError::ProgramFailed);
        }
        Ok(())
    }
}

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
