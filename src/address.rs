/// For addressing bytes and byte ranges
/// B is the number of bytes in a block
/// P is the number of bytes in a page
pub struct ByteAddress(u64);

impl ByteAddress {
    pub fn new(value: u64) -> Self {
        ByteAddress(value)
    }
}

impl From<u64> for ByteAddress {
    fn from(value: u64) -> Self {
        ByteAddress(value)
    }
}

impl From<u32> for ByteAddress {
    fn from(value: u32) -> Self {
        ByteAddress(value as u64)
    }
}

impl From<usize> for ByteAddress {
    fn from(value: usize) -> Self {
        ByteAddress(value as u64)
    }
}

impl From<ByteAddress> for u64 {
    fn from(value: ByteAddress) -> Self {
        value.0
    }
}

impl From<ByteAddress> for BlockAddress {
    fn from(value: ByteAddress) -> Self {
        BlockAddress((value.0 / B as u64) as u32)
    }
}

impl<const B: u32> From<ByteAddress<B>> for PageAddress {
    fn from(value: ByteAddress<B, P>) -> Self {
        PageAddress((value.0 / P as u64) as u32)
    }
}

/// For addressing blocks and block ranges
pub struct BlockAddress(u32);

/// For addressing pages and page ranges
pub struct PageAddress(u32);
