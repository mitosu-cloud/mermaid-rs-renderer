use fontdb::{Database, Family, Query, Stretch, Style, Weight};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Mutex;
use ttf_parser::{Face, GlyphId};

static TEXT_MEASURER: Lazy<Mutex<TextMeasurer>> = Lazy::new(|| Mutex::new(TextMeasurer::new()));

/// The Redux themes ship Recursive; use the same face for geometry and PNGs.
pub(crate) fn load_bundled_fonts(db: &mut Database) {
    db.load_font_data(include_bytes!("fonts/OpenSans-400-normal.ttf").to_vec());
    db.load_font_data(include_bytes!("fonts/OpenSans-400-italic.ttf").to_vec());
    db.load_font_data(include_bytes!("fonts/Recursive-Regular.ttf").to_vec());
    db.load_font_data(include_bytes!("fonts/Recursive-Bold.ttf").to_vec());
    // Chromium synthesizes a 0.25 shear for the normal-only Recursive webfont.
    // resvg needs explicit faces; these preserve the normal advances and GPOS
    // tables with that measured shear applied to their glyph outlines.
    db.load_font_data(include_bytes!("fonts/Recursive-Italic.ttf").to_vec());
    db.load_font_data(include_bytes!("fonts/Recursive-BoldItalic.ttf").to_vec());
}

pub fn measure_text_width(text: &str, font_size: f32, font_family: &str) -> Option<f32> {
    if text.is_empty() || font_size <= 0.0 {
        return Some(0.0);
    }
    let mut guard = TEXT_MEASURER.lock().ok()?;
    guard.measure(text, font_size, font_family)
}

/// Baseline offset from the center of a CSS line box, using Chromium's rounded
/// pixel ascent and descent before distributing the line leading.
pub fn centered_baseline_offset(font_size: f32, font_family: &str) -> Option<f32> {
    let mut guard = TEXT_MEASURER.lock().ok()?;
    let font = guard.face(font_family)?;
    let face = font.face.as_ref()?;
    let scale = font_size / font.units_per_em as f32;
    let ascent = (face.ascender() as f32 * scale).round();
    let descent = (-face.descender() as f32 * scale).round();
    Some((ascent - descent) / 2.0)
}

