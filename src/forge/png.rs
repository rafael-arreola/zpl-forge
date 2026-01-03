use std::cmp::max;
use std::collections::HashMap;
use std::sync::Arc;

use ab_glyph::{Font, PxScale, ScaleFont};
use base64::{engine::general_purpose, Engine as _};
use image::{imageops::overlay, ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::{
    draw_filled_circle_mut, draw_filled_ellipse_mut, draw_filled_rect_mut, draw_text_mut,
};
use imageproc::rect::Rect;
use rxing::{
    BarcodeFormat, EncodeHintType, EncodeHintValue, EncodeHints, MultiFormatWriter, Writer,
};

use crate::engine::{FontManager, ZplForgeBackend};
use crate::{ZplError, ZplResult};

/// A rendering backend that produces PNG images.
///
/// This backend uses the `image` and `imageproc` crates to draw ZPL instructions
/// onto an RGB canvas.
pub struct PngBackend {
    canvas: RgbImage,
    font_manager: Option<Arc<FontManager>>,
}

impl Default for PngBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl PngBackend {
    /// Creates a new `PngBackend` instance with an empty canvas.
    pub fn new() -> Self {
        Self {
            canvas: ImageBuffer::new(0, 0),
            font_manager: None,
        }
    }

    /// Performs an XOR overlay of a source image onto the canvas at (x, y).
    fn xor_overlay(&mut self, src: &RgbImage, x: i64, y: i64) {
        let (sw, sh) = src.dimensions();
        let (cw, ch) = self.canvas.dimensions();

        for sy in 0..sh {
            let dy = y + sy as i64;
            if dy < 0 || dy >= ch as i64 {
                continue;
            }

            for sx in 0..sw {
                let dx = x + sx as i64;
                if dx < 0 || dx >= cw as i64 {
                    continue;
                }

                let src_pixel = src[(sx, sy)];
                if src_pixel.0 != [255, 255, 255] {
                    let dest_pixel = &mut self.canvas[(dx as u32, dy as u32)];
                    dest_pixel.0[0] ^= 255;
                    dest_pixel.0[1] ^= 255;
                    dest_pixel.0[2] ^= 255;
                }
            }
        }
    }

    /// Inverts the colors within a specified rectangular area.
    fn invert_rect(&mut self, rect: Rect) {
        let (cw, ch) = self.canvas.dimensions();
        let x_start = rect.left().max(0) as u32;
        let y_start = rect.top().max(0) as u32;
        let x_end = (rect.right() as u32).min(cw);
        let y_end = (rect.bottom() as u32).min(ch);

        for py in y_start..y_end {
            for px in x_start..x_end {
                let pixel = &mut self.canvas[(px, py)];
                pixel.0[0] ^= 255;
                pixel.0[1] ^= 255;
                pixel.0[2] ^= 255;
            }
        }
    }

    /// Helper to execute a drawing operation.
    fn draw_wrapper<F>(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        reverse_print: bool,
        draw_op: F,
    ) -> ZplResult<()>
    where
        F: FnOnce(&mut RgbImage, i32, i32),
    {
        if reverse_print {
            let mut temp_buf = ImageBuffer::from_pixel(width, height, Rgb([255, 255, 255]));
            draw_op(&mut temp_buf, 0, 0);
            self.xor_overlay(&temp_buf, x as i64, y as i64);
        } else {
            draw_op(&mut self.canvas, x as i32, y as i32);
        }
        Ok(())
    }

    fn parse_hex_color(&self, color: &Option<String>) -> Rgb<u8> {
        if let Some(hex) = color {
            let hex = hex.trim_start_matches('#');
            if hex.len() == 6 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[0..2], 16),
                    u8::from_str_radix(&hex[2..4], 16),
                    u8::from_str_radix(&hex[4..6], 16),
                ) {
                    return Rgb([r, g, b]);
                }
            } else if hex.len() == 3 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[0..1], 16),
                    u8::from_str_radix(&hex[1..2], 16),
                    u8::from_str_radix(&hex[2..3], 16),
                ) {
                    return Rgb([r * 17, g * 17, b * 17]);
                }
            }
        }
        Rgb([0, 0, 0])
    }

    fn get_text_width(
        &self,
        text: &str,
        font_char: char,
        height: Option<u32>,
        width: Option<u32>,
    ) -> u32 {
        let font = match self.font_manager.as_ref() {
            Some(fm) => match fm.get_font(&font_char.to_string()) {
                Some(f) => f,
                None => match fm.get_font("0") {
                    Some(f) => f,
                    None => return 0,
                },
            },
            None => return 0,
        };

        let scale_y = height.unwrap_or(9) as f32;
        let scale_x = width.unwrap_or(scale_y as u32) as f32;
        let scale = PxScale {
            x: scale_x,
            y: scale_y,
        };

        let scaled_font = font.as_scaled(scale);
        let mut width = 0.0;
        let mut last_glyph_id = None;

        for c in text.chars() {
            let glyph_id = font.glyph_id(c);
            if let Some(last) = last_glyph_id {
                width += scaled_font.kern(last, glyph_id);
            }
            width += scaled_font.h_advance(glyph_id);
            last_glyph_id = Some(glyph_id);
        }

        width.ceil() as u32
    }
}

