#![no_std]
#![allow(async_fn_in_trait)]

use embedded_nand::{BlockIndex, BlockStatus};

mod address;
mod fmt;
pub mod iter;

pub use address::AddressConversions;

pub trait NandFlashError: core::fmt::Debug {
    /// Convert a specific NAND flash error into a generic error kind
    fn kind(&self) -> NandFlashErrorKind;
}

/// A trait that NandFlash implementations can use to share an error type.
pub trait ErrorType {
    /// Errors returned by this NAND flash.
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
    BlockFail(Option<u32>),

    /// Block is failing but operation was successful i.e ECC corrected read.
    /// Contains byte address of failing block, or [None] if specific block unknown
    BlockFailing(Option<u32>),

    /// Error specific to the implementation.
    Other,
}

/// NAND flash trait.
pub trait NandFlash: ErrorType {
    /// The minumum number of bytes the storage peripheral can read
    const READ_SIZE: usize;

    /// Size of a page in bytes
    const PAGE_SIZE: usize;

    /// Number of pages in a block
    const PAGES_PER_BLOCK: usize;

    /// Number of blocks
    const BLOCK_COUNT: usize;

    /// The minumum number of bytes the storage peripheral can erase (block or sector size)
    const ERASE_SIZE: usize;

    /// The minumum number of bytes the storage peripheral can write
    const WRITE_SIZE: usize;

    /// Read a slice of data from the storage peripheral, starting the read
    /// operation at the given address offset, and reading `bytes.len()` bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the arguments are not aligned or out of bounds. The implementation
    /// can use the [`check_read`] helper function.
    ///
    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error>;

    /// The capacity of the peripheral in bytes.
    fn capacity(&self) -> u32;

    /// Mark the block as bad
    async fn mark_block_bad(&mut self, block: BlockIndex) -> Result<(), Self::Error>;

    /// Check status of block according to bad block marker and ECC / Checksum status
    async fn block_status(&mut self, block: BlockIndex) -> Result<BlockStatus, Self::Error>;

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
    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error>;

    /// Erase a block by block index.
    async fn erase_block(&mut self, block: BlockIndex) -> Result<(), Self::Error>;

    /// If power is lost during write, the contents of the written words are undefined,
    /// but the rest of the page is guaranteed to be unchanged.
    /// It is not allowed to write to the same word twice.
    ///
    /// # Errors
    ///
    /// Returns an error if the arguments are not aligned or out of bounds. The implementation
    /// can use the [`check_write`] helper function.
    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error>;

    /// Copy data from one location to another.
    ///
    /// Some devices support internal copy commands, which are faster than
    /// reading and writing the data. This function should be used to
    /// implement the copy command.
    async fn copy(
        &mut self,
        src_offset: u32,
        dest_offset: u32,
        length: u32,
    ) -> Result<(), Self::Error>;
}

/// Return whether a read operation is within bounds.
pub fn check_read<T: NandFlash>(
    flash: &T,
    offset: u32,
    length: usize,
) -> Result<(), NandFlashErrorKind> {
    check_slice(flash, T::READ_SIZE, offset, length)
}

/// Return whether an erase operation is aligned and within bounds.
pub fn check_erase<T: NandFlash>(flash: &T, from: u32, to: u32) -> Result<(), NandFlashErrorKind> {
    if from > to || to > flash.capacity() {
        return Err(NandFlashErrorKind::OutOfBounds);
    }
    if from % T::ERASE_SIZE as u32 != 0 || to % T::ERASE_SIZE as u32 != 0 {
        return Err(NandFlashErrorKind::NotAligned);
    }
    Ok(())
}

/// Return whether a write operation is aligned and within bounds.
pub fn check_write<T: NandFlash>(
    flash: &T,
    offset: u32,
    length: usize,
) -> Result<(), NandFlashErrorKind> {
    check_slice(flash, T::WRITE_SIZE, offset, length)
}

pub fn check_slice<T: NandFlash>(
    flash: &T,
    align: usize,
    offset: u32,
    length: usize,
) -> Result<(), NandFlashErrorKind> {
    if length as u32 > flash.capacity() || offset > (flash.capacity() - (length as u32)) {
        return Err(NandFlashErrorKind::OutOfBounds);
    }
    if offset % align as u32 != 0 || length % align != 0 {
        return Err(NandFlashErrorKind::NotAligned);
    }
    Ok(())
}
