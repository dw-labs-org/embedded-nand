#![no_std]

//! Winbond SPI NAND Flash driver
//!
//! This crate provides the implementation of the [spi-nand::SpiNand],
//! the [spi-nand::cmd_blocking::SpiNandBlocking] and the
//! [spi-nand::cmd_async::SpiNandAsync] traits for the Winbond W25N series of SPI NAND flash devices.
//!
//! To use, create an instance of [spi-nand::SpiNandDevice] using an SPI peripheral
//! and the [w25n::W25N] struct for the specific device.

pub mod w25n;
