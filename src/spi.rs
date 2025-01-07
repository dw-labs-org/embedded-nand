use crate::traits::{ErrorType, NandFlash, NandFlashError, ReadNandFlash};
use core::fmt::Debug;
use embedded_hal::spi::{self, SpiDevice};

#[derive(Debug)]
/// Error type for SPI NAND flash operations
enum SpiNandError<SPI>
where
    SPI: SpiDevice,
{
    SPI(SPI::Error),
}

impl<SPI> NandFlashError for SpiNandError<SPI>
where
    SPI: SpiDevice + Debug,
{
    fn kind(&self) -> crate::traits::NandFlashErrorKind {
        todo!()
    }
}

/// Result from all SPI NAND flash operations
type SpiNandResult<T, SPI> = Result<T, SpiNandError<SPI>>;

/// Generic SPI NAND flash driver implementing common NAND flash operations.
/// Use a basis for specific flash chips
struct SpiNand<const C: u64, const B: u32, const P: u32, SPI: SpiDevice>(SPI);

impl<const C: u64, const B: u32, const P: u32, SPI: SpiDevice> SpiNand<C, B, P, SPI> {
    pub fn new(spi: SPI) -> Self {
        SpiNand(spi)
    }

    // Wrappers around SPI that map errors ===============>
    fn write(&mut self, buf: &[u8]) -> SpiNandResult<(), SPI> {
        self.0.write(buf).map_err(|e| SpiNandError::SPI(e))
    }
    fn read(&mut self, buf: &mut [u8]) -> SpiNandResult<(), SPI> {
        self.0.read(buf).map_err(|e| SpiNandError::SPI(e))
    }

    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> SpiNandResult<(), SPI> {
        self.0
            .transfer(read, write)
            .map_err(|e| SpiNandError::SPI(e))
    }

    fn transfer_in_place(&mut self, buf: &mut [u8]) -> SpiNandResult<(), SPI> {
        self.0
            .transfer_in_place(buf)
            .map_err(|e| SpiNandError::SPI(e))
    }

    fn transaction(&mut self, operations: &mut [spi::Operation<'_, u8>]) -> SpiNandResult<(), SPI> {
        self.0
            .transaction(operations)
            .map_err(|e| SpiNandError::SPI(e))
    }
}

impl<const C: u64, const B: u32, const P: u32, SPI: SpiDevice + Debug> ErrorType
    for SpiNand<C, B, P, SPI>
{
    type Error = SpiNandError<SPI>;
}

impl<const C: u64, const B: u32, const P: u32, SPI: SpiDevice> ReadNandFlash
    for SpiNand<C, B, P, SPI>
where
    SPI: SpiDevice + Debug,
{
    const READ_SIZE: usize = 1;

    fn read(&mut self, offset: u64, buffer: &mut [u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    fn capacity(&self) -> u64 {
        C
    }

    fn block_status(&mut self, address: u64) -> Result<crate::traits::BlockStatus, Self::Error> {
        todo!()
    }
}

impl<const C: u64, const B: u32, const P: u32, SPI: SpiDevice> NandFlash for SpiNand<C, B, P, SPI>
where
    SPI: SpiDevice + Debug,
{
    const WRITE_SIZE: usize = P as usize;

    const ERASE_SIZE: usize = B as usize;

    fn erase(&mut self, from: u64, to: u64) -> Result<(), Self::Error> {
        todo!()
    }

    fn write(&mut self, offset: u64, bytes: &[u8]) -> Result<(), Self::Error> {
        todo!()
    }

    fn mark_bad(&mut self, address: u64) -> Result<(), Self::Error> {
        Ok(())
    }
}
