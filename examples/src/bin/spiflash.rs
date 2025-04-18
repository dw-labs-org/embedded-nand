#![no_main]
#![no_std]

use cortex_m_semihosting::debug;
use defmt::dbg;
use embassy_executor::Spawner;

use embassy_stm32::gpio::Output;

use embedded_nand::{BlockIndex, PageIndex};
use spi_nand::cmd_blocking::SpiNandBlocking;
use spi_nand::{SpiNand, SpiNandDevice};
use spi_nand_devices::winbond::w25n::W25N02KV;

use {defmt_rtt as _, panic_probe as _}; // global logger

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = embassy_stm32::Config::default();

    let p = embassy_stm32::init(config);

    defmt::info!("Initialised peripherals");

    // Create an SPI instance that implements [embedded_hal::spi::SpiBus]
    let mut spi = embassy_stm32::spi::Spi::new(
        p.SPI2,
        p.PB13,
        p.PB15,
        p.PB14,
        p.GPDMA1_CH5,
        p.GPDMA1_CH4,
        embassy_stm32::spi::Config::default(),
    );

    // Get chip select pin
    let mut cs = Output::new(
        p.PB12,
        embassy_stm32::gpio::Level::High,
        embassy_stm32::gpio::Speed::High,
    );

    // Create exclusive access to the SPI bus as [embedded_hal::spi::SpiDevice]
    let spi_dev =
        embedded_hal_bus::spi::ExclusiveDevice::new(spi, cs, embedded_hal_bus::spi::NoDelay)
            .unwrap();

    // Create [spi_flash::device::SpiFlash] instance
    let device = W25N02KV::new();
    let b = <W25N02KV as SpiNand<2048>>::BLOCK_COUNT;

    let mut flash = SpiNandDevice::new(spi_dev, device);

    // Read the JEDEC ID
    dbg!(flash.reset_blocking());
    dbg!(flash.verify_jedec_blocking());
    // dbg!(flash.disable_block_protection().await);
    let ba = BlockIndex::new(0);
    let pa = PageIndex::from_block_address(ba, 64);

    embassy_time::Timer::after_secs(1).await;
    // // Mark a block as bad

    embassy_time::Timer::after_secs(1).await;
    defmt::info!("Checking bad blocks");
    // flash.mark_block_bad_blocking(1.into());

    for i in 0..2048 {
        if flash
            .device
            .block_marked_bad(&mut flash.spi, BlockIndex::new(i))
            .unwrap_or_else(|_| panic!("Failed to read block status"))
        {
            defmt::error!("Block {} is marked bad", i);
        }
    }
    defmt::info!("Checked bad blocks");
}
