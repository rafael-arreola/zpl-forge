use super::{common, state};
use crate::ast::cmd;
use crate::tools;
use crate::ZplResult;

/// A builder that converts a sequence of AST commands into renderable instructions.
///
/// It maintains a state machine to track the current label configuration (position,
/// font, barcodes) and emits a `ZplInstruction` whenever a field separator is encountered.
pub struct ZplInstructionBuilder {
    /// The stream of commands parsed from ZPL.
    commands: Vec<cmd::Command>,
    /// The current state of the builder.
    state: state::ZplInstructionState,
}

impl ZplInstructionBuilder {
    /// Creates a new builder with the given list of commands.
    pub fn new(commands: Vec<cmd::Command>) -> Self {
        Self {
            commands,
            state: state::ZplInstructionState::default(),
        }
    }

    /// Processes the commands and returns a vector of instructions.
    ///
    /// # Errors
    /// Returns an error if the command stream contains invalid state transitions
    /// or if data conversion fails.
    pub fn build(mut self) -> ZplResult<Vec<common::ZplInstruction>> {
        let mut instructions = Vec::new();
        let commands = self.commands.clone();

        for command in &commands {
            match command {
                cmd::Command::FieldOrigin { x, y } => {
                    if let Some(x) = x {
                        self.state.position.x = *x;
                    }
                    if let Some(y) = y {
                        self.state.position.y = *y;
                    }
                }

                cmd::Command::FieldTypeset { x, y } => {
                    if let Some(x) = x {
                        self.state.typeset.x = self.state.typeset.x.saturating_add(*x);
                    }
                    if let Some(y) = y {
                        self.state.typeset.y = self.state.typeset.y.saturating_add(*y);
                    }
                }

                cmd::Command::FieldReverse => {
                    self.state.reverse = !self.state.reverse;
                }

                cmd::Command::FontSpec {
                    font_name,
                    height,
                    width,
                } => {
                    self.state.font.font_name = *font_name;
                    if let Some(h) = height {
                        self.state.font.height = Some(*h);
                    }
                    if let Some(w) = width {
                        self.state.font.width = Some(*w);
                    }
                }

                cmd::Command::FontSpecFull {
                    font_name,
                    orientation,
                    height,
                    width,
                } => {
                    self.state.font.font_name = *font_name;
                    if let Some(o) = orientation {
                        self.state.font.orientation = Some(*o);
                    }
                    if let Some(h) = height {
                        self.state.font.height = Some(*h);
                    }
                    if let Some(w) = width {
                        self.state.font.width = Some(*w);
                    }
                }

                cmd::Command::FieldData { data } => {
                    self.state.value = Some(data.clone());
                }

                cmd::Command::GraphicBox {
                    width,
                    height,
                    border_thickness,
                    line_color,
                    corner_rounding,
                } => {
                    self.state.metrics.width = *width;
                    self.state.metrics.height = *height;
                    self.state.metrics.thickness = border_thickness.unwrap_or(1);
                    self.state.attributes.line_color = *line_color;
                    self.state.params.rounding = corner_rounding.unwrap_or(0);
                    self.state.instruction_type = Some(state::ZplInstructionType::GraphicBox);
                }

                cmd::Command::GraphicCircle {
                    diameter,
                    border_thickness,
                    line_color,
                } => {
                    self.state.metrics.width = diameter.unwrap_or(0);
                    self.state.metrics.thickness = border_thickness.unwrap_or(1);
                    self.state.attributes.line_color = *line_color;
                    self.state.instruction_type = Some(state::ZplInstructionType::GraphicCircle);
                }

                cmd::Command::GraphicEllipse {
                    width,
                    height,
                    border_thickness,
                    line_color,
                } => {
                    self.state.metrics.width = width.unwrap_or(0);
                    self.state.metrics.height = height.unwrap_or(0);
                    self.state.metrics.thickness = border_thickness.unwrap_or(1);
                    self.state.attributes.line_color = *line_color;
                    self.state.instruction_type = Some(state::ZplInstructionType::GraphicEllipse);
                }

                cmd::Command::GraphicTextColor { color } => {
                    self.state.font.color = Some(color.clone());
                }

                cmd::Command::GraphicLineColor { color } => {
                    self.state.attributes.custom_line_color = Some(color.clone());
                }

                cmd::Command::GraphicField {
                    compression_type,
                    binary_byte_count: _,
                    graphic_field_count,
                    bytes_per_row,
                    data,
                } => {
                    let compression_type = compression_type.unwrap_or('A');
                    let bytes: Vec<u8> = match compression_type {
                        'A' => {
                            let bpr_val = bytes_per_row.unwrap_or(0) as usize;
                            tools::zpl_decode(data, bpr_val)
                        }
                        'B' => {
                            // method not implemented
                            break;
                        }
                        'C' => {
                            // method not implemented
                            break;
                        }
                        'Z' => {
                            // method not implemented
                            break;
                        }
                        _ => {
                            tracing::warn!("Unsupported compression type: {}", compression_type);
                            break;
                        }
                    };

                    if let Some(bpr) = bytes_per_row {
                        self.state.metrics.width = bpr.saturating_mul(8);
                        if let Some(total) = graphic_field_count {
                            if *bpr > 0 {
                                self.state.metrics.height = total / bpr;
                            }
                        }
                    }
                    self.state.graphic_data = Some(bytes);
                    self.state.instruction_type = Some(state::ZplInstructionType::GraphicField);
                }

                // Barcode
                cmd::Command::BarcodeDefault {
                    module_width,
                    ratio,
                    height,
                } => {
                    if let Some(w) = module_width {
                        self.state.barcode_metrics.thickness = *w;
                    }
                    if let Some(h) = height {
                        self.state.barcode_metrics.height = *h;
                    }
                    if let Some(r) = ratio {
                        self.state.params.ratio = Some(*r as f64);
                    }
                }

                cmd::Command::Code128 {
                    orientation,
                    height,
                    interpretation_line,
                    interpretation_line_above,
                    check_digit,
                    mode,
                } => {
                    self.state.attributes.orientation = *orientation;
                    // Use command height OR default barcode height OR 10
                    self.state.metrics.height =
                        height.unwrap_or(if self.state.barcode_metrics.height > 0 {
                            self.state.barcode_metrics.height
                        } else {
                            10
                        });
                    self.state.attributes.interpretation_line = *interpretation_line;
                    self.state.attributes.interpretation_above = *interpretation_line_above;
                    self.state.attributes.check_digit = *check_digit;
                    self.state.attributes.mode = *mode;
                    self.state.instruction_type = Some(state::ZplInstructionType::Code128);
                }

                cmd::Command::Code39 {
                    orientation,
                    check_digit,
                    height,
                    interpretation_line,
                    interpretation_line_above,
                } => {
                    self.state.attributes.orientation = *orientation;
                    self.state.attributes.check_digit = *check_digit;
                    self.state.metrics.height =
                        height.unwrap_or(if self.state.barcode_metrics.height > 0 {
                            self.state.barcode_metrics.height
                        } else {
                            10
                        });
                    self.state.attributes.interpretation_line = *interpretation_line;
                    self.state.attributes.interpretation_above = *interpretation_line_above;
                    self.state.instruction_type = Some(state::ZplInstructionType::Code39);
                }

                cmd::Command::QRCode {
                    orientation,
                    model,
                    magnification,
                    error_correction,
                    mask,
                } => {
                    self.state.attributes.orientation = *orientation;
                    self.state.params.model = model.unwrap_or(2);
                    self.state.metrics.thickness =
                        magnification.unwrap_or(if self.state.barcode_metrics.thickness > 0 {
                            self.state.barcode_metrics.thickness
                        } else {
                            2
                        });
                    self.state.attributes.error_correction = *error_correction;
                    self.state.params.mask = mask.unwrap_or(7);
                    self.state.instruction_type = Some(state::ZplInstructionType::QRCode);
                }

                cmd::Command::CustomImage {
                    width,
                    height,
                    data,
                } => {
                    self.state.metrics.width = *width;
                    self.state.metrics.height = *height;
                    self.state.value = Some(data.clone());
                    self.state.instruction_type = Some(state::ZplInstructionType::CustomImage);
                }

                // Apply the instruction with the current state
                cmd::Command::FieldSeparator => {
                    let x = self.state.position.x;
                    let y = self.state.position.y;
                    let data = self.state.value.clone().unwrap_or_default();
                    let reverse_print = self.state.reverse;

                    if let Some(instr_type) = &self.state.instruction_type {
                        match instr_type {
                            state::ZplInstructionType::GraphicBox => {
                                instructions.push(common::ZplInstruction::GraphicBox {
                                    x,
                                    y,
                                    width: self.state.metrics.width,
                                    height: self.state.metrics.height,
                                    thickness: self.state.metrics.thickness,
                                    color: self.state.attributes.line_color.unwrap_or('B'),
                                    custom_color: self.state.attributes.custom_line_color.clone(),
                                    rounding: self.state.params.rounding,
                                    reverse_print,
                                });
                            }
                            state::ZplInstructionType::GraphicCircle => {
                                instructions.push(common::ZplInstruction::GraphicCircle {
                                    x,
                                    y,
                                    radius: self.state.metrics.width,
                                    thickness: self.state.metrics.thickness,
                                    color: self.state.attributes.line_color.unwrap_or('B'),
                                    custom_color: self.state.attributes.custom_line_color.clone(),
                                    reverse_print,
                                });
                            }
                            state::ZplInstructionType::GraphicEllipse => {
                                instructions.push(common::ZplInstruction::GraphicEllipse {
                                    x,
                                    y,
                                    width: self.state.metrics.width,
                                    height: self.state.metrics.height,
                                    thickness: self.state.metrics.thickness,
                                    color: self.state.attributes.line_color.unwrap_or('B'),
                                    custom_color: self.state.attributes.custom_line_color.clone(),
                                    reverse_print,
                                });
                            }
                            state::ZplInstructionType::GraphicField => {
                                if let Some(g_data) = &self.state.graphic_data {
                                    instructions.push(common::ZplInstruction::GraphicField {
                                        x,
                                        y,
                                        width: self.state.metrics.width,
                                        height: self.state.metrics.height,
                                        data: g_data.clone(),
                                        reverse_print,
                                    });
                                }
                            }
                            state::ZplInstructionType::CustomImage => {
                                instructions.push(common::ZplInstruction::CustomImage {
                                    x,
                                    y,
                                    width: self.state.metrics.width,
                                    height: self.state.metrics.height,
                                    data: data.clone(),
                                });
                            }
                            state::ZplInstructionType::Code128 => {
                                instructions.push(common::ZplInstruction::Code128 {
                                    x,
                                    y,
                                    orientation: self.state.attributes.orientation.unwrap_or('N'),
                                    height: self.state.metrics.height,
                                    module_width: if self.state.barcode_metrics.thickness > 0 {
                                        self.state.barcode_metrics.thickness
                                    } else {
                                        2
                                    },
                                    interpretation_line: self
                                        .state
                                        .attributes
                                        .interpretation_line
                                        .unwrap_or('Y'),
                                    interpretation_line_above: self
                                        .state
                                        .attributes
                                        .interpretation_above
                                        .unwrap_or('N'),
                                    check_digit: self.state.attributes.check_digit.unwrap_or('N'),
                                    mode: self.state.attributes.mode.unwrap_or('N'),
                                    data: data.clone(),
                                    reverse_print,
                                });
                            }
                            state::ZplInstructionType::Code39 => {
                                instructions.push(common::ZplInstruction::Code39 {
                                    x,
                                    y,
                                    orientation: self.state.attributes.orientation.unwrap_or('N'),
                                    check_digit: self.state.attributes.check_digit.unwrap_or('N'),
                                    height: self.state.metrics.height,
                                    module_width: if self.state.barcode_metrics.thickness > 0 {
                                        self.state.barcode_metrics.thickness
                                    } else {
                                        2
                                    },
                                    interpretation_line: self
                                        .state
                                        .attributes
                                        .interpretation_line
                                        .unwrap_or('Y'),
                                    interpretation_line_above: self
                                        .state
                                        .attributes
                                        .interpretation_above
                                        .unwrap_or('N'),
                                    data: data.clone(),
                                    reverse_print,
                                });
                            }
                            state::ZplInstructionType::QRCode => {
                                instructions.push(common::ZplInstruction::QRCode {
                                    x,
                                    y,
                                    orientation: self.state.attributes.orientation.unwrap_or('N'),
                                    model: self.state.params.model,
                                    magnification: self.state.metrics.thickness,
                                    error_correction: self
                                        .state
                                        .attributes
                                        .error_correction
                                        .unwrap_or('M'),
                                    mask: self.state.params.mask,
                                    data: data.clone(),
                                    reverse_print,
                                });
                            }
                            state::ZplInstructionType::Text => {
                                instructions.push(common::ZplInstruction::Text {
                                    x,
                                    y,
                                    font: self.state.font.font_name,
                                    height: self.state.font.height,
                                    width: self.state.font.width,
                                    text: data.clone(),
                                    reverse_print,
                                    color: self.state.font.color.clone(),
                                });
                            }
                        }
                    } else if let Some(text) = &self.state.value {
                        instructions.push(common::ZplInstruction::Text {
                            x,
                            y,
                            font: self.state.font.font_name,
                            height: self.state.font.height,
                            width: self.state.font.width,
                            text: text.clone(),
                            reverse_print,
                            color: self.state.font.color.clone(),
                        });
                    }

                    self.state.value = None;
                    self.state.instruction_type = None;
                    self.state.graphic_data = None;
                    self.state.reverse = false;
                }

                _ => {}
            }
        }

        Ok(instructions)
    }
}
