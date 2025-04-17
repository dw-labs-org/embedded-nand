#![no_std]
// Must be first to share macros across crate
pub(crate) mod fmt;

pub mod cmd_async;
pub mod cmd_blocking;
mod device;
pub mod error;

pub use device::SpiNandDevice;

/// Core trait that a NAND flash device must implement.
///
/// Enables use of the [crate::cmd_blocking::SpiNandBlocking] and
/// [crate::cmd_async::SpiNandAsync] traits.
///
/// At minimum requires [SpiNand::PAGE_SIZE] generic, [SpiNand::PAGES_PER_BLOCK] and
/// [SpiNand::BLOCK_COUNT] constants to define the size and layout of the device.
///
/// Default command implementations in [crate::blocking::SpiNandBlocking] and
/// [crate::async_trait::SpiNandAsync] can be overriden by changing the COMMAND constants.
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
    /// Command to enter deep power down
    const DEEP_POWER_DOWN_COMMAND: u8 = 0xB9;
    /// Command to exit deep power down
    const DEEP_POWER_DOWN_EXIT_COMMAND: u8 = 0xAB;
}

/// Possible ECC status values after performing a read operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ECCStatus {
    /// No errors detected or corrected
    Ok,
    /// Errors detected and corrected
    Corrected,
    /// Errors detected and corrected, below threshold for failure
    Failing,
    /// Errors detcted but not corrected
    Failed,
}

/// The JEDEC manufacturer ID of a flash device
/// See https://www.jedec.org/standards-documents/docs/jep-106ab for a list of JEDEC IDs
///
/// Bank refers to which byte the ID is located in but isnt read by command?
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct JedecID {
    /// First non 0x7F byte read from Jedec command
    id: u8,
    /// The device id. MSB first on wire
    device: u16,
}

impl JedecID {
    pub fn new(id: u8, device: u16) -> Self {
        JedecID { id, device }
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for JedecID {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "JedecID(id: {:02X}, device: {:04X})",
            self.id,
            self.device
        );
    }
}
