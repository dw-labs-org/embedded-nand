use crate::NandFlash;
use core::ops::{Add, AddAssign};

/// Index of a page in the flash device
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PageIndex(pub(crate) u32);

impl PageIndex {
    pub fn new(index: u32) -> Self {
        PageIndex(index)
    }
    pub fn as_u32(&self) -> u32 {
        self.0
    }

    pub fn from_byte_address(ba: ByteAddress, page_size: u32) -> Self {
        PageIndex(ba.0 / page_size)
    }

    /// Convert from a [BlockIndex]
    pub fn from_block_address(ba: BlockIndex, pages_per_block: u32) -> Self {
        PageIndex(ba.0 as u32 * pages_per_block)
    }
}

impl From<PageIndex> for u32 {
    fn from(pa: PageIndex) -> Self {
        pa.as_u32()
    }
}

/// Index of a block in the flash device
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockIndex(pub(crate) u16);

impl BlockIndex {
    pub fn new(index: u16) -> Self {
        BlockIndex(index)
    }

    pub fn as_u16(&self) -> u16 {
        self.0
    }

    pub fn as_page_index(&self, pages_per_block: u32) -> PageIndex {
        PageIndex((self.0 as u32) * pages_per_block)
    }

    pub fn as_byte_address(&self, block_size: u32) -> ByteAddress {
        ByteAddress((self.0 as u32) * block_size)
    }

    pub fn from_page_address(pa: PageIndex, pages_per_block: u32) -> Self {
        BlockIndex((pa.0 / pages_per_block) as u16)
    }

    pub fn from_byte_address(ba: ByteAddress, block_size: u32) -> Self {
        BlockIndex((ba.0 / (block_size)) as u16)
    }

    pub fn from_raw_byte_offset(offset: u32, block_size: u32) -> Self {
        BlockIndex((offset / block_size) as u16)
    }
}

impl From<BlockIndex> for u16 {
    fn from(bi: BlockIndex) -> Self {
        bi.as_u16()
    }
}

impl Add<u16> for BlockIndex {
    type Output = Self;

    fn add(self, rhs: u16) -> Self::Output {
        BlockIndex(self.0 + rhs)
    }
}

impl AddAssign<u16> for BlockIndex {
    fn add_assign(&mut self, rhs: u16) {
        self.0 += rhs;
    }
}

/// Address of a byte in the flash device
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ByteAddress(pub(crate) u32);

impl ByteAddress {
    pub fn new(address: u32) -> Self {
        ByteAddress(address)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }

    pub fn as_block_index(&self, block_size: u32) -> BlockIndex {
        BlockIndex((self.0 / block_size) as u16)
    }

    /// Number of bytes into the block
    pub fn block_offset(&self, block_size: u32) -> u32 {
        self.0 % block_size
    }

    pub fn as_page_index(&self, page_size: u32) -> PageIndex {
        PageIndex(self.0 / page_size)
    }

    pub fn as_column_address(&self, page_size: u32) -> ColumnAddress {
        ColumnAddress((self.0 % page_size) as u16)
    }
}

impl From<ByteAddress> for u32 {
    fn from(ba: ByteAddress) -> Self {
        ba.as_u32()
    }
}

impl Add<u32> for ByteAddress {
    type Output = Self;

    fn add(self, rhs: u32) -> Self::Output {
        ByteAddress(self.0 + rhs)
    }
}

impl AddAssign<u32> for ByteAddress {
    fn add_assign(&mut self, rhs: u32) {
        self.0 += rhs;
    }
}

/// Address of a byte within a page in the flash device
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColumnAddress(pub(crate) u16);

impl ColumnAddress {
    pub fn new(address: u16) -> Self {
        ColumnAddress(address)
    }

    pub fn as_u16(&self) -> u16 {
        self.0
    }

    pub fn from_byte_address(ba: ByteAddress, page_size: u32) -> Self {
        ColumnAddress((ba.0 % page_size) as u16)
    }
}

/// Convert a [PageIndex] to a [ByteAddress]
pub fn page_to_byte_address<F: NandFlash>(page: PageIndex) -> ByteAddress {
    ByteAddress(page.0 * F::PAGE_SIZE as u32)
}

/// Convert a [PageIndex] to a [BlockIndex]
pub fn page_to_block_address<F: NandFlash>(page: PageIndex) -> BlockIndex {
    BlockIndex((page.0 / F::PAGES_PER_BLOCK as u32) as u16)
}

/// Convert a [BlockIndex] to a [PageIndex]
pub fn block_to_page_address<F: NandFlash>(block: BlockIndex) -> PageIndex {
    PageIndex(block.0 as u32 * F::PAGES_PER_BLOCK as u32)
}
