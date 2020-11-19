use num_traits::AsPrimitive;

use crate::constants::CONTEXT_SIZE;
use crate::crc32mpeg2::crc32_mpeg2;
use crate::error::{Error, Result};
use crate::golomb::Coder as GolombCoder;
use crate::golomb::State;
use crate::jpeg2000rct::RCT;
use crate::pred::{derive_borders, get_context, get_median};
use crate::range::RangeCoder;
use crate::rangecoder::tables::DEFAULT_STATE_TRANSITION;
use crate::record::ConfigRecord;
use crate::slice::{
    count_slices, is_keyframe, InternalFrame, Slice, SliceHeader, SlicePlane,
};

#[allow(clippy::large_enum_variant)]
enum Coder<'a> {
    Golomb(GolombCoder<'a>),
    Range(RangeCoder<'a>),
}

/// Frame contains a decoded FFV1 frame and relevant
/// data about the frame.
///
/// If BitDepth is 8, image data is in Buf. If it is anything else,
/// image data is in Buf16.
///
/// Image data consists of up to four contiguous planes, as follows:
///   - If ColorSpace is YCbCr:
///     - Plane 0 is Luma (always present)
///     - If HasChroma is true, the next two planes are Cr and Cr, subsampled by
///       ChromaSubsampleV and ChromaSubsampleH.
///     - If HasAlpha is true, the next plane is alpha.
///  - If ColorSpace is RGB:
///    - Plane 0 is Green
///    - Plane 1 is Blue
///    - Plane 2 is Red
///    - If HasAlpha is true, plane 4 is alpha.
pub struct Frame {
    /// Image data. Valid only when BitDepth is 8.
    pub buf: Vec<Vec<u8>>,
    /// Image data. Valid only when BitDepth is greater than 8.
    pub buf16: Vec<Vec<u16>>,
    /// Unexported 32-bit scratch buffer for 16-bit JPEG2000-RCT RGB
    pub buf32: Vec<Vec<u32>>,
    /// Width of the frame, in pixels.
    #[allow(dead_code)]
    pub width: u32,
    /// Height of the frame, in pixels.
    #[allow(dead_code)]
    pub height: u32,
    /// Bitdepth of the frame (8-16).
    #[allow(dead_code)]
    pub bit_depth: u8,
    /// Colorspace of the frame. See the colorspace constants.
    #[allow(dead_code)]
    pub color_space: isize,
    /// Whether or not chroma planes are present.
    #[allow(dead_code)]
    pub has_chroma: bool,
    /// Whether or not an alpha plane is present.
    #[allow(dead_code)]
    pub has_alpha: bool,
    /// The log2 vertical chroma subampling value.
    #[allow(dead_code)]
    pub chroma_subsample_v: u8,
    /// The log2 horizontal chroma subsampling value.
    #[allow(dead_code)]
    pub chroma_subsample_h: u8,
}

/// Decoder is a FFV1 decoder instance.
pub struct Decoder {
    record: ConfigRecord,
    state_transition: [u8; 256],
    current_frame: InternalFrame,
}

impl Decoder {
    /// NewDecoder creates a new FFV1 decoder instance.
    ///
    /// 'record' is the codec private data provided by the container. For
    /// Matroska, this is what is in CodecPrivate (adjusted for e.g. VFW
    /// data that may be before it). For ISOBMFF, this is the 'glbl' box.
    ///
    /// 'width' and 'height' are the frame width and height provided by
    /// the container.
    pub fn new(record: &[u8], width: u32, height: u32) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(Error::InvalidInputData(format!(
                "invalid dimensions: {}x{}",
                width, height
            )));
        }

        if record.is_empty() {
            return Err(Error::InvalidInputData(
                "invalid record with length zero".to_owned(),
            ));
        }

        let record =
            match ConfigRecord::parse_config_record(&record, width, height) {
                Ok(record) => record,
                Err(err) => {
                    return Err(Error::InvalidInputData(format!(
                        "invalid v3 configuration record: {}",
                        err
                    )))
                }
            };

        let mut decoder = Decoder {
            record,
            state_transition: [0; 256],
            current_frame: InternalFrame {
                keyframe: false,
                slice_info: Vec::new(),
                slices: Vec::new(),
            },
        };

        decoder.initialize_states();

        Ok(decoder)
    }

    /// DecodeFrame takes a packet and decodes it to a ffv1.Frame.
    ///
    /// Slice threading is used by default, with one goroutine per
    /// slice.
    pub fn decode_frame(&mut self, frame_input: &[u8]) -> Result<Frame> {
        let mut frame = Frame {
            buf: Vec::new(),
            buf16: Vec::new(),
            buf32: Vec::new(),
            width: self.record.width,
            height: self.record.height,
            bit_depth: self.record.bits_per_raw_sample,
            color_space: self.record.colorspace_type as isize,
            has_chroma: self.record.chroma_planes,
            has_alpha: self.record.extra_plane,
            chroma_subsample_v: if self.record.chroma_planes {
                self.record.log2_v_chroma_subsample
            } else {
                0
            },
            chroma_subsample_h: if self.record.chroma_planes {
                self.record.log2_h_chroma_subsample
            } else {
                0
            },
        };

        let mut num_planes = 1;
        if self.record.chroma_planes {
            num_planes += 2;
        }
        if self.record.extra_plane {
            num_planes += 1;
        }

        let full_size = (self.record.width * self.record.height) as usize;
        let chroma_width =
            self.record.width >> self.record.log2_h_chroma_subsample;
        let chroma_height =
            self.record.height >> self.record.log2_v_chroma_subsample;
        let chroma_size = (chroma_width * chroma_height) as usize;

        // Hideous and temporary.
        if self.record.bits_per_raw_sample == 8 {
            frame.buf = vec![Vec::new(); num_planes];
            frame.buf[0] = vec![0; full_size];
            if self.record.chroma_planes {
                frame.buf[1] = vec![0; chroma_size];
                frame.buf[2] = vec![0; chroma_size];
            }
            if self.record.extra_plane {
                frame.buf[3] = vec![0; full_size];
            }
        }

        // We allocate *both* if it's 8bit RGB since I'm a terrible person and
        // I wanted to use it as a scratch space, since JPEG2000-RCT is very
        // annoyingly coded as n+1 bits, and I wanted the implementation
        // to be straightforward... RIP.
        if self.record.bits_per_raw_sample > 8
            || self.record.colorspace_type == 1
        {
            frame.buf16 = vec![Vec::new(); num_planes];
            frame.buf16[0] = vec![0; full_size];
            if self.record.chroma_planes {
                frame.buf16[1] = vec![0; chroma_size];
                frame.buf16[2] = vec![0; chroma_size];
            }
            if self.record.extra_plane {
                frame.buf16[3] = vec![0; full_size];
            }
        }

        // For 16-bit RGB we need a 32-bit scratch space beause we need to predict
        // based on 17-bit values in the JPEG2000-RCT space, so just allocate a
        // whole frame, because I am lazy. Is it slow? Yes.
        if self.record.bits_per_raw_sample == 16
            && self.record.colorspace_type == 1
        {
            frame.buf32 = vec![Vec::new(); num_planes];
            frame.buf32[0] = vec![0; full_size];
            frame.buf32[1] = vec![0; full_size];
            frame.buf32[2] = vec![0; full_size];
            if self.record.extra_plane {
                frame.buf32[3] = vec![0; full_size];
            }
        }

        // We parse the frame's keyframe info outside the slice decoding
        // loop so we know ahead of time if each slice has to refresh its
        // states or not. This allows easy slice threading.
        self.current_frame.keyframe = is_keyframe(frame_input);

        // We parse all the footers ahead of time too, for the same reason.
        // It allows us to know all the slice positions and sizes.
        //
        // See: 9.1.1. Multi-threading Support and Independence of Slices
        let err = self.parse_footers(frame_input);
        if let Err(err) = err {
            return Err(Error::FrameError(format!(
                "invalid frame footer: {}",
                err
            )));
        }

        // Slice threading lazymode (not using sync for now, only sequential code,
        // FIXME there could be errors here)
        for i in 0..self.current_frame.slices.len() {
            let err = self.decode_slice(frame_input, i, &mut frame);
            if let Err(err) = err {
                return Err(Error::SliceError(format!(
                    "slice {} failed: {}",
                    i, err
                )));
            }
        }

        // Delete the scratch buffer, if needed, as per above.
        if self.record.bits_per_raw_sample == 8
            && self.record.colorspace_type == 1
        {
            frame.buf16 = Vec::new();
        }

        // We'll never need this again.
        frame.buf32 = Vec::new();

        Ok(frame)
    }

    /// Initializes initial state for the range coder.
    ///
    /// See: 4.1.15. initial_state_delta
    fn initialize_states(&mut self) {
        for (i, default_state_transition) in
            DEFAULT_STATE_TRANSITION.iter().enumerate().skip(1)
        {
            self.state_transition[i] = (*default_state_transition as i16
                + self.record.state_transition_delta[i])
                as u8;
        }
    }

    /// Parses all footers in a frame and allocates any necessary slice structures.
    ///
    /// See: * 9.1.1. Multi-threading Support and Independence of Slices
    ///      * 3.8.1.3. Initial Values for the Context Model
    ///      * 3.8.2.4. Initial Values for the VLC context state
    fn parse_footers(&mut self, buf: &[u8]) -> Result<()> {
        let slice_info = count_slices(buf, self.record.ec != 0)?;
        self.current_frame.slice_info = slice_info;

        let mut slices: Vec<Slice> =
            vec![Default::default(); self.current_frame.slice_info.len()];

        if !self.current_frame.keyframe {
            if slices.len() != self.current_frame.slices.len() {
                return Err(Error::SliceError("inter frames must have the same number of slices as the preceding intra frame".to_owned()));
            }
            for (next, current) in
                slices.iter_mut().zip(self.current_frame.slices.iter())
            {
                next.state = current.state.clone();
            }

            if self.record.coder_type == 0 {
                for (next, current) in
                    slices.iter_mut().zip(self.current_frame.slices.iter())
                {
                    next.golomb_state = current.golomb_state.clone();
                }
            }
        }

        self.current_frame.slices = slices;

        Ok(())
    }

    /// Parses a slice's header.
    ///
    /// See: 4.5. Slice Header
    fn parse_slice_header(
        current_slice: &mut Slice,
        record: &ConfigRecord,
        coder: &mut RangeCoder,
    ) {
        // 4. Bitstream
        let mut slice_state: [u8; CONTEXT_SIZE] = [128; CONTEXT_SIZE];

        // 4.5.1. slice_x
        current_slice.header.slice_x = coder.ur(&mut slice_state);
        // 4.5.2. slice_y
        current_slice.header.slice_y = coder.ur(&mut slice_state);
        // 4.5.3 slice_width
        current_slice.header.slice_width_minus1 = coder.ur(&mut slice_state);
        // 4.5.4 slice_height
        current_slice.header.slice_height_minus1 = coder.ur(&mut slice_state);

        // 4.5.5. quant_table_set_index_count
        let mut quant_table_set_index_count = 1;
        if record.chroma_planes {
            quant_table_set_index_count += 1;
        }
        if record.extra_plane {
            quant_table_set_index_count += 1;
        }

        // 4.5.6. quant_table_set_index
        current_slice.header.quant_table_set_index =
            vec![0; quant_table_set_index_count];
        for i in 0..quant_table_set_index_count {
            current_slice.header.quant_table_set_index[i] =
                coder.ur(&mut slice_state) as u8;
        }

        // 4.5.7. picture_structure
        current_slice.header.picture_structure =
            coder.ur(&mut slice_state) as u8;

        // It's really weird for slices within the same frame to code
        // their own SAR values...
        //
        // See: * 4.5.8. sar_num
        //      * 4.5.9. sar_den
        current_slice.header.sar_num = coder.ur(&mut slice_state);
        current_slice.header.sar_den = coder.ur(&mut slice_state);

        // Calculate boundaries for easy use elsewhere
        //
        // See: * 4.6.3. slice_pixel_height
        //      * 4.6.4. slice_pixel_y
        //      * 4.7.2. slice_pixel_width
        //      * 4.7.3. slice_pixel_x
        let start_x = current_slice.header.slice_x * record.width
            / (record.num_h_slices_minus1 as u32 + 1);
        let start_y = current_slice.header.slice_y * record.height
            / (record.num_v_slices_minus1 as u32 + 1);
        let width = ((current_slice.header.slice_x
            + current_slice.header.slice_width_minus1
            + 1)
            * record.width
            / (record.num_h_slices_minus1 as u32 + 1))
            - start_x;
        let height = ((current_slice.header.slice_y
            + current_slice.header.slice_height_minus1
            + 1)
            * record.height
            / (record.num_v_slices_minus1 as u32 + 1))
            - start_y;

        let stride = record.width;
        let offset = start_x + start_y * stride;

        // Calculate the plane boundaries
        //
        // See: * 4.7.2.  plane_pixel_height
        //      * 4.8.1.  plane_pixel_width
        let full_plane = SlicePlane {
            start_x,
            start_y,
            width,
            height,
            stride,
            offset: offset as usize,
            quant: 0,
        };

        // alpha is an additiona full plane
        if record.extra_plane {
            let alpha_plane = SlicePlane {
                quant: 2,
                ..full_plane
            };
            current_slice.planes.push(alpha_plane);
        }

        current_slice.planes.push(full_plane);

        if record.chroma_planes {
            // This is, of course, silly, but I want to do it "by the spec".
            let start_x = (start_x as f64
                / ((1 << record.log2_v_chroma_subsample) as f64))
                .ceil() as u32;
            let start_y = (start_y as f64
                / ((1 << record.log2_h_chroma_subsample) as f64))
                .ceil() as u32;
            let width = (width as f64
                / (1 << record.log2_h_chroma_subsample) as f64)
                .ceil() as u32;
            let height = (height as f64
                / (1 << record.log2_v_chroma_subsample) as f64)
                .ceil() as u32;
            let stride = (record.width as f64
                / (1 << record.log2_h_chroma_subsample) as f64)
                .ceil() as u32;
            let offset = start_x + start_y * stride;
            let chroma_plane = SlicePlane {
                start_x,
                start_y,
                width,
                height,
                stride,
                offset: offset as usize,
                quant: 1,
            };

            current_slice.planes.push(chroma_plane.clone());
            current_slice.planes.push(chroma_plane);
        }
    }

    /// Line decoding.
    ///
    /// So, so many arguments. I would have just inlined this whole thing
    /// but it needs to be separate because of RGB mode where every line
    /// is done in its entirety instead of per plane.
    ///
    /// Many could be refactored into being in the context, but I haven't
    /// got to it yet, so instead, I shall repent once for each function
    /// argument, twice daily.
    ///
    /// See: 4.7. Line
    #[allow(clippy::too_many_arguments)]
    fn decode_line<T>(
        header: &SliceHeader,
        record: &ConfigRecord,
        coder: &mut Coder,
        state: &mut Vec<Vec<Vec<u8>>>,
        golomb_state: &mut Vec<Vec<State>>,
        buf: &mut [T],
        width: usize,
        height: usize,
        stride: usize,
        yy: usize,
        qt: usize,
    ) where
        T: AsPrimitive<usize>,
        u32: AsPrimitive<T>,
    {
        // Runs are horizontal and thus cannot run more than a line.
        //
        // See: 3.8.2.2.1. Run Length Coding
        if let Coder::Golomb(ref mut golomb_coder) = coder {
            golomb_coder.new_line();
        }

        // 3.8. Coding of the Sample Difference
        let shift = if record.colorspace_type == 1 {
            record.bits_per_raw_sample + 1
        } else {
            record.bits_per_raw_sample
        };

        let quant_table =
            &record.quant_tables[header.quant_table_set_index[qt] as usize];

        // 4.7.4. sample_difference
        for x in 0..width {
            // Derive neighbours
            //
            // See pred.go for details.
            #[allow(non_snake_case)]
            #[allow(clippy::many_single_char_names)]
            let (T, L, t, l, tr, tl) =
                derive_borders(buf, x, yy, width, height, stride);

            // See pred.go for details.
            //
            // See also: * 3.4. Context
            //           * 3.6. Quantization Table Set Indexes
            let mut context = get_context(quant_table, T, L, t, l, tr, tl);
            let sign = if context < 0 {
                context = -context;
                true
            } else {
                false
            };

            let mut diff = match coder {
                Coder::Golomb(ref mut golomb_coder) => golomb_coder.sg(
                    context,
                    &mut golomb_state[qt][context as usize],
                    shift as u32,
                ),
                Coder::Range(ref mut range_coder) => {
                    range_coder.sr(&mut state[qt][context as usize])
                }
            };

            // 3.4. Context
            if sign {
                diff = -diff;
            }

            // 3.8. Coding of the Sample Difference
            let mut val: i32 = diff;
            if record.colorspace_type == 0
                && record.bits_per_raw_sample == 16
                && matches!(coder, Coder::Golomb(_))
            {
                // 3.3. Median Predictor
                let left16s = if l >= 32768 { l - 65536 } else { l };
                let top16s = if t >= 32768 { t - 65536 } else { t };
                let diag16s = if tl >= 32768 { tl - 65536 } else { tl };

                val += get_median(
                    left16s as i32,
                    top16s as i32,
                    (left16s + top16s - diag16s) as i32,
                );
            } else {
                val += get_median(l as i32, t as i32, (l + t - tl) as i32);
            }

            val &= (1 << shift) - 1;

            let val1 = val as u32;

            buf[(yy * stride) + x] = val1.as_();
        }
    }

    /// YCbCr Mode
    ///
    /// Planes are independent.
    ///
    /// See: 3.7.1. YCbCr
    #[allow(clippy::needless_range_loop)]
    fn decode_slice_content_yuv<T>(
        current_slice: &mut Slice,
        record: &ConfigRecord,
        coder: &mut Coder,
        buf: &mut Vec<Vec<T>>,
    ) where
        T: AsPrimitive<usize>,
        u32: AsPrimitive<T>,
    {
        let planes = &current_slice.planes;
        let header = &current_slice.header;
        let state = &mut current_slice.state;
        let golomb_state = &mut current_slice.golomb_state;

        for (plane, buf) in planes.iter().zip(buf.iter_mut()) {
            // 3.8.2.2.1. Run Length Coding
            if let Coder::Golomb(ref mut golomb_coder) = coder {
                golomb_coder.new_plane(plane.width as u32);
            }

            for y in 0..plane.height as usize {
                Self::decode_line(
                    header,
                    record,
                    coder,
                    state,
                    golomb_state,
                    &mut buf[plane.offset..],
                    plane.width as usize,
                    plane.height as usize,
                    plane.stride as usize,
                    y,
                    plane.quant.into(),
                );
            }
        }
    }

    /// RGB (JPEG2000-RCT) Mode
    ///
    /// All planes are coded per line.
    ///
    /// See: 3.7.2. RGB
    fn decode_slice_content_rct<T>(
        current_slice: &mut Slice,
        record: &ConfigRecord,
        coder: &mut Coder,
        buf: &mut Vec<Vec<T>>,
    ) where
        T: AsPrimitive<usize>,
        u32: AsPrimitive<T>,
    {
        let planes = &current_slice.planes;
        // All the planes have the same dimension
        // Just the quantizer change.
        let stride = planes[0].stride as usize;
        let width = planes[0].width as usize;
        let height = planes[0].height as usize;
        let offset = planes[0].offset;

        let header = &current_slice.header;
        let state = &mut current_slice.state;
        let golomb_state = &mut current_slice.golomb_state;

        if let Coder::Golomb(ref mut golomb_coder) = coder {
            golomb_coder.new_plane(width as u32);
        }

        for y in 0..height {
            for (plane, buf) in planes.iter().zip(buf.iter_mut()) {
                Self::decode_line(
                    header,
                    record,
                    coder,
                    state,
                    golomb_state,
                    &mut buf[offset..],
                    width,
                    height,
                    stride,
                    y,
                    plane.quant.into(),
                );
            }
        }
    }

    /// Decoding happens here.
    ///
    /// See: * 4.6. Slice Content
    fn decode_slice_content(
        current_slice: &mut Slice,
        record: &ConfigRecord,
        coder: &mut Coder,
        frame: &mut Frame,
    ) {
        if record.colorspace_type != 1 {
            if record.bits_per_raw_sample == 8 {
                Self::decode_slice_content_yuv(
                    current_slice,
                    record,
                    coder,
                    &mut frame.buf,
                );
            } else if record.bits_per_raw_sample == 16 {
                Self::decode_slice_content_yuv(
                    current_slice,
                    record,
                    coder,
                    &mut frame.buf16,
                );
            }
        } else {
            let stride = current_slice.planes[0].stride as usize;
            let width = current_slice.planes[0].width as usize;
            let height = current_slice.planes[0].height as usize;
            let offset = current_slice.planes[0].offset;
            if record.bits_per_raw_sample == 8 {
                Self::decode_slice_content_rct(
                    current_slice,
                    record,
                    coder,
                    &mut frame.buf16,
                );
                RCT::rct(
                    &mut frame.buf,
                    &frame.buf16,
                    width,
                    height,
                    stride,
                    offset,
                    record.bits_per_raw_sample.into(),
                );
            } else if record.bits_per_raw_sample >= 9
                && record.bits_per_raw_sample <= 15
                && !record.extra_plane
            {
                Self::decode_slice_content_rct(
                    current_slice,
                    record,
                    coder,
                    &mut frame.buf16,
                );
                // See: 3.7.2. RGB
                RCT::rct(
                    &mut frame.buf16,
                    &frame.buf,
                    width,
                    height,
                    stride,
                    offset,
                    record.bits_per_raw_sample.into(),
                );
            } else {
                Self::decode_slice_content_rct(
                    current_slice,
                    record,
                    coder,
                    &mut frame.buf32,
                );
                RCT::rct(
                    &mut frame.buf16,
                    &frame.buf32,
                    width,
                    height,
                    stride,
                    offset,
                    record.bits_per_raw_sample.into(),
                );
            }
        }
    }

    /// Resets the range coder and Golomb-Rice coder states.
    fn reset_slice_states(current_slice: &mut Slice, record: &ConfigRecord) {
        // Range coder states
        current_slice.state = record.initial_states.clone();

        // Golomb-Rice Code states
        if record.coder_type == 0 {
            let count = record.quant_table_set_count;
            current_slice.golomb_state = record.context_count[..count]
                .iter()
                .map(|&len| vec![Default::default(); len as usize])
                .collect();
        }
    }

    fn decode_slice(
        &mut self,
        buf: &[u8],
        slicenum: usize,
        frame: &mut Frame,
    ) -> Result<()> {
        let slice_info = self.current_frame.slice_info[slicenum];
        let current_slice = &mut self.current_frame.slices[slicenum];
        let record = &self.record;
        // Before we do anything, let's try and check the integrity
        //
        // See: * 4.8.2. error_status
        //      * 4.8.3. slice_crc_parity
        if record.ec == 1 {
            if slice_info.error_status != 0 {
                return Err(Error::SliceError(format!(
                    "error_status is non-zero: {}",
                    slice_info.error_status
                )));
            }

            let slice_buf_first = &buf[slice_info.pos..];
            let slice_buf_end = &slice_buf_first[..slice_info.size + 8]; // 8 bytes for footer size
            if crc32_mpeg2(&slice_buf_end) != 0 {
                return Err(Error::InvalidInputData(
                    "CRC mismatch".to_owned(),
                ));
            }
        }

        // If this is a keyframe, refresh states.
        //
        // See: * 3.8.1.3. Initial Values for the Context Model
        //      * 3.8.2.4. Initial Values for the VLC context state
        if self.current_frame.keyframe {
            Self::reset_slice_states(current_slice, record);
        }

        let mut coder = RangeCoder::new(&buf[slice_info.pos..]);

        // 4. Bitstream
        let mut state: [u8; CONTEXT_SIZE] = [128; CONTEXT_SIZE];

        // Skip keyframe bit on slice 0
        if slicenum == 0 {
            coder.br(&mut state);
        }

        if record.coder_type == 2 {
            // Custom state transition table
            coder.set_table(&self.state_transition);
        }

        Self::parse_slice_header(current_slice, record, &mut coder);

        let mut coder = if record.coder_type == 0 {
            // We're switching to Golomb-Rice mode now so we need the bitstream
            // position.
            //
            // See: 3.8.1.1.1. Termination
            coder.sentinel_end();
            let offset = coder.get_pos() - 1;
            let coder = GolombCoder::new(&buf[slice_info.pos + offset..]);
            Coder::Golomb(coder)
        } else {
            Coder::Range(coder)
        };

        Self::decode_slice_content(current_slice, record, &mut coder, frame);

        Ok(())
    }
}
