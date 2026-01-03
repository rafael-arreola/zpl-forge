use thiserror::Error;

/// Errors that can occur during ZPL parsing and rendering.
#[derive(Debug, Error)]
pub enum ZplError {
    /// Error during the parsing phase (nom).
    #[error("Parse error at line {line}: {message}")]
    ParseError {
        /// Line number where the error occurred.
        line: usize,
        /// Description of the parse failure.
        message: String,
    },

    /// Error building instructions from commands.
    #[error("Instruction builder error: {0}")]
    InstructionError(String),

    /// Error specific to a rendering backend (PNG, PDF, etc).
    #[error("Rendering backend error: {0}")]
    BackendError(String),

    /// Error related to font loading or registration.
    #[error("Font error: {0}")]
    FontError(String),

    /// Input ZPL was empty or only contained whitespace.
    #[error("Empty or invalid ZPL input")]
    EmptyInput,

    /// Errors related to image decoding (Base64 or binary).
    #[error("Image processing error: {0}")]
    ImageError(String),

    /// Security limit reached (e.g., OOM protection).
    #[error("Security limit exceeded: {0}")]
    SecurityLimitExceeded(String),

    /// Generic unexpected error.
    #[error("Unexpected error: {0}")]
    Unexpected(String),
}

/// A specialized Result type for ZPL operations.
pub type ZplResult<T> = Result<T, ZplError>;
