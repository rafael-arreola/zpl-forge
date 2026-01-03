/// Represents a self-contained ZPL instruction ready for rendering.
///
/// Unlike AST commands, instructions are calculated based on the cumulative
/// state of the parser (e.g., coordinates are absolute, fonts are resolved).
#[derive(Debug)]
pub enum ZplInstruction {
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
        /// Text content.
        text: String,
        /// Whether to print white-on-black.
        reverse_print: bool,
        /// Custom text color.
        color: Option<String>,
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
    },
    /// Renders a bitmap graphic.
    GraphicField {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: Vec<u8>,
        reverse_print: bool,
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
    },
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
