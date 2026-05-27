use fontdb::{Database, Family, Query, Stretch, Style, Weight};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Mutex;
use ttf_parser::{Face, GlyphId};

static TEXT_MEASURER: Lazy<Mutex<TextMeasurer>> = Lazy::new(|| Mutex::new(TextMeasurer::new()));

pub fn measure_text_width(text: &str, font_size: f32, font_family: &str) -> Option<f32> {
    if text.is_empty() || font_size <= 0.0 {
        return Some(0.0);
    }
    let mut guard = TEXT_MEASURER.lock().ok()?;
    guard.measure(text, font_size, font_family)
}

pub fn measure_text_width_with_weight(
    text: &str,
    font_size: f32,
    font_family: &str,
    font_weight: u16,
) -> Option<f32> {
    if text.is_empty() || font_size <= 0.0 {
        return Some(0.0);
    }
    let mut guard = TEXT_MEASURER.lock().ok()?;
    guard.measure_with_weight(text, font_size, font_family, Weight(font_weight))
}

/// Compute the rendered width of a text string in pixels, mirroring the
/// browser's `SVGTextContentElement.getComputedTextLength()` API.
///
/// Uses the same font measurement pipeline as the rest of the renderer.
/// Falls back to a per-character width estimate when exact metrics are
/// unavailable.
pub fn get_computed_text_length(text: &str, font_size: f32, font_family: &str) -> f32 {
    if text.is_empty() || font_size <= 0.0 {
        return 0.0;
    }
    measure_text_width(text, font_size, font_family)
        .unwrap_or_else(|| fallback_text_width(text, font_size))
}

