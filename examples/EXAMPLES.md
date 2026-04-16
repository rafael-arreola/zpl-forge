# ZPL-Forge Examples

This directory contains code examples and test outputs demonstrating the rendering capabilities of `zpl-forge`.

## Custom Fonts

Demonstrates how to register and use multiple TrueType (`.ttf`) or OpenType (`.otf`) fonts using the `FontManager`.

- **Source:** [`custom_fonts.rs`](https://github.com/rafael-arreola/zpl-forge/blob/main/examples/custom_fonts.rs)

![Custom Fonts Output](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/custom_fonts_output.png)

---

## Integration Tests / Feature Showcases

The following outputs are generated automatically by our showcase example ([`zpl_showcase.rs`](https://github.com/rafael-arreola/zpl-forge/blob/main/examples/zpl_showcase.rs)).

### Label 01 (Complex Form)

A complete shipping label demonstrating text, graphic boxes (lines and rectangles), and barcodes.

- **Source:** [`zpl_showcase.rs#L53`](https://github.com/rafael-arreola/zpl-forge/blob/main/examples/zpl_showcase.rs#L53)
- **PDF Output:** [`test_01.pdf`](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/test_01.pdf)

![Test 01](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_01.png)

### Label 02 (Routing / Dispatch)

Demonstrates exact coordinate positioning with custom lines and alphanumeric fields.

- **Source:** [`zpl_showcase.rs#L114`](https://github.com/rafael-arreola/zpl-forge/blob/main/examples/zpl_showcase.rs#L114)

![Test 02](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_02.png)

### Label 03

Additional test output.

- **Source:** [`zpl_showcase.rs#L314`](https://github.com/rafael-arreola/zpl-forge/blob/main/examples/zpl_showcase.rs#L314)

![Test 03](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_03.png)

### Label 04

Additional test output.

- **Source:** [`zpl_showcase.rs#L382`](https://github.com/rafael-arreola/zpl-forge/blob/main/examples/zpl_showcase.rs#L382)

![Test 04](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_04.png)

### Bitmap Images (`^GF`)

Demonstrates decoding and rendering of ZPL bitmap image fields.

- **Source:** [`zpl_showcase.rs#L212`](https://github.com/rafael-arreola/zpl-forge/blob/main/examples/zpl_showcase.rs#L212)

![Test Image](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_image.png)
![Test Image 2](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_image2.png)

### Custom Colors (`^GIC`, `^GLC`, `^GTC`)

Demonstrates rendering logic using custom external colored images, custom line colors, and text colors.

- **Source:** [`zpl_showcase.rs#L250`](https://github.com/rafael-arreola/zpl-forge/blob/main/examples/zpl_showcase.rs#L250)

![Color Test 1](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_image_color.png)
![Color Test 2](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_image_color2.png)
![Color Test 3](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_image_color3.png)
![Color Test 4](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_image_color4.png)

### Conditional Rendering (`^IFC`)

Demonstrates the `^IFC` (If Condition Custom) command, which selectively renders objects based on variables evaluated at runtime.

- **Source:** [`zpl_showcase.rs#L414`](https://github.com/rafael-arreola/zpl-forge/blob/main/examples/zpl_showcase.rs#L414)

**Condition Met (Both elements shown):**
![Conditional Rendering True](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_ifc_true.png)

**Condition Failed (Admin elements hidden):**
![Conditional Rendering False](https://raw.githubusercontent.com/rafael-arreola/zpl-forge/main/examples/test_ifc_false.png)

---

## Multi-Page PDF

Demonstrates rendering 100 different shipping labels (each with unique data) into a single multi-page PDF document using `merge_pages_to_pdf`. The ZPL template is parsed once, then rendered 100 times with different variables, and finally merged into one PDF.

- **Source:** [`multi_page_pdf.rs`](https://github.com/rafael-arreola/zpl-forge/blob/main/examples/multi_page_pdf.rs)
- **PDF Output:** [`multi_page_labels.pdf`](https://github.com/rafael-arreola/zpl-forge/raw/main/examples/multi_page_labels.pdf)

Run it with:

```
cargo run --example multi_page_pdf
```
