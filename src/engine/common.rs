pub use crate::ast::commons::Barcode1DKind;

/// Represents a self-contained ZPL instruction ready for rendering.
///
/// Unlike AST commands, instructions are calculated based on the cumulative
/// state of the parser (e.g., coordinates are absolute, fonts are resolved).
#[derive(Debug)]
pub enum ZplInstruction {
    /// Starts a new page. Emitted between consecutive `^XA...^XZ` blocks.
    ///
    /// Backends that support multi-page output (PDF) start a fresh page;
    /// single-surface backends (PNG) may ignore it.
    PageBreak,
    /// Renders a text field.
    Text {
        /// Absolute X coordinate.
        x: u32,
        /// Absolute Y coordinate.
        y: u32,
        /// Font identifier.
        font: char,
        /// Height in dots.
        height: Option<u32>,
        /// Width in dots.
        width: Option<u32>,
        /// Field orientation from `^A` (N, R, I, B).
        orientation: char,
        /// Text content.
        text: String,
        /// Whether to print white-on-black.
        reverse_print: bool,
        /// Custom text color.
        color: Option<String>,
        /// `^FB` block formatting (wrap, max lines, justification).
        block: Option<TextBlock>,
        /// Condition for this instruction.
        condition: Option<(String, String)>,
    },
    /// Draws a rectangular box.
    GraphicBox {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        thickness: u32,
        color: char,
        custom_color: Option<String>,
        rounding: u32,
        reverse_print: bool,
        condition: Option<(String, String)>,
    },
    /// Draws a circle.
    GraphicCircle {
        x: u32,
        y: u32,
        radius: u32,
        thickness: u32,
        color: char,
        custom_color: Option<String>,
        reverse_print: bool,
        condition: Option<(String, String)>,
    },
    /// Draws an ellipse.
    GraphicEllipse {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        thickness: u32,
        color: char,
        custom_color: Option<String>,
        reverse_print: bool,
        condition: Option<(String, String)>,
    },
    /// Renders a bitmap graphic.
    GraphicField {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: Vec<u8>,
        reverse_print: bool,
        condition: Option<(String, String)>,
    },
    /// Renders a custom color image (extension).
    CustomImage {
        /// Absolute X coordinate.
        x: u32,
        /// Absolute Y coordinate.
        y: u32,
        /// Requested width (0 for natural/proportional).
        width: u32,
        /// Requested height (0 for natural/proportional).
        height: u32,
        /// Base64 encoded image data.
        data: String,
        condition: Option<(String, String)>,
    },
    /// Draws a Code 128 barcode.
    Code128 {
        x: u32,
        y: u32,
        orientation: char,
        height: u32,
        module_width: u32,
        interpretation_line: char,
        interpretation_line_above: char,
        check_digit: char,
        mode: char,
        data: String,
        reverse_print: bool,
        condition: Option<(String, String)>,
    },
    /// Draws a QR Code.
    QRCode {
        x: u32,
        y: u32,
        orientation: char,
        model: u32,
        magnification: u32,
        error_correction: char,
        mask: u32,
        data: String,
        reverse_print: bool,
        condition: Option<(String, String)>,
    },
    /// Draws a generic 1-D barcode (EAN-13, UPC-A, ITF, Code 93).
    Barcode1D {
        kind: Barcode1DKind,
        x: u32,
        y: u32,
        orientation: char,
        height: u32,
        module_width: u32,
        interpretation_line: char,
        interpretation_line_above: char,
        data: String,
        reverse_print: bool,
        condition: Option<(String, String)>,
    },
    /// Draws a diagonal line (`^GD`).
    GraphicDiagonal {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        thickness: u32,
        color: char,
        custom_color: Option<String>,
        /// Leaning: 'R' (`/`) or 'L' (`\`).
        diagonal_orientation: char,
        reverse_print: bool,
        condition: Option<(String, String)>,
    },
    /// Draws a Data Matrix (ECC 200) barcode.
    DataMatrix {
        x: u32,
        y: u32,
        orientation: char,
        /// Module size in dots (`^BX` dimensional height).
        module_size: u32,
        data: String,
        reverse_print: bool,
        condition: Option<(String, String)>,
    },
    /// Draws a PDF417 barcode.
    Pdf417 {
        x: u32,
        y: u32,
        orientation: char,
        /// Row height in dots.
        row_height: u32,
        /// Module width in dots (from `^BY`).
        module_width: u32,
        /// Error correction security level (0-8).
        security_level: u32,
        data: String,
        reverse_print: bool,
        condition: Option<(String, String)>,
    },
    /// Draws a Code 39 barcode.
    Code39 {
        x: u32,
        y: u32,
        orientation: char,
        check_digit: char,
        height: u32,
        module_width: u32,
        interpretation_line: char,
        interpretation_line_above: char,
        data: String,
        reverse_print: bool,
        condition: Option<(String, String)>,
    },
}

/// `^FB` field-block formatting parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextBlock {
    /// Block width in dots; lines wrap to fit it.
    pub width: u32,
    /// Maximum number of lines (default 1).
    pub max_lines: u32,
    /// Extra space added between lines, in dots.
    pub line_spacing: i32,
    /// Justification: 'L', 'C', 'R' or 'J' (J renders as L).
    pub justification: char,
    /// Hanging indent applied from the second line onwards, in dots.
    pub indent: u32,
}

/// Represents common printer resolutions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Resolution {
    /// 152 DPI (6 dots/mm)
    Dpi152,
    /// 203 DPI (8 dots/mm) - Zebra Standard
    Dpi203,
    /// 300 DPI (12 dots/mm)
    Dpi300,
    /// 600 DPI (24 dots/mm)
    Dpi600,
    /// Custom DPI value
    Custom(f32),
}

impl Resolution {
    /// Returns the dots per millimeter for this resolution.
    pub fn dpmm(&self) -> f32 {
        match self {
            Resolution::Dpi152 => 6.0,
            Resolution::Dpi203 => 8.0,
            Resolution::Dpi300 => 12.0,
            Resolution::Dpi600 => 24.0,
            Resolution::Custom(val) => val / 25.4,
        }
    }

    /// Returns the dots per inch for this resolution.
    pub fn dpi(&self) -> f32 {
        match self {
            Resolution::Dpi152 => 152.0,
            Resolution::Dpi203 => 203.2,
            Resolution::Dpi300 => 304.8,
            Resolution::Dpi600 => 609.6,
            Resolution::Custom(val) => *val,
        }
    }
}

/// Physical units of measurement supported by the engine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unit {
    /// Raw dots.
    Dots(u32),
    /// Inches.
    Inches(f32),
    /// Millimeters.
    Millimeters(f32),
    /// Centimeters.
    Centimeters(f32),
}

impl Unit {
    /// Converts the unit to dots based on the provided resolution.
    pub fn to_dots(&self, resolution: Resolution) -> u32 {
        match self {
            Unit::Dots(dots) => *dots,
            Unit::Inches(inches) => (inches.max(0.0) * resolution.dpi()).round() as u32,
            Unit::Millimeters(mm) => (mm.max(0.0) * resolution.dpmm()).round() as u32,
            Unit::Centimeters(cm) => (cm.max(0.0) * 10.0 * resolution.dpmm()).round() as u32,
        }
    }
}
