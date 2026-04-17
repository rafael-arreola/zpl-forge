# ZPL-Forge

A fast, memory-safe ZPL (Zebra Programming Language) parser and renderer for Rust. It converts ZPL code into PNG images, standard PDFs, or native, selectable vector PDFs.

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

ZPL-Forge is designed to be extremely fast and efficient, ideal for both single-label rendering and bulk generation.

**Single Label Render Times:**

- **PNG:** ~7.8 ms
- **Native PDF (Vector):** ~8.5 ms
- **Standard PDF (Raster):** ~21.1 ms

**Bulk PDF Generation (1000 labels):**
All 1000 pages render in **~400 ms** (0.4 ms/page).

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
zpl-forge = "0.2.1"
```

## Quick Start

The library provides three backends for different needs.

### 1. Render to PNG

Best for web previews or raster printing.

```rust
use zpl_forge::{ZplEngine, PngBackend};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zpl = "^XA^FO50,50^A0N,50,50^FDHello World^FS^XZ";
    let mut engine = ZplEngine::new(zpl);
    let mut backend = PngBackend::new();

    // Render at 4x4 inches
    engine.render(&mut backend, 4.0, 4.0, &[])?;
    backend.save("label.png")?;
    Ok(())
}
```

### 2. Render to Native Vector PDF

Outputs selectable text and vector graphics. Ultra-fast and small file size.

```rust
use zpl_forge::{ZplEngine, PdfNativeBackend};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zpl = "^XA^FO50,50^A0N,50,50^FDSelectable Text!^FS^XZ";
    let mut engine = ZplEngine::new(zpl);
    let mut backend = PdfNativeBackend::new();

    // Render at 4x4 inches
    engine.render(&mut backend, 4.0, 4.0, &[])?;
    backend.save("label.pdf")?;
    Ok(())
}
```

### 3. Render to Standard PDF (Raster)

Renders to an image first and then wraps it in a PDF. Pixel-perfect but non-selectable.

```rust
use zpl_forge::{ZplEngine, PdfBackend};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zpl = "^XA^FO50,50^A0N,50,50^FDPixel Perfect^FS^XZ";
    let mut engine = ZplEngine::new(zpl);
    let mut backend = PdfBackend::new();

    // Render at 4x4 inches
    engine.render(&mut backend, 4.0, 4.0, &[])?;
    backend.save("label_raster.pdf")?;
    Ok(())
}
```

## Advanced Usage

### Templating (Variables)

Inject dynamic data into your ZPL without extra allocations.

```rust
let zpl = "^XA^FO50,50^A0N,50,50^FDHello {{NAME}}^FS^XZ";
let mut engine = ZplEngine::new(zpl);
engine.render(&mut backend, 4.0, 4.0, &[("NAME", "ZPL-Forge")])?;
```

### Conditional Rendering (`^IFC`)

A custom extension to render elements only if a variable matches a specific value.

```rust
let zpl = "^XA^FO50,50^IFCuser_type,admin^A0N,50,50^FDAdmin Only Area^FS^XZ";
// "Admin Only Area" will only render if ("user_type", "admin") is passed to render.
```

### Multi-Page PDF Batching & Compression

You can reuse a `PdfNativeBackend` to bundle many labels into one file and control the output size.

```rust
use zpl_forge::{ZplEngine, PdfNativeBackend, CompressionLevel};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = PdfNativeBackend::new();

    for i in 1..=3 {
        let zpl = format!("^XA^FO50,50^A0N,50,50^FDPage {i}^FS^XZ");
        let mut engine = ZplEngine::new(&zpl);
        engine.render(&mut backend, 4.0, 4.0, &[])?;
    }

    // Save with a specific compression level: Fast, Default, or Best
    backend.save_with_compression("batch.pdf", CompressionLevel::Best)?;
    Ok(())
}
```

## Supported ZPL Commands

| Command | Name             | Parameters    | Description                                                                       |
| :------ | :--------------- | :------------ | :-------------------------------------------------------------------------------- |
| `^A`    | Font Spec        | `f,o,h,w`     | Specifies font (A..Z, 0..9), orientation (N, R, I, B), height, and width in dots. |
| `^B3`   | Code 39          | `o,e,h,f,g`   | Code 39 Barcode.                                                                  |
| `^BC`   | Code 128         | `o,h,f,g,e,m` | Code 128 Barcode.                                                                 |
| `^BQ`   | QR Code          | `o,m,s,e,k`   | QR Code (Model 1 or 2).                                                           |
| `^BY`   | Barcode Default  | `w,r,h`       | Sets default values for barcodes (module width, ratio, and height).               |
| `^CF`   | Change Def. Font | `f,h,w`       | Changes the default alphanumeric font.                                            |
| `^FD`   | Field Data       | `d`           | Data to print in the current field.                                               |
| `^FO`   | Field Origin     | `x,y`         | Sets the top-left corner of the field.                                            |
| `^FR`   | Field Reverse    | N/A           | Inverts the field color (white on black).                                         |
| `^FS`   | Field Separator  | N/A           | Indicates the end of a field definition.                                          |
| `^FT`   | Field Typeset    | `x,y`         | Sets field position relative to the text baseline.                                |
| `^GB`   | Graphic Box      | `w,h,t,c,r`   | Draws a box, line, or rectangle with rounded corners.                             |
| `^GC`   | Graphic Circle   | `d,t,c`       | Draws a circle by specifying its diameter.                                        |
| `^GE`   | Graphic Ellipse  | `w,h,t,c`     | Draws an ellipse.                                                                 |
| `^GF`   | Graphic Field    | `c,b,f,p,d`   | Renders a bitmap image (supports A/Hex type compression).                         |
| `^XA`   | Start Format     | N/A           | Indicates the start of a label.                                                   |
| `^XZ`   | End Format       | N/A           | Indicates the end of a label.                                                     |

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
