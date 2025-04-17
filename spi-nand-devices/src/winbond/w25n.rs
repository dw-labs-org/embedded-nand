use spi_nand::SpiNand;

/// Concrete type that implements all the flash device features
/// for the W25N series of NAND flash devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct W25N<const B: u32, const ID: u16>();

/// Specific flash device with block count and ID and features
pub type W25N512G = W25N<512, 0xAA20>;
impl ECCBasic for W25N512G {}
impl ODS for W25N512G {}
impl HoldDisable for W25N512G {}
impl ContinuousRead for W25N512G {}
impl BBM for W25N512G {}

pub type W25N01GV = W25N<1024, 0xAA21>;
pub type W25N01JW = W25N<1024, 0xBC21>;
pub type W25N01KW = W25N<1024, 0xBC21>;
pub type W25N01KV = W25N<1024, 0xAE21>;

pub type W25N02KV = W25N<2048, 0xAA22>;
impl ECC for W25N02KV {}

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

// ================== Feature traits ==================

/// For devices that implement Basic ECC. (single bit correction)
trait ECCBasic {
    // Register of 1 bit
    const ECC_ENABLE_REGISTER: u8 = 0xB0;
    // Position of lsb
    const ECC_ENABLE_BIT: u8 = 4;
    // Location of ECC status bits
    const ECC_STATUS_REGISTER: u8 = 0xC0;
    // Location of ECC status bits
    const ECC_STATUS_BIT: u8 = 4;
    // Command to lookup ECC page failure
    const ECC_PAGE_FAILURE_COMMAND: u8 = 0xA9;
}

/// For devices that implement ECC with configurable threshold (W25N512G)
trait ECC {
    /// Located in register 2
    const ECC_ENABLE_REGISTER: u8 = 0xB0;
    /// bit 4
    const ECC_ENABLE_MASK: u8 = 0b10000;
    // Location of ECC status bits
    const ECC_STATUS_REGISTER: u8 = 0xC0;
    // Location of ECC status bits
    const ECC_STATUS_BIT: u8 = 4;
    /// Extended registers. Only first register can be written
    const ECC_EXTENDED_REGISTERS: [u8; 5] = [0x10, 0x20, 0x30, 0x40, 0x50];
}

/// Configurable output driver strength
trait ODS {
    // Register of 2 bits
    const ODS_REGISTER: u8 = 0xB0;
    // Position of lsb
    const ODS_BIT: u8 = 1;
}

/// Hold disable
trait HoldDisable {
    // Register of 1 bit
    const HOLD_DISABLE_REGISTER: u8 = 0xB0;
    // Position of lsb
    const HOLD_DISABLE_BIT: u8 = 0;
}

/// Continuous read
trait ContinuousRead {
    // Register of 1 bit
    const CONTINUOUS_READ_REGISTER: u8 = 0xB0;
    // Position of BUF bit
    const CONTINUOUS_READ_BIT: u8 = 3;
}

/// Bad block managment with loookup table
trait BBM {
    // Command to swap block
    const SWAP_BLOCK_COMMAND: u8 = 0xA1;
    // Command to read LUT
    const READ_LUT_COMMAND: u8 = 0xA5;
    // Register with LUT full flag
    const LUT_FULL_REGISTER: u8 = 0xB0;
    // Position of LUT full flag
    const LUT_FULL_BIT: u8 = 6;
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

/// Output driver strength
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum ODSStrength {
    /// 100%
    Full = 0b00,
    /// 75%
    ThreeQuarters = 0b01,
    /// 50%
    Half = 0b10,
    /// 25%
    Quarter = 0b11,
}

impl From<u8> for ODSStrength {
    fn from(value: u8) -> Self {
        match value {
            0b00 => ODSStrength::Full,
            0b01 => ODSStrength::ThreeQuarters,
            0b10 => ODSStrength::Half,
            0b11 => ODSStrength::Quarter,
            _ => unreachable!(),
        }
    }
}

// Implement blocking trait
mod blocking {
    use super::{ECCBasic, ECCThreshold, ODSStrength, BBM, ECC, ODS, W25N};
    use embedded_hal::spi::SpiDevice;
    use embedded_nand::{BlockIndex, PageIndex};
    use spi_nand::{
        cmd_blocking::{
            utils::{spi_transfer_in_place, spi_write},
            SpiNandBlocking,
        },
        error::SpiFlashError,
        ECCStatus,
    };

    /// For W25N that implement the basic ECC
    pub trait ECCBasicBlocking<SPI: SpiDevice, const N: usize>:
        SpiNandBlocking<SPI, N> + ECCBasic
    {
        /// Enable ECC
        fn enable_ecc(&self, spi: &mut SPI) -> Result<(), SpiFlashError<SPI::Error>> {
            self.set_register_cmd(spi, Self::ECC_ENABLE_REGISTER, Self::ECC_ENABLE_BIT)
        }
        /// Disable ECC
        fn disable_ecc(&self, spi: &mut SPI) -> Result<(), SpiFlashError<SPI::Error>> {
            self.clear_register_cmd(spi, Self::ECC_ENABLE_REGISTER, Self::ECC_ENABLE_BIT)
        }
        /// Read the ECC status bits        
        fn ecc_status(&self, spi: &mut SPI) -> Result<ECCStatus, SpiFlashError<SPI::Error>> {
            let status = (self.read_register_cmd(spi, Self::ECC_STATUS_REGISTER)?
                >> Self::ECC_STATUS_BIT)
                & 0b11;
            match status {
                0b00 => Ok(ECCStatus::Ok),
                0b01 => Ok(ECCStatus::Corrected),
                0b10 => Ok(ECCStatus::Failed),
                // Can only happen in continuous read mode
                _ => Ok(ECCStatus::Failed),
            }
        }

        /// Get the last ECC page failure. Only applicable in continuous read mode
        fn ecc_last_page_failure(
            &self,
            spi: &mut SPI,
        ) -> Result<PageIndex, SpiFlashError<SPI::Error>> {
            let mut buf = [Self::ECC_PAGE_FAILURE_COMMAND, 0, 0, 0];
            spi_transfer_in_place(spi, &mut buf)?;
            // contruct page index from bytes
            Ok(PageIndex::from(&buf[1..].try_into().unwrap()))
        }
    }

