use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{tag, take, take_till},
    character::complete::{digit1, multispace0, none_of},
    combinator::{all_consuming, map_res, opt, recognize},
    error::Error,
    multi::many0,
    sequence::delimited,
};

use crate::ast::cmd;
use crate::{ZplError, ZplResult};

pub mod custom;
pub mod standard;

pub type Span<'a> = &'a str;
pub type Res<'a, T> = IResult<Span<'a>, T, Error<Span<'a>>>;

/// Parses a complete ZPL input string into a vector of [`Command`](cmd::Command) AST nodes.
///
/// Uses `nom::combinator::all_consuming` to ensure the entire input is consumed.
/// Returns a [`ZplError::ParseError`] with line information on failure, or
/// [`ZplError::EmptyInput`] on incomplete input.
///
/// # Arguments
/// * `input` - A full ZPL document string (e.g., `^XA...^XZ`).
///
/// # Returns
/// A `ZplResult<Vec<cmd::Command>>` with the ordered list of parsed commands.
pub fn parse_zpl(input: &str) -> ZplResult<Vec<cmd::Command>> {
    let mut parser = all_consuming(many0(delimited(
        multispace0,
        alt((
            alt((
                standard::cmd_xa,
                standard::cmd_xz,
                standard::cmd_lh,
                standard::cmd_ll,
                standard::cmd_fo,
                standard::cmd_ft,
                standard::cmd_fs,
                standard::cmd_lr,
                standard::cmd_fx,
                standard::cmd_a,
                standard::cmd_cf,
                standard::cmd_fd,
                standard::cmd_fb,
                standard::cmd_fr,
            )),
            alt((
                standard::cmd_gb,
                standard::cmd_gc,
                standard::cmd_ge,
                standard::cmd_gf,
                standard::cmd_bq,
                standard::cmd_b3,
                standard::cmd_by,
                standard::cmd_bx,
                standard::cmd_bc,
                custom::cmd_gic,
                custom::cmd_gtc,
                custom::cmd_glc,
                custom::cmd_ifc,
                cmd_unsupported,
            )),
        )),
        multispace0,
    )));

    match parser.parse(input) {
        Ok((_rest, cmds)) => Ok(cmds),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
            let offset = input.len() - e.input.len();
            let line = input[..offset].chars().filter(|&c| c == '\n').count() + 1;

            Err(ZplError::ParseError {
                line,
                message: format!(
                    "Invalid or malformed ZPL command (Error code: {:?})",
                    e.code
                ),
            })
        }
        Err(nom::Err::Incomplete(_)) => Err(ZplError::EmptyInput),
    }
}

/// Parses any unrecognized ZPL command starting with `^` followed by a 2-character code.
///
/// Captures the command code and all trailing arguments up to the next `^`.
/// Returns [`Command::UnsupportedCommand`](cmd::Command::UnsupportedCommand).
pub fn cmd_unsupported(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^").parse(input)?;
    let (input, command_code) = take(2usize).parse(input)?;
    let (input, args) = take_till(|c| c == '^').parse(input)?;
    Ok((
        input,
        cmd::Command::UnsupportedCommand {
            command: format!("^{}", command_code),
            args: args.trim().to_owned(),
        },
    ))
}

/// Parses a single character that is not a comma, caret, whitespace, or newline.
///
/// Used to extract single-character parameters (e.g., orientation, font name) from ZPL fields.
pub fn parse_char(input: Span) -> Res<char> {
    none_of(",^\r\n \t").parse(input)
}

/// Parses an unsigned 32-bit integer from decimal digit characters.
pub fn parse_u32(input: Span) -> Res<u32> {
    map_res(digit1, |s: Span| s.parse::<u32>()).parse(input)
}

/// Parses a 32-bit floating-point number, accepting both integer and decimal forms (e.g., `3` or `2.5`).
pub fn parse_f32(input: Span) -> Res<f32> {
    map_res(recognize((digit1, opt((tag("."), digit1)))), |s: Span| {
        s.parse::<f32>()
    })
    .parse(input)
}

/// Wraps a parser to make its parameter optional without requiring a leading comma.
///
/// Returns `None` if the input is empty or starts with `,` or `^` (i.e., the parameter was omitted).
/// Otherwise, applies the inner parser and wraps the result in `Some`.
pub fn opt_param<'a, O, P>(mut parser: P) -> impl FnMut(Span<'a>) -> Res<'a, Option<O>>
where
    P: Parser<Span<'a>, Output = O, Error = Error<Span<'a>>>,
{
    move |input: Span<'a>| {
        if input.is_empty() || input.starts_with(',') || input.starts_with('^') {
            Ok((input, None))
        } else {
            let (input, v) = parser.parse(input)?;
            Ok((input, Some(v)))
        }
    }
}

/// Parses a comma-separated parameter by consuming a leading `,` and then delegating to [`opt_param`].
///
/// Returns `None` if the value after the comma is absent; otherwise returns `Some(value)`.
pub fn param<'a, O, P>(mut parser: P) -> impl FnMut(Span<'a>) -> Res<'a, Option<O>>
where
    P: Parser<Span<'a>, Output = O, Error = Error<Span<'a>>>,
{
    move |input: Span<'a>| {
        let (input, _) = tag(",").parse(input)?;
        opt_param(|i| parser.parse(i)).parse(input)
    }
}

/// Parses an `x,y` coordinate pair where both values are optional `u32`.
///
/// The first value is parsed via [`opt_param`]; the second via [`param`] (comma-prefixed).
/// Commonly used by `^FO`, `^FT`, and `^LH` commands.
pub fn parse_xy(input: Span) -> Res<(Option<u32>, Option<u32>)> {
    let (input, x_opt) = opt_param(parse_u32).parse(input)?;
    let (input, y_opt) = param(parse_u32).parse(input).unwrap_or((input, None));
    Ok((input, (x_opt, y_opt)))
}
