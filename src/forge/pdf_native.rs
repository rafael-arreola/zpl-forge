//! Native vector PDF rendering backend for ZPL label output.
//!
//! This backend renders text, shapes, barcodes and images as native PDF
//! vector operations for maximum quality and minimal file size.

use std::cmp::max;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::sync::Arc;

use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use base64::{Engine as _, engine::general_purpose};
use flate2::Compression;
use flate2::write::ZlibEncoder;
use lopdf::{Document, FontData, Object, Stream, dictionary};
use rxing::common::BitMatrix;
use rxing::{BarcodeFormat, EncodeHintType, EncodeHintValue, EncodeHints};

use super::{barcode_1d_format, barcode_cache};
use crate::engine::{Barcode1DKind, FontManager, ZplForgeBackend};
use crate::{ZplError, ZplResult};

/// Bézier control-point factor for approximating a quarter-circle arc.
const KAPPA: f64 = 0.5522847498;

// ─── WinAnsi (CP1252) encoding ──────────────────────────────────────────────
//
// Embedded fonts are declared with /Encoding WinAnsiEncoding, so text shown
// with `Tj` must be CP1252 bytes — not UTF-8. This is what makes accented
// characters (ñ, á, é...) render and copy correctly.

/// Unicode characters for CP1252 codes 0x80..=0x9F (`\u{0}` = undefined).
const CP1252_80_9F: [char; 32] = [
    '\u{20AC}', '\u{0}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{0}', '\u{017D}', '\u{0}',
    '\u{0}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{0}', '\u{017E}', '\u{0178}',
];

/// Encodes a Unicode char to its CP1252 byte, when representable.
fn char_to_winansi(c: char) -> Option<u8> {
    let cp = c as u32;
    match cp {
        0x20..=0x7E => Some(cp as u8),
        // CP1252 0xA0..=0xFF is identical to Latin-1.
        0xA0..=0xFF => Some(cp as u8),
        _ => CP1252_80_9F
            .iter()
            .position(|&m| m == c && m != '\u{0}')
            .map(|i| 0x80 + i as u8),
    }
}

/// Decodes a CP1252 byte back to its Unicode char, when defined.
fn winansi_to_char(code: u8) -> Option<char> {
    match code {
        0x20..=0x7E => Some(code as char),
        0xA0..=0xFF => Some(code as char),
        0x80..=0x9F => {
            let c = CP1252_80_9F[(code - 0x80) as usize];
            (c != '\u{0}').then_some(c)
        }
        _ => None,
    }
}

/// Builds a ToUnicode CMap stream body for the WinAnsi code range.
fn build_tounicode_cmap() -> Vec<u8> {
    let mut s = String::with_capacity(4096);
    s.push_str(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
         1 begincodespacerange\n<20> <FF>\nendcodespacerange\n",
    );
    let entries: Vec<(u8, char)> = (0x20..=0xFFu32)
        .filter_map(|c| winansi_to_char(c as u8).map(|ch| (c as u8, ch)))
        .collect();
    for chunk in entries.chunks(100) {
        s.push_str(&format!("{} beginbfchar\n", chunk.len()));
        for (code, ch) in chunk {
            s.push_str(&format!("<{:02X}> <{:04X}>\n", code, *ch as u32));
        }
        s.push_str("endbfchar\n");
    }
    s.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    s.into_bytes()
}

// ─── Internal types ─────────────────────────────────────────────────────────

