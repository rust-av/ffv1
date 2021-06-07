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
pub fn derive_borders<T: num_traits::AsPrimitive<usize>>(
    plane: &[T],
    x: usize,
    y: usize,
    width: usize,
    _height: usize,
    stride: usize,
) -> (usize, usize, usize, usize, usize, usize) {
    let pos = y * stride + x;

    // This is really slow and stupid but matches the spec exactly.
    // Each of the neighbouring values has been left entirely separate,
    // and none skipped, even if they could be.
    //
    // Please never implement an actual decoder this way.

    // T
    let T = if y > 1 {
        plane[pos - (2 * stride)].as_()
    } else {
        0
    };

    // L
    let L = if y > 0 && x == 1 {
        plane[pos - stride - 1].as_()
    } else if x > 1 {
        plane[pos - 2].as_()
    } else {
        0
    };

    // t
    let t = if y > 0 { plane[pos - stride].as_() } else { 0 };

    // l
    let l = if x > 0 {
        plane[pos - 1].as_()
    } else if y > 0 {
        plane[pos - stride].as_()
    } else {
        0
    };

    // tl
    let tl = if y > 1 && x == 0 {
        plane[pos - (2 * stride)].as_()
    } else if y > 0 && x > 0 {
        plane[pos - stride - 1].as_()
    } else {
        0
    };

    // tr
    let tr = if y > 0 {
        plane[pos - stride + (width - 1 - x).min(1)].as_()
    } else {
        0
    };

    (T, L, t, l, tr, tl)
}

/// Given the neighbouring pixel values, calculate the context.
///
/// See: * 3.4. Context
///      * 3.5. Quantization Table Sets
pub fn get_context(
    quant_tables: &[[i16; 256]; 5],
    T: usize,
    L: usize,
    t: usize,
    l: usize,
    tr: usize,
    tl: usize,
) -> i32 {
    quant_tables[0][l.wrapping_sub(tl) & 255] as i32
        + quant_tables[1][tl.wrapping_sub(t) & 255] as i32
        + quant_tables[2][t.wrapping_sub(tr) & 255] as i32
        + quant_tables[3][L.wrapping_sub(l) & 255] as i32
        + quant_tables[4][T.wrapping_sub(t) & 255] as i32
}

/// Calculate the median value of 3 numbers
///
/// See: 2.2.5. Mathematical Functions
pub fn get_median(a: i32, b: i32, c: i32) -> i32 {
    a + b + c - a.min(b.min(c)) - a.max(b.max(c))
}
