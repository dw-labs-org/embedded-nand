use embedded_hal::spi::SpiDevice;
use spi_flash::{
    blocking::{utils::spi_transfer, SpiFlashError, SpiNandBlocking},
    JedecID, SpiNand,
};

/// Concrete type that implements all the flash device features
pub struct W25N<const N: u32>();

// /// Alias for [SpiFlash] that uses [W25NDevice]
// pub type W25N<SPI, const N: u32> = SpiFlash<SPI, W25NDevice<N>>;

/// Specific flash device with block count and features
pub type W25N02K = W25N<2048>;

impl<const N: u32> W25N<N> {
    pub fn new() -> Self {
        Self()
    }
}

impl<const N: u32> Default for W25N<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: u32> SpiNand for W25N<N> {
    const PAGE_SIZE: u32 = 2048;
    const PAGES_PER_BLOCK: u32 = 64;
    const BLOCK_COUNT: u32 = N;
}

impl<SPI: SpiDevice, const N: u32> SpiNandBlocking<SPI> for W25N<N> {
    fn read_jedec_id(&self, spi: &mut SPI) -> Result<JedecID, SpiFlashError<SPI>> {
        let mut buf = [0; 3];
        spi_transfer(spi, &mut buf, &[<W25N<N> as SpiNand>::JEDEC_COMMAND, 0, 0])?;
        Ok(JedecID::new(buf[2], 1))
    }
}
