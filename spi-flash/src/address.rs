/// Address of a page in the flash device
#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct PageAddress(pub u32);

impl PageAddress {
    pub fn from_byte_address(ba: ByteAddress, page_size: u32) -> Self {
        PageAddress(ba.0 / page_size)
    }

    /// Convert from a [BlockAddress]
    pub fn from_block_address(ba: BlockAddress, pages_per_block: u32) -> Self {
        PageAddress(ba.0 * pages_per_block)
    }
}

/// Address of a block in the flash device
#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct BlockAddress(pub u32);

/// Address of a byte in the flash device
#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct ByteAddress(pub u32);

impl ByteAddress {
    pub fn as_page_address(&self, page_size: u32) -> PageAddress {
        PageAddress(self.0 / page_size)
    }

    pub fn as_column_address(&self, page_size: u32) -> ColumnAddress {
        ColumnAddress((self.0 % page_size) as u16)
    }

    pub fn as_block_address(&self, block_size: u32) -> BlockAddress {
        BlockAddress(self.0 / (block_size))
    }
}

/// Address of a byte within a page in the flash device
#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct ColumnAddress(pub u16);

impl From<u16> for ColumnAddress {
    fn from(ca: u16) -> Self {
        ColumnAddress(ca)
    }
}

impl From<u32> for PageAddress {
    fn from(pa: u32) -> Self {
        PageAddress(pa)
    }
}

impl From<u32> for BlockAddress {
    fn from(ba: u32) -> Self {
        BlockAddress(ba)
    }
}

impl From<u32> for ByteAddress {
    fn from(ba: u32) -> Self {
        ByteAddress(ba)
    }
}
