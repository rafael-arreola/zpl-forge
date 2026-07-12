use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    FontManager, ZplError, ZplResult,
    ast::parse_zpl,
    engine::{backend, common, font, intr},
};

/// Measures the advance width of `text` in dots for the given ZPL font spec.
fn measure_text_dots(
    fm: &font::FontManager,
    font_char: char,
    height: Option<u32>,
    width: Option<u32>,
    text: &str,
) -> u32 {
    fm.measure_text(font_char, height, width, text)
}

/// Greedy word-wrap for `^FB`: fits words into `max_width` dots, hard-breaking
/// words that are longer than a full line. `\&` acts as an explicit line break.
fn wrap_text_block<F: Fn(&str) -> u32>(text: &str, max_width: u32, measure: F) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    for segment in text.split("\\&") {
        if max_width == 0 {
            lines.push(segment.trim().to_string());
            continue;
        }

        let mut current = String::new();
        for word in segment.split_whitespace() {
            let candidate = if current.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current, word)
            };

            if measure(&candidate) <= max_width {
                current = candidate;
                continue;
            }

            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }

            // The word alone may still overflow: hard-break it by characters.
            if measure(word) > max_width {
                let mut piece = String::new();
                for ch in word.chars() {
                    piece.push(ch);
                    if measure(&piece) > max_width && piece.chars().count() > 1 {
                        piece.pop();
                        lines.push(std::mem::take(&mut piece));
                        piece.push(ch);
                    }
                }
                current = piece;
            } else {
                current = word.to_string();
            }
        }
        lines.push(current);
    }

    lines
}

/// The main entry point for processing and rendering ZPL labels.
///
/// `ZplEngine` holds the parsed instructions, label dimensions, and configuration
/// required to render a label using a specific backend.
#[derive(Debug)]
pub struct ZplEngine {
    instructions: Vec<common::ZplInstruction>,
    width: common::Unit,
    height: common::Unit,
    resolution: common::Resolution,
    fonts: Option<Arc<font::FontManager>>,
}

impl ZplEngine {
    /// Creates a new `ZplEngine` instance by parsing a ZPL string.
    ///
    /// # Arguments
    /// * `zpl` - The raw ZPL string to parse.
    /// * `width` - The physical width of the label.
    /// * `height` - The physical height of the label.
    /// * `resolution` - The printing resolution (DPI).
    ///
    /// # Errors
    /// Returns an error if the ZPL is invalid or if the instruction building fails.
    pub fn new(
        zpl: &str,
        width: common::Unit,
        height: common::Unit,
        resolution: common::Resolution,
    ) -> ZplResult<Self> {
        let commands = parse_zpl(zpl)?;
        if commands.is_empty() {
            return Err(ZplError::EmptyInput);
        }

        let instructions = intr::ZplInstructionBuilder::new(commands);
        let instructions = instructions.build()?;

        Ok(Self {
            instructions,
            width,
            height,
            resolution,
            fonts: None,
        })
    }

    /// Sets the font manager to be used during rendering.
    ///
    /// If no font manager is provided, a default one will be used.
    pub fn set_fonts(&mut self, fonts: Arc<font::FontManager>) {
        self.fonts = Some(fonts);
    }

    /// Renders the parsed instructions using the provided backend.
    ///
    /// # Arguments
    /// * `backend` - An implementation of `ZplForgeBackend` (e.g., PNG, PDF).
    /// * `variables` - A map of template variables to replace in text fields (format: `{{key}}`).
    ///
    /// # Errors
    /// Returns an error if rendering fails at the backend level.
    pub fn render<B: backend::ZplForgeBackend>(
        &self,
        mut backend: B,
        variables: &HashMap<String, String>,
    ) -> ZplResult<Vec<u8>> {
        let w_dots = self.width.clone().to_dots(self.resolution);
        let h_dots = self.height.clone().to_dots(self.resolution);
        let font_manager = if let Some(fonts) = &self.fonts {
            fonts.clone()
        } else {
            Arc::new(FontManager::default())
        };

        backend.setup_page(w_dots as f64, h_dots as f64, self.resolution.dpi());
        backend.setup_font_manager(&font_manager);

        self.render_instructions(&mut backend, variables, &font_manager)?;

        let result = backend.finalize()?;

        Ok(result)
    }