/// Collected image data to be embedded as a PDF XObject during [`PdfNativeBackend::finalize`].
struct ImageXObject {
    name: String,
    data: Vec<u8>,
    width: u32,
    height: u32,
    /// `true` for 1-bit stencil masks (`^GF` bitmaps), `false` for 8-bit RGB.
    is_mask: bool,
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
    /// Raw PDF content-stream bytes for the page currently being drawn.
    content: Vec<u8>,
    /// Content streams of pages already finished via [`ZplForgeBackend::new_page`].
    finished_pages: Vec<Vec<u8>>,
    font_manager: Option<Arc<FontManager>>,
    images: Vec<ImageXObject>,
    image_counter: usize,
    /// Tracks which font identifiers (e.g. 'A', 'B', '0') have been used during rendering.
    used_fonts: HashSet<char>,
    compression: Compression,
    /// Optional document title for the PDF Info dictionary.
    title: Option<String>,
    /// Solid rectangles painted on the current page, in dots, with their
    /// fill colour. Used to compute `^FR` (reverse print) geometrically —
    /// blend modes are unreliable across viewers and print RIPs.
    backdrop_rects: Vec<(f64, f64, f64, f64, (f64, f64, f64))>,
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
            content: Vec::with_capacity(4096),
            finished_pages: Vec::new(),
            font_manager: None,
            images: Vec::new(),
            image_counter: 0,
            used_fonts: HashSet::new(),
            compression: Compression::default(),
            title: None,
            backdrop_rects: Vec::new(),
        }
    }

    /// Sets the zlib compression level for the PDF output (builder pattern).
    pub fn with_compression(mut self, compression: Compression) -> Self {
        self.compression = compression;
        self
    }

    /// Sets the document title written to the PDF Info dictionary (builder pattern).
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
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
    //
    // Operators are written directly as content-stream bytes instead of
    // accumulating `lopdf::content::Operation` values: barcodes and QR codes
    // emit thousands of `re` rectangles, and the per-operation allocations
    // dominated render time.

    /// Write a number with up to 3 decimals, trimming trailing zeros.
    fn put_num(buf: &mut Vec<u8>, v: f64) {
        if v == v.trunc() && v.abs() < 1e12 {
            let mut itoa = [0u8; 20];
            let mut n = v as i64;
            if n < 0 {
                buf.push(b'-');
                n = -n;
            }
            let mut i = itoa.len();
            loop {
                i -= 1;
                itoa[i] = b'0' + (n % 10) as u8;
                n /= 10;
                if n == 0 {
                    break;
                }
            }
            buf.extend_from_slice(&itoa[i..]);
        } else {
            let mut s = format!("{:.3}", v);
            while s.ends_with('0') {
                s.pop();
            }
            if s.ends_with('.') {
                s.pop();
            }
            buf.extend_from_slice(s.as_bytes());
        }
    }

    /// Emit `n1 n2 ... op\n`.
    fn emit_nums(&mut self, nums: &[f64], op: &str) {
        for n in nums {
            Self::put_num(&mut self.content, *n);
            self.content.push(b' ');
        }
        self.content.extend_from_slice(op.as_bytes());
        self.content.push(b'\n');
    }

    /// Emit a bare operator: `op\n`.
    fn emit_op(&mut self, op: &str) {
        self.content.extend_from_slice(op.as_bytes());
        self.content.push(b'\n');
    }

    /// Emit `/Name op\n`.
    fn emit_name_op(&mut self, name: &str, op: &str) {
        self.content.push(b'/');
        self.content.extend_from_slice(name.as_bytes());
        self.content.push(b' ');
        self.content.extend_from_slice(op.as_bytes());
        self.content.push(b'\n');
    }

    /// Emit `(escaped) Tj\n`, encoding the text as WinAnsi (CP1252) to match
    /// the embedded fonts' /Encoding. Unmappable characters become '?'.
    fn emit_tj(&mut self, text: &str) {
        self.content.push(b'(');
        for c in text.chars() {
            let b = char_to_winansi(c).unwrap_or(b'?');
            match b {
                b'(' | b')' | b'\\' => {
                    self.content.push(b'\\');
                    self.content.push(b);
                }
                _ => self.content.push(b),
            }
        }
        self.content.extend_from_slice(b") Tj\n");
    }

    fn set_fill_color(&mut self, r: f64, g: f64, b: f64) {
        self.emit_nums(&[r, g, b], "rg");
    }

    fn save_state(&mut self) {
        self.emit_op("q");
    }

    fn restore_state(&mut self) {
        self.emit_op("Q");
    }

    // ── reverse-print (geometric) ──────────────────────────────────
    //
    // ZPL `^FR` inverts the element against whatever lies beneath it. Instead
    // of relying on the `Difference` blend mode (poorly supported by Quartz/
    // Preview and ignored by many print RIPs), the backend tracks the solid
    // rectangles already painted and repaints their inverse inside a clip
    // shaped like the reversed element.

    /// Records a solid filled rectangle (in dots) as part of the backdrop.
    fn track_backdrop_rect(&mut self, x: f64, y: f64, w: f64, h: f64, color: (f64, f64, f64)) {
        if w > 0.0 && h > 0.0 {
            self.backdrop_rects.push((x, y, w, h, color));
        }
    }

    /// Topmost backdrop colour at a point (in dots); white when unpainted.
    fn backdrop_color_at(&self, px: f64, py: f64) -> (f64, f64, f64) {
        let mut color = (1.0, 1.0, 1.0);
        for (rx, ry, rw, rh, c) in &self.backdrop_rects {
            if px >= *rx && px < rx + rw && py >= *ry && py < ry + rh {
                color = *c;
            }
        }
        color
    }

    /// Paints the inverse of the backdrop across the element bounding box
    /// `(ex, ey, ew, eh)` in dots. The caller must have already established a
    /// clipping path shaped like the reversed element.
    fn fill_inverse_backdrop(&mut self, ex: f64, ey: f64, ew: f64, eh: f64) {
        // Unpainted page is white → its inverse is black.
        self.set_fill_color(0.0, 0.0, 0.0);
        let px = self.x_pt(ex);
        let py = self.y_pt_bottom(ey, eh);
        let (pw, ph) = (self.d2pt(ew), self.d2pt(eh));
        self.emit_nums(&[px, py, pw, ph], "re");
        self.emit_op("f");

        // Repaint intersections with tracked rects using their inverse, in
        // z-order so later fills win exactly like the original painting did.
        let rects = self.backdrop_rects.clone();
        for (rx, ry, rw, rh, (cr, cg, cb)) in rects {
            let ix0 = rx.max(ex);
            let iy0 = ry.max(ey);
            let ix1 = (rx + rw).min(ex + ew);
            let iy1 = (ry + rh).min(ey + eh);
            if ix1 > ix0 && iy1 > iy0 {
                self.set_fill_color(1.0 - cr, 1.0 - cg, 1.0 - cb);
                let px = self.x_pt(ix0);
                let py = self.y_pt_bottom(iy0, iy1 - iy0);
                self.emit_nums(&[px, py, self.d2pt(ix1 - ix0), self.d2pt(iy1 - iy0)], "re");
                self.emit_op("f");
            }
        }
    }

    // ── path construction ──────────────────────────────────────────

    /// Append path operators for a rounded rectangle.
    ///
    /// `(x, y)` is the **bottom-left** corner in PDF coordinates; `w` and `h`
    /// extend to the right and upward.
    fn push_rounded_rect_path(&mut self, x: f64, y: f64, w: f64, h: f64, r: f64) {
        let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
        if r < 0.001 {
            self.emit_nums(&[x, y, w, h], "re");
            return;
        }
        let kr = KAPPA * r;
        // bottom-left → right along bottom edge
        self.emit_nums(&[x + r, y], "m");
        self.emit_nums(&[x + w - r, y], "l");
        // bottom-right corner
        self.emit_nums(&[x + w - r + kr, y, x + w, y + r - kr, x + w, y + r], "c");
        // right edge upward
        self.emit_nums(&[x + w, y + h - r], "l");
        // top-right corner
        self.emit_nums(
            &[
                x + w,
                y + h - r + kr,
                x + w - r + kr,
                y + h,
                x + w - r,
                y + h,
            ],
            "c",
        );
        // top edge leftward
        self.emit_nums(&[x + r, y + h], "l");
        // top-left corner
        self.emit_nums(&[x + r - kr, y + h, x, y + h - r + kr, x, y + h - r], "c");
        // left edge downward
        self.emit_nums(&[x, y + r], "l");
        // bottom-left corner
        self.emit_nums(&[x, y + r - kr, x + r - kr, y, x + r, y], "c");
        self.emit_op("h");
    }

    /// Append path operators for an ellipse centred at `(cx, cy)` with radii
    /// `(rx, ry)`, approximated by four cubic Bézier curves.
    fn push_ellipse_path(&mut self, cx: f64, cy: f64, rx: f64, ry: f64) {
        let kx = KAPPA * rx;
        let ky = KAPPA * ry;
        // start at 3-o'clock
        self.emit_nums(&[cx + rx, cy], "m");
        // → 12-o'clock
        self.emit_nums(&[cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry], "c");
        // → 9-o'clock
        self.emit_nums(&[cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy], "c");
        // → 6-o'clock
        self.emit_nums(&[cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry], "c");
        // → back to 3-o'clock
        self.emit_nums(&[cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy], "c");
        self.emit_op("h");
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
        self.emit_nums(&[pw, 0.0, 0.0, ph, px, py], "cm");
        self.emit_name_op(&name, "Do");
        self.restore_state();

        self.images.push(ImageXObject {
            name,
            data: rgb_data,
            width: img_w,
            height: img_h,
            is_mask: false,
        });
    }

    /// Store 1-bit bitmap data as a future stencil-mask XObject and emit the
    /// operators that paint it on the page. Set bits (ZPL black) are painted
    /// with the current fill colour; clear bits are transparent.
    fn embed_mask_image(
        &mut self,
        x_dots: f64,
        y_dots: f64,
        img_w: u32,
        img_h: u32,
        bits: Vec<u8>,
        reverse_print: bool,
    ) {
        let name = format!("Im{}", self.image_counter);
        self.image_counter += 1;

        let px = self.x_pt(x_dots);
        let py = self.y_pt_bottom(y_dots, img_h as f64);
        let pw = self.d2pt(img_w as f64);
        let ph = self.d2pt(img_h as f64);

        self.save_state();
        if reverse_print {
            // Stencil masks can't be clipped per-pixel without SMasks, so
            // approximate: paint with the inverse of the backdrop colour at
            // the bitmap centre.
            let (br, bg, bb) =
                self.backdrop_color_at(x_dots + img_w as f64 / 2.0, y_dots + img_h as f64 / 2.0);
            self.set_fill_color(1.0 - br, 1.0 - bg, 1.0 - bb);
        } else {
            self.set_fill_color(0.0, 0.0, 0.0);
        }
        self.emit_nums(&[pw, 0.0, 0.0, ph, px, py], "cm");
        self.emit_name_op(&name, "Do");
        self.restore_state();

        self.images.push(ImageXObject {
            name,
            data: bits,
            width: img_w,
            height: img_h,
            is_mask: true,
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
        hints_key: &str,
    ) -> ZplResult<()> {
        let bit_matrix = barcode_cache::encode_cached(format, data, hints_key, hints.as_ref())?;

        let mw = max(module_width, 1);
        let bh = height;
        let bw = bit_matrix.getWidth() * mw;

        let (full_w, full_h) = match orientation {
            'R' | 'B' => (bh, bw),
            _ => (bw, bh),
        };

        // ── emit bar rectangles ────────────────────────────────────
        self.save_state();
        if !reverse_print {
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
                self.emit_nums(&[px, py, pw, ph], "re");
            }
        }
        if reverse_print {
            // Use the bars as a clip and invert the backdrop inside them.
            self.emit_op("W");
            self.emit_op("n");
            self.fill_inverse_backdrop(x as f64, y as f64, full_w as f64, full_h as f64);
        } else {
            self.emit_op("f");
        }
        self.restore_state();

        // ── interpretation line ────────────────────────────────────
        if interpretation_line == 'Y' {
            self.draw_interpretation_line(x, y, full_w, full_h, data, interpretation_line_above)?;
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_interpretation_line(
        &mut self,
        x: u32,
        y: u32,
        full_w: u32,
        full_h: u32,
        data: &str,
        interpretation_line_above: char,
    ) -> ZplResult<()> {
        {
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
                'N',
                data,
                false,
                None,
            )?;
        }

        Ok(())
    }

    /// Paints every set cell of a 2-D bit matrix as a filled rectangle,
    /// scaling each cell to `cell_w` × `cell_h` dots and applying the
    /// requested orientation.
    #[allow(clippy::too_many_arguments)]
    fn fill_matrix_cells(
        &mut self,
        x: u32,
        y: u32,
        orientation: char,
        cell_w: u32,
        cell_h: u32,
        bit_matrix: &BitMatrix,
        reverse_print: bool,
    ) {
        let bw = bit_matrix.getWidth();
        let bh = bit_matrix.getHeight();
        let full_w = bw * cell_w;
        let full_h = bh * cell_h;

        self.save_state();
        if !reverse_print {
            self.set_fill_color(0.0, 0.0, 0.0);
        }

        for gy in 0..bh {
            for gx in 0..bw {
                if bit_matrix.get(gx, gy) {
                    let (rx, ry, rw, rh) = Self::transform_2d_cell(
                        orientation,
                        x,
                        y,
                        (gx * cell_w) as i32,
                        (gy * cell_h) as i32,
                        cell_w,
                        cell_h,
                        full_w,
                        full_h,
                    );
                    let px = self.d2pt(rx as f64);
                    let py = self.height_pt - self.d2pt(ry as f64 + rh as f64);
                    let pw = self.d2pt(rw as f64);
                    let ph = self.d2pt(rh as f64);
                    self.emit_nums(&[px, py, pw, ph], "re");
                }
            }
        }
        if reverse_print {
            self.emit_op("W");
            self.emit_op("n");
            let (fw, fh) = match orientation {
                'R' | 'B' => (full_h, full_w),
                _ => (full_w, full_h),
            };
            self.fill_inverse_backdrop(x as f64, y as f64, fw as f64, fh as f64);
        } else {
            self.emit_op("f");
        }
        self.restore_state();
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

    fn new_page(&mut self) -> ZplResult<()> {
        self.finished_pages.push(std::mem::take(&mut self.content));
        self.backdrop_rects.clear();
        Ok(())
    }

    // ── text ───────────────────────────────────────────────────────

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
        } as f64;

        self.used_fonts.insert(font);

        let scale_x_pt = self.d2pt(scale_x_dots as f64);
        let scale_y_pt = self.d2pt(scale_y_dots as f64);
        let h_dots = scale_y_dots as f64;
        let x = x as f64;
        let y = y as f64;

        // Text width anchors 'I'/'B' rotations and sizes the reverse bbox.
        let tw_dots = if reverse_print || orientation == 'I' || orientation == 'B' {
            self.get_text_width(text, font, height, width) as f64
        } else {
            0.0
        };

        // Text matrix [a b c d tx ty]: scale plus the ^A rotation, with
        // (x, y) anchoring the top-left corner of the rotated cell.
        let tm = match orientation {
            'R' => [
                0.0,
                -scale_x_pt,
                scale_y_pt,
                0.0,
                self.x_pt(x + h_dots - ascent_dots),
                self.height_pt - y * self.scale,
            ],
            'I' => [
                -scale_x_pt,
                0.0,
                0.0,
                -scale_y_pt,
                self.x_pt(x + tw_dots),
                self.height_pt - (y + h_dots - ascent_dots) * self.scale,
            ],
            'B' => [
                0.0,
                scale_x_pt,
                -scale_y_pt,
                0.0,
                self.x_pt(x + ascent_dots),
                self.height_pt - (y + tw_dots) * self.scale,
            ],
            _ => [
                scale_x_pt,
                0.0,
                0.0,
                scale_y_pt,
                self.x_pt(x),
                self.height_pt - (y + ascent_dots) * self.scale,
            ],
        };

        self.save_state();
        if !reverse_print {
            let (r, g, b) = Self::parse_hex_color_f64(&color);
            self.set_fill_color(r, g, b);
        }

        self.emit_op("BT");
        if reverse_print {
            // Text rendering mode 7: glyph outlines become the clipping path.
            self.emit_nums(&[7.0], "Tr");
        }
        self.emit_nums(&tm, "Tm");
        let font_resource_name = format!("F_{}", font);
        self.emit_name_op(&format!("{} 1", font_resource_name), "Tf");
        self.emit_tj(text);
        self.emit_op("ET");

        if reverse_print {
            let (bw_dots, bh_dots) = match orientation {
                'R' | 'B' => (h_dots, tw_dots),
                _ => (tw_dots, h_dots),
            };
            self.fill_inverse_backdrop(x, y, bw_dots, bh_dots);
        }
        self.restore_state();

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

        let has_inner = t * 2.0 < w && t * 2.0 < h;

        if reverse_print {
            // Clip to the box (solid) or its border ring (even-odd) and
            // repaint the inverse of the backdrop inside it.
            self.save_state();
            self.push_rounded_rect_path(bx, by, bw, bh, br);
            if has_inner {
                let tp = self.d2pt(t);
                let inner_r = self.d2pt((r_dots - t).max(0.0));
                self.push_rounded_rect_path(
                    bx + tp,
                    by + tp,
                    bw - tp * 2.0,
                    bh - tp * 2.0,
                    inner_r,
                );
                self.emit_op("W*");
            } else {
                self.emit_op("W");
            }
            self.emit_op("n");
            self.fill_inverse_backdrop(x as f64, y as f64, w, h);
            self.restore_state();
        } else {
            self.save_state();
            let (r, g, b) = draw_color;
            self.set_fill_color(r, g, b);
            self.push_rounded_rect_path(bx, by, bw, bh, br);
            self.emit_op("f");
            self.track_backdrop_rect(x as f64, y as f64, w, h, draw_color);

            if has_inner {
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
                self.emit_op("f");
                self.track_backdrop_rect(
                    x as f64 + t,
                    y as f64 + t,
                    w - t * 2.0,
                    h - t * 2.0,
                    clear_color,
                );
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
            self.save_state();
            self.push_ellipse_path(cx_pt, cy_pt, r_pt, r_pt);
            if radius > thickness {
                let inner_r = self.d2pt((radius - thickness) as f64);
                self.push_ellipse_path(cx_pt, cy_pt, inner_r, inner_r);
                self.emit_op("W*");
            } else {
                self.emit_op("W");
            }
            self.emit_op("n");
            self.fill_inverse_backdrop(
                x as f64,
                y as f64,
                radius as f64 * 2.0,
                radius as f64 * 2.0,
            );
            self.restore_state();
        } else {
            self.save_state();
            let (r, g, b) = draw_color;
            self.set_fill_color(r, g, b);
            self.push_ellipse_path(cx_pt, cy_pt, r_pt, r_pt);
            self.emit_op("f");

            if radius > thickness {
                self.set_fill_color(1.0, 1.0, 1.0);
                let inner_r = self.d2pt((radius - thickness) as f64);
                self.push_ellipse_path(cx_pt, cy_pt, inner_r, inner_r);
                self.emit_op("f");
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
            self.save_state();
            self.push_ellipse_path(cx_pt, cy_pt, rx_pt, ry_pt);
            if (width as f64 / 2.0) > t && (height as f64 / 2.0) > t {
                let irx = self.d2pt(width as f64 / 2.0 - t);
                let iry = self.d2pt(height as f64 / 2.0 - t);
                self.push_ellipse_path(cx_pt, cy_pt, irx, iry);
                self.emit_op("W*");
            } else {
                self.emit_op("W");
            }
            self.emit_op("n");
            self.fill_inverse_backdrop(x as f64, y as f64, width as f64, height as f64);
            self.restore_state();
        } else {
            self.save_state();
            let (r, g, b) = draw_color;
            self.set_fill_color(r, g, b);
            self.push_ellipse_path(cx_pt, cy_pt, rx_pt, ry_pt);
            self.emit_op("f");

            if (width as f64 / 2.0) > t && (height as f64 / 2.0) > t {
                self.set_fill_color(1.0, 1.0, 1.0);
                let irx = self.d2pt(width as f64 / 2.0 - t);
                let iry = self.d2pt(height as f64 / 2.0 - t);
                self.push_ellipse_path(cx_pt, cy_pt, irx, iry);
                self.emit_op("f");
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
        reverse_print: bool,
    ) -> ZplResult<()> {
        if width == 0 || height == 0 {
            return Ok(());
        }

        // ZPL ^GF rows are already byte-padded (ceil(width/8) bytes per row),
        // exactly the layout a 1-bit PDF image expects. Pad or truncate to the
        // full bitmap size; padding bytes are 0 (unpainted with Decode [1 0]).
        let row_bytes = width.div_ceil(8) as usize;
        let total_bytes = row_bytes * height as usize;
        let mut bits = data.to_vec();
        bits.resize(total_bytes, 0x00);

        self.embed_mask_image(x as f64, y as f64, width, height, bits, reverse_print);
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
            (data, Some("B")) // Standard default is Code Set B
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
            hint_val.unwrap_or(""),
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

        let bit_matrix = barcode_cache::encode_cached(
            BarcodeFormat::QR_CODE,
            data,
            &format!("ec:{}", level),
            Some(&hints),
        )?;

        let mag = max(magnification, 1);
        self.fill_matrix_cells(x, y, orientation, mag, mag, &bit_matrix, reverse_print);
        Ok(())
    }

    // ── Data Matrix barcode ────────────────────────────────────────

    fn draw_datamatrix(
        &mut self,
        x: u32,
        y: u32,
        orientation: char,
        module_size: u32,
        data: &str,
        reverse_print: bool,
    ) -> ZplResult<()> {
        let bit_matrix = barcode_cache::encode_cached(BarcodeFormat::DATA_MATRIX, data, "", None)?;

        let m = max(module_size, 1);
        self.fill_matrix_cells(x, y, orientation, m, m, &bit_matrix, reverse_print);
        Ok(())
    }

    // ── PDF417 barcode ─────────────────────────────────────────────

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
    ) -> ZplResult<()> {
        let mut hints = HashMap::new();
        hints.insert(
            EncodeHintType::ERROR_CORRECTION,
            EncodeHintValue::ErrorCorrection(security_level.min(8).to_string()),
        );
        hints.insert(
            EncodeHintType::MARGIN,
            EncodeHintValue::Margin("0".to_owned()),
        );
        let hints: EncodeHints = hints.into();

        let bit_matrix = barcode_cache::encode_cached(
            BarcodeFormat::PDF_417,
            data,
            &format!("ec:{}", security_level.min(8)),
            Some(&hints),
        )?;

        let cw = max(module_width, 1);
        let ch = max(row_height, 1);
        self.fill_matrix_cells(x, y, orientation, cw, ch, &bit_matrix, reverse_print);
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
            "",
        )
    }

    // ── generic 1-D barcodes (EAN-13, UPC-A, ITF, Code 93) ────────

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
    ) -> ZplResult<()> {
        self.draw_1d_barcode(
            x,
            y,
            orientation,
            height,
            module_width,
            data,
            barcode_1d_format(kind),
            reverse_print,
            interpretation_line,
            interpretation_line_above,
            None,
            "",
        )
    }

    // ── diagonal line (^GD) ────────────────────────────────────────

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
    ) -> ZplResult<()> {
        let (draw_color, _) = Self::resolve_colors(color, &custom_color);

        let w = max(width, 1) as f64;
        let h = max(height, 1) as f64;
        let t = (max(thickness, 1) as f64).min(w);
        let x = x as f64;
        let y = y as f64;

        // Filled parallelogram with horizontal thickness `t`.
        let pts: [(f64, f64); 4] = if diagonal_orientation == 'L' {
            // '\' top-left → bottom-right
            [(x, y), (x + t, y), (x + w, y + h), (x + w - t, y + h)]
        } else {
            // '/' bottom-left → top-right
            [(x, y + h), (x + t, y + h), (x + w, y), (x + w - t, y)]
        };

        self.save_state();
        if !reverse_print {
            let (r, g, b) = draw_color;
            self.set_fill_color(r, g, b);
        }

        for (i, (dx, dy)) in pts.iter().enumerate() {
            let px = self.x_pt(*dx);
            let py = self.height_pt - dy * self.scale;
            self.emit_nums(&[px, py], if i == 0 { "m" } else { "l" });
        }
        self.emit_op("h");
        if reverse_print {
            self.emit_op("W");
            self.emit_op("n");
            self.fill_inverse_backdrop(x, y, w, h);
        } else {
            self.emit_op("f");
        }
        self.restore_state();

        Ok(())
    }

    // ── finalize ───────────────────────────────────────────────────

    fn finalize(&mut self) -> ZplResult<Vec<u8>> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();

        // ── embed fonts ────────────────────────────────────────────
        //
        // Font objects are built manually instead of using `lopdf::Document::
        // add_font`, which omits /Widths and /ToUnicode and stores descriptor
        // metrics in raw font units. Here every metric is normalized to the
        // 1000/em glyph space and a ToUnicode CMap makes text extraction
        // (copy/paste, search) work for the full WinAnsi range.
        let default_font_bytes: &[u8] = include_bytes!("../assets/IosevkaTermSlab-Regular.ttf");
        let mut font_dict = lopdf::Dictionary::new();
        // Dedup: multiple ZPL identifiers often map to the same font.
        let mut embedded_fonts: HashMap<String, lopdf::ObjectId> = HashMap::new();
        let tounicode_id = doc.add_object(Stream::new(dictionary! {}, build_tounicode_cmap()));

        for font_char in &self.used_fonts {
            let font_key = font_char.to_string();
            let resource_name = format!("F_{}", font_char);

            let actual_name = self
                .font_manager
                .as_ref()
                .and_then(|fm| fm.get_font_name(&font_key).map(|s| s.to_string()))
                .unwrap_or_else(|| "Iosevka Term Slab".to_string());

            if let Some(font_id) = embedded_fonts.get(&actual_name) {
                font_dict.set(resource_name.as_str(), *font_id);
                continue;
            }

            let raw_bytes = self
                .font_manager
                .as_ref()
                .and_then(|fm| fm.get_font_bytes(&font_key))
                .unwrap_or(default_font_bytes);

            let face = FontArc::try_from_vec(raw_bytes.to_vec())
                .map_err(|e| ZplError::FontError(format!("Invalid font data: {}", e)))?;
            let upem = face.units_per_em().unwrap_or(1000.0) as f64;
            let to_glyph_space = |v: f64| (v * 1000.0 / upem).round() as i64;

            // /Widths for the WinAnsi code range 32..=255.
            let widths: Vec<Object> = (0x20..=0xFFu32)
                .map(|code| {
                    let w = winansi_to_char(code as u8)
                        .map(|ch| to_glyph_space(face.h_advance_unscaled(face.glyph_id(ch)) as f64))
                        .unwrap_or(0);
                    w.into()
                })
                .collect();

            // Bounding box and style metrics via ttf-parser (lopdf::FontData).
            let fd = FontData::new(raw_bytes, actual_name.clone());

            let font_stream = Stream::new(
                dictionary! { "Length1" => raw_bytes.len() as i64 },
                raw_bytes.to_vec(),
            );
            let font_file_id = doc.add_object(font_stream);

            let descriptor_id = doc.add_object(dictionary! {
                "Type" => "FontDescriptor",
                "FontName" => Object::Name(actual_name.clone().into_bytes()),
                "Flags" => 32_i64,
                "FontBBox" => vec![
                    to_glyph_space(fd.font_bbox.0 as f64).into(),
                    to_glyph_space(fd.font_bbox.1 as f64).into(),
                    to_glyph_space(fd.font_bbox.2 as f64).into(),
                    to_glyph_space(fd.font_bbox.3 as f64).into(),
                ],
                "ItalicAngle" => fd.italic_angle,
                "Ascent" => to_glyph_space(fd.ascent as f64),
                "Descent" => to_glyph_space(fd.descent as f64),
                "CapHeight" => to_glyph_space(fd.cap_height as f64),
                "StemV" => 80_i64,
                "FontFile2" => font_file_id,
            });

            let font_id = doc.add_object(dictionary! {
                "Type" => "Font",
                "Subtype" => "TrueType",
                "BaseFont" => Object::Name(actual_name.clone().into_bytes()),
                "FirstChar" => 32_i64,
                "LastChar" => 255_i64,
                "Widths" => widths,
                "FontDescriptor" => descriptor_id,
                "Encoding" => "WinAnsiEncoding",
                "ToUnicode" => tounicode_id,
            });

            font_dict.set(resource_name.as_str(), font_id);
            embedded_fonts.insert(actual_name, font_id);
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

            let dict = if img.is_mask {
                // Stencil mask: sample 1 paints with the current fill colour
                // (Decode [1 0]), sample 0 leaves the page untouched.
                dictionary! {
                    "Type" => "XObject",
                    "Subtype" => "Image",
                    "Width" => img.width as i64,
                    "Height" => img.height as i64,
                    "ImageMask" => true,
                    "BitsPerComponent" => 1,
                    "Decode" => vec![1.into(), 0.into()],
                    "Filter" => "FlateDecode",
                }
            } else {
                dictionary! {
                    "Type" => "XObject",
                    "Subtype" => "Image",
                    "Width" => img.width as i64,
                    "Height" => img.height as i64,
                    "ColorSpace" => "DeviceRGB",
                    "BitsPerComponent" => 8,
                    "Filter" => "FlateDecode",
                }
            };
            let img_stream = Stream::new(dict, compressed);
            let img_id = doc.add_object(img_stream);
            xobject_dict.set(img.name.as_str(), img_id);
        }

        // ── resources ──────────────────────────────────────────────
        let resources_id = doc.add_object(dictionary! {
            "Font" => lopdf::Object::Dictionary(font_dict),
            "XObject" => lopdf::Object::Dictionary(xobject_dict),
        });

        // ── pages (one content stream each, shared resources) ──────
        let mut page_contents = std::mem::take(&mut self.finished_pages);
        page_contents.push(std::mem::take(&mut self.content));

        let mut kids: Vec<Object> = Vec::with_capacity(page_contents.len());
        for content_bytes in page_contents {
            let content_id = doc.add_object(Stream::new(dictionary! {}, content_bytes));
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
            kids.push(page_id.into());
        }

        // ── pages tree ─────────────────────────────────────────────
        let pages_dict = dictionary! {
            "Type" => "Pages",
            "Count" => kids.len() as i64,
            "Kids" => kids,
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

        // ── catalogue ──────────────────────────────────────────────
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        // ── document info ──────────────────────────────────────────
        let mut info = lopdf::Dictionary::new();
        info.set(
            "Producer",
            Object::string_literal(concat!("zpl-forge ", env!("CARGO_PKG_VERSION"))),
        );
        if let Some(title) = &self.title {
            info.set("Title", Object::string_literal(title.as_str()));
        }
        let info_id = doc.add_object(Object::Dictionary(info));
        doc.trailer.set("Info", info_id);

        doc.compress();

        // ── serialize ──────────────────────────────────────────────
        let mut buf = std::io::BufWriter::new(Vec::new());
        doc.save_to(&mut buf)
            .map_err(|e| ZplError::BackendError(format!("Failed to save PDF: {}", e)))?;
        buf.into_inner()
            .map_err(|e| ZplError::BackendError(format!("Failed to flush: {}", e)))
    }
}
