use crate::{FontManager, ZplResult};

/// Defines the interface for rendering ZPL instructions.
///
/// Implementing this trait allows `zpl-forge` to output label formats to
/// different targets such as images (PNG, JPG), PDF documents, or raw byte streams.
#[allow(clippy::too_many_arguments)]
pub trait ZplForgeBackend {
    /// Initializes the rendering surface with the specified dimensions.
    fn setup_page(&mut self, width: f64, height: f64, resolution: f32);

    /// Configures the font manager used for text rendering.
    fn setup_font_manager(&mut self, font_manager: &FontManager);

    /// Renders a text field.
    fn draw_text(
        &mut self,
        x: u32,
        y: u32,
        font: char,
        height: Option<u32>,
        width: Option<u32>,
        text: String,
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
        data: Vec<u8>,
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
        data: String,
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
        data: String,
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
        data: String,
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
        data: String,
        reverse_print: bool,
    ) -> ZplResult<()>;

    /// Finalizes the rendering process and returns the resulting data.
    fn finalize(&mut self) -> ZplResult<Vec<u8>>;
}