    /// Renders the same parsed template multiple times into a single multi-page document,
    /// using a different set of variables for each page.
    ///
    /// # Arguments
    /// * `backend` - An implementation of `ZplForgeBackend` that supports multi-page (e.g., `PdfNativeBackend`).
    /// * `pages_variables` - A slice of maps, where each map corresponds to a single page's variable assignments.
    ///
    /// # Errors
    /// Returns an error if rendering fails at the backend level.
    pub fn render_pages<B: backend::ZplForgeBackend>(
        &self,
        mut backend: B,
        pages_variables: &[HashMap<String, String>],
    ) -> ZplResult<Vec<u8>> {
        let w_dots = self.width.clone().to_dots(self.resolution);
        let h_dots = self.height.clone().to_dots(self.resolution);
        let font_manager = if let Some(fonts) = &self.fonts {
            fonts.clone()
        } else {
            Arc::new(FontManager::default())
        };

        backend.setup_page(w_dots as f64, h_dots as f64, self.resolution.dpi());
        backend.setup_font_manager(&font_manager);

        for (page_idx, variables) in pages_variables.iter().enumerate() {
            if page_idx > 0 {
                backend.new_page()?;
            }
            self.render_instructions(&mut backend, variables, &font_manager)?;
        }

        let result = backend.finalize()?;

        Ok(result)
    }

    /// Helper method to execute the parsed instructions on the provided backend.
    fn render_instructions<B: backend::ZplForgeBackend>(
        &self,
        backend: &mut B,
        variables: &HashMap<String, String>,
        font_manager: &FontManager,
    ) -> ZplResult<()> {
        fn replace_vars<'a>(
            s: &'a str,
            variables: &HashMap<String, String>,
        ) -> std::borrow::Cow<'a, str> {
            if variables.is_empty() || !s.contains("{{") {
                return std::borrow::Cow::Borrowed(s);
            }

            let mut result = String::new();
            let mut last_pos = 0;
            let mut found = false;
            let mut cursor = 0;

            while let Some(start_offset) = s[cursor..].find("{{") {
                let start = cursor + start_offset;
                if let Some(end_offset) = s[start + 2..].find("}}") {
                    let end = start + 2 + end_offset;
                    let key = &s[start + 2..end];
                    if let Some(value) = variables.get(key) {
                        if !found {
                            result.reserve(s.len());
                            found = true;
                        }
                        result.push_str(&s[last_pos..start]);
                        result.push_str(value);
                        last_pos = end + 2;
                        cursor = last_pos;
                        continue;
                    }
                }
                cursor = start + 2;
            }

