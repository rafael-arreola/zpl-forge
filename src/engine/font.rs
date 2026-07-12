use std::collections::HashMap;

use crate::{ZplError, ZplResult};
use ab_glyph::{Font, FontArc, PxScale, ScaleFont};

/// Default fallback font bytes embedded in the binary.
/// This guarantees the library runs on any OS/Platform without C dependencies.
const DEFAULT_FONT_BYTES: &[u8] = include_bytes!("../assets/IosevkaTermSlab-Regular.ttf");

/// List of valid ZPL font identifiers (A-Z and 0-9).
const FONT_MAP: &[char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
];

/// Zebra scalable fonts (e.g. `^A0`): capital letters span ~75% of the `^A`
/// height with the cap top sitting exactly on the field origin. Calibrated
/// against Labelary renders across `^A` sizes 30-190.
const SCALABLE_CAP_RATIO: f32 = 0.75;

/// Default `^A` height in dots when none was specified (ZPL font A default).
const DEFAULT_FONT_HEIGHT: u32 = 9;

/// Fallback cap height when a font has no 'H' outline, in em units.
const FALLBACK_CAP_RATIO: f32 = 0.7;

/// Geometry of a Zebra built-in bitmap font cell at 203 dpi, in dots.
///
/// `base_h`/`base_w` are the glyph matrix, `cell_w` is the horizontal advance
/// (matrix width + intercharacter gap) and `baseline` is the distance from the
/// cell top to the baseline, which equals the capital-letter height.
/// Values from the ZPL II programming guide font matrices.
struct BitmapCell {
    base_h: f32,
    base_w: f32,
    cell_w: f32,
    baseline: f32,
}

/// Zebra bitmap font matrices for identifiers A-H. Other identifiers are
/// treated as scalable fonts.
fn bitmap_cell(font_char: char) -> Option<BitmapCell> {
    let (base_h, base_w, cell_w, baseline) = match font_char {
        'A' => (9.0, 5.0, 6.0, 7.0),
        'B' => (11.0, 7.0, 9.0, 11.0),
        'C' | 'D' => (18.0, 10.0, 12.0, 14.0),
        'E' => (28.0, 15.0, 20.0, 23.0),
        'F' => (26.0, 13.0, 16.0, 21.0),
        'G' => (60.0, 40.0, 48.0, 48.0),
        'H' => (21.0, 13.0, 19.0, 21.0),
        _ => return None,
    };
    Some(BitmapCell {
        base_h,
        base_w,
        cell_w,
        baseline,
    })
}

/// Normalization metrics extracted once per registered font, in font units.
#[derive(Debug, Clone, Copy)]
struct FontMetrics {
    /// Units per em.
    units_per_em: f32,
    /// `ascent - descent` (what an `ab_glyph::PxScale` maps to its `y` value).
    height_unscaled: f32,
    /// Capital-letter height, measured from the 'H' outline.
    cap_height: f32,
    /// Representative advance width (digit '0'), used to fit monospace cells.
    advance: f32,
}

impl FontMetrics {
    fn from_font(font: &FontArc) -> Self {
        let units_per_em = font.units_per_em().unwrap_or(1000.0);
        // Outline bounds come in screen-style order (min.y holds the glyph
        // top in font units), so take the larger of the two y values.
        let cap_height = font
            .outline(font.glyph_id('H'))
            .map(|o| o.bounds.min.y.max(o.bounds.max.y))
            .filter(|&v| v > 0.0)
            .unwrap_or(units_per_em * FALLBACK_CAP_RATIO);
        let advance = font.h_advance_unscaled(font.glyph_id('0'));
        let advance = if advance > 0.0 {
            advance
        } else {
            units_per_em * 0.5
        };
        Self {
            units_per_em,
            height_unscaled: font.height_unscaled(),
            cap_height,
            advance,
        }
    }
}

/// Resolved scaling for one ZPL text field, in dots.
///
/// Both backends must consume this instead of building an `ab_glyph::PxScale`
/// from the raw `^A` parameters: `PxScale` maps `ascent - descent` (not the
/// em) to its `y` value, which shrinks glyphs and misplaces the baseline.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TextLayout {
    /// Rasterization scale for `ab_glyph`/`imageproc`.
    pub px: PxScale,
    /// Distance from the top of the ZPL character cell to the baseline.
    /// Capital letters render from the cell top down to this line.
    pub baseline: f32,
    /// Em size in dots, horizontal (vector backends: PDF text matrix).
    pub em_x: f32,
    /// Em size in dots, vertical.
    pub em_y: f32,
    /// Total character-cell height in dots (rotation anchors, reverse boxes).
    pub cell_h: f32,
}

