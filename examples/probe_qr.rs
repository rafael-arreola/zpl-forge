//! Dumps the QR module matrix `rxing` produces for every mask pattern, so an
//! external script can identify which mask a reference renderer selected.
//!
//! Reads `payload<TAB>ec_level<TAB>out_prefix` lines and writes
//! `<out_prefix>.mask<0..7>` files containing `0`/`1` rows.
//!
//! Usage: `cargo run --release --example probe_qr -- <manifest.tsv>`

use rxing::{BarcodeFormat, EncodeHints, MultiFormatWriter, Writer};

fn matrix_rows(m: &rxing::common::BitMatrix) -> Vec<String> {
    (0..m.getHeight())
        .map(|y| {
            (0..m.getWidth())
                .map(|x| if m.get(x, y) { '1' } else { '0' })
                .collect()
        })
        .collect()
}

fn main() {
    let path = std::env::args().nth(1).expect("manifest path");
    let manifest = std::fs::read_to_string(path).expect("read manifest");

    for line in manifest.lines().filter(|l| !l.trim().is_empty()) {
        let mut parts = line.split('\t');
        let payload = parts.next().unwrap();
        let ec = parts.next().unwrap();
        let prefix = parts.next().unwrap();

        for mask in 0..8u32 {
            let hints = EncodeHints {
                ErrorCorrection: Some(ec.to_string()),
                Margin: Some("0".to_string()),
                QrMaskPattern: Some(mask.to_string()),
                ..Default::default()
            };
            match MultiFormatWriter.encode_with_hints(
                payload,
                &BarcodeFormat::QR_CODE,
                0,
                0,
                &hints,
            ) {
                Ok(m) => {
                    let _ =
                        std::fs::write(format!("{prefix}.mask{mask}"), matrix_rows(&m).join("\n"));
                }
                Err(e) => println!("{payload}/{ec}/mask{mask}: ERROR {e}"),
            }
        }
        println!("dumped {prefix} for {payload:?} ec={ec}");
    }
}