impl ZplForgeBackend for PngBackend {
    fn setup_page(&mut self, width: f64, height: f64, _resolution: f32) {
        // Safety limit to avoid OOM: 8192x8192 is enough for most labels
        const MAX_DIM: u32 = 8192;
        let w = (width as u32).min(MAX_DIM);
        let h = (height as u32).min(MAX_DIM);
        self.canvas = ImageBuffer::from_pixel(w, h, Rgb([255, 255, 255]));
    }

    fn setup_font_manager(&mut self, font_manager: &FontManager) {
        self.font_manager = Some(Arc::new(font_manager.clone()));
    }

    fn draw_text(
        &mut self,
        x: u32,
        y: u32,
        font: char,
        height: Option<u32>,
        width: Option<u32>,
        text: String,
        _reverse_print: bool,
        color: Option<String>,
    ) -> ZplResult<()> {
        if text.is_empty() {
            return Ok(());
        }

        let font_data = match self.font_manager.as_ref() {
            Some(fm) => match fm.get_font(&font.to_string()) {
                Some(f) => f,
                None => match fm.get_font("0") {
                    Some(f) => f,
                    None => return Err(ZplError::FontError(format!("Font not found: {}", font))),
                },
            },
            None => return Err(ZplError::FontError("Font manager not initialized".into())),
        };

        let scale_y = height.unwrap_or(9) as f32;
        let scale_x = width.unwrap_or(scale_y as u32) as f32;
        let scale = PxScale {
            x: scale_x,
            y: scale_y,
        };

        let text_color = self.parse_hex_color(&color);

        draw_text_mut(
            &mut self.canvas,
            text_color,
            x as i32,
            y as i32,
            scale,
            font_data,
            &text,
        );
        Ok(())
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
        let w = max(width, 1);
        let h = max(height, 1);
        let t = thickness;
        let r = (rounding as f64 * 8.0) as i32;

        let (draw_color, clear_color) = if let Some(custom) = custom_color {
            (self.parse_hex_color(&Some(custom)), Rgb([255, 255, 255]))
        } else if color == 'B' {
            (Rgb([0, 0, 0]), Rgb([255, 255, 255]))
        } else {
            (Rgb([255, 255, 255]), Rgb([0, 0, 0]))
        };

        let draw_op = |img: &mut RgbImage, px: i32, py: i32| {
            let draw_rounded_fill =
                |img: &mut RgbImage, px: i32, py: i32, pw: u32, ph: u32, pr: i32, pc: Rgb<u8>| {
                    if pw == 0 || ph == 0 {
                        return;
                    }
                    if pr <= 0 {
                        draw_filled_rect_mut(img, Rect::at(px, py).of_size(pw, ph), pc);
                    } else {
                        let pr = pr.max(0).min((pw / 2) as i32).min((ph / 2) as i32);
                        let inner_w = pw.saturating_sub(2 * pr as u32).max(1);
                        let inner_h = ph.saturating_sub(2 * pr as u32).max(1);
                        draw_filled_rect_mut(img, Rect::at(px + pr, py).of_size(inner_w, ph), pc);
                        draw_filled_rect_mut(img, Rect::at(px, py + pr).of_size(pw, inner_h), pc);
                        draw_filled_circle_mut(img, (px + pr, py + pr), pr, pc);
                        draw_filled_circle_mut(img, (px + pw as i32 - pr - 1, py + pr), pr, pc);
                        draw_filled_circle_mut(img, (px + pr, py + ph as i32 - pr - 1), pr, pc);
                        draw_filled_circle_mut(
                            img,
                            (px + pw as i32 - pr - 1, py + ph as i32 - pr - 1),
                            pr,
                            pc,
                        );
                    }
                };

            draw_rounded_fill(img, px, py, w, h, r, draw_color);
            if t * 2 < w && t * 2 < h {
                draw_rounded_fill(
                    img,
                    px + t as i32,
                    py + t as i32,
                    w - t * 2,
                    h - t * 2,
                    (r - t as i32).max(0),
                    clear_color,
                );
            }
        };

        self.draw_wrapper(x, y, w, h, reverse_print, draw_op)
    }

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
        let color = self.parse_hex_color(&custom_color);
        let clear_color = Rgb([255, 255, 255]);

