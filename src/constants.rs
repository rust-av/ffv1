// Internal constants.
pub(crate) const MAX_QUANT_TABLES: usize = 8; // Only defined in FFmpeg?
pub(crate) const MAX_CONTEXT_INPUTS: usize = 5; // 4.9. Quantization Table Set
pub(crate) const CONTEXT_SIZE: usize = 32; // 4.1. Parameters

// API constants.

// FIXME: Use enum

/// Colorspaces.
/// From 4.1.5. colorspace_type
pub const YCBCR: usize = 0;
pub const RGB: usize = 1;
