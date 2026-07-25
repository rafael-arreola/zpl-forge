//! # Forge Layer
//!
//! The forge layer provides concrete implementations of rendering backends.
//! It translates the intermediate representation (`ZplInstruction`) into
//! specific output formats like images or documents.

#[cfg(feature = "pdf")]
pub mod pdf_native;
#[cfg(feature = "png")]
pub mod png;
#[cfg(any(feature = "png", feature = "pdf"))]
pub(crate) mod symbology;

/// Maps a generic 1-D symbology to its `rxing` barcode format.
#[cfg(any(feature = "png", feature = "pdf"))]
pub(crate) fn barcode_1d_format(kind: crate::engine::Barcode1DKind) -> rxing::BarcodeFormat {
    match kind {
        crate::engine::Barcode1DKind::Ean13 => rxing::BarcodeFormat::EAN_13,
        crate::engine::Barcode1DKind::Ean8 => rxing::BarcodeFormat::EAN_8,
        crate::engine::Barcode1DKind::UpcA => rxing::BarcodeFormat::UPC_A,
        crate::engine::Barcode1DKind::UpcE => rxing::BarcodeFormat::UPC_E,
        crate::engine::Barcode1DKind::Interleaved2of5 => rxing::BarcodeFormat::ITF,
        crate::engine::Barcode1DKind::Code93 => rxing::BarcodeFormat::CODE_93,
        crate::engine::Barcode1DKind::Codabar => rxing::BarcodeFormat::CODABAR,
        _ => rxing::BarcodeFormat::CODE_128,
    }
}

/// Splits a `^BC` payload's explicit code-set prefix from its data and returns
/// the `rxing` `FORCE_CODE_SET` value to use.
///
/// ZPL selects subset B unless the payload opens with an invocation code
/// (`>9` = A, `>:` = B, `>;` = C). `rxing` left to its own devices switches to
/// subset C for runs of digits, which is denser than what Zebra emits: for
/// `12345678` Zebra produces 123 modules (pure subset B) where subset C
/// produces 79, so the symbol came out a third of the correct width.
#[cfg(any(feature = "png", feature = "pdf"))]
pub(crate) fn code128_code_set(data: &str) -> (&str, &'static str) {
    if let Some(stripped) = data.strip_prefix(">9") {
        (stripped, "A")
    } else if let Some(stripped) = data.strip_prefix(">:") {
        (stripped, "B")
    } else if let Some(stripped) = data.strip_prefix(">;") {
        (stripped, "C")
    } else {
        (data, "B")
    }
}

/// Splits a `^BQ` payload into its error-correction level and the data to
/// encode.
///
/// QR is the one symbology where ZPL carries the error-correction level in the
/// *field data* rather than the barcode command: `^FDQA,payload` means level Q,
/// automatic input mode. Treating the whole string as the payload encoded the
/// `QA,` prefix into the symbol, so both the content and the correction level
/// were wrong even when the module count happened to line up.
///
/// Falls back to `default_ec` when no recognizable prefix is present.
#[cfg(any(feature = "png", feature = "pdf"))]
pub(crate) fn qr_field_data(data: &str, default_ec: char) -> (char, &str) {
    let bytes = data.as_bytes();
    // `<level><mode>,` where level is H/Q/M/L and mode is A (automatic) or
    // M (manual).
    if bytes.len() >= 3
        && matches!(bytes[0], b'H' | b'Q' | b'M' | b'L')
        && matches!(bytes[1], b'A' | b'M')
        && bytes[2] == b','
    {
        return (bytes[0] as char, &data[3..]);
    }
    // Tolerate `<level>,` which shows up in hand-written templates.
    if bytes.len() >= 2 && matches!(bytes[0], b'H' | b'Q' | b'M' | b'L') && bytes[1] == b',' {
        return (bytes[0] as char, &data[2..]);
    }
    (default_ec, data)
}

/// Fixed vertical offset, in dots, applied to `^BQ` QR symbols.
///
/// Zebra places the QR matrix 10 dots below the field origin instead of at it.
/// Measured constant across magnifications 3, 6 and 10 against Labelary, so it
/// is an offset rather than a magnification-scaled quiet zone.
#[cfg(any(feature = "png", feature = "pdf"))]
pub(crate) const QR_ORIGIN_Y_OFFSET: u32 = 10;

/// Builds the `rxing` PDF417 dimension constraint for a `^B7` field.
///
/// `^B7` takes an explicit column count; when it is omitted Zebra picks the
/// narrowest symbol that fits, which Labelary renders as a single data column
/// (86 modules) where `rxing` left to itself chooses four (137 modules).
#[cfg(any(feature = "png", feature = "pdf"))]
pub(crate) fn pdf417_dimensions(
    columns: Option<u32>,
    rows: Option<u32>,
) -> rxing::pdf417::encoder::Dimensions {
    let (min_c, max_c) = match columns.filter(|c| (1..=30).contains(c)) {
        Some(c) => (c as usize, c as usize),
        None => (1, 1),
    };
    let (min_r, max_r) = match rows.filter(|r| (3..=90).contains(r)) {
        Some(r) => (r as usize, r as usize),
        None => (3, 90),
    };
    rxing::pdf417::encoder::Dimensions::new(min_c, max_c, min_r, max_r)
}

