#![allow(non_snake_case)]

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
pub fn derive_borders<T: num_traits::AsPrimitive<isize>>(
    plane: &[T],
    x: usize,
    y: usize,
    width: usize,
    _height: usize,
    stride: usize,
) -> (isize, isize, isize, isize, isize, isize) {
    let mut T: isize = 0;
    let mut L: isize = 0;
    let mut t: isize = 0;
    let mut l: isize = 0;
    let mut tr: isize = 0;
    let mut tl: isize = 0;

    let pos = y * stride + x;

    // This is really slow and stupid but matches the spec exactly.
    // Each of the neighbouring values has been left entirely separate,
    // and none skipped, even if they could be.
    //
    // Please never implement an actual decoder this way.

    // T
    if y > 1 {
        T = plane[pos - (2 * stride)].as_();
    }

    // L
    if y > 0 && x == 1 {
        L = plane[pos - stride - 1].as_();
    } else if x > 1 {
        L = plane[pos - 2].as_();
    }

    // t
    if y > 0 {
        t = plane[pos - stride].as_();
    }

    // l
    if x > 0 {
        l = plane[pos - 1].as_();
    } else if y > 0 {
        l = plane[pos - stride].as_();
    }

    // tl
    if y > 1 && x == 0 {
        tl = plane[pos - (2 * stride)].as_();
    } else if y > 0 && x > 0 {
        tl = plane[pos - stride - 1].as_();
    }

    // tr
    if y > 0 {
        tr = plane[pos - stride + (width - 1 - x).min(1)].as_();
    }

    (T, L, t, l, tr, tl)
}

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

/// Calculate the median value of 3 numbers
///
/// See: 2.2.5. Mathematical Functions
pub fn get_median(a: isize, b: isize, c: isize) -> isize {
    a + b + c - a.min(b.min(c)) - a.max(b.max(c))
}
