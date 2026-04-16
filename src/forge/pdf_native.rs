//! Native vector PDF rendering backend for ZPL label output.
//!
//! Unlike [`PdfBackend`](super::pdf::PdfBackend), which rasterizes labels to PNG
//! first, this backend renders text, shapes, barcodes and images as native PDF
//! vector operations for maximum quality and minimal file size.

use std::cmp::max;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::sync::Arc;

use ab_glyph::{Font, PxScale, ScaleFont};
use base64::{Engine as _, engine::general_purpose};
use flate2::Compression;
use flate2::write::ZlibEncoder;
use lopdf::content::{Content, Operation};
use lopdf::{Document, FontData, Object, Stream, dictionary};
use rxing::{
    BarcodeFormat, EncodeHintType, EncodeHintValue, EncodeHints, MultiFormatWriter, Writer,
};

use crate::engine::{FontManager, ZplForgeBackend};
use crate::{ZplError, ZplResult};

/// Bézier control-point factor for approximating a quarter-circle arc.
const KAPPA: f64 = 0.5522847498;

// ─── Internal types ─────────────────────────────────────────────────────────

/// Collected image data to be embedded as a PDF XObject during [`PdfNativeBackend::finalize`].
struct ImageXObject {
    name: String,
    data: Vec<u8>,
    width: u32,
    height: u32,
}

// ─── Public struct ──────────────────────────────────────────────────────────

/// A rendering backend that produces PDF documents with native vector operations.
///
/// Text is rendered using an embedded TrueType font, shapes are drawn as PDF
/// paths with Bézier curves, and barcodes are composed of filled rectangles.
/// Bitmap data (graphic fields, custom images) is embedded as compressed
/// XObject image streams.
pub struct PdfNativeBackend {
    width_dots: f64,
    height_dots: f64,
    width_pt: f64,
    height_pt: f64,
    resolution: f32,
    /// `72.0 / dpi` – multiplier that converts dots to PDF points.
    scale: f64,
    operations: Vec<Operation>,
    font_manager: Option<Arc<FontManager>>,
    images: Vec<ImageXObject>,
    image_counter: usize,
    /// Tracks which font identifiers (e.g. 'A', 'B', '0') have been used during rendering.
    used_fonts: HashSet<char>,
    compression: Compression,
}

impl Default for PdfNativeBackend {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Construction ───────────────────────────────────────────────────────────

impl PdfNativeBackend {
    /// Creates a new `PdfNativeBackend` with default settings.
    pub fn new() -> Self {
        Self {
            width_dots: 0.0,
            height_dots: 0.0,
            width_pt: 0.0,
            height_pt: 0.0,
            resolution: 0.0,
            scale: 0.0,
            operations: Vec::new(),
            font_manager: None,
            images: Vec::new(),
            image_counter: 0,
            used_fonts: HashSet::new(),
            compression: Compression::default(),
        }
    }

    /// Sets the zlib compression level for the PDF output (builder pattern).
    pub fn with_compression(mut self, compression: Compression) -> Self {
        self.compression = compression;
        self
    }
}

// ─── Private helpers ────────────────────────────────────────────────────────

impl PdfNativeBackend {
    // ── coordinate helpers ──────────────────────────────────────────

    /// Convert a measurement in dots to PDF points.
    #[inline]
    fn d2pt(&self, dots: f64) -> f64 {
        dots * self.scale
    }

    /// ZPL x-dot → PDF x-point (origin stays at the left).
    #[inline]
    fn x_pt(&self, x: f64) -> f64 {
        x * self.scale
    }

    /// PDF y for the **bottom** edge of an object whose top-left is at ZPL row
    /// `y` with height `h` (both in dots).
    #[inline]
    fn y_pt_bottom(&self, y: f64, h: f64) -> f64 {
        self.height_pt - (y + h) * self.scale
    }

    // ── colour helpers ─────────────────────────────────────────────