/// Height of a single SVG text line using the browser's pixel-rounded metrics.
pub(crate) fn svg_text_height(font_size: f32, font_family: &str) -> Option<f32> {
    let mut guard = TEXT_MEASURER.lock().ok()?;
    let font = guard.face(font_family)?;
    let face = font.face.as_ref()?;
    let scale = font_size / font.units_per_em as f32;
    Some(
        (face.ascender() as f32 * scale).round()
            + (-face.descender() as f32 * scale * 2.0).round() / 2.0,
    )
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
            if c.is_ascii_uppercase() {
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

/// Measure browser-style label advances including horizontal font kerning.
pub fn measure_text_width_with_kerning(
    text: &str,
    font_size: f32,
    font_family: &str,
) -> Option<f32> {
    measure_styled_text_width(text, font_size, font_family, false, false)
}

/// Measure an inline label with its actual bold/italic face and kerning.
pub(crate) fn measure_styled_text_width(
    text: &str,
    font_size: f32,
    font_family: &str,
    bold: bool,
    italic: bool,
) -> Option<f32> {
    let mut measurer = TEXT_MEASURER.lock().ok()?;
    let font = measurer.styled_face(font_family, bold, italic)?;
    let width = font.measure_width(text, font_size)?;
    let face = font.face.as_ref()?;
    // Modern OpenType faces (including Recursive) store kerning in GPOS,
    // rather than the legacy `kern` table. Browser shaping prefers GPOS.
    if let Some(table) = face.tables().gpos {
        use ttf_parser::gpos::{PairAdjustment, PositioningSubtable};
        let mut indices = Vec::new();
        for feature in table
            .features
            .into_iter()
            .filter(|feature| feature.tag == ttf_parser::Tag::from_bytes(b"kern"))
        {
            for index in feature.lookup_indices {
                if !indices.contains(&index) {
                    indices.push(index);
                }
            }
        }
        if !indices.is_empty() {
            let glyphs: Vec<_> = text
                .chars()
                .map(|ch| {
                    if ch == '\n' {
                        None
                    } else {
                        face.glyph_index(ch)
                    }
                })
                .collect();
            let mut adjustment = 0i32;
            for index in indices {
                let Some(lookup) = table.lookups.get(index) else {
                    continue;
                };
                for pair in glyphs.windows(2) {
                    let (Some(left), Some(right)) = (pair[0], pair[1]) else {
                        continue;
                    };
                    for index in 0..lookup.subtables.len() {
                        let Some(PositioningSubtable::Pair(pair)) = lookup.subtables.get(index)
                        else {
                            continue;
                        };
                        let values = match pair {
                            PairAdjustment::Format1 { coverage, sets } => coverage
                                .get(left)
                                .and_then(|index| sets.get(index))
                                .and_then(|set| set.get(right)),
                            PairAdjustment::Format2 {
                                coverage,
                                classes,
                                matrix,
                            } => coverage.get(left).and_then(|_| {
                                matrix.get((classes.0.get(left), classes.1.get(right)))
                            }),
                        };
                        if let Some((left, right)) = values {
                            adjustment += left.x_advance as i32 + right.x_advance as i32;
                            break;
                        }
                    }
                }
            }
            return Some(
                (width + adjustment as f32 * font_size / font.units_per_em as f32).max(0.0),
            );
        }
    }
    let Some(table) = face.tables().kern else {
        return Some(width);
    };
    let mut adjustment = 0i32;
    let mut previous = None;
    for ch in text.chars() {
        let glyph = if ch == '\n' {
            None
        } else {
            face.glyph_index(ch)
        };
        if let (Some(left), Some(right)) = (previous, glyph) {
            for subtable in table.subtables {
                if subtable.horizontal && !subtable.variable && !subtable.has_cross_stream {
                    adjustment += subtable.glyphs_kerning(left, right).unwrap_or(0) as i32;
                }
            }
        }
        previous = glyph;
    }
    Some((width + adjustment as f32 * font_size / font.units_per_em as f32).max(0.0))
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

struct TextMeasurer {
    db: Database,
    loaded_system_fonts: bool,
    cache: HashMap<String, Option<FontFace>>,
}

impl TextMeasurer {
    fn new() -> Self {
        let mut db = Database::new();
        load_bundled_fonts(&mut db);
        Self {
            db,
            loaded_system_fonts: false,
            cache: HashMap::new(),
        }
    }

    fn face(&mut self, font_family: &str) -> Option<&mut FontFace> {
        self.styled_face(font_family, false, false)
    }

    fn styled_face(
        &mut self,
        font_family: &str,
        bold: bool,
        italic: bool,
    ) -> Option<&mut FontFace> {
        let family_key = styled_family_key(font_family, bold, italic);
        if !self.cache.contains_key(&family_key) {
            let face = self.load_face(font_family, bold, italic);
            self.cache.insert(family_key.clone(), face);
        }
        self.cache.get_mut(&family_key)?.as_mut()
    }

    fn measure(&mut self, text: &str, font_size: f32, font_family: &str) -> Option<f32> {
        let face = self.face(font_family)?;
        let normalized = text.replace('\t', "    ");
        face.measure_width(&normalized, font_size)
    }

    fn load_face(&mut self, font_family: &str, bold: bool, italic: bool) -> Option<FontFace> {
        let family_key = styled_family_key(font_family, bold, italic);
        if !font_family.contains("Recursive")
            && !font_family.contains("Open Sans")
            && let Some(face) = load_cached_face(&family_key)
        {
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
            #[cfg(target_os = "ios")]
            {
                self.db.load_fonts_dir("/System/Library/Fonts");
                self.db.load_fonts_dir("/System/Library/Fonts/Core");
            }
            self.loaded_system_fonts = true;
        }

        let query = Query {
            families: &families,
            weight: if bold { Weight::BOLD } else { Weight::NORMAL },
            stretch: Stretch::Normal,
            style: if italic { Style::Italic } else { Style::Normal },
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
}

struct FontFace {
    _data: Vec<u8>,
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
            _data: data,
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
        let fallback = font_size * 0.56;

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
                    width += fallback;
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
                width += fallback;
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

fn normalize_family_key(font_family: &str) -> String {
    let trimmed = font_family.trim();
    if trimmed.is_empty() {
        "sans-serif".to_string()
    } else {
        trimmed.to_string()
    }
}

fn styled_family_key(font_family: &str, bold: bool, italic: bool) -> String {
    let family = normalize_family_key(font_family);
    if bold || italic {
        format!("{family}\0bold={bold};italic={italic}")
    } else {
        family
    }
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
