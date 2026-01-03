//! # Engine State
//!
//! This module defines the state machine's internal structures used by the
//! engine to track modal settings across ZPL commands.

/// Represents the accumulated state for a single ZPL field.
#[derive(Default)]
pub struct ZplInstructionState {
    /// Field origin coordinates.
    pub position: ZplInstructionPosition,
    /// Field typeset coordinates.
    pub typeset: ZplInstructionTypeset,
    /// Dimensions for graphic elements or general metrics.
    pub metrics: ZplInstructionMetrics,
    /// Default barcode metrics (usually set by `^BY`).
    pub barcode_metrics: ZplInstructionMetrics,
    /// Qualitative attributes like orientation and check-digits.
    pub attributes: ZplInstructionAttributes,
    /// Algorithm-specific parameters (e.g., QR masks, rounding).
    pub params: ZplInstructionParams,
    /// Current font configuration.
    pub font: ZplInstructionFont,
    /// Whether reverse printing (white-on-black) is active.
    pub reverse: bool,
    /// The string content for text or barcode data.
    pub value: Option<String>,
    /// Raw binary data for graphic fields.
    pub graphic_data: Option<Vec<u8>>,
    /// The type of instruction currently being built.
    pub instruction_type: Option<ZplInstructionType>,
}

/// Represents absolute positioning for a field.
#[derive(Default)]
pub struct ZplInstructionPosition {
    /// X coordinate in dots.
    pub x: u32,
    /// Y coordinate in dots.
    pub y: u32,
}

/// Represents typeset positioning for a field.
#[derive(Default)]
pub struct ZplInstructionTypeset {
    /// X offset in dots.
    pub x: u32,
    /// Y offset in dots.
    pub y: u32,
}

/// Shared numeric data for various instructions.
#[derive(Default)]
pub struct ZplInstructionMetrics {
    /// Width in dots.
    pub width: u32,
    /// Height in dots.
    pub height: u32,
    /// Thickness, module width, or magnification depending on context.
    pub thickness: u32,
}

/// Qualitative flags and settings for fields.
#[derive(Default)]
pub struct ZplInstructionAttributes {
    /// Field orientation (N, R, I, B).
    pub orientation: Option<char>,
    /// Interpretation line visibility flag.
    pub interpretation_line: Option<char>,
    /// Whether the interpretation line is above the barcode.
    pub interpretation_above: Option<char>,
    /// Check digit verification flag.
    pub check_digit: Option<char>,
    /// Barcode-specific mode (e.g., UCC Case Mode).
    pub mode: Option<char>,
    /// Error correction level (e.g., for QR or PDF417).
    pub error_correction: Option<char>,
    /// Line color identifier ('B' for black, 'W' for white).
    pub line_color: Option<char>,
    /// Custom line color in hex format.
    pub custom_line_color: Option<String>,
}

/// Algorithm-specific values and complex parameters.
#[derive(Default)]
pub struct ZplInstructionParams {
    /// Corner rounding for boxes.
    pub rounding: u32,
    /// Model identifier (e.g., QR model 1 or 2).
    pub model: u32,
    /// Data mask for 2D barcodes.
    pub mask: u32,
    /// Wide-to-narrow bar ratio.
    pub ratio: Option<f64>,
}

/// Font specification state.
#[derive(Default)]
pub struct ZplInstructionFont {
    /// Font name/identifier.
    pub font_name: char,
    /// Font orientation.
    pub orientation: Option<char>,
    /// Character height in dots.
    pub height: Option<u32>,
    /// Character width in dots.
    pub width: Option<u32>,
    /// Custom text color in hex format.
    pub color: Option<String>,
}

/// Supported instruction types for rendering.
#[allow(dead_code)]
pub enum ZplInstructionType {
    /// Plain or formatted text.
    Text,
    /// Rectangular box.
    GraphicBox,
    /// Circle graphic.
    GraphicCircle,
    /// Ellipse graphic.
    GraphicEllipse,
    /// Bitmap image data.
    GraphicField,
    /// Code 128 barcode.
    Code128,
    /// QR Code barcode.
    QRCode,
    /// Code 39 barcode.
    Code39,
    /// Custom color image data.
    CustomImage,
}