/// Vertical scale factor baked into `rxing`'s PDF417 output.
///
/// The writer emits `getScaledMatrix(1, 4)`, so every codeword row occupies
/// four matrix rows. `^B7`'s height parameter sizes one *codeword row*, so the
/// matrix has to be decimated by this factor before scaling, otherwise symbols
/// come out four times too tall.
#[cfg(any(feature = "png", feature = "pdf"))]
pub(crate) const PDF417_ROW_SCALE: u32 = 4;

/// Collapses `rxing`'s 4x-oversampled PDF417 matrix back to one row per
/// codeword row so the caller can apply `^B7`'s row height directly.
#[cfg(any(feature = "png", feature = "pdf"))]
pub(crate) fn pdf417_descale(
    matrix: &rxing::common::BitMatrix,
) -> crate::ZplResult<rxing::common::BitMatrix> {
    let (w, h) = (matrix.getWidth(), matrix.getHeight());
    let rows = h / PDF417_ROW_SCALE;
    if rows == 0 {
        return Ok(matrix.clone());
    }
    let mut out = rxing::common::BitMatrix::new(w, rows)
        .map_err(|e| crate::ZplError::BackendError(format!("PDF417 rescale failed: {e}")))?;
    for r in 0..rows {
        let src_y = r * PDF417_ROW_SCALE;
        for x in 0..w {
            if matrix.get(x, src_y) {
                out.set(x, r);
            }
        }
    }
    Ok(out)
}

/// Process-wide, bounded cache of encoded barcode bit matrices.
///
/// Encoding is pure (same format + data + hints → same matrix), so results
/// are shared across renders. This is the hot path when one template is
/// rendered thousands of times with different variables but static barcodes.
#[cfg(any(feature = "png", feature = "pdf"))]
pub(crate) mod barcode_cache {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};

    use rxing::common::BitMatrix;
    use rxing::{BarcodeFormat, EncodeHints, MultiFormatWriter, Writer};

    use crate::{ZplError, ZplResult};

    /// Bound after which the cache is flushed wholesale. Each entry is a few
    /// KB at most, so the worst case stays in the low single-digit MB.
    const MAX_ENTRIES: usize = 512;

    type Key = (&'static str, String, String);

    fn cache() -> &'static Mutex<HashMap<Key, Arc<BitMatrix>>> {
        static CACHE: OnceLock<Mutex<HashMap<Key, Arc<BitMatrix>>>> = OnceLock::new();
        CACHE.get_or_init(|| Mutex::new(HashMap::new()))
    }

    fn format_key(format: &BarcodeFormat) -> &'static str {
        match format {
            BarcodeFormat::CODE_128 => "c128",
            BarcodeFormat::CODE_39 => "c39",
            BarcodeFormat::CODE_93 => "c93",
            BarcodeFormat::QR_CODE => "qr",
            BarcodeFormat::DATA_MATRIX => "dm",
            BarcodeFormat::PDF_417 => "p417",
            BarcodeFormat::EAN_13 => "e13",
            BarcodeFormat::EAN_8 => "e8",
            BarcodeFormat::UPC_A => "upca",
            BarcodeFormat::UPC_E => "upce",
            BarcodeFormat::ITF => "itf",
            BarcodeFormat::CODABAR => "codabar",
            BarcodeFormat::AZTEC => "aztec",
            _ => "other",
        }
    }

    /// Encodes `data` in the given format, reusing a cached matrix when the
    /// same (format, data, hints) triple was encoded before. `hints_key` must
    /// uniquely fingerprint the contents of `hints`.
    #[allow(clippy::collapsible_if)]
    pub fn encode_cached(
        format: BarcodeFormat,
        data: &str,
        hints_key: &str,
        hints: Option<&EncodeHints>,
    ) -> ZplResult<Arc<BitMatrix>> {
        let key: Key = (format_key(&format), data.to_string(), hints_key.to_string());

        if let Ok(guard) = cache().lock() {
            if let Some(hit) = guard.get(&key) {
                return Ok(hit.clone());
            }
        }

        // Suppress the writer's quiet zone unconditionally. `rxing` pads every
        // symbol with a margin (5 modules for most 1-D symbologies, 4 for
        // EAN/UPC and QR, 30 for PDF417), but `^FO` addresses the first *bar*,
        // not the start of the quiet zone. Leaving the padding in shifted every
        // barcode right and down by the margin and inflated its footprint;
        // Zebra leaves quiet-zone provisioning to the label designer.
        let mut effective = hints.cloned().unwrap_or_default();
        if effective.Margin.is_none() {
            effective.Margin = Some("0".to_string());
        }

        let writer = MultiFormatWriter;
        let matrix = writer
            .encode_with_hints(data, &format, 0, 0, &effective)
            .or_else(|e| {
                // A forced code set can be unable to represent the payload
                // (e.g. subset B with control characters). Retry letting the
                // writer choose rather than failing the whole render.
                if effective.ForceCodeSet.is_some() {
                    let mut relaxed = effective.clone();
                    relaxed.ForceCodeSet = None;
                    writer.encode_with_hints(data, &format, 0, 0, &relaxed)
                } else {
                    Err(e)
                }
            })
            .map_err(|e| ZplError::BackendError(format!("Barcode Generation Error: {}", e)))?;

        let matrix = Arc::new(matrix);
        if let Ok(mut guard) = cache().lock() {
            if guard.len() >= MAX_ENTRIES {
                guard.clear();
            }
            guard.insert(key, matrix.clone());
        }
        Ok(matrix)
    }
}
