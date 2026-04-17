# ZPL-Forge

A fast, memory-safe ZPL (Zebra Programming Language) parser and renderer for Rust. It converts ZPL code into PNG images or native, selectable PDFs.

[![Crates.io](https://img.shields.io/crates/v/zpl_forge.svg)](https://crates.io/crates/zpl_forge)
[![Docs.rs](https://docs.rs/zpl-forge/badge.svg)](https://docs.rs/zpl-forge)

## The Purpose

ZPL-Forge is intended to provide a quick and simple alternative for creating documents like **shipping guides, delivery receipts, and tickets**. It is optimized for use cases where speed and simplicity are preferred over extreme document detail.

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
- **Raster PDF:** ~21.1 ms

**Bulk PDF Generation (1000 labels):**
All 1000 pages render in **~400 ms** (0.4 ms/page).

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
zpl-forge = "0.2.1"
```

## Quick Start

### Render to PNG

```rust
use zpl_forge::{ZplEngine, PngBackend};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zpl = "^XA^FO50,50^A0N,50,50^FDHello World^FS^XZ";
    let mut engine = ZplEngine::new(zpl);
    let mut backend = PngBackend::new();

    engine.render(&mut backend, 4.0, 4.0, &[])?;
    backend.save("label.png")?;
    Ok(())
}
```

### Render to Native Vector PDF

```rust
use zpl_forge::{ZplEngine, PdfNativeBackend};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let zpl = "^XA^FO50,50^A0N,50,50^FDSelectable Text!^FS^XZ";
    let mut engine = ZplEngine::new(zpl);
    let mut backend = PdfNativeBackend::new();

    engine.render(&mut backend, 4.0, 4.0, &[])?;
    backend.save("label.pdf")?;
    Ok(())
}
```

### Zero-Allocation Templating

Dynamically inject variables into your ZPL without extra allocations.

```rust
let zpl = "^XA^FO50,50^A0N,50,50^FDHello {{NAME}}^FS^XZ";
let mut engine = ZplEngine::new(zpl);
engine.render(&mut backend, 4.0, 4.0, &[("NAME", "ZPL-Forge")])?;
```

## Advanced Usage

### Conditional Rendering (`^IFC`)

Render elements only if a variable matches a specific value.

```rust
let zpl = "^XA
    ^FO50,50^IFCuser_type,admin^A0N,50,50^FDAdmin Only Area^FS
    ^FO50,150^A0N,50,50^FDPublic Text^FS
    ^XZ";
// "Admin Only Area" will only show if ("user_type", "admin") is passed to render.
```

### Multi-Page PDF Batching

Reuse a `PdfNativeBackend` to bundle many labels into one file.

```rust
let mut backend = PdfNativeBackend::new();
for i in 1..=10 {
    let zpl = format!("^XA^FO50,50^A0N,50,50^FDPage {i}^FS^XZ");
    ZplEngine::new(&zpl).render(&mut backend, 4.0, 4.0, &[])?;
}
backend.save("batch.pdf")?;
```

## Supported ZPL Commands

| Command | Name           | Parameters    | Description                            |
| :------ | :------------- | :------------ | :------------------------------------- |
| `^A`    | Font Spec      | `f,o,h,w`     | Specifies font, orientation, and size. |
| `^B3`   | Code 39        | `o,e,h,f,g`   | Code 39 Barcode.                       |
| `^BC`   | Code 128       | `o,h,f,g,e,m` | Code 128 Barcode.                      |
| `^BQ`   | QR Code        | `o,m,s,e,k`   | QR Code (Model 1 or 2).                |
| `^BY`   | Barcode Def.   | `w,r,h`       | Sets default values for barcodes.      |
| `^CF`   | Def. Font      | `f,h,w`       | Changes the default font.              |
| `^FD`   | Field Data     | `d`           | Data to print in the current field.    |
| `^FO`   | Field Origin   | `x,y`         | Sets the top-left corner of the field. |
| `^FR`   | Field Reverse  | N/A           | Inverts field color (white on black).  |
| `^FS`   | Field Sep.     | N/A           | End of a field definition.             |
| `^GB`   | Graphic Box    | `w,h,t,c,r`   | Box, line, or rounded rectangle.       |
| `^GC`   | Graphic Circle | `d,t,c`       | Draws a circle.                        |
| `^GE`   | Graphic Ellip. | `w,h,t,c`     | Draws an ellipse.                      |
| `^GF`   | Graphic Field  | `c,b,f,p,d`   | Renders a bitmap image.                |
| `^XA`   | Start Format   | N/A           | Start of a label.                      |
| `^XZ`   | End Format     | N/A           | End of a label.                        |

## Custom Commands (Extensions)

| Command | Name         | Parameters | Description                              |
| :------ | :----------- | :--------- | :--------------------------------------- |
| `^GIC`  | Custom Image | `w,h,d`    | Renders a color PNG/JPG (Base64).        |
| `^GLC`  | Line Color   | `c`        | Sets HEX color for graphic elements.     |
| `^GTC`  | Text Color   | `c`        | Sets HEX color for text fields.          |
| `^IFC`  | Cond. Render | `var,val`  | Renders field only if variable == value. |

## Security and Limits

ZPL-Forge imposes limits to prevent memory exhaustion from malformed ZPL:

- **Maximum Document Size:** Bounded to prevent memory overflow.
- **Graphic Field Maximums:** Limits `^GF` memory allocation.
- **Maximum Text Size:** Prevents excessively large font sizes.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE).
