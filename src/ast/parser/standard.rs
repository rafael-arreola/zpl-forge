use nom::{
    bytes::complete::{tag, take_till},
    combinator::{cut, map, opt},
    Parser,
};

use super::{opt_param, param, parse_char, parse_f32, parse_u32, parse_xy, Res, Span};
use crate::ast::cmd;
use crate::ast::commons::{Justification, YesNo};

/// ^XA - Start Format
pub fn cmd_xa(input: Span) -> Res<cmd::Command> {
    map(tag("^XA"), |_| cmd::Command::StartFormat).parse(input)
}

/// ^XZ - End Format
pub fn cmd_xz(input: Span) -> Res<cmd::Command> {
    map(tag("^XZ"), |_| cmd::Command::EndFormat).parse(input)
}

/// ^LH - Label Home
pub fn cmd_lh(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^LH").parse(input)?;
    let (input, (x, y)) = cut(parse_xy).parse(input)?;
    Ok((input, cmd::Command::LabelHome { x, y }))
}

/// ^LL - Label Length
pub fn cmd_ll(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^LL").parse(input)?;
    let (input, length) = cut(opt_param(parse_u32)).parse(input)?;
    Ok((input, cmd::Command::LabelLength { length }))
}

/// ^FO - Field Origin
pub fn cmd_fo(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^FO").parse(input)?;
    let (input, (x, y)) = cut(parse_xy).parse(input)?;
    Ok((input, cmd::Command::FieldOrigin { x, y }))
}

/// ^FT - Field Typeset
pub fn cmd_ft(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^FT").parse(input)?;
    let (input, (x, y)) = cut(parse_xy).parse(input)?;
    Ok((input, cmd::Command::FieldTypeset { x, y }))
}

/// ^FS - Field Separator
pub fn cmd_fs(input: Span) -> Res<cmd::Command> {
    map(tag("^FS"), |_| cmd::Command::FieldSeparator).parse(input)
}

/// ^LR - Label Reverse
pub fn cmd_lr(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^LR").parse(input)?;
    let (input, val_opt) = cut(opt_param(parse_char)).parse(input)?;
    Ok((
        input,
        cmd::Command::LabelReverse {
            reverse: val_opt.map(|c: char| YesNo::from(c)),
        },
    ))
}

/// ^FX - Comment
pub fn cmd_fx(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^FX").parse(input)?;
    let (input, data) = take_till(|c| c == '^').parse(input)?;
    Ok((
        input,
        cmd::Command::Comment {
            text: data.trim().to_owned(),
        },
    ))
}

/// ^A - Font Specification (Full)
pub fn cmd_a(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^A").parse(input)?;
    let (input, font) = cut(parse_char).parse(input)?;
    let (input, orientation_opt) = opt_param(parse_char).parse(input)?;
    let (input, height_opt) = param(parse_u32).parse(input).unwrap_or((input, None));
    let (input, width_opt) = param(parse_u32).parse(input).unwrap_or((input, None));

    Ok((
        input,
        cmd::Command::FontSpecFull {
            font_name: font,
            orientation: orientation_opt,
            height: height_opt,
            width: width_opt,
        },
    ))
}

/// ^CF - Change Default Font
pub fn cmd_cf(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^CF").parse(input)?;
    let (input, font) = cut(parse_char).parse(input)?;
    let (input, height_opt) = param(parse_u32).parse(input).unwrap_or((input, None));
    let (input, width_opt) = param(parse_u32).parse(input).unwrap_or((input, None));

    Ok((
        input,
        cmd::Command::FontSpec {
            font_name: font,
            height: height_opt,
            width: width_opt,
        },
    ))
}

/// ^FD - Field Data
pub fn cmd_fd(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^FD").parse(input)?;
    let (input, data) = take_till(|c| c == '^').parse(input)?;
    Ok((
        input,
        cmd::Command::FieldData {
            data: data.trim().to_owned(),
        },
    ))
}

