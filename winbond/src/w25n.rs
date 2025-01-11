use embedded_hal::spi::SpiDevice;
use spi_flash::{utils::spi_transfer, JedecID, SpiFlashError, SpiNandRead, SpiNandWrite};

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

impl<SPI: SpiDevice, const N: u32> SpiNandRead<SPI> for W25N<N> {
    const PAGE_SIZE: u32 = 2048;
    const PAGES_PER_BLOCK: u32 = 64;
    const BLOCK_COUNT: u32 = N;

    fn read_jedec_id(&self, spi: &mut SPI) -> Result<JedecID, SpiFlashError<SPI>> {
        let mut buf = [0; 3];
        spi_transfer(
            spi,
            &mut buf,
            &[<W25N<N> as SpiNandRead<SPI>>::JEDEC_COMMAND, 0, 0],
        )?;
        Ok(JedecID::new(buf[2], 1))
    }
}

impl<SPI: SpiDevice, const N: u32> SpiNandWrite<SPI> for W25N<N> {}
