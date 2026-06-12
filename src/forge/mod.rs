//! # Forge Layer
//!
//! The forge layer provides concrete implementations of rendering backends.
//! It translates the intermediate representation (`ZplInstruction`) into
//! specific output formats like images or documents.

#[cfg(feature = "pdf")]
pub mod pdf_native;
#[cfg(feature = "png")]
pub mod png;

/// Maps a generic 1-D symbology to its `rxing` barcode format.
#[cfg(any(feature = "png", feature = "pdf"))]
pub(crate) fn barcode_1d_format(kind: crate::engine::Barcode1DKind) -> rxing::BarcodeFormat {
    match kind {
        crate::engine::Barcode1DKind::Ean13 => rxing::BarcodeFormat::EAN_13,
        crate::engine::Barcode1DKind::UpcA => rxing::BarcodeFormat::UPC_A,
        crate::engine::Barcode1DKind::Interleaved2of5 => rxing::BarcodeFormat::ITF,
        crate::engine::Barcode1DKind::Code93 => rxing::BarcodeFormat::CODE_93,
    }
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
            BarcodeFormat::UPC_A => "upca",
            BarcodeFormat::ITF => "itf",
            _ => "other",
        }
    }

    /// Encodes `data` in the given format, reusing a cached matrix when the
    /// same (format, data, hints) triple was encoded before. `hints_key` must
    /// uniquely fingerprint the contents of `hints`.
    pub fn encode_cached(
        format: BarcodeFormat,
        data: &str,
        hints_key: &str,
        hints: Option<&EncodeHints>,
    ) -> ZplResult<Arc<BitMatrix>> {
        let key: Key = (format_key(&format), data.to_string(), hints_key.to_string());

        if let Ok(guard) = cache().lock()
            && let Some(hit) = guard.get(&key)
        {
            return Ok(hit.clone());
        }

        let writer = MultiFormatWriter;
        let matrix = match hints {
            Some(h) => writer.encode_with_hints(data, &format, 0, 0, h),
            None => writer.encode(data, &format, 0, 0),
        }
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
