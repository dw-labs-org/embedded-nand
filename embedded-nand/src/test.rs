use crate::AddressConversions;
use crate::BlockIndex;
use crate::ByteAddress;

/// A virtual NAND flash implementation that can be used for testing purposes.
#[derive(Debug, Clone)]
pub struct VirtualNandFlash<
    const PAGE_SIZE: usize,
    const PAGES_PER_BLOCK: usize,
    const BLOCK_COUNT: usize,
> {
    storage: [[[u8; PAGE_SIZE]; PAGES_PER_BLOCK]; BLOCK_COUNT],
    block_status: [crate::BlockStatus; BLOCK_COUNT],
    ecc_failing: [bool; BLOCK_COUNT],
    erase_count: [u32; BLOCK_COUNT],
    read_count: [[u32; PAGES_PER_BLOCK]; BLOCK_COUNT],
    write_count: [[u32; PAGES_PER_BLOCK]; BLOCK_COUNT],
}

impl<const PAGE_SIZE: usize, const PAGES_PER_BLOCK: usize, const BLOCK_COUNT: usize>
    VirtualNandFlash<PAGE_SIZE, PAGES_PER_BLOCK, BLOCK_COUNT>
{
    /// Creates a new instance of the virtual NAND flash.
    pub fn new() -> Self {
        Self {
            storage: [[[0xFF; PAGE_SIZE]; PAGES_PER_BLOCK]; BLOCK_COUNT],
            block_status: [crate::BlockStatus::Ok; BLOCK_COUNT],
            ecc_failing: [false; BLOCK_COUNT],
            erase_count: [0; BLOCK_COUNT],
            read_count: [[0; PAGES_PER_BLOCK]; BLOCK_COUNT],
            write_count: [[0; PAGES_PER_BLOCK]; BLOCK_COUNT],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Misc
    Misc,
    /// block is failing
    BlockFailing,
    /// block is failed
    BlockFail,
    /// Out of bounds
    OutOfBounds,
    /// Not aligned
    NotAligned,
}

impl crate::NandFlashError for Error {
    fn kind(&self) -> crate::NandFlashErrorKind {
        match self {
            Error::Misc => crate::NandFlashErrorKind::Other,
            Error::BlockFailing => crate::NandFlashErrorKind::BlockFailing(None),
            Error::BlockFail => crate::NandFlashErrorKind::BlockFail(None),
            Error::OutOfBounds => crate::NandFlashErrorKind::OutOfBounds,
            Error::NotAligned => crate::NandFlashErrorKind::NotAligned,
        }
    }
}

impl<const PAGE_SIZE: usize, const PAGES_PER_BLOCK: usize, const BLOCK_COUNT: usize>
    crate::ErrorType for VirtualNandFlash<PAGE_SIZE, PAGES_PER_BLOCK, BLOCK_COUNT>
{
    type Error = Error;
}

impl<const PAGE_SIZE: usize, const PAGES_PER_BLOCK: usize, const BLOCK_COUNT: usize>
    crate::NandFlash for VirtualNandFlash<PAGE_SIZE, PAGES_PER_BLOCK, BLOCK_COUNT>
{
    const READ_SIZE: usize = 1;

    const PAGE_SIZE: usize = PAGE_SIZE as usize;

    const PAGES_PER_BLOCK: usize = PAGES_PER_BLOCK as usize;

    const BLOCK_COUNT: usize = BLOCK_COUNT as usize;

    const ERASE_SIZE: usize = Self::PAGE_SIZE as usize * Self::PAGES_PER_BLOCK as usize;

    const WRITE_SIZE: usize = 1;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let first_block = Self::byte_to_block_index(ByteAddress::new(offset));
        let last_block =
            Self::byte_to_block_index(ByteAddress::new(offset + bytes.len() as u32 - 1));
        for block in first_block.as_u16()..=last_block.as_u16() {
            if self.block_status[block as usize] == crate::BlockStatus::Failed {
                return Err(Error::BlockFail);
            } else if self.ecc_failing[block as usize] {
                return Err(Error::BlockFailing);
            }
        }
        trace!("Reading from blocks {} to {}", first_block.0, last_block.0);
        let mut start = unsafe { (self.storage.as_ptr() as *const u8).add(offset as usize) };
        bytes.copy_from_slice(unsafe { core::slice::from_raw_parts(start, bytes.len()) });
        Ok(())
    }

    fn capacity(&self) -> u32 {
        Self::PAGE_SIZE as u32 * Self::PAGES_PER_BLOCK as u32 * Self::BLOCK_COUNT as u32
    }

    fn block_status(
        &mut self,
        block: crate::BlockIndex,
    ) -> Result<crate::BlockStatus, Self::Error> {
        if block.0 >= Self::BLOCK_COUNT as u16 {
            return Err(Error::OutOfBounds);
        }
        Ok(self.block_status[block.0 as usize])
    }

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        if ByteAddress::new(from).block_offset(Self::ERASE_SIZE as u32) != 0 {
            return Err(Error::NotAligned);
        }
        if ByteAddress::new(to).block_offset(Self::ERASE_SIZE as u32) != 0 {
            return Err(Error::NotAligned);
        }
        let first_block = Self::byte_to_block_index(ByteAddress::new(from));
        let last_block = Self::byte_to_block_index(ByteAddress::new(to));
        trace!(
            "Erasing blocks {} to {}",
            first_block.as_u16(),
            last_block.as_u16() - 1
        );
        for block in first_block.as_u16()..last_block.as_u16() {
            if self.block_status[block as usize] == crate::BlockStatus::Failed {
                return Err(Error::BlockFail);
            }
            self.storage[block as usize]
                .iter_mut()
                .for_each(|page| page.fill(0xFF));
        }
        Ok(())
    }

    fn erase_block(&mut self, block: crate::BlockIndex) -> Result<(), Self::Error> {
        if block.0 >= Self::BLOCK_COUNT as u16 {
            return Err(Error::OutOfBounds);
        }
        if self.block_status[block.0 as usize] == crate::BlockStatus::Failed {
            Err(Error::BlockFail)
        } else {
            self.storage[block.0 as usize]
                .iter_mut()
                .for_each(|page| page.fill(0xFF));
            Ok(())
        }
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        // check for block status
        let first_block = Self::byte_to_block_index(ByteAddress::new(offset));
        let last_block =
            Self::byte_to_block_index(ByteAddress::new(offset + bytes.len() as u32 - 1));
        for block in first_block.as_u16()..=last_block.as_u16() {
            if self.block_status[block as usize] == crate::BlockStatus::Failed {
                return Err(Error::BlockFail);
            }
        }
        trace!("Writing to blocks {} to {}", first_block.0, last_block.0);
        let mut start = unsafe { (self.storage.as_ptr() as *mut u8).add(offset as usize) };
        let mut slice = unsafe { core::slice::from_raw_parts_mut(start, bytes.len()) };
        for (a, b) in slice.iter_mut().zip(bytes.iter()) {
            *a &= *b;
        }
        Ok(())
    }

    fn copy(&mut self, src_offset: u32, dest_offset: u32, length: u32) -> Result<(), Self::Error> {
        let src_slice = unsafe {
            core::slice::from_raw_parts(
                (self.storage.as_ptr() as *const u8).add(src_offset as usize),
                length as usize,
            )
        };
        let dest_slice = unsafe {
            core::slice::from_raw_parts_mut(
                (self.storage.as_ptr() as *mut u8).add(dest_offset as usize),
                length as usize,
            )
        };
        dest_slice.copy_from_slice(src_slice);
        Ok(())
    }

    fn mark_block_bad(&mut self, block: crate::BlockIndex) -> Result<(), Self::Error> {
        if block.0 >= Self::BLOCK_COUNT as u16 {
            return Err(Error::OutOfBounds);
        }
        self.block_status[block.0 as usize] = crate::BlockStatus::Failed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NandFlash;

    // Adds logging to the test automatically
    // control with RUST_LOG="LEVEL"
    // requires --features log passed to cargo test
    use test_log::test;

    const PAGE_SIZE: usize = 128;
    const PAGES_PER_BLOCK: usize = 8;
    const BLOCK_COUNT: usize = 256;
    const CAPACITY: usize = PAGE_SIZE * PAGES_PER_BLOCK * BLOCK_COUNT;

    /// Test read, write and erase of entire flash
    #[test]
    fn test_all_rwe() {
        let mut flash = VirtualNandFlash::<PAGE_SIZE, PAGES_PER_BLOCK, BLOCK_COUNT>::new();
        let buffer = [0; CAPACITY];
        flash.write(0, &buffer).unwrap();
        let mut rbuffer = [1; CAPACITY];
        flash.read(0, &mut rbuffer).unwrap();
        assert_eq!(buffer, rbuffer);
        flash.erase(0, CAPACITY as u32).unwrap();
        flash.read(0, &mut rbuffer).unwrap();
        assert_eq!(rbuffer, [0xFF; CAPACITY]);
    }

    /// Test read, write and erase of a single page
    #[test]
    fn test_page_rwe() {
        let mut flash = VirtualNandFlash::<PAGE_SIZE, PAGES_PER_BLOCK, BLOCK_COUNT>::new();
        for page in 0..BLOCK_COUNT * PAGES_PER_BLOCK {
            let offset = page * PAGE_SIZE;
            let buffer = [page as u8; PAGE_SIZE];
            flash.write(offset as u32, &buffer).unwrap();
            assert_eq!(
                flash.storage[page / PAGES_PER_BLOCK][page % PAGES_PER_BLOCK],
                buffer
            );
            let mut rbuffer = [0; PAGE_SIZE];
            flash.read(offset as u32, &mut rbuffer).unwrap();
            assert_eq!(buffer, rbuffer);
            let block = PAGES_PER_BLOCK * PAGE_SIZE * (page / PAGES_PER_BLOCK);
            debug!("Erasing block ast {}", block);
            flash
                .erase(block as u32, (block + PAGES_PER_BLOCK * PAGE_SIZE) as u32)
                .unwrap();

            assert!(flash
                .storage
                .iter()
                .all(|b| b.iter().all(|p| p.iter().all(|&x| x == 0xFF))));
        }
    }

    /// Test reading and writing over block boundaries
    #[test]
    fn test_block_boundary_rwe() {
        let mut flash = VirtualNandFlash::<PAGE_SIZE, PAGES_PER_BLOCK, BLOCK_COUNT>::new();
        let offset = PAGE_SIZE * 15 + PAGE_SIZE / 2;
        const length: usize = PAGE_SIZE * 2;
        let block = offset / (PAGE_SIZE * PAGES_PER_BLOCK);
        let page_in_block = (offset / PAGE_SIZE) - (block * PAGES_PER_BLOCK);
        let byte_in_page = offset % PAGE_SIZE;
        info!(
            "Writing at offset {}, block {}, page in block {}, byte in page {}",
            offset, block, page_in_block, byte_in_page
        );
        let buffer = [0; length];
        flash.write(offset as u32, &buffer).unwrap();
        assert!(
            flash.storage[block as usize][page_in_block as usize][byte_in_page..]
                .iter()
                .all(|&x| x == 0),
        );
        assert!(
            flash.storage[block as usize][page_in_block as usize][..byte_in_page]
                .iter()
                .all(|&x| x == 0xFF)
        );
        assert!(flash.storage[block as usize + 1][0].iter().all(|&x| x == 0));
        assert!(flash.storage[block as usize + 1][1][0..byte_in_page]
            .iter()
            .all(|&x| x == 0));
        assert!(flash.storage[block as usize + 1][1][byte_in_page..]
            .iter()
            .all(|&x| x == 0xFF));

        let mut rbuffer = [1; length];
        flash.read(offset as u32, &mut rbuffer).unwrap();
        assert_eq!(buffer, rbuffer);
    }
}
