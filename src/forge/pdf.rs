use crate::engine::{FontManager, ZplForgeBackend};
use crate::forge::png::PngBackend;
use crate::{ZplError, ZplResult};
use printpdf::*;

/// A rendering backend that produces PDF documents.
///
/// This backend acts as a wrapper around [`PngBackend`]. It renders the ZPL
/// commands into a high-resolution PNG image first, then embeds that image
/// into a PDF document of the corresponding physical size.
pub struct PdfBackend {
    png_backend: PngBackend,
    width_dots: f64,
    height_dots: f64,
    resolution: f32,
}

impl Default for PdfBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfBackend {
    /// Creates a new `PdfBackend` instance.
    pub fn new() -> Self {
        Self {
            png_backend: PngBackend::new(),
            width_dots: 0.0,
            height_dots: 0.0,
            resolution: 0.0,
        }
    }
}

impl ZplForgeBackend for PdfBackend {
    fn setup_page(&mut self, width: f64, height: f64, resolution: f32) {
        self.width_dots = width;
        self.height_dots = height;
        self.resolution = resolution;
        self.png_backend.setup_page(width, height, resolution);
    }

    fn setup_font_manager(&mut self, font_manager: &FontManager) {
        self.png_backend.setup_font_manager(font_manager);
    }

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
    ) -> ZplResult<()> {
        self.png_backend
            .draw_text(x, y, font, height, width, text, reverse_print, color)
    }

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
    ) -> ZplResult<()> {
        self.png_backend.draw_graphic_box(
            x,
            y,
            width,
            height,
            thickness,
            color,
            custom_color,
            rounding,
            reverse_print,
        )
    }

    fn draw_graphic_circle(
        &mut self,
        x: u32,
        y: u32,
        radius: u32,
        thickness: u32,
        color: char,
        custom_color: Option<String>,
        reverse_print: bool,
    ) -> ZplResult<()> {
        self.png_backend.draw_graphic_circle(
            x,
            y,
            radius,
            thickness,
            color,
            custom_color,
            reverse_print,
        )
    }

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
    ) -> ZplResult<()> {
        self.png_backend.draw_graphic_ellipse(
            x,
            y,
            width,
            height,
            thickness,
            color,
            custom_color,
            reverse_print,
        )
    }

    fn draw_graphic_field(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: Vec<u8>,
        reverse_print: bool,
    ) -> ZplResult<()> {
        self.png_backend
            .draw_graphic_field(x, y, width, height, data, reverse_print)
    }

    fn draw_graphic_image_custom(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: String,
    ) -> ZplResult<()> {
        self.png_backend
            .draw_graphic_image_custom(x, y, width, height, data)
    }

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
    ) -> ZplResult<()> {
        self.png_backend.draw_code128(
            x,
            y,
            orientation,
            height,
            module_width,
            interpretation_line,
            interpretation_line_above,
            check_digit,
            mode,
            data,
            reverse_print,
        )
    }

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
    ) -> ZplResult<()> {
        self.png_backend.draw_qr_code(
            x,
            y,
            orientation,
            model,
            magnification,
            error_correction,
            mask,
            data,
            reverse_print,
        )
    }

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
    ) -> ZplResult<()> {
        self.png_backend.draw_code39(
            x,
            y,
            orientation,
            check_digit,
            height,
            module_width,
            interpretation_line,
            interpretation_line_above,
            data,
            reverse_print,
        )
    }

    fn finalize(&mut self) -> ZplResult<Vec<u8>> {
        let png_data = self.png_backend.finalize()?;

        let dpi = if self.resolution == 0.0 {
            203.2
        } else {
            self.resolution as f64
        };
        let width_pt = (self.width_dots / dpi) * 72.0;
        let height_pt = (self.height_dots / dpi) * 72.0;

        let mut doc = PdfDocument::new("Label");

        // printpdf 0.8 requires collecting warnings manually
        let mut warnings = Vec::new();
        let image = RawImage::decode_from_bytes(&png_data, &mut warnings)
            .map_err(|e| ZplError::BackendError(format!("Failed to decode image: {}", e)))?;

        let image_id = doc.add_image(&image);

        let transform = XObjectTransform {
            translate_x: Some(Pt(0.0)),
            translate_y: Some(Pt(0.0)),
            rotate: None,
            scale_x: None,
            scale_y: None,
            dpi: Some(dpi as f32),
        };

        let op = Op::UseXobject {
            id: image_id,
            transform,
        };

        let page = PdfPage::new(
            Mm::from(Pt(width_pt as f32)),
            Mm::from(Pt(height_pt as f32)),
            vec![op],
        );

        doc.pages.push(page);

        let save_options = PdfSaveOptions::default();
        let pdf_bytes = doc.save(&save_options, &mut warnings);

        Ok(pdf_bytes)
    }
}
