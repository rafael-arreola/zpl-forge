use crate::{ZplError, ZplResult};
use image::GenericImageView;

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
pub fn zpl_decode(encoded_str: &str, bytes_per_row: usize) -> Vec<u8> {
    let mut output: Vec<u8> = Vec::new();
    let mut multiplier: usize = 0;
    let mut high_nibble: Option<u8> = None;
    let mut last_was_row_terminator = false;

    // Safety limit to avoid OOM from malformed input or memory exhaustion attacks.
    const MAX_DECODED_SIZE: usize = 10 * 1024 * 1024; // 10MB limit

    for c in encoded_str.chars() {
        if output.len() > MAX_DECODED_SIZE {
            break;
        }

        match c {
            'G'..='Y' => multiplier = multiplier.saturating_add((c as usize - 'G' as usize) + 1),
            'g'..='z' => {
                multiplier = multiplier.saturating_add(((c as usize - 'g' as usize) + 1) * 20)
            }

            ':' => {
                if let Some(high) = high_nibble {
                    output.push(high << 4);
                    high_nibble = None;
                }

                if bytes_per_row > 0 {
                    let current_row_pos = output.len() % bytes_per_row;
                    let total_repeats = if multiplier == 0 { 1 } else { multiplier };
                    let mut repeats_done = 0;

                    if current_row_pos > 0 {
                        let missing = bytes_per_row - current_row_pos;
                        let current_row_start = output.len() - current_row_pos;

                        if current_row_start >= bytes_per_row {
                            let prev_row_start = current_row_start - bytes_per_row;
                            let copy_start = prev_row_start + current_row_pos;
                            let copy_end = prev_row_start + bytes_per_row;

                            let suffix = output[copy_start..copy_end].to_vec();
                            output.extend_from_slice(&suffix);
                        } else {
                            output.extend(std::iter::repeat_n(0x00, missing));
                        }
                        repeats_done += 1;
                    }

                    if repeats_done < total_repeats {
                        let remaining = total_repeats - repeats_done;
                        // Avoid massive memory allocations
                        let remaining = remaining.min(1000);

                        if output.len() >= bytes_per_row {
                            let start = output.len() - bytes_per_row;
                            let last_row = output[start..].to_vec();
                            for _ in 0..remaining {
                                if output.len() + last_row.len() > MAX_DECODED_SIZE {
                                    break;
                                }
                                output.extend_from_slice(&last_row);
                            }
                        } else {
                            let empty_row = vec![0u8; bytes_per_row];
                            for _ in 0..remaining {
                                if output.len() + empty_row.len() > MAX_DECODED_SIZE {
                                    break;
                                }
                                output.extend_from_slice(&empty_row);
                            }
                        }
                    }
                }
                multiplier = 0;
                last_was_row_terminator = true;
            }

            c if c.is_ascii_hexdigit() => {
                let val = c.to_digit(16).unwrap_or(0) as u8;
                let count = if multiplier == 0 { 1 } else { multiplier };
                multiplier = 0;

                // Protection against extreme repeat counts
                let count = count.min(10000);

                for _ in 0..count {
                    if output.len() >= MAX_DECODED_SIZE {
                        break;
                    }
                    if let Some(high) = high_nibble {
                        output.push((high << 4) | val);
                        high_nibble = None;
                    } else {
                        high_nibble = Some(val);
                    }
                }
                last_was_row_terminator = false;
            }

            ',' => {
                if let Some(high) = high_nibble {
                    output.push(high << 4);
                    high_nibble = None;
                }

                if bytes_per_row > 0 {
                    let current_row_pos = output.len() % bytes_per_row;
                    if current_row_pos != 0 {
                        let padding = bytes_per_row - current_row_pos;
                        output.extend(std::iter::repeat_n(0x00, padding));
                    } else if last_was_row_terminator {
                        output.extend(std::iter::repeat_n(0x00, bytes_per_row));
                    }
                }
                multiplier = 0;
                last_was_row_terminator = true;
            }

            '!' => {
                if let Some(high) = high_nibble {
                    output.push((high << 4) | 0x0F);
                    high_nibble = None;
                }

                if bytes_per_row > 0 {
                    let current_row_pos = output.len() % bytes_per_row;
                    if current_row_pos != 0 {
                        let padding = bytes_per_row - current_row_pos;
                        output.extend(std::iter::repeat_n(0xFF, padding));
                    } else if last_was_row_terminator {
                        output.extend(std::iter::repeat_n(0xFF, bytes_per_row));
                    }
                }
                multiplier = 0;
                last_was_row_terminator = true;
            }

            _ => {}
        }
    }

    if let Some(high) = high_nibble {
        output.push(high << 4);
    }

    output
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

    let hex_str = hex::encode_upper(bitmap);
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
