# ZPL-Forge

[![Crates.io](https://img.shields.io/crates/v/zpl_forge.svg)](https://crates.io/crates/zpl_forge)
[![Docs.rs](https://docs.rs/zpl-forge/badge.svg)](https://docs.rs/zpl-forge)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/rafael-arreola/zpl-forge#license)

`zpl-forge` is a high-performance engine written in Rust for parsing, processing, and rendering Zebra Programming Language (ZPL) labels into **PNG**, **PDF** (raster), and **Native Vector PDF** formats. It features an AST-based parser, a global state machine, zero-allocation templating, and native multi-threading capabilities.

---

### The Results

|                                             Standard Complex Label                                             |                                                 Custom Image Extensions                                                  |
| :------------------------------------------------------------------------------------------------------------: | :----------------------------------------------------------------------------------------------------------------------: |
| <img src="https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_01.png" width="300" /> | <img src="https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_image_color2.png" width="300" /> |
|                                         Rendered to PNG in **~8.1 ms**                                         |                                             Rendered to PNG in **~22.2 ms**                                              |

Check out the [**Visual Documentation (EXAMPLES.md)**](https://github.com/rafael-arreola/zpl-forge/blob/main/examples/EXAMPLES.md) for more ready-to-run code samples and their generated output images.

---

## Why zpl-forge? (Use Cases)

If your business generates ZPL code (UPS, FedEx, USPS, internal routing), you likely need to handle that code outside of a physical Zebra printer:

- **Web & Mobile Previews**: Render ZPL to PNG to show customers or warehouse staff exactly what their shipping label will look like before printing.
- **Hardware Agnosticism**: Convert Zebra code to PDF to print on generic thermal, laser, or inkjet printers without buying expensive Zebra hardware.
- **Record Archiving**: Save exact digital PDF copies of physical shipping labels for compliance or customer support.
- **Dynamic Templating**: Inject variables (`{{tracking}}`, `{{name}}`) directly into the ZPL stream without string allocations.

## Three Rendering Backends

`zpl-forge` provides three output backends, each suited for different use cases:

| Backend              | Module              | Output                | How it works                                                                                                                                                                            |
| :------------------- | :------------------ | :-------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **PngBackend**       | `forge::png`        | Raster PNG image      | Draws onto an RGB canvas via `imageproc`. Best for previews and thumbnails.                                                                                                             |
| **PdfBackend**       | `forge::pdf`        | PDF (embedded raster) | Renders the label as a high-resolution PNG first, then embeds it into a PDF page. Simple and pixel-accurate, but not scalable.                                                          |
| **PdfNativeBackend** | `forge::pdf_native` | PDF (native vectors)  | Text, shapes, and barcodes are emitted as **native PDF operations** (paths, Bézier curves, embedded TTF fonts). Fully scalable, selectable text, smaller files for vector-heavy labels. |

### PdfNativeBackend Architecture

```
ZPL Instructions
       │
       ▼
┌─────────────────────────────────┐
│       PdfNativeBackend          │
│                                 │
│  ┌───────────┐  ┌────────────┐  │
│  │  Text     │  │  Shapes    │  │
│  │  BT/ET    │  │  Bézier    │  │
│  │  Tm + Tf  │  │  re/m/l/c  │  │
│  │  Tj       │  │  f         │  │
│  └───────────┘  └────────────┘  │
│  ┌───────────┐  ┌────────────┐  │
│  │  Barcodes │  │  Images    │  │
│  │  re + f   │  │  XObject   │  │
│  │  (native  │  │  (zlib     │  │
│  │   rects)  │  │   RGB)     │  │
│  └───────────┘  └────────────┘  │
│  ┌──────────────────────────┐   │
│  │  Reverse Print           │   │
│  │  ExtGState BM/Difference │   │
│  └──────────────────────────┘   │
│                                 │
│  Font: Embedded TTF (Oswald)    │
│  Coords: dots → PDF points     │
│           (Y-flip)              │
└─────────────────────────────────┘
       │
       ▼
   lopdf Document
   (PDF 1.5)
```

- **Text**: Embedded TrueType font via `FontData`/`add_font`. Positioned with the `Tm` text matrix to support independent width/height scaling. Baseline calculated from `ab_glyph` ascent metrics.
- **Shapes**: Rounded rectangles use cubic Bézier curves (κ = 0.5522). Circles and ellipses approximated with 4 Bézier segments. Hollow shapes rendered as outer fill + inner clear fill.
- **Barcodes**: `rxing` generates the `BitMatrix`; each bar/cell becomes a native `re` (rectangle) operation. Supports N/R/I/B orientations.
- **Images**: Bitmap fields (`^GF`) and custom images (`^GIC`) embedded as zlib-compressed XObject streams.
- **Reverse Print**: Simulated via `ExtGState` with `BlendMode = Difference`.

## ⚡ Blazing Fast Performance

`zpl-forge` is built for high-throughput enterprise environments. All three backends are benchmarked below.

### Render Time Comparison

| Label                                     | PngBackend | PdfBackend (raster) | PdfNativeBackend (vector) |
| :---------------------------------------- | :--------: | :-----------------: | :-----------------------: |
| **Shipping Label** (text, boxes, barcode) |   8.1 ms   |       21.8 ms       |        **4.4 ms**         |
| **Route Label** (text, lines)             |   1.0 ms   |       2.3 ms        |        **4.2 ms**         |
| **Color Label** (custom colors)           |   6.7 ms   |       19.8 ms       |        **4.5 ms**         |
| **Barcode Label** (Code128, Code39, QR)   |   7.9 ms   |       23.6 ms       |        **4.8 ms**         |
| **Bitmap Image** (`^GF`)                  |   5.1 ms   |       15.2 ms       |          7.6 ms           |
| **Full-Color Image** (`^GIC` base64)      |  14.2 ms   |       57.9 ms       |          43.2 ms          |

> **Key takeaway**: For **vector-heavy labels** (text, shapes, barcodes), `PdfNativeBackend` is **~5× faster** than `PdfBackend` and even outperforms `PngBackend` because it skips rasterization entirely. For image-heavy labels, the native backend remains competitive while producing scalable output.

### Bulk PDF Generation

🚀 Render **1,000 unique shipping labels** into a single multi-page PDF in **~1.0 second** (parallel multi-core rendering + PDF multiplexing). `ZplEngine` is `Send + Sync`.

| Compression | Merge Time | File Size |
| :---------- | :--------: | :-------: |
| `fast`      |   558 ms   |  36.4 MB  |
| `default`   |   1.03 s   |  24.6 MB  |
| `best`      |   2.07 s   |  21.0 MB  |

> _Benchmarks on Apple M-series. Reproduce via:_ `cargo run --example zpl_showcase`

---

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
zpl-forge = "0.2"
```

## Quick Start

### Render to PNG

```rust
use std::collections::HashMap;
use zpl_forge::{ZplEngine, Unit, Resolution};
use zpl_forge::forge::png::PngBackend;

fn main() -> zpl_forge::ZplResult<()> {
    let zpl = "^XA^FO50,50^A0N,50,50^FDHello ZPL-Forge^FS^XZ";
    let engine = ZplEngine::new(zpl, Unit::Inches(4.0), Unit::Inches(2.0), Resolution::Dpi203)?;
    let png_bytes = engine.render(PngBackend::new(), &HashMap::new())?;
    std::fs::write("label.png", png_bytes).ok();
    Ok(())
}
```

### Render to Native Vector PDF

```rust
use std::collections::HashMap;
use zpl_forge::{ZplEngine, Unit, Resolution};
use zpl_forge::forge::pdf_native::PdfNativeBackend;

fn main() -> zpl_forge::ZplResult<()> {
    let zpl = "^XA^FO50,50^A0N,50,50^FDHello ZPL-Forge^FS^XZ";
    let engine = ZplEngine::new(zpl, Unit::Inches(4.0), Unit::Inches(2.0), Resolution::Dpi203)?;
    let pdf_bytes = engine.render(PdfNativeBackend::new(), &HashMap::new())?;
    std::fs::write("label.pdf", pdf_bytes).ok();
    Ok(())
}
```

### Zero-Allocation Templating

```rust
use std::collections::HashMap;
use zpl_forge::{ZplEngine, Unit, Resolution};
use zpl_forge::forge::png::PngBackend;

fn main() -> zpl_forge::ZplResult<()> {
    let zpl_template = "^XA
        ^FO50,50^A0N,50,50^FDShip to: {{recipient}}^FS
        ^FO50,120^A0N,30,30^FDTracking: {{tracking_id}}^FS
        ^BY3,2,100^FO50,160^BC^FD{{tracking_id}}^FS
        ^XZ";

    let engine = ZplEngine::new(zpl_template, Unit::Inches(4.0), Unit::Inches(2.0), Resolution::Dpi203)?;

    let mut vars = HashMap::new();
    vars.insert("recipient".to_string(), "John Doe".to_string());
    vars.insert("tracking_id".to_string(), "1Z9999999999999999".to_string());

    let png_bytes = engine.render(PngBackend::new(), &vars)?;
    std::fs::write("label.png", png_bytes).ok();
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

---

## Advanced Usage

### Using Custom Fonts and Styles

```rust
use std::sync::Arc;
use std::collections::HashMap;
use zpl_forge::{ZplEngine, FontManager, Unit, Resolution};
use zpl_forge::forge::png::PngBackend;

fn main() -> zpl_forge::ZplResult<()> {
    let mut font_manager = FontManager::default();

    let roboto_regular = std::fs::read("fonts/Roboto-Regular.ttf").unwrap();
    let roboto_bold = std::fs::read("fonts/Roboto-Bold.ttf").unwrap();

    font_manager.register_font("Roboto Regular", &roboto_regular, 'A', 'A')?;
    font_manager.register_font("Roboto Bold", &roboto_bold, 'B', 'B')?;

    let zpl_input = "^XA
        ^FO50,50^AAN,50,50^FDThis is Regular^FS
        ^FO50,120^ABN,50,50^FDThis is Bold^FS
        ^XZ";

    let mut engine = ZplEngine::new(zpl_input, Unit::Inches(4.0), Unit::Inches(4.0), Resolution::Dpi203)?;
    engine.set_fonts(Arc::new(font_manager));

    engine.render(PngBackend::new(), &HashMap::new())?;
    Ok(())
}
```

### Conditional Rendering (`^IFC`)

```rust
use std::collections::HashMap;
use zpl_forge::{ZplEngine, Unit, Resolution};
use zpl_forge::forge::png::PngBackend;

fn main() {
    let zpl_input = "^XA
        ^FO50,50^IFCuser_type,admin^A0N,50,50^FDAdmin Only Area^FS
        ^FO50,150^A0N,50,50^FDPublic Text^FS
        ^XZ";

    let engine = ZplEngine::new(zpl_input, Unit::Inches(4.0), Unit::Inches(2.0), Resolution::Dpi203).unwrap();

    let mut vars = HashMap::new();
    vars.insert("user_type".to_string(), "guest".to_string());

    let png_bytes = engine.render(PngBackend::new(), &vars).unwrap();
}
```

### Multi-Page PDF Batching

```rust
use std::collections::HashMap;
use zpl_forge::{ZplEngine, Unit, Resolution};
use zpl_forge::forge::png::PngBackend;
use zpl_forge::forge::pdf::png_merge_pages_to_pdf;
use flate2::Compression;

fn main() -> zpl_forge::ZplResult<()> {
    let zpl_template = "^XA^FO50,50^FDOrder: {{order_id}}^FS^XZ";
    let (width, height, resolution) = (Unit::Inches(4.0), Unit::Inches(3.0), Resolution::Dpi203);
    let engine = ZplEngine::new(zpl_template, width, height, resolution)?;

    let mut pages: Vec<Vec<u8>> = Vec::new();
    for i in 0..100 {
        let mut vars = HashMap::new();
        vars.insert("order_id".to_string(), format!("ORD-{}", 1001 + i));
        pages.push(engine.render(PngBackend::new(), &vars)?);
    }

    let w = width.to_dots(resolution) as f64;
    let h = height.to_dots(resolution) as f64;
    let pdf_bytes = png_merge_pages_to_pdf(&pages, w, h, resolution.dpi(), Compression::default())?;
    std::fs::write("labels.pdf", pdf_bytes).ok();
    Ok(())
}
```

## Security and Limits

- **Canvas Size**: Maximum **8192 × 8192 pixels**.
- **ZPL Images (`^GF`)**: Decoded data capped at **10 MB** per command.
- **Safe Arithmetic**: Saturating arithmetic prevents integer overflows.
- **Unit Validation**: Negative physical dimensions are normalized.

## License

This project is licensed under either the MIT or Apache-2.0 license.
