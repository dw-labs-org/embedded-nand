# TODO
- Fix ECC impls for 4 bit ECC devices and W25N04LW
- Add rest of ECC features
- Async impls
- Device types with SPI, macros

## Adding new features
- Add a marker trait in the w25n module with any config consts required
- Split feature into a number of traits if devices vary significantly (e.g ECC)
- impl marker for each device that has it
- Add blocking and asyc traits in associated module
- Impl blocking and async traits for marker

### Features to implement
- Continuous read mode
- Fast reads
- OTP pages / parameter pages
- Status register locking
- Write protection config
- detailed block protection
- User data configuration