    /// For W25N that implement the more advanced ECC
    pub trait ECCBlocking<SPI: SpiDevice, const N: usize>: SpiNandBlocking<SPI, N> + ECC {
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
            let status = (self.read_register_cmd(spi, Self::ECC_STATUS_REGISTER)?
                >> Self::ECC_STATUS_BIT)
                & 0b11;
            match status {
                0b00 => Ok(ECCStatus::Ok),
                0b01 => Ok(ECCStatus::Corrected),
                0b10 => Ok(ECCStatus::Failed),
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

    pub trait ODSBlocking<SPI: SpiDevice, const N: usize>: ODS + SpiNandBlocking<SPI, N> {
        /// Set the output driver strength
        fn set_output_driver_strength(
            &self,
            spi: &mut SPI,
            strength: ODSStrength,
        ) -> Result<(), SpiFlashError<SPI::Error>> {
            self.write_register_cmd(spi, Self::ODS_REGISTER, (strength as u8) << Self::ODS_BIT)
        }

        /// Get the output driver strength
        fn get_output_driver_strength<SpiDevice>(
            &self,
            spi: &mut SPI,
        ) -> Result<ODSStrength, SpiFlashError<SPI::Error>> {
            let status = self.read_register_cmd(spi, Self::ODS_REGISTER)?;
            Ok(ODSStrength::from((status >> Self::ODS_BIT) & 0b11))
        }
    }

    pub trait BBMBlocking<SPI: SpiDevice, const N: usize>: BBM + SpiNandBlocking<SPI, N> {
        /// Check if the LUT is full
        fn is_lut_full(&self, spi: &mut SPI) -> Result<bool, SpiFlashError<SPI::Error>> {
            let status = self.read_register_cmd(spi, Self::LUT_FULL_REGISTER)?;
            Ok((status >> Self::LUT_FULL_BIT) & 0b1 == 1)
        }

        /// Read the lookup table
        fn read_lut_cmd(
            &self,
            spi: &mut SPI,
        ) -> Result<[(BlockIndex, BlockIndex); 10], SpiFlashError<SPI::Error>> {
            // 42 bytes, first byte is command, 2nd is dummy
            // 10 * u16 block pairs
            let mut buf = [0; 42];
            buf[0] = Self::READ_LUT_COMMAND;
            spi_transfer_in_place(spi, &mut buf)?;
            let mut lut: [(BlockIndex, BlockIndex); 10] = Default::default();
            for (i, chunk) in buf[2..].chunks_exact(4).enumerate() {
                let block = u16::from_be_bytes([chunk[0], chunk[1]]);
                let swap = u16::from_be_bytes([chunk[2], chunk[3]]);
                lut[i] = (BlockIndex::new(block), BlockIndex::new(swap));
            }
            Ok(lut)
        }

        /// Swap a block with the lookup table
        /// Logical is the bad block, physical is the good block it will be mapped to
        fn swap_block_cmd(
            &self,
            spi: &mut SPI,
            logical: BlockIndex,
            physical: BlockIndex,
        ) -> Result<(), SpiFlashError<SPI::Error>> {
            let mut buf = [Self::SWAP_BLOCK_COMMAND, 0, 0, 0, 0];
            buf[1..3].copy_from_slice(&logical.as_u16().to_be_bytes());
            buf[3..5].copy_from_slice(&physical.as_u16().to_be_bytes());
            spi_write(spi, &mut buf)?;
            Ok(())
        }
    }

    // Implement ECCBasicBlocking for ECCBasic devices
    impl<SPI: SpiDevice, const N: usize, T: ECCBasicBlocking<SPI, N>> ECCBasicBlocking<SPI, N> for T {}
    // Implement ECCBlocking for ECC devices
    impl<SPI: SpiDevice, const N: usize, T: ECC + SpiNandBlocking<SPI, N>> ECCBlocking<SPI, N> for T {}
    // Implement ODSBlocking for ODS devices
    impl<SPI: SpiDevice, const N: usize, T: ODS + SpiNandBlocking<SPI, N>> ODSBlocking<SPI, N> for T {}
    // Implement BBMBlocking for BBM devices
    impl<SPI: SpiDevice, const N: usize, T: BBM + SpiNandBlocking<SPI, N>> BBMBlocking<SPI, N> for T {}

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

#[cfg(test)]
mod tests {
    use super::blocking::BBMBlocking;
    use super::blocking::ECCBasicBlocking;
    use super::blocking::ODSBlocking;
    use super::W25N512G;

    #[test]
    fn features() {
        let device = W25N512G::new();
        // device.
        // device
        //     .set_output_driver_strength(spi, super::ODSStrength::Full)
        //     .unwrap();
    }
}
