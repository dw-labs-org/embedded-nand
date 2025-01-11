use embedded_hal::spi::SpiDevice;
use spi_flash::{
    async_trait::{SpiFlashErrorASync, SpiNandAsync},
    blocking::{utils::spi_transfer, SpiFlashError, SpiNandBlocking},
    JedecID, SpiNand,
};

/// Concrete type that implements all the flash device features
pub struct W25N<const B: u32>();

// /// Alias for [SpiFlash] that uses [W25NDevice]
// pub type W25N<SPI, const N: u32> = SpiFlash<SPI, W25NDevice<N>>;

/// Specific flash device with block count and features
pub type W25N02K = W25N<2048>;
pub type W25N01K = W25N<1048>;

impl<const B: u32> W25N<B> {
    pub fn new() -> Self {
        Self()
    }
}

impl<const B: u32> Default for W25N<B> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const B: u32> SpiNand<2048> for W25N<B> {
    const PAGES_PER_BLOCK: u32 = 64;
    const BLOCK_COUNT: u32 = B;
}

impl<SPI: SpiDevice, const B: u32> SpiNandBlocking<SPI, 2048> for W25N<B> {
    fn read_jedec_id(&self, spi: &mut SPI) -> Result<JedecID, SpiFlashError<SPI>> {
        let mut buf = [0; 3];
        spi_transfer(
            spi,
            &mut buf,
            &[<W25N<B> as SpiNand<2048>>::JEDEC_COMMAND, 0, 0],
        )?;
        Ok(JedecID::new(buf[2], 1))
    }
}

impl<SPI: embedded_hal_async::spi::SpiDevice, const B: u32> SpiNandAsync<SPI, 2048> for W25N<B> {
    async fn read_jedec_id(&self, spi: &mut SPI) -> Result<JedecID, SpiFlashErrorASync<SPI>> {
        let mut buf = [0; 3];
        spi_flash::async_trait::utils::spi_transfer(
            spi,
            &mut buf,
            &[<W25N<B> as SpiNand<2048>>::JEDEC_COMMAND, 0, 0],
        )
        .await?;
        Ok(JedecID::new(buf[2], 1))
    }
}
