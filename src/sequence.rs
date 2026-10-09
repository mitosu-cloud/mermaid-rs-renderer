//! Sequence settings and browser-equivalent text measurements shared by layout
//! and painting. Geometry follows Mermaid's sequenceRenderer / actorBands.
use crate::{
    layout::{TextBlock, TextLine},
    text_metrics,
    theme::Theme,
};
use serde_json::Value;

static ENTITY_TOKEN: once_cell::sync::Lazy<regex::Regex> =
    once_cell::sync::Lazy::new(|| regex::Regex::new(r"#(\w+);").unwrap());

fn protected_entities(text: &str) -> String {
    ENTITY_TOKEN
        .replace_all(text, |caps: &regex::Captures| {
            let token = &caps[1];
            format!(
                "ﬂ°{}{}¶ß",
                if token.chars().all(|c| c.is_ascii_digit()) {
                    "°"
                } else {
                    ""
                },
                token
            )
        })
        .into_owned()
}
fn painted_entities(text: String) -> String {
    crate::layout::decode_mermaid_entities(
        &text
            .replace("ﬂ°°", "#")
            .replace("ﬂ°", "#")
            .replace("¶ß", ";"),
    )
}

pub(crate) fn number(options: &Value, key: &str, default: f32) -> f32 {
    options
        .get("sequence")
        .and_then(|v| v.get(key))
        .and_then(|v| {
            v.as_f64()
                .map(|v| v as f32)
                .or_else(|| v.as_str()?.trim_end_matches("px").parse().ok())
        })
        .filter(|v: &f32| v.is_finite() && *v >= 0.0)
        .unwrap_or(default)
}

