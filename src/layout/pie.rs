use std::collections::BTreeMap;

use crate::config::LayoutConfig;
use crate::ir::Graph;
use crate::theme::Theme;

use super::{
    DiagramData, Layout, PieData, PieLegendItem, PieSliceLayout, PieTitleLayout, TextBlock,
    TextLine,
};

fn measure_pie_label(text: &str, font_size: f32, theme: &Theme) -> TextBlock {
    let width =
        crate::text_metrics::measure_text_width_with_kerning(text, font_size, &theme.font_family)
            .unwrap_or_else(|| {
                crate::text_metrics::get_computed_text_length(text, font_size, &theme.font_family)
            });
    TextBlock {
        lines: vec![TextLine::plain(text.to_string())],
        // Chromium quantizes SVG text measurements to CSS layout units.
        width: (width * 64.0).ceil() / 64.0,
        height: font_size,
    }
}

#[allow(dead_code)]
fn format_pie_value(value: f32) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if (rounded - rounded.round()).abs() < 0.001 {
        format!("{:.0}", rounded)
    } else {
        format!("{:.2}", rounded)
    }
}

pub(super) fn compute_pie_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    let pie_cfg = &config.pie;
    let mut slices = Vec::new();
    let mut legend = Vec::new();
    let title_block = graph
        .pie_title
        .as_ref()
        .map(|title| measure_pie_label(title, theme.pie_title_text_size, theme));
    let total: f32 = graph
        .pie_slices
        .iter()
        .map(|slice| slice.value.max(0.0))
        .sum();
    let visible: Vec<_> = graph
        .pie_slices
        .iter()
        .enumerate()
        .filter(|(_, slice)| {
            total > 0.0 && slice.value > 0.0 && slice.value / total * 100.0 >= pie_cfg.min_percent
        })
        .collect();
    // D3 lays out the retained slices in source order and normalizes their
    // angles to a full circle, even when sub-1% values are only in the legend.
    let visible_total: f32 = visible.iter().map(|(_, slice)| slice.value).sum();
    let mut angle = -std::f32::consts::FRAC_PI_2;
    for (index, slice) in visible {
        let span = slice.value / visible_total * std::f32::consts::TAU;
        slices.push(PieSliceLayout {
            label: measure_pie_label(&slice.label, theme.pie_section_text_size, theme),
            value: slice.value,
            start_angle: angle,
            end_angle: angle + span,
            color: theme.pie_colors[index % theme.pie_colors.len()].clone(),
        });
        angle += span;
    }

    let mut legend_width: f32 = 0.0;
    let mut legend_items: Vec<(TextBlock, String)> = Vec::new();
    for (index, slice) in graph.pie_slices.iter().enumerate() {
        let value_text = format_pie_value(slice.value);
        let label_text = if graph.pie_show_data {
            format!("{} [{}]", slice.label, value_text)
        } else {
            slice.label.clone()
        };
        let label = measure_pie_label(&label_text, theme.pie_legend_text_size, theme);
        legend_width = legend_width.max(label.width);
        let color = theme.pie_colors[index % theme.pie_colors.len()].clone();
        legend_items.push((label, color));
    }

    let legend_item_height = pie_cfg.legend_rect_size + pie_cfg.legend_spacing;
    let legend_offset = legend_item_height * legend_items.len() as f32 / 2.0;

    let height = pie_cfg.height.max(1.0);
    let pie_width = height;
    let radius = (pie_width.min(height) / 2.0 - pie_cfg.margin).max(1.0);
    let center_x = pie_width / 2.0;
    let center_y = height / 2.0;
    let legend_x = center_x + pie_cfg.legend_horizontal_multiplier * pie_cfg.legend_rect_size;

    for (idx, (label, color)) in legend_items.into_iter().enumerate() {
        let vertical = idx as f32 * legend_item_height - legend_offset;
        legend.push(PieLegendItem {
            x: legend_x,
            y: center_y + vertical,
            label,
            color,
            marker_size: pie_cfg.legend_rect_size,
            value: graph.pie_slices[idx].value,
        });
    }

    let width = pie_width
        + pie_cfg.margin
        + pie_cfg.legend_rect_size
        + pie_cfg.legend_spacing
        + legend_width;
    let title_layout = title_block.map(|text| PieTitleLayout {
        x: center_x,
        y: center_y - (height - 50.0) / 2.0,
        text,
    });

    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width: width.max(200.0),
        height: height.max(1.0),
        acc_title: None,
        acc_descr: None,
        diagram: DiagramData::Pie(PieData {
            slices,
            legend,
            center: (center_x, center_y),
            radius,
            title: title_layout,
        }),
    }
}
