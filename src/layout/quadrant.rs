use std::collections::BTreeMap;

use crate::config::LayoutConfig;
use crate::ir::Graph;
use crate::theme::Theme;

use super::{DiagramData, Layout, QuadrantLayout, QuadrantPointLayout, TextBlock, TextLine};

fn label(text: &Option<String>, font_size: f32, theme: &Theme) -> Option<TextBlock> {
    text.as_ref()
        .filter(|text| !text.is_empty())
        .map(|text| TextBlock {
            lines: vec![TextLine::plain(text.clone())],
            width: crate::text_metrics::measure_text_width_with_kerning(
                text,
                font_size,
                &theme.font_family,
            )
            .unwrap_or_else(|| {
                crate::text_metrics::get_computed_text_length(text, font_size, &theme.font_family)
            }),
            height: font_size,
        })
}

pub(super) fn compute_quadrant_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> Layout {
    let options = &config.quadrant_chart;
    let title = label(&graph.quadrant.title, options.title_font_size, theme);
    let x_left = label(
        &graph.quadrant.x_axis_left,
        options.x_axis_label_font_size,
        theme,
    );
    let x_right = label(
        &graph.quadrant.x_axis_right,
        options.x_axis_label_font_size,
        theme,
    );
    let y_bottom = label(
        &graph.quadrant.y_axis_bottom,
        options.y_axis_label_font_size,
        theme,
    );
    let y_top = label(
        &graph.quadrant.y_axis_top,
        options.y_axis_label_font_size,
        theme,
    );
    let title_space = if title.is_some() {
        options.title_font_size + options.title_padding * 2.0
    } else {
        0.0
    };
    let x_space = if x_left.is_some() || x_right.is_some() {
        options.x_axis_label_font_size + options.x_axis_label_padding * 2.0
    } else {
        0.0
    };
    let y_space = if y_bottom.is_some() || y_top.is_some() {
        options.y_axis_label_font_size + options.y_axis_label_padding * 2.0
    } else {
        0.0
    };
    // Charts with data points always place the X-axis below the plot.
    let axis_top = graph.quadrant.points.is_empty() && options.x_axis_position != "bottom";
    let axis_right = options.y_axis_position == "right";
    let width = options.chart_width.max(1.0);
    let height = options.chart_height.max(1.0);
    let grid_x = options.quadrant_padding + if axis_right { 0.0 } else { y_space };
    let grid_y = options.quadrant_padding + title_space + if axis_top { x_space } else { 0.0 };
    let grid_width = (width - options.quadrant_padding * 2.0 - y_space).max(0.0);
    let grid_height = (height - options.quadrant_padding * 2.0 - x_space - title_space).max(0.0);
    let points = graph
        .quadrant
        .points
        .iter()
        .rev()
        .map(|point| {
            let class = point
                .class_name
                .as_ref()
                .and_then(|name| graph.quadrant.classes.get(name))
                .cloned()
                .unwrap_or_default();
            QuadrantPointLayout {
                label: label(
                    &Some(point.label.clone()),
                    options.point_label_font_size,
                    theme,
                )
                .unwrap(),
                x: grid_x + point.x * grid_width,
                y: grid_y + (1.0 - point.y) * grid_height,
                // The reference's invalid default HSL fill inherits the root text fill.
                color: point
                    .style
                    .color
                    .as_ref()
                    .or(class.color.as_ref())
                    .cloned()
                    .unwrap_or_else(|| theme.text_color.clone()),
                radius: point
                    .style
                    .radius
                    .or(class.radius)
                    .unwrap_or(options.point_radius),
                stroke_color: point
                    .style
                    .stroke_color
                    .as_ref()
                    .or(class.stroke_color.as_ref())
                    .cloned()
                    .unwrap_or_else(|| theme.text_color.clone()),
                stroke_width: point
                    .style
                    .stroke_width
                    .or(class.stroke_width)
                    .unwrap_or(0.0),
            }
        })
        .collect();
    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width,
        height,
        acc_title: None,
        acc_descr: None,
        diagram: DiagramData::Quadrant(QuadrantLayout {
            title,
            title_x: width / 2.0,
            title_y: options.title_padding,
            x_axis_left: x_left,
            x_axis_right: x_right,
            y_axis_bottom: y_bottom,
            y_axis_top: y_top,
            quadrant_labels: std::array::from_fn(|index| {
                label(
                    &graph.quadrant.quadrant_labels[index],
                    options.quadrant_label_font_size,
                    theme,
                )
            }),
            points,
            grid_x,
            grid_y,
            grid_width,
            grid_height,
            x_axis_y: if axis_top {
                options.x_axis_label_padding + title_space
            } else {
                options.x_axis_label_padding + grid_y + grid_height + options.quadrant_padding
            },
            y_axis_x: if axis_right {
                options.y_axis_label_padding + grid_x + grid_width + options.quadrant_padding
            } else {
                options.y_axis_label_padding
            },
            center_quadrant_labels: graph.quadrant.points.is_empty(),
        }),
    }
}
