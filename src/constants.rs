// Internal constants.
pub(crate) const MAX_QUANT_TABLES: u8 = 8; // Only defined in FFmpeg?
pub(crate) const MAX_CONTEXT_INPUTS: u8 = 5; // 4.9. Quantization Table Set
pub(crate) const CONTEXT_SIZE: u8 = 32; // 4.1. Parameters

// API constants.

// FIXME: Use enum

/// Colorspaces.
/// From 4.1.5. colorspace_type
pub const YCBCR: u8 = 0;
pub const RGB: u8 = 1;
