// Run a bunch of tests on the W25N02KV flash chip
// Verify functionaility of implementation.
#![no_main]
#![no_std]

use cortex_m_semihosting::debug;
use defmt::{dbg, debug, info, trace};
use embassy_executor::Spawner;

use embassy_stm32::flash;
use embassy_stm32::gpio::Output;

use embassy_time::Timer;
use embedded_nand::NandFlash;
use embedded_nand::{BlockIndex, PageIndex};
use spi_nand::cmd_blocking::SpiNandBlocking;
use spi_nand::{SpiNand, SpiNandDevice};
use spi_nand_devices::winbond::w25n::blocking::{ECCBlocking, ODSBlocking};
use spi_nand_devices::winbond::w25n::{ECCThreshold, ODSStrength, W25N02KV};

use {defmt_rtt as _, panic_probe as _}; // global logger

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let config = embassy_stm32::Config::default();

    let p = embassy_stm32::init(config);

    defmt::info!("Initialised peripherals");

    // Create an SPI instance that implements [embedded_hal::spi::SpiBus]
    let spi = embassy_stm32::spi::Spi::new(
        p.SPI2,
        p.PB13,
        p.PB15,
        p.PB14,
        p.GPDMA1_CH5,
        p.GPDMA1_CH4,
        embassy_stm32::spi::Config::default(),
    );

    // Get chip select pin
    let cs = Output::new(
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

    let mut flash = SpiNandDevice::new(spi_dev, device);

    // =========== TESTING =========================
    // Reset the device before continuing
    flash.reset_blocking().unwrap();
    // Wait
    Timer::after_secs(1).await;
    // Verify the JEDEC ID
    info!("Checking JEDEC ID");
    assert!(flash.verify_jedec_blocking().unwrap());

    // Check the registers are in default state
    info!("Checking registers");
    assert_eq!(
        flash
            .device
            .read_register_cmd(&mut flash.spi, 0xA0)
            .unwrap(),
        0b01111100
    );
    assert_eq!(
        flash
            .device
            .read_register_cmd(&mut flash.spi, 0xB0)
            .unwrap(),
        0b00011001
    );
    assert_eq!(
        flash
            .device
            .read_register_cmd(&mut flash.spi, 0xC0)
            .unwrap(),
        0b00000000
    );

    // Check writing to registers
    flash
        .device
        .disable_block_protection(&mut flash.spi)
        .unwrap();
    assert_eq!(
        flash
            .device
            .read_register_cmd(&mut flash.spi, 0xA0)
            .unwrap(),
        0b100
    );

    // Set driver strength
    info!("Checking driver strength");
    for strength in 0..=3 {
        let strength = ODSStrength::from(strength);
        flash
            .device
            .set_output_driver_strength(&mut flash.spi, strength)
            .unwrap();
        assert_eq!(
            flash
                .device
                .get_output_driver_strength(&mut flash.spi)
                .unwrap(),
            strength
        );
    }

    // ECC theshold setting
    info!("Checking ECC threshold");
    for threshold in 1..=7 {
        let threshold = ECCThreshold::from(threshold);
        flash
            .device
            .ecc_set_bit_flip_threshold(&mut flash.spi, threshold)
            .unwrap();
        assert_eq!(
            flash.device.ecc_bit_flip_threshold(&mut flash.spi).unwrap(),
            threshold
        );
    }
    info!("Done")
}
