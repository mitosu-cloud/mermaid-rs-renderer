//! Class interface semantics and measurement, following Mermaid's classBox.
use crate::{
    ir::{DiagramKind, Graph, Node, NodeShape, NodeStyle},
    layout::TextBlock,
    theme::Theme,
};

pub(crate) fn enabled(graph: &Graph) -> bool {
    graph.kind == DiagramKind::Class
        && graph
            .appearance_config
            .get("_lollipop")
            .and_then(|v| v.as_bool())
            == Some(true)
}

pub(crate) fn sections(label: &TextBlock) -> [Vec<String>; 3] {
    let mut groups: [Vec<String>; 3] = Default::default();
    let mut index = 0;
    for line in &label.lines {
        let text = line.text();
        if text.trim() == "---" {
            index = (index + 1).min(2);
        } else if !text.trim().is_empty() {
            groups[index].push(text.into_owned());
        }
    }
    groups
}

pub(crate) fn heights(groups: &[Vec<String>; 3], theme: &Theme) -> [f32; 3] {
    let line = theme.font_size * 1.5;
    std::array::from_fn(|i| {
        if groups[i].is_empty() {
            18.0
        } else {
            groups[i].len() as f32 * line + 24.0
        }
    })
}

pub(crate) fn size(node: &Node, label: &TextBlock, theme: &Theme) -> (f32, f32) {
    if node.shape == NodeShape::Text {
        return (label.width + 32.0, theme.font_size * 1.5 + 24.0);
    }
    if node.shape == NodeShape::Note {
        return (label.width + 24.0, label.height + 24.0);
    }
    let groups = sections(label);
    let width = |text: &str, bold| {
        crate::text_metrics::measure_styled_text_width(
            text,
            theme.font_size,
            &theme.font_family,
            bold,
            false,
        )
        .unwrap_or(text.len() as f32 * theme.font_size * 0.6)
    };
    let title = groups[0]
        .iter()
        .map(|s| width(s, true))
        .fold(0.0_f32, f32::max);
    let body = groups[1..]
        .iter()
        .flatten()
        .map(|s| width(s, false))
        .fold(0.0_f32, f32::max);
    // textHelper centers the title at x=0 and left-aligns members at x=0;
    // classBox measures the union of those two ranges before adding padding.
    (
        title / 2.0 + (title / 2.0).max(body) + 24.0,
        heights(&groups, theme).iter().sum(),
    )
}

pub(crate) fn node_style(graph: &Graph, id: &str) -> NodeStyle {
    let mut style = NodeStyle::default();
    if !crate::usecase::redux(&graph.appearance_config)
        || graph
            .nodes
            .get(id)
            .is_none_or(|n| n.shape != NodeShape::Rectangle)
    {
        return style;
    }
    let mut classes: Vec<_> = graph
        .nodes
        .values()
        .filter(|n| n.shape == NodeShape::Rectangle)
        .collect();
    classes.sort_by_key(|n| graph.node_order.get(&n.id).copied().unwrap_or(usize::MAX));
    let slot = classes.iter().position(|n| n.id == id).unwrap_or(0);
    style.fill = Some(crate::usecase::BACKGROUNDS[slot % crate::usecase::BACKGROUNDS.len()].into());
    style.stroke = Some(crate::usecase::BORDERS[slot % crate::usecase::BORDERS.len()].into());
    style.stroke_width = Some(2.0);
    style
}
