#![no_std]

pub mod address;
pub mod async_trait;
pub mod blocking;
pub mod device;

pub trait SpiNand<const N: usize> {
    // Device layout
    /// The size of a page in bytes
    const PAGE_SIZE: u32 = N as u32;
    /// The number of pages in a block
    const PAGES_PER_BLOCK: u32;
    /// The number of blocks in the device
    const BLOCK_COUNT: u32;
    /// The size of a block in bytes
    const BLOCK_SIZE: u32 = Self::PAGE_SIZE * Self::PAGES_PER_BLOCK;
    /// The total capacity of the device in bytes
    const CAPACITY: u32 = Self::PAGE_SIZE * Self::PAGES_PER_BLOCK * Self::BLOCK_COUNT;
    /// Minimum number of bytes the storage peripheral can read
    const READ_SIZE: u32 = 1;

    // Commands
    /// The command to reset the flash device
    const RESET_COMMAND: u8 = 0xFF;
    /// The command to read the JEDEC ID of the flash device
    const JEDEC_COMMAND: u8 = 0x9F;
    /// Command to read the status register
    const STATUS_REGISTER_READ_COMMAND: u8 = 0x0F;
    /// Command to read a page into the device buffer/register
    const PAGE_READ_COMMAND: u8 = 0x13;
    /// Command to read a page from the device buffer/register
    const PAGE_READ_BUFFER_COMMAND: u8 = 0x03;
    /// Enable writing to the flash device, including erasing
    const WRITE_ENABLE_COMMAND: u8 = 0x06;
    /// Disable writing to the flash device
    const WRITE_DISABLE_COMMAND: u8 = 0x04;
    /// Command to erase a block of flash memory
    const BLOCK_ERASE_COMMAND: u8 = 0xD8;
    /// Command to write the status register
    const STATUS_REGISTER_WRITE_COMMAND: u8 = 0x1F;
    /// Command to write bytes to the device buffer/register, resetting current values (0xFF)
    const PROGRAM_LOAD_COMMAND: u8 = 0x02;
    /// Command to write bytes to the device buffer/register, without resetting current values
    const PROGRAM_RANDOM_LOAD_COMMAND: u8 = 0x84;
    /// Command to program the device buffer/register to a page
    const PROGRAM_EXECUTE_COMMAND: u8 = 0x10;
}

pub enum ECCStatus {
    Ok,
    Corrected,
    Failing,
    Failed,
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
