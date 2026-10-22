//! PNG rendering backend for ZPL label output.
//!
//! This module provides [`PngBackend`], which rasterizes ZPL commands into
//! RGB PNG images using the `image` and `imageproc` crates.

use std::cmp::max;
use std::collections::HashMap;
use std::sync::Arc;

use ab_glyph::{Font, ScaleFont};
use image::{
    ImageBuffer, Rgb, RgbImage, Rgba, RgbaImage,
    imageops::{overlay, rotate90, rotate180, rotate270},
};
use imageproc::drawing::{
    draw_filled_circle_mut, draw_filled_ellipse_mut, draw_filled_rect_mut, draw_polygon_mut,
    draw_text_mut,
};
use imageproc::point::Point;
use imageproc::rect::Rect;
use rxing::common::BitMatrix;
use rxing::datamatrix::encoder::SymbolShapeHint;
use rxing::{BarcodeFormat, EncodeHintType, EncodeHintValue, EncodeHints};

use super::{barcode_1d_format, barcode_cache, symbology};
use crate::engine::{Barcode1DKind, FontManager, ZplForgeBackend};
use crate::{ZplError, ZplResult};

/// `rxing`'s numeric code for PDF417 text compaction.
const PDF417_TEXT_COMPACTION: u32 = 1;

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
        let left = i64::from(rect.left());
        let top = i64::from(rect.top());
        let x_start = left.clamp(0, i64::from(cw)) as u32;
        let y_start = top.clamp(0, i64::from(ch)) as u32;
        let x_end = (left + i64::from(rect.width())).clamp(0, i64::from(cw)) as u32;
        let y_end = (top + i64::from(rect.height())).clamp(0, i64::from(ch)) as u32;

        for py in y_start..y_end {
            for px in x_start..x_end {
                let pixel = &mut self.canvas[(px, py)];
                pixel.0[0] ^= 255;
                pixel.0[1] ^= 255;
                pixel.0[2] ^= 255;
            }
        }
    }

    fn validate_draw_bounds(x: u32, y: u32, width: u32, height: u32) -> ZplResult<()> {
        super::validate_raster_dimensions(width, height)?;
        let right = i64::from(x) + i64::from(width);
        let bottom = i64::from(y) + i64::from(height);
        if right > i64::from(i32::MAX) || bottom > i64::from(i32::MAX) {
            return Err(ZplError::BackendError(format!(
                "Drawing bounds exceed supported coordinates: ({x}, {y}), {width}x{height}"
            )));
        }
        Ok(())
    }

    fn validate_barcode_input(height: u32, module_width: u32, data: &str) -> ZplResult<()> {
        super::validate_raster_dimensions(module_width.max(1), height)?;
        // Bound native encoding and numeric normalization before they allocate.
        if data.len() > super::MAX_IMAGE_DIMENSION as usize {
            return Err(ZplError::BackendError(
                "Barcode data exceeds the raster input limit".into(),
            ));
        }
        Ok(())
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
        Self::validate_draw_bounds(x, y, width, height)?;
        if reverse_print {
            let mut temp_buf = ImageBuffer::from_pixel(width, height, Rgb([255, 255, 255]));
            draw_op(&mut temp_buf, 0, 0);
            self.xor_overlay(&temp_buf, x as i64, y as i64);
        } else {
            draw_op(&mut self.canvas, x as i32, y as i32);
        }
        Ok(())
    }

    fn get_text_width(
        &self,
        text: &str,
        font_char: char,
        height: Option<u32>,
        width: Option<u32>,
    ) -> u32 {
        match self.font_manager.as_ref() {
            Some(fm) => fm.measure_text(font_char, height, width, text),
            None => 0,
        }
    }
}