    /// Parse `#RRGGBB` / `#RGB` into `(r, g, b)` in 0.0 – 1.0.  Defaults to
    /// black when the string is absent or malformed.
    fn parse_hex_color_f64(color: &Option<String>) -> (f64, f64, f64) {
        if let Some(hex) = color {
            let hex = hex.trim_start_matches('#');
            if hex.len() == 6 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[0..2], 16),
                    u8::from_str_radix(&hex[2..4], 16),
                    u8::from_str_radix(&hex[4..6], 16),
                ) {
                    return (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
                }
            } else if hex.len() == 3
                && let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[0..1], 16),
                    u8::from_str_radix(&hex[1..2], 16),
                    u8::from_str_radix(&hex[2..3], 16),
                )
            {
                return (
                    r as f64 * 17.0 / 255.0,
                    g as f64 * 17.0 / 255.0,
                    b as f64 * 17.0 / 255.0,
                );
            }
        }
        (0.0, 0.0, 0.0)
    }

    /// Resolve the *draw* and *clear* colours for a graphic element.
    ///
    /// Follows the same logic as `PngBackend`:
    /// - custom hex colour → (custom, white)
    /// - `'B'` → (black, white)
    /// - `'W'` → (white, black)
    fn resolve_colors(
        color: char,
        custom_color: &Option<String>,
    ) -> ((f64, f64, f64), (f64, f64, f64)) {
        if custom_color.is_some() {
            (Self::parse_hex_color_f64(custom_color), (1.0, 1.0, 1.0))
        } else if color == 'B' {
            ((0.0, 0.0, 0.0), (1.0, 1.0, 1.0))
        } else {
            ((1.0, 1.0, 1.0), (0.0, 0.0, 0.0))
        }
    }

    // ── low-level PDF operation emitters ────────────────────────────

    fn op(&mut self, operator: &str, operands: Vec<Object>) {
        self.operations.push(Operation::new(operator, operands));
    }

    fn set_fill_color(&mut self, r: f64, g: f64, b: f64) {
        self.op("rg", vec![r.into(), g.into(), b.into()]);
    }

    fn save_state(&mut self) {
        self.op("q", vec![]);
    }

    fn restore_state(&mut self) {
        self.op("Q", vec![]);
    }

    /// Enter *reverse-print* mode: save state, activate the `Difference` blend
    /// mode ExtGState, and set the fill colour to white so that drawing
    /// effectively XOR-inverts the background.
    fn begin_reverse(&mut self) {
        self.save_state();
        self.op("gs", vec!["GSDiff".into()]);
        self.set_fill_color(1.0, 1.0, 1.0);
    }

    fn end_reverse(&mut self) {
        self.restore_state();
    }

    // ── path construction ──────────────────────────────────────────

    /// Append path operators for a rounded rectangle.
    ///
    /// `(x, y)` is the **bottom-left** corner in PDF coordinates; `w` and `h`
    /// extend to the right and upward.
    fn push_rounded_rect_path(&mut self, x: f64, y: f64, w: f64, h: f64, r: f64) {
        let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
        if r < 0.001 {
            self.op("re", vec![x.into(), y.into(), w.into(), h.into()]);
            return;
        }
        let kr = KAPPA * r;
        // bottom-left → right along bottom edge
        self.op("m", vec![(x + r).into(), y.into()]);
        self.op("l", vec![(x + w - r).into(), y.into()]);
        // bottom-right corner
        self.op(
            "c",
            vec![
                (x + w - r + kr).into(),
                y.into(),
                (x + w).into(),
                (y + r - kr).into(),
                (x + w).into(),
                (y + r).into(),
            ],
        );
        // right edge upward
        self.op("l", vec![(x + w).into(), (y + h - r).into()]);
        // top-right corner
        self.op(
            "c",
            vec![
                (x + w).into(),
                (y + h - r + kr).into(),
                (x + w - r + kr).into(),
                (y + h).into(),
                (x + w - r).into(),
                (y + h).into(),
            ],
        );
        // top edge leftward
        self.op("l", vec![(x + r).into(), (y + h).into()]);
        // top-left corner
        self.op(
            "c",
            vec![
                (x + r - kr).into(),
                (y + h).into(),
                x.into(),
                (y + h - r + kr).into(),
                x.into(),
                (y + h - r).into(),
            ],
        );
        // left edge downward
        self.op("l", vec![x.into(), (y + r).into()]);
        // bottom-left corner
        self.op(
            "c",
            vec![
                x.into(),
                (y + r - kr).into(),
                (x + r - kr).into(),
                y.into(),
                (x + r).into(),
                y.into(),
            ],
        );
        self.op("h", vec![]);
    }

    /// Append path operators for an ellipse centred at `(cx, cy)` with radii
    /// `(rx, ry)`, approximated by four cubic Bézier curves.
    fn push_ellipse_path(&mut self, cx: f64, cy: f64, rx: f64, ry: f64) {
        let kx = KAPPA * rx;
        let ky = KAPPA * ry;
        // start at 3-o'clock
        self.op("m", vec![(cx + rx).into(), cy.into()]);
        // → 12-o'clock
        self.op(
            "c",
            vec![
                (cx + rx).into(),
                (cy + ky).into(),
                (cx + kx).into(),
                (cy + ry).into(),
                cx.into(),
                (cy + ry).into(),
            ],
        );
        // → 9-o'clock
        self.op(
            "c",
            vec![
                (cx - kx).into(),
                (cy + ry).into(),
                (cx - rx).into(),
                (cy + ky).into(),
                (cx - rx).into(),
                cy.into(),
            ],
        );
        // → 6-o'clock
        self.op(
            "c",
            vec![
                (cx - rx).into(),
                (cy - ky).into(),
                (cx - kx).into(),
                (cy - ry).into(),
                cx.into(),
                (cy - ry).into(),
            ],
        );
        // → back to 3-o'clock
        self.op(
            "c",
            vec![
                (cx + kx).into(),
                (cy - ry).into(),
                (cx + rx).into(),
                (cy - ky).into(),
                (cx + rx).into(),
                cy.into(),
            ],
        );
        self.op("h", vec![]);
    }

    // ── font / text helpers ────────────────────────────────────────

    fn get_font_arc(&self, font_char: char) -> ZplResult<&ab_glyph::FontArc> {
        let fm = self
            .font_manager
            .as_ref()
            .ok_or_else(|| ZplError::FontError("Font manager not initialized".into()))?;
        fm.get_font(&font_char.to_string())
            .or_else(|| fm.get_font("0"))
            .ok_or_else(|| ZplError::FontError(format!("Font not found: {}", font_char)))
    }

    fn get_text_width(
        &self,
        text: &str,
        font_char: char,
        height: Option<u32>,
        width: Option<u32>,
    ) -> u32 {
        let font = match self.get_font_arc(font_char) {
            Ok(f) => f,
            Err(_) => return 0,
        };
        let scale_y = height.unwrap_or(9) as f32;
        let scale_x = width.unwrap_or(scale_y as u32) as f32;
        let scale = PxScale {
            x: scale_x,
            y: scale_y,
        };
        let scaled = font.as_scaled(scale);
        let mut w = 0.0_f32;
        let mut last_glyph = None;
        for c in text.chars() {
            let gid = font.glyph_id(c);
            if let Some(prev) = last_glyph {
                w += scaled.kern(prev, gid);
            }
            w += scaled.h_advance(gid);
            last_glyph = Some(gid);
        }
        w.ceil() as u32
    }

    // ── image embedding ────────────────────────────────────────────

    /// Store raw RGB image data as a future XObject and emit the `cm` + `Do`
    /// operators that place it on the page.
    fn embed_rgb_image(
        &mut self,
        x_dots: f64,
        y_dots: f64,
        img_w: u32,
        img_h: u32,
        rgb_data: Vec<u8>,
    ) {
        let name = format!("Im{}", self.image_counter);
        self.image_counter += 1;

        let px = self.x_pt(x_dots);
        let py = self.y_pt_bottom(y_dots, img_h as f64);
        let pw = self.d2pt(img_w as f64);
        let ph = self.d2pt(img_h as f64);

        self.save_state();
        self.op(
            "cm",
            vec![
                pw.into(),
                0.into(),
                0.into(),
                ph.into(),
                px.into(),
                py.into(),
            ],
        );
        self.op("Do", vec![Object::Name(name.as_bytes().to_vec())]);
        self.restore_state();

        self.images.push(ImageXObject {
            name,
            data: rgb_data,
            width: img_w,
            height: img_h,
        });
    }

    // ── barcode orientation transforms ─────────────────────────────

    /// Map a local rectangle inside a 1-D barcode to absolute dot coordinates
    /// according to the requested orientation.
    ///
    /// Returns `(abs_x, abs_y, width, height)` – all in dots.
    #[allow(clippy::too_many_arguments)]
    fn transform_1d_bar(
        orientation: char,
        base_x: u32,
        base_y: u32,
        lx: i32,
        ly: i32,
        w: u32,
        h: u32,
        bw: u32,
        bh: u32,
    ) -> (i32, i32, u32, u32) {
        match orientation {
            'R' => {
                let nx = bh as i32 - (ly + h as i32);
                let ny = lx;
                (base_x as i32 + nx, base_y as i32 + ny, h, w)
            }
            'I' => {
                let nx = bw as i32 - (lx + w as i32);
                let ny = bh as i32 - (ly + h as i32);
                (base_x as i32 + nx, base_y as i32 + ny, w, h)
            }
            'B' => {
                let nx = ly;
                let ny = bw as i32 - (lx + w as i32);
                (base_x as i32 + nx, base_y as i32 + ny, h, w)
            }
            _ => (base_x as i32 + lx, base_y as i32 + ly, w, h),
        }
    }

    /// Same as [`Self::transform_1d_bar`] but for 2-D codes (QR).
    #[allow(clippy::too_many_arguments)]
    fn transform_2d_cell(
        orientation: char,
        base_x: u32,
        base_y: u32,
        lx: i32,
        ly: i32,
        w: u32,
        h: u32,
        full_w: u32,
        full_h: u32,
    ) -> (i32, i32, u32, u32) {
        match orientation {
            'R' => {
                let nx = full_h as i32 - (ly + h as i32);
                let ny = lx;
                (base_x as i32 + nx, base_y as i32 + ny, h, w)
            }
            'I' => {
                let nx = full_w as i32 - (lx + w as i32);
                let ny = full_h as i32 - (ly + h as i32);
                (base_x as i32 + nx, base_y as i32 + ny, w, h)
            }
            'B' => {
                let nx = ly;
                let ny = full_w as i32 - (lx + w as i32);
                (base_x as i32 + nx, base_y as i32 + ny, h, w)
            }
            _ => (base_x as i32 + lx, base_y as i32 + ly, w, h),
        }
    }

    // ── 1-D barcode rendering (shared by Code 128 / Code 39) ──────

    #[allow(clippy::too_many_arguments)]
    fn draw_1d_barcode(
        &mut self,
        x: u32,
        y: u32,
        orientation: char,
        height: u32,
        module_width: u32,
        data: &str,
        format: BarcodeFormat,
        reverse_print: bool,
        interpretation_line: char,
        interpretation_line_above: char,
        hints: Option<EncodeHints>,
    ) -> ZplResult<()> {
        let writer = MultiFormatWriter;
        let bit_matrix = if let Some(h) = hints {
            writer.encode_with_hints(data, &format, 0, 0, &h)
        } else {
            writer.encode(data, &format, 0, 0)
        }
        .map_err(|e| ZplError::BackendError(format!("Barcode Generation Error: {}", e)))?;

        let mw = max(module_width, 1);
        let bh = height;
        let bw = bit_matrix.getWidth() * mw;

        let (full_w, full_h) = match orientation {
            'R' | 'B' => (bh, bw),
            _ => (bw, bh),
        };

        // ── emit bar rectangles ────────────────────────────────────
        if reverse_print {
            self.begin_reverse();
        } else {
            self.save_state();
            self.set_fill_color(0.0, 0.0, 0.0);
        }

        for gx in 0..bit_matrix.getWidth() {
            if bit_matrix.get(gx, 0) {
                let (rx, ry, rw, rh) =
                    Self::transform_1d_bar(orientation, x, y, (gx * mw) as i32, 0, mw, bh, bw, bh);
                let px = self.d2pt(rx as f64);
                let py = self.height_pt - self.d2pt(ry as f64 + rh as f64);
                let pw = self.d2pt(rw as f64);
                let ph = self.d2pt(rh as f64);
                self.op("re", vec![px.into(), py.into(), pw.into(), ph.into()]);
            }
        }
        self.op("f", vec![]);

        if reverse_print {
            self.end_reverse();
        } else {
            self.restore_state();
        }

        // ── interpretation line ────────────────────────────────────
        if interpretation_line == 'Y' {
            let font_char = '0';
            let text_h: u32 = 18;
            let text_y = if interpretation_line_above == 'Y' {
                y.saturating_sub(text_h)
            } else {
                y + full_h
            } + 6;

            let text_width = self.get_text_width(data, font_char, Some(text_h), None);
            let text_x = if full_w > text_width {
                x + (full_w - text_width) / 2
            } else {
                x
            };

            self.draw_text(
                text_x,
                text_y,
                font_char,
                Some(text_h),
                None,
                data,
                false,
                None,
            )?;
        }

        Ok(())
    }
}

