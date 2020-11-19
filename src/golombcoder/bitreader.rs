pub struct BitReader<'a> {
    buf: &'a [u8],
    pos: usize,
    bit_buf: u32,
    bits_in_buf: u32,
}

impl<'a> BitReader<'a> {
    /// Creates a new bitreader.
    pub fn new(buf: &'a [u8]) -> Self {
        Self {
            buf,
            pos: 0,
            bit_buf: 0,
            bits_in_buf: 0,
        }
    }

    /// Reads 'count' bits, up to 32.
    pub fn u(&mut self, count: u32) -> u32 {
        if count > 32 {
            panic!("WTF more than 32 bits");
        }
        while count > self.bits_in_buf {
            self.bit_buf <<= 8;
            self.bit_buf |= self.buf[self.pos] as u32;
            self.bits_in_buf += 8;
            self.pos += 1;

            if self.bits_in_buf > 24 {
                if count <= self.bits_in_buf {
                    break;
                }
                if count <= 32 {
                    return self.u(16) << 16 | self.u(count - 16);
                }
            }
        }
        self.bits_in_buf -= count;
        (self.bit_buf >> self.bits_in_buf) & ((1 << count) - 1)
    }
}