impl ZplForgeBackend for PngBackend {
    fn setup_page(&mut self, width: f64, height: f64, _resolution: f32) {
        let w = (width as u32).min(super::MAX_IMAGE_DIMENSION);
        let h = (height as u32).min(super::MAX_IMAGE_DIMENSION);
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
        orientation: char,
        text: &str,
        reverse_print: bool,
        color: Option<String>,
    ) -> ZplResult<()> {
        let text_color = Rgb(super::parse_hex_color(color.as_deref())?);
        if text.is_empty() {
            return Ok(());
        }
        super::validate_raster_dimensions(width.unwrap_or(1).max(1), height.unwrap_or(1).max(1))?;

        let fm = self
            .font_manager
            .as_ref()
            .ok_or_else(|| ZplError::FontError("Font manager not initialized".into()))?;
        let (font_arc, layout) = fm
            .text_layout(font, height, width)
            .ok_or_else(|| ZplError::FontError(format!("Font not found: {}", font)))?;
        let font_data = font_arc.clone();
        let scale = layout.px;
        if !scale.x.is_finite()
            || !scale.y.is_finite()
            || scale.x <= 0.0
            || scale.y <= 0.0
            || !layout.cell_h.is_finite()
            || layout.cell_h <= 0.0
            || !layout.baseline.is_finite()
        {
            return Err(ZplError::FontError("Invalid text layout dimensions".into()));
        }

        // imageproc places the baseline at `y + ascent`; shift so capital
        // letters start exactly at the ZPL cell top (Zebra behavior).
        let ascent = font_data.as_scaled(scale).ascent();
        let offset = (layout.baseline - ascent).round();
        if !offset.is_finite() || offset.abs() > super::MAX_IMAGE_DIMENSION as f32 {
            return Err(ZplError::FontError(
                "Text baseline offset exceeds raster limits".into(),
            ));
        }
        let y_offset = offset as i32;
        let text_w = self.get_text_width(text, font, height, width).max(1);
        let font_h = (layout.cell_h.ceil() as u32).max(1);
        super::validate_raster_dimensions(text_w, font_h)?;

        // Ink can rise above the cap line and descend below the character cell.
        // Keep that padding in the mask, then compensate at the blit anchor.
        let top_pad = (-y_offset).max(0) as u32;
        let descent = font_data.as_scaled(scale).descent();
        let ink_bottom = (layout.baseline - descent).ceil();
        if !ink_bottom.is_finite() || ink_bottom.abs() > 2.0 * super::MAX_IMAGE_DIMENSION as f32 {
            return Err(ZplError::FontError(
                "Text ink bounds exceed raster limits".into(),
            ));
        }
        let bottom_pad = (ink_bottom as i32 - font_h as i32).max(0) as u32;
        let temp_h = font_h
            .checked_add(top_pad)
            .and_then(|h| h.checked_add(bottom_pad))
            .ok_or_else(|| ZplError::FontError("Text raster dimensions overflow".into()))?;
        super::validate_raster_dimensions(text_w, temp_h)?;

        if !reverse_print && !matches!(orientation, 'R' | 'I' | 'B') {
            Self::validate_draw_bounds(x, y, text_w, temp_h)?;
            let draw_y = (y as i32)
                .checked_add(y_offset)
                .ok_or_else(|| ZplError::FontError("Text coordinates overflow".into()))?;
            draw_text_mut(
                &mut self.canvas,
                text_color,
                x as i32,
                draw_y,
                scale,
                &font_data,
                text,
            );
            return Ok(());
        }

        // Keep RGB constant under transparent pixels so coverage blending does
        // not multiply an already coverage-weighted color a second time.
        let mut tmp = RgbaImage::from_pixel(
            text_w,
            temp_h,
            Rgba([text_color.0[0], text_color.0[1], text_color.0[2], 0]),
        );
        let text_rgba = Rgba([text_color.0[0], text_color.0[1], text_color.0[2], 255]);
        draw_text_mut(
            &mut tmp,
            text_rgba,
            0,
            y_offset + top_pad as i32,
            scale,
            &font_data,
            text,
        );

        // Only padding landing on the low-index side displaces the cell origin.
        let (rotated, pad_x, pad_y) = match orientation {
            // 90° cw: bottom padding lands on the left.
            'R' => (rotate90(&tmp), bottom_pad, 0),
            // 180°: bottom padding lands on top.
            'I' => (rotate180(&tmp), 0, bottom_pad),
            // 270° cw: top padding lands on the left.
            'B' => (rotate270(&tmp), top_pad, 0),
            _ => (tmp, 0, top_pad),
        };

        let (cw, ch) = self.canvas.dimensions();
        for (sx, sy, p) in rotated.enumerate_pixels() {
            let alpha = u32::from(p.0[3]);
            if alpha == 0 {
                continue;
            }
            let dx = i64::from(x) + i64::from(sx) - i64::from(pad_x);
            let dy = i64::from(y) + i64::from(sy) - i64::from(pad_y);
            if (0..i64::from(cw)).contains(&dx) && (0..i64::from(ch)).contains(&dy) {
                let pixel = &mut self.canvas[(dx as u32, dy as u32)];
                for channel in 0..3 {
                    let background = pixel.0[channel];
                    let foreground = if reverse_print {
                        background ^ 255
                    } else {
                        p.0[channel]
                    };
                    pixel.0[channel] = ((u32::from(background) * (255 - alpha)
                        + u32::from(foreground) * alpha
                        + 127)
                        / 255) as u8;
                }
            }
        }
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
            (
                Rgb(super::parse_hex_color(Some(&custom))?),
                Rgb([255, 255, 255]),
            )
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
            // Compare before doubling: an oversized thickness simply fills the box.
            if t < w.div_ceil(2) && t < h.div_ceil(2) {
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
        let color = Rgb(super::parse_hex_color(custom_color.as_deref())?);
        let clear_color = Rgb([255, 255, 255]);
        let diameter = radius
            .checked_mul(2)
            .ok_or_else(|| ZplError::BackendError("Circle dimensions overflow".into()))?;

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

        self.draw_wrapper(x, y, diameter, diameter, reverse_print, draw_op)
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
        let color = Rgb(super::parse_hex_color(custom_color.as_deref())?);
        let clear_color = Rgb([255, 255, 255]);

        let draw_op = |img: &mut RgbImage, px: i32, py: i32| {
            let rx = (width / 2) as i32;
            let ry = (height / 2) as i32;
            let center_x = px + rx;
            let center_y = py + ry;
            draw_filled_ellipse_mut(img, (center_x, center_y), rx, ry, color);

            if thickness < rx as u32 && thickness < ry as u32 {
                let t = thickness as i32;
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
        data: &[u8],
        reverse_print: bool,
    ) -> ZplResult<()> {
        let expected_bytes = super::bitmap_size(width, height)?;
        if data.len() != expected_bytes {
            return Err(ZplError::ImageError(format!(
                "Graphic field data length mismatch: expected {expected_bytes}, got {}",
                data.len()
            )));
        }
        let row_bytes = width.div_ceil(8) as usize;
        let draw_op = |img: &mut RgbImage, px: i32, py: i32| {
            let (img_w, img_h) = (img.width() as i32, img.height() as i32);

            for (row_idx, row_data) in data.chunks(row_bytes).enumerate() {
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
        data: &str,
    ) -> ZplResult<()> {
        super::validate_raster_dimensions(width.max(1), height.max(1))?;
        let img = super::decode_color_image(data)?;
        let (orig_w, orig_h) = img.dimensions();
        super::validate_raster_dimensions(orig_w, orig_h)?;
        let (target_w, target_h) = match (width, height) {
            (0, 0) => (orig_w, orig_h),
            (w, 0) => {
                let h = (orig_h as f64 * (w as f64 / orig_w as f64)).round() as u32;
                (w, h)
            }
            (0, h) => {
                let w = (orig_w as f64 * (h as f64 / orig_h as f64)).round() as u32;
                (w, h)
            }
            (w, h) => (w, h),
        };

        super::validate_raster_dimensions(target_w, target_h)?;
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
        data: &str,
        reverse_print: bool,
    ) -> ZplResult<()> {
        let (clean_data, code_set) = super::code128_code_set(data);

        let mut h = HashMap::new();
        h.insert(
            EncodeHintType::FORCE_CODE_SET,
            EncodeHintValue::ForceCodeSet(code_set.to_string()),
        );
        let hints = Some(EncodeHints::from(h));

        self.draw_1d_barcode(
            x,
            y,
            orientation,
            height,
            module_width,
            // Code 128 is a four-width symbology; ^BY's ratio does not apply.
            0.0,
            clean_data,
            BarcodeFormat::CODE_128,
            reverse_print,
            interpretation_line,
            interpretation_line_above,
            hints,
            code_set,
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
        data: &str,
        reverse_print: bool,
    ) -> ZplResult<()> {
        let (ec, payload) = super::qr_field_data(data, error_correction);
        let level = match ec {
            'L' => "L",
            'Q' => "Q",
            'H' => "H",
            _ => "M",
        };

        let mut hints = HashMap::new();
        hints.insert(
            EncodeHintType::ERROR_CORRECTION,
            EncodeHintValue::ErrorCorrection(level.to_string()),
        );
        let hints: EncodeHints = hints.into();

        let bit_matrix = barcode_cache::encode_cached(
            BarcodeFormat::QR_CODE,
            payload,
            &format!("ec:{}", level),
            Some(&hints),
        )?;

        let mag = max(magnification, 1);
        let y = y
            .checked_add(super::QR_ORIGIN_Y_OFFSET)
            .ok_or_else(|| ZplError::BackendError("QR origin coordinates overflow".into()))?;
        self.fill_matrix_cells(x, y, orientation, mag, mag, &bit_matrix, reverse_print)
    }

    fn draw_datamatrix(
        &mut self,
        x: u32,
        y: u32,
        orientation: char,
        module_size: u32,
        data: &str,
        reverse_print: bool,
    ) -> ZplResult<()> {
        // Zebra emits a square Data Matrix unless explicit rows/columns ask for
        // a rectangle; `rxing` defaults to the smallest fitting symbol, which is
        // often rectangular (26x12 instead of 18x18 for a 20-byte payload).
        let hints = EncodeHints {
            DataMatrixShape: Some(SymbolShapeHint::FORCE_SQUARE),
            ..Default::default()
        };

        let bit_matrix =
            barcode_cache::encode_cached(BarcodeFormat::DATA_MATRIX, data, "sq", Some(&hints))?;

        let m = max(module_size, 1);
        self.fill_matrix_cells(x, y, orientation, m, m, &bit_matrix, reverse_print)
    }

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
        let ec = security_level.min(8);
        let hints = EncodeHints {
            ErrorCorrection: Some(ec.to_string()),
            // Text compaction matches Zebra's choice for alphanumeric payloads.
            Pdf417Compaction: Some(PDF417_TEXT_COMPACTION.to_string()),
            Pdf417Dimensions: Some(super::pdf417_dimensions(None, None)),
            ..Default::default()
        };

        let bit_matrix = barcode_cache::encode_cached(
            BarcodeFormat::PDF_417,
            data,
            &format!("ec:{ec}:c1:t"),
            Some(&hints),
        )?;
        let bit_matrix = super::pdf417_descale(&bit_matrix)?;

        let cw = max(module_width, 1);
        let ch = max(row_height, 1);
        self.fill_matrix_cells(x, y, orientation, cw, ch, &bit_matrix, reverse_print)
    }

    fn draw_micropdf417(
        &mut self,
        _x: u32,
        _y: u32,
        _orientation: char,
        _height: u32,
        _mode: u32,
        _data: &str,
        _reverse_print: bool,
    ) -> ZplResult<()> {
        Err(ZplError::BackendError(
            "MicroPDF417 is not supported by the PNG backend".into(),
        ))
    }

    fn draw_aztec_code(
        &mut self,
        x: u32,
        y: u32,
        orientation: char,
        magnification: u32,
        data: &str,
        reverse_print: bool,
    ) -> ZplResult<()> {
        let bit_matrix = barcode_cache::encode_cached(BarcodeFormat::AZTEC, data, "", None)?;
        let m = max(magnification, 1);
        self.fill_matrix_cells(x, y, orientation, m, m, &bit_matrix, reverse_print)
    }

    fn draw_code39(
        &mut self,
        x: u32,
        y: u32,
        orientation: char,
        _check_digit: char,
        height: u32,
        module_width: u32,
        ratio: f32,
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
            ratio,
            data,
            BarcodeFormat::CODE_39,
            reverse_print,
            interpretation_line,
            interpretation_line_above,
            None,
            "",
        )
    }

    fn draw_barcode_1d(
        &mut self,
        kind: Barcode1DKind,
        x: u32,
        y: u32,
        orientation: char,
        height: u32,
        module_width: u32,
        ratio: f32,
        check_digit: char,
        interpretation_line: char,
        interpretation_line_above: char,
        data: &str,
        reverse_print: bool,
    ) -> ZplResult<()> {
        Self::validate_barcode_input(height, module_width, data)?;
        // MSI and POSTNET have no rxing writer; both are encoded natively.
        match kind {
            Barcode1DKind::Msi => {
                let narrow = module_width.max(1);
                let wide = ((narrow as f32) * ratio).round().max(narrow as f32 + 1.0);
                if !ratio.is_finite() || ratio <= 0.0 || wide > super::MAX_IMAGE_DIMENSION as f32 {
                    return Err(ZplError::BackendError(
                        "Invalid MSI wide:narrow ratio".into(),
                    ));
                }
                let check = symbology::MsiCheck::from_zpl(check_digit);
                let elements = symbology::msi_elements(data, check, module_width, ratio);
                let text = symbology::msi_text(data, check);
                return self.draw_elements(
                    x,
                    y,
                    orientation,
                    height,
                    module_width,
                    &elements,
                    reverse_print,
                    interpretation_line,
                    interpretation_line_above,
                    &text,
                );
            }
            Barcode1DKind::Postnet => {
                return self.draw_postnet(
                    x,
                    y,
                    orientation,
                    height,
                    module_width,
                    data,
                    reverse_print,
                    interpretation_line,
                    interpretation_line_above,
                );
            }
            _ => {}
        }

        self.draw_1d_barcode(
            x,
            y,
            orientation,
            height,
            module_width,
            ratio,
            data,
            barcode_1d_format(kind),
            reverse_print,
            interpretation_line,
            interpretation_line_above,
            None,
            "",
        )
    }

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
        let draw_color = if custom_color.is_some() {
            Rgb(super::parse_hex_color(custom_color.as_deref())?)
        } else if color == 'W' {
            Rgb([255, 255, 255])
        } else {
            Rgb([0, 0, 0])
        };

        let w = max(width, 1);
        let h = max(height, 1);
        let t = max(thickness, 1).min(w);

        let draw_op = move |img: &mut RgbImage, px: i32, py: i32| {
            let (w, h, t) = (w as i32, h as i32, t as i32);
            // Filled parallelogram with horizontal thickness `t`.
            let pts = if diagonal_orientation == 'L' {
                // '\' top-left → bottom-right
                [
                    Point::new(px, py),
                    Point::new(px + t, py),
                    Point::new(px + w, py + h),
                    Point::new(px + w - t, py + h),
                ]
            } else {
                // '/' bottom-left → top-right
                [
                    Point::new(px, py + h),
                    Point::new(px + t, py + h),
                    Point::new(px + w, py),
                    Point::new(px + w - t, py),
                ]
            };
            draw_polygon_mut(img, &pts, draw_color);
        };

        self.draw_wrapper(x, y, w, h, reverse_print, draw_op)
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
    ) -> ZplResult<()> {
        let bw = bit_matrix.getWidth();
        let bh = bit_matrix.getHeight();
        let full_w = bw
            .checked_mul(cell_w)
            .ok_or_else(|| ZplError::BackendError("Barcode width overflows".into()))?;
        let full_h = bh
            .checked_mul(cell_h)
            .ok_or_else(|| ZplError::BackendError("Barcode height overflows".into()))?;
        let (draw_w, draw_h) = if matches!(orientation, 'R' | 'B') {
            (full_h, full_w)
        } else {
            (full_w, full_h)
        };
        Self::validate_draw_bounds(x, y, draw_w, draw_h)?;

        for gy in 0..bh {
            for gx in 0..bw {
                if !bit_matrix.get(gx, gy) {
                    continue;
                }
                let lx = (gx * cell_w) as i32;
                let ly = (gy * cell_h) as i32;
                let (w, h) = (cell_w, cell_h);
                let rect = match orientation {
                    'R' => {
                        let nx = full_h as i32 - (ly + h as i32);
                        Rect::at(x as i32 + nx, y as i32 + lx).of_size(h, w)
                    }
                    'I' => {
                        let nx = full_w as i32 - (lx + w as i32);
                        let ny = full_h as i32 - (ly + h as i32);
                        Rect::at(x as i32 + nx, y as i32 + ny).of_size(w, h)
                    }
                    'B' => {
                        let ny = full_w as i32 - (lx + w as i32);
                        Rect::at(x as i32 + ly, y as i32 + ny).of_size(h, w)
                    }
                    _ => Rect::at(x as i32 + lx, y as i32 + ly).of_size(w, h),
                };
                if reverse_print {
                    self.invert_rect(rect);
                } else {
                    draw_filled_rect_mut(&mut self.canvas, rect, Rgb([0, 0, 0]));
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_1d_barcode(
        &mut self,
        x: u32,
        y: u32,
        orientation: char,
        height: u32,
        module_width: u32,
        ratio: f32,
        data: &str,
        format: BarcodeFormat,
        reverse_print: bool,
        interpretation_line: char,
        interpretation_line_above: char,
        hints: Option<EncodeHints>,
        hints_key: &str,
    ) -> ZplResult<()> {
        Self::validate_barcode_input(height, module_width, data)?;
        let mw = module_width.max(1);
        if symbology::is_two_width(format)
            && (!ratio.is_finite()
                || ratio <= 0.0
                || ((mw as f32) * ratio).round() > super::MAX_IMAGE_DIMENSION as f32)
        {
            return Err(ZplError::BackendError(
                "Invalid barcode wide:narrow ratio".into(),
            ));
        }
        let numeric_data;
        let data = match format {
            BarcodeFormat::EAN_13
            | BarcodeFormat::EAN_8
            | BarcodeFormat::UPC_A
            | BarcodeFormat::UPC_E
            | BarcodeFormat::ITF => {
                numeric_data = data
                    .chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect::<String>();
                &numeric_data
            }
            _ => data,
        };

        let padded_data;
        let data = if format == BarcodeFormat::ITF && data.len() % 2 != 0 {
            padded_data = format!("0{}", data);
            &padded_data
        } else {
            data
        };

        let bit_matrix = barcode_cache::encode_cached(format, data, hints_key, hints.as_ref())?;
        super::validate_raster_dimensions(bit_matrix.getWidth(), bit_matrix.getHeight())?;
        // Two-width symbologies are re-laid-out at the ^BY ratio; every other
        // symbology keeps the encoder's module widths.
        let elements = symbology::matrix_elements(
            &bit_matrix,
            mw,
            symbology::is_two_width(format).then_some(ratio),
        );

        self.draw_elements(
            x,
            y,
            orientation,
            height,
            module_width,
            &elements,
            reverse_print,
            interpretation_line,
            interpretation_line_above,
            data,
        )
    }

    /// Paints a 1-D barcode from pre-computed dot-width elements and draws its
    /// interpretation line.
    ///
    /// Shared by the `rxing`-encoded symbologies and the natively-encoded ones,
    /// so bar placement, rotation and interpretation-line layout stay identical.
    #[allow(clippy::too_many_arguments)]
    fn draw_elements(
        &mut self,
        x: u32,
        y: u32,
        orientation: char,
        height: u32,
        module_width: u32,
        elements: &[symbology::Element],
        reverse_print: bool,
        interpretation_line: char,
        interpretation_line_above: char,
        data: &str,
    ) -> ZplResult<()> {
        let bh = height;
        let bw = elements.iter().try_fold(0u32, |total, element| {
            if element.width == 0 {
                return Err(ZplError::BackendError(
                    "Barcode element has zero width".into(),
                ));
            }
            total
                .checked_add(element.width)
                .ok_or_else(|| ZplError::BackendError("Barcode width overflows".into()))
        })?;

        let (full_w, full_h) = match orientation {
            'N' | 'I' => (bw, bh),
            'R' | 'B' => (bh, bw),
            _ => (bw, bh),
        };
        Self::validate_draw_bounds(x, y, full_w, full_h)?;
        super::validate_raster_dimensions(module_width.max(1), 1)?;

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

        let mut cursor = 0u32;
        for element in elements {
            if element.bar {
                let rect = transform_rect(cursor as i32, 0, element.width, bh);
                if reverse_print {
                    self.invert_rect(rect);
                } else {
                    draw_filled_rect_mut(&mut self.canvas, rect, Rgb([0, 0, 0]));
                }
            }
            cursor += element.width;
        }

        if interpretation_line == 'Y' {
            let (font_char, text_h, text_w, gap) =
                crate::engine::font::interpretation_metrics(module_width);
            let text_y = if interpretation_line_above == 'Y' {
                y.saturating_sub(text_h + gap)
            } else {
                y.checked_add(full_h)
                    .and_then(|v| v.checked_add(gap))
                    .ok_or_else(|| {
                        ZplError::BackendError("Barcode text coordinates overflow".into())
                    })?
            };

            let text_width = self.get_text_width(data, font_char, Some(text_h), Some(text_w));
            let text_x = if full_w > text_width {
                x.checked_add((full_w - text_width) / 2).ok_or_else(|| {
                    ZplError::BackendError("Barcode text coordinates overflow".into())
                })?
            } else {
                x
            };

            self.draw_text(
                text_x,
                text_y,
                font_char,
                Some(text_h),
                Some(text_w),
                'N',
                data,
                reverse_print,
                None,
            )?;
        }

        Ok(())
    }

    /// Paints a POSTNET symbol.
    ///
    /// POSTNET is the one supported symbology that encodes data in bar *height*
    /// rather than bar width, so it needs its own painter: all bars share the
    /// module width and sit on a fixed pitch, and a binary 0 is a bottom-aligned
    /// half-height bar.
    #[allow(clippy::too_many_arguments)]
    fn draw_postnet(
        &mut self,
        x: u32,
        y: u32,
        orientation: char,
        height: u32,
        module_width: u32,
        data: &str,
        reverse_print: bool,
        interpretation_line: char,
        interpretation_line_above: char,
    ) -> ZplResult<()> {
        let bars = symbology::postnet_bars(data);
        let (bar_w, gap) = symbology::postnet_pitch(module_width);
        let pitch = bar_w
            .checked_add(gap)
            .ok_or_else(|| ZplError::BackendError("POSTNET pitch overflows".into()))?;
        let bar_count = u32::try_from(bars.len())
            .map_err(|_| ZplError::BackendError("POSTNET bar count overflows".into()))?;
        let full_w = bar_count
            .checked_mul(pitch)
            .and_then(|w| w.checked_sub(gap))
            .ok_or_else(|| ZplError::BackendError("POSTNET width overflows".into()))?;
        let (draw_w, draw_h) = if matches!(orientation, 'R' | 'B') {
            (height, full_w)
        } else {
            (full_w, height)
        };
        Self::validate_draw_bounds(x, y, draw_w, draw_h)?;
        let short_h = ((height as f32) * symbology::POSTNET_SHORT_RATIO).round() as u32;
        if short_h == 0 {
            return Err(ZplError::BackendError(
                "POSTNET short bars have zero height".into(),
            ));
        }

        for (i, bar) in bars.iter().enumerate() {
            let h = match bar {
                symbology::BarHeight::Full => height,
                symbology::BarHeight::Half => short_h,
            };
            // Short bars are bottom-aligned with the full-height bars.
            let top = height.saturating_sub(h);
            let lx = i as u32 * pitch;
            let rect = match orientation {
                'R' => Rect::at(x as i32 + top as i32, y as i32 + lx as i32).of_size(h, bar_w),
                'I' => Rect::at(
                    x as i32 + (full_w - lx - bar_w) as i32,
                    y as i32 + (height - h - top) as i32,
                )
                .of_size(bar_w, h),
                'B' => Rect::at(
                    x as i32 + (height - h - top) as i32,
                    y as i32 + (full_w - lx - bar_w) as i32,
                )
                .of_size(h, bar_w),
                _ => Rect::at(x as i32 + lx as i32, y as i32 + top as i32).of_size(bar_w, h),
            };
            if reverse_print {
                self.invert_rect(rect);
            } else {
                draw_filled_rect_mut(&mut self.canvas, rect, Rgb([0, 0, 0]));
            }
        }

        if interpretation_line == 'Y' {
            let (font_char, text_h, text_w, gap) =
                crate::engine::font::interpretation_metrics(module_width);
            let text_y = if interpretation_line_above == 'Y' {
                y.saturating_sub(text_h + gap)
            } else {
                y.checked_add(height)
                    .and_then(|v| v.checked_add(gap))
                    .ok_or_else(|| {
                        ZplError::BackendError("POSTNET text coordinates overflow".into())
                    })?
            };
            let digits: String = data.chars().filter(|c| c.is_ascii_digit()).collect();
            let text_width = self.get_text_width(&digits, font_char, Some(text_h), Some(text_w));
            let text_x = if full_w > text_width {
                x.checked_add((full_w - text_width) / 2).ok_or_else(|| {
                    ZplError::BackendError("POSTNET text coordinates overflow".into())
                })?
            } else {
                x
            };
            self.draw_text(
                text_x,
                text_y,
                font_char,
                Some(text_h),
                Some(text_w),
                'N',
                &digits,
                reverse_print,
                None,
            )?;
        }

        Ok(())
    }
}
