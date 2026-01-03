use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    ast::parse_zpl,
    engine::{backend, common, font, intr},
    FontManager, ZplError, ZplResult,
};

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
        let replace_vars = |s: &str| -> String {
            let mut result = s.to_string();
            for (k, v) in variables {
                result = result.replace(&format!("{{{{{}}}}}", k), v);
            }
            result
        };

        let w_dots = self.width.clone().to_dots(self.resolution);
        let h_dots = self.height.clone().to_dots(self.resolution);
        let font_manager = if let Some(fonts) = &self.fonts {
            fonts.clone()
        } else {
            Arc::new(FontManager::default())
        };

        backend.setup_page(w_dots as f64, h_dots as f64, self.resolution.dpi());
        backend.setup_font_manager(&font_manager);

        for instruction in &self.instructions {
            match instruction {
                common::ZplInstruction::Text {
                    x,
                    y,
                    font,
                    height,
                    width,
                    text,
                    reverse_print,
                    color,
                } => {
                    backend.draw_text(
                        *x,
                        *y,
                        *font,
                        *height,
                        *width,
                        replace_vars(text),
                        *reverse_print,
                        color.clone(),
                    )?;
                }
                common::ZplInstruction::GraphicBox {
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
                    x,
                    y,
                    width,
                    height,
                    data,
                    reverse_print,
                } => {
                    backend.draw_graphic_field(
                        *x,
                        *y,
                        *width,
                        *height,
                        data.clone(),
                        *reverse_print,
                    )?;
                }
                common::ZplInstruction::Code128 {
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
                        replace_vars(data),
                        *reverse_print,
                    )?;
                }
                common::ZplInstruction::QRCode {
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
                        replace_vars(data),
                        *reverse_print,
                    )?;
                }
                common::ZplInstruction::Code39 {
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
                        replace_vars(data),
                        *reverse_print,
                    )?;
                }
                common::ZplInstruction::CustomImage {
                    x,
                    y,
                    width,
                    height,
                    data,
                } => {
                    backend.draw_graphic_image_custom(*x, *y, *width, *height, data.clone())?;
                }
            }
        }

        let result = backend.finalize()?;

        Ok(result)
    }
}
