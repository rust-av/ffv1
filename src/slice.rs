use crate::constants::CONTEXT_SIZE;
use crate::error::Result;
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
    pub(crate) pos: usize,
    pub(crate) size: usize,
    pub(crate) error_status: u8,
}

#[derive(Clone, Default)]
pub struct Slice {
    pub(crate) header: SliceHeader,
    pub(crate) state: Vec<Vec<Vec<u8>>>,
    pub(crate) golomb_state: Vec<Vec<State>>,
    pub(crate) planes: Vec<SlicePlane>,
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

#[derive(Clone)]
pub struct SlicePlane {
    #[allow(dead_code)]
    pub(crate) start_x: u32,
    #[allow(dead_code)]
    pub(crate) start_y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) stride: u32,
    pub(crate) offset: usize,
    pub(crate) quant: u8,
}

/// Determines whether a given frame is a keyframe.
///
/// See: 4.3. Frame
pub fn is_keyframe(buf: &[u8]) -> bool {
    // 4. Bitstream
    let mut state: [u8; CONTEXT_SIZE] = [128; CONTEXT_SIZE];

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
    let mut end_pos = buf.len();
    let mut slice_info = Vec::new();
    while end_pos > 0 {
        let mut info: SliceInfo = Default::default();

        // 4.8.1. slice_size
        let mut size = (buf[end_pos - footer_size] as u32) << 16;
        size |= (buf[end_pos - footer_size + 1] as u32) << 8;
        size |= buf[end_pos - footer_size + 2] as u32;
        info.size = size as usize;

        // 4.8.2. error_status
        info.error_status = buf[end_pos - footer_size + 3] as u8;

        let pos = end_pos - info.size - footer_size;
        info.pos = pos;
        slice_info.push(info);
        end_pos = pos;
    }

    // Preappend here
    slice_info.reverse();

    Ok(slice_info)
}
