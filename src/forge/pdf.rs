use crate::engine::{FontManager, ZplForgeBackend};
use crate::forge::png::PngBackend;
use crate::{ZplError, ZplResult};

use flate2::Compression;
use flate2::write::ZlibEncoder;
use image::ImageDecoder;
use image::codecs::png::PngDecoder;
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, Stream, dictionary};
use rayon::prelude::*;
use std::io::{BufWriter, Write};

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

/// Decodes a PNG buffer into zlib-compressed RGB pixels, returning (compressed_bytes, width, height).
type PreparedPage = (Vec<u8>, u32, u32);

fn decode_and_compress_png(png_data: &[u8]) -> Result<PreparedPage, String> {
    let decoder = PngDecoder::new(std::io::Cursor::new(png_data))
        .map_err(|e| format!("Failed to create PNG decoder: {}", e))?;
    let (w, h) = decoder.dimensions();
    let channels = decoder.color_type().channel_count() as usize;
    let mut raw_buf = vec![0u8; decoder.total_bytes() as usize];
    decoder
        .read_image(&mut raw_buf)
        .map_err(|e| format!("Failed to decode PNG: {}", e))?;

    // PngBackend generates RGB (3 channels). If for any reason it's RGBA (4 channels),
    // composite against a white background to produce clean RGB.
    let rgb_buf = if channels == 4 {
        let mut rgb = Vec::with_capacity((w * h * 3) as usize);
        for pixel in raw_buf.chunks_exact(4) {
            let a = pixel[3] as u16;
            let inv_a = 255 - a;
            rgb.push(((pixel[0] as u16 * a + 255 * inv_a) / 255) as u8);
            rgb.push(((pixel[1] as u16 * a + 255 * inv_a) / 255) as u8);
            rgb.push(((pixel[2] as u16 * a + 255 * inv_a) / 255) as u8);
        }
        rgb
    } else {
        // Already RGB — use as-is
        raw_buf
    };

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&rgb_buf)
        .map_err(|e| format!("Failed to compress: {}", e))?;
    let compressed = encoder
        .finish()
        .map_err(|e| format!("Failed to finish compression: {}", e))?;

    Ok((compressed, w, h))
}

/// Builds a complete PDF document from pre-processed page data.
fn build_pdf(
    prepared_pages: &[(Vec<u8>, u32, u32)],
    page_w_pt: f64,
    page_h_pt: f64,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut page_ids: Vec<Object> = Vec::with_capacity(prepared_pages.len());

    for (compressed_pixels, img_w, img_h) in prepared_pages {
        let img_stream = Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => *img_w as i64,
                "Height" => *img_h as i64,
                "ColorSpace" => "DeviceRGB",
                "BitsPerComponent" => 8,
                "Filter" => "FlateDecode",
                "Length" => compressed_pixels.len() as i64,
            },
            compressed_pixels.clone(),
        );
        let img_id = doc.add_object(img_stream);

        let content = Content {
            operations: vec![
                Operation::new("q", vec![]),
                Operation::new(
                    "cm",
                    vec![
                        page_w_pt.into(),
                        0.into(),
                        0.into(),
                        page_h_pt.into(),
                        0.into(),
                        0.into(),
                    ],
                ),
                Operation::new("Do", vec!["Im0".into()]),
                Operation::new("Q", vec![]),
            ],
        };
        let content_bytes = content
            .encode()
            .map_err(|e| format!("Failed to encode content: {}", e))?;
        let content_id = doc.add_object(Stream::new(dictionary! {}, content_bytes));

        let resources = dictionary! {
            "XObject" => dictionary! {
                "Im0" => img_id,
            },
        };

        let page_obj = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![
                0.into(),
                0.into(),
                Object::Real(page_w_pt as f32),
                Object::Real(page_h_pt as f32),
            ],
            "Contents" => content_id,
            "Resources" => resources,
        };
        let page_id = doc.add_object(page_obj);
        page_ids.push(page_id.into());
    }

    let pages_dict = dictionary! {
        "Type" => "Pages",
        "Count" => page_ids.len() as i64,
        "Kids" => page_ids,
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

    let catalog = dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    };
    let catalog_id = doc.add_object(catalog);
    doc.trailer.set("Root", catalog_id);
    doc.compress();

    let mut buf = BufWriter::new(Vec::new());
    doc.save_to(&mut buf)
        .map_err(|e| format!("Failed to save PDF: {}", e))?;

    buf.into_inner()
        .map_err(|e| format!("Failed to flush PDF buffer: {}", e))
}

/// Merges multiple PNG images into a single multi-page PDF document.
///
/// Each PNG in `pages` becomes one page in the resulting PDF.
/// All pages share the same dimensions and resolution.
/// PNG decoding and zlib compression are parallelized across all available CPU cores.
///
/// # Arguments
/// * `pages` - A slice of PNG byte buffers (each element is a complete PNG image).
/// * `width_dots` - The width of each page in dots.
/// * `height_dots` - The height of each page in dots.
/// * `dpi` - The resolution in dots per inch.
///
/// # Example
/// ```rust,no_run
/// use zpl_forge::forge::pdf::png_merge_pages_to_pdf;
/// let png1_bytes: Vec<u8> = vec![]; // PNG bytes from PngBackend
/// let png2_bytes: Vec<u8> = vec![]; // PNG bytes from PngBackend
/// let pdf_bytes = png_merge_pages_to_pdf(&[png1_bytes, png2_bytes], 812.0, 406.0, 203.2).unwrap();
/// ```
pub fn png_merge_pages_to_pdf(
    pages: &[Vec<u8>],
    width_dots: f64,
    height_dots: f64,
    dpi: f32,
) -> ZplResult<Vec<u8>> {
    if pages.is_empty() {
        return Err(ZplError::BackendError("No pages to merge".to_string()));
    }

    let dpi_f64 = if dpi == 0.0 { 203.2 } else { dpi as f64 };
    let page_w_pt = (width_dots / dpi_f64) * 72.0;
    let page_h_pt = (height_dots / dpi_f64) * 72.0;

    // Parallel: decode PNGs and compress to zlib (one thread per CPU core)
    let prepared: Vec<Result<PreparedPage, String>> = pages
        .par_iter()
        .map(|png_data| decode_and_compress_png(png_data))
        .collect();

    // Check for errors
    let prepared: Vec<(Vec<u8>, u32, u32)> = prepared
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(ZplError::BackendError)?;

    // Sequential: assemble the PDF (preserving page order)
    let pdf_bytes = build_pdf(&prepared, page_w_pt, page_h_pt).map_err(ZplError::BackendError)?;

    Ok(pdf_bytes)
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
        png_merge_pages_to_pdf(
            &[png_data],
            self.width_dots,
            self.height_dots,
            self.resolution,
        )
    }
}