pub(crate) fn flag(options: &Value, key: &str, default: bool) -> bool {
    options
        .get("sequence")
        .and_then(|v| v.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

pub(crate) fn neo(options: &Value) -> bool {
    options.get("look").and_then(Value::as_str).unwrap_or("neo") == "neo"
}

pub(crate) fn variable(options: &Value, key: &str, default: &str) -> String {
    options
        .get("themeVariables")
        .and_then(|v| v.get(key))
        .and_then(Value::as_str)
        .unwrap_or(default)
        .into()
}

pub(crate) fn theme(options: &Value, original: &Theme) -> Theme {
    let mut theme = crate::agentflow::theme(options, original);
    // Redux's other diagrams use 14px. Sequence CSS and its own config use 16px.
    theme.font_size = number(options, "messageFontSize", 16.0);
    if crate::usecase::redux(options) {
        theme.sequence_actor_fill = variable(options, "actorBkg", "#ffffff");
        theme.sequence_actor_border = variable(options, "actorBorder", "#28253D");
        theme.sequence_actor_line = variable(options, "actorLineColor", "#28253D");
        theme.sequence_note_fill = variable(options, "noteBkgColor", "#fff5ad");
        theme.sequence_note_border = variable(options, "noteBorderColor", "#FACC15");
        theme.text_color = variable(options, "actorTextColor", "#28253D");
        theme.line_color = variable(options, "signalColor", "#28253D");
    }
    if let Some(family) = options.get("fontFamily").and_then(Value::as_str) {
        theme.font_family = family.trim_end_matches(';').into();
    }
    theme
}

pub(crate) fn actor_colors(options: &Value, theme: &Theme, slot: usize) -> (String, String) {
    if options
        .get("theme")
        .and_then(Value::as_str)
        .unwrap_or("redux-color")
        == "redux-color"
    {
        let color = |key: &str, fallback: &[&str]| {
            options
                .get("themeVariables")
                .and_then(|v| v.get(key))
                .and_then(Value::as_array)
                .and_then(|a| a.get(slot % a.len().max(1)))
                .and_then(Value::as_str)
                .unwrap_or(fallback[slot % fallback.len()])
                .to_string()
        };
        (
            color("bkgColorArray", &crate::usecase::BACKGROUNDS),
            color("borderColorArray", &crate::usecase::BORDERS),
        )
    } else {
        (
            theme.sequence_actor_fill.clone(),
            theme.sequence_actor_border.clone(),
        )
    }
}

pub(crate) struct Metrics {
    pub family: String,
    pub size: f32,
    pub line_height: f32,
    pub painted_line_height: f32,
    pub wrap: bool,
}

impl Metrics {
    pub fn new(options: &Value, theme: &Theme) -> Self {
        // Mermaid's default config ends its font-family value with a semicolon.
        // SVG style.setProperty rejects that value on its unscoped measurement
        // nodes; Chromium measures in the document's serif fallback (Times).
        // Drawing is separately scoped to Redux's Recursive webfont. A valid
        // explicit family must instead be measured in that supplied family.
        let family = options
            .get("fontFamily")
            .and_then(Value::as_str)
            .filter(|s| !s.ends_with(';'))
            .unwrap_or("Times New Roman")
            .to_string();
        let size = options
            .get("fontSize")
            .and_then(Value::as_f64)
            .map(|v| v as f32)
            .unwrap_or_else(|| number(options, "messageFontSize", 16.0));
        let line_height = text_metrics::svg_text_height(size, &family)
            .unwrap_or(size * 1.125)
            .round();
        let painted_line_height =
            text_metrics::svg_text_height(theme.font_size, &theme.font_family)
                .unwrap_or(theme.font_size * 1.1875)
                .round();
        Self {
            family,
            size,
            line_height,
            painted_line_height,
            wrap: flag(options, "wrap", false),
        }
    }

    pub fn width(&self, text: &str) -> f32 {
        text_metrics::measure_styled_text_width(text, self.size, &self.family, false, false)
            .unwrap_or_else(|| {
                text_metrics::get_computed_text_length(text, self.size, &self.family)
            })
            .round()
    }

    pub fn label(&self, text: &str, width: Option<f32>) -> TextBlock {
        let (text, wrap) = if let Some(t) = text.strip_prefix("wrap:") {
            (t.trim(), true)
        } else if let Some(t) = text.strip_prefix("nowrap:") {
            (t.trim(), false)
        } else {
            (text, self.wrap)
        };
        let text = text
            .replace("<br />", "\n")
            .replace("<br/>", "\n")
            .replace("<br>", "\n");
        // Mermaid protects entities before parsing, measures the protected
        // tokens, then restores the real characters in the final SVG.
        let text = protected_entities(&text);
        let mut lines = Vec::new();
        for line in text.split('\n') {
            if wrap && let Some(width) = width {
                lines.extend(text_metrics::wrap_text(
                    line,
                    width.max(1.0),
                    self.size,
                    &self.family,
                ));
            } else {
                lines.push(line.to_string());
            }
        }
        let width = lines.iter().map(|s| self.width(s)).fold(0.0f32, f32::max);
        TextBlock {
            width,
            height: self.line_height * lines.len() as f32,
            lines: lines
                .into_iter()
                .map(painted_entities)
                .map(TextLine::plain)
                .collect(),
        }
    }

    pub fn frame_label(&self, text: Option<&str>, width: f32) -> TextBlock {
        let Some(text) = text.filter(|t| !t.is_empty()) else {
            return self.label("", None);
        };
        let text = format!(
            "[{}]",
            text.trim_start_matches("wrap:")
                .trim_start_matches("nowrap:")
                .trim()
        );
        let mut result = self.label(&text, None);
        // Loop titles are always wrapped in upstream calculateLoopBounds.
        let lines: Vec<_> = result
            .lines
            .iter()
            .flat_map(|l| {
                text_metrics::wrap_text(&l.text(), width.max(1.0), self.size, &self.family)
            })
            .collect();
        result.width = lines.iter().map(|s| self.width(s)).fold(0.0f32, f32::max);
        result.height = lines.len() as f32 * self.line_height;
        result.lines = lines.into_iter().map(TextLine::plain).collect();
        result
    }
}
