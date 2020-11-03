//! Package golomb implements a Golomb-Rice coder as per
//! Section 3.8.2. Golomb Rice Mode of draft-ietf-cellar-ffv1.

use crate::golombcoder::bitreader::BitReader;
use crate::golombcoder::tables::LOG2_RUN;

/// Coder is an instance of a Golomb-Rice coder
/// as described in 3.8.2. Golomb Rice Mode.
pub struct Coder<'a> {
    r: BitReader<'a>,
    run_mode: isize,
    run_count: isize,
    run_index: isize,
    x: u32,
    w: u32,
}

/// State contains a single set of states for the a Golomb-Rice coder as
/// defined in 3.8.2.4.
///
/// Initial Values for the VLC context state.
#[derive(Debug, Clone)]
pub struct State {
    drift: i32,
    error_sum: i32,
    bias: i32,
    count: i32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            drift: 0,
            error_sum: 4,
            bias: 0,
            count: 1,
        }
    }
}

/// Simple sign extension.
pub fn sign_extend(n: i32, bits: usize) -> i32 {
    if bits == 8 {
        let ret = n as i8;
        ret as i32
    } else {
        let mut ret = n;
        ret <<= 32 - bits;
        ret >>= 32 - bits;
        ret
    }
}

impl<'a> Coder<'a> {
    /// Creates a new Golomb-Rice coder.
    pub fn new(buf: &'a [u8]) -> Self {
        let r = BitReader::new(buf);
        Self {
            r,
            run_mode: 0,
            run_count: 0,
            run_index: 0,
            x: 0,
            w: 0,
        }
    }

    /// newPlane should be called on a given Coder as each new Plane is
    /// processed. It resets the run index and sets the slice width.
    ///
    /// See: 3.8.2.2.1. Run Length Coding
    pub fn new_plane(&mut self, width: u32) {
        self.w = width;
        self.run_index = 0;
    }

    /// Starts a new run.
    pub fn new_run(&mut self) {
        self.run_mode = 0;
        self.run_count = 0;
    }

    /// newLine resets the x position and starts a new run,
    /// since runs can only be per-line.
    pub fn new_line(&mut self) {
        self.new_run();
        self.x = 0;
    }

    /// SG gets the next Golomb-Rice coded signed scalar symbol.
    ///
    /// See: * 3.8.2. Golomb Rice Mode
    ///      * 4. Bitstream
    pub fn sg(&mut self, context: i32, state: &mut State, bits: usize) -> i32 {
        // Section 3.8.2.2. Run Mode
        if context == 0 && self.run_mode == 0 {
            self.run_mode = 1;
        }

        // Section 3.8.2.2.1. Run Length Coding
        if self.run_mode != 0 {
            if self.run_count == 0 && self.run_mode == 1 {
                if self.r.u(1) == 1 {
                    self.run_count = 1 << LOG2_RUN[self.run_index as usize];
                    if self.x + self.run_count as u32 <= self.w {
                        self.run_index += 1;
                    }
                } else {
                    if LOG2_RUN[self.run_index as usize] != 0 {
                        self.run_count =
                            self.r.u(LOG2_RUN[self.run_index as usize] as u32)
                                as isize;
                    } else {
                        self.run_count = 0;
                    }
                    if self.run_index != 0 {
                        self.run_index -= 1;
                    }
                    // This is in the spec but how it works is... non-obvious.
                    self.run_mode = 2;
                }
            }

            self.run_count -= 1;
            // No more repeats; the run is over. Read a new symbol.
            if self.run_count < 0 {
                self.new_run();
                let mut diff = self.get_vlc_symbol(state, bits);
                // 3.8.2.2.2. Level Coding
                if diff >= 0 {
                    diff += 1;
                }
                self.x += 1;
                diff
            } else {
                // The run is still going; return a difference of zero.
                self.x += 1;
                0
            }
        } else {
            // We aren't in run mode; get a new symbol.
            self.x += 1;
            self.get_vlc_symbol(state, bits)
        }
    }

    /// Gets the next Golomb-Rice coded symbol.
    ///
    /// See: 3.8.2.3. Scalar Mode
    pub fn get_vlc_symbol(&mut self, state: &mut State, bits: usize) -> i32 {
        let mut i = state.count;
        let mut k = 0 as u32;

        while i < state.error_sum {
            k += 1;
            i += i;
        }

        let mut v = self.get_sr_golomb(k, bits);

        if 2 * state.drift < -state.count {
            v = -1 - v;
        }

        let ret = sign_extend(v + state.bias, bits);

        state.error_sum += v.abs();
        state.drift += v;

        if state.count == 128 {
            state.count >>= 1;
            state.drift >>= 1;
            state.error_sum >>= 1;
        }
        state.count += 1;
        if state.drift <= -state.count {
            state.bias = (state.bias - 1).max(-128);
            state.drift = (state.drift + state.count).max(-state.count + 1);
        } else if state.drift > 0 {
            state.bias = (state.bias + 1).min(127);
            state.drift = (state.drift - state.count).min(0);
        }

        ret
    }

    /// Gets the next signed Golomb-Rice code
    ///
    /// See: 3.8.2.1. Signed Golomb Rice Codes
    pub fn get_sr_golomb(&mut self, k: u32, bits: usize) -> i32 {
        let v = self.get_ur_golomb(k, bits);
        if v & 1 == 1 {
            -(v >> 1) - 1
        } else {
            v >> 1
        }
    }

    /// Gets the next unsigned Golomb-Rice code
    ///
    /// See: 3.8.2.1. Signed Golomb Rice Codes
    pub fn get_ur_golomb(&mut self, k: u32, bits: usize) -> i32 {
        for prefix in 0..12 {
            if self.r.u(1) == 1 {
                return self.r.u(k) as i32 + (prefix << k) as i32;
            }
        }
        self.r.u(bits as u32) as i32 + 11
    }
}