/// Interpretation-line geometry for 1-D barcodes: `(font_height, gap)` in
/// dots, both proportional to the module width as rendered by Labelary
/// (digit ink measured 36 dots tall, 6 dots below the bars, at `^BY5`).
pub(crate) fn interpretation_metrics(module_width: u32) -> (u32, u32) {
    let module = module_width.max(1) as f32;
    let text_h = (module * 36.0 / 5.0 / SCALABLE_CAP_RATIO).round() as u32;
    let gap = ((module * 1.2).round() as u32).max(1);
    (text_h, gap)
}

/// Manages fonts and their mapping to ZPL font identifiers.
///
/// This structure tracks registered fonts and maps them to the single-character
/// identifiers used in ZPL commands (e.g., '^A0', '^AA').
#[derive(Debug, Clone)]
pub struct FontManager {
    /// Maps ZPL font identifiers (as Strings) to internal font names.
    font_map: HashMap<String, String>,
    /// Stores the actual font data indexed by internal font names.
    font_index: HashMap<String, FontArc>,
    /// Stores the raw TTF/OTF bytes indexed by internal font names.
    font_bytes: HashMap<String, Vec<u8>>,
    /// Normalization metrics per internal font name, computed at registration.
    font_metrics: HashMap<String, FontMetrics>,
}

impl Default for FontManager {
    /// Creates a `FontManager` with a lightweight open-source default font
    /// registered for all identifiers ('A' to '9').
    ///
    /// Uses Iosevka Term Slab (SIL Open Font License, Version 1.1)
    /// embedded directly in the binary, ensuring zero native dependencies on the host OS.
    fn default() -> Self {
        let mut current = Self {
            font_map: HashMap::new(),
            font_index: HashMap::new(),
            font_bytes: HashMap::new(),
            font_metrics: HashMap::new(),
        };

        // Register the embedded font for all alphanumeric ZPL identifiers
        let _ = current.register_font("Iosevka Term Slab", DEFAULT_FONT_BYTES, 'A', '9');

        current
    }
}

impl FontManager {
    /// Retrieves the raw TTF/OTF bytes for a font by its ZPL identifier.
    ///
    /// This is used by backends that need the raw font data (e.g., PDF embedding).
    pub fn get_font_bytes(&self, name: &str) -> Option<&[u8]> {
        let font_name = self.font_map.get(name)?;
        self.font_bytes.get(font_name).map(|v| v.as_slice())
    }

    /// Returns the internal font name mapped to a ZPL identifier.
    pub fn get_font_name(&self, name: &str) -> Option<&str> {
        self.font_map.get(name).map(|s| s.as_str())
    }

    /// Retrieves a font by its ZPL identifier.
    ///
    /// # Arguments
    /// * `name` - The ZPL font identifier (e.g., "0", "A").
    pub fn get_font(&self, name: &str) -> Option<&FontArc> {
        let font_name = self.font_map.get(name);
        if let Some(font_name) = font_name {
            self.font_index.get(font_name)
        } else {
            None
        }
    }

