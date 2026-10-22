//! High-level decoding and data manipulation utilities for ZPL assets.

use crate::{ZplError, ZplResult};
#[cfg(any(feature = "png", feature = "pdf"))]
use image::GenericImageView;

pub(crate) const MAX_DECODED_SIZE: usize = 10 * 1024 * 1024;

fn reserve_decoded(output: &mut Vec<u8>, additional: usize) -> ZplResult<usize> {
    let size = output
        .len()
        .checked_add(additional)
        .filter(|&size| size <= MAX_DECODED_SIZE)
        .ok_or_else(|| ZplError::SecurityLimitExceeded("Decoded bitmap exceeds 10 MiB".into()))?;
    output.try_reserve(additional).map_err(|error| {
        ZplError::SecurityLimitExceeded(format!("Cannot allocate decoded bitmap: {error}"))
    })?;
    Ok(size)
}

/// Decodes ZPL compressed image data, typically used in the `^GF` (Graphic Field) command.
///
/// This function handles:
/// - Hexadecimal nibbles.
/// - Repeat characters: `G..Y` (1..19) and `g..z` (20..400).
/// - Special row markers: `:` (repeat previous row), `,` (fill rest of row with 0x00), and `!` (fill rest of row with 0xFF).
///
/// It is commonly used when parsing raw ZPL streams to reconstruct the bitmap image intended for printing.
///
/// # Arguments
/// * `encoded_str` - The ZPL-encoded string.
/// * `bytes_per_row` - The number of bytes in a single row of the image.
///
/// # Errors
/// Returns an error for invalid data, a zero row size, allocation failure, or
/// decoded data exceeding 10 MiB. No partial bitmap is returned on failure.
pub fn zpl_decode(encoded_str: &str, bytes_per_row: usize) -> ZplResult<Vec<u8>> {
    if bytes_per_row == 0 {
        return Err(ZplError::ImageError(
            "Bitmap row size must be positive".into(),
        ));
    }
    if bytes_per_row > MAX_DECODED_SIZE {
        return Err(ZplError::SecurityLimitExceeded(
            "Bitmap row exceeds 10 MiB".into(),
        ));
    }

    let mut output = Vec::new();
    let mut multiplier = 0usize;
    let mut high_nibble = None;
    let mut last_was_row_terminator = false;

    for c in encoded_str.chars() {
        match c {
            'G'..='Y' | 'g'..='z' => {
                let repeat = if c.is_ascii_uppercase() {
                    c as usize - 'G' as usize + 1
                } else {
                    (c as usize - 'g' as usize + 1) * 20
                };
                multiplier = multiplier.checked_add(repeat).ok_or_else(|| {
                    ZplError::SecurityLimitExceeded("Bitmap repeat count overflow".into())
                })?;
            }
            ':' => {
                if let Some(high) = high_nibble.take() {
                    reserve_decoded(&mut output, 1)?;
                    output.push(high << 4);
                }

                let current_row_pos = output.len() % bytes_per_row;
                let total_repeats = multiplier.max(1);
                let mut repeats_done = 0;
                if current_row_pos > 0 {
                    let missing = bytes_per_row - current_row_pos;
                    let current_row_start = output.len() - current_row_pos;
                    let size = reserve_decoded(&mut output, missing)?;
                    if current_row_start >= bytes_per_row {
                        let previous = current_row_start - bytes_per_row;
                        output.extend_from_within(
                            previous + current_row_pos..previous + bytes_per_row,
                        );
                    } else {
                        output.resize(size, 0);
                    }
                    repeats_done = 1;
                }

                let remaining = total_repeats - repeats_done;
                let additional = bytes_per_row.checked_mul(remaining).ok_or_else(|| {
                    ZplError::SecurityLimitExceeded("Bitmap repeat size overflow".into())
                })?;
                let size = reserve_decoded(&mut output, additional)?;
                if output.len() >= bytes_per_row {
                    let end = output.len();
                    let start = end - bytes_per_row;
                    for _ in 0..remaining {
                        output.extend_from_within(start..end);
                    }
                } else {
                    output.resize(size, 0);
                }
                multiplier = 0;
                last_was_row_terminator = true;
            }
            c if c.is_ascii_hexdigit() => {
                let value = c
                    .to_digit(16)
                    .ok_or_else(|| ZplError::ImageError(format!("Invalid bitmap hex digit: {c}")))?
                    as u8;
                let count = multiplier.max(1);
                let nibbles = count
                    .checked_add(usize::from(high_nibble.is_some()))
                    .ok_or_else(|| {
                        ZplError::SecurityLimitExceeded("Bitmap repeat count overflow".into())
                    })?;
                reserve_decoded(&mut output, nibbles / 2)?;
                for _ in 0..count {
                    if let Some(high) = high_nibble.take() {
                        output.push((high << 4) | value);
                    } else {
                        high_nibble = Some(value);
                    }
                }
                multiplier = 0;
                last_was_row_terminator = false;
            }
            ',' | '!' => {
                let fill = if c == ',' { 0 } else { 0xFF };
                if let Some(high) = high_nibble.take() {
                    reserve_decoded(&mut output, 1)?;
                    output.push((high << 4) | (fill & 0x0F));
                }
                let row_pos = output.len() % bytes_per_row;
                let padding = if row_pos != 0 {
                    bytes_per_row - row_pos
                } else if last_was_row_terminator || output.is_empty() {
                    bytes_per_row
                } else {
                    0
                };
                let size = reserve_decoded(&mut output, padding)?;
                output.resize(size, fill);
                multiplier = 0;
                last_was_row_terminator = true;
            }
            c if c.is_ascii_whitespace() => {}
            _ => {
                return Err(ZplError::ImageError(format!(
                    "Invalid bitmap data character: {c}"
                )));
            }
        }
    }

    if multiplier != 0 {
        return Err(ZplError::ImageError(
            "Bitmap repeat count has no following data".into(),
        ));
    }
    if let Some(high) = high_nibble {
        reserve_decoded(&mut output, 1)?;
        output.push(high << 4);
    }
    Ok(output)
}

