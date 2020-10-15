#![allow(non_snake_case)]

macro_rules! deriveBorders {
    ($func_name: ident, $type: ty) => {
        /// Calculates all the neighbouring pixel values given:
        ///
        /// +---+---+---+---+
        /// |   |   | T |   |
        /// +---+---+---+---+
        /// |   |tl | t |tr |
        /// +---+---+---+---+
        /// | L | l | X |   |
        /// +---+---+---+---+
        ///
        /// where 'X' is the pixel at our current position, and borders are:
        ///
        /// +---+---+---+---+---+---+---+---+
        /// | 0 | 0 |   | 0 | 0 | 0 |   | 0 |
        /// +---+---+---+---+---+---+---+---+
        /// | 0 | 0 |   | 0 | 0 | 0 |   | 0 |
        /// +---+---+---+---+---+---+---+---+
        /// |   |   |   |   |   |   |   |   |
        /// +---+---+---+---+---+---+---+---+
        /// | 0 | 0 |   | a | b | c |   | c |
        /// +---+---+---+---+---+---+---+---+
        /// | 0 | a |   | d | e | f |   | f |
        /// +---+---+---+---+---+---+---+---+
        /// | 0 | d |   | g | h | i |   | i |
        /// +---+---+---+---+---+---+---+---+
        ///
        /// where 'a' through 'i' are pixel values in a plane.
        ///
        /// See: * 3.1. Border
        ///      * 3.2. Samples
        pub fn $func_name(
            plane: &[$type],
            x: isize,
            y: isize,
            width: isize,
            _height: isize,
            stride: isize,
        ) -> (isize, isize, isize, isize, isize, isize) {
            let mut T: isize = 0;
            let mut L: isize = 0;
            let mut t: isize = 0;
            let mut l: isize = 0;
            let mut tr: isize = 0;
            let mut tl: isize = 0;

            let stride = stride as usize;
            let pos = y as usize * stride + x as usize;

            // This is really slow and stupid but matches the spec exactly.
            // Each of the neighbouring values has been left entirely separate,
            // and none skipped, even if they could be.
            //
            // Please never implement an actual decoder this way.

            // T
            if y > 1 {
                T = plane[pos - (2 * stride)] as isize;
            }

            // L
            if y > 0 && x == 1 {
                L = plane[pos - stride - 1] as isize;
            } else if x > 1 {
                L = plane[pos - 2] as isize;
            }

            // t
            if y > 0 {
                t = plane[pos - stride] as isize;
            }

            // l
            if x > 0 {
                l = plane[pos - 1] as isize;
            } else if y > 0 {
                l = plane[pos - stride] as isize;
            }

            // tl
            if y > 1 && x == 0 {
                tl = plane[pos - (2 * stride)] as isize;
            } else if y > 0 && x > 0 {
                tl = plane[pos - stride - 1] as isize;
            }

            // tr
            if y > 0 {
                tr = plane[pos - stride + min(1, width - 1 - x) as usize]
                    as isize;
            }

            (T, L, t, l, tr, tl)
        }
    };
}

deriveBorders!(derive_borders, u8);
deriveBorders!(derive_borders16, u16);
deriveBorders!(derive_borders32, u32);

/// Given the neighbouring pixel values, calculate the context.
///
/// See: * 3.4. Context
///      * 3.5. Quantization Table Sets
pub fn get_context(
    quant_tables: &[[i16; 256]; 5],
    T: isize,
    L: isize,
    t: isize,
    l: isize,
    tr: isize,
    tl: isize,
) -> i32 {
    quant_tables[0][(l - tl) as usize & 255] as i32
        + quant_tables[1][(tl - t) as usize & 255] as i32
        + quant_tables[2][(t - tr) as usize & 255] as i32
        + quant_tables[3][(L - l) as usize & 255] as i32
        + quant_tables[4][(T - t) as usize & 255] as i32
}

fn min(a: isize, b: isize) -> isize {
    if a < b {
        a
    } else {
        b
    }
}

fn max(a: isize, b: isize) -> isize {
    if a > b {
        a
    } else {
        b
    }
}

/// Calculate the median value of 3 numbers
///
/// See: 2.2.5. Mathematical Functions
pub fn get_median(a: isize, b: isize, c: isize) -> isize {
    a + b + c - min(a, min(b, c)) - max(a, max(b, c))
}
