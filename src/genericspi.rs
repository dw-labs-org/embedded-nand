use embedded_hal::spi::{self, SpiDevice};

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

impl<SPI: SpiDevice, D: FlashRead<SPI>> SpiFlash<SPI, D> {
    /// Get the Jedec ID of the flash device
    pub fn jedec(&mut self) -> Result<JedecID, SpiFlashError<SPI>> {
        self.device.read_jedec_id(&mut self.spi)
    }
    /// Reset the flash device
    pub fn reset(&mut self) -> Result<(), SpiFlashError<SPI>> {
        self.device.reset(&mut self.spi)
    }
    /// Read the status register
    pub fn read_status_register(&mut self) -> Result<u8, SpiFlashError<SPI>> {
        self.device.read_status_register(&mut self.spi)
    }

    /// Check if the device is busy
    pub fn is_busy(&mut self) -> Result<bool, SpiFlashError<SPI>> {
        self.device.is_busy(&mut self.spi)
    }
}

impl<SPI: SpiDevice, D: FlashWrite<SPI>> SpiFlash<SPI, D> {
    /// Enable writing to the flash device
    pub fn write_enable(&mut self) -> Result<(), SpiFlashError<SPI>> {
        self.device.write_enable(&mut self.spi)
    }
    /// Disable writing to the flash device
    pub fn write_disable(&mut self) -> Result<(), SpiFlashError<SPI>> {
        self.device.write_disable(&mut self.spi)
    }
    /// Erase a block of flash memory
    pub fn erase_block(&mut self, address: u32) -> Result<(), SpiFlashError<SPI>> {
        self.device.erase_block(&mut self.spi, address)
    }
    /// Erase the whole flash device
    pub fn erase_chip(&mut self) -> Result<(), SpiFlashError<SPI>> {
        self.device.erase_chip(&mut self.spi)
    }

    /// Check if writing to the device is enabled
    pub fn is_write_enabled(&mut self) -> Result<bool, SpiFlashError<SPI>> {
        self.device.is_write_enabled(&mut self.spi)
    }
}

#[derive(Debug, defmt::Format)]
pub enum SpiFlashError<SPI: SpiDevice> {
    SPI(SPI::Error),
}

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
    operations: &mut [spi::Operation<'_, u8>],
) -> Result<(), SpiFlashError<SPI>> {
    spi.transaction(operations)
        .map_err(|e| SpiFlashError::SPI(e))
}

pub trait FlashRead<SPI: SpiDevice> {
    // Device layout
    /// The size of a page in bytes
    const PAGE_SIZE: u32;
    /// The number of pages in a block
    const PAGES_PER_BLOCK: u32;
    /// The number of blocks in the device
    const BLOCK_COUNT: u32;
    /// The size of a block in bytes
    const BLOCK_SIZE: u32 = Self::PAGE_SIZE * Self::PAGES_PER_BLOCK;
    /// The total capacity of the device in bytes
    const CAPACITY: u32 = Self::PAGE_SIZE * Self::PAGES_PER_BLOCK * Self::BLOCK_COUNT;
    /// Command to read the status register
    const STATUS_REGISTER_READ_COMMAND: u8 = 0x05;

    // Commands
    /// The command to reset the flash device
    const RESET_COMMAND: u8 = 0xFF;
    /// The command to read the JEDEC ID of the flash device
    const JEDEC_COMMAND: u8 = 0x9F;

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

    /// Read the status register
    fn read_status_register(&self, spi: &mut SPI) -> Result<u8, SpiFlashError<SPI>> {
        let mut buf = [Self::STATUS_REGISTER_READ_COMMAND, 0];
        spi_transfer_in_place(spi, &mut buf)?;
        Ok(buf[1])
    }

    /// Check if busy flag is set
    fn is_busy(&self, spi: &mut SPI) -> Result<bool, SpiFlashError<SPI>> {
        let status = self.read_status_register(spi)?;
        Ok((status & 0x01) != 0)
    }
}

pub trait FlashWrite<SPI: SpiDevice>: FlashRead<SPI> {
    /// Enable writing to the flash device, including erasing
    const WRITE_ENABLE_COMMAND: u8 = 0x06;
    /// Disable writing to the flash device
    const WRITE_DISABLE_COMMAND: u8 = 0x04;
    /// Command to erase a block of flash memory
    const BLOCK_ERASE_COMMAND: u8 = 0xD8;
    /// Command to erase thw whole flash device
    const CHIP_ERASE_COMMAND: u8 = 0xC7;
    /// Command to write the status register
    const STATUS_REGISTER_WRITE_COMMAND: u8 = 0x01;

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
        let status = self.read_status_register(spi)?;
        Ok((status & 0x02) != 0)
    }

    /// Erase a block of flash memory
    fn erase_block(&self, spi: &mut SPI, address: u32) -> Result<(), SpiFlashError<SPI>> {
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

    /// Erase the whole flash device
    fn erase_chip(&self, spi: &mut SPI) -> Result<(), SpiFlashError<SPI>> {
        // Enable writing first
        self.write_enable(spi)?;
        spi_write(spi, &[Self::CHIP_ERASE_COMMAND])
    }
}

/// The JEDEC manufacturer ID of a flash device
/// See https://www.jedec.org/standards-documents/docs/jep-106ab for a list of JEDEC IDs
#[derive(Debug, Clone, Copy)]
pub struct JedecID {
    /// First non 0x7F byte read from Jedec command
    id: u8,
    /// Bank refers to which byte the ID is located in
    /// 1 = first byte, 2 = second byte etc, up to 16 as of 01/2025
    bank: u8,
}

impl JedecID {
    pub fn new(id: u8, bank: u8) -> Self {
        JedecID { id, bank }
    }
}

impl defmt::Format for JedecID {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "JedecID(id: {:02X}, bank: {})", self.id, self.bank);
    }
}
