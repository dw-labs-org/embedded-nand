use embedded_nand::{ByteAddress, PageIndex};

use crate::{BlockIndex, NandFlash};

/// Iterate over block indices and byte addresses of blocks in nand flash
pub struct BlockIter {
    block_size: u32,
    current: BlockIndex,
    end: BlockIndex,
}

impl BlockIter {
    /// Create a new iterator over blocks
    pub fn new(start: BlockIndex, end: BlockIndex, block_size: u32) -> Self {
        BlockIter {
            block_size,
            current: start,
            end,
        }
    }
}

impl Iterator for BlockIter {
    type Item = (BlockIndex, ByteAddress);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.end {
            let block = self.current;
            let byte_address = block.as_byte_address(self.block_size);
            self.current.inc();
            Some((block, byte_address))
        } else {
            None
        }
    }
}

/// Iterate over page indices and byte addresses of pages in nand flash
pub struct PageIter {
    page_size: u32,
    current: PageIndex,
    end: PageIndex,
}

impl PageIter {
    /// Create a new iterator over pages
    pub fn new(start: PageIndex, end: PageIndex, page_size: u32) -> Self {
        PageIter {
            page_size,
            current: start,
            end,
        }
    }
}

impl Iterator for PageIter {
    type Item = (PageIndex, ByteAddress);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.end {
            let page = self.current;
            let byte_address = page.as_byte_address(self.page_size);
            self.current.inc();
            Some((page, byte_address))
        } else {
            None
        }
    }
}

pub trait NandFlashIter {
    /// Iterate over a range of block indices
    fn block_iter_range(&self, start: BlockIndex, end: BlockIndex) -> BlockIter;
    /// Iterate over a range of page indices
    fn page_iter_range(&self, start: PageIndex, end: PageIndex) -> PageIter;
    /// Iterate over all blocks
    fn block_iter(&self) -> BlockIter;
    /// Iterate over all pages
    fn page_iter(&self) -> PageIter;
    /// Iterate over blocks starting from a specific block
    fn block_iter_from(&self, start: BlockIndex) -> BlockIter;
    /// Iterate over pages starting from a specific page
    fn page_iter_from(&self, start: PageIndex) -> PageIter;
}

impl<T: NandFlash> NandFlashIter for T {
    fn block_iter_range(&self, start: BlockIndex, end: BlockIndex) -> BlockIter {
        let block_size = Self::ERASE_SIZE as u32;
        BlockIter::new(start, end, block_size)
    }

    fn page_iter_range(&self, start: PageIndex, end: PageIndex) -> PageIter {
        let page_size = Self::PAGE_SIZE as u32;
        PageIter::new(start, end, page_size)
    }

    fn block_iter(&self) -> BlockIter {
        self.block_iter_range(
            BlockIndex::new(0),
            BlockIndex::new(Self::BLOCK_COUNT as u16),
        )
    }

    fn page_iter(&self) -> PageIter {
        self.page_iter_range(
            PageIndex::new(0),
            PageIndex::new((Self::PAGES_PER_BLOCK * Self::BLOCK_COUNT) as u32),
        )
    }

    fn block_iter_from(&self, start: BlockIndex) -> BlockIter {
        self.block_iter_range(start, BlockIndex::new(Self::BLOCK_COUNT as u16))
    }
    fn page_iter_from(&self, start: PageIndex) -> PageIter {
        self.page_iter_range(
            start,
            PageIndex::new((Self::PAGES_PER_BLOCK * Self::BLOCK_COUNT) as u32),
        )
    }
}
