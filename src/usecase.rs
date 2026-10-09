//! Shared measurements for use case glyphs and JSON tables.
use crate::{
    config::LayoutConfig,
    ir::{NodeShape, UseCaseData},
    layout::{TextBlock, TextLine},
    text_metrics,
    theme::Theme,
};

pub(crate) const BORDERS: [&str; 12] = [
    "#E879F9", "#2DD4BF", "#FB923C", "#22D3EE", "#4ADE80", "#A78BFA", "#F87171", "#FACC15",
    "#818CF8", "#A3E635", "#38BDF8", "#FB7185",
];
pub(crate) const BACKGROUNDS: [&str; 12] = [
    "#FDF4FF", "#F0FDFA", "#FFF7ED", "#ECFEFF", "#F0FDF4", "#F5F3FF", "#FEF2F2", "#FEFCE8",
    "#EEF2FF", "#F7FEE7", "#F0F9FF", "#FFF1F2",
];

pub(crate) fn redux(config: &serde_json::Value) -> bool {
    matches!(
        config
            .get("theme")
            .and_then(|v| v.as_str())
            .unwrap_or("redux-color"),
        "redux" | "redux-color"
    )
}

fn variable(config: &serde_json::Value, keys: &[&str], fallback: &str) -> String {
    keys.iter()
        .find_map(|key| config.get("themeVariables")?.get(key)?.as_str())
        .unwrap_or(fallback)
        .to_string()
}

pub(crate) fn diagram_theme(data: &UseCaseData, theme: &Theme) -> Theme {
    let mut theme = crate::agentflow::theme(&data.config, theme);
    if redux(&data.config) {
        theme.primary_text_color = variable(&data.config, &["primaryTextColor"], "#28253D");
        theme.primary_color = variable(&data.config, &["mainBkg"], "#ffffff");
        theme.primary_border_color = variable(&data.config, &["nodeBorder"], "#28253D");
        theme.line_color = variable(&data.config, &["lineColor"], "#000000");
        theme.text_color = theme.primary_text_color.clone();
    }
    theme
}

pub(crate) fn node_style(data: &UseCaseData, id: &str, shape: NodeShape) -> crate::ir::NodeStyle {
    let mut style = crate::ir::NodeStyle::default();
    if !redux(&data.config) {
        return style;
    }
    let color = data
        .config
        .get("theme")
        .and_then(|v| v.as_str())
        .unwrap_or("redux-color")
        == "redux-color";
    let actor = data
        .nodes
        .get(id)
        .is_some_and(|node| node.actor_type.is_some());
    let table = data.json_tables.contains_key(id);
    let (fill, stroke) = if table {
        (
            variable(&data.config, &["mainBkg"], "#ffffff"),
            variable(&data.config, &["nodeBorder"], "#28253D"),
        )
    } else if shape == NodeShape::Note {
        (
            variable(&data.config, &["noteBkgColor"], "#fff5ad"),
            variable(&data.config, &["noteBorderColor"], "#FACC15"),
        )
    } else if actor {
        (
            variable(
                &data.config,
                &["usecaseActorBkg", "actorBkg", "mainBkg"],
                if color { "#F5F3FF" } else { "#ffffff" },
            ),
            variable(
                &data.config,
                &["usecaseActorBorder", "actorBorder"],
                if color { "#A78BFA" } else { "#28253D" },
            ),
        )
    } else {
        (
            variable(
                &data.config,
                &["usecaseBkg", "mainBkg"],
                if color { "#F0FDFA" } else { "#ffffff" },
            ),
            variable(
                &data.config,
                &["usecaseBorder", "nodeBorder"],
                if color { "#2DD4BF" } else { "#28253D" },
            ),
        )
    };
    style.fill = Some(fill);
    style.stroke = Some(stroke);
    style.text_color = Some(variable(
        &data.config,
        if actor {
            &["actorTextColor", "primaryTextColor"]
        } else {
            &["primaryTextColor"]
        },
        "#28253D",
    ));
    if !table {
        style.stroke_width = Some(2.0);
    }
    style
}

