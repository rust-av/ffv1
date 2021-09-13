#![allow(non_snake_case)]

pub trait Rct<S>: Sized {
    fn rct(
        dst: &mut [Vec<Self>],
        src: &[Vec<S>],
        width: usize,
        height: usize,
        stride: usize,
        offset: usize,
        bits: usize,
    );
}

/// Converts one line from 9-bit JPEG2000-RCT to planar GBR.
///
/// See: 3.7.2. RGB
impl Rct<u16> for u8 {
    fn rct(
        dst: &mut [Vec<u8>],
        src: &[Vec<u16>],
        width: usize,
        height: usize,
        stride: usize,
        offset: usize,
        _bits: usize,
    ) {
        let Y = &src[0][offset..];
        let Cb = &src[1][offset..];
        let Cr = &src[2][offset..];
        for y in 0..height {
            for x in 0..width {
                let Cbtmp = Cb[(y * stride) + x].wrapping_sub(1 << 8); // See: 3.7.2.1. RGB
                let Crtmp = Cr[(y * stride) + x].wrapping_sub(1 << 8); // See: 3.7.2.1. RGB
                let green = Y[(y * stride) + x]
                    .wrapping_sub((Cbtmp.wrapping_add(Crtmp)) >> 2); // See: 3.7.2.1. RGB
                let red = Crtmp.wrapping_add(green); // See: 3.7.2.1 RGB
                let blue = Cbtmp.wrapping_add(green); // See: 3.7.2.1 RGB
                dst[0][offset + (y * stride) + x] = green as u8;
                dst[1][offset + (y * stride) + x] = blue as u8;
                dst[2][offset + (y * stride) + x] = red as u8;
            }
        }
        if src.len() == 4 {
            let s = &src[3][offset..];
            let d = &mut dst[3][offset..];
            for y in 0..height {
                for x in 0..width {
                    d[(y * stride) + x] = s[(y * stride) + x] as u8;
                }
            }
        }
    }
}

/// Converts one line from 10 to 16 bit JPEG2000-RCT to planar GBR, in place.
///
/// See: 3.7.2. RGB
impl Rct<u8> for u16 {
    fn rct(
        dst: &mut [Vec<u16>],
        _src: &[Vec<u8>],
        width: usize,
        height: usize,
        stride: usize,
        offset: usize,
        bits: usize,
    ) {
        let src = dst;
        for y in 0..height {
            for x in 0..width {
                let Cbtmp = (src[1][offset + (y * stride) + x]
                    .wrapping_sub(1))
                    << bits; // See: 3.7.2.1. RGB
                let Crtmp = (src[2][offset + (y * stride) + x]
                    .wrapping_sub(1))
                    << bits; // See: 3.7.2.1. RGB
                let blue = src[0][offset + (y * stride) + x]
                    .wrapping_sub((Cbtmp + Crtmp) >> 2); // See: 3.7.2.1. RGB
                let red = Crtmp.wrapping_add(blue);
                let green = Cbtmp.wrapping_add(blue);
                src[0][offset + (y * stride) + x] = green as u16;
                src[1][offset + (y * stride) + x] = blue as u16;
                src[2][offset + (y * stride) + x] = red as u16;
            }
        }
    }
}

/// Converts one line from 17-bit JPEG2000-RCT to planar GBR, in place.
///
/// See: 3.7.2. RGB
impl Rct<u32> for u16 {
    fn rct(
        dst: &mut [Vec<u16>],
        src: &[Vec<u32>],
        width: usize,
        height: usize,
        stride: usize,
        offset: usize,
        _bits: usize,
    ) {
        let Y = &src[0][offset..];
        let Cb = &src[1][offset..];
        let Cr = &src[2][offset..];
        for y in 0..height {
            for x in 0..width {
                let Cbtmp = Cb[(y * stride) + x].wrapping_sub(1 << 16); // See: 3.7.2.1. RGB
                let Crtmp = Cr[(y * stride) + x].wrapping_sub(1 << 16); // See: 3.7.2.1. RGB
                let green = Y[(y * stride) + x]
                    .wrapping_sub((Cbtmp.wrapping_add(Crtmp)) >> 2); // See: 3.7.2.1. RGB
                let red = Crtmp.wrapping_add(green); // See: 3.7.2.1. RGB
                let blue = Cbtmp.wrapping_add(green); // See: 3.7.2.1. RGB
                dst[0][offset + (y * stride) + x] = green as u16;
                dst[1][offset + (y * stride) + x] = blue as u16;
                dst[2][offset + (y * stride) + x] = red as u16;
            }
        }
        if src.len() == 4 {
            let s = &src[3][offset..];
            let d = &mut dst[3][offset..];
            for y in 0..height {
                for x in 0..width {
                    d[(y * stride) + x] = s[(y * stride) + x] as u16;
                }
            }
        }
    }
}