            if found {
                result.push_str(&s[last_pos..]);
                std::borrow::Cow::Owned(result)
            } else {
                std::borrow::Cow::Borrowed(s)
            }
        }

        for instruction in &self.instructions {
            if let common::ZplInstruction::PageBreak = instruction {
                backend.new_page()?;
                continue;
            }

            let condition = match instruction {
                common::ZplInstruction::PageBreak => continue,
                common::ZplInstruction::Text { condition, .. } => condition,
                common::ZplInstruction::GraphicBox { condition, .. } => condition,
                common::ZplInstruction::GraphicCircle { condition, .. } => condition,
                common::ZplInstruction::GraphicEllipse { condition, .. } => condition,
                common::ZplInstruction::GraphicField { condition, .. } => condition,
                common::ZplInstruction::CustomImage { condition, .. } => condition,
                common::ZplInstruction::Code128 { condition, .. } => condition,
                common::ZplInstruction::QRCode { condition, .. } => condition,
                common::ZplInstruction::Code39 { condition, .. } => condition,
                common::ZplInstruction::DataMatrix { condition, .. } => condition,
                common::ZplInstruction::Pdf417 { condition, .. } => condition,
                common::ZplInstruction::Barcode1D { condition, .. } => condition,
                common::ZplInstruction::GraphicDiagonal { condition, .. } => condition,
            };

            if let Some((var, expected)) = condition
                && variables.get(var) != Some(expected)
            {
                continue;
            }

            match instruction {
                common::ZplInstruction::PageBreak => {}
                common::ZplInstruction::Text {
                    condition: _,
                    x,
                    y,
                    font,
                    height,
                    width,
                    orientation,
                    text,
                    reverse_print,
                    color,
                    block,
                } => {
                    let resolved = replace_vars(text, variables);

                    let Some(b) = block else {
                        backend.draw_text(
                            *x,
                            *y,
                            *font,
                            *height,
                            *width,
                            *orientation,
                            &resolved,
                            *reverse_print,
                            color.clone(),
                        )?;
                        continue;
                    };

                    // ^FB: wrap into lines, justify, and place each line
                    // according to the field orientation.
                    let measure =
                        |s: &str| measure_text_dots(font_manager, *font, *height, *width, s);
                    let lines = wrap_text_block(&resolved, b.width, measure);
                    let n_lines = lines.len().min(b.max_lines.max(1) as usize);

                    let font_h = height.unwrap_or(9) as i32;
                    let line_advance = (font_h + b.line_spacing).max(1);
                    let block_span = (n_lines as i32 - 1) * line_advance;

                    for (i, line) in lines.iter().take(n_lines).enumerate() {
                        if line.is_empty() {
                            continue;
                        }
                        let lw = measure(line) as i32;
                        let indent = if i > 0 { b.indent as i32 } else { 0 };
                        let avail = (b.width as i32 - indent).max(0);
                        let jx = indent
                            + match b.justification {
                                'C' => (avail - lw).max(0) / 2,
                                'R' => (avail - lw).max(0),
                                _ => 0,
                            };
                        let ly = i as i32 * line_advance;

                        // Cell top-left offset, rotated with the field.
                        let (dx, dy) = match orientation {
                            'R' => (block_span - ly, jx),
                            'I' => (b.width as i32 - jx - lw, block_span - ly),
                            'B' => (ly, b.width as i32 - jx - lw),
                            _ => (jx, ly),
                        };

                        let fx = (*x as i32 + dx).max(0) as u32;
                        let fy = (*y as i32 + dy).max(0) as u32;
                        backend.draw_text(
                            fx,
                            fy,
                            *font,
                            *height,
                            *width,
                            *orientation,
                            line,
                            *reverse_print,
                            color.clone(),
                        )?;
                    }
                }
                common::ZplInstruction::GraphicBox {
                    condition: _,
                    x,
                    y,
                    width,
                    height,
                    thickness,
                    color,
                    custom_color,
                    rounding,
                    reverse_print,
                } => {
                    backend.draw_graphic_box(
                        *x,
                        *y,
                        *width,
                        *height,
                        *thickness,
                        *color,
                        custom_color.clone(),
                        *rounding,
                        *reverse_print,
                    )?;
                }
                common::ZplInstruction::GraphicCircle {
                    condition: _,
                    x,
                    y,
                    radius,
                    thickness,
                    color,
                    custom_color,
                    reverse_print,
                } => {
                    backend.draw_graphic_circle(
                        *x,
                        *y,
                        *radius,
                        *thickness,
                        *color,
                        custom_color.clone(),
                        *reverse_print,
                    )?;
                }
                common::ZplInstruction::GraphicEllipse {
                    condition: _,
                    x,
                    y,
                    width,
                    height,
                    thickness,
                    color,
                    custom_color,
                    reverse_print,
                } => {
                    backend.draw_graphic_ellipse(
                        *x,
                        *y,
                        *width,
                        *height,
                        *thickness,
                        *color,
                        custom_color.clone(),
                        *reverse_print,
                    )?;
                }
                common::ZplInstruction::GraphicField {
                    condition: _,
                    x,
                    y,
                    width,
                    height,
                    data,
                    reverse_print,
                } => {
                    backend.draw_graphic_field(*x, *y, *width, *height, data, *reverse_print)?;
                }
                common::ZplInstruction::Code128 {
                    condition: _,
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
                } => {
                    backend.draw_code128(
                        *x,
                        *y,
                        *orientation,
                        *height,
                        *module_width,
                        *interpretation_line,
                        *interpretation_line_above,
                        *check_digit,
                        *mode,
                        &replace_vars(data, variables),
                        *reverse_print,
                    )?;
                }
                common::ZplInstruction::QRCode {
                    condition: _,
                    x,
                    y,
                    orientation,
                    model,
                    magnification,
                    error_correction,
                    mask,
                    data,
                    reverse_print,
                } => {
                    backend.draw_qr_code(
                        *x,
                        *y,
                        *orientation,
                        *model,
                        *magnification,
                        *error_correction,
                        *mask,
                        &replace_vars(data, variables),
                        *reverse_print,
                    )?;
                }
                common::ZplInstruction::Barcode1D {
                    condition: _,
                    kind,
                    x,
                    y,
                    orientation,
                    height,
                    module_width,
                    interpretation_line,
                    interpretation_line_above,
                    data,
                    reverse_print,
                } => {
                    backend.draw_barcode_1d(
                        *kind,
                        *x,
                        *y,
                        *orientation,
                        *height,
                        *module_width,
                        *interpretation_line,
                        *interpretation_line_above,
                        &replace_vars(data, variables),
                        *reverse_print,
                    )?;
                }
                common::ZplInstruction::GraphicDiagonal {
                    condition: _,
                    x,
                    y,
                    width,
                    height,
                    thickness,
                    color,
                    custom_color,
                    diagonal_orientation,
                    reverse_print,
                } => {
                    backend.draw_graphic_diagonal(
                        *x,
                        *y,
                        *width,
                        *height,
                        *thickness,
                        *color,
                        custom_color.clone(),
                        *diagonal_orientation,
                        *reverse_print,
                    )?;
                }
                common::ZplInstruction::DataMatrix {
                    condition: _,
                    x,
                    y,
                    orientation,
                    module_size,
                    data,
                    reverse_print,
                } => {
                    backend.draw_datamatrix(
                        *x,
                        *y,
                        *orientation,
                        *module_size,
                        &replace_vars(data, variables),
                        *reverse_print,
                    )?;
                }
                common::ZplInstruction::Pdf417 {
                    condition: _,
                    x,
                    y,
                    orientation,
                    row_height,
                    module_width,
                    security_level,
                    data,
                    reverse_print,
                } => {
                    backend.draw_pdf417(
                        *x,
                        *y,
                        *orientation,
                        *row_height,
                        *module_width,
                        *security_level,
                        &replace_vars(data, variables),
                        *reverse_print,
                    )?;
                }
                common::ZplInstruction::Code39 {
                    condition: _,
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
                } => {
                    backend.draw_code39(
                        *x,
                        *y,
                        *orientation,
                        *check_digit,
                        *height,
                        *module_width,
                        *interpretation_line,
                        *interpretation_line_above,
                        &replace_vars(data, variables),
                        *reverse_print,
                    )?;
                }
                common::ZplInstruction::CustomImage {
                    condition: _,
                    x,
                    y,
                    width,
                    height,
                    data,
                } => {
                    backend.draw_graphic_image_custom(*x, *y, *width, *height, data)?;
                }
            }
        }

        Ok(())
    }
}
