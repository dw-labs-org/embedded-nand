#![no_main]
#![no_std]

use cortex_m::asm::wfi;
use cortex_m::register::apsr::read;
use cortex_m_semihosting::debug;
use defmt::dbg;
use embassy_executor::Spawner;
use embassy_stm32::lptim::timer::Timer;
use embassy_stm32::{can::BufferedCan, gpio::Output};
use spi_flash::address::{BlockAddress, PageAddress};
use spi_flash::blocking::SpiNandBlocking;
use spi_flash::{
    SpiNand,
    device::{self, SpiFlash},
};
use winbond::w25n::W25N02K;
use {defmt_rtt as _, panic_probe as _}; // global logger

// same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}

/// Terminates the application and makes a semihosting-capable debug tool exit
/// with status code 0.
pub fn exit() -> ! {
    loop {
        debug::exit(debug::EXIT_SUCCESS);
    }
}

/// Hardfault handler.
///
/// Terminates the application and makes a semihosting-capable debug tool exit
/// with an error. This seems better than the default, which is to spin in a
/// loop.
#[cortex_m_rt::exception]
unsafe fn HardFault(_frame: &cortex_m_rt::ExceptionFrame) -> ! {
    loop {
        debug::exit(debug::EXIT_FAILURE);
    }
}

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
    let device = winbond::w25n::W25N02K::new();
    let b = <W25N02K as SpiNand<2048>>::BLOCK_COUNT;

    let mut flash = spi_flash::device::SpiFlash::new(spi_dev, device);

    // Read the JEDEC ID
    dbg!(flash.reset_blocking());
    dbg!(flash.jedec_blocking());
    dbg!(flash.disable_block_protection().await);

    // initialise the flashmap with 2000 logical blocks (46 spare, 2 for map)
    let mut flashmap = flashmap::FlashMap::<_, 2000>::init(flash).unwrap_or_else(|_| panic!());
}
