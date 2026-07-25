# ZPL-Forge Examples & Capability Showcase

This document provides ready-to-run code examples and performance metrics for the `zpl-forge` capability showcase.

---

## 🚀 Capability Showcase Modules & Output Files

`zpl-forge` includes a consolidated showcase suite (`examples/zpl_showcase.rs`) that renders all 5 primary capability modules into PNG and native vector PDF formats.

### Showcase Summary & Performance Table

| Showcase Module | PNG Output | Native Vector PDF | Output File Artifacts | Features Demonstrated |
| :--- | :---: | :---: | :--- | :--- |
| **1. Shipping Label** | `showcase_01_shipping.png` (5.36 ms) | `showcase_01_shipping.pdf` (7.33 ms) | [`examples/showcase_01_shipping.png`](file:///Users/rafael-arreola/sites/rafael-arreola/zpl-forge/examples/showcase_01_shipping.png) | Standard logistics layout, text wrapping, `^GB` boxes, Code 128 barcode |
| **2. Barcode Gallery** | `showcase_02_barcodes.png` (14.35 ms) | `showcase_02_barcodes.pdf` (10.27 ms) | [`examples/showcase_02_barcodes.png`](file:///Users/rafael-arreola/sites/rafael-arreola/zpl-forge/examples/showcase_02_barcodes.png) | 2-column layout of all 17 1D & 2D barcode symbologies with ZPL command codes & sample data labels |
| **3. Graphics & Color** | `showcase_03_graphics.png` (6.63 ms) | `showcase_03_graphics.pdf` (4.97 ms) | [`examples/showcase_03_graphics.png`](file:///Users/rafael-arreola/sites/rafael-arreola/zpl-forge/examples/showcase_03_graphics.png) | Monochrome bitmap (`^GF`), Base64 color image (`^GI`), scaling (`^GIC`), custom text/line colors (`^GTC`/`^GLC`) |
| **4. Logic & Custom Fonts** | `showcase_04_logic_and_fonts.png` (5.57 ms) | `showcase_04_logic_and_fonts.pdf` (4.67 ms) | [`examples/showcase_04_logic_and_fonts.png`](file:///Users/rafael-arreola/sites/rafael-arreola/zpl-forge/examples/showcase_04_logic_and_fonts.png) | Template variables `{{var}}`, `^IFC` conditions, vector shapes (`^GC`/`^GE`/`^GD`), custom TTF font embedding |
| **5. Multi-Page Batch** | — | `showcase_05_multi_page.pdf` (156.60 ms) | [`examples/showcase_05_multi_page.pdf`](file:///Users/rafael-arreola/sites/rafael-arreola/zpl-forge/examples/showcase_05_multi_page.pdf) | High-throughput 1,000-page vector PDF generation of high-density ZPL-FORGE EXPRESS shipping labels (1.85 MB file size, 0.157 ms/page) |

> **Note on Generated Files:** The generated `.png` and `.pdf` output files in `examples/` are excluded from Git version control via `.gitignore`. You can generate all showcase files locally at any time using Cargo.

---

## 💻 How to Run Showcase Examples Locally

If you have cloned the repository, you can execute the showcase suite and additional comparison commands directly from your terminal:

```bash
# 1. Run the main showcase suite (generates all showcase_01..05 files in examples/)
cargo run --release --example zpl_showcase

# 2. Run visual comparison suite (generates side-by-side comparison images)
cargo run --release --example zpl_comparison

# 3. Run full Labelary API visual parity analysis (requires Python 3)
python3 tools/parity.py --report
```

---

## 🎨 Color & Styling Extensions (`^GI`, `^GLC`, `^GTC`)

`zpl-forge` provides extended support for rendering custom colors and full-color images:

- **Base64 Color Images (`^GI` / `^GIC`):** Render full-color JPEG/PNG images encoded in Base64 natively.
- **Custom Line & Shape Colors (`^GLC#RRGGBB`):** Override default black strokes with custom RGB hex colors.
- **Custom Text Colors (`^GTC#RRGGBB`):** Style text elements with custom hex colors.

```rust
use zpl_forge::{render_png, render_pdf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zpl_color = r#"
        ^XA
        ^GLC#FF5733
        ^FO50,50^GB200,100,5^FS
        
        ^GTC#2E86C1
        ^CF0,40
        ^FO70,80^FDColor Styled Label^FS
        ^XZ
    "#;

    let png_bytes = render_png(zpl_color)?;
    std::fs::write("examples/showcase_color.png", png_bytes)?;

    let pdf_bytes = render_pdf(zpl_color)?;
    std::fs::write("examples/showcase_color.pdf", pdf_bytes)?;
    Ok(())
}
```

---

## 🛠 Ready-to-Run Code Examples

### 1. Basic Quick Start (`examples/basic.rs`)

Simple label generation in both PNG and PDF formats using default embedded fonts.

```rust
use std::collections::HashMap;
use zpl_forge::forge::pdf_native::PdfNativeBackend;
use zpl_forge::forge::png::PngBackend;
use zpl_forge::{Resolution, Unit, ZplEngine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zpl = "^XA^FO50,50^A0N,40,40^FDZPL FORGE QUICKSTART^FS^FO50,110^GB700,5,5^FS^XZ";

    let engine = ZplEngine::new(zpl, Unit::Inches(4.0), Unit::Inches(2.0), Resolution::Dpi203)?;

    let png_bytes = engine.render(PngBackend::new(), &HashMap::new())?;
    std::fs::write("examples/showcase_basic.png", png_bytes)?;

    let pdf_bytes = engine.render(PdfNativeBackend::new(), &HashMap::new())?;
    std::fs::write("examples/showcase_basic.pdf", pdf_bytes)?;

    println!("Successfully generated showcase_basic.png and showcase_basic.pdf!");
    Ok(())
}
```

---

### 2. External TTF Custom Fonts (`examples/custom_fonts.rs`)

Register and embed custom TrueType fonts (`.ttf`) into label renderings.

```rust
use std::collections::HashMap;
use std::sync::Arc;
use zpl_forge::forge::pdf_native::PdfNativeBackend;
use zpl_forge::{FontManager, Resolution, Unit, ZplEngine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut font_manager = FontManager::default();
    let font_bytes = std::fs::read("examples/fonts/Roboto-Regular.ttf")?;

    font_manager.register_font("Roboto", &font_bytes, 'A', 'A')?;

    let zpl = "^XA^FO50,50^AAN,50,50^FDRoboto Custom Font^FS^XZ";
    let mut engine = ZplEngine::new(zpl, Unit::Inches(4.0), Unit::Inches(2.0), Resolution::Dpi203)?;
    engine.set_fonts(Arc::new(font_manager));

    let pdf_bytes = engine.render(PdfNativeBackend::new(), &HashMap::new())?;
    std::fs::write("examples/showcase_custom_fonts.pdf", pdf_bytes)?;
    Ok(())
}
```