/// ^FB - Field Block
pub fn cmd_fb(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^FB").parse(input)?;
    let (input, w_opt) = cut(opt_param(parse_u32)).parse(input)?;
    let (input, m_opt) = param(parse_u32).parse(input).unwrap_or((input, None));
    let (input, l_opt) = param(parse_u32).parse(input).unwrap_or((input, None));
    let (input, j_opt) = param(parse_char).parse(input).unwrap_or((input, None));
    let (input, i_opt) = param(parse_u32).parse(input).unwrap_or((input, None));

    Ok((
        input,
        cmd::Command::FieldBlock {
            width: w_opt,
            max_lines: m_opt,
            line_spacing: l_opt,
            justification: j_opt.map(|c: char| Justification::from(c)),
            indent: i_opt,
        },
    ))
}

/// ^FR - Field Reverse Print
pub fn cmd_fr(input: Span) -> Res<cmd::Command> {
    map(tag("^FR"), |_| cmd::Command::FieldReverse).parse(input)
}

/// ^GB - Graphic Box
pub fn cmd_gb(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^GB").parse(input)?;
    let (input, width) = cut(parse_u32).parse(input)?;
    let (input, _) = cut(tag(",")).parse(input)?;
    let (input, height) = cut(parse_u32).parse(input)?;

    let (input, border_thickness) = param(parse_u32).parse(input).unwrap_or((input, None));
    let (input, line_color) = param(parse_char).parse(input).unwrap_or((input, None));
    let (input, corner_rounding) = param(parse_u32).parse(input).unwrap_or((input, None));
    Ok((
        input,
        cmd::Command::GraphicBox {
            width,
            height,
            border_thickness,
            line_color,
            corner_rounding,
        },
    ))
}

/// ^GC - Graphic Circle
pub fn cmd_gc(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^GC").parse(input)?;
    let (input, diameter) = cut(opt_param(parse_u32)).parse(input)?;
    let (input, border_thickness) = param(parse_u32).parse(input).unwrap_or((input, None));
    let (input, line_color) = param(parse_char).parse(input).unwrap_or((input, None));

    Ok((
        input,
        cmd::Command::GraphicCircle {
            diameter,
            border_thickness,
            line_color,
        },
    ))
}

/// ^GE - Graphic Ellipse
pub fn cmd_ge(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^GE").parse(input)?;
    let (input, width) = cut(opt_param(parse_u32)).parse(input)?;
    let (input, height) = param(parse_u32).parse(input).unwrap_or((input, None));
    let (input, border_thickness) = param(parse_u32).parse(input).unwrap_or((input, None));
    let (input, line_color) = param(parse_char).parse(input).unwrap_or((input, None));

    Ok((
        input,
        cmd::Command::GraphicEllipse {
            width,
            height,
            border_thickness,
            line_color,
        },
    ))
}

/// ^GF - Graphic Field
pub fn cmd_gf(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^GF").parse(input)?;
    let (input, compression_type) = cut(opt_param(parse_char)).parse(input)?;
    let (input, binary_byte_count) = param(parse_u32).parse(input).unwrap_or((input, None));
    let (input, graphic_field_count) = param(parse_u32).parse(input).unwrap_or((input, None));
    let (input, bytes_per_row) = param(parse_u32).parse(input).unwrap_or((input, None));
    let (input, _) = opt(tag(",")).parse(input)?;
    let (input, raw_data) = take_till(|c| c == '^').parse(input)?;

    Ok((
        input,
        cmd::Command::GraphicField {
            compression_type,
            binary_byte_count,
            graphic_field_count,
            bytes_per_row,
            data: raw_data.trim().to_owned(),
        },
    ))
}

