#![no_std]

use core::{fmt::Debug, ops::Range};

use embedded_nand::{BlockIndex, ByteAddress, NandFlashError, NandFlashErrorKind, PageIndex};
use thiserror::Error;
mod fmt;
use embedded_nand::{AddressConversions, NandFlashIter};

/// Magic bytes at the start of the flashmap
const MAGIC: [u8; 4] = *b"FMAP";
/// Version of the flashmap
const VERSION: u16 = 1;

#[derive(Debug, Error)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<F: embedded_nand::NandFlash> {
    #[error("Flash")]
    Flash(F::Error),
    #[error("Invalid configuration for map")]
    InvalidConfg,
    #[error("Not enough valid blocks")]
    NotEnoughValidBlocks,
    #[error("Request not aligned")]
    NotAligned,
    #[error("Request out of bounds")]
    OutOfBounds,
    #[error("No superblocks available")]
    NoSuperBlocks,
    #[error("Other Error")]
    Other,
}

impl<F> embedded_nand::NandFlashError for Error<F>
where
    F: embedded_nand::NandFlash + Debug,
{
    fn kind(&self) -> embedded_nand::NandFlashErrorKind {
        match self {
            Error::Flash(e) => e.kind(),
            Error::InvalidConfg => embedded_nand::NandFlashErrorKind::Other,
            Error::NotEnoughValidBlocks => embedded_nand::NandFlashErrorKind::Other,
            Error::NoSuperBlocks => embedded_nand::NandFlashErrorKind::Other,
            Error::NotAligned => embedded_nand::NandFlashErrorKind::NotAligned,
            Error::OutOfBounds => embedded_nand::NandFlashErrorKind::OutOfBounds,
            Error::Other => embedded_nand::NandFlashErrorKind::Other,
        }
    }
}

// This is for convenience, to convert from the NandFlashErrorKind to the Error
// when calling the check_slice functions
impl<F> From<NandFlashErrorKind> for Error<F>
where
    F: embedded_nand::NandFlash,
{
    fn from(e: NandFlashErrorKind) -> Self {
        match e {
            NandFlashErrorKind::NotAligned => Error::NotAligned,
            NandFlashErrorKind::OutOfBounds => Error::OutOfBounds,
            NandFlashErrorKind::Other => Error::Other,
            _ => todo!(),
        }
    }
}

/// Mapping of logical blocks to physical blocks
///
/// BC is the number of blocks in the device
/// LBC is the number of logical blocks
///
/// The map consumes the first 2 valid blocks in the device.
/// Spare blocks = BC - LBC - 2
#[derive(Debug)]
pub struct FlashMap<F, const LBC: usize> {
    /// The flash device that is used to store the mapping
    flash: F,
    /// Data that defines the map
    data: FlashMapData<LBC>,
    /// Number of pages for the [FlashMapData] struct, map array and terminator
    map_page_count: u32,
    /// Address of map data
    data_address: ByteAddress,
}

