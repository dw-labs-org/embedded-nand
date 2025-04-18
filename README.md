This repo contains:

- **embeddded-nand**: An attempt to create a NAND equialent of the NOR traits in [embedded-storage](https://github.com/rust-embedded-community/embedded-storage). Probably a bit more complicated than required at the moment. Contains helpers for converting between byte addresses, block addresses and page addresses and for iterating over blocks and pages.
- **embedded-nand-async**: Async version of the above
- **spi-nand**: A generic driver for SPI NAND flash chips. Implements the `embedded-nand` and `embedded-nand-async` traits. Adding support for most devices should be trivial
- **spi-nand-devices**: Device support crate that enables the used of the `spi-nand` device type for specific devices. Currently supports the winbond W25N range. Also supports device specific features outside the scope of the generic `spi-nand` device.
- **flashmap**: A simple read focussed flash translation layer that targets the `embedded-nand` and `embedded-nand-async` traits. Maps logical blocks to physical blocks, remapping bad blocks when a read/write/erase fails.