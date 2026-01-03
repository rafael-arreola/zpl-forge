use super::commons::{Justification, YesNo};

/// Represents the supported ZPL commands in the AST.
#[derive(Debug, Clone)]
#[allow(dead_code, clippy::enum_variant_names)]
pub enum Command {
    /// ^XA - Start Format
    /// Starts the definition of a label format.
    StartFormat,

    /// ^XZ - End Format
    /// Terminates the definition of a label format.
    EndFormat,

    /// ^LH - Label Home
    /// Defines the default home position for the label.
    LabelHome {
        /// X coordinate of the home position (in dots)
        x: Option<u32>,
        /// Y coordinate of the home position (in dots)
        y: Option<u32>,
    },

    /// ^LL - Label Length
    /// Defines the length of the label (Y axis).
    LabelLength {
        /// Length of the label in dots
        length: Option<u32>,
    },

    /// ^FO - Field Origin
    /// Sets the top-left corner of the field area relative to the Label Home.
    FieldOrigin {
        /// X coordinate (in dots)
        x: Option<u32>,
        /// Y coordinate (in dots)
        y: Option<u32>,
    },

    /// ^FT - Field Typeset
    /// Sets the field position relative to the Label Home, defining the font's base point.
    FieldTypeset {
        /// X coordinate (in dots)
        x: Option<u32>,
        /// Y coordinate (in dots)
        y: Option<u32>,
    },

    /// ^FS - Field Separator
    /// Indicates the end of a field definition.
    FieldSeparator,

    /// ^LR - Label Reverse
    /// Inverts the printing of the entire label (white on black).
    LabelReverse {
        /// Reverse label (Y/N)
        reverse: Option<YesNo>,
    },

    /// ^FX - Comment
    /// A comment that does not print or affect the label.
    Comment {
        /// Comment text
        text: String,
    },

    /// ^A - Font Specification (Full)
    /// Specifies the font to be used in the following text field.
    FontSpecFull {
        /// Font name/letter (A-Z, 0-9)
        font_name: char,
        /// Field orientation (N, R, I, B)
        orientation: Option<char>,
        /// Character height in dots
        height: Option<u32>,
        /// Character width in dots
        width: Option<u32>,
    },

    /// ^CF - Change Default Font
    /// Sets the default alphanumeric font.
    FontSpec {
        /// Font name/letter
        font_name: char,
        /// Character height
        height: Option<u32>,
        /// Character width
        width: Option<u32>,
    },

    /// ^FD - Field Data
    /// Defines the data to be printed in the field.
    FieldData {
        /// Data string
        data: String,
    },

    /// ^FB - Field Block
    /// Formats a block of text within a defined rectangle.
    FieldBlock {
        /// Width of the text block
        width: Option<u32>,
        /// Maximum number of lines
        max_lines: Option<u32>,
        /// Extra space between lines
        line_spacing: Option<u32>,
        /// Text justification (L, C, R, J)
        justification: Option<Justification>,
        /// Indentation for the second line onwards
        indent: Option<u32>,
    },

    /// ^CI - Change International Font/Encoding
    /// Changes the character set or international encoding.
    ChangeIntFont {
        /// Character set identifier
        charset: Option<u32>,
    },

    /// ^FR - Field Reverse Print
    /// Prints the field as white on black (inverted).
    FieldReverse,

    /// ^GB - Graphic Box
    /// Draws boxes or lines.
    GraphicBox {
        /// Box width
        width: u32,
        /// Box height
        height: u32,
        /// Border thickness
        border_thickness: Option<u32>,
        /// Line color (B=Black, W=White)
        line_color: Option<char>,
        /// Corner rounding (0-8)
        corner_rounding: Option<u32>,
    },

    /// ^GC - Graphic Circle
    /// Draws a circle.
    GraphicCircle {
        /// Circle diameter
        diameter: Option<u32>,
        /// Border thickness
        border_thickness: Option<u32>,
        /// Line color
        line_color: Option<char>,
    },

    /// ^GE - Graphic Ellipse
    /// Draws an ellipse.
    GraphicEllipse {
        /// Ellipse width
        width: Option<u32>,
        /// Ellipse height
        height: Option<u32>,
        /// Border thickness
        border_thickness: Option<u32>,
        /// Line color
        line_color: Option<char>,
    },

