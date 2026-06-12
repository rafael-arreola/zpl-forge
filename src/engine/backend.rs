use crate::engine::common::Barcode1DKind;
use crate::{FontManager, ZplResult};

/// Defines the interface for rendering ZPL instructions.
///
/// Implementing this trait allows `zpl-forge` to output label formats to
/// different targets such as images (PNG, JPG), PDF documents, or raw byte streams.
#[allow(clippy::too_many_arguments)]
pub trait ZplForgeBackend {
    /// Initializes the rendering surface with the specified dimensions.
    fn setup_page(&mut self, width: f64, height: f64, resolution: f32);

    /// Starts a new page, called between consecutive `^XA...^XZ` blocks.
    ///
    /// Multi-page backends (PDF) finish the current page and begin a fresh
    /// one with the same dimensions. The default implementation is a no-op,
    /// which keeps single-surface backends (PNG) drawing on the same canvas.
    fn new_page(&mut self) -> ZplResult<()> {
        Ok(())
    }

    /// Configures the font manager used for text rendering.
    fn setup_font_manager(&mut self, font_manager: &FontManager);

    /// Renders a single line of text.
    ///
    /// `orientation` follows `^A`: 'N' (normal), 'R' (rotated 90° clockwise),
    /// 'I' (inverted 180°), 'B' (read from bottom up, 270° clockwise).
    /// `(x, y)` is always the top-left corner of the rendered (rotated) cell.
    fn draw_text(
        &mut self,
        x: u32,
        y: u32,
        font: char,
        height: Option<u32>,
        width: Option<u32>,
        orientation: char,
        text: &str,
        reverse_print: bool,
        color: Option<String>,
    ) -> ZplResult<()>;

    /// Draws a rectangular box.
    fn draw_graphic_box(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        thickness: u32,
        color: char,
        custom_color: Option<String>,
        rounding: u32,
        reverse_print: bool,
    ) -> ZplResult<()>;

    /// Draws a circle.
    fn draw_graphic_circle(
        &mut self,
        x: u32,
        y: u32,
        radius: u32,
        thickness: u32,
        color: char,
        custom_color: Option<String>,
        reverse_print: bool,
    ) -> ZplResult<()>;

    /// Draws an ellipse.
    fn draw_graphic_ellipse(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        thickness: u32,
        color: char,
        custom_color: Option<String>,
        reverse_print: bool,
    ) -> ZplResult<()>;

    /// Renders a raw graphic field (bitmap data).
    fn draw_graphic_field(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: &[u8],
        reverse_print: bool,
    ) -> ZplResult<()>;

    /// Renders a custom color image from base64 data.
    ///
    /// If width and height are 0, natural size is used.
    /// If one is 0, the other is scaled proportionally.
    fn draw_graphic_image_custom(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: &str,
    ) -> ZplResult<()>;

    /// Draws a Code 128 barcode.
    fn draw_code128(
        &mut self,
        x: u32,
        y: u32,
        orientation: char,
        height: u32,
        module_width: u32,
        interpretation_line: char,
        interpretation_line_above: char,
        check_digit: char,
        mode: char,
        data: &str,
        reverse_print: bool,
    ) -> ZplResult<()>;

    /// Draws a QR Code.
    fn draw_qr_code(
        &mut self,
        x: u32,
        y: u32,
        orientation: char,
        model: u32,
        magnification: u32,
        error_correction: char,
        mask: u32,
        data: &str,
        reverse_print: bool,
    ) -> ZplResult<()>;

    /// Draws a generic 1-D barcode (EAN-13, UPC-A, Interleaved 2 of 5, Code 93).
    #[allow(clippy::too_many_arguments)]
    fn draw_barcode_1d(
        &mut self,
        kind: Barcode1DKind,
        x: u32,
        y: u32,
        orientation: char,
        height: u32,
        module_width: u32,
        interpretation_line: char,
        interpretation_line_above: char,
        data: &str,
        reverse_print: bool,
    ) -> ZplResult<()>;

    /// Draws a diagonal line (`^GD`). `diagonal_orientation` is 'R' (`/`) or 'L' (`\`).
    #[allow(clippy::too_many_arguments)]
    fn draw_graphic_diagonal(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        thickness: u32,
        color: char,
        custom_color: Option<String>,
        diagonal_orientation: char,
        reverse_print: bool,
    ) -> ZplResult<()>;

    /// Draws a Data Matrix (ECC 200) barcode.
    ///
    /// `module_size` is the side of each module cell in dots.
    fn draw_datamatrix(
        &mut self,
        x: u32,
        y: u32,
        orientation: char,
        module_size: u32,
        data: &str,
        reverse_print: bool,
    ) -> ZplResult<()>;

    /// Draws a PDF417 barcode.
    ///
    /// `row_height` is the bar height per matrix row and `module_width` the
    /// narrow bar width, both in dots. `security_level` is the PDF417 error
    /// correction level (0-8).
    fn draw_pdf417(
        &mut self,
        x: u32,
        y: u32,
        orientation: char,
        row_height: u32,
        module_width: u32,
        security_level: u32,
        data: &str,
        reverse_print: bool,
    ) -> ZplResult<()>;

    /// Draws a Code 39 barcode.
    fn draw_code39(
        &mut self,
        x: u32,
        y: u32,
        orientation: char,
        check_digit: char,
        height: u32,
        module_width: u32,
        interpretation_line: char,
        interpretation_line_above: char,
        data: &str,
        reverse_print: bool,
    ) -> ZplResult<()>;

    /// Finalizes the rendering process and returns the resulting data.
    fn finalize(&mut self) -> ZplResult<Vec<u8>>;
}