impl<F, const LBC: usize> FlashMap<F, LBC>
where
    F: embedded_nand::NandFlash + Debug,
{
    /// Try to load the mapping from flash, create new one if not present
    ///
    /// SPI flash must be initialised (verify prescence, disable block protection)
    pub fn init(flash: F) -> Result<Self, Error<F>> {
        // Do some verification on block count, logical block count etc.
        if (LBC + 2) > F::BLOCK_COUNT as usize {
            return Err(Error::InvalidConfg);
        }
        if LBC == 0 {
            return Err(Error::InvalidConfg);
        }

        info!(
            "Initialising flashmap with {} logical blocks and {} physical blocks",
            LBC,
            F::BLOCK_COUNT
        );

        // Calculate how many pages the map takes up in flash
        let map_page_count = (Self::map_size_in_flash()).div_ceil(F::PAGE_SIZE) as u32;
        info!(
            "Pages per flashmap: {} ({} bytes)",
            map_page_count,
            Self::map_size_in_flash()
        );

        // Create a new flashmap
        let mut flashmap = FlashMap {
            flash,
            data: FlashMapData::new(F::BLOCK_COUNT as u16, LBC as u16),
            data_address: Default::default(),
            map_page_count,
        };

        // Track which are the first 2 valid blocks.
        // This is only used if a new map is created.
        // If a map is found, the values in the map are used to ensure that if one block
        // is bad, no other blocks are overwritten
        let mut count = 0;
        let mut map_blocks = [BlockIndex::default(); 2];
        let mut valid_header = None;
        // Go through first 2 valid blocks to try find a map
        for (block_ind, block_address) in flashmap.flash.block_iter_from(BlockIndex::new(0)) {
            debug!(
                "Checking block {} at {} for map",
                block_ind.as_u16(),
                block_address.as_u32()
            );
            // check if block is good
            if flashmap
                .flash
                .block_status(block_ind)
                .map_err(|e| Error::Flash(e))?
                .is_ok()
            {
                map_blocks[count] = block_ind;
                count += 1;

                // Go through each possible map location
                for page in (0..F::PAGES_PER_BLOCK).step_by(map_page_count as usize) {
                    let address = block_address + (page * F::PAGE_SIZE) as u32;
                    // Check if the map is valid
                    if let Some(new_header) = flashmap
                        .get_map_data(address)
                        .map_err(|e| Error::Flash(e))?
                    {
                        debug!(
                            "Found valid map at {} with {} writes",
                            address.as_u32(),
                            new_header.write_count
                        );
                        // Check if it is more recent than the current one
                        if let Some((current_header, _)) = valid_header {
                            if new_header > current_header {
                                // Found a more recent map
                                valid_header = Some((new_header, address));
                            }
                        } else {
                            // First valid map found
                            valid_header = Some((new_header, address));
                        }
                    }
                }
                if count >= 2 {
                    break;
                }
            }
        }

        // If a valid map was found, load it
        if let Some((new_map, address)) = valid_header {
            flashmap.data.header = new_map;
            flashmap.data_address = address;
            flashmap.load_map_array()?;
            info!(
                "Loaded map from {} with {} writes",
                address.as_u32(),
                flashmap.data.header.write_count
            );
            return Ok(flashmap);
        }
        // No valid map found, create a new one
        info!("No valid map found, creating new one");
        let first_block = map_blocks[1] + 1;
        debug!("First block to use: {}", first_block.as_u16());

        // Iterate over the blocks to find LBC good blocks
        let mut logical_ind = 0;
        let mut final_block = None;
        for (block_ind, _) in flashmap.flash.block_iter_from(first_block) {
            // Check if the block is good
            if flashmap
                .flash
                .block_status(block_ind)
                .map_err(|e| Error::Flash(e))?
                .is_ok()
            {
                // Record logical to physical mapping
                flashmap.data.map[logical_ind] = block_ind;
                logical_ind += 1;
                // Check if we have enough blocks
                if logical_ind >= LBC {
                    // We have enough blocks, so break out of the loop
                    final_block = Some(block_ind);

                    break;
                }
            } else {
                warn!("Block {} is bad", block_ind.as_u16());
            }
        }

        // check that we got at least LBC blocks
        if logical_ind < LBC {
            error!("Not enough valid blocks found");
            return Err(Error::NotEnoughValidBlocks);
        } else if let Some(next) = final_block {
            // Set the next block to use
            flashmap.data.header.final_block = next;
            info!("Next block to use: {}", next.as_u16());
        } else {
            warn!("No valid blocks remaining");
            flashmap.data.header.final_block = BlockIndex::new(0);
        }
        // Save the map config
        flashmap.data.header.map_blocks = map_blocks;
        flashmap.data.header.write_count = 1;
        flashmap.data_address = Self::block_to_byte_address(map_blocks[0]);
        // erase the block that will be used for the map
        debug!("Erasing block {} for map", map_blocks[0].as_u16());
        flashmap
            .flash
            .erase_block(map_blocks[0])
            .map_err(|e| Error::Flash(e))?;
        // Write the map to flash
        flashmap.write_map()?;

        Ok(flashmap)
    }

    /// Size of flashmap, map array and terminator in bytes
    const fn map_size_in_flash() -> usize {
        // Size of the map array
        let map_size = core::mem::size_of::<u16>() * LBC;
        // Size of the flashmap data structure
        let data_size = core::mem::size_of::<FlashMapHeader>();
        // Size of the terminator
        let terminator_size = MAGIC.len();
        // Total size
        map_size + data_size + terminator_size
    }

    /// Convert a logical block index to a physical block
    fn logical_to_physical(&self, logical_block: BlockIndex) -> Result<BlockIndex, Error<F>> {
        if logical_block.as_u16() >= LBC as u16 {
            return Err(Error::OutOfBounds);
        }
        Ok(self.data.map[logical_block.as_u16() as usize])
    }

    /// Convert a logical page index to a physical page
    fn _logical_to_physical_page(&self, logical_page: PageIndex) -> Result<PageIndex, Error<F>> {
        if logical_page.as_u32() >= (LBC as u32 * F::PAGES_PER_BLOCK as u32) {
            return Err(Error::OutOfBounds);
        }
        let logical_block = logical_page.as_block_index(F::PAGES_PER_BLOCK as u32);
        let physical_block = self.logical_to_physical(logical_block)?;
        let page_offset = Self::page_in_block(logical_page);
        Ok(Self::block_to_page_index(physical_block) + page_offset)
    }

    /// Convert a logical byte address to a physical byte address
    fn logical_to_physical_byte(&self, logical_byte: ByteAddress) -> Result<ByteAddress, Error<F>> {
        let logical_block = Self::byte_to_block_index(logical_byte);
        let physical_block = self.logical_to_physical(logical_block)?;
        let block_offset = Self::byte_in_block(logical_byte);
        Ok(Self::block_to_byte_address(physical_block) + block_offset)
    }

    /// Address of the map array
    fn map_address(&self) -> ByteAddress {
        self.data_address + core::mem::size_of::<FlashMapHeader>() as u32
    }

    /// Try to load a map from the given address
    fn get_map_data(&mut self, address: ByteAddress) -> Result<Option<FlashMapHeader>, F::Error> {
        // Create data and
        let mut data = FlashMapHeader::default();

        let slice = unsafe {
            core::slice::from_raw_parts_mut(
                &mut data as *mut FlashMapHeader as *mut u8,
                core::mem::size_of::<FlashMapHeader>(),
            )
        };
        self.flash.read(address.into(), slice)?;
        // Check it matches what is expected
        if self.data.header.is_valid(&data) {
            // check that terminating magic bytes are present
            let mut term = [0; 4];
            let term_location = Self::map_size_in_flash() - MAGIC.len();
            self.flash
                .read((address + term_location as u32).as_u32(), &mut term)?;

            if term != MAGIC {
                trace!("Missing terminator at {:#X}", address);
                Ok(None)
            } else {
                trace!("Found valid map at {:#X}", address);
                Ok(Some(data))
            }
        } else {
            // Logs as to why this isnt a map (if magic bytes are present)
            // Would be better to pass this on and have a force init function
            if data.magic == MAGIC {
                warn!("Invalid map at {:#X}", address);
                warn!("Version: {} != {}", data.version, VERSION);
                warn!("Block count: {} != {}", data.block_count, F::BLOCK_COUNT);
                warn!(
                    "Logical block count: {} != {}",
                    data.logical_block_count, LBC
                );
            }
            Ok(None)
        }
    }

    /// Load the map array from flash. Must only be called when it is guaranteed to be valid.
    ///
    /// Map must have been loaded with [Self::get_map_data].
    fn load_map_array(&mut self) -> Result<(), Error<F>> {
        let map_address = self.map_address();
        debug!("Loading map array from {:#X}", map_address);
        let slice = unsafe {
            core::slice::from_raw_parts_mut(
                &mut self.data.map as *mut [BlockIndex; LBC] as *mut u8,
                core::mem::size_of::<[u16; LBC]>(),
            )
        };
        self.flash
            .read(map_address.as_u32(), slice)
            .map_err(|e| Error::Flash(e))?;
        Ok(())
    }

    /// Updates the map on flash.
    ///
    /// increments write count, goes to other block when run out of space on current
    fn update_map(&mut self) -> Result<(), Error<F>> {
        // Increment the write count
        self.data.header.write_count += 1;
        // Check if we need to write to a new block
        let current_block = Self::byte_to_block_index(self.data_address);
        self.data_address += self.map_page_count * F::PAGE_SIZE as u32;
        let mut new_block = Self::byte_to_block_index(self.data_address);
        if new_block != current_block {
            // Get the other block for map
            new_block = if self.data.header.map_blocks[0] == current_block {
                self.data.header.map_blocks[1]
            } else {
                self.data.header.map_blocks[0]
            };
            // Erase the block
            // if it fails, keep using the current block
            if !self.checked_erase_block(new_block)? {
                warn!(
                    "Failed to erase block {}, only 1 superblock available",
                    new_block
                );
                let _ = self.flash.mark_block_bad(new_block);
                new_block = current_block;
                // if cannot erase other block, critical error
                if !self.checked_erase_block(new_block)? {
                    error!(
                        "Failed to erase block {}, no superblocks available",
                        new_block
                    );
                    let _ = self.flash.mark_block_bad(new_block);
                    return Err(Error::NoSuperBlocks);
                }
            }
            // Update the address
            self.data_address = Self::block_to_byte_address(new_block);
        }

        // Write the map to flash
        self.write_map()?;
        Ok(())
    }

    /// Write the map to flash.
    ///
    /// Includes the map data, map array and terminator.
    fn write_map(&mut self) -> Result<(), Error<F>> {
        trace!("Writing map to {}", self.data_address);
        // Cast the data structure to a byte array
        let slice = unsafe {
            core::slice::from_raw_parts_mut(
                &mut self.data as *mut FlashMapData<LBC> as *mut u8,
                core::mem::size_of::<FlashMapData<LBC>>(),
            )
        };
        // Write the data to flash
        self.flash
            .write(self.data_address.as_u32(), slice)
            .map_err(|e| Error::Flash(e))?;
        Ok(())
    }

    /// Finds the next usable block that is spare.
    ///
    /// This will erase the block and return the block number, updating the final block in the header.
    /// If no blocks are available, it will return an error.
    fn next_spare_block(&mut self) -> Result<BlockIndex, Error<F>> {
        loop {
            self.data.header.final_block += 1;
            if self.data.header.final_block.as_u16() >= self.data.header.block_count {
                // No more blocks available
                return Err(Error::NotEnoughValidBlocks);
            }
            // Check if the block is good
            if self
                .flash
                .block_status(self.data.header.final_block)
                .map_err(|e| Error::Flash(e))?
                .is_ok()
                && self.flash.erase_block(self.data.header.final_block).is_ok()
            {
                return Ok(self.data.header.final_block);
            }
        }
    }

    /// Erase a physical block, checking if the erase fails and it needs replacing.
    ///
    /// Returns true if Ok, false if the block is bad.
    fn checked_erase_block(&mut self, block: BlockIndex) -> Result<bool, Error<F>> {
        match self.flash.erase_block(block) {
            Ok(_) => Ok(true),
            // check if block has failed
            Err(e) => {
                if let embedded_nand::NandFlashErrorKind::BlockFail(_) = e.kind() {
                    // Return false as its a block fail error
                    Ok(false)
                } else {
                    Err(Error::Flash(e))
                }
            }
        }
    }

    /// Read a physical slice from flash that does not cross a block boundary, checking for block errors
    ///
    /// WARNING: Does not move data if failing, up to caller to handle this.
    fn checked_read_slice(
        &mut self,
        offset: ByteAddress,
        bytes: &mut [u8],
    ) -> Result<bool, Error<F>> {
        match self.flash.read(offset.as_u32(), bytes) {
            Ok(_) => Ok(true),
            Err(e) => match e.kind() {
                embedded_nand::NandFlashErrorKind::BlockFailing(_) => Ok(false),
                _ => Err(Error::Flash(e)),
            },
        }
    }

    /// Write a physical slice to flash that does not cross a block boundary, checking for block errors
    ///
    /// WARNING: Does not move data if failing, up to caller to handle this.
    ///
    /// Both BlockFailing and BlockFail are considered recoverable errors.
    fn checked_write_slice(&mut self, offset: ByteAddress, bytes: &[u8]) -> Result<bool, Error<F>> {
        match self.flash.write(offset.as_u32(), bytes) {
            Ok(_) => Ok(true),
            Err(e) => match e.kind() {
                embedded_nand::NandFlashErrorKind::BlockFailing(_) => Ok(false),
                embedded_nand::NandFlashErrorKind::BlockFail(_) => Ok(false),
                _ => Err(Error::Flash(e)),
            },
        }
    }

    /// Move the physical block of a logical block to a new location.
    ///
    /// Copies over the block up to length, updates the map, marks the old block as bad
    /// and erases it.
    fn remap_block(&mut self, logical_block: BlockIndex, length: u32) -> Result<(), Error<F>> {
        // Get the physical block
        let physical_block = self.logical_to_physical(logical_block)?;

        // Get the next spare block
        let next_block = self.next_spare_block()?;
        info!(
            "Remapping block {} from {} to {}",
            logical_block, physical_block, next_block
        );
        // Copy the block to the new location
        self.flash
            .copy(
                physical_block
                    .as_byte_address(F::ERASE_SIZE as u32)
                    .as_u32(),
                next_block.as_byte_address(F::ERASE_SIZE as u32).as_u32(),
                length,
            )
            .map_err(|e| Error::Flash(e))?;
        // Mark the old block as bad (ignore errors)
        let _ = self.flash.mark_block_bad(physical_block);
        // Update the map
        self.data.map[logical_block.as_u16() as usize] = next_block;
        // Write the map to flash
        self.update_map()
    }

    /// Convert a logical offset and slice into a physical offset and range within block boundaries
    fn logical_to_physical_range(
        &self,
        logical_offset: u32,
        bytes: &[u8],
        slice_offset: usize,
    ) -> Result<(ByteAddress, Range<usize>), Error<F>> {
        let logical_offset = ByteAddress::new(logical_offset + slice_offset as u32);
        // Get the logical block
        let logical_block = logical_offset.as_block_index(F::ERASE_SIZE as u32);
        // Get the physical block
        let physical_block = self.logical_to_physical(logical_block)?;
        // Get the offset into the block
        let block_offset = logical_offset.block_offset(F::ERASE_SIZE as u32);
        // Get the physical byte address
        let physical_offset = physical_block.as_byte_address(F::ERASE_SIZE as u32) + block_offset;
        // Get the number of bytes to read
        let block_remaining = F::ERASE_SIZE as u32 - block_offset;
        // The number of bytes left to in slice
        let bytes_remaining = bytes.len() as u32 - slice_offset as u32;
        // Get the number of bytes to read
        let read_length = if bytes_remaining > block_remaining {
            block_remaining
        } else {
            bytes.len() as u32
        };

        Ok((
            physical_offset,
            slice_offset..(slice_offset + read_length as usize),
        ))
    }
}

