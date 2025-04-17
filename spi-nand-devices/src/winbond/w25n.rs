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

/// ECC error threshold for considering a block failed when reading
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum ECCThreshold {
    /// 1 bit
    OneBit = 0b0001,
    /// 2 bits
    TwoBits = 0b0010,
    /// 3 bits
    ThreeBits = 0b0011,
    /// 4 bits
    FourBits = 0b0100,
    /// 5 bits
    FiveBits = 0b0101,
    /// 6 bits
    SixBits = 0b0110,
    /// 7 bits
    SevenBits = 0b0111,
}

// Implement blocking trait
mod blocking {
    use super::{ECCThreshold, W25N};
    use embedded_hal::spi::SpiDevice;
    use spi_nand::{
        cmd_async::utils::spi_transfer,
        cmd_blocking::{
            utils::{spi_read, spi_write},
            SpiNandBlocking,
        },
        error::SpiFlashError,
        ECCStatus,
    };

    /// For W25N that implement ECC ( I think all of them)
    trait ECC<SPI: SpiDevice, const N: usize>: SpiNandBlocking<SPI, N> {
        /// Located in register 2
        const ECC_ENABLE_REGISTER: u8 = 0xB0;
        /// bit 4
        const ECC_ENABLE_MASK: u8 = 0b10000;
        /// Extended registers. Only first register can be written
        const ECC_EXTENDED_REGISTERS: [u8; 5] = [0x10, 0x20, 0x30, 0x40, 0x50];

        /// Enable ECC
        fn enable_ecc(&self, spi: &mut SPI) -> Result<(), SpiFlashError<SPI::Error>> {
            self.set_register_cmd(spi, Self::ECC_ENABLE_REGISTER, Self::ECC_ENABLE_MASK)
        }
        /// Disable ECC
        fn disable_ecc(&self, spi: &mut SPI) -> Result<(), SpiFlashError<SPI::Error>> {
            self.clear_register_cmd(spi, Self::ECC_ENABLE_REGISTER, Self::ECC_ENABLE_MASK)
        }
        /// Read the ECC status bits        
        fn ecc_status(&self, spi: &mut SPI) -> Result<ECCStatus, SpiFlashError<SPI::Error>> {
            let status = self.read_register_cmd(spi, Self::STATUS_REGISTER)? & 0x30;
            match status {
                0x00 => Ok(ECCStatus::Ok),
                0x10 => Ok(ECCStatus::Corrected),
                0x20 => Ok(ECCStatus::Failed),
                _ => Ok(ECCStatus::Failing),
            }
        }
        /// Get the bit flip detect threshold (1 to 7 bits)
        fn ecc_bit_flip_threshold(&self, spi: &mut SPI) -> Result<u8, SpiFlashError<SPI::Error>> {
            Ok(self.read_register_cmd(spi, Self::ECC_EXTENDED_REGISTERS[0])? >> 4)
        }

        /// Set the bit flip detect threshold (1 to 7 bits)
        fn ecc_set_bit_flip_threshold(
            &self,
            spi: &mut SPI,
            threshold: ECCThreshold,
        ) -> Result<(), SpiFlashError<SPI::Error>> {
            self.write_register_cmd(spi, Self::ECC_EXTENDED_REGISTERS[0], (threshold as u8) << 4)
        }

        /// Get the bit flip count detection status (BFS3->BFS0)
        fn ecc_bit_flip_count_status(
            &self,
            spi: &mut SPI,
        ) -> Result<u8, SpiFlashError<SPI::Error>> {
            self.read_register_cmd(spi, Self::ECC_EXTENDED_REGISTERS[1])
        }

        /// Get the bit flip count report (BFR15->BFR0)
        fn ecc_bit_flip_count_report(
            &self,
            spi: &mut SPI,
        ) -> Result<u16, SpiFlashError<SPI::Error>> {
            Ok(
                (((self.read_register_cmd(spi, Self::ECC_EXTENDED_REGISTERS[4])?) as u16) << 8)
                    + (self.read_register_cmd(spi, Self::ECC_EXTENDED_REGISTERS[3])? as u16),
            )
        }
    }

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
