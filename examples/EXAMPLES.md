# ZPL-Forge Examples & Capability Showcase

This document provides ready-to-run examples, a detailed feature showcase, and up-to-date performance benchmarks for `zpl-forge` utilizing the embedded default font set (**TeX Gyre Heros Cn** for scalable text, **Iosevka Term Slab** for monospace/bitmap emulation, plus **OCR-A** and **OCR-B** for identifiers `H` and `E`).

---

## 🌐 Multi-Language & Unicode Coverage

To maintain an extremely compact compiled binary size and sub-millisecond loading performance, the embedded monospace font (Iosevka Term Slab) is subsetted to the standard **ASCII + Latin-1 Supplement** Unicode range (`U+0020-00FF`).

This provides 100% full, compliant typographic support for:

- **Major Languages:** English, Spanish, French, German, Portuguese, Italian, Dutch, Swedish, Danish, Norwegian, Finnish, Icelandic, Irish, Basque, and Catalan.
- **Diacritics & Accents:** All standard Western European vowels, symbols, and punctuation marks (`ñ, á, é, í, ó, ú, ü, ç, ß, ä, ö, à, â, æ, ø, å, ¿, ¡`) are rendered flawlessly.
- **Extensibility:** If your thermal labels require Cyrillic, Greek, Asian character sets, or Eastern European Latin diacritics (like Polish `ł` or Turkish `ğ`), you can easily register custom full TrueType fonts using the `FontManager` (see **Example 5**).

---

## 🚀 Performance Benchmarks

All metrics below are measured on Apple M-series silicon (M1/M2/M3) in **release mode** (`cargo run --release --example zpl_showcase`).

### Single Label Rendering Times

Below is a detailed, sorted list of rendering times across both backends. These figures represent the **exact total processing times** (including parsing the ZPL template, injecting variables, drawing lines/vectors, and writing outputs on disk).

| Single Label Scenario                    | PNG Backend (`PngBackend`) | Native PDF (`PdfNativeBackend`) | Key Characteristics / Features Shown                            |
| :--------------------------------------- | :------------------------: | :-----------------------------: | :-------------------------------------------------------------- |
| **Route/Dispatch Label (`test_02`)**     |        **0.63 ms**         |             2.50 ms             | Minimal coordinate plotting, line drawing, simple text          |
| **Bitmap Image (`test_image`)**          |          6.62 ms           |           **1.53 ms**           | Standard monochrome bitmap decoding and draw (`^GF`)            |
| **Custom Colors Label (`test_03`)**      |          3.80 ms           |           **2.12 ms**           | Color customization (`^GLC`, `^GTC` custom hex)                 |
| **Rotated Bitmap Image (`test_image2`)** |          5.15 ms           |           **2.26 ms**           | Rotated / scale-adjusted monochrome bitmap                      |
| **Barcodes Label (`test_04`)**           |          4.71 ms           |           **2.49 ms**           | Standard Code 128, Code 39, QR, rotated barcodes                |
| **Conditional Label (`test_ifc`)**       |        **2.10 ms**         |             2.04 ms             | Dynamic elements rendered via variable conditions (`^IFC`)      |
| **Shipping Label (`test_01`)**           |          15.62 ms          |           **2.58 ms**           | Complex label: text wrapping, lines, barcodes, reverse print    |
| **Custom Fonts Label (`custom_fonts`)**  |        **4.94 ms**         |            106.09 ms            | Embeds **10 full, external, un-subsetted** TTF fonts (see note) |

_💡 **Note on Custom Fonts:** While compiling the lightweight **64 KB embedded Iosevka subset** takes only `1.1 ms`, loading 10 large, un-subsetted external fonts from disk (totaling over 15MB) requires a one-time cold-start parse of 106.09 ms in the PDF backend. Subsequent rendering calls for these fonts run in under 2 ms because of internal caching._

---

### Massive Batching Performance (1,000 Labels)

Generating large multi-page documents (such as daily shipping logs or warehouse inventory catalogs) is incredibly fast and highly optimized.

| Document Type                 | Rendering Engine            | Time to Render 1,000 Pages | Output File Size |
| :---------------------------- | :-------------------------- | :------------------------: | :--------------: |
| **Multi-Page Shipping Guide** | `PdfNativeBackend` (Vector) |        **97.24 ms**        |   **0.82 MB**    |

