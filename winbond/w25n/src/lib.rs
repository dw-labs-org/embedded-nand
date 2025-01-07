#![no_std]

use embedded_hal::spi::SpiDevice;
use embedded_nand::genericspi::{
    spi_transfer, spi_transfer_in_place, FlashRead, FlashWrite, JedecID,
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
    pub fn read_status_register_2<SPI: SpiDevice>(
        &self,
        spi: &mut SPI,
    ) -> Result<u8, embedded_nand::genericspi::SpiFlashError<SPI>> {
        let mut buf = [
            <W25N<N> as FlashRead<SPI>>::STATUS_REGISTER_READ_COMMAND,
            0xB0,
            0,
        ];
        spi_transfer_in_place(spi, &mut buf)?;
        Ok(buf[2])
    }

    pub fn read_status_register_3<SPI: SpiDevice>(
        &self,
        spi: &mut SPI,
    ) -> Result<u8, embedded_nand::genericspi::SpiFlashError<SPI>> {
        let mut buf = [
            <W25N<N> as FlashRead<SPI>>::STATUS_REGISTER_READ_COMMAND,
            0xC0,
            0,
        ];
        spi_transfer_in_place(spi, &mut buf)?;
        Ok(buf[2])
    }
}

impl<const N: u32> Default for W25N<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<SPI: SpiDevice, const N: u32> FlashRead<SPI> for W25N<N> {
    const PAGE_SIZE: u32 = 2048;
    const PAGES_PER_BLOCK: u32 = 64;
    const BLOCK_COUNT: u32 = N;

    fn read_status_register(
        &self,
        spi: &mut SPI,
    ) -> Result<u8, embedded_nand::genericspi::SpiFlashError<SPI>> {
        let mut buf = [
            <W25N<N> as FlashRead<SPI>>::STATUS_REGISTER_READ_COMMAND,
            0xA0,
            0,
        ];
        spi_transfer_in_place(spi, &mut buf)?;
        Ok(buf[2])
    }

    fn read_jedec_id(
        &self,
        spi: &mut SPI,
    ) -> Result<embedded_nand::genericspi::JedecID, embedded_nand::genericspi::SpiFlashError<SPI>>
    {
        let mut buf = [0; 3];
        spi_transfer(
            spi,
            &mut buf,
            &[<W25N<N> as FlashRead<SPI>>::JEDEC_COMMAND, 0, 0],
        )?;
        Ok(JedecID::new(buf[2], 1))
    }

    fn is_busy(
        &self,
        spi: &mut SPI,
    ) -> Result<bool, embedded_nand::genericspi::SpiFlashError<SPI>> {
        let status = self.read_status_register_3(spi)?;
        Ok((status & 0x01) != 0)
    }
}

impl<SPI: SpiDevice, const N: u32> FlashWrite<SPI> for W25N<N> {
    fn is_write_enabled(
        &self,
        spi: &mut SPI,
    ) -> Result<bool, embedded_nand::genericspi::SpiFlashError<SPI>> {
        let status = self.read_status_register_3(spi)?;
        Ok((status & 0x02) != 0)
    }
}
