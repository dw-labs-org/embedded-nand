use spi_nand::SpiNand;

/// Concrete type that implements all the flash device features
/// for the W25N series of NAND flash devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct W25N<const B: u32, const ID: u16>();

/// Specific flash device with block count and ID and features
pub type W25N512G = W25N<512, 0xAA20>;
pub type W25N01GV = W25N<1024, 0xAA21>;
pub type W25N01JW = W25N<1024, 0xBC21>;
pub type W25N01KW = W25N<1024, 0xBC21>;
pub type W25N01KV = W25N<1024, 0xAE21>;
pub type W25N02K = W25N<2048, 0xAA22>;

impl<const B: u32, const ID: u16> W25N<B, ID> {
    /// Creates a new instance of the W25N flash device.
    pub fn new() -> Self {
        Self()
    }
}

impl<const B: u32, const ID: u16> Default for W25N<B, ID> {
    fn default() -> Self {
        Self::new()
    }
}
// All W25N devices have 2048 byte pages
impl<const B: u32, const ID: u16> SpiNand<2048> for W25N<B, ID> {
    const PAGES_PER_BLOCK: u32 = 64;
    const BLOCK_COUNT: u32 = B;
    const JEDEC_MANUFACTURER_ID: u8 = 0xEF;
    const JEDEC_DEVICE_ID: u16 = ID;
}

// Implement blocking trait
mod blocking {
    use super::W25N;
    use embedded_hal::spi::SpiDevice;
    use spi_nand::cmd_blocking::SpiNandBlocking;

    impl<SPI: SpiDevice, const B: u32, const ID: u16> SpiNandBlocking<SPI, 2048> for W25N<B, ID> {}
}

// Implement async trait
mod asyn {
    use super::W25N;
    use spi_nand::cmd_async::SpiNandAsync;

    impl<SPI: embedded_hal_async::spi::SpiDevice, const B: u32, const ID: u16>
        SpiNandAsync<SPI, 2048> for W25N<B, ID>
    {
    }
}
