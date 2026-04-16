# ZPL-Forge

[![Crates.io](https://img.shields.io/crates/v/zpl_forge.svg)](https://crates.io/crates/zpl_forge)
[![Docs.rs](https://docs.rs/zpl-forge/badge.svg)](https://docs.rs/zpl-forge)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/rafael-arreola/zpl-forge#license)

`zpl-forge` is a high-performance engine written in Rust for parsing, processing, and rendering Zebra Programming Language (ZPL) labels into formats like **PNG** and **PDF**. It features an AST-based parser, a global state machine, zero-allocation templating, and native multi-threading capabilities.

---

### The Results

|                                             Standard Complex Label                                             |                                                 Custom Image Extensions                                                  |
| :------------------------------------------------------------------------------------------------------------: | :----------------------------------------------------------------------------------------------------------------------: |
| <img src="https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_01.png" width="300" /> | <img src="https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_image_color2.png" width="300" /> |
|                                        Rendered to PNG in **~20.6 ms**                                         |                                             Rendered to PNG in **~21.8 ms**                                              |

Check out the [**Visual Documentation (EXAMPLES.md)**](https://github.com/rafael-arreola/zpl-forge/blob/main/examples/EXAMPLES.md) for more ready-to-run code samples and their generated output images.

---

## Why zpl-forge? (Use Cases)

If your business generates ZPL code (UPS, FedEx, USPS, internal routing), you likely need to handle that code outside of a physical Zebra printer:

- **Web & Mobile Previews**: Render ZPL to PNG to show customers or warehouse staff exactly what their shipping label will look like before printing.
- **Hardware Agnosticism**: Convert Zebra code to PDF to print on generic thermal, laser, or inkjet printers without buying expensive Zebra hardware.
- **Record Archiving**: Save exact digital PDF copies of physical shipping labels for compliance or customer support.
- **Dynamic Templating**: Inject variables (`{{tracking}}`, `{{name}}`) directly into the ZPL stream without string allocations.

## ⚡ Blazing Fast Performance

`zpl-forge` is built for high-throughput enterprise environments.

| Operation                                       | Format | Render Time | Total Processing Time |
| :---------------------------------------------- | :----: | :---------: | :-------------------: |
| **Complex Shipping Label** (Barcodes, Graphics) | `PNG`  |  ~29.8 ms   |       ~30.4 ms        |
| **Complex Shipping Label** (Barcodes, Graphics) | `PDF`  |  ~26.6 ms   |       ~27.0 ms        |
| **Simple Dispatch Label** (Text, Lines)         | `PNG`  |   ~1.3 ms   |        ~1.5 ms        |
| **Bitmap Image Decoding** (`^GF`)               | `PNG`  |   ~5.5 ms   |        ~5.9 ms        |
| **Conditional Rendering** (`^IFC`)              | `PNG`  |   ~4.0 ms   |        ~4.2 ms        |

🚀 **Bulk PDF Generation (Parallel)**: Render **1,000 unique shipping labels** into a single multi-page PDF in **~1.0 second** (0.5s for parallel multi-core rendering + 0.5s for PDF multiplexing with `fast` compression). `ZplEngine` is `Send + Sync` — render thousands of pages across all CPU cores simultaneously.

> _Benchmarks run on Apple Silicon. You can reproduce these locally via:_
> `cargo run --example zpl_showcase`

---

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
zpl-forge = "0.2"
```

## Quick Start: Zero-Allocation Templating

Tired of using `.replace()` on strings? `zpl-forge` natively parses double-brace `{{variables}}` and evaluates them securely at render time without allocating new strings.

```rust
use std::collections::HashMap;
use zpl_forge::{ZplEngine, Unit, Resolution};
use zpl_forge::forge::png::PngBackend;

fn main() -> zpl_forge::ZplResult<()> {
    // 1. The Raw ZPL (acts as your template)
    let zpl_template = "^XA
        ^FO50,50^A0N,50,50^FDShip to: {{recipient}}^FS
        ^FO50,120^A0N,30,30^FDTracking: {{tracking_id}}^FS
        ^BY3,2,100^FO50,160^BC^FD{{tracking_id}}^FS
        ^XZ";

    // 2. Parse the AST and layout engine ONCE
    let engine = ZplEngine::new(zpl_template, Unit::Inches(4.0), Unit::Inches(2.0), Resolution::Dpi203)?;

    // 3. Inject variables dynamically
    let mut vars = HashMap::new();
    vars.insert("recipient".to_string(), "John Doe".to_string());
    vars.insert("tracking_id".to_string(), "1Z9999999999999999".to_string());

    // 4. Render to PNG
    let png_bytes = engine.render(PngBackend::new(), &vars)?;
    std::fs::write("label.png", png_bytes).ok();

    Ok(())
}
```

## Supported ZPL Commands

The following commands are currently implemented and operational:

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

You can load and use your own TrueType (`.ttf`) or OpenType (`.otf`) fonts by registering them with the `FontManager`.

_Note: In ZPL, the `^A` command does not inherently support applying **bold** or **italic** modifiers. To use those font weights, you must map the respective font files to separate, unique identifiers._

```rust
use std::sync::Arc;
use std::collections::HashMap;
use zpl_forge::{ZplEngine, FontManager, Unit, Resolution};
use zpl_forge::forge::png::PngBackend;

fn main() -> zpl_forge::ZplResult<()> {
    let mut font_manager = FontManager::default();

    // 1. Load font bytes (Regular, Bold, Italic)
    let roboto_regular = std::fs::read("fonts/Roboto-Regular.ttf").unwrap();
    let roboto_bold = std::fs::read("fonts/Roboto-Bold.ttf").unwrap();
    let roboto_italic = std::fs::read("fonts/Roboto-Italic.ttf").unwrap();

    // 2. Register the fonts and map them to distinct ZPL identifiers ('A', 'B', 'C')
    font_manager.register_font("Roboto Regular", &roboto_regular, 'A', 'A')?;
    font_manager.register_font("Roboto Bold", &roboto_bold, 'B', 'B')?;
    font_manager.register_font("Roboto Italic", &roboto_italic, 'C', 'C')?;

    // 3. Render using specific mapped identifiers
    let zpl_input = "^XA
        ^FO50,50^AAN,50,50^FDThis is Regular^FS
        ^FO50,120^ABN,50,50^FDThis is Bold^FS
        ^FO50,190^ACN,50,50^FDThis is Italic^FS
        ^XZ";

    let mut engine = ZplEngine::new(zpl_input, Unit::Inches(4.0), Unit::Inches(4.0), Resolution::Dpi203)?;
    engine.set_fonts(Arc::new(font_manager));

    engine.render(PngBackend::new(), &HashMap::new())?;
    Ok(())
}
```

### Conditional Rendering (`^IFC`)

You can hide or show fields dynamically based on variables evaluated at runtime. This prevents you from having to string-manipulate the ZPL structure to remove a box.

```rust
fn main() {
    // If the variable "user_type" does not strictly equal "admin",
    // the first line will NOT be rendered.
    let zpl_input = "^XA
        ^FO50,50^IFCuser_type,admin^A0N,50,50^FDAdmin Only Area^FS
        ^FO50,150^A0N,50,50^FDPublic Text^FS
        ^XZ";

    let mut engine = ZplEngine::new(zpl_input, Unit::Inches(4.0), Unit::Inches(2.0), Resolution::Dpi203).unwrap();

    let mut vars = HashMap::new();
    vars.insert("user_type".to_string(), "guest".to_string()); // Condition fails

    // Only "Public Text" will be generated in the output
    let png_bytes = engine.render(PngBackend::new(), &vars).unwrap();
}
```

### Multi-Page PDF Batching

You can render hundreds of labels with different dynamic data into a single multi-page PDF document using `png_merge_pages_to_pdf`. Parse the ZPL template once, render each page as a PNG in parallel, and multiplex them efficiently with configurable compression levels.

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

    // 1. Render each page individually as PNG
    let mut pages: Vec<Vec<u8>> = Vec::new();
    for i in 0..100 {
        let mut vars = HashMap::new();
        vars.insert("order_id".to_string(), format!("ORD-{}", 1001 + i));
        pages.push(engine.render(PngBackend::new(), &vars)?);
    }

    // 2. Merge all PNGs into a single multi-page PDF
    let w = width.to_dots(resolution) as f64;
    let h = height.to_dots(resolution) as f64;
    // Available: Compression::fast(), Compression::default(), Compression::best()
    let pdf_bytes = png_merge_pages_to_pdf(&pages, w, h, resolution.dpi(), Compression::default())?;

    std::fs::write("labels.pdf", pdf_bytes).ok();
    Ok(())
}
```

## Security and Limits

To ensure stability and prevent Denial of Service (DoS) attacks via memory exhaustion, `zpl-forge` implements the following restrictions:

- **Canvas Size**: Rendering is limited to a maximum of **8192 x 8192 pixels**.
- **ZPL Images (`^GF`)**: Decoded image data cannot exceed **10 MB** per command.
- **Safe Arithmetic**: Saturating arithmetic is used for all coordinate and dimension calculations, preventing integer overflows.
- **Unit Validation**: Input values for physical dimensions (inches, mm, cm) are normalized to prevent negative values.

## License

This project is licensed under either the MIT or Apache-2.0 license.
