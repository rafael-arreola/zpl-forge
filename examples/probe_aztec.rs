//! Parameter sweep for `^B0` (Aztec), to establish whether a payload/EC/layer
//! combination reproduces a Labelary reference exactly.
//!
//! Usage: `cargo run --release --example probe_aztec -- <reference_matrix.txt>`

use rxing::{BarcodeFormat, EncodeHints, MultiFormatWriter, Writer};

fn rows_of(m: &rxing::common::BitMatrix) -> Vec<String> {
    (0..m.getHeight())
        .map(|y| {
            (0..m.getWidth())
                .map(|x| if m.get(x, y) { '1' } else { '0' })
                .collect()
        })
        .collect()
}

fn main() {
    let reference: Vec<String> = std::env::args()
        .nth(1)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.lines().map(|l| l.trim().to_string()).collect())
        .expect("reference matrix");

    let payloads = ["AZTEC-CODE-2D", "0AZTEC-CODE-2D", "N,AZTEC-CODE-2D"];
    let mut best: Option<(usize, String)> = None;

    for payload in payloads {
        for ec in [
            None,
            Some(0u32),
            Some(5),
            Some(10),
            Some(23),
            Some(25),
            Some(33),
            Some(50),
        ] {
            for layers in [None, Some(1i32), Some(2), Some(3), Some(4)] {
                let hints = EncodeHints {
                    ErrorCorrection: ec.map(|v| v.to_string()),
                    Margin: Some("0".to_string()),
                    AztecLayers: layers,
                    ..Default::default()
                };
                let Ok(m) = MultiFormatWriter.encode_with_hints(
                    payload,
                    &BarcodeFormat::AZTEC,
                    0,
                    0,
                    &hints,
                ) else {
                    continue;
                };
                let rows = rows_of(&m);
                if rows.len() != reference.len() {
                    continue;
                }
                let d: usize = rows
                    .iter()
                    .zip(&reference)
                    .map(|(a, b)| a.chars().zip(b.chars()).filter(|(p, q)| p != q).count())
                    .sum();
                let key = format!("{payload:?} ec={ec:?} layers={layers:?}");
                if d == 0 {
                    println!("EXACT MATCH: {key}");
                }
                if best.as_ref().is_none_or(|(bd, _)| d < *bd) {
                    best = Some((d, key));
                }
            }
        }
    }
    let total: usize = reference.iter().map(|r| r.len()).sum();
    if let Some((d, key)) = best {
        println!("best: {key} -> {d} differing modules of {total}");
    }
}