// ─── ZplForgeBackend ────────────────────────────────────────────────────────

impl ZplForgeBackend for PdfNativeBackend {
    fn setup_page(&mut self, width: f64, height: f64, resolution: f32) {
        let dpi = if resolution == 0.0 { 203.2 } else { resolution };
        self.width_dots = width;
        self.height_dots = height;
        self.resolution = dpi;
        self.scale = 72.0 / dpi as f64;
        self.width_pt = width * self.scale;
        self.height_pt = height * self.scale;
    }

    fn setup_font_manager(&mut self, font_manager: &FontManager) {
        self.font_manager = Some(Arc::new(font_manager.clone()));
    }

    // ── text ───────────────────────────────────────────────────────

    fn draw_text(
        &mut self,
        x: u32,
        y: u32,
        font: char,
        height: Option<u32>,
        width: Option<u32>,
        text: &str,
        reverse_print: bool,
        color: Option<String>,
    ) -> ZplResult<()> {
        if text.is_empty() {
            return Ok(());
        }

        let scale_y_dots = height.unwrap_or(9) as f32;
        let scale_x_dots = width.unwrap_or(scale_y_dots as u32) as f32;
        let px_scale = PxScale {
            x: scale_x_dots,
            y: scale_y_dots,
        };

        // Compute ascent in a scoped borrow so `font_arc` is dropped before the mutable insert.
        let ascent_dots = {
            let font_arc = self.get_font_arc(font)?;
            font_arc.as_scaled(px_scale).ascent()
        };

        self.used_fonts.insert(font);

        let scale_x_pt = self.d2pt(scale_x_dots as f64);
        let scale_y_pt = self.d2pt(scale_y_dots as f64);
        let tx = self.x_pt(x as f64);
        // Baseline position: page_height - (y_top + ascent) * scale
        let ty = self.height_pt - (y as f64 + ascent_dots as f64) * self.scale;

        if reverse_print {
            self.begin_reverse();
        } else {
            let (r, g, b) = Self::parse_hex_color_f64(&color);
            self.save_state();
            self.set_fill_color(r, g, b);
        }

        self.op("BT", vec![]);
        self.op(
            "Tm",
            vec![
                scale_x_pt.into(),
                0.into(),
                0.into(),
                scale_y_pt.into(),
                tx.into(),
                ty.into(),
            ],
        );
        let font_resource_name = format!("F_{}", font);
        self.op(
            "Tf",
            vec![
                Object::Name(font_resource_name.into_bytes()),
                Object::Real(1.0),
            ],
        );
        self.op("Tj", vec![Object::string_literal(text.as_bytes().to_vec())]);
        self.op("ET", vec![]);

        if reverse_print {
            self.end_reverse();
        } else {
            self.restore_state();
        }

        Ok(())
    }

