# ZPL-Forge Examples

This directory contains code examples and test outputs demonstrating the rendering capabilities of `zpl-forge` across its three backends.

## Rendering Backends

Every label in the showcase is rendered with all three backends for comparison:

| Suffix        | Backend              | Description                       |
| :------------ | :------------------- | :-------------------------------- |
| `.png`        | **PngBackend**       | Raster image (RGB canvas)         |
| `.pdf`        | **PdfBackend**       | PDF with embedded raster image    |
| `_native.pdf` | **PdfNativeBackend** | PDF with native vector operations |

---

## Benchmark Summary

All times measured on Apple M-series silicon via `cargo run --example zpl_showcase`.

| Label                               | PngBackend | PdfBackend | PdfNativeBackend |
| :---------------------------------- | :--------: | :--------: | :--------------: |
| Shipping Label (test_01)            |   8.1 ms   |  21.8 ms   |    **4.4 ms**    |
| Route Label (test_02)               |   1.0 ms   |   2.3 ms   |      4.2 ms      |
| Color Label (test_03)               |   6.7 ms   |  19.8 ms   |    **4.5 ms**    |
| Barcode Label (test_04)             |   7.9 ms   |  23.6 ms   |    **4.8 ms**    |
| Bitmap Image (test_image)           |   5.1 ms   |  15.2 ms   |      7.6 ms      |
| Full-Color Image (test_image_color) |  14.2 ms   |  57.9 ms   |     43.2 ms      |

---

## Custom Fonts

Demonstrates how to register and use multiple TrueType (`.ttf`) or OpenType (`.otf`) fonts using the `FontManager`.

- **Source:** [`custom_fonts.rs`](https://github.com/rafael-arreola/zpl-forge/blob/main/examples/custom_fonts.rs)

![Custom Fonts Output](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/custom_fonts_output.png)

---

## Integration Tests / Feature Showcases

The following outputs are generated automatically by the showcase example ([`zpl_showcase.rs`](https://github.com/rafael-arreola/zpl-forge/blob/main/examples/zpl_showcase.rs)).

### Label 01 — Shipping Label

A complete shipping label demonstrating text, graphic boxes (lines and rectangles), barcodes, and reverse print.

- **Source:** [`zpl_showcase.rs`](https://github.com/rafael-arreola/zpl-forge/blob/main/examples/zpl_showcase.rs)
- **Outputs:** `test_01.png` · [`test_01.pdf`](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/test_01.pdf) · [`test_01_native.pdf`](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/test_01_native.pdf)

|                                         PngBackend (8.1 ms)                                          |                                         PdfNativeBackend (4.4 ms)                                         |
| :--------------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------: |
| ![Test 01 PNG](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_01.png) | Vector PDF — [download](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/test_01_native.pdf) |

### Label 02 — Routing / Dispatch

Demonstrates exact coordinate positioning with custom lines and alphanumeric fields.

- **Outputs:** `test_02.png` · [`test_02.pdf`](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/test_02.pdf) · [`test_02_native.pdf`](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/test_02_native.pdf)

![Test 02](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_02.png)

### Label 03 — Custom Colors (`^GLC`, `^GTC`)

Demonstrates colored lines and text fields using hexadecimal color extensions.

- **Outputs:** `test_03.png` · [`test_03.pdf`](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/test_03.pdf) · [`test_03_native.pdf`](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/test_03_native.pdf)

![Test 03](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_03.png)

### Label 04 — Barcodes (Code 128, Code 39, QR, Rotated)

Demonstrates all barcode types including rotated orientations.

- **Outputs:** `test_04.png` · [`test_04.pdf`](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/test_04.pdf) · [`test_04_native.pdf`](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/test_04_native.pdf)

![Test 04](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_04.png)

### Bitmap Images (`^GF`)

Demonstrates decoding and rendering of ZPL bitmap image fields.

- **Outputs:** `test_image.png` · `test_image.pdf` · `test_image_native.pdf`

![Test Image](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_image.png)
![Test Image 2](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_image2.png)

### Custom Color Images (`^GIC`)

Demonstrates rendering with custom external colored images in various size configurations.

![Color Test 1](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_image_color.png)
![Color Test 2](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_image_color2.png)
![Color Test 3](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_image_color3.png)
![Color Test 4](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_image_color4.png)

### Conditional Rendering (`^IFC`)

Demonstrates the `^IFC` (If Condition Custom) command, which selectively renders objects based on variables evaluated at runtime.

- **Outputs:** `test_ifc_true.png` · `test_ifc_true_native.pdf` · `test_ifc_false.png` · `test_ifc_false_native.pdf`

**Condition Met (Both elements shown):**
![Conditional True](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_ifc_true.png)

**Condition Failed (Admin elements hidden):**
![Conditional False](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_ifc_false.png)

---

## Multi-Page PDF (1,000 Labels)

Demonstrates rendering 1,000 different shipping labels into single multi-page PDF documents using `png_merge_pages_to_pdf`. The ZPL template is parsed once, rendered 1,000 times with different variables in parallel (via `rayon`), and merged into three PDFs — one per compression level.

| Compression | Merge Time | File Size |
| :---------- | :--------: | :-------: |
| `fast`      |   558 ms   |  36.4 MB  |
| `default`   |   1.03 s   |  24.6 MB  |
| `best`      |   2.07 s   |  21.0 MB  |

- **PDF Outputs:**
  - [`multi_page_labels_fast.pdf`](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/multi_page_labels_fast.pdf)
  - [`multi_page_labels_default.pdf`](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/multi_page_labels_default.pdf)
  - [`multi_page_labels_best.pdf`](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/multi_page_labels_best.pdf)

---

## Run All Examples

```
cargo run --example zpl_showcase
```

All output files are written to the `examples/` directory.