_💡 **File Footprint:** The native vector PDF engine generates fully compressed, searchable, selectable text vector PDFs of only ~820 bytes per page, keeping bulk output sizes extremely compact and fast to transmit over networks._

---

## 🎨 Backends Compared

### 1. PNG Backend (`PngBackend`)

- **Type:** Raster rendering.
- **Output:** Pixel-perfect PNG image bytes (`Vec<u8>`).
- **Sizing:** Restricted to a maximum safe resolution of `8192 x 8192` pixels to prevent memory overflow.
- **Use Cases:** Web browser label previews, thermal print previews.

### 2. Native PDF Backend (`PdfNativeBackend`)

- **Type:** High-performance vector graphics and searchable text.
- **Output:** Compressed vector PDF bytes (`Vec<u8>`).
- **Sizing:** Scalable, resolution-independent vector layouts.
- **Use Cases:** Electronic shipping documents, document archiving, digital invoices.

---

## 📁 Compiled Demos & Results (Direct PDF Links)

Since `zpl-forge` compiles these vector assets natively, you can inspect the output files directly from this repository on GitHub. Click the links below to download or view the generated vector PDF results:

- 🖨️ [**Standard Shipping Label (test_01_native.pdf)**](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/test_01_native.pdf): Shows text wrapping, standard line drawing, Code 128 barcode, and reverse printing (`^FR`).
- 🎨 [**Custom Hex Colors (test_03_native.pdf)**](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/test_03_native.pdf): Displays the extension properties `^GLC` and `^GTC` styling graphic rectangles and text in vibrant custom hex codes.
- 🖼️ [**Full-Color Base64 Images (test_image_color2_native.pdf)**](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/test_image_color2_native.pdf): Renders a rich color PNG/JPG image natively using the custom `^GIC` extension.
- 🔤 [**Multi-Font Typography (custom_fonts_output_native.pdf)**](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/custom_fonts_output_native.pdf): Showcases loading and embedding 10 distinct, external open-source TrueType fonts.
- 📄 [**1,000-Page Bulk Batch (multi_page_labels.pdf)**](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/multi_page_labels.pdf): Our high-throughput demo compiling 1,000 unique labels in **97 ms**, resulting in a tiny, compressed, hyper-crisp 0.82 MB file.

---

## 🛠 Feature & Code Examples

### 1. Simple Quick Start (PNG & PDF)

This shows how to compile a simple ZPL label and output it to both formats using the default embedded fonts (no system dependencies required).

```rust
use std::collections::HashMap;
use zpl_forge::{ZplEngine, Unit, Resolution};
use zpl_forge::forge::png::PngBackend;
use zpl_forge::forge::pdf_native::PdfNativeBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zpl_string = "^XA^FO50,50^A0N,40,40^FDZPL FORGE SHIPPING^FS^FO50,110^GB700,5,5^FS^XZ";

    // 1. Parse the template and define layout constraints (4x3 inches at 203 DPI)
    let engine = ZplEngine::new(
        zpl_string,
        Unit::Inches(4.0),
        Unit::Inches(3.0),
        Resolution::Dpi203,
    )?;

    // 2. Render to PNG
    let png_bytes = engine.render(PngBackend::new(), &HashMap::new())?;
    std::fs::write("quickstart.png", png_bytes)?;

    // 3. Render to Native Vector PDF
    let pdf_bytes = engine.render(PdfNativeBackend::new(), &HashMap::new())?;
    std::fs::write("quickstart.pdf", pdf_bytes)?;

    Ok(())
}
```

---

### 2. Standardized Barcode Generation

Standard Code 128 (`^BC` command) in Mode `N` defaults to **Code Set B** in `zpl-forge` to match physical thermal printer hardware. This guarantees that your generated barcodes are identical in width and bar spacing to the physical print output.

