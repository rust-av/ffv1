use crate::constants::{CONTEXT_SIZE, MAX_CONTEXT_INPUTS, MAX_QUANT_TABLES};
use crate::crc32mpeg2::crc32_mpeg2;
use crate::error::{Error, Result};
use crate::range::RangeCoder;

pub struct ConfigRecord {
    pub version: u8,
    pub micro_version: u8,
    pub coder_type: u8,
    pub state_transition_delta: [i16; 256],
    pub colorspace_type: u8,
    pub bits_per_raw_sample: u8,
    pub chroma_planes: bool,
    pub log2_h_chroma_subsample: u8,
    pub log2_v_chroma_subsample: u8,
    pub extra_plane: bool,
    pub num_h_slices_minus1: u8,
    pub num_v_slices_minus1: u8,
    pub quant_table_set_count: usize,
    pub context_count: [i32; MAX_QUANT_TABLES],
    pub quant_tables: [[[i16; 256]; MAX_CONTEXT_INPUTS]; MAX_QUANT_TABLES],
    pub states_coded: bool,
    pub initial_state_delta: Vec<Vec<Vec<i16>>>, // FIXME: This is horrible
    pub initial_states: Vec<Vec<Vec<u8>>>,
    pub ec: u8,
    pub intra: u8,
    pub width: u32,
    pub height: u32,
}