        let draw_op = |img: &mut RgbImage, px: i32, py: i32| {
            let center_x = px + radius as i32;
            let center_y = py + radius as i32;
            draw_filled_circle_mut(img, (center_x, center_y), radius as i32, color);

            if radius > thickness {
                draw_filled_circle_mut(
                    img,
                    (center_x, center_y),
                    (radius - thickness) as i32,
                    clear_color,
                );
            }
        };

        self.draw_wrapper(x, y, radius * 2, radius * 2, reverse_print, draw_op)
    }

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
        let color = self.parse_hex_color(&custom_color);
        let clear_color = Rgb([255, 255, 255]);

        let draw_op = |img: &mut RgbImage, px: i32, py: i32| {
            let rx = (width / 2) as i32;
            let ry = (height / 2) as i32;
            let center_x = px + rx;
            let center_y = py + ry;
            draw_filled_ellipse_mut(img, (center_x, center_y), rx, ry, color);

            let t = thickness as i32;
            if rx > t && ry > t {
                draw_filled_ellipse_mut(img, (center_x, center_y), rx - t, ry - t, clear_color);
            }
        };

        self.draw_wrapper(x, y, width, height, reverse_print, draw_op)
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
        let draw_op = |img: &mut RgbImage, px: i32, py: i32| {
            let row_bytes = width.div_ceil(8);
            let (img_w, img_h) = (img.width() as i32, img.height() as i32);

            for (row_idx, row_data) in data.chunks(row_bytes as usize).enumerate() {
                let dy = py + row_idx as i32;
                if dy < 0 || dy >= img_h || row_idx as u32 >= height {
                    continue;
                }

                for (byte_idx, &byte) in row_data.iter().enumerate() {
                    if byte == 0 {
                        continue;
                    }
                    let base_x = px + (byte_idx as i32 * 8);
                    for bit_idx in 0..8 {
                        let col_idx = byte_idx as u32 * 8 + bit_idx;
                        if col_idx >= width {
                            break;
                        }

                        if (byte & (0x80 >> bit_idx)) != 0 {
                            let dx = base_x + bit_idx as i32;
                            if dx >= 0 && dx < img_w {
                                img[(dx as u32, dy as u32)] = Rgb([0, 0, 0]);
                            }
                        }
                    }
                }
            }
        };

        self.draw_wrapper(x, y, width, height, reverse_print, draw_op)
    }

    fn draw_graphic_image_custom(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: String,
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

        let resized_img = if target_w != orig_w || target_h != orig_h {
            image::imageops::resize(
                &img,
                target_w,
                target_h,
                image::imageops::FilterType::Lanczos3,
            )
        } else {
            img
        };

        overlay(&mut self.canvas, &resized_img, x as i64, y as i64);
        Ok(())
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
        _check_digit: char,
        _mode: char,
        data: String,
        reverse_print: bool,
    ) -> ZplResult<()> {
        let (clean_data, hint_val) = if let Some(stripped) = data.strip_prefix(">:") {
            (stripped, Some("B"))
        } else if let Some(stripped) = data.strip_prefix(">;") {
            (stripped, Some("C"))
        } else if let Some(stripped) = data.strip_prefix(">9") {
            (stripped, Some("A"))
        } else {
            (data.as_str(), None)
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

    fn draw_qr_code(
        &mut self,
        x: u32,
        y: u32,
        orientation: char,
        _model: u32,
        magnification: u32,
        error_correction: char,
        _mask: u32,
        data: String,
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
            .encode_with_hints(&data, &BarcodeFormat::QR_CODE, 0, 0, &hints)
            .map_err(|e| ZplError::BackendError(format!("QR Generation Error: {}", e)))?;

        let mag = max(magnification, 1);
        let bw = bit_matrix.getWidth();
        let bh = bit_matrix.getHeight();
        let full_width = bw * mag;
        let full_height = bh * mag;

        let transform_rect = |lx: i32, ly: i32, w: u32, h: u32| -> Rect {
            match orientation {
                'N' => Rect::at(x as i32 + lx, y as i32 + ly).of_size(w, h),
                'R' => {
                    let new_x = full_height as i32 - (ly + h as i32);
                    let new_y = lx;
                    Rect::at(x as i32 + new_x, y as i32 + new_y).of_size(h, w)
                }
                'I' => {
                    let new_x = full_width as i32 - (lx + w as i32);
                    let new_y = full_height as i32 - (ly + h as i32);
                    Rect::at(x as i32 + new_x, y as i32 + new_y).of_size(w, h)
                }
                'B' => {
                    let new_x = ly;
                    let new_y = full_width as i32 - (lx + w as i32);
                    Rect::at(x as i32 + new_x, y as i32 + new_y).of_size(h, w)
                }
                _ => Rect::at(x as i32 + lx, y as i32 + ly).of_size(w, h),
            }
        };

        for gy in 0..bh {
            for gx in 0..bw {
                if bit_matrix.get(gx, gy) {
                    let rect = transform_rect((gx * mag) as i32, (gy * mag) as i32, mag, mag);
                    if reverse_print {
                        self.invert_rect(rect);
                    } else {
                        draw_filled_rect_mut(&mut self.canvas, rect, Rgb([0, 0, 0]));
                    }
                }
            }
        }

        Ok(())
    }

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
        data: String,
        reverse_print: bool,
    ) -> ZplResult<()> {
        self.draw_1d_barcode(
            x,
            y,
            orientation,
            height,
            module_width,
            &data,
            BarcodeFormat::CODE_39,
            reverse_print,
            interpretation_line,
            interpretation_line_above,
            None,
        )
    }

    fn finalize(&mut self) -> ZplResult<Vec<u8>> {
        let mut bytes = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut bytes);
        self.canvas
            .write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| ZplError::BackendError(format!("Failed to write PNG: {}", e)))?;
        Ok(bytes)
    }
}