```rust
use std::collections::HashMap;
use zpl_forge::{ZplEngine, Unit, Resolution};
use zpl_forge::forge::png::PngBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zpl = r#"
        ^XA
        ^BY4,2,150                   (Set module width to 4, height to 150)
        ^FO100,50^BCN,150,Y,N,N^FD12345678^FS  (Draw standard Code 128)
        ^XZ
    "#;

    let engine = ZplEngine::new(
        zpl,
        Unit::Inches(4.0),
        Unit::Inches(3.0),
        Resolution::Dpi203,
    )?;

    let png_bytes = engine.render(PngBackend::new(), &HashMap::new())?;
    std::fs::write("standard_barcode.png", png_bytes)?;
    Ok(())
}
```

---

### 3. Dynamic Variables Substitution

You can define placeholders inside your ZPL using the `{{variable_name}}` syntax. Simply pass a `HashMap` of variables at render time, leaving the pre-parsed engine structure fully reusable.

```rust
use std::collections::HashMap;
use zpl_forge::{ZplEngine, Unit, Resolution};
use zpl_forge::forge::png::PngBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zpl_template = "^XA^FO50,50^A0N,40,40^FDCustomer: {{NAME}}^FS^FO50,110^FDId: {{ORDER_ID}}^FS^XZ";

    let engine = ZplEngine::new(
        zpl_template,
        Unit::Inches(4.0),
        Unit::Inches(2.0),
        Resolution::Dpi203,
    )?;

    // Inject variable map dynamically at render time
    let mut variables = HashMap::new();
    variables.insert("NAME".to_string(), "Alice Smith".to_string());
    variables.insert("ORDER_ID".to_string(), "9876543210".to_string());

    let png_bytes = engine.render(PngBackend::new(), &variables)?;
    std::fs::write("variables_substitution.png", png_bytes)?;
    Ok(())
}
```

---

### 4. Custom Styling & Logic Extensions

ZPL-Forge expands standard ZPL commands with native coloring properties and logical routing.

#### A. Custom Hex Colors (`^GLC` and `^GTC`)

Style your lines and typography with custom hexadecimal colors.

- `^GLC#RRGGBB` sets the color for graphic elements (rectangles, circles, lines).
- `^GTC#RRGGBB` sets the color for text.

```zpl
^XA
^GLC#FF5733               (Sets graphic element color to Vibrant Orange)
^FO50,50^GB200,100,5^FS
^GTC#2E86C1               (Sets typography color to Ocean Blue)
^FO50,180^A0N,40,40^FDColored Extension^FS
^XZ
```

#### B. Conditional Logic (`^IFC`)

Allows optional rendering depending on dynamic parameters. If the conditional variable does not match the expected value, the instruction (scoped up to the next `^FS`) will skip rendering.

- Syntax: `^IFCvariable_name,matching_value`

```zpl
^XA
^FO50,50^IFCuser_type,admin^A0N,40,40^FDAdministrator Console Badge^FS
^XZ
```

---

### 5. Custom Font Registration (TrueType)

You can load and register custom external TrueType (`.ttf`) or OpenType (`.otf`) fonts dynamically, linking them to ZPL font identifiers (A-Z and 0-9).

```rust
use std::collections::HashMap;
use std::sync::Arc;
use zpl_forge::{ZplEngine, FontManager, Unit, Resolution};
use zpl_forge::forge::pdf_native::PdfNativeBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut font_manager = FontManager::default();

    // Load custom font file bytes from disk
    let font_bytes = std::fs::read("examples/fonts/Roboto-Regular.ttf")?;

    // Register font name and link it to identifier 'A'
    font_manager.register_font("Roboto", &font_bytes, 'A', 'A')?;

    let zpl_input = "^XA^FO50,50^AAN,50,50^FDRoboto Typography on Label^FS^XZ";

    let mut engine = ZplEngine::new(
        zpl_input,
        Unit::Inches(4.0),
        Unit::Inches(2.0),
        Resolution::Dpi203,
    )?;

    // Supply font manager to the engine
    engine.set_fonts(Arc::new(font_manager));

    let pdf_bytes = engine.render(PdfNativeBackend::new(), &HashMap::new())?;
    std::fs::write("custom_fonts.pdf", pdf_bytes)?;
    Ok(())
}
```

---

## 💻 Running the Showcase Locally

All of these outputs are automatically generated and benchmarked by the showcase script. You can execute it locally by running:

```bash
cargo run --release --example zpl_showcase
```

Output files (`.png` and `.pdf`) will be generated directly within the `examples/` directory.
