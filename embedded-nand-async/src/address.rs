use embedded_nand::{BlockIndex, ByteAddress, ColumnAddress, PageIndex};

use crate::NandFlash;

/// Trait for converting between page and block indices and byte and column addresses
pub trait AddressConversions {
    fn page_to_byte_address(page: PageIndex) -> ByteAddress;
    fn page_to_block_index(page: PageIndex) -> BlockIndex;
    fn page_in_block(page: PageIndex) -> u32;
    fn page_range_from_length(length: u32) -> u32;
    fn block_to_page_index(block: BlockIndex) -> PageIndex;
    fn block_to_byte_address(block: BlockIndex) -> ByteAddress;
    fn byte_to_page_index(byte: ByteAddress) -> PageIndex;
    fn byte_to_block_index(byte: ByteAddress) -> BlockIndex;
    fn byte_to_column_address(byte: ByteAddress) -> ColumnAddress;
    fn byte_in_block(byte: ByteAddress) -> u32;
    fn raw_byte_to_block_index(offset: u32) -> BlockIndex;
    fn is_block_aligned(byte: ByteAddress) -> bool;
    fn is_page_aligned(byte: ByteAddress) -> bool;
}

impl<T: NandFlash> AddressConversions for T {
    fn page_to_byte_address(page: PageIndex) -> ByteAddress {
        page.as_byte_address(Self::PAGE_SIZE as u32)
    }
    fn page_to_block_index(page: PageIndex) -> BlockIndex {
        page.as_block_index(Self::PAGES_PER_BLOCK as u32)
    }
    fn page_in_block(page: PageIndex) -> u32 {
        page.as_u32() % Self::PAGES_PER_BLOCK as u32
    }
    fn page_range_from_length(length: u32) -> u32 {
        length / Self::PAGE_SIZE as u32
    }
    fn block_to_page_index(block: BlockIndex) -> PageIndex {
        block.as_page_index(Self::PAGES_PER_BLOCK as u32)
    }
    fn block_to_byte_address(block: BlockIndex) -> ByteAddress {
        block.as_byte_address(Self::ERASE_SIZE as u32)
    }
    fn byte_to_page_index(byte: ByteAddress) -> PageIndex {
        byte.as_page_index(Self::PAGE_SIZE as u32)
    }
    fn byte_to_block_index(byte: ByteAddress) -> BlockIndex {
        byte.as_block_index(Self::ERASE_SIZE as u32)
    }
    fn byte_to_column_address(byte: ByteAddress) -> ColumnAddress {
        byte.as_column_address(Self::PAGE_SIZE as u32)
    }
    fn byte_in_block(byte: ByteAddress) -> u32 {
        byte.block_offset(Self::ERASE_SIZE as u32)
    }
    fn raw_byte_to_block_index(offset: u32) -> BlockIndex {
        BlockIndex::from_raw_byte_offset(offset, Self::ERASE_SIZE as u32)
    }
    fn is_block_aligned(byte: ByteAddress) -> bool {
        byte.as_u32() % Self::ERASE_SIZE as u32 == 0
    }
    fn is_page_aligned(byte: ByteAddress) -> bool {
        byte.as_u32() % Self::PAGE_SIZE as u32 == 0
    }
}