/// ^BC - Code 128 Barcode
pub fn cmd_bc(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^BC").parse(input)?;
    let (rest, args) = cut(take_till(|c| c == '^')).parse(input)?;
    let (args_input, orientation) = opt_param(parse_char).parse(args)?;
    let (args_input, height) = param(parse_u32)
        .parse(args_input)
        .unwrap_or((args_input, None));
    let (args_input, interpretation_line) = param(parse_char)
        .parse(args_input)
        .unwrap_or((args_input, None));
    let (args_input, interpretation_line_above) = param(parse_char)
        .parse(args_input)
        .unwrap_or((args_input, None));
    let (args_input, check_digit) = param(parse_char)
        .parse(args_input)
        .unwrap_or((args_input, None));
    let (_, mode) = param(parse_char)
        .parse(args_input)
        .unwrap_or((args_input, None));

    Ok((
        rest,
        cmd::Command::Code128 {
            orientation,
            height,
            interpretation_line,
            interpretation_line_above,
            check_digit,
            mode,
        },
    ))
}

/// ^BQ - QR Code Barcode
pub fn cmd_bq(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^BQ").parse(input)?;
    let (rest, args) = cut(take_till(|c| c == '^')).parse(input)?;
    let (args_input, orientation) = opt_param(parse_char).parse(args)?;
    let (args_input, model) = param(parse_u32)
        .parse(args_input)
        .unwrap_or((args_input, None));
    let (args_input, magnification) = param(parse_u32)
        .parse(args_input)
        .unwrap_or((args_input, None));
    let (args_input, error_correction) = param(parse_char)
        .parse(args_input)
        .unwrap_or((args_input, None));
    let (_, mask) = param(parse_u32)
        .parse(args_input)
        .unwrap_or((args_input, None));

    Ok((
        rest,
        cmd::Command::QRCode {
            orientation,
            model,
            magnification,
            error_correction,
            mask,
        },
    ))
}

/// ^B3 - Code 39 Barcode
pub fn cmd_b3(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^B3").parse(input)?;
    let (rest, args) = cut(take_till(|c| c == '^')).parse(input)?;
    let (args_input, orientation) = opt_param(parse_char).parse(args)?;
    let (args_input, check_digit) = param(parse_char)
        .parse(args_input)
        .unwrap_or((args_input, None));
    let (args_input, height) = param(parse_u32)
        .parse(args_input)
        .unwrap_or((args_input, None));
    let (args_input, interpretation_line) = param(parse_char)
        .parse(args_input)
        .unwrap_or((args_input, None));
    let (_, interpretation_line_above) = param(parse_char)
        .parse(args_input)
        .unwrap_or((args_input, None));

    Ok((
        rest,
        cmd::Command::Code39 {
            orientation,
            check_digit,
            height,
            interpretation_line,
            interpretation_line_above,
        },
    ))
}

/// ^BY - Barcode Field Default
pub fn cmd_by(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^BY").parse(input)?;
    let (rest, args) = cut(take_till(|c| c == '^')).parse(input)?;
    let (args_input, module_width) = opt_param(parse_u32).parse(args)?;
    let (args_input, ratio) = param(parse_f32)
        .parse(args_input)
        .unwrap_or((args_input, None));
    let (_, height) = param(parse_u32)
        .parse(args_input)
        .unwrap_or((args_input, None));

    Ok((
        rest,
        cmd::Command::BarcodeDefault {
            module_width,
            ratio,
            height,
        },
    ))
}

/// ^BX - Data Matrix Barcode
pub fn cmd_bx(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^BX").parse(input)?;
    let (rest, args) = cut(take_till(|c| c == '^')).parse(input)?;
    let (args_input, orientation) = opt_param(parse_char).parse(args)?;
    let (args_input, height) = param(parse_u32)
        .parse(args_input)
        .unwrap_or((args_input, None));
    let (args_input, quality) = param(parse_u32)
        .parse(args_input)
        .unwrap_or((args_input, None));
    let (args_input, columns) = param(parse_u32)
        .parse(args_input)
        .unwrap_or((args_input, None));
    let (_, rows) = param(parse_u32)
        .parse(args_input)
        .unwrap_or((args_input, None));

    Ok((
        rest,
        cmd::Command::DataMatrix {
            orientation,
            height,
            quality,
            columns,
            rows,
        },
    ))
}
