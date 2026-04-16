use std::collections::HashMap;

use crate::{ZplError, ZplResult};
use ab_glyph::FontArc;

/// Default fallback font bytes embedded in the binary.
/// This guarantees the library runs on any OS/Platform without C dependencies.
const DEFAULT_FONT_BYTES: &[u8] = include_bytes!("../assets/OpenSans-Regular.ttf");

/// List of valid ZPL font identifiers (A-Z and 0-9).
const FONT_MAP: &[char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
];

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
}

impl Default for FontManager {
    /// Creates a `FontManager` with a lightweight open-source default font
    /// registered for all identifiers ('A' to '9').
    ///
    /// Uses Open Sans (SIL Open Font License) embedded directly in the binary,
    /// ensuring zero native dependencies on the host OS.
    fn default() -> Self {
        let mut current = Self {
            font_map: HashMap::new(),
            font_index: HashMap::new(),
        };

        // Register the embedded font for all alphanumeric ZPL identifiers
        let _ = current.register_font("OpenSans", DEFAULT_FONT_BYTES, 'A', '9');

        current
    }
}

impl FontManager {
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
    /// // let font_bytes = std::fs::read("fonts/Roboto-Regular.ttf")?;
    ///
    /// // Register it for a range of ZPL identifiers (e.g., from 'A' to 'Z')
    /// // font_manager.register_font("Roboto", &font_bytes, 'A', 'Z')?;
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
        self.font_index.insert(name.to_string(), font);
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
