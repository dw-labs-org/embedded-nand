use spi_nand::SpiNand;

/// Concrete type that implements all the flash device features
/// for the W25N series of NAND flash devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct W25N<const B: u32>();

/// Specific flash device with block count and features
pub type W25N02K = W25N<2048>;
pub type W25N01K = W25N<1048>;

impl<const B: u32> W25N<B> {
    /// Creates a new instance of the W25N flash device.
    pub fn new() -> Self {
        Self()
    }
}

impl<const B: u32> Default for W25N<B> {
    fn default() -> Self {
        Self::new()
    }
}
// All W25N devices have 2048 byte pages
impl<const B: u32> SpiNand<2048> for W25N<B> {
    const PAGES_PER_BLOCK: u32 = 64;
    const BLOCK_COUNT: u32 = B;
}

// Implement blocking trait
mod blocking {
    use spi_nand::{
        cmd_blocking::{utils::spi_transfer, SpiNandBlocking},
        error::SpiFlashError,
        JedecID, SpiNand,
    };

    use super::W25N;

    impl<SPI: embedded_hal::spi::SpiDevice, const B: u32> SpiNandBlocking<SPI, 2048> for W25N<B> {
        fn read_jedec_id_cmd(&self, spi: &mut SPI) -> Result<JedecID, SpiFlashError<SPI::Error>> {
            let mut buf = [0; 3];
            spi_transfer(
                spi,
                &mut buf,
                &[<W25N<B> as SpiNand<2048>>::JEDEC_COMMAND, 0, 0],
            )?;
            Ok(JedecID::new(buf[2], 1))
        }
    }
}

// Implement async trait
mod asyn {
    use spi_nand::cmd_async::SpiNandAsync;

    use super::W25N;

    impl<SPI: embedded_hal_async::spi::SpiDevice, const B: u32> SpiNandAsync<SPI, 2048> for W25N<B> {
        // async fn read_jedec_id(&self, spi: &mut SPI) -> Result<JedecID, SpiFlashErrorASync<SPI>> {
        //     let mut buf = [0; 3];
        //     spi_flash::async_trait::utils::spi_transfer(
        //         spi,
        //         &mut buf,
        //         &[<W25N<B> as SpiNand<2048>>::JEDEC_COMMAND, 0, 0],
        //     )
        //     .await?;
        //     Ok(JedecID::new(buf[2], 1))
        // }
    }
}
