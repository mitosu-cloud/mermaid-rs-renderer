uniffi::setup_scaffolding!();

use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MermaidError {
    #[error("Parse failed: {msg}")]
    ParseFailed { msg: String },
    #[error("Render failed: {msg}")]
    RenderFailed { msg: String },
    #[error("IO failed: {msg}")]
    IoFailed { msg: String },
    #[error("Theme parse failed: {msg}")]
    ThemeParseFailed { msg: String },
}

// ── Checksum ────────────────────────────────────────────────────────────────

/// SHA256 of mermaid source, truncated to 12 hex chars.
fn compute_checksum(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    let result = hasher.finalize();
    hex_encode(&result[..6])
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Sanitise theme name for use in filenames: lowercase, spaces → hyphens.
fn sanitise_theme_name(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

// ── Theme mapping ───────────────────────────────────────────────────────────

/// Build a `mermaid_rs_renderer::Theme` from Mitosu theme JSON.
fn build_theme(theme_json: &str) -> Result<mermaid_rs_renderer::Theme, MermaidError> {
    let val: serde_json::Value = serde_json::from_str(theme_json)
        .map_err(|e| MermaidError::ThemeParseFailed { msg: e.to_string() })?;

    let colors = val.get("colors").and_then(|v| v.as_object());
    let is_dark = val
        .get("type")
        .and_then(|v| v.as_str())
        .map(|t| t == "dark")
        .unwrap_or(true);

    // Start from a base theme matching the mode
    let mut theme = if is_dark {
        mermaid_rs_renderer::Theme::dark()
    } else {
        mermaid_rs_renderer::Theme::modern()
    };

    let colors = match colors {
        Some(c) => c,
        None => return Ok(theme),
    };

    // Helper: look up a key, returning None if missing
    let get = |key: &str| -> Option<String> {
        colors.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
    };

    // Direct mermaid.* keys take priority, then fallback mapping
    macro_rules! map_field {
        ($field:ident, $mermaid_key:expr, [ $($fallback:expr),* ]) => {
            if let Some(v) = get($mermaid_key) {
                theme.$field = v;
            } $(else if let Some(v) = get($fallback) {
                theme.$field = v;
            })*
        };
    }

    map_field!(background, "mermaid.background", ["markdownEditor.background", "ui.background"]);
    map_field!(primary_color, "mermaid.primaryColor", ["markdownEditor.codeBlockBackground"]);
    map_field!(primary_text_color, "mermaid.primaryTextColor", ["markdownEditor.text", "ui.text"]);
    map_field!(primary_border_color, "mermaid.primaryBorderColor", ["ui.divider"]);
    map_field!(line_color, "mermaid.lineColor", ["ui.divider"]);
    map_field!(secondary_color, "mermaid.secondaryColor", ["wysiwygEditor.blockquoteBackground"]);
    map_field!(tertiary_color, "mermaid.tertiaryColor", ["ui.secondaryBackground"]);
    map_field!(text_color, "mermaid.textColor", ["markdownEditor.text", "ui.text"]);
    map_field!(edge_label_background, "mermaid.edgeLabelBackground", ["markdownEditor.background", "ui.background"]);
    map_field!(cluster_background, "mermaid.clusterBackground", ["ui.secondaryBackground"]);
    map_field!(cluster_border, "mermaid.clusterBorder", ["ui.divider"]);
    map_field!(sequence_actor_fill, "mermaid.sequenceActorFill", ["markdownEditor.codeBlockBackground"]);
    map_field!(sequence_actor_border, "mermaid.sequenceActorBorder", ["ui.divider"]);
    map_field!(sequence_note_fill, "mermaid.sequenceNoteFill", ["wysiwygEditor.blockquoteBackground"]);
    map_field!(sequence_note_border, "mermaid.sequenceNoteBorder", ["ui.divider"]);

    // Copy actor border to actor line if not explicitly set
    theme.sequence_actor_line = theme.sequence_actor_border.clone();
    // Copy some defaults from primary
    theme.sequence_activation_fill = theme.secondary_color.clone();
    theme.sequence_activation_border = theme.primary_border_color.clone();

    Ok(theme)
}

// ── Metadata ────────────────────────────────────────────────────────────────

/// In-memory representation of metadata.json
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Metadata {
    #[serde(default)]
    mermaid_diagrams: HashMap<String, DiagramEntry>,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
struct DiagramEntry {
    files: HashMap<String, String>,
}

fn metadata_path(note_folder: &Path) -> PathBuf {
    note_folder.join("metadata.json")
}

fn read_metadata(note_folder: &Path) -> Metadata {
    let path = metadata_path(note_folder);
    match fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Metadata::default(),
    }
}

fn write_metadata(note_folder: &Path, meta: &Metadata) -> Result<(), MermaidError> {
    let path = metadata_path(note_folder);
    let json = serde_json::to_string_pretty(meta)
        .map_err(|e| MermaidError::IoFailed { msg: e.to_string() })?;
    fs::write(&path, json).map_err(|e| MermaidError::IoFailed { msg: e.to_string() })?;
    Ok(())
}

// ── Public FFI functions ────────────────────────────────────────────────────

/// Primary render function — handles everything:
/// 1. Parses the Mitosu theme JSON → maps to mermaid Theme
/// 2. Computes SHA256 checksum of mermaid source
/// 3. Checks if output file already exists on disk (cache hit)
/// 4. If not, renders and writes to disk (SVG string or PNG via resvg)
/// 5. Updates metadata.json
/// Returns the filename (not full path) of the output file.
///
/// `output_format` should be `"svg"` or `"png"`. Defaults to SVG for unknown values.
#[uniffi::export]
pub fn render_mermaid_for_note(
    mermaid_source: String,
    note_folder_path: String,
    theme_json: String,
    theme_name: String,
    width: f32,
    height: f32,
    output_format: String,
) -> Result<String, MermaidError> {
    let note_folder = Path::new(&note_folder_path);
    let checksum = compute_checksum(&mermaid_source);
    let safe_theme = sanitise_theme_name(&theme_name);
    let use_png = output_format.eq_ignore_ascii_case("png");
    let ext = if use_png { "png" } else { "svg" };
    let filename = format!("mermaid_{}_{}.{}", checksum, safe_theme, ext);
    let output_path = note_folder.join(&filename);

    // Cache hit — file already exists
    if output_path.exists() {
        return Ok(filename);
    }

    // Build theme from Mitosu JSON
    let theme = build_theme(&theme_json)?;

    // Render SVG string
    let options = mermaid_rs_renderer::RenderOptions {
        theme: theme.clone(),
        layout: mermaid_rs_renderer::LayoutConfig::default(),
    };
    let svg = mermaid_rs_renderer::render_with_options(&mermaid_source, options)
        .map_err(|e| MermaidError::RenderFailed { msg: e.to_string() })?;

    if use_png {
        // SVG → PNG via resvg
        let render_cfg = mermaid_rs_renderer::RenderConfig {
            width,
            height,
            background: theme.background.clone(),
        };
        mermaid_rs_renderer::write_output_png(&svg, &output_path, &render_cfg, &theme)
            .map_err(|e| MermaidError::RenderFailed { msg: e.to_string() })?;
    } else {
        // Write SVG string directly
        fs::write(&output_path, &svg)
            .map_err(|e| MermaidError::IoFailed { msg: e.to_string() })?;
    }

    // Update metadata.json
    let mut meta = read_metadata(note_folder);
    let entry = meta
        .mermaid_diagrams
        .entry(checksum.clone())
        .or_default();
    entry.files.insert(safe_theme, filename.clone());
    write_metadata(note_folder, &meta)?;

    Ok(filename)
}

/// Compute checksum only (for Swift-side cache checks without full render).
#[uniffi::export]
pub fn mermaid_checksum(mermaid_source: String) -> String {
    compute_checksum(&mermaid_source)
}

/// Validate mermaid syntax without rendering.
#[uniffi::export]
pub fn validate_mermaid(mermaid_source: String) -> Result<(), MermaidError> {
    mermaid_rs_renderer::parse_mermaid(&mermaid_source)
        .map_err(|e| MermaidError::ParseFailed { msg: e.to_string() })?;
    Ok(())
}

/// Remove all mermaid PNGs whose checksum is NOT in `valid_checksums`.
/// Updates metadata.json. Returns list of deleted filenames.
#[uniffi::export]
pub fn cleanup_stale_mermaid_files(
    note_folder_path: String,
    valid_checksums: Vec<String>,
) -> Result<Vec<String>, MermaidError> {
    let note_folder = Path::new(&note_folder_path);
    let mut meta = read_metadata(note_folder);
    let mut deleted: Vec<String> = Vec::new();
    let valid_set: HashSet<&str> = valid_checksums.iter().map(|s| s.as_str()).collect();

    // Collect checksums to remove from metadata
    let stale_checksums: Vec<String> = meta
        .mermaid_diagrams
        .keys()
        .filter(|cs| !valid_set.contains(cs.as_str()))
        .cloned()
        .collect();

    for cs in &stale_checksums {
        if let Some(entry) = meta.mermaid_diagrams.remove(cs) {
            for (_theme, filename) in &entry.files {
                let path = note_folder.join(filename);
                if path.exists() {
                    let _ = fs::remove_file(&path);
                    deleted.push(filename.clone());
                }
            }
        }
    }

    // Also scan for untracked mermaid PNGs on disk
    if let Ok(entries) = fs::read_dir(note_folder) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("mermaid_") && (name.ends_with(".png") || name.ends_with(".svg")) {
                // Extract checksum from filename: mermaid_<checksum>_<theme>.(png|svg)
                if let Some(cs) = name
                    .strip_prefix("mermaid_")
                    .and_then(|rest| rest.split('_').next())
                {
                    if !valid_set.contains(cs) && !deleted.contains(&name) {
                        let path = entry.path();
                        if path.exists() {
                            let _ = fs::remove_file(&path);
                            deleted.push(name);
                        }
                    }
                }
            }
        }
    }

    if !stale_checksums.is_empty() {
        write_metadata(note_folder, &meta)?;
    }

    Ok(deleted)
}

/// Read mermaid metadata from a note folder's metadata.json.
/// Returns JSON string of the mermaid_diagrams section (or empty object).
#[uniffi::export]
pub fn get_mermaid_metadata(note_folder_path: String) -> Result<String, MermaidError> {
    let note_folder = Path::new(&note_folder_path);
    let meta = read_metadata(note_folder);
    serde_json::to_string(&meta.mermaid_diagrams)
        .map_err(|e| MermaidError::IoFailed { msg: e.to_string() })
}