impl ConfigRecord {
    /// Parse the configuration record from the codec private data
    /// and store the width and height provided by the container.
    ///
    /// See: * 4.1. Parameters
    ///      * 4.2. Configuration Record
    pub fn parse_config_record(
        buf: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Self> {
        // Before we do anything, CRC check.
        //
        // See: 4.2.2. configuration_record_crc_parity
        if crc32_mpeg2(buf) != 0 {
            return Err(Error::InvalidConfiguration(
                "failed CRC check for configuration record".to_owned(),
            ));
        }
        let mut coder = RangeCoder::new(buf);
        let mut state_transition_delta: [i16; 256] = [0; 256];
        let mut context_count: [i32; MAX_QUANT_TABLES] = [0; MAX_QUANT_TABLES];
        let mut quant_tables: [[[i16; 256]; MAX_CONTEXT_INPUTS];
            MAX_QUANT_TABLES] =
            [[[0; 256]; MAX_CONTEXT_INPUTS]; MAX_QUANT_TABLES];

        // 4. Bitstream
        let mut state: [u8; CONTEXT_SIZE] = [128; CONTEXT_SIZE];

        // 4.1.1. version
        let version = coder.ur(&mut state) as u8;
        if version != 3 {
            return Err(Error::InvalidConfiguration(
                "only FFV1 version 3 is supported".to_owned(),
            ));
        }

        // 4.1.2. micro_version
        let micro_version = coder.ur(&mut state) as u8;
        if micro_version < 1 {
            return Err(Error::InvalidConfiguration(
                "only FFV1 micro version >1 supported".to_owned(),
            ));
        }

        // 4.1.3. coder_type
        let coder_type = coder.ur(&mut state) as u8;
        if coder_type > 2 {
            return Err(Error::InvalidConfiguration(format!(
                "invalid coder_type: {}",
                coder_type
            )));
        }

        // 4.1.4. state_transition_delta
        if coder_type > 1 {
            for state_transition_delta in
                state_transition_delta.iter_mut().skip(1)
            {
                *state_transition_delta = coder.sr(&mut state) as i16;
            }
        }

        // 4.1.5. colorspace_type
        let colorspace_type = coder.ur(&mut state) as u8;
        if colorspace_type > 1 {
            return Err(Error::InvalidConfiguration(format!(
                "invalid colorspace_type: {}",
                colorspace_type
            )));
        }

        // 4.1.7. bits_per_raw_sample
        let mut bits_per_raw_sample = coder.ur(&mut state) as u8;
        if bits_per_raw_sample == 0 {
            bits_per_raw_sample = 8;
        }
        if coder_type == 0 && bits_per_raw_sample != 8 {
            return Err(Error::InvalidConfiguration(
                "golomb-rice mode cannot have >8bit per sample".to_owned(),
            ));
        }

        // 4.1.6. chroma_planes
        let chroma_planes = coder.br(&mut state);
        if colorspace_type == 1 && !chroma_planes {
            return Err(Error::InvalidConfiguration(
                "RGB must contain chroma planes".to_owned(),
            ));
        }

        // 4.1.8. log2_h_chroma_subsample
        let log2_h_chroma_subsample = coder.ur(&mut state) as u8;
        if colorspace_type == 1 && log2_h_chroma_subsample != 0 {
            return Err(Error::InvalidConfiguration(
                "RGB cannot be subsampled".to_owned(),
            ));
        }

        // 4.1.9. log2_v_chroma_subsample
        let log2_v_chroma_subsample = coder.ur(&mut state) as u8;
        if colorspace_type == 1 && log2_v_chroma_subsample != 0 {
            return Err(Error::InvalidConfiguration(
                "RGB cannot be subsampled".to_owned(),
            ));
        }

        // 4.1.10. extra_plane
        let extra_plane = coder.br(&mut state);
        // 4.1.11. num_h_slices
        let num_h_slices_minus1 = coder.ur(&mut state) as u8;
        // 4.1.12. num_v_slices
        let num_v_slices_minus1 = coder.ur(&mut state) as u8;

        // 4.1.13. quant_table_set_count
        let quant_table_set_count = coder.ur(&mut state) as usize;
        if quant_table_set_count == 0 {
            return Err(Error::InvalidConfiguration(
                "quant_table_set_count may not be zero".to_owned(),
            ));
        } else if quant_table_set_count > MAX_QUANT_TABLES {
            return Err(Error::InvalidConfiguration(format!(
                "too many quant tables: {} > {}",
                quant_table_set_count, MAX_QUANT_TABLES
            )));
        }

        for i in 0..quant_table_set_count {
            // 4.9.  Quantization Table Set
            let mut scale = 1;
            for j in 0..MAX_CONTEXT_INPUTS {
                // Each table has its own state table.
                let mut quant_state: [u8; CONTEXT_SIZE] = [128; CONTEXT_SIZE];
                let mut v = 0;
                let mut k = 0;
                while k < 128 {
                    let len_minus1 = coder.ur(&mut quant_state);
                    for _ in 0..(len_minus1 + 1) as usize {
                        quant_tables[i][j][k] = (scale * v) as i16;
                        k += 1;
                    }
                    v += 1;
                }
                for k in 1..128 {
                    quant_tables[i][j][256 - k] = -quant_tables[i][j][k];
                }
                quant_tables[i][j][128] = -quant_tables[i][j][127];
                scale *= 2 * v - 1;
            }
            context_count[i] = (scale + 1) as i32 / 2;
        }

        // Why on earth did they choose to do a variable length buffer in the
        // *middle and start* of a 3D array?
        let mut initial_state_delta: Vec<Vec<Vec<i16>>> =
            vec![Vec::new(); quant_table_set_count];
        for i in 0..quant_table_set_count {
            initial_state_delta[i] =
                vec![Vec::new(); context_count[i] as usize];
            for j in 0..context_count[i] as usize {
                initial_state_delta[i][j] = vec![0; CONTEXT_SIZE as usize];
            }
            let states_coded = coder.br(&mut state);
            if states_coded {
                for j in 0..context_count[i] as usize {
                    for k in 0..CONTEXT_SIZE {
                        initial_state_delta[i][j][k] =
                            coder.sr(&mut state) as i16;
                    }
                }
            }
        }

        let mut initial_states = vec![Vec::new(); initial_state_delta.len()];
        for i in 0..initial_state_delta.len() {
            initial_states[i] = vec![Vec::new(); initial_state_delta[i].len()];
            for j in 0..initial_state_delta[i].len() {
                initial_states[i][j] =
                    vec![0; initial_state_delta[i][j].len()];
                for k in 0..initial_state_delta[i][j].len() {
                    let pred = if j != 0 {
                        initial_states[i][j - 1][k] as i16
                    } else {
                        128
                    };
                    initial_states[i][j][k] =
                        ((pred + initial_state_delta[i][j][k]) & 255) as u8;
                }
            }
        }

        // 4.1.16. ec
        let ec = coder.ur(&mut state) as u8;
        // 4.1.17. intra
        let intra = coder.ur(&mut state) as u8;

        let config_record = ConfigRecord {
            version,
            micro_version,
            coder_type,
            state_transition_delta,
            colorspace_type,
            bits_per_raw_sample,
            chroma_planes,
            log2_h_chroma_subsample,
            log2_v_chroma_subsample,
            extra_plane,
            num_h_slices_minus1,
            num_v_slices_minus1,
            quant_table_set_count,
            context_count,
            quant_tables,
            states_coded: false,
            initial_state_delta,
            initial_states,
            ec,
            intra,
            width,
            height,
        };

        Ok(config_record)
    }
}
