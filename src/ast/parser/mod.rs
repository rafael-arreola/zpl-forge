use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_till},
    character::complete::{digit1, multispace0, none_of},
    combinator::{all_consuming, map_res, opt, recognize},
    error::Error,
    multi::many0,
    sequence::delimited,
    IResult, Parser,
};

use crate::ast::cmd;
use crate::{ZplError, ZplResult};

pub mod custom;
pub mod standard;

pub type Span<'a> = &'a str;
pub type Res<'a, T> = IResult<Span<'a>, T, Error<Span<'a>>>;

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

pub fn parse_char(input: Span) -> Res<char> {
    none_of(",^\r\n \t").parse(input)
}

pub fn parse_u32(input: Span) -> Res<u32> {
    map_res(digit1, |s: Span| s.parse::<u32>()).parse(input)
}

pub fn parse_f32(input: Span) -> Res<f32> {
    map_res(recognize((digit1, opt((tag("."), digit1)))), |s: Span| {
        s.parse::<f32>()
    })
    .parse(input)
}

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

pub fn param<'a, O, P>(mut parser: P) -> impl FnMut(Span<'a>) -> Res<'a, Option<O>>
where
    P: Parser<Span<'a>, Output = O, Error = Error<Span<'a>>>,
{
    move |input: Span<'a>| {
        let (input, _) = tag(",").parse(input)?;
        opt_param(|i| parser.parse(i)).parse(input)
    }
}

pub fn parse_xy(input: Span) -> Res<(Option<u32>, Option<u32>)> {
    let (input, x_opt) = opt_param(parse_u32).parse(input)?;
    let (input, y_opt) = param(parse_u32).parse(input).unwrap_or((input, None));
    Ok((input, (x_opt, y_opt)))
}
