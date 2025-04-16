/// Iterate over byte addresses of blocks in nand flash
pub struct BlockIter {
    pub(crate) block_size: u32,
    pub(crate) count: u16,
    pub(crate) block_count: u16,
}

impl Iterator for BlockIter {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count < self.block_count {
            let block = self.count as u32 * self.block_size;
            self.count += 1;
            Some(block)
        } else {
            None
        }
    }
}

/// Iterate over page addresses of blocks in nand flash
pub struct PageIter {
    pub(crate) page_size: u32,
    pub(crate) count: u32,
    pub(crate) page_count: u32,
}

impl Iterator for PageIter {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count < self.page_count {
            let page = self.count as u32 * self.page_size;
            self.count += 1;
            Some(page)
        } else {
            None
        }
    }
}