impl<F, const LBC: usize> embedded_nand::ErrorType for FlashMap<F, LBC>
where
    F: embedded_nand::NandFlash + Debug,
{
    type Error = Error<F>;
}

// Impl the embedded nand trait to form the core public interface
impl<F, const LBC: usize> embedded_nand::NandFlash for FlashMap<F, LBC>
where
    F: embedded_nand::NandFlash + Debug,
{
    const READ_SIZE: usize = F::READ_SIZE;
    const PAGE_SIZE: usize = F::PAGE_SIZE;
    const PAGES_PER_BLOCK: usize = F::PAGES_PER_BLOCK;
    const BLOCK_COUNT: usize = LBC as usize;
    const ERASE_SIZE: usize = F::ERASE_SIZE;
    const WRITE_SIZE: usize = F::WRITE_SIZE;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        // Only required to not read over block boundaries (would invalidate the map)
        // Alignment is checked by the flash device

        // Track number of bytes read into buffer
        let mut read = 0;
        loop {
            let (physical_offset, range) = self.logical_to_physical_range(offset, bytes, read)?;
            read += range.len();
            if !self.checked_read_slice(physical_offset, &mut bytes[range])? {
                // Block is failing but read was fine, remap the whole block
                self.remap_block(
                    BlockIndex::from_raw_byte_offset(offset, F::ERASE_SIZE as u32),
                    Self::ERASE_SIZE as u32,
                )?;
            }
            if read >= bytes.len() {
                return Ok(());
            }
        }
    }

    fn capacity(&self) -> u32 {
        // The capacity of the device is the number of blocks * block size
        F::BLOCK_COUNT as u32 * Self::PAGES_PER_BLOCK as u32 * F::PAGE_SIZE as u32
    }

    /// This should always return OK
    fn block_status(
        &mut self,
        block: BlockIndex,
    ) -> Result<embedded_nand::BlockStatus, Self::Error> {
        self.flash
            .block_status(self.logical_to_physical(block)?)
            .map_err(|e| Error::Flash(e))
    }

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        // check alignment
        for (block, _) in self.block_iter_range(
            Self::byte_to_block_index(ByteAddress::new(from)),
            Self::byte_to_block_index(ByteAddress::new(to)),
        ) {
            self.erase_block(block)?;
        }
        Ok(())
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        // Only required to not write over block boundaries (would invalidate the map)
        // Alignment is checked by the flash device

        // Track number of bytes written into buffer
        let mut written = 0;
        loop {
            // Get the physical offset and range to write
            let (physical_offset, range) =
                self.logical_to_physical_range(offset, bytes, written)?;

            let write_length = range.len();

            // Try to write the slice to flash
            if !self.checked_write_slice(physical_offset, &bytes[range])? {
                // Block is failing, write was not successful
                // Remap the block and try again on new physical block
                // Only remap up to just before this write
                self.remap_block(
                    BlockIndex::from_raw_byte_offset(offset, F::ERASE_SIZE as u32),
                    physical_offset.block_offset(Self::ERASE_SIZE as u32),
                )?;
                // Continue allows a retry on the new block
                continue;
            }
            // Exit when all bytes are written
            written += write_length;
            if written >= bytes.len() {
                return Ok(());
            }
        }
    }

    /// Marks the underlying physical block of the supplied logical block as bad and remaps.
    ///
    /// This will find the next spare block and remap the logical block to it.
    /// WARNING: Does not move data, which is effectively lost.
    fn mark_block_bad(&mut self, block: BlockIndex) -> Result<(), Self::Error> {
        let physical = self.logical_to_physical(block)?;
        // Mark the block as bad (ignore error if it fails)
        let _ = self.flash.mark_block_bad(physical);
        // Find the next valid block
        let next_block = self.next_spare_block()?;
        // Update the mapping
        self.data.map[block.as_u16() as usize] = next_block;
        // write to flash
        self.update_map()
    }

    /// Erases the physical block of the supplied logical block.
    ///
    /// If the erase fails with [embedded_nand::NandFlashErrorKind::BlockFail], it will mark the block as bad and remap it.
    fn erase_block(&mut self, block: BlockIndex) -> Result<(), Self::Error> {
        let block = self.logical_to_physical(block)?;
        // Erase the block, checing for fail
        if self.checked_erase_block(block)? {
            Ok(())
        } else {
            self.mark_block_bad(block)
        }
    }

    fn copy(&mut self, src_offset: u32, dest_offset: u32, length: u32) -> Result<(), Self::Error> {
        // Check that everything is within a single block
        let src_block = Self::byte_to_block_index(ByteAddress::new(src_offset));
        let final_block = Self::byte_to_block_index(ByteAddress::new(src_offset + length));
        if src_block != final_block {
            error!("Cannot copy slice over a block boundary");
            return Err(Error::NotAligned);
        }
        // Pass copy operation to the flash device
        self.flash
            .copy(
                self.logical_to_physical_byte(ByteAddress::new(src_offset))?
                    .as_u32(),
                self.logical_to_physical_byte(ByteAddress::new(dest_offset))?
                    .as_u32(),
                length,
            )
            .map_err(|e| Error::Flash(e))
    }
}