/// Word-wrap `text` so that no line exceeds `max_width` pixels.
///
/// Uses [`get_computed_text_length`] for measurement.  Returns the
/// resulting lines; a single-word line that exceeds `max_width` is
/// kept intact (never broken mid-word).
pub fn wrap_text(text: &str, max_width: f32, font_size: f32, font_family: &str) -> Vec<String> {
    if get_computed_text_length(text, font_size, font_family) <= max_width {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current, word)
        };
        if get_computed_text_length(&candidate, font_size, font_family) > max_width
            && !current.is_empty()
        {
            lines.push(current);
            current = word.to_string();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Fast fallback: sum per-character width factors × font_size.
fn fallback_text_width(text: &str, font_size: f32) -> f32 {
    text.chars()
        .map(|c| {
            if is_wide_symbol_fallback(c) {
                1.0
            } else if c.is_ascii_uppercase() {
                0.75
            } else if c.is_ascii_lowercase() {
                0.55
            } else if c == ' ' {
                0.3
            } else {
                0.6
            }
        })
        .sum::<f32>()
        * font_size
}

fn missing_glyph_width(ch: char, font_size: f32) -> f32 {
    if is_wide_symbol_fallback(ch) {
        font_size
    } else {
        font_size * 0.56
    }
}

fn is_wide_symbol_fallback(ch: char) -> bool {
    matches!(
        ch,
        '←' | '↑' | '→' | '↓' | '↔' | '↕' | '⇐' | '⇑' | '⇒' | '⇓' | '⇔' | '⇕'
    )
}

pub fn average_char_width(font_family: &str, font_size: f32) -> Option<f32> {
    if font_size <= 0.0 {
        return None;
    }
    let sample = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let width = measure_text_width(sample, font_size, font_family)?;
    let count = sample.chars().count().max(1) as f32;
    Some(width / count)
}

#[derive(Debug, Clone)]
pub struct EmbeddedFontData {
    pub bytes: Vec<u8>,
    pub mime_type: &'static str,
    pub format_hint: &'static str,
}

pub fn embedded_font_data(font_family: &str) -> Option<EmbeddedFontData> {
    let mut guard = TEXT_MEASURER.lock().ok()?;
    guard.embedded_font_data(font_family)
}

struct TextMeasurer {
    db: Database,
    loaded_system_fonts: bool,
    cache: HashMap<String, Option<FontFace>>,
}

impl TextMeasurer {
    fn new() -> Self {
        let db = Database::new();
        Self {
            db,
            loaded_system_fonts: false,
            cache: HashMap::new(),
        }
    }

    fn measure(&mut self, text: &str, font_size: f32, font_family: &str) -> Option<f32> {
        self.measure_with_weight(text, font_size, font_family, Weight::NORMAL)
    }

    fn measure_with_weight(
        &mut self,
        text: &str,
        font_size: f32,
        font_family: &str,
        weight: Weight,
    ) -> Option<f32> {
        let family_key = cache_key(font_family, weight);
        let face = if self.cache.contains_key(&family_key) {
            self.cache
                .get_mut(&family_key)
                .and_then(|face| face.as_mut())
        } else {
            let face = self.load_face(font_family, weight);
            self.cache.insert(family_key.clone(), face);
            self.cache
                .get_mut(&family_key)
                .and_then(|face| face.as_mut())
        }?;
        let normalized = text.replace('\t', "    ");
        face.measure_width(&normalized, font_size)
    }

    fn load_face(&mut self, font_family: &str, weight: Weight) -> Option<FontFace> {
        let family_key = cache_key(font_family, weight);
        if let Some(face) = load_preferred_known_face(font_family, weight) {
            return Some(face);
        }
        if let Some(face) = load_cached_face(&family_key) {
            return Some(face);
        }
        #[derive(Clone, Copy)]
        enum FamilyToken {
            Generic(fontdb::Family<'static>),
            Name(usize),
        }

        let mut names: Vec<String> = Vec::new();
        let mut order: Vec<FamilyToken> = Vec::new();
        for part in font_family.split(',') {
            let raw = part.trim().trim_matches('"').trim_matches('\'');
            if raw.is_empty() {
                continue;
            }
            let lower = raw.to_ascii_lowercase();
            match lower.as_str() {
                "serif" => order.push(FamilyToken::Generic(Family::Serif)),
                "sans-serif" => order.push(FamilyToken::Generic(Family::SansSerif)),
                "monospace" => order.push(FamilyToken::Generic(Family::Monospace)),
                "cursive" => order.push(FamilyToken::Generic(Family::Cursive)),
                "fantasy" => order.push(FamilyToken::Generic(Family::Fantasy)),
                "system-ui" | "-apple-system" | "ui-sans-serif" => {
                    order.push(FamilyToken::Generic(Family::SansSerif))
                }
                "ui-monospace" => order.push(FamilyToken::Generic(Family::Monospace)),
                _ => {
                    let idx = names.len();
                    names.push(raw.to_string());
                    order.push(FamilyToken::Name(idx));
                }
            }
        }
        if order.is_empty() {
            order.push(FamilyToken::Generic(Family::SansSerif));
        }

        let mut families: Vec<Family<'_>> = Vec::with_capacity(order.len());
        for token in order {
            match token {
                FamilyToken::Generic(family) => families.push(family),
                FamilyToken::Name(idx) => families.push(Family::Name(names[idx].as_str())),
            }
        }

        if !self.loaded_system_fonts {
            self.db.load_system_fonts();
            self.db.load_fonts_dir("/System/Library/Fonts/Supplemental");
            self.db.load_fonts_dir("/Library/Fonts");
            #[cfg(target_os = "ios")]
            {
                self.db.load_fonts_dir("/System/Library/Fonts");
                self.db.load_fonts_dir("/System/Library/Fonts/Core");
            }
            self.loaded_system_fonts = true;
        }

        let query = Query {
            families: &families,
            weight,
            stretch: Stretch::Normal,
            style: Style::Normal,
        };
        let id = self.db.query(&query)?;
        let mut loaded: Option<FontFace> = None;
        self.db.with_face_data(id, |data, index| {
            let bytes = data.to_vec();
            if let Ok(face) = Face::parse(&bytes, index) {
                let units_per_em = face.units_per_em().max(1);
                if let Some((font_path, meta_path)) = cache_paths(&family_key)
                    && !font_path.exists()
                {
                    if let Some(parent) = font_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::write(&font_path, &bytes);
                    let _ = fs::write(&meta_path, index.to_string());
                }
                loaded = Some(FontFace::new(bytes, index, units_per_em));
            }
        });
        loaded
    }

    fn embedded_font_data(&mut self, font_family: &str) -> Option<EmbeddedFontData> {
        let family_key = cache_key(font_family, Weight::NORMAL);
        if !self.cache.contains_key(&family_key) {
            let face = self.load_face(font_family, Weight::NORMAL);
            self.cache.insert(family_key.clone(), face);
        }
        let face = self.cache.get(&family_key)?.as_ref()?;
        let (mime_type, format_hint) = font_data_format(&face.data);
        Some(EmbeddedFontData {
            bytes: face.data.clone(),
            mime_type,
            format_hint,
        })
    }
}

struct FontFace {
    data: Vec<u8>,
    _index: u32,
    units_per_em: u16,
    face: Option<Face<'static>>,
    ascii_advances: Option<[u16; 128]>,
    glyph_cache: HashMap<char, Option<u16>>,
    advance_cache: HashMap<u16, u16>,
}

impl FontFace {
    fn new(data: Vec<u8>, index: u32, units_per_em: u16) -> Self {
        let face = Face::parse(&data, index)
            .ok()
            .map(|parsed| unsafe { std::mem::transmute::<Face<'_>, Face<'static>>(parsed) });
        let ascii_advances = face.as_ref().map(|parsed| {
            let mut advances = [0u16; 128];
            for byte in 0u8..=127 {
                let ch = byte as char;
                if let Some(glyph_id) = parsed.glyph_index(ch) {
                    advances[byte as usize] = parsed.glyph_hor_advance(glyph_id).unwrap_or(0);
                }
            }
            advances
        });
        Self {
            data,
            _index: index,
            units_per_em,
            face,
            ascii_advances,
            glyph_cache: HashMap::new(),
            advance_cache: HashMap::new(),
        }
    }

    fn measure_width(&mut self, text: &str, font_size: f32) -> Option<f32> {
        let scale = font_size / self.units_per_em as f32;

        if text.is_ascii()
            && let Some(advances) = &self.ascii_advances
        {
            let mut width = 0.0f32;
            for byte in text.as_bytes() {
                if *byte == b'\n' {
                    continue;
                }
                let advance = advances[*byte as usize];
                if advance == 0 {
                    width += missing_glyph_width(*byte as char, font_size);
                } else {
                    width += advance as f32 * scale;
                }
            }
            return Some(width.max(0.0));
        }

        let face = self.face.as_ref()?;
        let scale = font_size / self.units_per_em as f32;
        let mut width = 0.0f32;

        for ch in text.chars() {
            if ch == '\n' {
                continue;
            }
            let glyph = if let Some(cached) = self.glyph_cache.get(&ch) {
                *cached
            } else {
                let glyph = face.glyph_index(ch).map(|id| id.0);
                self.glyph_cache.insert(ch, glyph);
                glyph
            };

            let Some(glyph_id) = glyph else {
                width += missing_glyph_width(ch, font_size);
                continue;
            };

            let advance = if let Some(value) = self.advance_cache.get(&glyph_id) {
                *value
            } else {
                let value = face.glyph_hor_advance(GlyphId(glyph_id)).unwrap_or(0);
                self.advance_cache.insert(glyph_id, value);
                value
            };
            width += advance as f32 * scale;
        }

        Some(width.max(0.0))
    }
}

fn load_preferred_known_face(font_family: &str, weight: Weight) -> Option<FontFace> {
    if preferred_named_family(font_family).as_deref() != Some("trebuchet ms") {
        return None;
    }

    let paths: &[&str] = if weight.0 >= Weight::SEMIBOLD.0 {
        &[
            "/System/Library/Fonts/Supplemental/Trebuchet MS Bold.ttf",
            "/Library/Fonts/Trebuchet MS Bold.ttf",
            "C:\\Windows\\Fonts\\trebucbd.ttf",
            "/System/Library/Fonts/Supplemental/Trebuchet MS.ttf",
            "/Library/Fonts/Trebuchet MS.ttf",
            "C:\\Windows\\Fonts\\trebuc.ttf",
        ]
    } else {
        &[
            "/System/Library/Fonts/Supplemental/Trebuchet MS.ttf",
            "/Library/Fonts/Trebuchet MS.ttf",
            "C:\\Windows\\Fonts\\trebuc.ttf",
        ]
    };

    for path in paths {
        let Ok(bytes) = fs::read(path) else {
            continue;
        };
        let index = 0;
        let Ok(face) = Face::parse(&bytes, index) else {
            continue;
        };
        let units_per_em = face.units_per_em().max(1);
        return Some(FontFace::new(bytes, index, units_per_em));
    }
    None
}

fn preferred_named_family(font_family: &str) -> Option<String> {
    for part in font_family.split(',') {
        let raw = part.trim().trim_matches('"').trim_matches('\'').trim();
        if raw.is_empty() {
            continue;
        }
        let lower = raw.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "serif"
                | "sans-serif"
                | "monospace"
                | "cursive"
                | "fantasy"
                | "system-ui"
                | "-apple-system"
                | "ui-sans-serif"
                | "ui-monospace"
        ) {
            continue;
        }
        return Some(lower);
    }
    None
}

fn font_data_format(data: &[u8]) -> (&'static str, &'static str) {
    match data.get(0..4) {
        Some(b"wOF2") => ("font/woff2", "woff2"),
        Some(b"wOFF") => ("font/woff", "woff"),
        Some(b"OTTO") => ("font/otf", "opentype"),
        Some(b"ttcf") => ("font/collection", "truetype-collection"),
        _ => ("font/ttf", "truetype"),
    }
}

fn normalize_family_key(font_family: &str) -> String {
    let trimmed = font_family.trim();
    if trimmed.is_empty() {
        "sans-serif".to_string()
    } else {
        trimmed.to_string()
    }
}

fn cache_key(font_family: &str, weight: Weight) -> String {
    format!("{}|{}", normalize_family_key(font_family), weight.0)
}

fn cache_paths(family_key: &str) -> Option<(PathBuf, PathBuf)> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    family_key.hash(&mut hasher);
    let hash = hasher.finish();
    let dir = base.join("mmdr").join("font-cache");
    let font_path = dir.join(format!("{hash:x}.font"));
    let meta_path = dir.join(format!("{hash:x}.meta"));
    Some((font_path, meta_path))
}

fn load_cached_face(family_key: &str) -> Option<FontFace> {
    let (font_path, meta_path) = cache_paths(family_key)?;
    if !font_path.exists() || !meta_path.exists() {
        return None;
    }
    let bytes = fs::read(font_path).ok()?;
    let index: u32 = fs::read_to_string(meta_path).ok()?.trim().parse().ok()?;
    let face = Face::parse(&bytes, index).ok()?;
    let units_per_em = face.units_per_em().max(1);
    Some(FontFace::new(bytes, index, units_per_em))
}

#[cfg(test)]
mod tests {
    use super::preferred_named_family;

    #[test]
    fn preferred_named_family_skips_generic_fallbacks() {
        assert_eq!(
            preferred_named_family("'trebuchet ms', verdana, arial, sans-serif").as_deref(),
            Some("trebuchet ms")
        );
        assert_eq!(
            preferred_named_family("sans-serif, Verdana").as_deref(),
            Some("verdana")
        );
    }
}
