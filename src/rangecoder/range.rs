//! Implements a range coder as per  3.8.1. Range Coding Mode
//! of draft-ietf-cellar-ffv1.
//!
//! Cross-references are to
//! https://tools.ietf.org/id/draft-ietf-cellar-ffv1-17

use crate::rangecoder::tables::DEFAULT_STATE_TRANSITION;
use crate::rangecoder::util::min32;

/// RangeCoder is an instance of a range coder, as defined in:
///     Martin, G. Nigel N., "Range encoding: an algorithm for
///     removing redundancy from a digitised message.", July 1979.
pub struct RangeCoder<'a> {
    buf: &'a [u8],
    pos: isize,
    low: u16,
    rng: u16,
    #[allow(dead_code)]
    cur_byte: i32,
    zero_state: [u8; 256],
    one_state: [u8; 256],
}

impl<'a> RangeCoder<'a> {
    /// Creates a new range coder instance.
    ///
    /// See: 3.8.1. Range Coding Mode
    pub fn new(buf: &'a [u8]) -> Self {
        // Figure 15.
        let mut pos: isize = 2;
        // Figure 14.
        let mut low = (buf[0] as u16) << 8 | buf[1] as u16;
        // Figure 13.
        let rng = 0xFF00;

        if low >= rng {
            low = rng;
            pos = buf.len() as isize - 1;
        }

        let mut coder = Self {
            buf,
            pos,
            low,
            rng,
            cur_byte: -1,
            zero_state: [0; 256],
            one_state: [0; 256],
        };

        // 3.8.1.3. Initial Values for the Context Model
        coder.set_table(&DEFAULT_STATE_TRANSITION);
        coder
    }

    /// Refills the buffer.
    pub fn refill(&mut self) {
        // Figure 12.
        if self.rng < 0x100 {
            self.rng <<= 8;
            self.low <<= 8;
            if self.pos < self.buf.len() as isize {
                self.low += self.buf[self.pos as usize] as u16;
                self.pos += 1;
            }
        }
    }

    /// Gets the next boolean state.
    pub fn get(&mut self, state: &mut u8) -> bool {
        // Figure 10.
        let rangeoff = ((self.rng as u32 * *state as u32) >> 8) as u16;
        self.rng -= rangeoff;
        if self.low < self.rng {
            *state = self.zero_state[*state as usize];
            self.refill();
            false
        } else {
            self.low -= self.rng;
            *state = self.one_state[*state as usize];
            self.rng = rangeoff;
            self.refill();
            true
        }
    }

    /// Gets the next range coded unsigned scalar symbol.
    ///
    /// See: 4. Bitstream
    pub fn ur(&mut self, state: &mut [u8]) -> u32 {
        self.symbol(state, false) as u32
    }

    /// Gets the next range coded signed scalar symbol.
    ///
    /// See: 4. Bitstream
    pub fn sr(&mut self, state: &mut [u8]) -> i32 {
        self.symbol(state, true)
    }

    /// Gets the next range coded Boolean symbol.
    ///
    /// See: 4. Bitstream
    pub fn br(&mut self, state: &mut [u8]) -> bool {
        self.get(&mut state[0])
    }

    /// Gets the next range coded symbol.
    ///
    /// See: 3.8.1.2. Range Non Binary Values
    pub fn symbol(&mut self, state: &mut [u8], signed: bool) -> i32 {
        if self.get(&mut state[0]) {
            return 0;
        }

        let mut e: i32 = 0;
        while self.get(&mut state[1 + min32(e, 9) as usize]) {
            e += 1;
            if e > 31 {
                panic!("WTF range coder!");
            }
        }

        let mut a: u32 = 1;
        for i in (0..e).rev() {
            a *= 2;
            if self.get(&mut state[22 + min32(i, 9) as usize]) {
                a += 1;
            }
        }

        if signed && self.get(&mut state[11 + min32(e, 10) as usize]) {
            -(a as i32)
        } else {
            a as i32
        }
    }

    pub fn set_table(&mut self, table: &[u8; 256]) {
        // 3.8.1.4. State Transition Table

        // Figure 17.
        self.one_state[..256].clone_from_slice(&table[..256]);

        // Figure 18.
        for i in 1..255 {
            self.zero_state[i] = (256 - self.one_state[256 - i] as u16) as u8;
        }
    }

    /// Ends the current range coder.
    ///
    /// See: 3.8.1.1.1. Termination
    ///        * Sentinal Mode
    pub fn sentinal_end(&mut self) {
        let mut state: u8 = 129;
        self.get(&mut state);
    }

    /// Gets the current position in the bitstream.
    pub fn get_pos(&self) -> isize {
        if self.rng < 0x100 {
            return self.pos - 1;
        }
        self.pos
    }
}