pub(crate) fn boundary_style(data: &UseCaseData, index: usize) -> crate::ir::NodeStyle {
    let mut style = crate::ir::NodeStyle::default();
    if !redux(&data.config) {
        return style;
    }
    let variable_palette = |key: &str, defaults: &[&str]| -> Option<String> {
        if let Some(values) = data
            .config
            .get("themeVariables")
            .and_then(|v| v.get(key))
            .and_then(|v| v.as_array())
        {
            if values.is_empty() {
                return None;
            }
            return values[index % values.len()].as_str().map(str::to_string);
        }
        Some(defaults[index % defaults.len()].to_string())
    };
    if data
        .config
        .get("theme")
        .and_then(|v| v.as_str())
        .unwrap_or("redux-color")
        == "redux-color"
    {
        style.stroke = variable_palette("borderColorArray", &BORDERS);
        if style.stroke.is_some() {
            style.fill = variable_palette("bkgColorArray", &BACKGROUNDS);
        }
    }
    style.stroke.get_or_insert_with(|| {
        variable(
            &data.config,
            &["usecaseBoundaryBorder", "clusterBorder"],
            "#BDBCCC",
        )
    });
    style.fill.get_or_insert_with(|| {
        variable(
            &data.config,
            &["usecaseBoundaryBkg", "clusterBkg"],
            "#FAFAFC",
        )
    });
    style
}

pub(crate) fn node_theme(data: &UseCaseData, id: &str, theme: &Theme) -> Theme {
    if data.json_tables.contains_key(id) || id.starts_with("__usecase_note_") {
        return diagram_theme(data, theme);
    }
    let mut result = diagram_theme(data, theme);
    let actor = data
        .nodes
        .get(id)
        .and_then(|node| node.actor_type)
        .is_some();
    let prefix = if actor { "actor" } else { "usecase" };
    result.font_size = data
        .config
        .get(format!("{prefix}FontSize"))
        .and_then(|value| value.as_f64())
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value as f32)
        .unwrap_or(if actor { 14.0 } else { 12.0 });
    result.font_family = data
        .config
        .get(format!("{prefix}FontFamily"))
        .and_then(|value| value.as_str())
        .unwrap_or("Open Sans, sans-serif")
        .to_string();
    result
}

pub(crate) fn text_block(value: &str, theme: &Theme, config: &LayoutConfig) -> TextBlock {
    let lines: Vec<_> = value
        .split('\n')
        .map(|line| TextLine::plain(line.to_string()))
        .collect();
    let width = lines
        .iter()
        .map(|line| {
            text_metrics::measure_text_width_with_kerning(
                &line.text(),
                theme.font_size,
                &theme.font_family,
            )
            .unwrap_or(line.text().chars().count() as f32 * theme.font_size * 0.5)
        })
        .fold(0.0, f32::max);
    let height = lines.len() as f32 * theme.font_size * config.label_line_height;
    TextBlock {
        lines,
        width,
        height,
    }
}

pub(crate) fn font_weight(data: &UseCaseData, id: &str) -> Option<String> {
    let prefix = if data
        .nodes
        .get(id)
        .and_then(|node| node.actor_type)
        .is_some()
    {
        "actor"
    } else {
        "usecase"
    };
    data.config
        .get(format!("{prefix}FontWeight"))
        .and_then(|value| {
            value
                .as_str()
                .map(str::to_string)
                .or_else(|| value.as_u64().map(|weight| weight.to_string()))
        })
}

pub(crate) fn measure_bold(block: &mut TextBlock, theme: &Theme) {
    block.width = block
        .lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| {
                    text_metrics::measure_styled_text_width(
                        &span.text,
                        theme.font_size,
                        &theme.font_family,
                        true,
                        span.style.italic,
                    )
                    .unwrap_or(span.text.chars().count() as f32 * theme.font_size * 0.6)
                })
                .sum::<f32>()
        })
        .fold(0.0, f32::max);
}

/// Wrap an already parsed label without discarding inline bold or italic spans.
/// A wrapped HTML label occupies the configured width even when its last line is short.
pub(crate) fn wrap_label(block: &mut TextBlock, limit: f32, theme: &Theme, config: &LayoutConfig) {
    wrap_label_impl(block, limit, theme, config, false);
}

pub(crate) fn wrap_flowchart_label(
    block: &mut TextBlock,
    limit: f32,
    theme: &Theme,
    config: &LayoutConfig,
) {
    wrap_label_impl(block, limit, theme, config, true);
}