/// Data structure that is written to flash consiting of header, map array and terminator
#[derive(Debug)]
#[repr(C)]
struct FlashMapData<const LBC: usize> {
    /// Header
    header: FlashMapHeader,
    /// Map array
    map: [BlockIndex; LBC],
    /// Terminator
    terminator: [u8; 4],
}

impl<const LBC: usize> FlashMapData<LBC> {
    /// Create a new instance of the data structure
    fn new(block_count: u16, logical_block_count: u16) -> Self {
        FlashMapData {
            header: FlashMapHeader::new(block_count, logical_block_count),
            map: [BlockIndex::new(0); LBC],
            terminator: MAGIC,
        }
    }
}

/// Data structure  that contains the
/// configuration of the mapping
///
/// When written to flash, it takes up an integer number of pages.
///
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct FlashMapHeader {
    /// Magic bytes that idenitfy the structure
    magic: [u8; 4],
    /// Version of the structure
    version: u16,
    /// Number of blocks in the device
    block_count: u16,
    /// Number of logical blocks
    logical_block_count: u16,
    /// Next block to use
    final_block: BlockIndex,
    /// Number of times the map has been written
    write_count: u32,
    /// The blocks used for the map
    map_blocks: [BlockIndex; 2],
}

impl Ord for FlashMapHeader {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.write_count.cmp(&other.write_count)
    }
}

impl PartialOrd for FlashMapHeader {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl FlashMapHeader {
    /// Create a new instance of the data structure
    fn new(block_count: u16, logical_block_count: u16) -> Self {
        FlashMapHeader {
            magic: MAGIC,
            version: VERSION,
            block_count,
            logical_block_count,
            final_block: BlockIndex::new(0),
            write_count: 0,
            map_blocks: [BlockIndex::new(0); 2],
        }
    }

    /// Compare 2 instances and return if they are valid
    /// Magic, version, block count and logical block count must be the same
    fn is_valid(&self, other: &Self) -> bool {
        self.magic == other.magic
            && self.version == other.version
            && self.block_count == other.block_count
            && self.logical_block_count == other.logical_block_count
    }
}

#[cfg(test)]
mod tests {}
