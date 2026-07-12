# ZPL-Forge

A fast, memory-safe ZPL (Zebra Programming Language) parser and renderer for Rust. It converts ZPL code into PNG images or native, selectable vector PDFs.

[![Crates.io](https://img.shields.io/crates/v/zpl_forge.svg)](https://crates.io/crates/zpl_forge)
[![Docs.rs](https://docs.rs/zpl-forge/badge.svg)](https://docs.rs/zpl-forge)

## The Purpose

ZPL-Forge provides a quick and simple alternative for creating documents like **shipping guides, delivery receipts, and tickets**. It is optimized for use cases where speed and ease of integration are preferred over extreme document complexity.

## The Results

|                                             Standard Complex Label                                             |                                                 Custom Image Extensions                                                  |
| :------------------------------------------------------------------------------------------------------------: | :----------------------------------------------------------------------------------------------------------------------: |
| <img src="https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_01.png" width="300" /> | <img src="https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_image_color2.png" width="300" /> |

Check out the [**Visual Documentation (EXAMPLES.md)**](https://github.com/rafael-arreola/zpl-forge/blob/main/examples/EXAMPLES.md) for more ready-to-run code samples and their generated output images.

## Performance

ZPL-Forge is engineered to deliver enterprise-grade performance and ultra-low latency, making it perfect for both instant single-label previews and high-throughput bulk generation.

**Single Label Render Times (Measured in release mode using the embedded 64 KB Iosevka Term Slab font):**

- **Routing/Dispatch Label (`test_02`):**
  - PNG Output (`PngBackend`): **0.62 ms**
  - Native PDF Output (`PdfNativeBackend`): **2.06 ms**
- **Standard Shipping Label (`test_01`):**
  - PNG Output (`PngBackend`): **4.96 ms**
  - Native PDF Output (`PdfNativeBackend`): **2.33 ms**

**Massive Bulk Batching (Measured in release mode):**

- **Native Vector PDF (1,000 Combined Labels):** **75 ms** total time (**0.1 ms / page**), producing a compact, searchable file of only **0.82 MB** (a throughput of over **13,000 pages per second**!).

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
zpl-forge = "0.3.0"
```

### Cargo Features

| Feature   | Default | Enables                                              |
| :-------- | :-----: | :--------------------------------------------------- |
| `png`     |   ✅    | `PngBackend` raster output (`image`, `imageproc`)    |
| `pdf`     |   ✅    | `PdfNativeBackend` vector output (`lopdf`, `flate2`) |
| `tracing` |   ❌    | Internal debug logging via the `tracing` crate       |

If you only need one output format, disable default features to cut compile time and binary size:

```toml
[dependencies]
zpl-forge = { version = "0.3.0", default-features = false, features = ["pdf"] }
```

## Quick Start

The library provides two backends for different needs.

### 1. Render to PNG

Best for web previews or raster printing. Gated under the `png` cargo feature.

```rust
use std::collections::HashMap;
use zpl_forge::{Resolution, Unit, ZplEngine};
use zpl_forge::forge::png::PngBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zpl = "^XA^FO50,50^A0N,50,50^FDHello World^FS^XZ";

    // Parse the ZPL and define label constraints (4x4 inches at 203 DPI)
    let engine = ZplEngine::new(
        zpl,
        Unit::Inches(4.0),
        Unit::Inches(4.0),
        Resolution::Dpi203,
    )?;

    // Create the PNG rendering backend
    let backend = PngBackend::new();

    // Render the label (returns a Vec<u8> of raw PNG bytes)
    let png_bytes = engine.render(backend, &HashMap::new())?;

    // Save the bytes to a file
    std::fs::write("label.png", png_bytes)?;
    Ok(())
}
```

### 2. Render to Native Vector PDF

Outputs selectable text and native vector graphics. Ultra-fast, extremely small file size, and requires zero rasterization. Gated under the `pdf` cargo feature.

```rust
use std::collections::HashMap;
use zpl_forge::{Resolution, Unit, ZplEngine};
use zpl_forge::forge::pdf_native::PdfNativeBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zpl = "^XA^FO50,50^A0N,50,50^FDSelectable Text!^FS^XZ";

    // Parse the ZPL and define label constraints (4x4 inches at 203 DPI)
    let engine = ZplEngine::new(
        zpl,
        Unit::Inches(4.0),
        Unit::Inches(4.0),
        Resolution::Dpi203,
    )?;

    // Create the native vector PDF rendering backend
    let backend = PdfNativeBackend::new();

    // Render the label (returns a Vec<u8> of raw PDF bytes)
    let pdf_bytes = engine.render(backend, &HashMap::new())?;

    // Save the bytes to a file
    std::fs::write("label.pdf", pdf_bytes)?;
    Ok(())
}
```

## Advanced Usage

### Templating (Variables)

Inject dynamic data into your ZPL without extra allocations. Simply use the `{{variable_name}}` syntax in your ZPL code and pass a variables map to `.render()`.

```rust
use std::collections::HashMap;
use zpl_forge::{Resolution, Unit, ZplEngine};
use zpl_forge::forge::png::PngBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zpl = "^XA^FO50,50^A0N,50,50^FDHello {{NAME}}^FS^XZ";
    let engine = ZplEngine::new(
        zpl,
        Unit::Inches(4.0),
        Unit::Inches(2.0),
        Resolution::Dpi203,
    )?;

    // Populate the template variables
    let mut variables = HashMap::new();
    variables.insert("NAME".to_string(), "ZPL-Forge".to_string());

    // Render with variables injected
    let png_bytes = engine.render(PngBackend::new(), &variables)?;
    std::fs::write("hello.png", png_bytes)?;
    Ok(())
}
```

### Conditional Rendering (`^IFC`)

A custom extension to render elements only if a variable matches a specific value.

```rust
let zpl = "^XA^FO50,50^IFCuser_type,admin^A0N,50,50^FDAdmin Only Area^FS^XZ";
// "Admin Only Area" will only render if a variable ("user_type", "admin") is passed to render.
```

### Multi-Page PDF Batching & Compression

You can combine multiple physical labels into a single multi-page PDF document simply by concatenating multiple `^XA...^XZ` blocks in your ZPL input. The `PdfNativeBackend` automatically treats each block as a separate page, drawing it natively. You can also customize the zlib compression level using `flate2::Compression`.

```rust
use std::collections::HashMap;
use zpl_forge::{ZplEngine, Unit, Resolution};
use zpl_forge::forge::pdf_native::PdfNativeBackend;
use flate2::Compression;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate concatenated ZPL representing a multi-page document
    let mut batch_zpl = String::new();
    for i in 1..=3 {
        batch_zpl.push_str(&format!(
            "^XA^FO50,50^A0N,40,40^FDThis is native PDF page {i}^FS^XZ\n"
        ));
    }

    let engine = ZplEngine::new(
        &batch_zpl,
        Unit::Inches(4.0),
        Unit::Inches(3.0),
        Resolution::Dpi203,
    )?;

    // Instantiate backend with Best compression & document title metadata
    let backend = PdfNativeBackend::new()
        .with_compression(Compression::best())
        .with_title("Batch PDF Generation");

    // Renders all pages in one extremely efficient vector PDF document
    let pdf_bytes = engine.render(backend, &HashMap::new())?;
    std::fs::write("batch.pdf", pdf_bytes)?;
    Ok(())
}
```

### Custom Fonts

ZPL-Forge ships with an embedded open-source font (Iosevka Term Slab) mapped to every ZPL identifier, so it works out of the box with zero system dependencies. To use your own typography, register TrueType (`.ttf`) or OpenType (`.otf`) bytes on a `FontManager` and attach it to the engine. Each font is bound to a range of ZPL identifiers (`A`–`Z`, `0`–`9`) and selected from ZPL with `^A` or `^CF`.

```rust
use std::collections::HashMap;
use std::sync::Arc;
use zpl_forge::{FontManager, Resolution, Unit, ZplEngine};
use zpl_forge::forge::png::PngBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut fonts = FontManager::default();

    // From a bundled asset (compiled into your binary):
    // fonts.register_font("Roboto", include_bytes!("../assets/Roboto-Regular.ttf"), '0', '0')?;

    // From any file on disk, including OS font directories
    // (macOS: /System/Library/Fonts, Linux: /usr/share/fonts, Windows: C:\Windows\Fonts):
    let bytes = std::fs::read("/usr/share/fonts/truetype/roboto/Roboto-Regular.ttf")?;
    fonts.register_font("Roboto", &bytes, 'T', 'T')?; // available as ^AT / ^CFT

    let zpl = "^XA^FO50,50^ATN,50,50^FDRendered with Roboto^FS^XZ";
    let mut engine = ZplEngine::new(zpl, Unit::Inches(4.0), Unit::Inches(2.0), Resolution::Dpi203)?;
    engine.set_fonts(Arc::new(fonts));

    let png = engine.render(PngBackend::new(), &HashMap::new())?;
    std::fs::write("custom_font.png", png)?;
    Ok(())
}
```

See [`examples/custom_fonts.rs`](examples/custom_fonts.rs) for a runnable demo that registers ten different families at once.

#### Weights and styles (Light / Regular / Bold)

ZPL-Forge does not synthesize weights or slants — the glyphs come straight from the font file. To offer multiple weights, register one **static instance per weight** under its own identifier and pick it from ZPL:

```rust
fonts.register_font("Roboto Light", &light_bytes, 'L', 'L')?;   // ^ALN,40,40
fonts.register_font("Roboto Regular", &regular_bytes, 'R', 'R')?; // ^ARN,40,40
fonts.register_font("Roboto Bold", &bold_bytes, 'B', 'B')?;     // ^ABN,40,40
```

> **Variable fonts:** axes (`wght`, `wdth`, `MONO`, …) are not supported — a variable TTF renders at its default instance only. Export static instances first, e.g. with fonttools: `fonttools varLib.instancer Font-VF.ttf wght=700 -o Font-Bold.ttf`.

#### Condensation (horizontal scale)

Two ways to condense or expand text:

1. **The `w` parameter of `^A`** — glyphs are scaled horizontally by the `w/h` ratio, exactly like a Zebra printer. `^A0N,60,40` renders 60-dot-tall glyphs compressed to ⅔ of their natural width; `^A0N,60,90` expands them 1.5×. Omitting `w` keeps the font's natural proportions.
2. **Register a naturally condensed family** (e.g., Roboto Condensed, Archivo Narrow) when you want true condensed letterforms instead of a geometric squeeze.

#### Identifier semantics: bitmap vs scalable

To match real printer output, identifiers **`A`–`H`** emulate Zebra's built-in **bitmap fonts**: heights snap to integer magnifications of the original dot-matrix cell (font A is 9×5 dots) and the glyphs are stretched to the fixed cell aspect — this is what makes `^CFA,30` look exactly like Labelary/hardware. Identifiers **`0`–`9`** and **`I`–`Z`** use the **scalable** model (any height, natural glyph proportions, capital letters spanning ~75% of the `^A` height).

> Register decorative or brand fonts on scalable identifiers (`0`–`9`, `I`–`Z`). A custom font placed on `A`–`H` inherits the bitmap cell geometry and will look artificially stretched.

## Supported ZPL Commands

| Command | Name             | Parameters    | Description                                                                                                 |
| :------ | :--------------- | :------------ | :---------------------------------------------------------------------------------------------------------- |
| `^A`    | Font Spec        | `f,o,h,w`     | Specifies font (A..Z, 0..9), orientation (N, R, I, B — text rotation supported), height, and width in dots. |
| `^B2`   | Interleaved 2/5  | `o,h,f,g,e`   | Interleaved 2 of 5 Barcode (cartons, ITF-14).                                                               |
| `^B3`   | Code 39          | `o,e,h,f,g`   | Code 39 Barcode.                                                                                            |
| `^B7`   | PDF417           | `o,h,s,c,r,t` | PDF417 two-dimensional Barcode.                                                                             |
| `^BA`   | Code 93          | `o,h,f,g,e`   | Code 93 Barcode.                                                                                            |
| `^BC`   | Code 128         | `o,h,f,g,e,m` | Code 128 Barcode.                                                                                           |
| `^BE`   | EAN-13           | `o,h,f,g`     | EAN-13 Barcode (retail).                                                                                    |
| `^BQ`   | QR Code          | `o,m,s,e,k`   | QR Code (Model 1 or 2).                                                                                     |
| `^BU`   | UPC-A            | `o,h,f,g,e`   | UPC-A Barcode (retail).                                                                                     |
| `^BX`   | Data Matrix      | `o,h,s,c,r`   | Data Matrix (ECC 200) two-dimensional Barcode.                                                              |
| `^BY`   | Barcode Default  | `w,r,h`       | Sets default values for barcodes (module width, ratio, and height).                                         |
| `^CF`   | Change Def. Font | `f,h,w`       | Changes the default alphanumeric font.                                                                      |
| `^FB`   | Field Block      | `w,l,s,j,i`   | Wraps text in a block: width, max lines, line spacing, justification (L/C/R), indent. `\&` breaks lines.    |
| `^FD`   | Field Data       | `d`           | Data to print in the current field.                                                                         |
| `^FO`   | Field Origin     | `x,y`         | Sets the top-left corner of the field.                                                                      |
| `^FR`   | Field Reverse    | N/A           | Inverts the field color (white on black).                                                                   |
| `^FS`   | Field Separator  | N/A           | Indicates the end of a field definition.                                                                    |
| `^FT`   | Field Typeset    | `x,y`         | Sets field position relative to the text baseline.                                                          |
| `^GB`   | Graphic Box      | `w,h,t,c,r`   | Draws a box, line, or rectangle with rounded corners.                                                       |
| `^GC`   | Graphic Circle   | `d,t,c`       | Draws a circle by specifying its diameter.                                                                  |
| `^GD`   | Graphic Diagonal | `w,h,t,c,o`   | Draws a diagonal line (`/` or `\`).                                                                         |
| `^GE`   | Graphic Ellipse  | `w,h,t,c`     | Draws an ellipse.                                                                                           |
| `^GF`   | Graphic Field    | `c,b,f,p,d`   | Renders a bitmap image (supports A/Hex type compression).                                                   |
| `^XA`   | Start Format     | N/A           | Indicates the start of a label. Multiple `^XA...^XZ` blocks become pages in the native PDF backend.         |
| `^XZ`   | End Format       | N/A           | Indicates the end of a label.                                                                               |

## Custom Commands (Extensions)

| Command | Name         | Parameters | Description                                                                                                                                                   |
| :------ | :----------- | :--------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `^GIC`  | Custom Image | `w,h,d`    | Renders a color image. **w** and **h** define size. **d** is the binary (PNG/JPG) in **Base64**.                                                              |
| `^GLC`  | Line Color   | `c`        | Sets the color for graphic elements in hexadecimal format (e.g., `#FF0000`).                                                                                  |
| `^GTC`  | Text Color   | `c`        | Sets the color for text fields in hexadecimal format (e.g., `#0000FF`).                                                                                       |
| `^IFC`  | Cond. Render | `var,val`  | **If Condition Custom:** Evaluates if a variable matches a specific value. If false, the field won't be rendered. Scope limited up to the next `^FS` command. |

## Security and Limits

ZPL-Forge imposes reasonable limits to prevent resource exhaustion from malformed or malicious ZPL inputs.

- **Maximum Document Size:** Bounded to prevent memory overflow on excessively large labels.
- **Graphic Field Maximums:** Prevents malicious `^GF` commands from allocating unlimited memory.
- **Maximum Text Size:** Prevents excessively large font sizes.

## License

Dual-licensed under either:

- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

At your option.