fn wrap_label_impl(
    block: &mut TextBlock,
    limit: f32,
    theme: &Theme,
    config: &LayoutConfig,
    inline_icons: bool,
) {
    use crate::layout::{SpanStyle, TextSpan};
    fn line(units: &[(String, SpanStyle)]) -> TextLine {
        let mut spans: Vec<TextSpan> = Vec::new();
        for (text, style) in units {
            if let Some(last) = spans.last_mut().filter(|last| last.style == *style) {
                last.text.push_str(text);
            } else {
                spans.push(TextSpan {
                    text: text.clone(),
                    style: *style,
                });
            }
        }
        if spans.is_empty() {
            spans.push(TextSpan {
                text: String::new(),
                style: SpanStyle::default(),
            });
        }
        TextLine { spans }
    }
    let measure = |line: &TextLine| {
        line.spans
            .iter()
            .map(|span| {
                if inline_icons {
                    if let Some(width) = crate::icons::inline_width(
                        &span.text,
                        theme.font_size,
                        &theme.font_family,
                        span.style.bold,
                        span.style.italic,
                    ) {
                        return width;
                    }
                }
                text_metrics::measure_styled_text_width(
                    &span.text,
                    theme.font_size,
                    &theme.font_family,
                    span.style.bold,
                    span.style.italic,
                )
                .unwrap_or(span.text.chars().count() as f32 * theme.font_size * 0.5)
            })
            .sum::<f32>()
    };
    let mut lines = Vec::new();
    let mut wrapped = false;
    for original in &block.lines {
        let mut current = Vec::new();
        for span in &original.spans {
            // Treat each recognized icon token as one indivisible unit. Its
            // name must not trigger wrapping before the full token is measured.
            let units: Vec<String> = if inline_icons {
                crate::icons::label_parts(&span.text)
                    .into_iter()
                    .flat_map(|part| match part {
                        crate::icons::LabelPart::Text(text) => {
                            text.chars().map(|ch| ch.to_string()).collect()
                        }
                        crate::icons::LabelPart::Icon(name) => vec![name.to_string()],
                    })
                    .collect()
            } else {
                span.text.chars().map(|ch| ch.to_string()).collect()
            };
            for unit in units {
                current.push((unit, span.style));
                if measure(&line(&current)) > limit {
                    if let Some(split) = current
                        .iter()
                        .rposition(|(text, _)| text.chars().all(char::is_whitespace))
                        .filter(|split| *split > 0)
                    {
                        let rest = current.split_off(split + 1);
                        while current
                            .last()
                            .is_some_and(|(text, _)| text.chars().all(char::is_whitespace))
                        {
                            current.pop();
                        }
                        lines.push(line(&current));
                        current = rest;
                        wrapped = true;
                    }
                }
            }
        }
        lines.push(line(&current));
    }
    block.width = lines
        .iter()
        .map(&measure)
        .fold(if wrapped { limit } else { 0.0 }, f32::max);
    block.height = lines.len() as f32 * theme.font_size * config.label_line_height;
    block.lines = lines;
}

pub(crate) struct TableMetrics {
    pub key_width: f32,
    pub width: f32,
    pub height: f32,
    pub title_height: f32,
    pub row_heights: Vec<f32>,
}

pub(crate) fn table_metrics(
    rows: &[(String, String)],
    title: &TextBlock,
    theme: &Theme,
    config: &LayoutConfig,
    border_width: f32,
) -> TableMetrics {
    let mut key_width = 16.0_f32;
    let mut value_width = 16.0_f32;
    let mut row_heights = Vec::new();
    for (key, value) in rows {
        let key = text_block(key, theme, config);
        let value = text_block(value, theme, config);
        key_width = key_width.max(key.width + 16.0);
        value_width = value_width.max(value.width + 16.0);
        row_heights.push(key.height.max(value.height) + 8.0);
    }
    let border_width = border_width.max(0.0);
    let width = (key_width + value_width).max(title.width + 16.0) + border_width * 2.0;
    let title_height = title.height + 8.0;
    let height = title_height + row_heights.iter().sum::<f32>() + border_width * 2.0;
    TableMetrics {
        key_width,
        width,
        height,
        title_height,
        row_heights,
    }
}

pub(crate) fn node_size(
    data: &UseCaseData,
    id: &str,
    shape: NodeShape,
    label: &TextBlock,
    stereotype: Option<&TextBlock>,
    theme: &Theme,
    config: &LayoutConfig,
    border_width: f32,
) -> (f32, f32) {
    if let Some(rows) = data.json_tables.get(id) {
        let metrics = table_metrics(rows, label, theme, config, border_width);
        return (metrics.width, metrics.height);
    }
    let width = label
        .width
        .max(stereotype.map(|label| label.width).unwrap_or(0.0));
    let height = label.height + stereotype.map(|label| label.height + 2.0).unwrap_or(0.0);
    if data
        .nodes
        .get(id)
        .and_then(|node| node.actor_type)
        .is_some()
    {
        return (width.max(56.0) + 20.0, 72.0 + 8.0 + height + 20.0);
    }
    match shape {
        NodeShape::Ellipse => (width + 40.0, height + 40.0),
        NodeShape::Rectangle
            if data.config.get("look").and_then(|v| v.as_str()) == Some("classic") =>
        {
            (width + 40.0, height + 20.0)
        }
        NodeShape::Rectangle => (width + 32.0, height + 24.0),
        _ => (width + 20.0, height + 20.0),
    }
}