/// Encodes raw image bytes into a ZPL-compatible hexadecimal string for use with the `^GF` command.
///
/// This function converts common image formats (PNG, JPEG, etc.) to a black-and-white bitmap (1 bit per pixel).
/// It applies Zebra's standard ASCII compression (repeat characters G-z) to reduce string size.
/// A pixel is considered black (1) if its luminance is below 50%, otherwise it is white (0).
///
/// This is commonly used to embed custom logos, icons, or external graphics into a ZPL label format.
///
/// # Arguments
/// * `image_bytes` - The raw bytes of the image (e.g., from a file).
///
/// # Returns
/// A `ZplResult` containing a tuple with:
/// 1. The encoded string (hexadecimal with ASCII compression).
/// 2. Total number of bytes in the bitmap.
/// 3. Bytes per row (required by the `^GF` command).
#[cfg(any(feature = "png", feature = "pdf"))]
pub fn zpl_encode(image_bytes: &[u8]) -> ZplResult<(String, usize, usize)> {
    let img = image::load_from_memory(image_bytes)
        .map_err(|e| ZplError::ImageError(format!("Failed to load image from bytes: {}", e)))?;

    let (width, height) = img.dimensions();
    let luma_img = img.to_luma8();
    let bytes_per_row = (width as usize).div_ceil(8);
    let total_bytes = bytes_per_row * height as usize;
    let mut bitmap = vec![0u8; total_bytes];

    for (y, row) in luma_img.rows().enumerate() {
        let row_offset = y * bytes_per_row;
        for (x, pixel) in row.enumerate() {
            // In ZPL ^GF: 1 is black, 0 is white.
            // luminance < 128 means dark/black.
            if pixel.0[0] < 128 {
                let byte_idx = row_offset + (x / 8);
                let bit_idx = 7 - (x % 8);
                bitmap[byte_idx] |= 1 << bit_idx;
            }
        }
    }

    const HEX_UPPER: &[u8; 16] = b"0123456789ABCDEF";
    let mut hex_str = String::with_capacity(bitmap.len() * 2);
    for byte in &bitmap {
        hex_str.push(HEX_UPPER[(byte >> 4) as usize] as char);
        hex_str.push(HEX_UPPER[(byte & 0x0F) as usize] as char);
    }
    let mut encoded = String::new();
    let chars: Vec<char> = hex_str.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        let mut count = 1;
        while i + count < chars.len() && chars[i + count] == chars[i] && count < 400 {
            count += 1;
        }

        if count > 1 {
            let mut remaining = count;

            // Use multiples of 20 (g-z)
            while remaining >= 20 {
                let factor = (remaining / 20).min(20);
                let repeat_char = (b'g' + (factor as u8) - 1) as char;
                encoded.push(repeat_char);
                remaining -= factor * 20;
            }

            // Use units (G-Y)
            if remaining > 0 {
                let repeat_char = (b'G' + (remaining as u8) - 1) as char;
                encoded.push(repeat_char);
            }

            encoded.push(chars[i]);
        } else {
            encoded.push(chars[i]);
        }

        i += count;
    }

    Ok((encoded, total_bytes, bytes_per_row))
}
