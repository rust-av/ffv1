use thiserror::Error;

/// General decoding errors.
#[derive(Debug, Error)]
pub enum Error {
    /// Invalid input data.
    #[error("Invalid input data: {0}")]
    InvalidInputData(String),
    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    /// Frame error.
    #[error("Frame error: {0}")]
    FrameError(String),
    /// Slice error.
    #[error("Slice error: {0}")]
    SliceError(String),
}

/// A specialised `Result` type for decoding operations.
pub type Result<T> = ::std::result::Result<T, Error>;