impl PngBackend {
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
            'N' | 'I' => (bw, bh),
            'R' | 'B' => (bh, bw),
            _ => (bw, bh),
        };

        let transform_rect = |lx: i32, ly: i32, w: u32, h: u32| -> Rect {
            match orientation {
                'N' => Rect::at(x as i32 + lx, y as i32 + ly).of_size(w, h),
                'R' => {
                    let new_x = bh as i32 - (ly + h as i32);
                    let new_y = lx;
                    Rect::at(x as i32 + new_x, y as i32 + new_y).of_size(h, w)
                }
                'I' => {
                    let new_x = bw as i32 - (lx + w as i32);
                    let new_y = bh as i32 - (ly + h as i32);
                    Rect::at(x as i32 + new_x, y as i32 + new_y).of_size(w, h)
                }
                'B' => {
                    let new_x = ly;
                    let new_y = bw as i32 - (lx + w as i32);
                    Rect::at(x as i32 + new_x, y as i32 + new_y).of_size(h, w)
                }
                _ => Rect::at(x as i32 + lx, y as i32 + ly).of_size(w, h),
            }
        };

        for gx in 0..bit_matrix.getWidth() {
            if bit_matrix.get(gx, 0) {
                let rect = transform_rect((gx * mw) as i32, 0, mw, bh);
                if reverse_print {
                    self.invert_rect(rect);
                } else {
                    draw_filled_rect_mut(&mut self.canvas, rect, Rgb([0, 0, 0]));
                }
            }
        }

        if interpretation_line == 'Y' {
            let font_char = '0';
            let text_h = 18;
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
                data.to_string(),
                false,
                None,
            )?;
        }

        Ok(())
    }
}
