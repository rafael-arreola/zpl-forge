//! Native encoders for symbologies `rxing` does not provide, plus wide/narrow
//! ratio handling for the two-width symbologies it hardcodes.
//!
//! `rxing` has no writer for MSI or POSTNET, and [`super::barcode_1d_format`]
//! previously mapped both onto `CODE_128`. That produced a scannable symbol of
//! the *wrong symbology* rather than a slightly-off one, so these are encoded
//! here instead. Both were reverse-engineered from Labelary renders and the
//! published specifications.

/// One element of a 1-D barcode: a bar or a space of a given width in dots.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Element {
    /// `true` for a bar, `false` for a space.
    pub bar: bool,
    /// Width in dots.
    pub width: u32,
}

/// Relative height of a POSTNET bar. POSTNET encodes data in bar *height*
/// rather than bar width, so it cannot go through the run-length path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum BarHeight {
    /// Full-height bar (a binary 1).
    Full,
    /// Half-height, bottom-aligned bar (a binary 0).
    Half,
}

/// Fraction of the `^BZ` height used by POSTNET's short bars. Labelary renders
/// 40 dots of short bar against a 100-dot field.
pub(crate) const POSTNET_SHORT_RATIO: f32 = 0.4;

/// POSTNET bar pitch as a multiple of the bar width.
///
/// Measured against Labelary at `^BY2`/`^BY3`/`^BY5`, which render pitches of
/// 5, 7 and 12 dots for bar widths of 2, 3 and 5 - i.e. `round(2.4 * bar)`.
const POSTNET_PITCH_RATIO: f32 = 2.4;

/// POSTNET per-digit bar patterns: two full bars out of five.
const POSTNET_DIGITS: [[u8; 5]; 10] = [
    [1, 1, 0, 0, 0], // 0
    [0, 0, 0, 1, 1], // 1
    [0, 0, 1, 0, 1], // 2
    [0, 0, 1, 1, 0], // 3
    [0, 1, 0, 0, 1], // 4
    [0, 1, 0, 1, 0], // 5
    [0, 1, 1, 0, 0], // 6
    [1, 0, 0, 0, 1], // 7
    [1, 0, 0, 1, 0], // 8
    [1, 0, 1, 0, 0], // 9
];

/// Encodes `data` as POSTNET bar heights, framed by full-height guard bars and
/// terminated by a mod-10 correction digit.
///
/// Non-digits are ignored, matching Zebra's handling of formatted ZIP input
/// such as `12345-6789`.
pub(crate) fn postnet_bars(data: &str) -> Vec<BarHeight> {
    let digits: Vec<u8> = data
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c as u8 - b'0')
        .collect();

    // POSTNET's check digit brings the digit sum up to the next multiple of 10.
    let sum: u32 = digits.iter().map(|&d| d as u32).sum();
    let check = ((10 - (sum % 10)) % 10) as u8;

    let mut bars = Vec::with_capacity(digits.len() * 5 + 7);
    bars.push(BarHeight::Full); // leading frame bar
    for &d in digits.iter().chain(std::iter::once(&check)) {
        for &bit in &POSTNET_DIGITS[(d % 10) as usize] {
            bars.push(if bit == 1 {
                BarHeight::Full
            } else {
                BarHeight::Half
            });
        }
    }
    bars.push(BarHeight::Full); // trailing frame bar
    bars
}

/// Horizontal geometry of a POSTNET symbol: `(bar_width, gap)` in dots.
pub(crate) fn postnet_pitch(module_width: u32) -> (u32, u32) {
    let bar = module_width.max(1);
    let pitch = ((bar as f32) * POSTNET_PITCH_RATIO).round() as u32;
    (bar, pitch.saturating_sub(bar).max(1))
}

/// `^BM` check-digit modes. ZPL spells these A-D, defaulting to B.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum MsiCheck {
    /// A - no check digit.
    None,
    /// B - one mod-10 check digit (ZPL default).
    Mod10,
    /// C - two mod-10 check digits.
    Mod10Twice,
    /// D - one mod-11 followed by one mod-10 check digit.
    Mod11Mod10,
}

