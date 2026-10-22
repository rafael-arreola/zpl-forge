use nom::{
    IResult, Offset, Parser,
    branch::alt,
    bytes::complete::{tag, take, take_till},
    character::complete::{digit1, multispace0, none_of},
    combinator::{all_consuming, cut, map_res, opt, recognize, verify},
    error::{Error, ErrorKind},
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
                standard::cmd_gd,
                standard::cmd_ge,
                standard::cmd_gf,
                standard::cmd_bq,
                standard::cmd_b2,
                standard::cmd_b3,
                standard::cmd_b7,
                standard::cmd_ba,
                standard::cmd_be,
                standard::cmd_bu,
                standard::cmd_by,
                standard::cmd_bx,
                standard::cmd_bc,
                custom::cmd_gic,
                custom::cmd_gtc,
                custom::cmd_glc,
                custom::cmd_ifc,
            )),
            alt((
                standard::cmd_b8,
                standard::cmd_b9,
                standard::cmd_bb,
                standard::cmd_bm,
                standard::cmd_bz,
                standard::cmd_bs,
                standard::cmd_b0,
                standard::cmd_bf,
                standard::cmd_br,
                standard::cmd_fh,
                standard::cmd_ci,
                cmd_unsupported,
            )),
        )),
        multispace0,
    )));

    match parser.parse(input) {
        Ok((_rest, cmds)) => Ok(cmds),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
            // Captured argument errors can reference an interior subslice, not the full remainder.
            let offset = input.offset(e.input);
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

/// Parses any unrecognized ZPL command starting with `^` followed by a 2-character ASCII alphanumeric code.
///
/// Captures the command code and all trailing arguments up to the next `^`.
/// Returns [`Command::UnsupportedCommand`](cmd::Command::UnsupportedCommand).
pub fn cmd_unsupported(input: Span) -> Res<cmd::Command> {
    let (input, _) = tag("^").parse(input)?;
    let (input, command_code) = cut(verify(take(2usize), |code: &str| {
        code.bytes().all(|c| c.is_ascii_alphanumeric())
    }))
    .parse(input)?;
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

/// Parses a single character and converts it to ASCII uppercase for case-insensitive parameter matching.
///
/// # Arguments
/// * `input` - Input slice of ZPL text.
///
/// # Returns
/// An `IResult` containing the remaining input and the parsed uppercase `char`.
pub fn parse_char_upper(input: Span) -> Res<char> {
    let (input, c) = none_of(",^\r\n \t").parse(input)?;
    Ok((input, c.to_ascii_uppercase()))
}

fn is_command_end(input: Span) -> bool {
    let input = input.trim_start_matches([' ', '\t', '\r', '\n']);
    input.is_empty() || input.starts_with('^')
}

fn numeric_end(input: Span) -> Res<()> {
    if input.starts_with(',') || is_command_end(input) {
        Ok((input, ()))
    } else {
        Err(nom::Err::Error(Error::new(input, ErrorKind::Verify)))
    }
}

/// Parses a complete unsigned 32-bit decimal parameter without accepting numeric prefixes.
pub fn parse_u32(input: Span) -> Res<u32> {
    let (input, value) = map_res(digit1, |s: Span| s.parse::<u32>()).parse(input)?;
    let (input, _) = numeric_end(input)?;
    Ok((input, value))
}

/// Parses a 32-bit floating-point number, accepting both integer and decimal forms (e.g., `3` or `2.5`).
pub fn parse_f32(input: Span) -> Res<f32> {
    let (input, value) = verify(
        map_res(recognize((digit1, opt((tag("."), digit1)))), |s: Span| {
            s.parse::<f32>()
        }),
        |value: &f32| value.is_finite(),
    )
    .parse(input)?;
    let (input, _) = numeric_end(input)?;
    Ok((input, value))
}

/// Wraps a parser to make its parameter optional without requiring a leading comma.
///
/// Returns `None` at a comma or command end, allowing trailing formatting whitespace.
/// Otherwise, parses a present value with `cut`, so invalid values cannot backtrack into a fallback.
pub fn opt_param<'a, O, P>(mut parser: P) -> impl FnMut(Span<'a>) -> Res<'a, Option<O>>
where
    P: Parser<Span<'a>, Output = O, Error = Error<Span<'a>>>,
{
    move |input: Span<'a>| {
        if input.starts_with(',') || is_command_end(input) {
            Ok((input, None))
        } else {
            let (input, v) = cut(|i| parser.parse(i)).parse(input)?;
            Ok((input, Some(v)))
        }
    }
}

/// Parses a comma-separated parameter by consuming a leading `,` and then delegating to [`opt_param`].
///
/// Returns `None` at a command end without a comma, or if the value after a comma is absent.
/// A present invalid value or a missing separator produces `Failure`.
pub fn param<'a, O, P>(mut parser: P) -> impl FnMut(Span<'a>) -> Res<'a, Option<O>>
where
    P: Parser<Span<'a>, Output = O, Error = Error<Span<'a>>>,
{
    move |input: Span<'a>| {
        if is_command_end(input) {
            return Ok((input, None));
        }
        let (input, _) = cut(tag(",")).parse(input)?;
        opt_param(|i| parser.parse(i)).parse(input)
    }
}

/// Rejects unconsumed command arguments, allowing only trailing formatting whitespace.
pub fn finish_args(input: Span) -> Res<()> {
    let (input, _) = cut(all_consuming(multispace0)).parse(input)?;
    Ok((input, ()))
}

/// Parses an `x,y` coordinate pair where both values are optional `u32`.
///
/// The first value is parsed via [`opt_param`]; the second via [`param`] (comma-prefixed).
/// Commonly used by `^FO`, `^FT`, and `^LH` commands.
pub fn parse_xy(input: Span) -> Res<(Option<u32>, Option<u32>)> {
    let (input, x_opt) = opt_param(parse_u32).parse(input)?;
    let (input, y_opt) = param(parse_u32).parse(input)?;
    Ok((input, (x_opt, y_opt)))
}
