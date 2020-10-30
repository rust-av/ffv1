use crate::constants::CONTEXT_SIZE;
use crate::error::{Error, Result};
use crate::golomb::State;
use crate::range::RangeCoder;

#[derive(Clone, Default)]
pub struct InternalFrame {
    pub keyframe: bool,
    pub slice_info: Vec<SliceInfo>,
    pub slices: Vec<Slice>,
}

#[derive(Clone, Default, Copy)]
pub struct SliceInfo {
    pub(crate) pos: isize,
    pub(crate) size: u32,
    pub(crate) error_status: u8,
}

#[derive(Clone, Default)]
pub struct Slice {
    pub(crate) header: SliceHeader,
    pub(crate) start_x: u32,
    pub(crate) start_y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) state: Vec<Vec<Vec<u8>>>,
    pub(crate) golomb_state: Vec<Vec<State>>,
}

#[derive(Clone, Default)]
pub struct SliceHeader {
    pub(crate) slice_width_minus1: u32,
    pub(crate) slice_height_minus1: u32,
    pub(crate) slice_x: u32,
    pub(crate) slice_y: u32,
    pub(crate) quant_table_set_index: Vec<u8>,
    pub(crate) picture_structure: u8,
    pub(crate) sar_num: u32,
    pub(crate) sar_den: u32,
}

/// Determines whether a given frame is a keyframe.
///
/// See: 4.3. Frame
pub fn is_keyframe(buf: &[u8]) -> bool {
    // 4. Bitstream
    let mut state: [u8; CONTEXT_SIZE as usize] = [128; CONTEXT_SIZE as usize];

    let mut coder = RangeCoder::new(buf);

    coder.br(&mut state)
}

/// Counts the number of slices in a frame, as described in
/// 9.1.1. Multi-threading Support and Independence of Slices.
///
/// See: 4.8. Slice Footer
pub fn count_slices(buf: &[u8], ec: bool) -> Result<Vec<SliceInfo>> {
    let mut footer_size = 3;
    if ec {
        footer_size += 5;
    }

    // Go over the packet from the end to start, reading the footer,
    // so we can derive the slice positions within the packet, and
    // allow multithreading.
    let mut end_pos = buf.len() as isize;
    let mut slice_info = Vec::new();
    while end_pos > 0 {
        let mut info: SliceInfo = Default::default();

        // 4.8.1. slice_size
        let mut size = (buf[end_pos as usize - footer_size] as u32) << 16;
        size |= (buf[end_pos as usize - footer_size + 1] as u32) << 8;
        size |= buf[end_pos as usize - footer_size + 2] as u32;
        info.size = size;

        // 4.8.2. error_status
        info.error_status = buf[end_pos as usize - footer_size + 3] as u8;

        info.pos = end_pos - size as isize - footer_size as isize;
        let pos = info.pos;
        slice_info.push(info);
        end_pos = pos;
    }

    if end_pos < 0 {
        return Err(Error::SliceError("invalid slice footer".to_owned()));
    }

    // Preappend here
    slice_info.reverse();

    Ok(slice_info)
}
