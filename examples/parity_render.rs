//! Batch renderer for the parity harness.
//!
//! Reads `tools/parity_cases.tsv` (`name<TAB>width_in<TAB>height_in<TAB>zpl`)
//! and renders every case at its declared label size into a target directory.
//! Rendering every case in one process keeps the parity loop fast and, unlike
//! `render_png`, honours each case's own label dimensions.
//!
//! Usage: `cargo run --release --example parity_render -- [out_dir] [manifest]`

use std::collections::HashMap;
use zpl_forge::forge::png::PngBackend;
use zpl_forge::{Resolution, Unit, ZplEngine};

fn main() {
    let mut args = std::env::args().skip(1);
    let out_dir = args.next().unwrap_or_else(|| ".parity/ours".to_string());
    let manifest = args
        .next()
        .unwrap_or_else(|| "tools/parity_cases.tsv".to_string());

    std::fs::create_dir_all(&out_dir).expect("create out dir");
    let text = std::fs::read_to_string(&manifest).expect("read manifest");

    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        assert!(parts.len() >= 4, "malformed manifest line: {line}");
        let (name, w, h, zpl) = (
            parts[0],
            parts[1].parse::<f32>().expect("width"),
            parts[2].parse::<f32>().expect("height"),
            parts[3],
        );

        let engine = ZplEngine::new(zpl, Unit::Inches(w), Unit::Inches(h), Resolution::Dpi203)
            .unwrap_or_else(|e| panic!("engine failed for {name}: {e:?}"));
        let png = engine
            .render(PngBackend::new(), &HashMap::new())
            .unwrap_or_else(|e| panic!("png render failed for {name}: {e:?}"));

        std::fs::write(format!("{out_dir}/{name}.png"), &png).expect("write png");
        println!("rendered {name} ({w}x{h}in)");
    }
}
