#!/usr/bin/env python3
"""Calibration harness: compares zpl-forge output against the Labelary reference.

Usage:
    cargo run --release --example zpl_showcase   # regenerates examples/test_01.png
    python3 tools/calibrate.py

Measures ink bounding boxes of known text blocks in examples/test_01.png
(ours) vs examples/reference.png (Labelary reference, 812x1218 @ 203dpi)
and reports per-block width/height ratios. Blocks whose ratio deviates
more than TOLERANCE from 1.0 are flagged.

To regenerate the reference, POST the render_01 ZPL from
examples/zpl_showcase.rs to the Labelary API:
http://api.labelary.com/v1/printers/8dpmm/labels/4x6/0/ (Accept: image/png).

Width ratios for scalable-font (^CF0) blocks depend on the typeface itself
and are only informative until a metrically-matched font is embedded
(planned as a separate phase); height and position are font-independent.
"""

import sys

import numpy as np
from PIL import Image

TOLERANCE = 0.03
INK_THRESHOLD = 128  # >50% dark counts as ink (reference render is bilevel)

# (name, x0, x1, y0, y1, check_w, check_h) — crops isolate a single text run
# and exclude neighbouring lines, box borders and barcode bars.
# check_w=False: width driven by typeface choice (font selection phase).
# check_h=False: ink height includes descenders, whose depth is a typeface
#                metric (cap height geometry is covered by cap-only blocks).
REGIONS = [
    ("title 'I' cap  ^CF0,60 @50",    225, 245,  30, 112, False, True),
    ("title full     ^CF0,60 @50",    225, 700,  30, 112, False, False),
    ("addr line 1    ^CF0,30 @115",   220, 560, 110, 150, False, False),
    ("'John Doe'     ^CFA,30 @300",    50, 200, 292, 336, True, True),
    ("'100 Main St..'^CFA,30 @340",    50, 420, 336, 378, True, True),
    ("'Permit'       ^CFA,15 @340",   610, 745, 330, 362, True, True),
    ("'123456'       ^CFA,15 @390",   610, 745, 380, 412, True, True),
    ("interp digits  ^BY5 barcode",    60, 760, 822, 898, False, True),
    ("'Ctr. X34B-1'  ^CF0,40 @960",    95, 390, 946, 1000, False, True),
    ("'CA'           ^CF0,190 @955",  408, 744, 908, 1145, False, True),
]


def ink_bbox(img: np.ndarray, x0: int, x1: int, y0: int, y1: int):
    """Bounding box of dark pixels inside a crop, in full-image coordinates."""
    region = img[y0:y1, x0:x1] < INK_THRESHOLD
    rows = np.where(region.any(axis=1))[0]
    cols = np.where(region.any(axis=0))[0]
    if len(rows) == 0:
        return None
    return (
        cols.min() + x0, cols.max() + x0,  # x min/max
        rows.min() + y0, rows.max() + y0,  # y min/max
    )


def main() -> int:
    ref = np.array(Image.open("examples/reference.png").convert("L"))
    out = np.array(Image.open("examples/test_01.png").convert("L"))

    failures = 0
    header = (
        f"{'block':32} {'ref WxH @y':>16} {'ours WxH @y':>16} "
        f"{'w':>7} {'h':>7} {'dy':>4}  status"
    )
    print(header)
    print("-" * len(header))

    for name, x0, x1, y0, y1, check_w, check_h in REGIONS:
        rb = ink_bbox(ref, x0, x1, y0, y1)
        ob = ink_bbox(out, x0, x1, y0, y1)
        if rb is None or ob is None:
            print(f"{name:32} MISSING ink (ref={rb}, ours={ob})")
            failures += 1
            continue

        rw, rh = rb[1] - rb[0] + 1, rb[3] - rb[2] + 1
        ow, oh = ob[1] - ob[0] + 1, ob[3] - ob[2] + 1
        wr, hr = ow / rw, oh / rh
        dy = ob[2] - rb[2]  # vertical offset of ink top vs reference

        # 1px slack absorbs bilevel-vs-antialiased edge quantization noise.
        # Saira Condensed's or Inter's ascenders/descenders specific to some letters (like 'I')
        # can extend slightly above/below the cap-height by design (up to 3px).
        def close(ours, refv):
            return abs(ours - refv) <= max(1, refv * TOLERANCE)

        h_ok = (not check_h) or abs(oh - rh) <= max(2, rh * TOLERANCE)
        w_ok = (not check_w) or close(ow, rw)
        pos_tolerance = max(3, rh * TOLERANCE)
        pos_ok = abs(dy) <= pos_tolerance
        ok = h_ok and w_ok and pos_ok
        if not ok:
            failures += 1

        w_mark = f"{wr:6.3f}" + (" " if check_w else "*")
        h_mark = f"{hr:6.3f}" + (" " if check_h else "*")
        status = "ok" if ok else "FAIL" + (
            ("" if h_ok else " h") + ("" if w_ok else " w") + ("" if pos_ok else " y")
        )
        print(
            f"{name:32} {rw:>5}x{rh:<4}@{rb[2]:<5} {ow:>5}x{oh:<4}@{ob[2]:<5} "
            f"{w_mark} {h_mark} {dy:>4}  {status}"
        )

    print("-" * len(header))
    print("(*) informative only: depends on embedded typeface (font phase)")
    print(f"tolerance ±{TOLERANCE:.0%} → {failures} failing block(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
