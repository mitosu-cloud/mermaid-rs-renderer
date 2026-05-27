use std::collections::BTreeMap;

use crate::config::LayoutConfig;
use crate::ir::{Graph, QuadrantPointStyle};
use crate::theme::Theme;

use super::text::measure_label;
use super::{DiagramData, Layout, QuadrantLayout, QuadrantPointLayout, TextBlock};

pub(super) fn compute_quadrant_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> Layout {
    let quadrant_config = &config.quadrant;
    let chart_width = quadrant_config.chart_width;
    let chart_height = quadrant_config.chart_height;
    let has_points = !graph.quadrant.points.is_empty();
    let show_x_axis = quadrant_config.show_x_axis
        && (graph.quadrant.x_axis_left.is_some() || graph.quadrant.x_axis_right.is_some());
    let show_y_axis = quadrant_config.show_y_axis
        && (graph.quadrant.y_axis_bottom.is_some() || graph.quadrant.y_axis_top.is_some());
    let show_title = quadrant_config.show_title && graph.quadrant.title.is_some();
    let x_axis_position = if has_points {
        "bottom"
    } else {
        quadrant_config.x_axis_position.as_str()
    };

    let x_axis_space = if show_x_axis {
        quadrant_config.x_axis_label_padding * 2.0 + quadrant_config.x_axis_label_font_size
    } else {
        0.0
    };
    let x_axis_top = if x_axis_position == "top" {
        x_axis_space
    } else {
        0.0
    };
    let x_axis_bottom = if x_axis_position == "bottom" {
        x_axis_space
    } else {
        0.0
    };
    let y_axis_space = if show_y_axis {
        quadrant_config.y_axis_label_padding * 2.0 + quadrant_config.y_axis_label_font_size
    } else {
        0.0
    };
    let y_axis_left = if quadrant_config.y_axis_position == "left" {
        y_axis_space
    } else {
        0.0
    };
    let y_axis_right = if quadrant_config.y_axis_position == "right" {
        y_axis_space
    } else {
        0.0
    };
    let title_space = if show_title {
        quadrant_config.title_font_size + quadrant_config.title_padding * 2.0
    } else {
        0.0
    };

    let grid_x = quadrant_config.quadrant_padding + y_axis_left;
    let grid_y = quadrant_config.quadrant_padding + x_axis_top + title_space;
    let grid_width =
        chart_width - quadrant_config.quadrant_padding * 2.0 - y_axis_left - y_axis_right;
    let grid_height = chart_height
        - quadrant_config.quadrant_padding * 2.0
        - x_axis_top
        - x_axis_bottom
        - title_space;

    let measure_with_size = |text: &str, font_size: f32| {
        let mut sized_theme = theme.clone();
        sized_theme.font_size = font_size;
        measure_label(text, &sized_theme, config)
    };

    let title = graph
        .quadrant
        .title
        .as_ref()
        .filter(|_| show_title)
        .map(|t| measure_with_size(t, quadrant_config.title_font_size));

    // Measure axis labels
    let x_left = graph
        .quadrant
        .x_axis_left
        .as_ref()
        .filter(|_| show_x_axis)
        .map(|t| measure_with_size(t, quadrant_config.x_axis_label_font_size));
    let x_right = graph
        .quadrant
        .x_axis_right
        .as_ref()
        .filter(|_| show_x_axis)
        .map(|t| measure_with_size(t, quadrant_config.x_axis_label_font_size));
    let y_bottom = graph
        .quadrant
        .y_axis_bottom
        .as_ref()
        .filter(|_| show_y_axis)
        .map(|t| measure_with_size(t, quadrant_config.y_axis_label_font_size));
    let y_top = graph
        .quadrant
        .y_axis_top
        .as_ref()
        .filter(|_| show_y_axis)
        .map(|t| measure_with_size(t, quadrant_config.y_axis_label_font_size));

    // Measure quadrant labels
    let q_labels: [Option<TextBlock>; 4] = [
        graph.quadrant.quadrant_labels[0]
            .as_ref()
            .map(|t| measure_with_size(t, quadrant_config.quadrant_label_font_size)),
        graph.quadrant.quadrant_labels[1]
            .as_ref()
            .map(|t| measure_with_size(t, quadrant_config.quadrant_label_font_size)),
        graph.quadrant.quadrant_labels[2]
            .as_ref()
            .map(|t| measure_with_size(t, quadrant_config.quadrant_label_font_size)),
        graph.quadrant.quadrant_labels[3]
            .as_ref()
            .map(|t| measure_with_size(t, quadrant_config.quadrant_label_font_size)),
    ];

    // Layout points
    let points: Vec<QuadrantPointLayout> = graph
        .quadrant
        .points
        .iter()
        .map(|p| {
            let px = grid_x + p.x.clamp(0.0, 1.0) * grid_width;
            let py = grid_y + (1.0 - p.y.clamp(0.0, 1.0)) * grid_height;
            let mut style = p
                .class_name
                .as_ref()
                .and_then(|class_name| graph.quadrant.point_classes.get(class_name))
                .cloned()
                .unwrap_or_default();
            merge_quadrant_point_style(&mut style, &p.style);
            let color = style
                .color
                .clone()
                .unwrap_or_else(|| theme.quadrant.point_fill.clone());
            QuadrantPointLayout {
                label: measure_with_size(&p.label, quadrant_config.point_label_font_size),
                x: px,
                y: py,
                stroke_color: style
                    .stroke_color
                    .clone()
                    .unwrap_or_else(|| theme.quadrant.point_fill.clone()),
                stroke_width: style
                    .stroke_width
                    .clone()
                    .unwrap_or_else(|| "0px".to_string()),
                color,
                radius: style.radius.unwrap_or(quadrant_config.point_radius),
            }
        })
        .collect();

    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width: chart_width,
        height: chart_height,
        acc_title: None,
        acc_descr: None,
        diagram: DiagramData::Quadrant(QuadrantLayout {
            title,
            width: chart_width,
            height: chart_height,
            use_max_width: quadrant_config.use_max_width,
            x_axis_left: x_left,
            x_axis_right: x_right,
            y_axis_bottom: y_bottom,
            y_axis_top: y_top,
            quadrant_labels: q_labels,
            points,
            grid_x,
            grid_y,
            grid_width,
            grid_height,
        }),
    }
}

fn merge_quadrant_point_style(base: &mut QuadrantPointStyle, overlay: &QuadrantPointStyle) {
    if overlay.radius.is_some() {
        base.radius = overlay.radius;
    }
    if overlay.color.is_some() {
        base.color = overlay.color.clone();
    }
    if overlay.stroke_color.is_some() {
        base.stroke_color = overlay.stroke_color.clone();
    }
    if overlay.stroke_width.is_some() {
        base.stroke_width = overlay.stroke_width.clone();
    }
}
