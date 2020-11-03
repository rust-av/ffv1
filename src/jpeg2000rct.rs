#![allow(non_snake_case)]

pub trait RCT<S>: Sized {
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
impl RCT<u16> for u8 {
    fn rct(
        dst: &mut [Vec<u8>],
        src: &[Vec<u16>],
        width: usize,
        height: usize,
        stride: usize,
        offset: usize,
        _bits: usize,
    ) {
        let Y = &src[0][offset as usize..];
        let Cb = &src[1][offset as usize..];
        let Cr = &src[2][offset as usize..];
        for y in 0..height as usize {
            for x in 0..width as usize {
                let Cbtmp = Cb[(y * stride as usize) + x] as i32 - (1 << 8); // Missing from spec
                let Crtmp = Cr[(y * stride as usize) + x] as i32 - (1 << 8); // Missing from spec
                let green = Y[(y * stride as usize) + x] as i32
                    - ((Cbtmp as i32 + Crtmp as i32) >> 2);
                let red = Crtmp as i32 + green;
                let blue = Cbtmp as i32 + green;
                dst[0][offset as usize + (y * stride as usize) + x] =
                    green as u8;
                dst[1][offset as usize + (y * stride as usize) + x] =
                    blue as u8;
                dst[2][offset as usize + (y * stride as usize) + x] =
                    red as u8;
            }
        }
        if src.len() == 4 {
            let s = &src[3][offset as usize..];
            let d = &mut dst[3][offset as usize..];
            for y in 0..height as usize {
                for x in 0..width as usize {
                    d[(y * stride as usize) + x] =
                        s[(y * stride as usize) + x] as u8;
                }
            }
        }
    }
}

/// Converts one line from 10 to 16 bit JPEG2000-RCT to planar GBR, in place.
///
/// See: 3.7.2. RGB
impl RCT<u8> for u16 {
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
        for y in 0..height as usize {
            for x in 0..width as usize {
                let Cbtmp = (src[1]
                    [offset as usize + (y * stride as usize) + x]
                    as i32
                    - 1)
                    << bits as i32; // Missing from spec
                let Crtmp = (src[2]
                    [offset as usize + (y * stride as usize) + x]
                    as i32
                    - 1)
                    << bits as i32; // Missing from spec
                let blue = src[0][offset as usize + (y * stride as usize) + x]
                    as i32
                    - ((Cbtmp as i32 + Crtmp as i32) >> 2);
                let red = Crtmp as i32 + blue;
                let green = Cbtmp as i32 + blue;
                src[0][offset as usize + (y * stride as usize) + x] =
                    green as u16;
                src[1][offset as usize + (y * stride as usize) + x] =
                    blue as u16;
                src[2][offset as usize + (y * stride as usize) + x] =
                    red as u16;
            }
        }
    }
}

/// Converts one line from 17-bit JPEG2000-RCT to planar GBR, in place.
///
/// See: 3.7.2. RGB
impl RCT<u32> for u16 {
    fn rct(
        dst: &mut [Vec<u16>],
        src: &[Vec<u32>],
        width: usize,
        height: usize,
        stride: usize,
        offset: usize,
        _bits: usize,
    ) {
        let Y = &src[0][offset as usize..];
        let Cb = &src[1][offset as usize..];
        let Cr = &src[2][offset as usize..];
        for y in 0..height as usize {
            for x in 0..width as usize {
                let Cbtmp = Cb[(y * stride as usize) + x] as i32 - (1 << 16); // Missing from spec
                let Crtmp = Cr[(y * stride as usize) + x] as i32 - (1 << 16); // Missing from spec
                let green = Y[(y * stride as usize) + x] as i32
                    - ((Cbtmp as i32 + Crtmp as i32) >> 2);
                let red = Crtmp as i32 + green;
                let blue = Cbtmp as i32 + green;
                dst[0][offset as usize + (y * stride as usize) + x] =
                    green as u16;
                dst[1][offset as usize + (y * stride as usize) + x] =
                    blue as u16;
                dst[2][offset as usize + (y * stride as usize) + x] =
                    red as u16;
            }
        }
        if src.len() == 4 {
            let s = &src[3][offset as usize..];
            let d = &mut dst[3][offset as usize..];
            for y in 0..height as usize {
                for x in 0..width as usize {
                    d[(y * stride as usize) + x] =
                        s[(y * stride as usize) + x] as u16;
                }
            }
        }
    }
}