impl MsiCheck {
    /// Maps a `^BM` second parameter to its mode, defaulting to `Mod10`.
    pub(crate) fn from_zpl(c: char) -> Self {
        match c.to_ascii_uppercase() {
            'A' => MsiCheck::None,
            'C' => MsiCheck::Mod10Twice,
            'D' => MsiCheck::Mod11Mod10,
            _ => MsiCheck::Mod10,
        }
    }
}

/// Luhn-style mod-10 check digit as used by MSI: the digits in alternate
/// positions counting from the right are concatenated, doubled, and their
/// digit-sum added to the remaining digits.
fn msi_mod10(digits: &[u8]) -> u8 {
    let mut doubled = String::new();
    let mut rest_sum = 0u32;
    for (i, &d) in digits.iter().rev().enumerate() {
        if i % 2 == 0 {
            doubled.insert(0, (b'0' + d) as char);
        } else {
            rest_sum += d as u32;
        }
    }
    let doubled: u32 = doubled.parse::<u32>().unwrap_or(0) * 2;
    let digit_sum: u32 = doubled.to_string().bytes().map(|b| (b - b'0') as u32).sum();
    ((10 - ((digit_sum + rest_sum) % 10)) % 10) as u8
}

/// IBM mod-11 check digit with weights cycling 2..7 from the right.
fn msi_mod11(digits: &[u8]) -> u8 {
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| d as u32 * (2 + (i % 6) as u32))
        .sum();
    let rem = sum % 11;
    if rem == 0 { 0 } else { (11 - rem) as u8 }
}

/// Encodes `data` as MSI / Modified Plessey elements.
///
/// Each digit contributes four bits, most-significant first; a `1` is a wide
/// bar followed by a narrow space and a `0` the reverse. The symbol is framed
/// by a `110` start and a `1001` stop pattern.
pub(crate) fn msi_elements(
    data: &str,
    check: MsiCheck,
    module_width: u32,
    ratio: f32,
) -> Vec<Element> {
    let narrow = module_width.max(1);
    let wide = ((narrow as f32) * ratio).round().max(narrow as f32 + 1.0) as u32;

    let mut digits: Vec<u8> = data
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c as u8 - b'0')
        .collect();

    match check {
        MsiCheck::None => {}
        MsiCheck::Mod10 => digits.push(msi_mod10(&digits)),
        MsiCheck::Mod10Twice => {
            digits.push(msi_mod10(&digits));
            digits.push(msi_mod10(&digits));
        }
        MsiCheck::Mod11Mod10 => {
            digits.push(msi_mod11(&digits));
            digits.push(msi_mod10(&digits));
        }
    }

    let mut out = Vec::with_capacity(digits.len() * 8 + 5);
    let push = |bar: bool, width: u32, out: &mut Vec<Element>| out.push(Element { bar, width });

    // Start: wide bar, narrow space.
    push(true, wide, &mut out);
    push(false, narrow, &mut out);

    for &d in &digits {
        for shift in (0..4).rev() {
            if (d >> shift) & 1 == 1 {
                push(true, wide, &mut out);
                push(false, narrow, &mut out);
            } else {
                push(true, narrow, &mut out);
                push(false, wide, &mut out);
            }
        }
    }

    // Stop: narrow bar, wide space, narrow bar.
    push(true, narrow, &mut out);
    push(false, wide, &mut out);
    push(true, narrow, &mut out);

    out
}

/// Digits actually encoded into an MSI symbol, for the interpretation line.
pub(crate) fn msi_text(data: &str, check: MsiCheck) -> String {
    let mut digits: Vec<u8> = data
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c as u8 - b'0')
        .collect();
    match check {
        MsiCheck::None => {}
        MsiCheck::Mod10 => digits.push(msi_mod10(&digits)),
        MsiCheck::Mod10Twice => {
            digits.push(msi_mod10(&digits));
            digits.push(msi_mod10(&digits));
        }
        MsiCheck::Mod11Mod10 => {
            digits.push(msi_mod11(&digits));
            digits.push(msi_mod10(&digits));
        }
    }
    digits.iter().map(|d| (b'0' + d) as char).collect()
}