    /// Resolves a ZPL font identifier (falling back to font '0') and computes
    /// the Zebra-calibrated [`TextLayout`] for the given `^A` height/width.
    ///
    /// Bitmap identifiers (A-H) use integer cell magnification like real
    /// printers; every other identifier uses the scalable-font model.
    pub(crate) fn text_layout(
        &self,
        font_char: char,
        height: Option<u32>,
        width: Option<u32>,
    ) -> Option<(&FontArc, TextLayout)> {
        let mut buf = [0; 4];
        let key = font_char.encode_utf8(&mut buf);
        let name = self.font_map.get(key).or_else(|| self.font_map.get("0"))?;
        let font = self.font_index.get(name)?;
        let metrics = self
            .font_metrics
            .get(name)
            .copied()
            .unwrap_or_else(|| FontMetrics::from_font(font));

        let h = height.unwrap_or(DEFAULT_FONT_HEIGHT).max(1) as f32;

        let (em_x, em_y, baseline, cell_h) = if let Some(cell) = bitmap_cell(font_char) {
            // Bitmap fonts magnify a fixed dot matrix by integer factors.
            let mag_h = (h / cell.base_h).round().max(1.0);
            let mag_w = match width {
                Some(w) if w > 0 => (w as f32 / cell.base_w).round().max(1.0),
                _ => mag_h,
            };
            let cap_px = cell.baseline * mag_h;
            let advance_px = cell.cell_w * mag_w;
            let em_y = cap_px * metrics.units_per_em / metrics.cap_height;
            let em_x = advance_px * metrics.units_per_em / metrics.advance;
            (em_x, em_y, cap_px, cell.base_h * mag_h)
        } else {
            // Scalable fonts: caps span SCALABLE_CAP_RATIO of the ^A height.
            let cap_px = SCALABLE_CAP_RATIO * h;
            let em_y = cap_px * metrics.units_per_em / metrics.cap_height;
            let em_x = match width {
                Some(w) if w > 0 => em_y * w as f32 / h,
                _ => em_y,
            };
            (em_x, em_y, cap_px, h)
        };

        // ab_glyph's PxScale maps (ascent - descent) to its value, so convert
        // the em sizes through the font's own vertical extent.
        let px = PxScale {
            x: em_x * metrics.height_unscaled / metrics.units_per_em,
            y: em_y * metrics.height_unscaled / metrics.units_per_em,
        };

        Some((
            font,
            TextLayout {
                px,
                baseline,
                em_x,
                em_y,
                cell_h,
            },
        ))
    }

    /// Measures the advance width of `text` in dots for the given `^A` spec.
    /// Single source of truth for every backend and for `^FB` wrapping.
    pub(crate) fn measure_text(
        &self,
        font_char: char,
        height: Option<u32>,
        width: Option<u32>,
        text: &str,
    ) -> u32 {
        let Some((font, layout)) = self.text_layout(font_char, height, width) else {
            return 0;
        };
        let scaled = font.as_scaled(layout.px);
        let mut w = 0.0_f32;
        let mut last = None;
        for c in text.chars() {
            let gid = font.glyph_id(c);
            if let Some(prev) = last {
                w += scaled.kern(prev, gid);
            }
            w += scaled.h_advance(gid);
            last = Some(gid);
        }
        w.ceil() as u32
    }

    /// Registers a new font and maps it to a range of ZPL identifiers.
    ///
    /// Custom fonts must be in TrueType (`.ttf`) or OpenType (`.otf`) format.
    /// Once registered, the font can be used in ZPL commands like `^A` or `^CF`
    /// by referencing the assigned identifiers.
    ///
    /// # Arguments
    /// * `name` - An internal name for the font.
    /// * `bytes` - The raw TrueType/OpenType font data.
    /// * `from` - The starting ZPL identifier in the range (A-Z, 0-9).
    /// * `to` - The ending ZPL identifier in the range (A-Z, 0-9).
    ///
    /// # Errors
    /// Returns an error if the font data is invalid.
    ///
    /// # Example
    ///
    /// ```rust
    /// use zpl_forge::FontManager;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut font_manager = FontManager::default();
    ///
    /// // Load your font file bytes
    /// // let font_bytes = std::fs::read("fonts/IosevkaTermSlab-Regular.ttf")?;
    ///
    /// // Register it for a range of ZPL identifiers (e.g., from 'A' to 'Z')
    /// // font_manager.register_font("Iosevka Term Slab", &font_bytes, 'A', 'Z')?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn register_font(
        &mut self,
        name: &str,
        bytes: &[u8],
        from: char,
        to: char,
    ) -> ZplResult<()> {
        let font = FontArc::try_from_vec(bytes.to_vec())
            .map_err(|_| ZplError::FontError("Invalid font data".into()))?;
        self.font_metrics
            .insert(name.to_string(), FontMetrics::from_font(&font));
        self.font_index.insert(name.to_string(), font);
        self.font_bytes.insert(name.to_string(), bytes.to_vec());
        self.assign_font(name, from, to);
        Ok(())
    }

    /// Internal helper to assign a registered font to a range of ZPL identifiers.
    fn assign_font(&mut self, name: &str, from: char, to: char) {
        let from_idx = FONT_MAP.iter().position(|&x| x == from);
        let to_idx = FONT_MAP.iter().position(|&x| x == to);

        if from_idx.is_none() || to_idx.is_none() {
            return;
        }

        if let (Some(start), Some(end)) = (from_idx, to_idx)
            && start <= end
        {
            for key in &FONT_MAP[start..=end] {
                self.font_map.insert(key.to_string(), name.to_string());
            }
        }
    }
}
