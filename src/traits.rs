pub trait NandFlashError: core::fmt::Debug {
    /// Convert a specific NAND flash error into a generic error kind
    fn kind(&self) -> NandFlashErrorKind;
}

/// A trait that NorFlash implementations can use to share an error type.
pub trait ErrorType {
    /// Errors returned by this NOR flash.
    type Error: NandFlashError;
}

/// NAND flash error kinds.
///
/// NAND flash implementations must map their error to those generic error kinds through the
/// [`NandFlashError`] trait.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum NandFlashErrorKind {
    /// The arguments are not properly aligned.
    NotAligned,

    /// The arguments are out of bounds.
    OutOfBounds,

    /// Block has failed either during erase, write or read checksum.
    /// Contains byte address of failed block, or [None] if specific block unknown
    BlockFail(Option<u64>),

    /// Block is failing but operation was successful i.e ECC corrected read.
    /// Contains byte address of failing block, or [None] if specific block unknown
    BlockFailing(Option<u64>),

    /// Error specific to the implementation.
    Other,
}

/// Read only NAND flash trait.
pub trait ReadNandFlash: ErrorType {
    /// The minumum number of bytes the storage peripheral can read
    const READ_SIZE: usize;

    /// Read a slice of data from the storage peripheral, starting the read
    /// operation at the given address offset, and reading `bytes.len()` bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the arguments are not aligned or out of bounds. The implementation
    /// can use the [`check_read`] helper function.
    ///
    fn read(&mut self, offset: u64, bytes: &mut [u8]) -> Result<(), Self::Error>;

    /// The capacity of the peripheral in bytes.
    fn capacity(&self) -> u64;

    /// Check status of block according to bad block marker and ECC / Checksum status
    fn block_status(&mut self, address: u64) -> Result<BlockStatus, Self::Error>;

    /// Check if the block is marked as bad
    fn block_is_bad(&mut self, address: u64) -> Result<bool, Self::Error> {
        match self.block_status(address)? {
            BlockStatus::Failed => Ok(true),
            _ => Ok(false),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum BlockStatus {
    /// Marked OK and passes ECC / Checksum
    Ok,
    /// Not marked as failed by the manufacturer
    MarkedOk,
    /// Marked as failed or failed ECC / Checksum
    Failed,
}

/// Return whether a read operation is within bounds.
pub fn check_read<T: ReadNandFlash>(
    flash: &T,
    offset: u64,
    length: usize,
) -> Result<(), NandFlashErrorKind> {
    check_slice(flash, T::READ_SIZE, offset, length)
}

/// NAND flash trait.
pub trait NandFlash: ReadNandFlash {
    /// The minumum number of bytes the storage peripheral can write (page size)
    const WRITE_SIZE: usize;

    /// The minumum number of bytes the storage peripheral can erase (block or sector size)
    const ERASE_SIZE: usize;

    /// Erase the given storage range, clearing all data within `[from..to]`.
    /// The given range will contain all 1s afterwards.
    ///
    /// If power is lost during erase, contents of the page are undefined.
    ///
    /// # Errors
    ///
    /// Returns an error if the arguments are not aligned or out of bounds (the case where `to >
    /// from` is considered out of bounds). The implementation can use the [`check_erase`]
    /// helper function.
    fn erase(&mut self, from: u64, to: u64) -> Result<(), Self::Error>;

    /// If power is lost during write, the contents of the written words are undefined,
    /// but the rest of the page is guaranteed to be unchanged.
    /// It is not allowed to write to the same word twice.
    ///
    /// # Errors
    ///
    /// Returns an error if the arguments are not aligned or out of bounds. The implementation
    /// can use the [`check_write`] helper function.
    fn write(&mut self, offset: u64, bytes: &[u8]) -> Result<(), Self::Error>;

    /// Mark the block as bad
    fn mark_bad(&mut self, address: u64) -> Result<(), Self::Error>;
}

/// Return whether an erase operation is aligned and within bounds.
pub fn check_erase<T: NandFlash>(flash: &T, from: u64, to: u64) -> Result<(), NandFlashErrorKind> {
    if from > to || to > flash.capacity() {
        return Err(NandFlashErrorKind::OutOfBounds);
    }
    if from % T::ERASE_SIZE as u64 != 0 || to % T::ERASE_SIZE as u64 != 0 {
        return Err(NandFlashErrorKind::NotAligned);
    }
    Ok(())
}

/// Return whether a write operation is aligned and within bounds.
pub fn check_write<T: NandFlash>(
    flash: &T,
    offset: u64,
    length: usize,
) -> Result<(), NandFlashErrorKind> {
    check_slice(flash, T::WRITE_SIZE, offset, length)
}

fn check_slice<T: ReadNandFlash>(
    flash: &T,
    align: usize,
    offset: u64,
    length: usize,
) -> Result<(), NandFlashErrorKind> {
    if length as u64 > flash.capacity() || offset > (flash.capacity() - (length as u64)) {
        return Err(NandFlashErrorKind::OutOfBounds);
    }
    if offset % align as u64 != 0 || length % align != 0 {
        return Err(NandFlashErrorKind::NotAligned);
    }
    Ok(())
}