    // ── graphic box (rounded rectangle) ────────────────────────────

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
        let w = max(width, 1) as f64;
        let h = max(height, 1) as f64;
        let t = thickness as f64;
        let r_dots = rounding as f64 * 8.0;

        let (draw_color, clear_color) = Self::resolve_colors(color, &custom_color);

        let bx = self.x_pt(x as f64);
        let by = self.y_pt_bottom(y as f64, h);
        let bw = self.d2pt(w);
        let bh = self.d2pt(h);
        let br = self.d2pt(r_dots);

        if reverse_print {
            // With Difference blend-mode, drawing white inverts the area.
            // Drawing the inner cutout a second time re-inverts it back to
            // the original, leaving only the border ring inverted.
            self.begin_reverse();
            self.push_rounded_rect_path(bx, by, bw, bh, br);
            self.op("f", vec![]);
            if t * 2.0 < w && t * 2.0 < h {
                let tp = self.d2pt(t);
                let inner_r = self.d2pt((r_dots - t).max(0.0));
                self.push_rounded_rect_path(
                    bx + tp,
                    by + tp,
                    bw - tp * 2.0,
                    bh - tp * 2.0,
                    inner_r,
                );
                self.op("f", vec![]);
            }
            self.end_reverse();
        } else {
            self.save_state();
            let (r, g, b) = draw_color;
            self.set_fill_color(r, g, b);
            self.push_rounded_rect_path(bx, by, bw, bh, br);
            self.op("f", vec![]);

            if t * 2.0 < w && t * 2.0 < h {
                let (cr, cg, cb) = clear_color;
                self.set_fill_color(cr, cg, cb);
                let tp = self.d2pt(t);
                let inner_r = self.d2pt((r_dots - t).max(0.0));
                self.push_rounded_rect_path(
                    bx + tp,
                    by + tp,
                    bw - tp * 2.0,
                    bh - tp * 2.0,
                    inner_r,
                );
                self.op("f", vec![]);
            }
            self.restore_state();
        }

