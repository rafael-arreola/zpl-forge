# ZPL-Forge

[![Crates.io](https://img.shields.io/crates/v/zpl-forge.svg)](https://crates.io/crates/zpl-forge)
[![Docs.rs](https://docs.rs/zpl-forge/badge.svg)](https://docs.rs/zpl-forge)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/rafael-arreola/zpl-forge#license)

[English] | [Español](docs/README_ES.md)

`zpl-forge` is a high-performance engine written in Rust for parsing, processing, and rendering Zebra Programming Language (ZPL) labels. The project transforms raw ZPL strings into an optimized Intermediate Representation (IR), enabling export to various formats such as PNG images or PDF documents.

## Key Features

- **AST-Based Architecture**: Uses `nom` for robust and efficient ZPL command parsing.
- **State Machine Engine**: Converts the command stream into a list of self-contained instructions, managing the global label state (fonts, positions, etc.).
- **Flexible Backends**: Native support for rendering to PNG (via `imageproc`) and PDF (via `printpdf`).
- **Extensibility**: Custom commands for color support and external image loading.
- **Performance**: Designed to minimize allocations and remain safe in concurrent environments.

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

| Command | Name         | Parameters | Description                                                                                      |
| :------ | :----------- | :--------- | :----------------------------------------------------------------------------------------------- |
| `^GIC`  | Custom Image | `w,h,d`    | Renders a color image. **w** and **h** define size. **d** is the binary (PNG/JPG) in **Base64**. |
| `^GLC`  | Line Color   | `c`        | Sets the color for graphic elements in hexadecimal format (e.g., `#FF0000`).                     |
| `^GTC`  | Text Color   | `c`        | Sets the color for text fields in hexadecimal format (e.g., `#0000FF`).                          |

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
zpl-forge = "0.1.0"
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

## Security and Limits

To ensure stability and prevent Denial of Service (DoS) attacks via memory exhaustion, `zpl-forge` implements the following restrictions:

- **Canvas Size**: Rendering is limited to a maximum of **8192 x 8192 pixels**.
- **ZPL Images (`^GF`)**: Decoded image data cannot exceed **10 MB** per command.
- **Safe Arithmetic**: Saturating arithmetic is used for all coordinate and dimension calculations, preventing integer overflows.
- **Unit Validation**: Input values for physical dimensions (inches, mm, cm) are normalized to prevent negative values.

## License

This project is licensed under either the MIT or Apache-2.0 license.
