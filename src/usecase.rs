//! Shared measurements for use case glyphs and JSON tables.
use crate::{
    config::LayoutConfig,
    ir::{NodeShape, UseCaseData},
    layout::{TextBlock, TextLine},
    text_metrics,
    theme::Theme,
};

pub(crate) fn node_theme(data: &UseCaseData, id: &str, theme: &Theme) -> Theme {
    let mut result = theme.clone();
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
    let width = (key_width + value_width).max(title.width + 16.0) + 2.0;
    let title_height = title.height + 8.0;
    let height = title_height + row_heights.iter().sum::<f32>() + 2.0;
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
) -> (f32, f32) {
    if let Some(rows) = data.json_tables.get(id) {
        let metrics = table_metrics(rows, label, theme, config);
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
        _ => (width + 20.0, height + 20.0),
    }
}
