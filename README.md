# ZPL-Forge

[![Crates.io](https://img.shields.io/crates/v/zpl_forge.svg)](https://crates.io/crates/zpl_forge)
[![Docs.rs](https://docs.rs/zpl-forge/badge.svg)](https://docs.rs/zpl-forge)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/rafael-arreola/zpl-forge#license)

Check out the [examples documentation](https://github.com/rafael-arreola/zpl-forge/blob/main/examples/EXAMPLES.md) for ready-to-run code samples and their generated output images. You can run the examples locally with:

```sh
cargo run --example zpl_showcase        # Render all label showcases
cargo run --example multi_page_pdf      # Render 100 labels into a single PDF
cargo run --example custom_fonts        # Render using custom TTF fonts (requires downloading fonts first)
```

`zpl-forge` is a high-performance engine written in Rust for parsing, processing, and rendering Zebra Programming Language (ZPL) labels. The project transforms raw ZPL strings into an optimized Intermediate Representation (IR), enabling export to various formats such as PNG images or PDF documents.

## Key Features

- **AST-Based Architecture**: Uses `nom` for robust and efficient ZPL command parsing.
- **State Machine Engine**: Converts the command stream into a list of self-contained instructions, managing the global label state (fonts, positions, etc.).
- **Flexible Backends**: Native support for rendering to PNG (via `imageproc`) and single or multi-page PDF (via `lopdf`).
- **Extensibility**: Custom commands for color support, external image loading, and logic rendering.
- **Performance**: Designed to minimize allocations (Zero-allocation templating) and remain safe in concurrent environments.

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

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
zpl-forge = "0.1"
```

## Usage

### Render to PNG

```rust
use std::collections::HashMap;
use zpl_forge::{ZplEngine, Unit, Resolution};
use zpl_forge::forge::png::PngBackend;

fn main() {
    let zpl_input = "^XA^FO50,50^A0N,50,50^FDZPL Forge^FS^XZ";

    let engine = ZplEngine::new(
        zpl_input,
        Unit::Inches(4.0),
        Unit::Inches(2.0),
        Resolution::Dpi203
    ).expect("Error parsing ZPL");

    let png_backend = PngBackend::new();
    let png_bytes = engine.render(png_backend, &HashMap::new())
        .expect("Error rendering");

    std::fs::write("output.png", png_bytes).ok();
}
```

### Render to PDF

```rust
use std::collections::HashMap;
use zpl_forge::{ZplEngine, Unit, Resolution};
use zpl_forge::forge::pdf::PdfBackend;

fn main() {
    let zpl_input = "^XA^FO50,50^A0N,50,50^FDZPL Forge^FS^XZ";

    let engine = ZplEngine::new(
        zpl_input,
        Unit::Inches(4.0),
        Unit::Inches(2.0),
        Resolution::Dpi203
    ).expect("Error parsing ZPL");

    let pdf_backend = PdfBackend::new();
    let pdf_bytes = engine.render(pdf_backend, &HashMap::new())
        .expect("Error rendering");

    std::fs::write("output.pdf", pdf_bytes).ok();
}
```

### Using Custom Fonts and Styles

You can load and use your own TrueType (`.ttf`) or OpenType (`.otf`) fonts by registering them with the `FontManager` before rendering.

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

    let mut engine = ZplEngine::new(
        zpl_input,
        Unit::Inches(4.0),
        Unit::Inches(4.0),
        Resolution::Dpi203
    )?;

    engine.set_fonts(Arc::new(font_manager));
    engine.render(PngBackend::new(), &HashMap::new())?;

    Ok(())
}
```

### Conditional Rendering (`^IFC`)

You can hide or show fields dynamically based on variables passed to the render engine. The condition expires automatically at the next `^FS` command.

```rust
use std::collections::HashMap;
use zpl_forge::{ZplEngine, Unit, Resolution};
use zpl_forge::forge::png::PngBackend;

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

    let png_backend = PngBackend::new();
    // Only "Public Text" will be generated in the output
    let png_bytes = engine.render(png_backend, &vars).unwrap();
}
```

### Multi-Page PDF

You can render multiple labels with different data into a single multi-page PDF document using `merge_pages_to_pdf`. Parse the ZPL template once, render each page as a PNG, and merge them all at the end.

```rust
use std::collections::HashMap;
use zpl_forge::{ZplEngine, Unit, Resolution};
use zpl_forge::forge::png::PngBackend;
use zpl_forge::forge::pdf::merge_pages_to_pdf;

fn main() -> zpl_forge::ZplResult<()> {
    let zpl_template = "^XA
        ^CF0,30
        ^FO50,50^FDOrder: {{order_id}}^FS
        ^FO50,100^FDRecipient: {{name}}^FS
        ^BY3,2,100
        ^FO100,160^BC^FD{{order_id}}^FS
        ^XZ";

    let width = Unit::Inches(4.0);
    let height = Unit::Inches(3.0);
    let resolution = Resolution::Dpi203;

    let engine = ZplEngine::new(zpl_template, width, height, resolution)?;

    // Render each page individually as PNG
    let mut pages: Vec<Vec<u8>> = Vec::new();
    for i in 0..100 {
        let mut vars = HashMap::new();
        vars.insert("order_id".to_string(), format!("ORD-{}", 1001 + i));
        vars.insert("name".to_string(), format!("Customer {}", i + 1));

        pages.push(engine.render(PngBackend::new(), &vars)?);
    }

    // Merge all PNGs into a single multi-page PDF
    let w = width.to_dots(resolution) as f64;
    let h = height.to_dots(resolution) as f64;
    let pdf_bytes = merge_pages_to_pdf(&pages, w, h, resolution.dpi())?;

    std::fs::write("labels.pdf", pdf_bytes).ok();
    Ok(())
}
```

> A full working example with 100 pages and benchmarks is available at [`examples/multi_page_pdf.rs`](https://github.com/rafael-arreola/zpl-forge/blob/main/examples/multi_page_pdf.rs).
> Run it with: `cargo run --example multi_page_pdf`

## Security and Limits

To ensure stability and prevent Denial of Service (DoS) attacks via memory exhaustion, `zpl-forge` implements the following restrictions:

- **Canvas Size**: Rendering is limited to a maximum of **8192 x 8192 pixels**.
- **ZPL Images (`^GF`)**: Decoded image data cannot exceed **10 MB** per command.
- **Safe Arithmetic**: Saturating arithmetic is used for all coordinate and dimension calculations, preventing integer overflows.
- **Unit Validation**: Input values for physical dimensions (inches, mm, cm) are normalized to prevent negative values.

## License

This project is licensed under either the MIT or Apache-2.0 license.
