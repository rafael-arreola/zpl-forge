use nom::{
    bytes::complete::{tag, take_till},
    combinator::{cut, map_res},
    Parser,
};

use super::{param, parse_u32, Res, Span};
use crate::ast::cmd;

/// ^GIC - Custom Image Color
/// Format: ^GIC<width>,<height>,<base64>
/// All parameters are mandatory.
pub fn cmd_gic(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^GIC").parse(input)?;

    let (input, width) = cut(parse_u32).parse(input)?;

    let (input, height) = cut(map_res(param(parse_u32), |opt| {
        opt.ok_or("height is mandatory")
    }))
    .parse(input)?;

    let (input, _) = cut(tag(",")).parse(input)?;

    let (input, data) = cut(map_res(take_till(|c| c == '^'), |s: &str| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            Err("base64 data is mandatory")
        } else {
            Ok(trimmed.to_owned())
        }
    }))
    .parse(input)?;

    Ok((
        input,
        cmd::Command::CustomImage {
            width,
            height,
            data,
        },
    ))
}

/// ^GTC - Custom Text Color
/// Format: ^GTC<hex_color>
pub fn cmd_gtc(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^GTC").parse(input)?;
    let (input, color) = take_till(|c| c == '^').parse(input)?;

    Ok((
        input,
        cmd::Command::GraphicTextColor {
            color: color.trim().to_owned(),
        },
    ))
}

/// ^GLC - Custom Line Color
/// Format: ^GLC<hex_color>
pub fn cmd_glc(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^GLC").parse(input)?;
    let (input, color) = take_till(|c| c == '^').parse(input)?;

    Ok((
        input,
        cmd::Command::GraphicLineColor {
            color: color.trim().to_owned(),
        },
    ))
}