    /// ^GF - Graphic Field
    /// Allows downloading graphic data directly to the bitmap buffer.
    GraphicField {
        /// Compression type (A, B, C)
        compression_type: Option<char>,
        /// Total binary data byte count
        binary_byte_count: Option<u32>,
        /// Total graphic field byte count
        graphic_field_count: Option<u32>,
        /// Bytes per data row
        bytes_per_row: Option<u32>,
        /// Image data (hexadecimal or binary)
        data: String,
    },

    /// ^GIC - Custom Image Color (Extension)
    /// Custom command to load color images with width, height and base64 data.
    CustomImage {
        /// Image width
        width: u32,
        /// Image height
        height: u32,
        /// Base64 encoded image data
        data: String,
    },

    /// ^GTC - Custom Text Color (Extension)
    /// Custom command to set the color of subsequent text fields.
    GraphicTextColor {
        /// Color in hex format (e.g., #FF0000)
        color: String,
    },

    /// ^GLC - Custom Line Color (Extension)
    /// Custom command to set the color of subsequent graphic elements.
    GraphicLineColor {
        /// Color in hex format (e.g., #FF0000)
        color: String,
    },

    /// ^BC - Code 128 Barcode
    /// Code 128 Barcode (Subsets A, B, and C).
    Code128 {
        /// Orientation
        orientation: Option<char>,
        /// Bar height
        height: Option<u32>,
        /// Print interpretation line (Y/N)
        interpretation_line: Option<char>,
        /// Interpretation line above (Y/N)
        interpretation_line_above: Option<char>,
        /// Verify check digit (Y/N)
        check_digit: Option<char>,
        /// Mode (N=Not selected, U=UCC Case Mode, A=Automatic, D=UCC/EAN Display)
        mode: Option<char>,
    },

    /// ^BQ - QR Code Barcode
    /// QR Code Barcode.
    QRCode {
        /// Orientation
        orientation: Option<char>,
        /// Model (1=Original, 2=Enhanced)
        model: Option<u32>,
        /// Magnification factor (1-10)
        magnification: Option<u32>,
        /// Error correction level (H, Q, M, L)
        error_correction: Option<char>,
        /// Data mask (0-7)
        mask: Option<u32>,
    },

    /// ^B3 - Code 39 Barcode
    /// Code 39 Barcode.
    Code39 {
        /// Orientation
        orientation: Option<char>,
        /// Check digit
        check_digit: Option<char>,
        /// Bar height
        height: Option<u32>,
        /// Print interpretation line
        interpretation_line: Option<char>,
        /// Interpretation line above
        interpretation_line_above: Option<char>,
    },

    /// ^BY - Barcode Field Default
    /// Changes the default values for barcodes.
    BarcodeDefault {
        /// Module width (in dots)
        module_width: Option<u32>,
        /// Wide to narrow bar ratio
        ratio: Option<f32>,
        /// Bar height
        height: Option<u32>,
    },

    /// ^BX - Data Matrix Barcode
    /// Two-dimensional Data Matrix Barcode.
    DataMatrix {
        /// Orientation
        orientation: Option<char>,
        /// Dimensional height
        height: Option<u32>,
        /// Quality level
        quality: Option<u32>,
        /// Columns to encode
        columns: Option<u32>,
        /// Rows to encode
        rows: Option<u32>,
    },

    /// ^B7 - PDF417 Barcode
    /// Multi-dimensional PDF417 Barcode.
    Pdf417 {
        /// Orientation
        orientation: Option<char>,
        /// Bar height
        height: Option<u32>,
        /// Security level
        security_level: Option<u32>,
        /// Number of data columns
        columns: Option<u32>,
        /// Number of rows
        rows: Option<u32>,
        /// Truncate symbol (Y/N)
        truncate: Option<YesNo>,
    },

    /// Unsupported or unknown command
    UnsupportedCommand {
        /// Command code (e.g., ^XY)
        command: String,
        /// Raw command arguments
        args: String,
    },
}
