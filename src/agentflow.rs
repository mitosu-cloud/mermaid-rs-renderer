//! Agentflow defaults layered over its shared flowchart rendering pipeline.
use crate::theme::Theme;
use serde_json::Value;

pub(crate) fn options(config: &Value) -> &Value {
    config.get("agentflow").unwrap_or(&Value::Null)
}

pub(crate) fn theme(config: &Value, theme: &Theme) -> Theme {
    let mut result = theme.clone();
    let name = config
        .get("theme")
        .or_else(|| options(config).get("theme"))
        .and_then(Value::as_str)
        .unwrap_or("redux-color");
    if name.starts_with("redux") {
        result.font_family = "\"Recursive Variable\", arial, sans-serif".to_string();
        result.font_size = 14.0;
    }
    if matches!(name, "redux" | "redux-color") {
        let color = |key: &str, fallback: &str| {
            config
                .get("themeVariables")
                .and_then(|variables| variables.get(key))
                .and_then(Value::as_str)
                .unwrap_or(fallback)
                .to_string()
        };
        result.primary_color = color("mainBkg", "#ffffff");
        result.primary_border_color = color("nodeBorder", "#28253D");
        result.primary_text_color = color("primaryTextColor", "#28253D");
        result.text_color = result.primary_text_color.clone();
        result.line_color = color("lineColor", "#000000");
    }
    if let Some(variables) = config.get("themeVariables") {
        if let Some(family) = variables.get("fontFamily").and_then(Value::as_str) {
            result.font_family = family.to_string();
        }
        if let Some(size) = variables
            .get("fontSize")
            .and_then(|value| {
                value
                    .as_f64()
                    .map(|value| value as f32)
                    .or_else(|| value.as_str()?.trim_end_matches("px").parse::<f32>().ok())
            })
            .filter(|size| size.is_finite() && *size > 0.0)
        {
            result.font_size = size;
        }
    }
    result
}

pub(crate) fn node_style(graph: &crate::ir::Graph, id: &str) -> crate::ir::NodeStyle {
    use crate::ir::{NodeShape, NodeStyle};
    let mut style = NodeStyle::default();
    let Some(config) = graph.agentflow_config.as_ref() else {
        return style;
    };
    let name = config
        .get("theme")
        .or_else(|| options(config).get("theme"))
        .and_then(Value::as_str)
        .unwrap_or("redux-color");
    if !matches!(name, "redux" | "redux-color") {
        return style;
    }
    let Some(node) = graph.nodes.get(id) else {
        return style;
    };
    style.corner_radius = Some(
        config
            .get("themeVariables")
            .and_then(|v| v.get("radius"))
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite() && *value >= 0.0)
            .unwrap_or(12.0) as f32,
    );
    style.stroke_width = Some(2.0);
    if node.shape == NodeShape::CollapsedGroup {
        let container = container_style(graph, id);
        style.fill = container.fill;
        style.stroke = container.stroke;
        return style;
    }
    if name != "redux-color" {
        return style;
    }
    let slot = if graph
        .element_metadata
        .get(id)
        .and_then(|v| v.get("_agentflowKind"))
        .and_then(Value::as_str)
        == Some("connector")
    {
        5
    } else {
        match node.shape {
            NodeShape::Subroutine => 0,
            NodeShape::Diamond => 2,
            NodeShape::Parallelogram => 3,
            NodeShape::ReferenceDocument => 4,
            NodeShape::Hexagon => 6,
            _ => 1,
        }
    };
    let palette = |key: &str, defaults: &[&str]| -> Option<String> {
        if let Some(values) = config
            .get("themeVariables")
            .and_then(|v| v.get(key))
            .and_then(Value::as_array)
        {
            if values.is_empty() {
                return None;
            }
            return values[slot % values.len()].as_str().map(str::to_string);
        }
        Some(defaults[slot % defaults.len()].to_string())
    };
    style.stroke = palette("borderColorArray", &crate::usecase::BORDERS);
    if style.stroke.is_some() {
        style.fill = palette("bkgColorArray", &crate::usecase::BACKGROUNDS);
    }
    style
}

pub(crate) fn neo(config: &Value) -> bool {
    config
        .get("look")
        .or_else(|| options(config).get("look"))
        .and_then(Value::as_str)
        .unwrap_or("neo")
        == "neo"
}

pub(crate) fn node_size(
    shape: crate::ir::NodeShape,
    label: &crate::layout::TextBlock,
    config: &Value,
) -> Option<(f32, f32)> {
    match shape {
        crate::ir::NodeShape::CollapsedGroup => {
            Some(((label.width + 16.0).max(80.0), label.height + 44.0))
        }
        crate::ir::NodeShape::Hexagon => {
            let height = label.height + if neo(config) { 70.0 } else { 15.0 };
            let inset = height / if neo(config) { 3.5 } else { 4.0 };
            Some((
                label.width + inset * 2.0 + if neo(config) { 32.0 } else { 15.0 },
                height,
            ))
        }
        _ => None,
    }
}

pub(crate) fn container_style(graph: &crate::ir::Graph, id: &str) -> crate::ir::NodeStyle {
    let mut style = crate::ir::NodeStyle::default();
    let Some(config) = graph.agentflow_config.as_ref() else {
        return style;
    };
    let name = config
        .get("theme")
        .or_else(|| options(config).get("theme"))
        .and_then(Value::as_str)
        .unwrap_or("redux-color");
    if name != "redux-color" {
        return style;
    }
    let index = graph
        .element_metadata
        .get(id)
        .and_then(|value| value.get("_containerIndex"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let palette = |key: &str, defaults: &[&str], slot: usize| -> Option<String> {
        if let Some(values) = config
            .get("themeVariables")
            .and_then(|v| v.get(key))
            .and_then(Value::as_array)
        {
            if values.is_empty() {
                return None;
            }
            return values[slot % values.len()].as_str().map(str::to_string);
        }
        Some(defaults[slot % defaults.len()].to_string())
    };
    let length = config
        .get("themeVariables")
        .and_then(|v| v.get("borderColorArray"))
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(12);
    if length == 0 {
        return style;
    }
    let slot = (7 + index % length.saturating_sub(7).max(1)) % length;
    style.stroke_width = Some(0.75);
    style.stroke = palette("borderColorArray", &crate::usecase::BORDERS, slot);
    if style.stroke.is_some() {
        style.fill = palette("bkgColorArray", &crate::usecase::BACKGROUNDS, slot);
    }
    style
}