        Ok(())
    }

    // ── graphic circle ─────────────────────────────────────────────

    fn draw_graphic_circle(
        &mut self,
        x: u32,
        y: u32,
        radius: u32,
        thickness: u32,
        _color: char,
        custom_color: Option<String>,
        reverse_print: bool,
    ) -> ZplResult<()> {
        let (draw_color, _) = Self::resolve_colors('B', &custom_color);

        let r_pt = self.d2pt(radius as f64);
        // ZPL (x,y) = top-left of bounding box → centre
        let cx_pt = self.x_pt(x as f64) + r_pt;
        let cy_pt = self.height_pt - (y as f64 + radius as f64) * self.scale;

        if reverse_print {
            self.begin_reverse();
            self.push_ellipse_path(cx_pt, cy_pt, r_pt, r_pt);
            self.op("f", vec![]);
            if radius > thickness {
                let inner_r = self.d2pt((radius - thickness) as f64);
                self.push_ellipse_path(cx_pt, cy_pt, inner_r, inner_r);
                self.op("f", vec![]);
            }
            self.end_reverse();
        } else {
            self.save_state();
            let (r, g, b) = draw_color;
            self.set_fill_color(r, g, b);
            self.push_ellipse_path(cx_pt, cy_pt, r_pt, r_pt);
            self.op("f", vec![]);

            if radius > thickness {
                self.set_fill_color(1.0, 1.0, 1.0);
                let inner_r = self.d2pt((radius - thickness) as f64);
                self.push_ellipse_path(cx_pt, cy_pt, inner_r, inner_r);
                self.op("f", vec![]);
            }
            self.restore_state();
        }

        Ok(())
    }

    // ── graphic ellipse ────────────────────────────────────────────

    fn draw_graphic_ellipse(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        thickness: u32,
        _color: char,
        custom_color: Option<String>,
        reverse_print: bool,
    ) -> ZplResult<()> {
        let (draw_color, _) = Self::resolve_colors('B', &custom_color);

        let rx_pt = self.d2pt(width as f64 / 2.0);
        let ry_pt = self.d2pt(height as f64 / 2.0);
        let cx_pt = self.x_pt(x as f64) + rx_pt;
        let cy_pt = self.height_pt - (y as f64 + height as f64 / 2.0) * self.scale;

        let t = thickness as f64;

        if reverse_print {
            self.begin_reverse();
            self.push_ellipse_path(cx_pt, cy_pt, rx_pt, ry_pt);
            self.op("f", vec![]);
            if (width as f64 / 2.0) > t && (height as f64 / 2.0) > t {
                let irx = self.d2pt(width as f64 / 2.0 - t);
                let iry = self.d2pt(height as f64 / 2.0 - t);
                self.push_ellipse_path(cx_pt, cy_pt, irx, iry);
                self.op("f", vec![]);
            }
            self.end_reverse();
        } else {
            self.save_state();
            let (r, g, b) = draw_color;
            self.set_fill_color(r, g, b);
            self.push_ellipse_path(cx_pt, cy_pt, rx_pt, ry_pt);
            self.op("f", vec![]);

            if (width as f64 / 2.0) > t && (height as f64 / 2.0) > t {
                self.set_fill_color(1.0, 1.0, 1.0);
                let irx = self.d2pt(width as f64 / 2.0 - t);
                let iry = self.d2pt(height as f64 / 2.0 - t);
                self.push_ellipse_path(cx_pt, cy_pt, irx, iry);
                self.op("f", vec![]);
            }
            self.restore_state();
        }

        Ok(())
    }

    // ── graphic field (1-bit bitmap) ───────────────────────────────

    fn draw_graphic_field(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: &[u8],
        _reverse_print: bool,
    ) -> ZplResult<()> {
        if width == 0 || height == 0 {
            return Ok(());
        }

        let row_bytes = width.div_ceil(8) as usize;
        let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);

        for row_idx in 0..height {
            let row_start = row_idx as usize * row_bytes;
            let row_end = (row_start + row_bytes).min(data.len());
            let row_data = if row_start < data.len() {
                &data[row_start..row_end]
            } else {
                &[]
            };

            for col in 0..width {
                let byte_idx = (col / 8) as usize;
                let bit_idx = 7 - (col % 8);
                let is_set =
                    byte_idx < row_data.len() && (row_data[byte_idx] & (1 << bit_idx)) != 0;
                if is_set {
                    rgb_data.extend_from_slice(&[0, 0, 0]);
                } else {
                    rgb_data.extend_from_slice(&[255, 255, 255]);
                }
            }
        }

        self.embed_rgb_image(x as f64, y as f64, width, height, rgb_data);
        Ok(())
    }

    // ── custom colour image (base64) ───────────────────────────────

    fn draw_graphic_image_custom(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: &str,
    ) -> ZplResult<()> {
        let image_data = general_purpose::STANDARD
            .decode(data.trim())
            .map_err(|e| ZplError::ImageError(format!("Failed to decode base64: {}", e)))?;

        let img = image::load_from_memory(&image_data)
            .map_err(|e| ZplError::ImageError(format!("Failed to load image: {}", e)))?
            .to_rgb8();

        let (orig_w, orig_h) = img.dimensions();
        let (target_w, target_h) = match (width, height) {
            (0, 0) => (orig_w, orig_h),
            (w, 0) => {
                let h = (orig_h as f32 * (w as f32 / orig_w as f32)).round() as u32;
                (w, h)
            }
            (0, h) => {
                let w = (orig_w as f32 * (h as f32 / orig_h as f32)).round() as u32;
                (w, h)
            }
            (w, h) => (w, h),
        };

        let final_img = if target_w != orig_w || target_h != orig_h {
            image::imageops::resize(
                &img,
                target_w,
                target_h,
                image::imageops::FilterType::Lanczos3,
            )
        } else {
            img
        };

        let rgb_data = final_img.into_raw();
        self.embed_rgb_image(x as f64, y as f64, target_w, target_h, rgb_data);
        Ok(())
    }

    // ── Code 128 barcode ───────────────────────────────────────────

    fn draw_code128(
        &mut self,
        x: u32,
        y: u32,
        orientation: char,
        height: u32,
        module_width: u32,
        interpretation_line: char,
        interpretation_line_above: char,
        _check_digit: char,
        _mode: char,
        data: &str,
        reverse_print: bool,
    ) -> ZplResult<()> {
        let (clean_data, hint_val) = if let Some(stripped) = data.strip_prefix(">:") {
            (stripped, Some("B"))
        } else if let Some(stripped) = data.strip_prefix(">;") {
            (stripped, Some("C"))
        } else if let Some(stripped) = data.strip_prefix(">9") {
            (stripped, Some("A"))
        } else {
            (data, None)
        };

        let hints = hint_val.map(|v| {
            let mut h = HashMap::new();
            h.insert(
                EncodeHintType::FORCE_CODE_SET,
                EncodeHintValue::ForceCodeSet(v.to_string()),
            );
            EncodeHints::from(h)
        });

        self.draw_1d_barcode(
            x,
            y,
            orientation,
            height,
            module_width,
            clean_data,
            BarcodeFormat::CODE_128,
            reverse_print,
            interpretation_line,
            interpretation_line_above,
            hints,
        )
    }

    // ── QR code ────────────────────────────────────────────────────

    fn draw_qr_code(
        &mut self,
        x: u32,
        y: u32,
        orientation: char,
        _model: u32,
        magnification: u32,
        error_correction: char,
        _mask: u32,
        data: &str,
        reverse_print: bool,
    ) -> ZplResult<()> {
        let level = match error_correction {
            'L' => "L",
            'M' => "M",
            'Q' => "Q",
            'H' => "H",
            _ => "M",
        };

        let mut hints = HashMap::new();
        hints.insert(
            EncodeHintType::ERROR_CORRECTION,
            EncodeHintValue::ErrorCorrection(level.to_string()),
        );
        hints.insert(
            EncodeHintType::MARGIN,
            EncodeHintValue::Margin("0".to_owned()),
        );
        let hints: EncodeHints = hints.into();

        let writer = MultiFormatWriter;
        let bit_matrix = writer
            .encode_with_hints(data, &BarcodeFormat::QR_CODE, 0, 0, &hints)
            .map_err(|e| ZplError::BackendError(format!("QR Generation Error: {}", e)))?;

        let mag = max(magnification, 1);
        let bw = bit_matrix.getWidth();
        let bh = bit_matrix.getHeight();
        let full_w = bw * mag;
        let full_h = bh * mag;

        if reverse_print {
            self.begin_reverse();
        } else {
            self.save_state();
            self.set_fill_color(0.0, 0.0, 0.0);
        }

        for gy in 0..bh {
            for gx in 0..bw {
                if bit_matrix.get(gx, gy) {
                    let (rx, ry, rw, rh) = Self::transform_2d_cell(
                        orientation,
                        x,
                        y,
                        (gx * mag) as i32,
                        (gy * mag) as i32,
                        mag,
                        mag,
                        full_w,
                        full_h,
                    );
                    let px = self.d2pt(rx as f64);
                    let py = self.height_pt - self.d2pt(ry as f64 + rh as f64);
                    let pw = self.d2pt(rw as f64);
                    let ph = self.d2pt(rh as f64);
                    self.op("re", vec![px.into(), py.into(), pw.into(), ph.into()]);
                }
            }
        }
        self.op("f", vec![]);

        if reverse_print {
            self.end_reverse();
        } else {
            self.restore_state();
        }

        Ok(())
    }

    // ── Code 39 barcode ────────────────────────────────────────────

    fn draw_code39(
        &mut self,
        x: u32,
        y: u32,
        orientation: char,
        _check_digit: char,
        height: u32,
        module_width: u32,
        interpretation_line: char,
        interpretation_line_above: char,
        data: &str,
        reverse_print: bool,
    ) -> ZplResult<()> {
        self.draw_1d_barcode(
            x,
            y,
            orientation,
            height,
            module_width,
            data,
            BarcodeFormat::CODE_39,
            reverse_print,
            interpretation_line,
            interpretation_line_above,
            None,
        )
    }

    // ── finalize ───────────────────────────────────────────────────

    fn finalize(&mut self) -> ZplResult<Vec<u8>> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();

        // ── embed fonts ────────────────────────────────────────────
        let default_font_bytes: &[u8] = include_bytes!("../../assets/Oswald-Regular.ttf");
        let mut font_dict = lopdf::Dictionary::new();
        let mut embedded_fonts: HashSet<String> = HashSet::new();

        for font_char in &self.used_fonts {
            let font_key = font_char.to_string();
            let resource_name = format!("F_{}", font_char);

            // Get the font name to deduplicate (multiple chars may map to same font)
            let font_name = self
                .font_manager
                .as_ref()
                .and_then(|fm| fm.get_font_name(&font_key).map(|s| s.to_string()));

            let actual_name = font_name.unwrap_or_else(|| "Oswald".to_string());

            // Skip if we already embedded this font under a different char
            // but still add the resource alias
            if embedded_fonts.contains(&actual_name) {
                // Find the already-embedded font id by looking through font_dict
                // Simpler: just embed again (lopdf handles dedup at compression)
            }

            let raw_bytes = self
                .font_manager
                .as_ref()
                .and_then(|fm| fm.get_font_bytes(&font_key))
                .unwrap_or(default_font_bytes);

            let font_data = FontData::new(raw_bytes, actual_name.clone());
            let font_id = doc
                .add_font(font_data)
                .map_err(|e| ZplError::BackendError(format!("Failed to embed font: {}", e)))?;

            font_dict.set(resource_name.as_str(), font_id);
            embedded_fonts.insert(actual_name);
        }

        // ── XObject images ─────────────────────────────────────────
        let mut xobject_dict = lopdf::Dictionary::new();
        for img in &self.images {
            let mut encoder = ZlibEncoder::new(Vec::new(), self.compression);
            encoder
                .write_all(&img.data)
                .map_err(|e| ZplError::BackendError(e.to_string()))?;
            let compressed = encoder
                .finish()
                .map_err(|e| ZplError::BackendError(e.to_string()))?;

            let img_stream = Stream::new(
                dictionary! {
                    "Type" => "XObject",
                    "Subtype" => "Image",
                    "Width" => img.width as i64,
                    "Height" => img.height as i64,
                    "ColorSpace" => "DeviceRGB",
                    "BitsPerComponent" => 8,
                    "Filter" => "FlateDecode",
                },
                compressed,
            );
            let img_id = doc.add_object(img_stream);
            xobject_dict.set(img.name.as_str(), img_id);
        }

        // ── ExtGState for reverse-print blend modes ────────────────
        let mut gs_dict = lopdf::Dictionary::new();
        let gs_diff = doc.add_object(dictionary! {
            "Type" => "ExtGState",
            "BM" => "Difference",
        });
        gs_dict.set("GSDiff", gs_diff);

        let gs_normal = doc.add_object(dictionary! {
            "Type" => "ExtGState",
            "BM" => "Normal",
        });
        gs_dict.set("GSNormal", gs_normal);

        // ── resources ──────────────────────────────────────────────
        let resources_id = doc.add_object(dictionary! {
            "Font" => lopdf::Object::Dictionary(font_dict),
            "XObject" => lopdf::Object::Dictionary(xobject_dict),
            "ExtGState" => lopdf::Object::Dictionary(gs_dict),
        });

        // ── content stream ─────────────────────────────────────────
        let content = Content {
            operations: std::mem::take(&mut self.operations),
        };
        let content_bytes = content
            .encode()
            .map_err(|e| ZplError::BackendError(format!("Failed to encode content: {}", e)))?;
        let content_id = doc.add_object(Stream::new(dictionary! {}, content_bytes));

        // ── page ───────────────────────────────────────────────────
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![
                0.into(),
                0.into(),
                Object::Real(self.width_pt as f32),
                Object::Real(self.height_pt as f32),
            ],
            "Contents" => content_id,
            "Resources" => resources_id,
        });

        // ── pages tree ─────────────────────────────────────────────
        let pages_dict = dictionary! {
            "Type" => "Pages",
            "Count" => 1_i64,
            "Kids" => vec![page_id.into()],
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

        // ── catalogue ──────────────────────────────────────────────
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        doc.compress();

        // ── serialize ──────────────────────────────────────────────
        let mut buf = std::io::BufWriter::new(Vec::new());
        doc.save_to(&mut buf)
            .map_err(|e| ZplError::BackendError(format!("Failed to save PDF: {}", e)))?;
        buf.into_inner()
            .map_err(|e| ZplError::BackendError(format!("Failed to flush: {}", e)))
    }
}