/// Whether a symbology renders only two element widths, so that ZPL's `^BY`
/// wide:narrow ratio selects the wide multiple.
///
/// `rxing` bakes a fixed ratio into these writers (2:1 for Code 39, 3:1 for
/// ITF and Codabar) and ignores `^BY`, so their patterns are rescaled from the
/// encoded matrix rather than used verbatim.
pub(crate) fn is_two_width(format: rxing::BarcodeFormat) -> bool {
    matches!(
        format,
        rxing::BarcodeFormat::CODE_39 | rxing::BarcodeFormat::ITF | rxing::BarcodeFormat::CODABAR
    )
}

/// Converts row 0 of an encoded 1-D matrix into dot-width elements.
///
/// For two-width symbologies every run longer than one module is a "wide"
/// element and is re-emitted at `ratio * module_width` dots, which is what
/// `^BY` asks for. Every other symbology keeps its module widths verbatim.
pub(crate) fn matrix_elements(
    matrix: &rxing::common::BitMatrix,
    module_width: u32,
    ratio: Option<f32>,
) -> Vec<Element> {
    let narrow = module_width.max(1);
    let width = matrix.getWidth();
    let mut out = Vec::new();
    if width == 0 {
        return out;
    }

    let mut run_start = 0u32;
    let mut current = matrix.get(0, 0);
    for x in 1..=width {
        let value = if x < width {
            matrix.get(x, 0)
        } else {
            !current
        };
        if value != current {
            let modules = x - run_start;
            let dots = match ratio {
                // Two-width symbology: collapse rxing's fixed wide multiple
                // onto the ratio the label actually asked for.
                Some(r) if modules > 1 => ((narrow as f32) * r).round() as u32,
                Some(_) => narrow,
                None => modules * narrow,
            };
            out.push(Element {
                bar: current,
                width: dots.max(1),
            });
            current = value;
            run_start = x;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postnet_matches_labelary_geometry() {
        // ^BZ with "12345": frame + 5 digits + check digit + frame = 32 bars,
        // and Labelary renders a 157-dot-wide symbol at ^BY2.
        let bars = postnet_bars("12345");
        assert_eq!(bars.len(), 32);
        assert_eq!(bars[0], BarHeight::Full);
        assert_eq!(bars[31], BarHeight::Full);
        // Digit 1 = 00011 -> short short short full full.
        assert_eq!(
            &bars[1..6],
            &[
                BarHeight::Half,
                BarHeight::Half,
                BarHeight::Half,
                BarHeight::Full,
                BarHeight::Full
            ]
        );
        let (bar_w, gap) = postnet_pitch(2);
        let total = bars.len() as u32 * bar_w + (bars.len() as u32 - 1) * gap;
        assert_eq!(total, 157);
    }

    #[test]
    fn msi_check_digit_and_width() {
        // Labelary encodes "123456" as 7 digits (6 data + 1 mod-10) in 61
        // elements spanning 91 narrow modules.
        assert_eq!(msi_text("123456", MsiCheck::Mod10), "1234566");
        let els = msi_elements("123456", MsiCheck::Mod10, 2, 2.0);
        assert_eq!(els.len(), 61);
        let total: u32 = els.iter().map(|e| e.width).sum();
        assert_eq!(total, 182); // 91 modules at ^BY2
    }

    #[test]
    fn msi_no_check_digit_leaves_data_alone() {
        assert_eq!(msi_text("123456", MsiCheck::None), "123456");
    }

    #[test]
    fn two_width_classification() {
        assert!(is_two_width(rxing::BarcodeFormat::ITF));
        assert!(is_two_width(rxing::BarcodeFormat::CODE_39));
        assert!(!is_two_width(rxing::BarcodeFormat::CODE_128));
        assert!(!is_two_width(rxing::BarcodeFormat::EAN_13));
    }
}
