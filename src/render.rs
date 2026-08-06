use crate::config::LayoutConfig;
#[cfg(feature = "png")]
use crate::config::RenderConfig;
use crate::layout::label_placement::{
    CLASS_ENDPOINT_LABEL_FONT_SIZE, edge_endpoint_label_position, edge_label_padding,
    endpoint_label_padding,
};
use crate::layout::{
    C4BoundaryLayout, C4Layout, C4RelLayout, C4ShapeLayout, CynefinLayout, DiagramData,
    ErrorLayout, EventModelingLayout, GitGraphLayout, JourneyLayout, Layout, PacketLayout, PieData,
    SankeyLayout, TextBlock, TextLine, VennLayout,
};
use crate::text_metrics;
use crate::theme::{Theme, adjust_color};
use anyhow::Result;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::Path;

const FLOWCHART_ICON_ASSET_SIZE: f32 = 48.0;
const FLOWCHART_ICON_LABEL_PADDING: f32 = 8.0;
const FLOWCHART_ICON_LABEL_TOP_INSET: f32 = 2.0;
const FLOWCHART_ICON_CIRCLE_PADDING: f32 = 20.0;
const FLOWCHART_ICON_SQUARE_PADDING: f32 = 4.0;
const FLOWCHART_DIVIDED_RECT_HEADER_RATIO: f32 = 0.2;
const FLOWCHART_BANG_BBOX_SCALE: f32 = 1.25;
const FLOWCHART_WINDOW_PANE_OFFSET: f32 = 10.0;
const SEQUENCE_TEXT_LINE_HEIGHT: f32 = 1.1875;

fn flowchart_tilted_cylinder_rx(height: f32) -> f32 {
    let ry = height / 2.0;
    ry / (2.5 + height / 50.0)
}

fn flowchart_wave_document_amplitude(total_height: f32) -> f32 {
    total_height / 10.0
}

fn flowchart_cylinder_ry(width: f32) -> f32 {
    let rx = width / 2.0;
    rx / (2.5 + width / 50.0)
}

fn flowchart_divided_rect_offset(total_height: f32) -> f32 {
    total_height * FLOWCHART_DIVIDED_RECT_HEADER_RATIO / (1.0 + FLOWCHART_DIVIDED_RECT_HEADER_RATIO)
}

fn fit_dimensions_to_preferred_ratio(
    width: f32,
    height: f32,
    preferred_ratio: Option<f32>,
) -> (f32, f32) {
    let mut width = width.max(1.0);
    let mut height = height.max(1.0);
    let Some(target_ratio) = preferred_ratio else {
        return (width, height);
    };
    if !target_ratio.is_finite() || target_ratio <= 0.0 {
        return (width, height);
    }
    let current_ratio = width / height;
    if (current_ratio - target_ratio).abs() < 1e-6 {
        return (width, height);
    }
    if current_ratio < target_ratio {
        width = height * target_ratio;
    } else {
        height = width / target_ratio;
    }
    (width.max(1.0), height.max(1.0))
}

fn sankey_content_y_bounds(layout: &SankeyLayout) -> (f32, f32) {
    let mut min_y = 0.0_f32;
    let mut max_y = layout.height;
    let label_dy = if layout.show_values { 0.0 } else { 14.0 * 0.35 };
    // Browser getBBox for Mermaid's 14px Trebuchet labels contributes about
    // 13px above and 3px below the text baseline.
    const LABEL_ASCENT: f32 = 13.0;
    const LABEL_DESCENT: f32 = 3.0;
    for node in &layout.nodes {
        let baseline = node.y + node.height / 2.0 + label_dy;
        min_y = min_y.min(baseline - LABEL_ASCENT);
        max_y = max_y.max(baseline + LABEL_DESCENT);
    }
    (min_y, max_y)
}

fn sankey_content_viewbox_y(layout: &SankeyLayout) -> f32 {
    sankey_content_y_bounds(layout).0
}

fn sankey_content_viewbox_height(layout: &SankeyLayout) -> f32 {
    let (min_y, max_y) = sankey_content_y_bounds(layout);
    max_y - min_y
}

fn edge_dom_id(edge_idx: usize) -> String {
    format!("edge-{edge_idx}")
}

fn sequence_frame_border_color(theme: &Theme) -> &str {
    theme.sequence_actor_border.as_str()
}

const STATE_FORK_JOIN_RENDER_HEIGHT: f32 = 10.0;

pub fn render_svg(layout: &Layout, theme: &Theme, config: &LayoutConfig) -> String {
    let mut svg = String::new();
    let state_font_size = if layout.kind == crate::ir::DiagramKind::State {
        theme.font_size * 0.85
    } else {
        theme.font_size
    };
    let graph_title = if let DiagramData::Graph { title, .. } = &layout.diagram {
        title.as_deref()
    } else {
        None
    };
    let graph_title_extra = if matches!(
        layout.kind,
        crate::ir::DiagramKind::Er | crate::ir::DiagramKind::Class
    ) && graph_title.is_some()
    {
        48.0
    } else {
        0.0
    };
    let (width, height, viewbox_x, viewbox_y, viewbox_width, viewbox_height) =
        if let DiagramData::Error(error) = &layout.diagram {
            (
                error.render_width,
                error.render_height,
                0.0,
                0.0,
                error.viewbox_width,
                error.viewbox_height,
            )
        } else if layout.kind == crate::ir::DiagramKind::Requirement {
            let pad_x = config.requirement.render_padding_x;
            let pad_y = config.requirement.render_padding_y;
            let mut width = layout.width + pad_x * 2.0;
            let mut height = layout.height + pad_y * 2.0;
            width = width.max(1.0);
            height = height.max(1.0);
            (width, height, 0.0, 0.0, width, height)
        } else if let DiagramData::C4(c4) = &layout.diagram {
            let width = layout.width.max(1.0);
            let height = layout.height.max(1.0);
            (
                width,
                height,
                c4.viewbox_x,
                c4.viewbox_y,
                c4.viewbox_width,
                c4.viewbox_height,
            )
        } else if let DiagramData::GitGraph(gitgraph) = &layout.diagram {
            let width = layout.width.max(1.0);
            let height = layout.height.max(1.0);
            let viewbox_x = -gitgraph.offset_x;
            let viewbox_y = -gitgraph.offset_y;
            (
                width,
                height,
                viewbox_x,
                viewbox_y,
                gitgraph.width,
                gitgraph.height,
            )
        } else if let DiagramData::Journey(journey) = &layout.diagram {
            let width = journey.width.max(1.0);
            let height = journey.height.max(1.0);
            (width, height, 0.0, -25.0, width, height)
        } else if layout.kind == crate::ir::DiagramKind::Mindmap {
            let pad = config.mindmap.padding;
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            for node in layout.nodes.values() {
                min_x = min_x.min(node.x);
                min_y = min_y.min(node.y);
                max_x = max_x.max(node.x + node.width);
                max_y = max_y.max(node.y + node.height);
            }
            if min_x == f32::MAX {
                min_x = 0.0;
                max_x = 1.0;
            }
            if min_y == f32::MAX {
                min_y = 0.0;
                max_y = 1.0;
            }
            let width = (max_x - min_x + pad * 2.0).max(1.0);
            let height = (max_y - min_y + pad * 2.0).max(1.0);
            let viewbox_x = min_x - pad;
            let viewbox_y = min_y - pad;
            (width, height, viewbox_x, viewbox_y, width, height)
        } else if let DiagramData::Timeline(tl) = &layout.diagram {
            // Match JS viewBox: origin at (100, -61) with title, (100, 0) without.
            let has_title = tl.title.is_some();
            let vb_x = 100.0_f32;
            let vb_y = if has_title { -61.0_f32 } else { 0.0 };
            let vb_w = tl.width;
            let vb_h = tl.height - vb_y;
            (tl.width, tl.height, vb_x, vb_y, vb_w, vb_h)
        } else if let DiagramData::Ishikawa(ish) = &layout.diagram {
            // Ishikawa uses negative x (head at x=0, spine extends left).
            // The layout node stores actual min_x/min_y in its position.
            let node = layout.nodes.values().next();
            let (min_x, min_y) = node.map(|n| (n.x, n.y)).unwrap_or((0.0, 0.0));
            (ish.width, ish.height, min_x, min_y, ish.width, ish.height)
        } else if let DiagramData::Sequence(seq) = &layout.diagram {
            let width = layout.width.max(1.0);
            let height = layout.height.max(1.0);
            // Sequence frames (critical/loop/alt/opt) and their labelBox
            // polygons can extend left of x=0. Push viewBox-x negative so the
            // frame's left border isn't clipped (mirrors mermaid JS, which
            // sets a negative viewBox-x for the same reason).
            let min_frame_x = seq
                .frames
                .iter()
                .map(|f| f.x)
                .fold(0.0_f32, |acc, x| acc.min(x));
            if min_frame_x < 0.0 {
                let pad = 8.0_f32;
                let viewbox_x = min_frame_x - pad;
                let viewbox_width = (width - viewbox_x).max(1.0);
                (viewbox_width, height, viewbox_x, 0.0, viewbox_width, height)
            } else {
                (width, height, 0.0, 0.0, width, height)
            }
        } else if let DiagramData::Sankey(sankey) = &layout.diagram {
            let width = sankey.width.max(1.0);
            let height = sankey_content_viewbox_height(sankey).max(1.0);
            let viewbox_y = sankey_content_viewbox_y(sankey);
            (width, height, 0.0, viewbox_y, width, height)
        } else if let DiagramData::TreeView(tree_view) = &layout.diagram {
            let width = tree_view.width.max(1.0);
            let height = tree_view.height.max(1.0);
            (width, height, -0.5, 0.0, width, height)
        } else if let DiagramData::EventModeling(eventmodeling) = &layout.diagram {
            (
                eventmodeling.width.max(1.0),
                eventmodeling.height.max(1.0),
                eventmodeling.viewbox_x,
                eventmodeling.viewbox_y,
                eventmodeling.viewbox_width.max(1.0),
                eventmodeling.viewbox_height.max(1.0),
            )
        } else if let DiagramData::Cynefin(cynefin) = &layout.diagram {
            (
                cynefin.width.max(1.0),
                cynefin.height.max(1.0),
                0.0,
                0.0,
                cynefin.width.max(1.0),
                cynefin.height.max(1.0),
            )
        } else {
            let width = layout.width.max(1.0);
            let height = layout.height.max(1.0);
            if graph_title_extra > 0.0 {
                (
                    width,
                    height + graph_title_extra,
                    0.0,
                    -graph_title_extra,
                    width,
                    height + graph_title_extra,
                )
            } else {
                (width, height, 0.0, 0.0, width, height)
            }
        };
    let seq_data = if let DiagramData::Sequence(s) = &layout.diagram {
        Some(s)
    } else {
        None
    };
    let is_sequence = seq_data.is_some();
    let is_state = layout.kind == crate::ir::DiagramKind::State;
    let is_class = layout.kind == crate::ir::DiagramKind::Class;
    let is_block = layout.kind == crate::ir::DiagramKind::Block;
    let is_architecture = layout.kind == crate::ir::DiagramKind::Architecture;
    let is_c4 = matches!(layout.diagram, DiagramData::C4(_));
    let is_venn = matches!(layout.diagram, DiagramData::Venn(_));
    let is_packet = matches!(layout.diagram, DiagramData::Packet(_));
    let is_sankey = matches!(layout.diagram, DiagramData::Sankey(_));
    let is_quadrant = matches!(layout.diagram, DiagramData::Quadrant(_));
    let is_tree_view = matches!(layout.diagram, DiagramData::TreeView(_));
    let is_ishikawa = matches!(layout.diagram, DiagramData::Ishikawa(_));
    let is_gitgraph = matches!(layout.diagram, DiagramData::GitGraph(_));
    let is_eventmodeling = matches!(layout.diagram, DiagramData::EventModeling(_));
    let is_cynefin = matches!(layout.diagram, DiagramData::Cynefin(_));
    let is_treemap = layout.kind == crate::ir::DiagramKind::Treemap;
    let has_links = is_c4
        || is_eventmodeling
        || layout.nodes.values().any(|node| node.link.is_some())
        || seq_data
            .iter()
            .flat_map(|s| s.footboxes.iter())
            .any(|node| node.link.is_some());

    let preferred_ratio = config
        .preferred_aspect_ratio
        .filter(|ratio| ratio.is_finite() && *ratio > 0.0);
    let (target_width, target_height) =
        fit_dimensions_to_preferred_ratio(width, height, preferred_ratio);

    let mut width_attr = target_width.to_string();
    let mut height_attr = target_height.to_string();
    let mut style_attr = String::new();
    let preferred_ratio_style = preferred_ratio
        .map(|ratio| format!("aspect-ratio: {:.6};", ratio))
        .unwrap_or_default();
    if !matches!(layout.diagram, DiagramData::Error(_)) {
        if let DiagramData::C4(c4) = &layout.diagram {
            if c4.use_max_width {
                width_attr = "100%".to_string();
                height_attr.clear();
                style_attr = format!(
                    " style=\"max-width: {:.3}px;{}\"",
                    viewbox_width, preferred_ratio_style
                );
            }
        } else if matches!(layout.diagram, DiagramData::GitGraph(_))
            && config.gitgraph.use_max_width
        {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(
                " style=\"max-width: {:.3}px; background-color: white;{}\"",
                viewbox_width, preferred_ratio_style
            );
        } else if matches!(layout.diagram, DiagramData::Journey(_)) {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(
                " style=\"max-width: {:.0}px; background-color: white;{}\"",
                viewbox_width, preferred_ratio_style
            );
        } else if layout.kind == crate::ir::DiagramKind::Mindmap && config.mindmap.use_max_width {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(
                " style=\"max-width: {:.3}px;{}\"",
                viewbox_width, preferred_ratio_style
            );
        } else if matches!(layout.diagram, DiagramData::Timeline(_)) {
            // Timeline: responsive width + white background (matching JS).
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(
                " style=\"max-width: {:.0}px; background-color: white;{}\"",
                viewbox_width, preferred_ratio_style
            );
        } else if matches!(layout.diagram, DiagramData::Ishikawa(_)) {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(
                " style=\"max-width: {:.3}px; background-color: white;{}\"",
                viewbox_width, preferred_ratio_style
            );
        } else if matches!(layout.diagram, DiagramData::TreeView(_)) {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(
                " style=\"max-width: {:.3}px; background-color: white;{}\"",
                viewbox_width, preferred_ratio_style
            );
        } else if let DiagramData::EventModeling(eventmodeling) = &layout.diagram
            && eventmodeling.use_max_width
        {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(
                " style=\"max-width: {:.3}px; background-color: white;{}\"",
                viewbox_width, preferred_ratio_style
            );
        } else if let DiagramData::Cynefin(cynefin) = &layout.diagram
            && cynefin.use_max_width
        {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(
                " style=\"max-width: {:.3}px; background-color: white;{}\"",
                viewbox_width, preferred_ratio_style
            );
        } else if is_treemap {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(
                " style=\"max-width: {:.0}px; background-color: white;{}\"",
                viewbox_width, preferred_ratio_style
            );
        } else if matches!(layout.diagram, DiagramData::XYChart(_)) {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(
                " style=\"max-width: {:.0}px; background-color: white;{}\"",
                viewbox_width, preferred_ratio_style
            );
        } else if let DiagramData::Quadrant(quadrant) = &layout.diagram
            && quadrant.use_max_width
        {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(
                " style=\"max-width: {:.0}px; background-color: white;{}\"",
                viewbox_width, preferred_ratio_style
            );
        } else if let DiagramData::Sankey(sankey) = &layout.diagram
            && sankey.use_max_width
        {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(
                " style=\"max-width: {:.0}px; background-color: white;{}\"",
                viewbox_width, preferred_ratio_style
            );
        } else if is_venn || is_packet {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(
                " style=\"max-width: {:.0}px; background-color: white;{}\"",
                viewbox_width, preferred_ratio_style
            );
        } else if layout.kind == crate::ir::DiagramKind::Pie && config.pie.use_max_width {
            width_attr = "100%".to_string();
            height_attr.clear();
            style_attr = format!(
                " style=\"max-width: {:.3}px;{}\"",
                viewbox_width, preferred_ratio_style
            );
        } else if !preferred_ratio_style.is_empty() {
            style_attr = format!(" style=\"{preferred_ratio_style}\"");
        }
    } else if !preferred_ratio_style.is_empty() {
        style_attr = format!(" style=\"{preferred_ratio_style}\"");
    }
    // Build accessibility ARIA attributes.
    let has_acc_title = layout.acc_title.is_some();
    let has_acc_descr = layout.acc_descr.is_some();
    let mut aria_attrs = String::new();
    if has_acc_title || has_acc_descr {
        aria_attrs.push_str(" role=\"img\"");
        let mut labelledby = Vec::new();
        if has_acc_title {
            labelledby.push("chart-title");
        }
        if has_acc_descr {
            labelledby.push("chart-desc");
        }
        aria_attrs.push_str(&format!(" aria-labelledby=\"{}\"", labelledby.join(" ")));
    } else if is_sankey {
        aria_attrs.push_str(" role=\"graphics-document document\" aria-roledescription=\"sankey\"");
    } else if is_tree_view {
        aria_attrs
            .push_str(" role=\"graphics-document document\" aria-roledescription=\"treeView\"");
    } else if is_eventmodeling {
        aria_attrs.push_str(
            " role=\"graphics-document document\" aria-roledescription=\"eventmodeling\"",
        );
    } else if is_cynefin {
        aria_attrs
            .push_str(" role=\"graphics-document document\" aria-roledescription=\"cynefin\"");
    } else if is_treemap {
        aria_attrs
            .push_str(" role=\"graphics-document document\" aria-roledescription=\"treemap\"");
    }

    let svg_id_attr = if is_sankey || is_tree_view || is_treemap || is_eventmodeling || is_cynefin {
        " id=\"my-svg\""
    } else {
        ""
    };
    let svg_class_attr = if is_treemap {
        " class=\"flowchart\""
    } else {
        ""
    };
    svg.push_str(&format!(
        "<svg{svg_id_attr} xmlns=\"http://www.w3.org/2000/svg\"{} width=\"{width_attr}\"{} viewBox=\"{viewbox_x} {viewbox_y} {viewbox_width} {viewbox_height}\"{style_attr}{svg_class_attr}{aria_attrs}>",
        if has_links || is_sankey || is_tree_view || is_treemap {
            " xmlns:xlink=\"http://www.w3.org/1999/xlink\""
        } else {
            ""
        },
        if height_attr.is_empty() {
            String::new()
        } else {
            format!(" height=\"{height_attr}\"")
        }
    ));
    svg.push_str(&svg_font_style_block(layout, theme, config));

    // Emit accessibility <title> and <desc> elements.
    if let Some(title) = &layout.acc_title {
        svg.push_str(&format!(
            "<title id=\"chart-title\">{}</title>",
            escape_xml(title)
        ));
    }
    if let Some(descr) = &layout.acc_descr {
        svg.push_str(&format!(
            "<desc id=\"chart-desc\">{}</desc>",
            escape_xml(descr)
        ));
    }

    if matches!(layout.diagram, DiagramData::Error(_)) {
        svg.push_str(&error_style_block(theme));
    }

    // Timeline, GitGraph, Ishikawa, XYChart, QuadrantChart, Venn, Sankey, and Packet supply their own background.
    if !matches!(
        layout.diagram,
        DiagramData::Timeline(_)
            | DiagramData::GitGraph(_)
            | DiagramData::Ishikawa(_)
            | DiagramData::XYChart(_)
            | DiagramData::Quadrant(_)
            | DiagramData::Venn(_)
            | DiagramData::Sankey(_)
            | DiagramData::TreeView(_)
            | DiagramData::EventModeling(_)
            | DiagramData::Cynefin(_)
            | DiagramData::Packet(_)
    ) && !is_treemap
    {
        svg.push_str(&format!(
            "<rect x=\"{viewbox_x}\" y=\"{viewbox_y}\" width=\"{viewbox_width}\" height=\"{viewbox_height}\" fill=\"{}\"/>",
            theme.background
        ));
    }
    if matches!(
        layout.kind,
        crate::ir::DiagramKind::Er | crate::ir::DiagramKind::Class
    ) && let Some(title) = graph_title
    {
        let title_class = if layout.kind == crate::ir::DiagramKind::Class {
            "classDiagramTitleText"
        } else {
            "erDiagramTitleText"
        };
        svg.push_str(&format!(
            "<text text-anchor=\"middle\" x=\"{:.2}\" y=\"-25\" class=\"{}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
            viewbox_x + viewbox_width / 2.0,
            title_class,
            normalize_font_family(&theme.font_family),
            theme.font_size,
            theme.primary_text_color,
            escape_xml(title)
        ));
    }

    if let DiagramData::C4(ref c4) = layout.diagram {
        svg.push_str(&render_c4(c4, config));
        svg.push_str("</svg>");
        return svg;
    }

    let default_edge_stroke = default_edge_stroke_for_kind(layout.kind, theme);
    let mut colors = Vec::new();
    colors.push(default_edge_stroke.clone());
    for edge in &layout.edges {
        if let Some(color) = &edge.override_style.stroke
            && !colors.contains(color)
        {
            colors.push(color.clone());
        }
    }
    let mut color_ids: HashMap<String, usize> = HashMap::new();
    for (idx, color) in colors.iter().enumerate() {
        color_ids.insert(color.clone(), idx);
    }

    // Diagram-specific renderers skip generic flowchart markers unless they
    // still reference the shared marker ids.
    let is_timeline = matches!(layout.diagram, DiagramData::Timeline(_));
    let is_xychart = matches!(layout.diagram, DiagramData::XYChart(_));
    let architecture_needs_markers = is_architecture
        && layout
            .edges
            .iter()
            .any(|edge| edge.arrow_start || edge.arrow_end);
    if is_timeline
        || is_xychart
        || is_quadrant
        || is_venn
        || is_sankey
        || is_packet
        || is_tree_view
        || is_ishikawa
        || is_gitgraph
        || is_treemap
        || is_eventmodeling
        || is_cynefin
        || (is_architecture && !architecture_needs_markers)
    {
        // Jump past generic marker defs.
    } else {
        svg.push_str("<defs>");
        for color in &colors {
            let idx = color_ids.get(color).copied().unwrap_or(0);
            let (point_ref_x, point_size) = if is_block { ("6", "12") } else { ("5", "8") };
            let start_size = if is_block { "12" } else { "8" };
            svg.push_str(&format!(
            "<marker id=\"arrow-{idx}\" viewBox=\"0 0 10 10\" refX=\"{point_ref_x}\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"{point_size}\" markerHeight=\"{point_size}\" orient=\"auto\"><path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"1,0\"/></marker>",
            color, color
        ));
            svg.push_str(&format!(
            "<marker id=\"arrow-start-{idx}\" viewBox=\"0 0 10 10\" refX=\"4.5\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"{start_size}\" markerHeight=\"{start_size}\" orient=\"auto\"><path d=\"M 0 5 L 10 10 L 10 0 z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"1,0\"/></marker>",
            color, color
        ));
            if is_sequence {
                svg.push_str(&format!(
                "<marker id=\"arrow-seq-{idx}\" viewBox=\"-1 0 12 10\" refX=\"7.9\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"12\" markerHeight=\"12\" orient=\"auto-start-reverse\"><path d=\"M -1 0 L 10 5 L 0 10 z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"1,0\"/></marker>",
                color,
                color
            ));
                svg.push_str(&format!(
                "<marker id=\"arrow-start-seq-{idx}\" viewBox=\"-1 0 12 10\" refX=\"2.1\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"12\" markerHeight=\"12\" orient=\"auto\"><path d=\"M 11 0 L 0 5 L 11 10 z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"1,0\"/></marker>",
                color,
                color
            ));
                // Cross marker for -x / --x arrows
                svg.push_str(&format!(
                "<marker id=\"cross-seq-{idx}\" viewBox=\"0 0 8 9\" refX=\"4\" refY=\"4.5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"15\" markerHeight=\"8\" orient=\"auto\"><path fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" d=\"M 1,2 L 6,7 M 6,2 L 1,7\" style=\"stroke-dasharray: 0, 0;\"/></marker>",
                color
            ));
                // Open (async) arrow marker for -) / --) arrows
                svg.push_str(&format!(
                "<marker id=\"open-seq-{idx}\" refX=\"15.5\" refY=\"7\" markerWidth=\"20\" markerHeight=\"28\" orient=\"auto\"><path d=\"M 18,7 L9,13 L14,7 L9,1 Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/></marker>",
                color, color
            ));
            }
            if is_state {
                svg.push_str(&format!(
                "<marker id=\"arrow-state-{idx}\" viewBox=\"0 0 20 14\" refX=\"19\" refY=\"7\" markerUnits=\"userSpaceOnUse\" markerWidth=\"20\" markerHeight=\"14\" orient=\"auto\"><path d=\"M 19 7 L 9 13 L 14 7 L 9 1 Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"1,0\"/></marker>",
                color, color
            ));
            }
            if is_class {
                svg.push_str(&format!(
                "<marker id=\"arrow-class-open-{idx}\" viewBox=\"0 0 20 14\" refX=\"1\" refY=\"7\" markerUnits=\"userSpaceOnUse\" markerWidth=\"20\" markerHeight=\"14\" orient=\"auto\"><path d=\"M 1 1 V 13 L 18 7 Z\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"1,0\"/></marker>",
                color
            ));
                svg.push_str(&format!(
                "<marker id=\"arrow-class-open-start-{idx}\" viewBox=\"0 0 20 14\" refX=\"18\" refY=\"7\" markerUnits=\"userSpaceOnUse\" markerWidth=\"20\" markerHeight=\"14\" orient=\"auto\"><path d=\"M 1 7 L 18 13 V 1 Z\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"1,0\"/></marker>",
                color
            ));
                svg.push_str(&format!(
                "<marker id=\"arrow-class-dep-{idx}\" viewBox=\"0 0 20 14\" refX=\"13\" refY=\"7\" markerUnits=\"userSpaceOnUse\" markerWidth=\"20\" markerHeight=\"14\" orient=\"auto\"><path d=\"M 18 7 L 9 13 L 14 7 L 9 1 Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"1,0\"/></marker>",
                color, color
            ));
                svg.push_str(&format!(
                "<marker id=\"arrow-class-dep-start-{idx}\" viewBox=\"0 0 20 14\" refX=\"6\" refY=\"7\" markerUnits=\"userSpaceOnUse\" markerWidth=\"20\" markerHeight=\"14\" orient=\"auto\"><path d=\"M 5 7 L 9 13 L 1 7 L 9 1 Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"1,0\"/></marker>",
                color, color
            ));
            }
        }
        svg.push_str("</defs>");
    } // end !is_timeline defs block

    if let DiagramData::Error(ref error) = layout.diagram {
        svg.push_str(&render_error(error, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::Sankey(ref sankey) = layout.diagram {
        svg.push_str("<g/>");
        svg.push_str(&render_sankey(sankey, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if layout.kind == crate::ir::DiagramKind::Architecture {
        svg.push_str(&render_architecture(layout, theme, config, &color_ids));
        svg.push_str("</svg>");
        return svg;
    }

    if layout.kind == crate::ir::DiagramKind::Radar {
        svg.push_str(&render_radar(layout, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if layout.kind == crate::ir::DiagramKind::Requirement {
        svg.push_str(&render_requirement(layout, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::Pie(ref pie) = layout.diagram {
        svg.push_str(&render_pie(pie, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::Quadrant(ref quadrant) = layout.diagram {
        svg.push_str(&render_quadrant(quadrant, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::Gantt(ref gantt) = layout.diagram {
        svg.push_str(&render_gantt(gantt, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::XYChart(ref xychart) = layout.diagram {
        svg.push_str(&render_xychart(xychart, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::Timeline(ref timeline) = layout.diagram {
        svg.push_str(&render_timeline(timeline, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::Journey(ref journey) = layout.diagram {
        svg.push_str(&render_journey(journey, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::GitGraph(ref gitgraph) = layout.diagram {
        svg.push_str(&render_gitgraph(gitgraph, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::Venn(ref venn) = layout.diagram {
        svg.push_str(&render_venn(venn, theme, config));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::Packet(ref packet) = layout.diagram {
        svg.push_str(&render_packet(packet));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::TreeView(ref tv) = layout.diagram {
        svg.push_str(&render_tree_view(tv, theme));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::Ishikawa(ref ish) = layout.diagram {
        svg.push_str(&render_ishikawa(ish, theme));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::Wardley(ref w) = layout.diagram {
        svg.push_str(&render_wardley(w, theme));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::EventModeling(ref eventmodeling) = layout.diagram {
        svg.push_str(&render_eventmodeling(eventmodeling));
        svg.push_str("</svg>");
        return svg;
    }

    if let DiagramData::Cynefin(ref cynefin) = layout.diagram {
        svg.push_str(&render_cynefin(cynefin, theme));
        svg.push_str("</svg>");
        return svg;
    }

    for (subgraph_index, subgraph) in layout.subgraphs.iter().enumerate() {
        let label_empty = subgraph.label.trim().is_empty();
        if is_state {
            let sub_fill = subgraph.style.fill.as_ref().unwrap_or(&theme.primary_color);
            let sub_stroke = subgraph
                .style
                .stroke
                .as_ref()
                .unwrap_or(&theme.primary_border_color);
            let sub_stroke_width = subgraph.style.stroke_width.unwrap_or(1.0);
            let invisible = label_empty
                && sub_fill.as_str() == "none"
                && sub_stroke.as_str() == "none"
                && sub_stroke_width <= 0.0;
            if invisible {
                continue;
            }
            let header_h = if label_empty {
                0.0
            } else {
                subgraph.label_block.height + 2.0
            };
            let header_fill = if sub_fill.as_str() == "none" {
                "none".to_string()
            } else {
                adjust_color(sub_fill, 0.0, 0.0, -4.0)
            };
            let body_fill = if sub_fill.as_str() == "none" {
                theme.background.clone()
            } else {
                adjust_color(sub_fill, 0.0, -12.0, 10.0)
            };
            let rounded_with_title = header_h > 0.0;
            if rounded_with_title {
                svg.push_str(&format!(
                    "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"none\"/>",
                    subgraph.x,
                    subgraph.y,
                    subgraph.width,
                    subgraph.height,
                    header_fill
                ));
            }
            let inner_y = subgraph.y + header_h;
            let inner_h = if rounded_with_title {
                (subgraph.height - subgraph.label_block.height - 6.0).max(0.0)
            } else {
                subgraph.height
            };
            if inner_h > 0.0 {
                svg.push_str(&format!(
                    "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"none\"/>",
                    subgraph.x,
                    inner_y,
                    subgraph.width,
                    inner_h,
                    body_fill
                ));
            }
            if header_h > 0.0 {
                svg.push_str(&format!(
                    "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1\"/>",
                    subgraph.x,
                    inner_y,
                    subgraph.x + subgraph.width,
                    inner_y,
                    sub_stroke
                ));
            }
            let sub_dash = subgraph
                .style
                .stroke_dasharray
                .as_ref()
                .map(|value| format!(" stroke-dasharray=\"{}\"", value))
                .unwrap_or_default();
            svg.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"6\" ry=\"6\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                subgraph.x,
                subgraph.y,
                subgraph.width,
                subgraph.height,
                sub_stroke,
                sub_stroke_width,
                sub_dash
            ));
            if !label_empty {
                // JS centers composite-state cluster titles on the cluster's
                // horizontal midpoint (see mermaid-js stateDiagram .cluster-label
                // translate computation).
                let label_x = subgraph.x + subgraph.width / 2.0;
                let label_y = subgraph.y + header_h / 2.0;
                let weight = subgraph.style.font_weight.as_deref().unwrap_or("600");
                svg.push_str(&text_block_svg_with_font_attrs(
                    label_x,
                    label_y,
                    &subgraph.label_block,
                    theme,
                    config,
                    state_font_size,
                    "middle",
                    subgraph.style.text_color.as_deref(),
                    Some(weight),
                    subgraph.style.font_style.as_deref(),
                    false,
                ));
            }
        } else {
            let block_cluster_fill = if is_block {
                Some(color_with_opacity(&theme.cluster_background, 0.5))
            } else {
                None
            };
            let block_cluster_stroke = if is_block {
                Some(color_with_opacity(&theme.cluster_border, 0.2))
            } else {
                None
            };
            let kanban_section_fill = if layout.kind == crate::ir::DiagramKind::Kanban {
                Some(default_kanban_section_fill(subgraph_index))
            } else {
                None
            };
            let default_fill = if is_block {
                block_cluster_fill
                    .as_deref()
                    .unwrap_or(&theme.cluster_background)
            } else if let Some(fill) = kanban_section_fill {
                fill
            } else {
                &theme.cluster_background
            };
            let default_stroke = if is_block {
                block_cluster_stroke
                    .as_deref()
                    .unwrap_or(&theme.cluster_border)
            } else if let Some(fill) = kanban_section_fill {
                fill
            } else {
                &theme.cluster_border
            };
            let sub_fill = subgraph.style.fill.as_deref().unwrap_or(default_fill);
            let sub_stroke = subgraph.style.stroke.as_deref().unwrap_or(default_stroke);
            let sub_dash = subgraph
                .style
                .stroke_dasharray
                .as_ref()
                .map(|value| format!(" stroke-dasharray=\"{}\"", value))
                .unwrap_or_default();
            let sub_stroke_width = subgraph.style.stroke_width.unwrap_or(1.0);
            let invisible = label_empty
                && sub_fill == "none"
                && sub_stroke == "none"
                && sub_stroke_width <= 0.0;
            if !invisible {
                let radius = if matches!(
                    layout.kind,
                    crate::ir::DiagramKind::Block | crate::ir::DiagramKind::Flowchart
                ) {
                    0
                } else if layout.kind == crate::ir::DiagramKind::Kanban {
                    5
                } else {
                    10
                };
                svg.push_str(&format!(
                    "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{} />",
                    subgraph.x,
                    subgraph.y,
                    subgraph.width,
                    subgraph.height,
                    radius,
                    radius,
                    sub_fill,
                    sub_stroke,
                    sub_stroke_width,
                    sub_dash
                ));
            }
            if !label_empty {
                let label_x = subgraph.x + subgraph.width / 2.0;
                let label_y = if matches!(
                    layout.kind,
                    crate::ir::DiagramKind::Flowchart | crate::ir::DiagramKind::Kanban
                ) {
                    // Mermaid JS positions flowchart cluster labels at the top
                    // edge of the cluster (`subGraphTitleTopMargin`, normally
                    // zero) and lets the label's measured box occupy the title
                    // band. Our text helper takes the label center, so use the
                    // center of that measured band here.
                    subgraph.y + subgraph.label_block.height / 2.0
                } else {
                    subgraph.y + 12.0 + subgraph.label_block.height / 2.0
                };
                let default_label_color = if layout.kind == crate::ir::DiagramKind::Kanban {
                    "black"
                } else {
                    theme.primary_text_color.as_str()
                };
                let label_color = subgraph
                    .style
                    .text_color
                    .as_deref()
                    .unwrap_or(default_label_color);
                svg.push_str(&text_block_svg(
                    label_x,
                    label_y,
                    &subgraph.label_block,
                    theme,
                    config,
                    false,
                    Some(label_color),
                ));
            }
        }
    }

    let overlay_flowchart = layout.kind == crate::ir::DiagramKind::Flowchart;
    let mut overlay_arrows: Vec<(bool, (f32, f32), f32, String, f32)> = Vec::new();

    if let Some(seq) = seq_data {
        for seq_box in &seq.boxes {
            let stroke = theme.primary_border_color.as_str();
            let fill = seq_box.color.as_deref().unwrap_or("none");
            let mut fill_attr = format!("fill=\"{}\"", fill);
            if seq_box.color.is_some() && fill != "none" {
                fill_attr.push_str(" fill-opacity=\"0.12\"");
            }
            svg.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" {fill_attr} stroke=\"{}\" stroke-width=\"1.2\"/>",
                seq_box.x, seq_box.y, seq_box.width, seq_box.height, stroke
            ));
            if let Some(label) = seq_box.label.as_ref() {
                // Center the box title horizontally above the actor row,
                // matching mermaid.js convention. The vertical offset places
                // the label center in the gap reserved by `actor_y_offset`.
                let label_x = seq_box.x + seq_box.width / 2.0;
                let label_y = seq_box.y + theme.font_size * 0.85;
                svg.push_str(&sequence_text_block_svg_anchor(
                    label_x,
                    label_y,
                    label,
                    theme,
                    "middle",
                    Some(theme.primary_text_color.as_str()),
                ));
            }
        }
    }

    for frame in seq_data.map(|s| s.frames.as_slice()).unwrap_or_default() {
        let stroke = sequence_frame_border_color(theme);
        // `rect <color>` blocks render as a solid filled background with no
        // dashed border and no label box (matches mermaid.js convention).
        if matches!(frame.kind, crate::ir::SequenceFrameKind::Rect) {
            let fill = frame
                .fill_color
                .as_deref()
                .unwrap_or("rgba(200,200,200,0.3)");
            svg.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"none\"/>",
                frame.x, frame.y, frame.width, frame.height, fill
            ));
            continue;
        }
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2.0\" stroke-dasharray=\"2 2\"/>",
            frame.x, frame.y, frame.width, frame.height, stroke
        ));
        for divider_y in &frame.dividers {
            svg.push_str(&format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"2.0\" stroke-dasharray=\"3 3\"/>",
                frame.x,
                divider_y,
                frame.x + frame.width,
                divider_y,
                stroke
            ));
        }
        let (box_x, box_y, box_w, box_h) = frame.label_box;
        let notch_x = box_x + box_w * 0.8;
        let notch_y = box_y + box_h;
        let mid_y = box_y + box_h * 0.65;
        svg.push_str(&format!(
            "<polygon points=\"{box_x:.2},{box_y:.2} {end_x:.2},{box_y:.2} {end_x:.2},{mid_y:.2} {notch_x:.2},{notch_y:.2} {box_x:.2},{notch_y:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.1\"/>",
            theme.primary_color,
            stroke,
            end_x = box_x + box_w,
            mid_y = mid_y,
            notch_x = notch_x,
            notch_y = notch_y
        ));
        svg.push_str(&sequence_text_block_svg(
            frame.label.x,
            frame.label.y,
            &frame.label.text,
            theme,
            false,
            Some(theme.primary_text_color.as_str()),
        ));
        for label in &frame.section_labels {
            svg.push_str(&sequence_text_block_svg(
                label.x,
                label.y,
                &label.text,
                theme,
                false,
                None,
            ));
        }
    }

    for lifeline in seq_data.map(|s| s.lifelines.as_slice()).unwrap_or_default() {
        svg.push_str(&format!(
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
            lifeline.x,
            lifeline.y1,
            lifeline.x,
            lifeline.y2,
            theme.sequence_actor_line
        ));
    }

    // Destroyed actors are indicated by:
    //   1. The `crosshead` (cross-seq-N) marker on the destroy message's
    //      arrow tip — already emitted via marker-end on the edge.
    //   2. The actor's footer rect repositioned to the destroy Y instead of
    //      the diagram bottom.
    // mermaid.js does NOT draw a standalone X-cross on the lifeline; doing so
    // here would visibly overlap the footer rect's top border. Keep
    // `destroy_markers` in the layout for any future renderer that wants it,
    // but skip rendering them here.
    let _unused_destroy_markers = seq_data.map(|s| s.destroy_markers.as_slice());

    let mut activations_sorted: Vec<_> = seq_data
        .map(|s| s.activations.as_slice())
        .unwrap_or_default()
        .iter()
        .collect();
    // Render larger (outer) activations FIRST so smaller (inner) stacked
    // activations layer on top and remain visible. Otherwise outer rects
    // would cover inner ones, hiding the stack.
    activations_sorted.sort_by(|a, b| {
        b.height
            .partial_cmp(&a.height)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for activation in activations_sorted {
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            activation.x,
            activation.y,
            activation.width,
            activation.height,
            theme.sequence_activation_fill,
            theme.sequence_activation_border
        ));
    }

    for note in seq_data.map(|s| s.notes.as_slice()).unwrap_or_default() {
        let fill = theme.sequence_note_fill.as_str();
        let stroke = theme.sequence_note_border.as_str();
        // Sequence notes are plain rectangles in mermaid.js (yellow with
        // dark border). State/class notes use a folded-corner path; sequence
        // notes do not.
        svg.push_str(&format!(
            "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
            x = note.x,
            y = note.y,
            w = note.width,
            h = note.height,
        ));
        let center_x = note.x + note.width / 2.0;
        let center_y = note.y + note.height / 2.0;
        svg.push_str(&sequence_text_block_svg(
            center_x,
            center_y,
            &note.label,
            theme,
            false,
            Some(theme.primary_text_color.as_str()),
        ));
    }

    if let DiagramData::Graph { state_notes, .. } = &layout.diagram {
        for (note_idx, note) in state_notes.iter().enumerate() {
            let fill = theme.sequence_note_fill.as_str();
            let stroke = theme.sequence_note_border.as_str();
            let fold = (theme.font_size * 0.8)
                .max(8.0)
                .min(note.width.min(note.height) * 0.3);
            let x = note.x;
            let y = note.y;
            let x2 = note.x + note.width;
            let y2 = note.y + note.height;
            let fold_x = x2 - fold;
            let fold_y = y + fold;
            // Draw dashed connector from note to its target state first so
            // the note shape paints over the connector ends.
            if let Some(target) = layout.nodes.get(&note.target) {
                let note_cx = note.x + note.width / 2.0;
                let note_cy = note.y + note.height / 2.0;
                let target_cx = target.x + target.width / 2.0;
                let target_cy = target.y + target.height / 2.0;
                let (start, end) = if note_cy > target_cy + 1.0 {
                    let biased_x = target_cx + (note_cx - target_cx) * 0.38;
                    ((biased_x, target.y + target.height), (note_cx, note.y))
                } else if note_cy + 1.0 < target_cy {
                    let biased_x = target_cx + (note_cx - target_cx) * 0.26;
                    ((note_cx, note.y + note.height), (biased_x, target.y))
                } else if note_cx < target_cx {
                    ((note.x + note.width, note_cy), (target.x, target_cy))
                } else {
                    ((note.x, note_cy), (target.x + target.width, target_cy))
                };
                let dy = end.1 - start.1;
                let c1 = (start.0, start.1 + dy * 0.33);
                let c2 = (end.0, end.1 - dy * 0.33);
                let connector_id = format!("state-note-edge-{note_idx}");
                svg.push_str(&format!(
                    "<path id=\"{connector_id}\" class=\"edge-thickness-normal edge-pattern-solid transition note-edge\" data-edge=\"true\" data-et=\"edge\" data-id=\"{connector_id}\" d=\"M {sx:.3},{sy:.3} C {c1x:.3},{c1y:.3} {c2x:.3},{c2y:.3} {ex:.3},{ey:.3}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"5\" fill=\"none\"/>",
                    default_edge_stroke,
                    sx = start.0,
                    sy = start.1,
                    c1x = c1.0,
                    c1y = c1.1,
                    c2x = c2.0,
                    c2y = c2.1,
                    ex = end.0,
                    ey = end.1
                ));
            }
            svg.push_str(&format!(
                "<path d=\"M {x:.2} {y:.2} L {fold_x:.2} {y:.2} L {x2:.2} {fold_y:.2} L {x2:.2} {y2:.2} L {x:.2} {y2:.2} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.1\"/>"
            ));
            svg.push_str(&format!(
                "<polyline points=\"{fold_x:.2},{y:.2} {fold_x:.2},{fold_y:.2} {x2:.2},{fold_y:.2}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.0\"/>"
            ));
            let center_x = note.x + note.width / 2.0;
            let center_y = note.y + note.height / 2.0;
            svg.push_str(&text_block_svg_with_font_size(
                center_x,
                center_y,
                &note.label,
                theme,
                config,
                state_font_size,
                "middle",
                Some(theme.primary_text_color.as_str()),
                false,
            ));
        }
    }

    let mut er_decoration_overlays: Vec<String> = Vec::new();

    if is_sequence {
        for (edge_idx, edge) in layout.edges.iter().enumerate() {
            let d = points_to_path(&edge.points);
            let mut stroke = default_edge_stroke.clone();
            let edge_id = edge_dom_id(edge_idx);
            if let Some(color) = &edge.override_style.stroke {
                stroke = color.clone();
            }
            let edge_label_fill = theme.edge_label_background.as_str();
            let edge_label_stroke = theme.primary_border_color.as_str();
            let (center_pad_x, center_pad_y) = edge_label_padding(layout.kind, config);
            let (endpoint_pad_x, endpoint_pad_y) = endpoint_label_padding(layout.kind);
            let marker_id = color_ids.get(&stroke).copied().unwrap_or(0);
            let marker_end = match edge.sequence_arrow_end {
                Some(crate::ir::SequenceArrowHead::Filled) => {
                    format!("marker-end=\"url(#arrow-seq-{marker_id})\"")
                }
                Some(crate::ir::SequenceArrowHead::Cross) => {
                    format!("marker-end=\"url(#cross-seq-{marker_id})\"")
                }
                Some(crate::ir::SequenceArrowHead::Open) => {
                    format!("marker-end=\"url(#open-seq-{marker_id})\"")
                }
                _ => {
                    if edge.arrow_end {
                        format!("marker-end=\"url(#arrow-seq-{marker_id})\"")
                    } else {
                        String::new()
                    }
                }
            };
            let marker_start = match edge.sequence_arrow_start {
                Some(crate::ir::SequenceArrowHead::Filled) => {
                    format!("marker-start=\"url(#arrow-start-seq-{marker_id})\"")
                }
                Some(crate::ir::SequenceArrowHead::Cross) => {
                    format!("marker-start=\"url(#cross-seq-{marker_id})\"")
                }
                Some(crate::ir::SequenceArrowHead::Open) => {
                    format!("marker-start=\"url(#open-seq-{marker_id})\"")
                }
                _ => {
                    if edge.arrow_start {
                        format!("marker-start=\"url(#arrow-start-seq-{marker_id})\"")
                    } else {
                        String::new()
                    }
                }
            };

            let mut dash = String::new();
            if edge.style == crate::ir::EdgeStyle::Dotted {
                dash = "stroke-dasharray=\"2,2\"".to_string();
            }
            if let Some(dash_override) = &edge.override_style.dasharray {
                dash = format!("stroke-dasharray=\"{}\"", dash_override);
            }
            let stroke_width = edge.override_style.stroke_width.unwrap_or_else(|| {
                if edge.style == crate::ir::EdgeStyle::Invisible {
                    0.0
                } else {
                    1.5
                }
            });
            svg.push_str(&format!(
                "<path id=\"{edge_id}\" class=\"edgePath\" data-edge-id=\"{edge_id}\" d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" {} {} {} stroke-linecap=\"round\" stroke-linejoin=\"round\" />",
                d, stroke, stroke_width, marker_end, marker_start, dash
            ));

            if let Some(point) = edge.points.first().copied()
                && let Some(decoration) = edge.start_decoration
            {
                let angle = edge_endpoint_angle(&edge.points, true);
                svg.push_str(&edge_decoration_svg(
                    point,
                    angle,
                    decoration,
                    &stroke,
                    stroke_width,
                    true,
                ));
            }
            if let Some(point) = edge.points.last().copied()
                && let Some(decoration) = edge.end_decoration
            {
                let angle = edge_endpoint_angle(&edge.points, false);
                svg.push_str(&edge_decoration_svg(
                    point,
                    angle,
                    decoration,
                    &stroke,
                    stroke_width,
                    false,
                ));
            }

            if let Some(label) = edge.label.as_ref() {
                // Sequence message labels always sit ABOVE the line with a
                // consistent gap (matches mermaid.js: text bottom ~5px above
                // the line, total label center ≈ font_size*0.85 above the line
                // for a 1-line label). Override the y from any anchor that
                // would put the text on top of the connector line.
                let start = edge.points.first().copied().unwrap_or((0.0, 0.0));
                let end = edge.points.last().copied().unwrap_or(start);
                let line_y = start.1;
                let gap = (theme.font_size * 0.4).max(5.0);
                let anchor_x = edge
                    .label_anchor
                    .map(|p| p.0)
                    .unwrap_or((start.0 + end.0) / 2.0);
                let mid_x = anchor_x;
                let label_y = line_y - gap - label.height / 2.0;
                let label_color = edge
                    .override_style
                    .label_color
                    .as_deref()
                    .unwrap_or(theme.primary_text_color.as_str());
                // Upstream mermaid renders sequence message labels directly
                // above the line with no background rect.
                let _ = (center_pad_x, center_pad_y);
                svg.push_str(&format!(
                    "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"center\">"
                ));
                svg.push_str(&sequence_text_block_svg(
                    mid_x,
                    label_y,
                    label,
                    theme,
                    false,
                    Some(label_color),
                ));
                svg.push_str("</g>");
            }

            let end_label_offset = (theme.font_size * 0.6).max(8.0);
            let label_color = edge
                .override_style
                .label_color
                .as_deref()
                .unwrap_or(theme.primary_text_color.as_str());
            if let Some(label) = edge.start_label.as_ref()
                && let Some((x, y)) = edge
                    .start_label_anchor
                    .or_else(|| edge_endpoint_label_position(edge, true, end_label_offset))
            {
                if edge_label_fill != "none" {
                    let rect = LabelRect::from_center(
                        x,
                        y,
                        label.width,
                        label.height,
                        endpoint_pad_x,
                        endpoint_pad_y,
                    );
                    let visible = edge_label_background_visible(
                        layout.kind,
                        EdgeLabelKind::Start,
                        &edge.points,
                        rect,
                    );
                    let fill_opacity = if visible { 0.88 } else { 0.0 };
                    let stroke_opacity = if visible { 0.28 } else { 0.0 };
                    svg.push_str(&format!(
                        "<rect class=\"edgeLabel sequenceEndpointLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"start\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" fill-opacity=\"{:.2}\" stroke=\"{}\" stroke-opacity=\"{:.2}\" stroke-width=\"0.75\"/>",
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        edge_label_fill,
                        fill_opacity,
                        edge_label_stroke,
                        stroke_opacity
                    ));
                }
                svg.push_str(&format!(
                    "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"start\">"
                ));
                svg.push_str(&sequence_text_block_svg(
                    x,
                    y,
                    label,
                    theme,
                    false,
                    Some(label_color),
                ));
                svg.push_str("</g>");
            }
            if let Some(label) = edge.end_label.as_ref()
                && let Some((x, y)) = edge
                    .end_label_anchor
                    .or_else(|| edge_endpoint_label_position(edge, false, end_label_offset))
            {
                if edge_label_fill != "none" {
                    let rect = LabelRect::from_center(
                        x,
                        y,
                        label.width,
                        label.height,
                        endpoint_pad_x,
                        endpoint_pad_y,
                    );
                    let visible = edge_label_background_visible(
                        layout.kind,
                        EdgeLabelKind::End,
                        &edge.points,
                        rect,
                    );
                    let fill_opacity = if visible { 0.88 } else { 0.0 };
                    let stroke_opacity = if visible { 0.28 } else { 0.0 };
                    svg.push_str(&format!(
                        "<rect class=\"edgeLabel sequenceEndpointLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"end\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" fill-opacity=\"{:.2}\" stroke=\"{}\" stroke-opacity=\"{:.2}\" stroke-width=\"0.75\"/>",
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        edge_label_fill,
                        fill_opacity,
                        edge_label_stroke,
                        stroke_opacity
                    ));
                }
                svg.push_str(&format!(
                    "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"end\">"
                ));
                svg.push_str(&sequence_text_block_svg(
                    x,
                    y,
                    label,
                    theme,
                    false,
                    Some(label_color),
                ));
                svg.push_str("</g>");
            }
        }

        for number in seq_data.map(|s| s.numbers.as_slice()).unwrap_or_default() {
            // Match upstream mermaid: small circle (r ≈ 6) drawn behind a 12px
            // sans-serif numeral, regardless of the diagram font size.
            let r = 8.0;
            svg.push_str(&format!(
                "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                number.x,
                number.y,
                r,
                theme.sequence_activation_fill,
                theme.sequence_activation_border
            ));
            let label = number.value.to_string();
            svg.push_str(&format!(
                "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"trebuchet ms,verdana,arial,sans-serif\" font-size=\"12\" fill=\"{}\">{}</text>",
                number.x,
                number.y + 4.0,
                theme.primary_text_color,
                label
            ));
        }
    } else {
        let base_edge_width = match layout.kind {
            crate::ir::DiagramKind::Class
            | crate::ir::DiagramKind::Flowchart
            | crate::ir::DiagramKind::State
            | crate::ir::DiagramKind::Er => 1.0,
            _ => 2.0,
        };
        for (edge_idx, edge) in layout.edges.iter().enumerate() {
            let edge_curve = edge.curve.unwrap_or(config.flowchart.curve);
            let render_points = class_symbol_render_points(edge, layout.kind);
            let path_points = if layout.kind == crate::ir::DiagramKind::Flowchart {
                flowchart_marker_offset_render_points(&render_points, edge)
            } else {
                render_points.clone()
            };
            let d = {
                let raw = if edge_curve == crate::ir::CurveType::Basis
                    && matches!(
                        layout.kind,
                        crate::ir::DiagramKind::Block
                            | crate::ir::DiagramKind::Class
                            | crate::ir::DiagramKind::State
                            | crate::ir::DiagramKind::Flowchart
                    ) {
                    if matches!(
                        layout.kind,
                        crate::ir::DiagramKind::Flowchart | crate::ir::DiagramKind::State
                    ) {
                        let basis_points = flowchart_d3_basis_points(&path_points);
                        points_to_d3_basis_path(&basis_points)
                    } else {
                        points_to_d3_basis_path(&path_points)
                    }
                } else {
                    points_to_curved_path(&path_points, edge_curve)
                };
                if config.look == crate::ir::DiagramLook::HandDrawn {
                    let seed = hand_drawn_seed(
                        path_points.first().map(|p| p.0).unwrap_or(0.0),
                        path_points.first().map(|p| p.1).unwrap_or(0.0),
                        path_points.last().map(|p| p.0).unwrap_or(0.0),
                        path_points.last().map(|p| p.1).unwrap_or(0.0),
                    );
                    hand_drawn_path_jitter(&raw, 1.0, seed)
                } else {
                    raw
                }
            };
            let mut stroke = default_edge_stroke.clone();
            let edge_id = edge_dom_id(edge_idx);
            let (mut dash, mut stroke_width) = match edge.style {
                crate::ir::EdgeStyle::Solid => (String::new(), base_edge_width),
                crate::ir::EdgeStyle::Dotted => {
                    ("stroke-dasharray=\"2\"".to_string(), base_edge_width)
                }
                crate::ir::EdgeStyle::Thick => (String::new(), 3.5),
                crate::ir::EdgeStyle::Invisible => (String::new(), 0.0),
            };

            if let Some(color) = &edge.override_style.stroke {
                stroke = color.clone();
            }
            let marker_id = color_ids.get(&stroke).copied().unwrap_or(0);
            let marker_end = if edge.arrow_end && !overlay_flowchart {
                match layout.kind {
                    crate::ir::DiagramKind::State => {
                        format!("marker-end=\"url(#arrow-state-{marker_id})\"")
                    }
                    crate::ir::DiagramKind::Class => match edge.arrow_end_kind {
                        Some(crate::ir::EdgeArrowhead::OpenTriangle) => {
                            format!("marker-end=\"url(#arrow-class-open-{marker_id})\"")
                        }
                        Some(crate::ir::EdgeArrowhead::ClassDependency) => {
                            format!("marker-end=\"url(#arrow-class-dep-{marker_id})\"")
                        }
                        None => format!("marker-end=\"url(#arrow-{marker_id})\""),
                    },
                    _ => format!("marker-end=\"url(#arrow-{marker_id})\""),
                }
            } else {
                String::new()
            };
            let marker_start = if edge.arrow_start && !overlay_flowchart {
                match layout.kind {
                    crate::ir::DiagramKind::State => {
                        format!("marker-start=\"url(#arrow-state-{marker_id})\"")
                    }
                    crate::ir::DiagramKind::Class => match edge.arrow_start_kind {
                        Some(crate::ir::EdgeArrowhead::OpenTriangle) => {
                            format!("marker-start=\"url(#arrow-class-open-start-{marker_id})\"")
                        }
                        Some(crate::ir::EdgeArrowhead::ClassDependency) => {
                            format!("marker-start=\"url(#arrow-class-dep-start-{marker_id})\"")
                        }
                        None => format!("marker-start=\"url(#arrow-start-{marker_id})\""),
                    },
                    _ => format!("marker-start=\"url(#arrow-start-{marker_id})\""),
                }
            } else {
                String::new()
            };
            if let Some(width) = edge.override_style.stroke_width {
                stroke_width = width;
            }
            if let Some(dash_override) = &edge.override_style.dasharray {
                dash = format!("stroke-dasharray=\"{}\"", dash_override);
            }
            svg.push_str(&format!(
                "<path id=\"{edge_id}\" class=\"edgePath\" data-edge-id=\"{edge_id}\" d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" {} {} {} stroke-linecap=\"round\" stroke-linejoin=\"round\" />",
                d, stroke, stroke_width, marker_end, marker_start, dash
            ));

            if overlay_flowchart {
                if edge.arrow_start {
                    if let Some(point) = render_points.first().copied() {
                        let angle = edge_endpoint_angle(&render_points, true);
                        overlay_arrows.push((true, point, angle, stroke.clone(), stroke_width));
                    }
                }
                if edge.arrow_end {
                    if let Some(point) = render_points.last().copied() {
                        let angle = edge_endpoint_angle(&render_points, false);
                        overlay_arrows.push((false, point, angle, stroke.clone(), stroke_width));
                    }
                }
            }

            let overlay_er_decoration = layout.kind == crate::ir::DiagramKind::Er;
            if let Some(point) = render_points.first().copied()
                && let Some(decoration) = edge.start_decoration
            {
                let angle = edge_endpoint_angle(&render_points, true);
                let decoration_svg =
                    edge_decoration_svg(point, angle, decoration, &stroke, stroke_width, true);
                if overlay_er_decoration {
                    er_decoration_overlays.push(decoration_svg);
                } else {
                    svg.push_str(&decoration_svg);
                }
            }
            if let Some(point) = render_points.last().copied()
                && let Some(decoration) = edge.end_decoration
            {
                let mut angle = edge_endpoint_angle(&render_points, false);
                if overlay_er_decoration {
                    angle += 180.0;
                }
                let decoration_svg =
                    edge_decoration_svg(point, angle, decoration, &stroke, stroke_width, false);
                if overlay_er_decoration {
                    er_decoration_overlays.push(decoration_svg);
                } else {
                    svg.push_str(&decoration_svg);
                }
            }

            if let Some(label) = edge.label.as_ref()
                && let Some((x, y)) = edge.label_anchor
            {
                let (pad_x, pad_y) = edge_label_padding(layout.kind, config);
                let (fill_opacity, stroke_opacity) = match layout.kind {
                    crate::ir::DiagramKind::State => (0.7, 0.25),
                    crate::ir::DiagramKind::Flowchart => (0.95, 0.45),
                    _ => (0.85, 0.35),
                };
                let label_scale = if layout.kind == crate::ir::DiagramKind::State {
                    (state_font_size / theme.font_size).min(1.0)
                } else {
                    1.0
                };
                let label_w = label.width * label_scale;
                let label_h = label.height * label_scale;
                let rect = LabelRect::from_center(x, y, label_w, label_h, pad_x, pad_y);
                let label_fill = theme.edge_label_background.as_str();
                if label_fill != "none" {
                    let visible = edge_label_background_visible(
                        layout.kind,
                        EdgeLabelKind::Center,
                        &edge.points,
                        rect,
                    );
                    let fill_opacity = if visible { fill_opacity } else { 0.0 };
                    let stroke_opacity = if visible { stroke_opacity } else { 0.0 };
                    svg.push_str(&format!(
                        "<rect data-edge-id=\"{edge_id}\" data-label-kind=\"center\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" fill-opacity=\"{:.2}\" stroke=\"{}\" stroke-opacity=\"{:.2}\" stroke-width=\"0.8\"/>",
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        label_fill,
                        fill_opacity,
                        theme.primary_border_color,
                        stroke_opacity
                    ));
                }
                if layout.kind == crate::ir::DiagramKind::State {
                    svg.push_str(&format!(
                        "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"center\">"
                    ));
                    svg.push_str(&text_block_svg_with_font_size(
                        x,
                        y,
                        label,
                        theme,
                        config,
                        state_font_size,
                        "middle",
                        edge.override_style.label_color.as_deref(),
                        false,
                    ));
                    svg.push_str("</g>");
                } else {
                    svg.push_str(&format!(
                        "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"center\">"
                    ));
                    svg.push_str(&text_block_svg(
                        x,
                        y,
                        label,
                        theme,
                        config,
                        true,
                        edge.override_style.label_color.as_deref(),
                    ));
                    svg.push_str("</g>");
                }
            }

            let endpoint_label_scale = if layout.kind == crate::ir::DiagramKind::State {
                (state_font_size / theme.font_size).min(1.0)
            } else {
                1.0
            };
            let endpoint_font_size = match layout.kind {
                crate::ir::DiagramKind::State => Some(state_font_size),
                crate::ir::DiagramKind::Class => Some(CLASS_ENDPOINT_LABEL_FONT_SIZE),
                _ => None,
            };
            let (endpoint_pad_x, endpoint_pad_y) = endpoint_label_padding(layout.kind);
            let (endpoint_fill_opacity, endpoint_stroke_opacity) = match layout.kind {
                crate::ir::DiagramKind::State => (0.7, 0.25),
                crate::ir::DiagramKind::Flowchart => (0.95, 0.45),
                crate::ir::DiagramKind::Class => (0.9, 0.4),
                _ => (0.85, 0.35),
            };
            let endpoint_label_fill = theme.edge_label_background.as_str();
            let label_color = edge
                .override_style
                .label_color
                .as_deref()
                .unwrap_or(theme.primary_text_color.as_str());
            if let Some(label) = edge.start_label.as_ref()
                && let Some((x, y)) = edge.start_label_anchor
            {
                let label_w = label.width * endpoint_label_scale;
                let label_h = label.height * endpoint_label_scale;
                let rect =
                    LabelRect::from_center(x, y, label_w, label_h, endpoint_pad_x, endpoint_pad_y);
                if endpoint_label_fill != "none" {
                    let visible = edge_label_background_visible(
                        layout.kind,
                        EdgeLabelKind::Start,
                        &edge.points,
                        rect,
                    );
                    let fill_opacity = if visible { endpoint_fill_opacity } else { 0.0 };
                    let stroke_opacity = if visible {
                        endpoint_stroke_opacity
                    } else {
                        0.0
                    };
                    svg.push_str(&format!(
                        "<rect data-edge-id=\"{edge_id}\" data-label-kind=\"start\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" fill-opacity=\"{:.2}\" stroke=\"{}\" stroke-opacity=\"{:.2}\" stroke-width=\"0.8\"/>",
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        endpoint_label_fill,
                        fill_opacity,
                        theme.primary_border_color,
                        stroke_opacity
                    ));
                }
                if let Some(font_size) = endpoint_font_size {
                    svg.push_str(&format!(
                        "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"start\">"
                    ));
                    svg.push_str(&text_block_svg_with_font_size(
                        x,
                        y,
                        label,
                        theme,
                        config,
                        font_size,
                        "middle",
                        Some(label_color),
                        false,
                    ));
                    svg.push_str("</g>");
                } else {
                    svg.push_str(&format!(
                        "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"start\">"
                    ));
                    svg.push_str(&text_block_svg(
                        x,
                        y,
                        label,
                        theme,
                        config,
                        false,
                        Some(label_color),
                    ));
                    svg.push_str("</g>");
                }
            }
            if let Some(label) = edge.end_label.as_ref()
                && let Some((x, y)) = edge.end_label_anchor
            {
                let label_w = label.width * endpoint_label_scale;
                let label_h = label.height * endpoint_label_scale;
                let rect =
                    LabelRect::from_center(x, y, label_w, label_h, endpoint_pad_x, endpoint_pad_y);
                if endpoint_label_fill != "none" {
                    let visible = edge_label_background_visible(
                        layout.kind,
                        EdgeLabelKind::End,
                        &edge.points,
                        rect,
                    );
                    let fill_opacity = if visible { endpoint_fill_opacity } else { 0.0 };
                    let stroke_opacity = if visible {
                        endpoint_stroke_opacity
                    } else {
                        0.0
                    };
                    svg.push_str(&format!(
                        "<rect data-edge-id=\"{edge_id}\" data-label-kind=\"end\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" fill-opacity=\"{:.2}\" stroke=\"{}\" stroke-opacity=\"{:.2}\" stroke-width=\"0.8\"/>",
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        endpoint_label_fill,
                        fill_opacity,
                        theme.primary_border_color,
                        stroke_opacity
                    ));
                }
                if let Some(font_size) = endpoint_font_size {
                    svg.push_str(&format!(
                        "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"end\">"
                    ));
                    svg.push_str(&text_block_svg_with_font_size(
                        x,
                        y,
                        label,
                        theme,
                        config,
                        font_size,
                        "middle",
                        Some(label_color),
                        false,
                    ));
                    svg.push_str("</g>");
                } else {
                    svg.push_str(&format!(
                        "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"end\">"
                    ));
                    svg.push_str(&text_block_svg(
                        x,
                        y,
                        label,
                        theme,
                        config,
                        false,
                        Some(label_color),
                    ));
                    svg.push_str("</g>");
                }
            }
        }
    }

    if !is_sequence {
        let mut nodes_to_draw: Vec<&crate::layout::NodeLayout> =
            if layout.kind == crate::ir::DiagramKind::Treemap {
                let mut nodes: Vec<&crate::layout::NodeLayout> = layout.nodes.values().collect();
                nodes.sort_by(|a, b| {
                    let area_a = a.width * a.height;
                    let area_b = b.width * b.height;
                    area_b.partial_cmp(&area_a).unwrap_or(Ordering::Equal)
                });
                nodes
            } else {
                layout.nodes.values().collect()
            };

        for node in nodes_to_draw.drain(..) {
            if node.hidden {
                continue;
            }
            if node.anchor_subgraph.is_some() {
                continue;
            }
            if let Some(link) = node.link.as_ref() {
                svg.push_str(&format!("<a {}>", link_attrs(link)));
                if let Some(title) = link.title.as_deref() {
                    svg.push_str(&format!("<title>{}</title>", escape_xml(title)));
                }
            }
            if layout.kind == crate::ir::DiagramKind::Er {
                svg.push_str(&render_er_node(node, theme, config));
                if node.link.is_some() {
                    svg.push_str("</a>");
                }
                continue;
            }
            if layout.kind == crate::ir::DiagramKind::Kanban {
                svg.push_str(&render_kanban_item_node(node, theme, config));
                if node.link.is_some() {
                    svg.push_str("</a>");
                }
                continue;
            }
            if layout.kind == crate::ir::DiagramKind::Treemap {
                svg.push_str(&treemap_shape_svg(node, theme));
            } else {
                svg.push_str(&shape_svg(node, theme, config, layout.kind));
            }
            if layout.kind != crate::ir::DiagramKind::Er {
                let divider_line_height = if layout.kind == crate::ir::DiagramKind::Class {
                    theme.font_size * config.class_diagram_label_line_height()
                } else {
                    theme.font_size * config.label_line_height
                };
                svg.push_str(&divider_lines_svg(
                    node,
                    theme,
                    divider_line_height,
                    layout.kind == crate::ir::DiagramKind::Class,
                    layout.kind == crate::ir::DiagramKind::Class,
                ));
            }
            let mut center_x = node.x + node.width / 2.0;
            if node.shape == crate::ir::NodeShape::Asymmetric {
                center_x += left_inv_arrow_notch(node.width, node.height) / 2.0;
            } else if node.shape == crate::ir::NodeShape::OddShape {
                center_x += flowchart_odd_notch(node.height) / 2.0;
            }
            let center_y = node.y + node.height / 2.0;
            let hide_label = node
                .label
                .lines
                .iter()
                .all(|line| line.text().trim().is_empty())
                || node.id.starts_with("__start_")
                || node.id.starts_with("__end_")
                || (layout.kind == crate::ir::DiagramKind::Flowchart
                    && matches!(
                        node.shape,
                        crate::ir::NodeShape::CrossedCircle
                            | crate::ir::NodeShape::Hourglass
                            | crate::ir::NodeShape::LightningBolt
                    ));
            if !hide_label {
                let label_svg = if layout.kind == crate::ir::DiagramKind::Treemap {
                    if node.is_treemap_leaf {
                        treemap_leaf_label_svg(node, theme)
                    } else {
                        treemap_section_label_svg(node, theme)
                    }
                } else if layout.kind == crate::ir::DiagramKind::Er {
                    render_er_node_label(node, theme, config).unwrap_or_else(|| {
                        if node
                            .label
                            .lines
                            .iter()
                            .any(|line| is_divider_text_line(line))
                        {
                            text_block_svg_class(
                                node,
                                theme,
                                config,
                                node.style.text_color.as_deref(),
                            )
                        } else {
                            text_block_svg(
                                center_x,
                                center_y,
                                &node.label,
                                theme,
                                config,
                                false,
                                node.style.text_color.as_deref(),
                            )
                        }
                    })
                } else if node.shape == crate::ir::NodeShape::Note {
                    text_block_svg_with_font_size(
                        node.x + 6.0,
                        center_y,
                        &node.label,
                        theme,
                        config,
                        theme.font_size,
                        "start",
                        node.style.text_color.as_deref(),
                        false,
                    )
                } else if layout.kind == crate::ir::DiagramKind::Flowchart
                    && is_flowchart_icon_shape(node.shape)
                {
                    let label_y = flowchart_icon_label_center_y(node);
                    text_block_svg(
                        center_x,
                        label_y,
                        &node.label,
                        theme,
                        config,
                        false,
                        node.style.text_color.as_deref(),
                    )
                } else if layout.kind == crate::ir::DiagramKind::Flowchart
                    && node.shape == crate::ir::NodeShape::HorizontalCylinder
                {
                    let label_offset_x = flowchart_tilted_cylinder_rx(node.height);
                    text_block_svg(
                        center_x - label_offset_x,
                        center_y,
                        &node.label,
                        theme,
                        config,
                        false,
                        node.style.text_color.as_deref(),
                    )
                } else if layout.kind == crate::ir::DiagramKind::Flowchart
                    && node.shape == crate::ir::NodeShape::Document
                {
                    text_block_svg(
                        center_x,
                        center_y - flowchart_wave_document_amplitude(node.height),
                        &node.label,
                        theme,
                        config,
                        false,
                        node.style.text_color.as_deref(),
                    )
                } else if layout.kind == crate::ir::DiagramKind::Flowchart
                    && node.shape == crate::ir::NodeShape::LinedDocument
                {
                    let wave_amplitude = flowchart_wave_document_amplitude(node.height);
                    let body_width = node.width / 1.1;
                    text_block_svg(
                        center_x + body_width * 0.025,
                        center_y - wave_amplitude / 2.0,
                        &node.label,
                        theme,
                        config,
                        false,
                        node.style.text_color.as_deref(),
                    )
                } else if layout.kind == crate::ir::DiagramKind::Flowchart
                    && node.shape == crate::ir::NodeShape::TagDocument
                {
                    text_block_svg(
                        center_x,
                        center_y - flowchart_wave_document_amplitude(node.height) / 2.0,
                        &node.label,
                        theme,
                        config,
                        false,
                        node.style.text_color.as_deref(),
                    )
                } else if layout.kind == crate::ir::DiagramKind::Flowchart
                    && node.shape == crate::ir::NodeShape::LinedCylinder
                {
                    text_block_svg(
                        center_x,
                        center_y + flowchart_cylinder_ry(node.width),
                        &node.label,
                        theme,
                        config,
                        false,
                        node.style.text_color.as_deref(),
                    )
                } else if layout.kind == crate::ir::DiagramKind::Flowchart
                    && node.shape == crate::ir::NodeShape::DividedRect
                {
                    text_block_svg(
                        center_x,
                        center_y + flowchart_divided_rect_offset(node.height) / 2.0,
                        &node.label,
                        theme,
                        config,
                        false,
                        node.style.text_color.as_deref(),
                    )
                } else if layout.kind == crate::ir::DiagramKind::Flowchart
                    && node.shape == crate::ir::NodeShape::SlopedRect
                {
                    text_block_svg(
                        center_x,
                        center_y + node.height / 6.0,
                        &node.label,
                        theme,
                        config,
                        false,
                        node.style.text_color.as_deref(),
                    )
                } else if layout.kind == crate::ir::DiagramKind::Flowchart
                    && node.shape == crate::ir::NodeShape::WindowPane
                {
                    text_block_svg(
                        center_x + FLOWCHART_WINDOW_PANE_OFFSET / 2.0,
                        center_y + FLOWCHART_WINDOW_PANE_OFFSET / 2.0,
                        &node.label,
                        theme,
                        config,
                        false,
                        node.style.text_color.as_deref(),
                    )
                } else if layout.kind == crate::ir::DiagramKind::Flowchart
                    && node.shape == crate::ir::NodeShape::BraceLeft
                {
                    text_block_svg(
                        center_x - flowchart_curly_brace_radius(node.height) / 2.0,
                        center_y,
                        &node.label,
                        theme,
                        config,
                        false,
                        node.style.text_color.as_deref(),
                    )
                } else if node
                    .label
                    .lines
                    .iter()
                    .any(|line| is_divider_text_line(line))
                {
                    text_block_svg_class(node, theme, config, node.style.text_color.as_deref())
                } else if layout.kind == crate::ir::DiagramKind::State {
                    text_block_svg_with_font_size(
                        center_x,
                        center_y,
                        &node.label,
                        theme,
                        config,
                        state_font_size,
                        "middle",
                        node.style.text_color.as_deref(),
                        false,
                    )
                } else {
                    text_block_svg(
                        center_x,
                        center_y,
                        &node.label,
                        theme,
                        config,
                        false,
                        node.style.text_color.as_deref(),
                    )
                };
                svg.push_str(&label_svg);
            }
            if node.link.is_some() {
                svg.push_str("</a>");
            }
        }

        if overlay_flowchart && !overlay_arrows.is_empty() {
            for (is_start, point, angle, stroke, stroke_width) in overlay_arrows {
                let final_angle = if is_start { angle + 180.0 } else { angle };
                svg.push_str(&arrowhead_svg(
                    point,
                    final_angle,
                    stroke.as_str(),
                    stroke_width,
                ));
            }
        }
        if layout.kind == crate::ir::DiagramKind::Er && !er_decoration_overlays.is_empty() {
            svg.push_str("<g class=\"erEdgeDecorations\">");
            for decoration in er_decoration_overlays {
                svg.push_str(&decoration);
            }
            svg.push_str("</g>");
        }

        for footbox in seq_data.map(|s| s.footboxes.as_slice()).unwrap_or_default() {
            if let Some(link) = footbox.link.as_ref() {
                svg.push_str(&format!("<a {}>", link_attrs(link)));
                if let Some(title) = link.title.as_deref() {
                    svg.push_str(&format!("<title>{}</title>", escape_xml(title)));
                }
            }
            svg.push_str(&shape_svg(
                footbox,
                theme,
                config,
                crate::ir::DiagramKind::Sequence,
            ));
            let divider_line_height = theme.font_size * config.label_line_height;
            svg.push_str(&divider_lines_svg(
                footbox,
                theme,
                divider_line_height,
                false,
                false,
            ));
            let center_x = footbox.x + footbox.width / 2.0;
            let center_y = footbox.y + footbox.height / 2.0;
            let hide_label = footbox
                .label
                .lines
                .iter()
                .all(|line| line.text().trim().is_empty())
                || footbox.id.starts_with("__start_")
                || footbox.id.starts_with("__end_");
            if !hide_label {
                let label_svg = if footbox
                    .label
                    .lines
                    .iter()
                    .any(|line| is_divider_text_line(line))
                {
                    text_block_svg_class(
                        footbox,
                        theme,
                        config,
                        footbox.style.text_color.as_deref(),
                    )
                } else {
                    text_block_svg(
                        center_x,
                        center_y,
                        &footbox.label,
                        theme,
                        config,
                        false,
                        footbox.style.text_color.as_deref(),
                    )
                };
                svg.push_str(&label_svg);
            }
            if footbox.link.is_some() {
                svg.push_str("</a>");
            }
        }
    } else {
        for node in layout.nodes.values() {
            if node.hidden {
                continue;
            }
            if node.anchor_subgraph.is_some() {
                continue;
            }
            if let Some(link) = node.link.as_ref() {
                svg.push_str(&format!("<a {}>", link_attrs(link)));
                if let Some(title) = link.title.as_deref() {
                    svg.push_str(&format!("<title>{}</title>", escape_xml(title)));
                }
            }
            render_sequence_actor_shape(&mut svg, node, theme, config, false);
            if node.link.is_some() {
                svg.push_str("</a>");
            }
        }
        for footbox in seq_data.map(|s| s.footboxes.as_slice()).unwrap_or_default() {
            if let Some(link) = footbox.link.as_ref() {
                svg.push_str(&format!("<a {}>", link_attrs(link)));
                if let Some(title) = link.title.as_deref() {
                    svg.push_str(&format!("<title>{}</title>", escape_xml(title)));
                }
            }
            render_sequence_actor_shape(&mut svg, footbox, theme, config, false);
            if footbox.link.is_some() {
                svg.push_str("</a>");
            }
        }
    }

    svg.push_str("</svg>");
    svg
}

fn points_to_path(points: &[(f32, f32)]) -> String {
    points_to_curved_path(points, crate::ir::CurveType::Linear)
}

fn points_to_curved_path(points: &[(f32, f32)], curve: crate::ir::CurveType) -> String {
    if points.is_empty() {
        return String::new();
    }
    let deduped = dedupe_points(points);
    if deduped.len() == 1 {
        return format!("M {:.3},{:.3}", deduped[0].0, deduped[0].1);
    }
    // Iter 264: for non-linear curves, interpolate intermediate waypoints
    // along long segments. This converts straight-line cubic Beziers into
    // multi-segment Beziers with smoother arrowhead tangents, matching JS
    // dagre's M..L..C..C..L SVG markup style.
    let interpolated;
    let pts = if !matches!(curve, crate::ir::CurveType::Linear) && deduped.len() <= 4 {
        // Compute total polyline length.
        let mut total_len = 0.0f32;
        for w in deduped.windows(2) {
            let (sx, sy) = w[0];
            let (ex, ey) = w[1];
            total_len += ((ex - sx).powi(2) + (ey - sy).powi(2)).sqrt();
        }
        // Only interpolate if total length is meaningful (>80px) and we'd
        // benefit (currently <5 input points → at most single C from
        // curve_tangent_bezier).
        if total_len > 80.0 {
            // For each segment, insert 1-2 intermediate points so total
            // becomes 5+. Distribute proportionally to segment length.
            let mut new_pts: Vec<(f32, f32)> = Vec::with_capacity(deduped.len() + 4);
            new_pts.push(deduped[0]);
            for w in deduped.windows(2) {
                let (sx, sy) = w[0];
                let (ex, ey) = w[1];
                let seg_len = ((ex - sx).powi(2) + (ey - sy).powi(2)).sqrt();
                let n_inserts = if seg_len > 100.0 {
                    3
                } else if seg_len > 50.0 {
                    2
                } else if seg_len > 20.0 {
                    1
                } else {
                    0
                };
                let step = 1.0 / (n_inserts as f32 + 1.0);
                for i in 1..=n_inserts {
                    let t = step * i as f32;
                    new_pts.push((sx + (ex - sx) * t, sy + (ey - sy) * t));
                }
                new_pts.push((ex, ey));
            }
            interpolated = new_pts;
            &interpolated
        } else {
            &deduped
        }
    } else {
        &deduped
    };
    match curve {
        crate::ir::CurveType::Linear => {
            let mut d = format!("M {:.3},{:.3}", pts[0].0, pts[0].1);
            for (x, y) in pts.iter().skip(1) {
                d.push_str(&format!(" L {:.3},{:.3}", x, y));
            }
            d
        }
        crate::ir::CurveType::Basis => curve_basis(pts),
        crate::ir::CurveType::Step => curve_step(pts, 0.5),
        crate::ir::CurveType::StepBefore => curve_step(pts, 0.0),
        crate::ir::CurveType::StepAfter => curve_step(pts, 1.0),
        crate::ir::CurveType::Natural => curve_natural(pts),
        crate::ir::CurveType::MonotoneX | crate::ir::CurveType::BumpX => curve_monotone_x(pts),
        crate::ir::CurveType::MonotoneY | crate::ir::CurveType::BumpY => curve_monotone_y(pts),
        crate::ir::CurveType::Cardinal | crate::ir::CurveType::CatmullRom => {
            curve_cardinal(pts, 0.5)
        }
    }
}

fn points_to_d3_basis_path(points: &[(f32, f32)]) -> String {
    if points.is_empty() {
        return String::new();
    }
    let pts = dedupe_points(points);
    match pts.len() {
        0 => return String::new(),
        1 => return format!("M {:.3},{:.3}", pts[0].0, pts[0].1),
        2 => {
            return format!(
                "M {:.3},{:.3} L {:.3},{:.3}",
                pts[0].0, pts[0].1, pts[1].0, pts[1].1
            );
        }
        _ => {}
    }

    let mut d = format!("M {:.3},{:.3}", pts[0].0, pts[0].1);
    let first_line = (
        (5.0 * pts[0].0 + pts[1].0) / 6.0,
        (5.0 * pts[0].1 + pts[1].1) / 6.0,
    );
    d.push_str(&format!(" L {:.3},{:.3}", first_line.0, first_line.1));

    for i in 2..pts.len() {
        let p0 = pts[i - 2];
        let p1 = pts[i - 1];
        let p2 = pts[i];
        let c1 = ((2.0 * p0.0 + p1.0) / 3.0, (2.0 * p0.1 + p1.1) / 3.0);
        let c2 = ((p0.0 + 2.0 * p1.0) / 3.0, (p0.1 + 2.0 * p1.1) / 3.0);
        let end = (
            (p0.0 + 4.0 * p1.0 + p2.0) / 6.0,
            (p0.1 + 4.0 * p1.1 + p2.1) / 6.0,
        );
        d.push_str(&format!(
            " C {:.3},{:.3} {:.3},{:.3} {:.3},{:.3}",
            c1.0, c1.1, c2.0, c2.1, end.0, end.1
        ));
    }

    let n = pts.len();
    let p0 = pts[n - 2];
    let p1 = pts[n - 1];
    let c1 = ((2.0 * p0.0 + p1.0) / 3.0, (2.0 * p0.1 + p1.1) / 3.0);
    let c2 = ((p0.0 + 2.0 * p1.0) / 3.0, (p0.1 + 2.0 * p1.1) / 3.0);
    let end = ((p0.0 + 5.0 * p1.0) / 6.0, (p0.1 + 5.0 * p1.1) / 6.0);
    d.push_str(&format!(
        " C {:.3},{:.3} {:.3},{:.3} {:.3},{:.3} L {:.3},{:.3}",
        c1.0, c1.1, c2.0, c2.1, end.0, end.1, p1.0, p1.1
    ));
    d
}

fn flowchart_d3_basis_points(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let pts = dedupe_points(points);
    if pts.len() == 2 {
        let mid = ((pts[0].0 + pts[1].0) * 0.5, (pts[0].1 + pts[1].1) * 0.5);
        vec![pts[0], mid, pts[1]]
    } else {
        pts
    }
}

fn flowchart_marker_offset_render_points(
    points: &[(f32, f32)],
    edge: &crate::layout::EdgeLayout,
) -> Vec<(f32, f32)> {
    const ARROW_POINT_OFFSET: f32 = 4.0;

    let mut adjusted = points.to_vec();
    if adjusted.len() < 2 {
        return adjusted;
    }

    if edge.arrow_start {
        offset_endpoint(&mut adjusted, true, ARROW_POINT_OFFSET);
    }
    if edge.arrow_end {
        offset_endpoint(&mut adjusted, false, ARROW_POINT_OFFSET);
    }

    adjusted
}

fn offset_endpoint(points: &mut [(f32, f32)], start: bool, offset: f32) {
    if points.len() < 2 || offset == 0.0 {
        return;
    }

    let (endpoint_idx, adjacent_idx) = if start {
        (0, 1)
    } else {
        (points.len() - 1, points.len() - 2)
    };
    let endpoint = points[endpoint_idx];
    let adjacent = points[adjacent_idx];
    let dx = adjacent.0 - endpoint.0;
    let dy = adjacent.1 - endpoint.1;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= f32::EPSILON {
        return;
    }

    points[endpoint_idx].0 += offset * dx / len;
    points[endpoint_idx].1 += offset * dy / len;
}

/// B-spline (basis) curve through points.
///
/// For short paths (≤ 5 points) this uses cubic Bezier curves that
/// guarantee the tangent at each endpoint matches the first/last
/// segment direction — so SVG arrowheads (orient="auto") are always
/// perpendicular to the node border.
///
/// For longer paths the classic B-spline approximation is used with
/// a corrected closing segment that respects the final approach
/// direction.
fn curve_basis(pts: &[(f32, f32)]) -> String {
    if pts.len() < 3 {
        let mut d = format!("M {:.3},{:.3}", pts[0].0, pts[0].1);
        for p in pts.iter().skip(1) {
            d.push_str(&format!(" L {:.3},{:.3}", p.0, p.1));
        }
        return d;
    }

    // Use a tangent-preserving cubic Bezier approach for most paths.
    // This produces the smoothest possible curve while guaranteeing
    // arrowhead perpendicularity at both ends.
    if pts.len() <= 8 {
        return curve_tangent_bezier(pts);
    }

    // Longer paths: classic B-spline with a corrected closing segment.
    let mut d = format!("M {:.3},{:.3}", pts[0].0, pts[0].1);
    let x1 = (2.0 * pts[0].0 + pts[1].0) / 3.0;
    let y1 = (2.0 * pts[0].1 + pts[1].1) / 3.0;
    let x2 = (pts[0].0 + 2.0 * pts[1].0) / 3.0;
    let y2 = (pts[0].1 + 2.0 * pts[1].1) / 3.0;
    let mx = (x2 + (pts[1].0 + 2.0 * pts[2].0) / 3.0) / 2.0;
    let my = (y2 + (pts[1].1 + 2.0 * pts[2].1) / 3.0) / 2.0;
    d.push_str(&format!(
        " C {x1:.3},{y1:.3} {x2:.3},{y2:.3} {mx:.3},{my:.3}"
    ));
    let mut last_ex = mx;
    let mut last_ey = my;
    for i in 2..pts.len() - 1 {
        let p0 = pts[i - 1];
        let p1 = pts[i];
        let p2 = pts[i + 1];
        let cx2 = (p0.0 + 2.0 * p1.0) / 3.0;
        let cy2 = (p0.1 + 2.0 * p1.1) / 3.0;
        let nx1 = (p1.0 + 2.0 * p2.0) / 3.0;
        let ny1 = (p1.1 + 2.0 * p2.1) / 3.0;
        let ex = (cx2 + nx1) / 2.0;
        let ey = (cy2 + ny1) / 2.0;
        d.push_str(&format!(" S {cx2:.3},{cy2:.3} {ex:.3},{ey:.3}"));
        last_ex = ex;
        last_ey = ey;
    }
    // Close with a cubic Bezier that arrives at the endpoint from the
    // approach direction (second-to-last → last point), ensuring the
    // arrowhead is perpendicular.
    let last = pts[pts.len() - 1];
    let approach = pts[pts.len() - 2];
    let cp2x = last.0 + (approach.0 - last.0) / 3.0;
    let cp2y = last.1 + (approach.1 - last.1) / 3.0;
    // Reflect the previous control point for C1 continuity.
    let cp1x = 2.0 * last_ex - (pts[pts.len() - 3].0 + 2.0 * approach.0) / 3.0;
    let cp1y = 2.0 * last_ey - (pts[pts.len() - 3].1 + 2.0 * approach.1) / 3.0;
    d.push_str(&format!(
        " C {cp1x:.3},{cp1y:.3} {cp2x:.3},{cp2y:.3} {:.3},{:.3}",
        last.0, last.1
    ));
    d
}

/// Construct a smooth cubic Bezier path for short edge paths (≤ 5 pts).
///
/// For a path `[start, depart, ..., approach, end]`:
/// - The tangent at `start` points toward `depart`
/// - The tangent at `end` comes from the direction `approach → end`
/// - Any intermediate points are interpolated with Catmull-Rom–style
///   tangents so the curve passes near them without sharp turns.
fn curve_tangent_bezier(pts: &[(f32, f32)]) -> String {
    let n = pts.len();
    debug_assert!(n >= 3);

    // 3 points: single cubic Bezier.
    // [start, mid, end] → tangent at start = (start→mid), tangent at end = (mid→end)
    if n == 3 {
        let (start, mid, end) = (pts[0], pts[1], pts[2]);
        // Control points at 1/3 and 2/3 of the way, aligned with tangents.
        let c1x = start.0 + (mid.0 - start.0) * 0.66;
        let c1y = start.1 + (mid.1 - start.1) * 0.66;
        let c2x = end.0 + (mid.0 - end.0) * 0.66;
        let c2y = end.1 + (mid.1 - end.1) * 0.66;
        return format!(
            "M {:.3},{:.3} C {c1x:.3},{c1y:.3} {c2x:.3},{c2y:.3} {:.3},{:.3}",
            start.0, start.1, end.0, end.1
        );
    }

    // 4 points: [start, depart, approach, end]
    // Use depart as first control point, approach as second → perfect
    // tangents at both ends.
    if n == 4 {
        let (start, depart, approach, end) = (pts[0], pts[1], pts[2], pts[3]);
        // Pass 9 Defect A fix: when all four points share the same axis
        // (collinear vertical or horizontal) AND the depart/approach
        // overshoot the start/end span, the router produced a degenerate
        // path that would render as a Bezier wobble. Collapse to a clean
        // straight `L` instead. Triggered for the
        // stateDiagram-nested-composite-states `Second→[*]_First` edge
        // where waypoints zigzagged ±6 px across a 7-px endpoint span.
        let collinear_x = (start.0 - depart.0).abs() < 0.1
            && (start.0 - approach.0).abs() < 0.1
            && (start.0 - end.0).abs() < 0.1;
        let collinear_y = (start.1 - depart.1).abs() < 0.1
            && (start.1 - approach.1).abs() < 0.1
            && (start.1 - end.1).abs() < 0.1;
        if collinear_x || collinear_y {
            let (lo, hi) = if collinear_x {
                (start.1.min(end.1), start.1.max(end.1))
            } else {
                (start.0.min(end.0), start.0.max(end.0))
            };
            let depart_v = if collinear_x { depart.1 } else { depart.0 };
            let approach_v = if collinear_x { approach.1 } else { approach.0 };
            let depart_outside = depart_v < lo - 0.1 || depart_v > hi + 0.1;
            let approach_outside = approach_v < lo - 0.1 || approach_v > hi + 0.1;
            if depart_outside || approach_outside {
                return format!(
                    "M {:.3},{:.3} L {:.3},{:.3}",
                    start.0, start.1, end.0, end.1
                );
            }
        }
        return format!(
            "M {:.3},{:.3} C {:.3},{:.3} {:.3},{:.3} {:.3},{:.3}",
            start.0, start.1, depart.0, depart.1, approach.0, approach.1, end.0, end.1
        );
    }

    // 5+ points: Catmull-Rom–style multi-segment cubic Bezier.
    // Compute tangents at each point, then build one cubic Bezier per
    // segment.  The first tangent = (pts[0]→pts[1]), the last tangent
    // = (pts[n-2]→pts[n-1]), and interior tangents are estimated from
    // the neighbouring points.
    let mut tangents: Vec<(f32, f32)> = Vec::with_capacity(n);
    // First tangent: departure direction.
    tangents.push((pts[1].0 - pts[0].0, pts[1].1 - pts[0].1));
    for i in 1..n - 1 {
        tangents.push((
            (pts[i + 1].0 - pts[i - 1].0) * 0.5,
            (pts[i + 1].1 - pts[i - 1].1) * 0.5,
        ));
    }
    // Last tangent: approach direction.
    tangents.push((pts[n - 1].0 - pts[n - 2].0, pts[n - 1].1 - pts[n - 2].1));

    // Compute the bounding box of all edge waypoints to clamp control
    // points and prevent the curve from swinging outside the SVG.
    let mut bb_min_x = f32::MAX;
    let mut bb_min_y = f32::MAX;
    let mut bb_max_x = f32::MIN;
    let mut bb_max_y = f32::MIN;
    for p in pts {
        bb_min_x = bb_min_x.min(p.0);
        bb_min_y = bb_min_y.min(p.1);
        bb_max_x = bb_max_x.max(p.0);
        bb_max_y = bb_max_y.max(p.1);
    }
    // Allow control points to extend up to 20% beyond the waypoint
    // bounding box for natural-looking curves.
    let margin_x = (bb_max_x - bb_min_x) * 0.20 + 8.0;
    let margin_y = (bb_max_y - bb_min_y) * 0.20 + 8.0;
    let clamp_min_x = bb_min_x - margin_x;
    let clamp_max_x = bb_max_x + margin_x;
    let clamp_min_y = bb_min_y - margin_y;
    let clamp_max_y = bb_max_y + margin_y;

    let mut d = format!("M {:.3},{:.3}", pts[0].0, pts[0].1);
    for i in 0..n - 1 {
        let p0 = pts[i];
        let p1 = pts[i + 1];
        // Scale tangents by 1/3 of the segment length for natural-looking curves.
        let seg_len = ((p1.0 - p0.0).powi(2) + (p1.1 - p0.1).powi(2)).sqrt();
        let t0_len = (tangents[i].0.powi(2) + tangents[i].1.powi(2))
            .sqrt()
            .max(1e-6);
        let t1_len = (tangents[i + 1].0.powi(2) + tangents[i + 1].1.powi(2))
            .sqrt()
            .max(1e-6);
        let scale0 = seg_len / (3.0 * t0_len);
        let scale1 = seg_len / (3.0 * t1_len);
        let c1x = (p0.0 + tangents[i].0 * scale0).clamp(clamp_min_x, clamp_max_x);
        let c1y = (p0.1 + tangents[i].1 * scale0).clamp(clamp_min_y, clamp_max_y);
        let c2x = (p1.0 - tangents[i + 1].0 * scale1).clamp(clamp_min_x, clamp_max_x);
        let c2y = (p1.1 - tangents[i + 1].1 * scale1).clamp(clamp_min_y, clamp_max_y);
        d.push_str(&format!(
            " C {c1x:.3},{c1y:.3} {c2x:.3},{c2y:.3} {:.3},{:.3}",
            p1.0, p1.1
        ));
    }
    d
}

/// Step curve: horizontal-then-vertical (or vice versa) with a configurable t.
fn curve_step(pts: &[(f32, f32)], t: f32) -> String {
    let mut d = format!("M {:.3},{:.3}", pts[0].0, pts[0].1);
    for i in 1..pts.len() {
        let (x0, y0) = pts[i - 1];
        let (x1, y1) = pts[i];
        let mx = x0 + (x1 - x0) * t;
        let my = y0 + (y1 - y0) * t;
        if (t - 0.0).abs() < 0.01 {
            // stepBefore: vertical first, then horizontal
            d.push_str(&format!(" V {y1:.3} H {x1:.3}"));
        } else if (t - 1.0).abs() < 0.01 {
            // stepAfter: horizontal first, then vertical
            d.push_str(&format!(" H {x1:.3} V {y1:.3}"));
        } else {
            // step: midpoint split
            d.push_str(&format!(" H {mx:.3} V {my:.3} H {x1:.3} V {y1:.3}"));
        }
    }
    d
}

/// Natural cubic spline through points.
fn curve_natural(pts: &[(f32, f32)]) -> String {
    if pts.len() < 3 {
        let mut d = format!("M {:.3},{:.3}", pts[0].0, pts[0].1);
        for p in pts.iter().skip(1) {
            d.push_str(&format!(" L {:.3},{:.3}", p.0, p.1));
        }
        return d;
    }
    let n = pts.len() - 1;
    // Solve for cubic spline coefficients
    let xs: Vec<f32> = pts.iter().map(|p| p.0).collect();
    let ys: Vec<f32> = pts.iter().map(|p| p.1).collect();
    let (cx1x, cx2x) = natural_spline_control_points(&xs);
    let (cx1y, cx2y) = natural_spline_control_points(&ys);
    let mut d = format!("M {:.3},{:.3}", pts[0].0, pts[0].1);
    for i in 0..n {
        d.push_str(&format!(
            " C {:.3},{:.3} {:.3},{:.3} {:.3},{:.3}",
            cx1x[i],
            cx1y[i],
            cx2x[i],
            cx2y[i],
            pts[i + 1].0,
            pts[i + 1].1
        ));
    }
    d
}

/// Compute natural cubic spline control points for one dimension.
fn natural_spline_control_points(k: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let n = k.len() - 1;
    if n == 0 {
        return (vec![], vec![]);
    }
    let mut a = vec![0.0f32; n];
    let mut b = vec![0.0f32; n];
    let mut c = vec![0.0f32; n];
    let mut r = vec![0.0f32; n];
    a[0] = 0.0;
    b[0] = 2.0;
    c[0] = 1.0;
    r[0] = k[0] + 2.0 * k[1];
    for i in 1..n - 1 {
        a[i] = 1.0;
        b[i] = 4.0;
        c[i] = 1.0;
        r[i] = 4.0 * k[i] + 2.0 * k[i + 1];
    }
    if n > 1 {
        a[n - 1] = 2.0;
        b[n - 1] = 7.0;
        c[n - 1] = 0.0;
        r[n - 1] = 8.0 * k[n - 1] + k[n];
    }
    // Forward sweep
    for i in 1..n {
        let m = a[i] / b[i - 1];
        b[i] -= m * c[i - 1];
        r[i] -= m * r[i - 1];
    }
    // Back substitution
    let mut p1 = vec![0.0f32; n];
    p1[n - 1] = r[n - 1] / b[n - 1];
    for i in (0..n - 1).rev() {
        p1[i] = (r[i] - c[i] * p1[i + 1]) / b[i];
    }
    let mut p2 = vec![0.0f32; n];
    for i in 0..n - 1 {
        p2[i] = 2.0 * k[i + 1] - p1[i + 1];
    }
    p2[n - 1] = (k[n] + p1[n - 1]) / 2.0;
    (p1, p2)
}

/// Monotone X cubic interpolation (Fritsch-Carlson).
fn curve_monotone_x(pts: &[(f32, f32)]) -> String {
    if pts.len() < 3 {
        let mut d = format!("M {:.3},{:.3}", pts[0].0, pts[0].1);
        for p in pts.iter().skip(1) {
            d.push_str(&format!(" L {:.3},{:.3}", p.0, p.1));
        }
        return d;
    }
    let n = pts.len();
    let mut tangents = vec![0.0f32; n];
    let mut deltas = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let dx = pts[i + 1].0 - pts[i].0;
        let dy = pts[i + 1].1 - pts[i].1;
        deltas.push(if dx.abs() < 1e-6 { 0.0 } else { dy / dx });
    }
    tangents[0] = deltas[0];
    for i in 1..n - 1 {
        if deltas[i - 1].signum() != deltas[i].signum() || deltas[i].abs() < 1e-6 {
            tangents[i] = 0.0;
        } else {
            tangents[i] = (deltas[i - 1] + deltas[i]) / 2.0;
        }
    }
    tangents[n - 1] = deltas[n - 2];
    let mut d = format!("M {:.3},{:.3}", pts[0].0, pts[0].1);
    for i in 0..n - 1 {
        let dx = pts[i + 1].0 - pts[i].0;
        let cx1 = pts[i].0 + dx / 3.0;
        let cy1 = pts[i].1 + tangents[i] * dx / 3.0;
        let cx2 = pts[i + 1].0 - dx / 3.0;
        let cy2 = pts[i + 1].1 - tangents[i + 1] * dx / 3.0;
        d.push_str(&format!(
            " C {cx1:.3},{cy1:.3} {cx2:.3},{cy2:.3} {:.3},{:.3}",
            pts[i + 1].0,
            pts[i + 1].1,
        ));
    }
    d
}

/// Monotone Y cubic interpolation (transpose of monotone X).
fn curve_monotone_y(pts: &[(f32, f32)]) -> String {
    // Swap x/y, run monotone_x, swap back in the path
    if pts.len() < 3 {
        let mut d = format!("M {:.3},{:.3}", pts[0].0, pts[0].1);
        for p in pts.iter().skip(1) {
            d.push_str(&format!(" L {:.3},{:.3}", p.0, p.1));
        }
        return d;
    }
    let swapped: Vec<(f32, f32)> = pts.iter().map(|&(x, y)| (y, x)).collect();
    let n = swapped.len();
    let mut tangents = vec![0.0f32; n];
    let mut deltas = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let dx = swapped[i + 1].0 - swapped[i].0;
        let dy = swapped[i + 1].1 - swapped[i].1;
        deltas.push(if dx.abs() < 1e-6 { 0.0 } else { dy / dx });
    }
    tangents[0] = deltas[0];
    for i in 1..n - 1 {
        if deltas[i - 1].signum() != deltas[i].signum() || deltas[i].abs() < 1e-6 {
            tangents[i] = 0.0;
        } else {
            tangents[i] = (deltas[i - 1] + deltas[i]) / 2.0;
        }
    }
    tangents[n - 1] = deltas[n - 2];
    let mut d = format!("M {:.3},{:.3}", pts[0].0, pts[0].1);
    for i in 0..n - 1 {
        let dy = pts[i + 1].1 - pts[i].1;
        let cx1 = pts[i].0 + tangents[i] * dy / 3.0;
        let cy1 = pts[i].1 + dy / 3.0;
        let cx2 = pts[i + 1].0 - tangents[i + 1] * dy / 3.0;
        let cy2 = pts[i + 1].1 - dy / 3.0;
        d.push_str(&format!(
            " C {cx1:.3},{cy1:.3} {cx2:.3},{cy2:.3} {:.3},{:.3}",
            pts[i + 1].0,
            pts[i + 1].1,
        ));
    }
    d
}

/// Cardinal spline (Catmull-Rom variant) with tension parameter.
fn curve_cardinal(pts: &[(f32, f32)], tension: f32) -> String {
    if pts.len() < 3 {
        let mut d = format!("M {:.3},{:.3}", pts[0].0, pts[0].1);
        for p in pts.iter().skip(1) {
            d.push_str(&format!(" L {:.3},{:.3}", p.0, p.1));
        }
        return d;
    }
    let s = (1.0 - tension) / 2.0;
    let mut d = format!("M {:.3},{:.3}", pts[0].0, pts[0].1);
    for i in 0..pts.len() - 1 {
        let p0 = if i == 0 { pts[0] } else { pts[i - 1] };
        let p1 = pts[i];
        let p2 = pts[i + 1];
        let p3 = if i + 2 < pts.len() {
            pts[i + 2]
        } else {
            pts[i + 1]
        };
        let cx1 = p1.0 + s * (p2.0 - p0.0) / 3.0;
        let cy1 = p1.1 + s * (p2.1 - p0.1) / 3.0;
        let cx2 = p2.0 - s * (p3.0 - p1.0) / 3.0;
        let cy2 = p2.1 - s * (p3.1 - p1.1) / 3.0;
        d.push_str(&format!(
            " C {cx1:.3},{cy1:.3} {cx2:.3},{cy2:.3} {:.3},{:.3}",
            p2.0, p2.1,
        ));
    }
    d
}

/// Simple seeded PRNG for deterministic hand-drawn perturbations.
/// Uses a basic xorshift32 algorithm.
struct HandDrawnRng {
    state: u32,
}

impl HandDrawnRng {
    fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Returns a random f32 in [-1.0, 1.0].
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// Generate a seed from node position/dimensions for deterministic output.
fn hand_drawn_seed(x: f32, y: f32, w: f32, h: f32) -> u32 {
    let mut s = 2166136261u32;
    for v in [x, y, w, h] {
        s ^= v.to_bits();
        s = s.wrapping_mul(16777619);
    }
    if s == 0 { 1 } else { s }
}

/// Add slight jitter to an SVG path string for hand-drawn look.
/// Only perturbs coordinate values, preserving SVG path commands.
fn hand_drawn_path_jitter(path_d: &str, amplitude: f32, seed: u32) -> String {
    let mut rng = HandDrawnRng::new(seed);
    let mut result = String::with_capacity(path_d.len());
    let mut chars = path_d.chars().peekable();
    while let Some(&ch) = chars.peek() {
        if ch == '-' || ch == '.' || ch.is_ascii_digit() {
            // Parse a number
            let mut num_str = String::new();
            if ch == '-' {
                num_str.push(ch);
                chars.next();
            }
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() || c == '.' {
                    num_str.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            if let Ok(val) = num_str.parse::<f32>() {
                let jittered = val + rng.next_f32() * amplitude;
                result.push_str(&format!("{:.3}", jittered));
            } else {
                result.push_str(&num_str);
            }
        } else {
            result.push(ch);
            chars.next();
        }
    }
    result
}

fn dedupe_points(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut out = Vec::with_capacity(points.len());
    for point in points.iter().copied() {
        if out
            .last()
            .map(|prev: &(f32, f32)| {
                (prev.0 - point.0).abs() < 1e-3 && (prev.1 - point.1).abs() < 1e-3
            })
            .unwrap_or(false)
        {
            continue;
        }
        out.push(point);
    }
    out
}

#[derive(Debug, Clone, Copy)]
struct LabelRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl LabelRect {
    fn from_center(
        center_x: f32,
        center_y: f32,
        label_w: f32,
        label_h: f32,
        pad_x: f32,
        pad_y: f32,
    ) -> Self {
        let width = (label_w + pad_x * 2.0).max(0.0);
        let height = (label_h + pad_y * 2.0).max(0.0);
        Self {
            x: center_x - width * 0.5,
            y: center_y - height * 0.5,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgeLabelKind {
    Center,
    Start,
    End,
}

fn edge_label_background_visible(
    diagram_kind: crate::ir::DiagramKind,
    label_kind: EdgeLabelKind,
    edge_points: &[(f32, f32)],
    rect: LabelRect,
) -> bool {
    if edge_points.len() < 2 || rect.width <= 0.0 || rect.height <= 0.0 {
        return true;
    }
    let gap = polyline_rect_gap(edge_points, &rect);
    match label_kind {
        EdgeLabelKind::Center => {
            // State diagrams (per mermaid-js .edgeLabel rect{opacity:0.5}) always
            // render the label chip, regardless of edge proximity — the chip's
            // 50% fill prevents the curved bidirectional transitions from
            // visibly piercing the label text.
            if matches!(diagram_kind, crate::ir::DiagramKind::State) {
                return true;
            }
            let gap_limit = match diagram_kind {
                crate::ir::DiagramKind::Flowchart => 1.2,
                crate::ir::DiagramKind::Sequence => (rect.height * 0.16).clamp(1.2, 2.4),
                crate::ir::DiagramKind::Requirement => 1.0,
                _ => 0.9,
            };
            gap <= gap_limit
        }
        EdgeLabelKind::Start | EdgeLabelKind::End => match diagram_kind {
            crate::ir::DiagramKind::Sequence => gap <= (rect.height * 0.12).clamp(0.6, 1.4),
            crate::ir::DiagramKind::Flowchart | crate::ir::DiagramKind::Requirement => gap <= 0.35,
            _ => false,
        },
    }
}

fn polyline_rect_gap(points: &[(f32, f32)], rect: &LabelRect) -> f32 {
    if points.len() < 2 {
        return f32::INFINITY;
    }
    let mut best = f32::INFINITY;
    for segment in points.windows(2) {
        let dist = segment_rect_gap(segment[0], segment[1], rect);
        best = best.min(dist);
        if best <= 1e-6 {
            return 0.0;
        }
    }
    best
}

fn segment_rect_gap(a: (f32, f32), b: (f32, f32), rect: &LabelRect) -> f32 {
    if segment_intersects_rect(a, b, rect) {
        return 0.0;
    }
    let mut best = point_rect_distance(a, rect).min(point_rect_distance(b, rect));
    let corners = [
        (rect.x, rect.y),
        (rect.x + rect.width, rect.y),
        (rect.x + rect.width, rect.y + rect.height),
        (rect.x, rect.y + rect.height),
    ];
    for corner in corners {
        best = best.min(point_segment_distance(corner, a, b));
    }
    best
}

fn point_rect_distance(point: (f32, f32), rect: &LabelRect) -> f32 {
    let (px, py) = point;
    let x1 = rect.x;
    let y1 = rect.y;
    let x2 = rect.x + rect.width;
    let y2 = rect.y + rect.height;
    let dx = if px < x1 {
        x1 - px
    } else if px > x2 {
        px - x2
    } else {
        0.0
    };
    let dy = if py < y1 {
        y1 - py
    } else if py > y2 {
        py - y2
    } else {
        0.0
    };
    (dx * dx + dy * dy).sqrt()
}

fn point_segment_distance(point: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let ab_x = b.0 - a.0;
    let ab_y = b.1 - a.1;
    let len_sq = ab_x * ab_x + ab_y * ab_y;
    if len_sq <= 1e-9 {
        let dx = point.0 - a.0;
        let dy = point.1 - a.1;
        return (dx * dx + dy * dy).sqrt();
    }
    let t = ((point.0 - a.0) * ab_x + (point.1 - a.1) * ab_y) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let proj_x = a.0 + ab_x * t;
    let proj_y = a.1 + ab_y * t;
    let dx = point.0 - proj_x;
    let dy = point.1 - proj_y;
    (dx * dx + dy * dy).sqrt()
}

fn segment_intersects_rect(a: (f32, f32), b: (f32, f32), rect: &LabelRect) -> bool {
    if point_in_rect(a, rect) || point_in_rect(b, rect) {
        return true;
    }
    let corners = [
        (rect.x, rect.y),
        (rect.x + rect.width, rect.y),
        (rect.x + rect.width, rect.y + rect.height),
        (rect.x, rect.y + rect.height),
    ];
    let edges = [
        (corners[0], corners[1]),
        (corners[1], corners[2]),
        (corners[2], corners[3]),
        (corners[3], corners[0]),
    ];
    edges
        .iter()
        .any(|(c0, c1)| segments_intersect(a, b, *c0, *c1))
}

fn point_in_rect(point: (f32, f32), rect: &LabelRect) -> bool {
    point.0 >= rect.x
        && point.0 <= rect.x + rect.width
        && point.1 >= rect.y
        && point.1 <= rect.y + rect.height
}

fn segments_intersect(a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32)) -> bool {
    let eps = 1e-6;
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);

    if o1.abs() < eps && on_segment(a, b, c, eps) {
        return true;
    }
    if o2.abs() < eps && on_segment(a, b, d, eps) {
        return true;
    }
    if o3.abs() < eps && on_segment(c, d, a, eps) {
        return true;
    }
    if o4.abs() < eps && on_segment(c, d, b, eps) {
        return true;
    }

    (o1 > 0.0) != (o2 > 0.0) && (o3 > 0.0) != (o4 > 0.0)
}

fn orient(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn on_segment(a: (f32, f32), b: (f32, f32), c: (f32, f32), eps: f32) -> bool {
    c.0 >= a.0.min(b.0) - eps
        && c.0 <= a.0.max(b.0) + eps
        && c.1 >= a.1.min(b.1) - eps
        && c.1 <= a.1.max(b.1) + eps
}

fn format_sankey_value(value: f32) -> String {
    let rounded_2 = (value * 100.0).round() / 100.0;
    if (rounded_2 - rounded_2.round()).abs() < 0.001 {
        return format!("{rounded_2:.0}");
    }
    let rounded_1 = (value * 10.0).round() / 10.0;
    if (rounded_1 - rounded_2).abs() < 0.001 {
        format!("{rounded_1:.1}")
    } else {
        format!("{rounded_2:.2}")
    }
}

fn render_sankey(layout: &SankeyLayout, _theme: &Theme, _config: &LayoutConfig) -> String {
    let mut svg = String::new();

    svg.push_str("<g class=\"nodes\">");
    for (idx, node) in layout.nodes.iter().enumerate() {
        let node_id = idx + 1;
        svg.push_str(&format!(
            "<g class=\"node\" id=\"node-{node_id}\" transform=\"translate({:.3},{:.3})\" x=\"{:.3}\" y=\"{:.3}\">",
            node.x, node.y, node.x, node.y
        ));
        svg.push_str(&format!(
            "<rect height=\"{}\" width=\"{}\" fill=\"{}\"/>",
            node.height,
            layout.node_width,
            escape_xml(&node.color)
        ));
        svg.push_str("</g>");
    }
    svg.push_str("</g>");

    svg.push_str("<g class=\"node-labels\" font-size=\"14\">");
    for node in &layout.nodes {
        let label_on_right = node.x < layout.width / 2.0;
        let text_anchor = if label_on_right { "start" } else { "end" };
        let x = if label_on_right {
            node.x + node.width + 6.0
        } else {
            node.x - 6.0
        };
        let y = node.y + node.height / 2.0;
        let dy = if layout.show_values { "0em" } else { "0.35em" };
        let text = if layout.show_values {
            format!(
                "{}\n{}{}{}",
                node.label,
                layout.prefix,
                format_sankey_value(node.total),
                layout.suffix
            )
        } else {
            node.label.clone()
        };
        svg.push_str(&format!(
            "<text x=\"{x}\" y=\"{y}\" dy=\"{dy}\" text-anchor=\"{text_anchor}\">{}</text>",
            escape_xml_text_node(&text)
        ));
    }
    svg.push_str("</g>");

    svg.push_str("<g class=\"links\" fill=\"none\" stroke-opacity=\"0.5\">");
    for link in &layout.links {
        let mid_x = (link.start.0 + link.end.0) / 2.0;
        let gradient_id = escape_xml(&link.gradient_id);
        let stroke = match layout.link_color.as_str() {
            "gradient" => format!("url(#{gradient_id})"),
            "source" => escape_xml(&link.color_start),
            "target" => escape_xml(&link.color_end),
            other => escape_xml(other),
        };
        svg.push_str("<g class=\"link\" style=\"mix-blend-mode: multiply;\">");
        if layout.link_color == "gradient" {
            svg.push_str(&format!(
                "<linearGradient id=\"{}\" gradientUnits=\"userSpaceOnUse\" x1=\"{}\" x2=\"{}\">",
                gradient_id, link.start.0, link.end.0
            ));
            svg.push_str(&format!(
                "<stop offset=\"0%\" stop-color=\"{}\"/>",
                escape_xml(&link.color_start)
            ));
            svg.push_str(&format!(
                "<stop offset=\"100%\" stop-color=\"{}\"/>",
                escape_xml(&link.color_end)
            ));
            svg.push_str("</linearGradient>");
        }
        svg.push_str(&format!(
            "<path d=\"M{},{}C{},{},{},{},{},{}\" stroke=\"{}\" stroke-width=\"{}\"/>",
            link.start.0,
            link.start.1,
            mid_x,
            link.start.1,
            mid_x,
            link.end.1,
            link.end.0,
            link.end.1,
            stroke,
            link.thickness.max(1.0)
        ));
        svg.push_str("</g>");
    }
    svg.push_str("</g>");

    svg
}

fn render_error(layout: &ErrorLayout, _theme: &Theme, _config: &LayoutConfig) -> String {
    // Mermaid CLI renders a dedicated error diagram for unsupported syntax.
    // We mirror that here so treemap diagrams can match CLI output closely.
    const ERROR_ICON_PATHS: [&str; 6] = [
        "m411.313,123.313c6.25-6.25 6.25-16.375 0-22.625s-16.375-6.25-22.625,0l-32,32-9.375,9.375-20.688-20.688c-12.484-12.5-32.766-12.5-45.25,0l-16,16c-1.261,1.261-2.304,2.648-3.31,4.051-21.739-8.561-45.324-13.426-70.065-13.426-105.867,0-192,86.133-192,192s86.133,192 192,192 192-86.133 192-192c0-24.741-4.864-48.327-13.426-70.065 1.402-1.007 2.79-2.049 4.051-3.31l16-16c12.5-12.492 12.5-32.758 0-45.25l-20.688-20.688 9.375-9.375 32.001-31.999zm-219.313,100.687c-52.938,0-96,43.063-96,96 0,8.836-7.164,16-16,16s-16-7.164-16-16c0-70.578 57.422-128 128-128 8.836,0 16,7.164 16,16s-7.164,16-16,16z",
        "m459.02,148.98c-6.25-6.25-16.375-6.25-22.625,0s-6.25,16.375 0,22.625l16,16c3.125,3.125 7.219,4.688 11.313,4.688 4.094,0 8.188-1.563 11.313-4.688 6.25-6.25 6.25-16.375 0-22.625l-16.001-16z",
        "m340.395,75.605c3.125,3.125 7.219,4.688 11.313,4.688 4.094,0 8.188-1.563 11.313-4.688 6.25-6.25 6.25-16.375 0-22.625l-16-16c-6.25-6.25-16.375-6.25-22.625,0s-6.25,16.375 0,22.625l15.999,16z",
        "m400,64c8.844,0 16-7.164 16-16v-32c0-8.836-7.156-16-16-16-8.844,0-16,7.164-16,16v32c0,8.836 7.156,16 16,16z",
        "m496,96.586h-32c-8.844,0-16,7.164-16,16 0,8.836 7.156,16 16,16h32c8.844,0 16-7.164 16-16 0-8.836-7.156-16-16-16z",
        "m436.98,75.605c3.125,3.125 7.219,4.688 11.313,4.688 4.094,0 8.188-1.563 11.313-4.688l32-32c6.25-6.25 6.25-16.375 0-22.625s-16.375-6.25-22.625,0l-32,32c-6.251,6.25-6.251,16.375-0.001,22.625z",
    ];

    let mut svg = String::new();
    let needs_transform =
        layout.icon_scale != 1.0 || layout.icon_tx != 0.0 || layout.icon_ty != 0.0;

    let fmt = |value: f32| -> String {
        if (value - value.round()).abs() < 0.001 {
            format!("{:.0}", value)
        } else {
            format!("{:.2}", value)
        }
    };

    svg.push_str("<g>");
    if needs_transform {
        let transform = format!(
            "translate({},{}) scale({})",
            fmt(layout.icon_tx),
            fmt(layout.icon_ty),
            fmt(layout.icon_scale)
        );
        svg.push_str(&format!("<g transform=\"{transform}\">"));
    }
    for path in ERROR_ICON_PATHS {
        svg.push_str(&format!("<path class=\"error-icon\" d=\"{path}\"/>"));
    }
    if needs_transform {
        svg.push_str("</g>");
    }

    let message = escape_xml(&layout.message);
    let version = escape_xml(&format!("mermaid version {}", layout.version));
    svg.push_str(&format!(
        "<text class=\"error-text\" x=\"{}\" y=\"{}\" font-size=\"{}px\" style=\"text-anchor: middle;\">{}</text>",
        fmt(layout.text_x),
        fmt(layout.text_y),
        fmt(layout.text_size),
        message
    ));
    svg.push_str(&format!(
        "<text class=\"error-text\" x=\"{}\" y=\"{}\" font-size=\"{}px\" style=\"text-anchor: middle;\">{}</text>",
        fmt(layout.version_x),
        fmt(layout.version_y),
        fmt(layout.version_size),
        version
    ));
    svg.push_str("</g>");

    svg
}

fn normalize_font_family(font_family: &str) -> String {
    font_family
        .split(',')
        .map(|part| part.trim().trim_matches('\'').trim_matches('"'))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}

fn svg_font_style_block(layout: &Layout, theme: &Theme, config: &LayoutConfig) -> String {
    let css_font_family = css_font_family_list(&theme.font_family);
    let embedded_font = embedded_font_faces_css(&font_faces_to_embed(layout, theme, config));
    format!(
        "<style>{embedded_font}svg{{font-family:{font_family};font-size:{font_size}px;fill:{fill};}}</style>",
        embedded_font = embedded_font,
        font_family = css_font_family,
        font_size = theme.font_size,
        fill = theme.text_color
    )
}

fn error_style_block(_theme: &Theme) -> String {
    "<style>.error-icon{fill:#552222;}.error-text{fill:#552222;stroke:#552222;}</style>".to_string()
}

fn embedded_font_faces_css(font_faces: &[(&str, u16)]) -> String {
    let mut seen = Vec::new();
    let mut css = String::new();
    for (font_family, font_weight) in font_faces {
        let Some(family) = primary_named_font_family(font_family) else {
            continue;
        };
        if seen
            .iter()
            .any(|(seen_family, seen_weight): &(String, u16)| {
                seen_family.eq_ignore_ascii_case(&family) && seen_weight == font_weight
            })
        {
            continue;
        }
        let font = if *font_weight == 400 {
            text_metrics::embedded_font_data(font_family)
        } else {
            text_metrics::embedded_font_data_with_weight(font_family, *font_weight)
        };
        let Some(font) = font else {
            continue;
        };
        seen.push((family.clone(), *font_weight));
        css.push_str(&format!(
            "@font-face{{font-family:{};src:url(data:{};base64,{}) format(\"{}\");font-weight:{};font-style:normal;}}",
            css_string(&family),
            font.mime_type,
            base64_encode(&font.bytes),
            font.format_hint,
            font_weight
        ));
    }
    css
}

fn font_faces_to_embed<'a>(
    layout: &Layout,
    theme: &'a Theme,
    config: &'a LayoutConfig,
) -> Vec<(&'a str, u16)> {
    let mut families = vec![(theme.font_family.as_str(), 400)];
    if matches!(layout.diagram, DiagramData::Cynefin(_)) {
        families.push((theme.font_family.as_str(), 700));
    }
    if !matches!(layout.diagram, DiagramData::C4(_)) {
        return families;
    }
    let c4 = &config.c4;
    families.extend([
        (c4.person_font_family.as_str(), 400),
        (c4.external_person_font_family.as_str(), 400),
        (c4.system_font_family.as_str(), 400),
        (c4.external_system_font_family.as_str(), 400),
        (c4.system_db_font_family.as_str(), 400),
        (c4.external_system_db_font_family.as_str(), 400),
        (c4.system_queue_font_family.as_str(), 400),
        (c4.external_system_queue_font_family.as_str(), 400),
        (c4.boundary_font_family.as_str(), 400),
        (c4.message_font_family.as_str(), 400),
        (c4.container_font_family.as_str(), 400),
        (c4.external_container_font_family.as_str(), 400),
        (c4.container_db_font_family.as_str(), 400),
        (c4.external_container_db_font_family.as_str(), 400),
        (c4.container_queue_font_family.as_str(), 400),
        (c4.external_container_queue_font_family.as_str(), 400),
        (c4.component_font_family.as_str(), 400),
        (c4.external_component_font_family.as_str(), 400),
        (c4.component_db_font_family.as_str(), 400),
        (c4.external_component_db_font_family.as_str(), 400),
        (c4.component_queue_font_family.as_str(), 400),
        (c4.external_component_queue_font_family.as_str(), 400),
    ]);
    families
}

fn primary_named_font_family(font_family: &str) -> Option<String> {
    font_family
        .split(',')
        .map(clean_font_family_token)
        .find(|part| !part.is_empty() && !is_generic_font_family(part))
}

fn css_font_family_list(font_family: &str) -> String {
    let tokens: Vec<String> = font_family
        .split(',')
        .map(clean_font_family_token)
        .filter(|part| !part.is_empty())
        .map(|part| {
            if is_generic_font_family(&part) || is_css_identifier(&part) {
                part
            } else {
                css_string(&part)
            }
        })
        .collect();

    if tokens.is_empty() {
        "sans-serif".to_string()
    } else {
        tokens.join(",")
    }
}

fn clean_font_family_token(token: &str) -> String {
    token
        .trim()
        .trim_matches('\'')
        .trim_matches('"')
        .trim()
        .to_string()
}

fn is_generic_font_family(family: &str) -> bool {
    matches!(
        family.to_ascii_lowercase().as_str(),
        "serif"
            | "sans-serif"
            | "monospace"
            | "cursive"
            | "fantasy"
            | "system-ui"
            | "ui-sans-serif"
            | "ui-monospace"
            | "-apple-system"
    )
}

fn is_css_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_' || first == '-') {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn css_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\a "),
            '\r' => {}
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | b2 as u32;
        out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn render_requirement(layout: &Layout, theme: &Theme, config: &LayoutConfig) -> String {
    let mut svg = String::new();
    let req = &config.requirement;
    let font_family = normalize_font_family(&theme.font_family);
    let measure_font_size = theme.font_size.max(16.0);
    let line_height = measure_font_size * config.label_line_height;

    let render_line = |x: f32,
                       y: f32,
                       line: &TextLine,
                       color: &str,
                       bold: bool,
                       anchor: &str,
                       inherited_weight: Option<&str>,
                       inherited_style: Option<&str>|
     -> String {
        let effective_weight = if bold {
            Some("bold")
        } else {
            inherited_weight.filter(|value| !value.trim().is_empty())
        };
        let weight = effective_weight
            .map(|value| format!(" font-weight=\"{}\"", escape_xml(value.trim())))
            .unwrap_or_default();
        let font_style = inherited_style
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!(" font-style=\"{}\"", escape_xml(value.trim())))
            .unwrap_or_default();
        let mut text = format!(
            "<text x=\"{x:.2}\" y=\"{y:.2}\" text-anchor=\"{anchor}\" font-family=\"{font_family}\" font-size=\"{size}\" fill=\"{color}\"{weight}{font_style}>",
            x = x,
            y = y,
            anchor = anchor,
            font_family = font_family,
            size = theme.font_size,
            color = color,
            weight = weight,
            font_style = font_style
        );
        if line.has_formatting() {
            render_formatted_tspans(&mut text, x, 0.0, line, false);
        } else {
            text.push_str(&escape_xml(&line.text()));
        }
        text.push_str("</text>");
        text
    };

    // Requirement-specific markers.
    let edge_stroke = escape_xml(&req.edge_stroke);
    svg.push_str("<defs>");
    svg.push_str(&format!(
        "<marker id=\"req-contains-start\" refX=\"0\" refY=\"10\" markerWidth=\"20\" markerHeight=\"20\" orient=\"auto\"><g><circle cx=\"10\" cy=\"10\" r=\"9\" fill=\"none\" stroke=\"{edge_stroke}\" stroke-width=\"1\"/><line x1=\"1\" x2=\"19\" y1=\"10\" y2=\"10\" stroke=\"{edge_stroke}\"/><line y1=\"1\" y2=\"19\" x1=\"10\" x2=\"10\" stroke=\"{edge_stroke}\"/></g></marker>"
    ));
    svg.push_str(&format!(
        "<marker id=\"req-arrow-end\" refX=\"20\" refY=\"10\" markerWidth=\"20\" markerHeight=\"20\" orient=\"auto\"><path d=\"M0,0 L20,10 M20,10 L0,20\" fill=\"none\" stroke=\"{edge_stroke}\" stroke-width=\"1\"/></marker>"
    ));
    svg.push_str("</defs>");

    let pad_x = req.render_padding_x;
    let pad_y = req.render_padding_y;
    let has_padding = pad_x.abs() > f32::EPSILON || pad_y.abs() > f32::EPSILON;
    if has_padding {
        svg.push_str(&format!(
            "<g transform=\"translate({:.2},{:.2})\">",
            pad_x, pad_y
        ));
    }

    for (edge_idx, edge) in layout.edges.iter().enumerate() {
        let edge_id = edge_dom_id(edge_idx);
        let stroke = edge
            .override_style
            .stroke
            .as_deref()
            .unwrap_or(req.edge_stroke.as_str());
        let stroke_width = edge
            .override_style
            .stroke_width
            .unwrap_or(req.edge_stroke_width);
        let dash = edge
            .override_style
            .dasharray
            .as_deref()
            .map(|value| format!(" stroke-dasharray=\"{}\"", value))
            .unwrap_or_default();
        let marker_start = if edge.arrow_start {
            " marker-start=\"url(#req-contains-start)\""
        } else {
            ""
        };
        let marker_end = if edge.arrow_end {
            " marker-end=\"url(#req-arrow-end)\""
        } else {
            ""
        };
        let d = if edge.points.len() >= 3 {
            points_to_d3_basis_path(&edge.points)
        } else {
            points_to_path(&edge.points)
        };
        svg.push_str(&format!(
            "<path id=\"{edge_id}\" data-edge-id=\"{edge_id}\" d=\"{d}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{dash}{marker_start}{marker_end} stroke-linecap=\"round\" stroke-linejoin=\"round\"/>"
        ));

        if let Some(label) = edge.label.as_ref()
            && let Some((x, y)) = edge.label_anchor
        {
            let (pad_x, pad_y) = edge_label_padding(layout.kind, config);
            let rect = LabelRect::from_center(x, y, label.width, label.height, pad_x, pad_y);
            if req.edge_label_background != "none" {
                let visible = edge_label_background_visible(
                    layout.kind,
                    EdgeLabelKind::Center,
                    &edge.points,
                    rect,
                );
                let fill_opacity = if visible { 0.5 } else { 0.0 };
                svg.push_str(&format!(
                    "<rect data-edge-id=\"{edge_id}\" data-label-kind=\"center\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" fill-opacity=\"{:.2}\" stroke=\"none\"/>",
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    req.edge_label_background,
                    fill_opacity
                ));
            }
            let label_color = edge
                .override_style
                .label_color
                .as_deref()
                .unwrap_or(req.edge_label_color.as_str());
            svg.push_str(&format!(
                "<g class=\"edgeLabel\" data-edge-id=\"{edge_id}\" data-label-kind=\"center\">"
            ));
            svg.push_str(&text_block_svg(
                x,
                y,
                label,
                theme,
                config,
                true,
                Some(label_color),
            ));
            svg.push_str("</g>");
        }
    }

    for node in layout.nodes.values() {
        if node.hidden {
            continue;
        }
        if node.anchor_subgraph.is_some() {
            continue;
        }
        let fill = node.style.fill.as_deref().unwrap_or(req.fill.as_str());
        let base_stroke = node
            .style
            .stroke
            .as_deref()
            .unwrap_or(req.box_stroke.as_str());
        let base_stroke_width = node.style.stroke_width.unwrap_or(req.box_stroke_width);
        let label_color = node
            .style
            .text_color
            .as_deref()
            .unwrap_or(req.label_color.as_str());

        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>",
            node.x, node.y, node.width, node.height, fill, base_stroke, base_stroke_width
        ));
        if req.stroke != "none" && req.stroke_width > 0.0 {
            svg.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/>",
                node.x,
                node.y,
                node.width,
                node.height,
                req.stroke,
                req.stroke_width
            ));
        }

        let lines = &node.label.lines;
        let header_count = lines.len().min(2);
        let body_lines = if lines.len() > 2 { &lines[2..] } else { &[] };
        let header_x = node.x + node.width / 2.0;
        let body_x = node.x + req.label_padding_x;
        let first_baseline = node.y + req.label_padding_y + theme.font_size;
        let inherited_weight = node.style.font_weight.as_deref();
        let inherited_style = node.style.font_style.as_deref();
        if header_count >= 1 {
            svg.push_str(&render_line(
                header_x,
                first_baseline,
                &lines[0],
                label_color,
                false,
                "middle",
                inherited_weight,
                inherited_style,
            ));
        }
        if header_count >= 2 {
            let id_y = first_baseline + req.header_line_gap.max(line_height);
            svg.push_str(&render_line(
                header_x,
                id_y,
                &lines[1],
                label_color,
                true,
                "middle",
                inherited_weight,
                inherited_style,
            ));
        }

        if !body_lines.is_empty() {
            let divider_y = node.y + req.header_band_height;
            svg.push_str(&format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{}\"/>",
                node.x,
                divider_y,
                node.x + node.width,
                divider_y,
                req.divider_color,
                req.divider_width
            ));
            let mut body_y = divider_y + req.label_padding_y + theme.font_size;
            for line in body_lines {
                svg.push_str(&render_line(
                    body_x,
                    body_y,
                    line,
                    label_color,
                    false,
                    "start",
                    inherited_weight,
                    inherited_style,
                ));
                body_y += line_height;
            }
        }
    }

    if has_padding {
        svg.push_str("</g>");
    }

    svg
}

fn render_radar(layout: &Layout, theme: &Theme, _config: &LayoutConfig) -> String {
    use std::f32::consts::PI;

    const WIDTH: f32 = 700.0;
    const HEIGHT: f32 = 700.0;
    const CENTER_X: f32 = WIDTH / 2.0;
    const CENTER_Y: f32 = HEIGHT / 2.0;
    const MAX_RADIUS: f32 = 300.0;
    const AXIS_LABEL_FACTOR: f32 = 1.05;
    const LEGEND_BOX_SIZE: f32 = 12.0;
    const LEGEND_GAP: f32 = 4.0;
    const LEGEND_LINE_HEIGHT: f32 = 20.0;
    const CURVE_TENSION: f32 = 0.17;

    fn radar_index(id: &str) -> usize {
        id.rsplit('_')
            .next()
            .and_then(|part| part.parse::<usize>().ok())
            .unwrap_or(usize::MAX)
    }

    fn parse_series(node: &crate::layout::NodeLayout) -> Option<(String, Vec<(String, f32)>)> {
        let text_lines: Vec<String> = node
            .label
            .lines
            .iter()
            .map(|l| l.text().into_owned())
            .collect();
        let mut lines = text_lines
            .iter()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty());
        let name = lines.next()?.to_string();
        let mut pairs = Vec::new();
        for line in lines {
            let Some((axis_raw, value_raw)) = line.split_once(':') else {
                continue;
            };
            let axis = axis_raw.trim();
            let value_str = value_raw.trim();
            if axis.is_empty() || value_str.is_empty() {
                continue;
            }
            let Ok(value) = value_str.parse::<f32>() else {
                continue;
            };
            pairs.push((axis.to_string(), value));
        }
        if pairs.is_empty() {
            None
        } else {
            Some((name, pairs))
        }
    }

    fn closed_round_curve(points: &[(f32, f32)], tension: f32) -> String {
        let num_points = points.len();
        let mut d = format!("M{:.3},{:.3}", points[0].0, points[0].1);
        for idx in 0..num_points {
            let p0 = points[(idx + num_points - 1) % num_points];
            let p1 = points[idx];
            let p2 = points[(idx + 1) % num_points];
            let p3 = points[(idx + 2) % num_points];
            let cp1 = (
                p1.0 + (p2.0 - p0.0) * tension,
                p1.1 + (p2.1 - p0.1) * tension,
            );
            let cp2 = (
                p2.0 - (p3.0 - p1.0) * tension,
                p2.1 - (p3.1 - p1.1) * tension,
            );
            d.push_str(&format!(
                " C{:.3},{:.3} {:.3},{:.3} {:.3},{:.3}",
                cp1.0, cp1.1, cp2.0, cp2.1, p2.0, p2.1
            ));
        }
        d.push_str(" Z");
        d
    }

    fn radar_polygon_points(
        radius: f32,
        axis_count: usize,
        start_angle: f32,
        angle_step: f32,
    ) -> String {
        (0..axis_count)
            .map(|idx| {
                let angle = start_angle + angle_step * idx as f32;
                format!("{:.3},{:.3}", radius * angle.cos(), radius * angle.sin())
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    let (title, show_legend, ticks, configured_max, min_value, graticule) = match &layout.diagram {
        DiagramData::Radar(radar) => (
            radar.title.as_deref().unwrap_or(""),
            radar.show_legend,
            radar.ticks.max(1),
            radar.max,
            radar.min,
            radar.graticule,
        ),
        DiagramData::Graph { title, .. } => (
            title.as_deref().unwrap_or(""),
            true,
            5,
            None,
            0.0,
            crate::ir::RadarGraticule::Circle,
        ),
        _ => ("", true, 5, None, 0.0, crate::ir::RadarGraticule::Circle),
    };

    let mut nodes: Vec<&crate::layout::NodeLayout> =
        layout.nodes.values().filter(|node| !node.hidden).collect();
    nodes.sort_by_key(|node| radar_index(&node.id));

    let mut raw_series = Vec::new();
    for node in nodes {
        if let Some(series) = parse_series(node) {
            raw_series.push(series);
        }
    }
    let Some((_, first_pairs)) = raw_series.first() else {
        return String::new();
    };

    let axes: Vec<String> = first_pairs.iter().map(|(axis, _)| axis.clone()).collect();
    let axis_count = axes.len();
    if axis_count == 0 {
        return String::new();
    }

    let mut series_values: Vec<(String, Vec<f32>)> = Vec::new();
    let mut max_value = configured_max.unwrap_or(0.0);
    for (name, pairs) in &raw_series {
        let mut values = Vec::with_capacity(axis_count);
        for axis in &axes {
            let value = pairs
                .iter()
                .find_map(|(a, v)| (a == axis).then_some(*v))
                .unwrap_or(0.0);
            if configured_max.is_none() {
                max_value = max_value.max(value);
            }
            values.push(value);
        }
        series_values.push((name.clone(), values));
    }

    if max_value <= min_value {
        max_value = min_value + 1.0;
    }
    let value_span = (max_value - min_value).max(1.0);
    let angle_step = 2.0 * PI / axis_count as f32;
    let start_angle = -PI / 2.0;

    let mut svg = String::new();
    svg.push_str(&format!(
        "<g transform=\"translate({:.3}, {:.3})\">",
        CENTER_X, CENTER_Y
    ));

    for step in 1..=ticks {
        let r = MAX_RADIUS * step as f32 / ticks as f32;
        match graticule {
            crate::ir::RadarGraticule::Circle => {
                svg.push_str(&format!(
                    "<circle r=\"{:.3}\" fill=\"{}\" fill-opacity=\"{}\" stroke=\"{}\" stroke-width=\"{}\" />",
                    r,
                    theme.radar.graticule_color,
                    theme.radar.graticule_opacity,
                    theme.radar.graticule_color,
                    theme.radar.graticule_stroke_width
                ));
            }
            crate::ir::RadarGraticule::Polygon => {
                let points = radar_polygon_points(r, axis_count, start_angle, angle_step);
                svg.push_str(&format!(
                    "<polygon points=\"{}\" fill=\"{}\" fill-opacity=\"{}\" stroke=\"{}\" stroke-width=\"{}\" />",
                    points,
                    theme.radar.graticule_color,
                    theme.radar.graticule_opacity,
                    theme.radar.graticule_color,
                    theme.radar.graticule_stroke_width
                ));
            }
        }
    }

    for (idx, axis) in axes.iter().enumerate() {
        let angle = start_angle + angle_step * idx as f32;
        let x = MAX_RADIUS * angle.cos();
        let y = MAX_RADIUS * angle.sin();
        svg.push_str(&format!(
            "<line x1=\"0\" y1=\"0\" x2=\"{:.3}\" y2=\"{:.3}\" stroke=\"{}\" stroke-width=\"{}\" />",
            x, y, theme.radar.axis_color, theme.radar.axis_stroke_width
        ));
        let label_r = MAX_RADIUS * AXIS_LABEL_FACTOR;
        let lx = label_r * angle.cos();
        let ly = label_r * angle.sin();
        svg.push_str(&format!(
            "<text x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
            lx,
            ly,
            normalize_font_family(&theme.font_family),
            theme.radar.axis_label_font_size,
            theme.radar.axis_color,
            escape_xml(axis)
        ));
    }

    for (series_idx, (name, values)) in series_values.iter().enumerate() {
        let color = theme
            .cscale_colors
            .get(series_idx)
            .map(String::as_str)
            .unwrap_or(
                crate::theme::MERMAID_RADAR_COLORS
                    [series_idx % crate::theme::MERMAID_RADAR_COLORS.len()],
            );
        let mut points = Vec::with_capacity(axis_count);
        for (idx, value) in values.iter().enumerate() {
            let angle = start_angle + angle_step * idx as f32;
            let clipped = value.clamp(min_value, max_value);
            let r = MAX_RADIUS * (clipped - min_value) / value_span;
            points.push((r * angle.cos(), r * angle.sin()));
        }
        if points.is_empty() {
            continue;
        }
        match graticule {
            crate::ir::RadarGraticule::Circle => {
                let d = closed_round_curve(&points, CURVE_TENSION);
                svg.push_str(&format!(
                    "<path d=\"{}\" fill=\"{}\" fill-opacity=\"{}\" stroke=\"{}\" stroke-width=\"{}\" />",
                    d,
                    escape_xml(&color),
                    theme.radar.curve_opacity,
                    escape_xml(&color),
                    theme.radar.curve_stroke_width
                ));
            }
            crate::ir::RadarGraticule::Polygon => {
                let points_attr = points
                    .iter()
                    .map(|(x, y)| format!("{:.3},{:.3}", x, y))
                    .collect::<Vec<_>>()
                    .join(" ");
                svg.push_str(&format!(
                    "<polygon points=\"{}\" fill=\"{}\" fill-opacity=\"{}\" stroke=\"{}\" stroke-width=\"{}\" />",
                    points_attr,
                    escape_xml(&color),
                    theme.radar.curve_opacity,
                    escape_xml(&color),
                    theme.radar.curve_stroke_width
                ));
            }
        }

        if show_legend {
            let legend_x = ((MAX_RADIUS + 50.0) * 3.0) / 4.0;
            let legend_y =
                (-(MAX_RADIUS + 50.0) * 3.0) / 4.0 + series_idx as f32 * LEGEND_LINE_HEIGHT;
            svg.push_str(&format!(
                "<g transform=\"translate({:.3}, {:.3})\">",
                legend_x, legend_y
            ));
            svg.push_str(&format!(
                "<rect width=\"{}\" height=\"{}\" fill=\"{}\" fill-opacity=\"{}\" stroke=\"{}\" />",
                LEGEND_BOX_SIZE,
                LEGEND_BOX_SIZE,
                escape_xml(&color),
                theme.radar.curve_opacity,
                escape_xml(&color)
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"0\" text-anchor=\"start\" dominant-baseline=\"hanging\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                LEGEND_BOX_SIZE + LEGEND_GAP,
                normalize_font_family(&theme.font_family),
                theme.radar.legend_font_size,
                theme.text_color,
                escape_xml(name)
            ));
            svg.push_str("</g>");
        }
    }

    svg.push_str(&format!(
        "<text x=\"0\" y=\"{:.3}\" text-anchor=\"middle\" dominant-baseline=\"hanging\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
        -(MAX_RADIUS + 50.0),
        normalize_font_family(&theme.font_family),
        theme.font_size,
        theme.radar.title_color,
        escape_xml(title)
    ));

    svg.push_str("</g>");
    svg
}

/// Render an architecture diagram icon as SVG.
/// Returns SVG elements (paths/circles) drawn within the given width/height box.
fn architecture_icon_svg(icon_type: Option<&str>, w: f32, h: f32, fill: &str) -> String {
    let cx = w / 2.0;
    let cy = h / 2.0;
    let r = w.min(h) * 0.35;
    let sw = (w * 0.02).max(1.5);
    let style = format!(
        "fill=\"none\" stroke=\"{}\" stroke-width=\"{:.1}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"",
        fill, sw
    );
    match icon_type {
        Some("internet") | Some("globe") => {
            // Globe: circle + vertical ellipse + horizontal line + vertical line
            format!(
                "<circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"{r:.1}\" {style}/>\
                 <ellipse cx=\"{cx:.1}\" cy=\"{cy:.1}\" rx=\"{rx:.1}\" ry=\"{r:.1}\" {style}/>\
                 <line x1=\"{x1:.1}\" y1=\"{cy:.1}\" x2=\"{x2:.1}\" y2=\"{cy:.1}\" {style}/>\
                 <line x1=\"{cx:.1}\" y1=\"{y1:.1}\" x2=\"{cx:.1}\" y2=\"{y2:.1}\" {style}/>",
                rx = r * 0.5,
                x1 = cx - r,
                x2 = cx + r,
                y1 = cy - r,
                y2 = cy + r,
            )
        }
        Some("server") => {
            // Server rack — matching Iconify icon from mermaid-js.
            // Source icon is 80×80, scale to fit w×h.
            let s = w.min(h) / 80.0;
            let t = format!("transform=\"scale({s:.3})\"");
            let lst = format!(
                "fill=\"none\" stroke=\"{fill}\" stroke-miterlimit=\"10\" stroke-width=\"2\""
            );
            let bar_fill = format!("fill=\"{fill}\" stroke-width=\"0\"");
            let bar_stroke = format!("fill=\"none\" stroke=\"{fill}\" stroke-miterlimit=\"10\"");
            let dot = format!("fill=\"{fill}\" stroke=\"{fill}\" stroke-miterlimit=\"10\"");
            format!(
                "<g {t}>\
                 <rect x=\"17.5\" y=\"17.5\" width=\"45\" height=\"45\" rx=\"2\" ry=\"2\" {lst}/>\
                 <line x1=\"17.5\" y1=\"32.5\" x2=\"62.5\" y2=\"32.5\" {lst}/>\
                 <line x1=\"17.5\" y1=\"47.5\" x2=\"62.5\" y2=\"47.5\" {lst}/>\
                 <g><path d=\"m56.25,25c0,.27-.45.5-1,.5h-10.5c-.55,0-1-.23-1-.5s.45-.5,1-.5h10.5c.55,0,1,.23,1,.5Z\" {bar_fill}/><path d=\"m56.25,25c0,.27-.45.5-1,.5h-10.5c-.55,0-1-.23-1-.5s.45-.5,1-.5h10.5c.55,0,1,.23,1,.5Z\" {bar_stroke}/></g>\
                 <g><path d=\"m56.25,40c0,.27-.45.5-1,.5h-10.5c-.55,0-1-.23-1-.5s.45-.5,1-.5h10.5c.55,0,1,.23,1,.5Z\" {bar_fill}/><path d=\"m56.25,40c0,.27-.45.5-1,.5h-10.5c-.55,0-1-.23-1-.5s.45-.5,1-.5h10.5c.55,0,1,.23,1,.5Z\" {bar_stroke}/></g>\
                 <g><path d=\"m56.25,55c0,.27-.45.5-1,.5h-10.5c-.55,0-1-.23-1-.5s.45-.5,1-.5h10.5c.55,0,1,.23,1,.5Z\" {bar_fill}/><path d=\"m56.25,55c0,.27-.45.5-1,.5h-10.5c-.55,0-1-.23-1-.5s.45-.5,1-.5h10.5c.55,0,1,.23,1,.5Z\" {bar_stroke}/></g>\
                 <g><circle cx=\"32.5\" cy=\"25\" r=\".75\" {dot}/><circle cx=\"27.5\" cy=\"25\" r=\".75\" {dot}/><circle cx=\"22.5\" cy=\"25\" r=\".75\" {dot}/></g>\
                 <g><circle cx=\"32.5\" cy=\"40\" r=\".75\" {dot}/><circle cx=\"27.5\" cy=\"40\" r=\".75\" {dot}/><circle cx=\"22.5\" cy=\"40\" r=\".75\" {dot}/></g>\
                 <g><circle cx=\"32.5\" cy=\"55\" r=\".75\" {dot}/><circle cx=\"27.5\" cy=\"55\" r=\".75\" {dot}/><circle cx=\"22.5\" cy=\"55\" r=\".75\" {dot}/></g>\
                 </g>"
            )
        }
        Some("database") => {
            // Database cylinder — matching Iconify icon from mermaid-js.
            let s = w.min(h) / 80.0;
            let t = format!("transform=\"scale({s:.3})\"");
            let lst = format!(
                "fill=\"none\" stroke=\"{fill}\" stroke-miterlimit=\"10\" stroke-width=\"2\""
            );
            format!(
                "<g {t}>\
                 <path d=\"m20,57.86c0,3.94,8.95,7.14,20,7.14s20-3.2,20-7.14\" {lst}/>\
                 <path d=\"m20,45.95c0,3.94,8.95,7.14,20,7.14s20-3.2,20-7.14\" {lst}/>\
                 <path d=\"m20,34.05c0,3.94,8.95,7.14,20,7.14s20-3.2,20-7.14\" {lst}/>\
                 <ellipse cx=\"40\" cy=\"22.14\" rx=\"20\" ry=\"7.14\" {lst}/>\
                 <line x1=\"20\" y1=\"22.14\" x2=\"20\" y2=\"57.86\" {lst}/>\
                 <line x1=\"60\" y1=\"22.14\" x2=\"60\" y2=\"57.86\" {lst}/>\
                 </g>"
            )
        }
        Some("disk") => {
            // Hard drive — matching Iconify icon from mermaid-js.
            // Casing + corner screws + platter + spindle + actuator arm.
            let s = w.min(h) / 80.0;
            let t = format!("transform=\"scale({s:.3})\"");
            let lst = format!(
                "fill=\"none\" stroke=\"{fill}\" stroke-miterlimit=\"10\" stroke-width=\"2\""
            );
            let fst = format!("fill=\"{fill}\"");
            format!(
                "<g {t}>\
                 <rect x=\"20\" y=\"15\" width=\"40\" height=\"50\" rx=\"1\" ry=\"1\" {lst}/>\
                 <ellipse cx=\"24\" cy=\"19.17\" rx=\".8\" ry=\".83\" {lst}/>\
                 <ellipse cx=\"56\" cy=\"19.17\" rx=\".8\" ry=\".83\" {lst}/>\
                 <ellipse cx=\"24\" cy=\"60.83\" rx=\".8\" ry=\".83\" {lst}/>\
                 <ellipse cx=\"56\" cy=\"60.83\" rx=\".8\" ry=\".83\" {lst}/>\
                 <ellipse cx=\"40\" cy=\"33.75\" rx=\"14\" ry=\"14.58\" {lst}/>\
                 <ellipse cx=\"40\" cy=\"33.75\" rx=\"4\" ry=\"4.17\" {fst} stroke=\"{fill}\" stroke-width=\"2\"/>\
                 <path d=\"m37.51,42.52l-4.83,13.22c-.26.71-1.1,1.02-1.76.64l-4.18-2.42c-.66-.38-.81-1.26-.33-1.84l9.01-10.8c.88-1.05,2.56-.08,2.09,1.2Z\" {fst}/>\
                 </g>"
            )
        }
        Some("cloud") => {
            // Cloud — matching Iconify icon from mermaid-js.
            let s = w.min(h) / 80.0;
            let t = format!("transform=\"scale({s:.3})\"");
            let lst = format!(
                "fill=\"none\" stroke=\"{fill}\" stroke-miterlimit=\"10\" stroke-width=\"2\""
            );
            format!(
                "<g {t}>\
                 <path d=\"m65,47.5c0,2.76-2.24,5-5,5H20c-2.76,0-5-2.24-5-5,0-1.87,1.03-3.51,2.56-4.36-.04-.21-.06-.42-.06-.64,0-2.6,2.48-4.74,5.65-4.97,1.65-4.51,6.34-7.76,11.85-7.76.86,0,1.69.08,2.5.23,2.09-1.57,4.69-2.5,7.5-2.5,6.1,0,11.19,4.38,12.28,10.17,2.14.56,3.72,2.51,3.72,4.83,0,.03,0,.07-.01.1,2.29.46,4.01,2.48,4.01,4.9Z\" {lst}/>\
                 </g>"
            )
        }
        // Keep old fallback for unrecognized database/disk references
        Some(t) if t.contains("database") || t.contains("cylinder") => {
            let s = w.min(h) / 80.0;
            let tf = format!("transform=\"scale({s:.3})\"");
            let lst = format!(
                "fill=\"none\" stroke=\"{fill}\" stroke-miterlimit=\"10\" stroke-width=\"2\""
            );
            format!(
                "<g {tf}>\
                 <path d=\"m20,57.86c0,3.94,8.95,7.14,20,7.14s20-3.2,20-7.14\" {lst}/>\
                 <ellipse cx=\"40\" cy=\"22.14\" rx=\"20\" ry=\"7.14\" {lst}/>\
                 <line x1=\"20\" y1=\"22.14\" x2=\"20\" y2=\"57.86\" {lst}/>\
                 <line x1=\"60\" y1=\"22.14\" x2=\"60\" y2=\"57.86\" {lst}/>\
                 </g>"
            )
        }
        Some(t) if t.contains("cloud") => {
            let s = w.min(h) / 80.0;
            let tf = format!("transform=\"scale({s:.3})\"");
            let lst = format!(
                "fill=\"none\" stroke=\"{fill}\" stroke-miterlimit=\"10\" stroke-width=\"2\""
            );
            format!(
                "<g {tf}><path d=\"m65,47.5c0,2.76-2.24,5-5,5H20c-2.76,0-5-2.24-5-5,0-1.87,1.03-3.51,2.56-4.36-.04-.21-.06-.42-.06-.64,0-2.6,2.48-4.74,5.65-4.97,1.65-4.51,6.34-7.76,11.85-7.76.86,0,1.69.08,2.5.23,2.09-1.57,4.69-2.5,7.5-2.5,6.1,0,11.19,4.38,12.28,10.17,2.14.56,3.72,2.51,3.72,4.83,0,.03,0,.07-.01.1,2.29.46,4.01,2.48,4.01,4.9Z\" {lst}/></g>"
            )
        }
        Some(_) => {
            // Mermaid architecture only registers the mermaid-architecture pack.
            // Other prefixes, such as logos:aws-*, use Iconify's unknown icon.
            let s = w.min(h) / 80.0;
            let t = format!("transform=\"scale({s:.3})\"");
            let fill = escape_xml(fill);
            format!(
                "<g {t}><text transform=\"translate(21.16 64.67)\" style=\"fill: {fill}; font-family: ArialMT, Arial; font-size: 67.75px;\"><tspan x=\"0\" y=\"0\">?</tspan></text></g>"
            )
        }
        None => {
            // Fallback for omitted icons preserves the pre-existing blank-icon behavior.
            format!(
                "<text x=\"{cx:.1}\" y=\"{y:.1}\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{fill}\" font-size=\"{fs:.0}\">?</text>",
                y = cy + w * 0.08,
                fs = w * 0.7,
            )
        }
    }
}

fn render_architecture(
    layout: &Layout,
    theme: &Theme,
    _config: &LayoutConfig,
    color_ids: &HashMap<String, usize>,
) -> String {
    const ICON_FILL: &str = "#087ebf";
    const ICON_TEXT_FILL: &str = "#ffffff";
    const GROUP_ICON_SIZE: f32 = 30.0;
    const GROUP_ICON_OFFSET: f32 = 1.0;
    const GROUP_STROKE: &str = "hsl(240, 60%, 86.2745098039%)";

    fn sanitize_group_suffix(label: &str) -> String {
        let mut out = String::with_capacity(label.len());
        for ch in label.chars() {
            if ch.is_ascii_alphanumeric() {
                out.push(ch.to_ascii_lowercase());
            } else if ch == '_' || ch == '-' {
                out.push(ch);
            } else {
                out.push('-');
            }
        }
        let trimmed = out.trim_matches('-');
        if trimmed.is_empty() {
            "group".to_string()
        } else {
            trimmed.to_string()
        }
    }

    fn first_line(text: &str) -> &str {
        text.lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or(text)
    }

    let default_marker_idx = color_ids.get(&theme.line_color).copied().unwrap_or(0);
    let mut svg = String::new();

    svg.push_str("<g class=\"architecture-edges\">");
    for edge in &layout.edges {
        if edge.points.len() < 2 {
            continue;
        }
        let stroke = edge
            .override_style
            .stroke
            .as_ref()
            .unwrap_or(&theme.line_color);
        let stroke_width = edge.override_style.stroke_width.unwrap_or(3.0);
        let marker_idx = color_ids.get(stroke).copied().unwrap_or(default_marker_idx);
        let dash_attr = edge
            .override_style
            .dasharray
            .as_ref()
            .map(|dash| format!(" stroke-dasharray=\"{}\"", dash))
            .unwrap_or_default();
        let path_data = edge
            .points
            .iter()
            .enumerate()
            .map(|(idx, (x, y))| {
                let command = if idx == 0 { "M" } else { "L" };
                format!("{command} {x:.3} {y:.3}")
            })
            .collect::<Vec<_>>()
            .join(" ");
        let marker_start_attr = if edge.arrow_start {
            format!(" marker-start=\"url(#arrow-start-{marker_idx})\"")
        } else {
            String::new()
        };
        let marker_end_attr = if edge.arrow_end {
            format!(" marker-end=\"url(#arrow-{marker_idx})\"")
        } else {
            String::new()
        };
        svg.push_str(&format!(
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} />",
            path_data,
            escape_xml(stroke),
            stroke_width,
            marker_start_attr,
            marker_end_attr,
            dash_attr,
        ));
    }
    svg.push_str("</g>");

    svg.push_str("<g class=\"architecture-services\">");
    for node in layout.nodes.values() {
        if node.hidden {
            continue;
        }
        let icon_fill = node.style.fill.as_deref().unwrap_or(ICON_FILL);
        let label_text = node
            .label
            .lines
            .iter()
            .find(|line| !line.text().trim().is_empty())
            .map(|line| line.text().into_owned())
            .unwrap_or_else(|| node.id.clone());
        let label_y = node.height + theme.font_size * 0.5;
        svg.push_str(&format!(
            "<g id=\"service-{}\" class=\"architecture-service\" transform=\"translate({:.3},{:.3})\">",
            escape_xml(&node.id),
            node.x,
            node.y
        ));
        svg.push_str(&format!(
            "<rect width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"none\" />",
            node.width,
            node.height,
            escape_xml(icon_fill)
        ));
        svg.push_str(&architecture_icon_svg(
            node.icon.as_deref(),
            node.width,
            node.height,
            ICON_TEXT_FILL,
        ));
        svg.push_str(&format!(
            "<text x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
            node.width / 2.0,
            label_y,
            normalize_font_family(&theme.font_family),
            theme.font_size,
            escape_xml(&theme.primary_text_color),
            escape_xml(&label_text)
        ));
        svg.push_str("</g>");
    }
    for node in layout.nodes.values() {
        if !node.hidden {
            continue;
        }
        svg.push_str(&format!(
            "<g class=\"architecture-junction\" transform=\"translate({:.3},{:.3})\"><g><rect id=\"node-{}\" fill-opacity=\"0\" width=\"{}\" height=\"{}\" /></g></g>",
            node.x,
            node.y,
            escape_xml(&node.id),
            node.width,
            node.height,
        ));
    }
    svg.push_str("</g>");

    svg.push_str("<g class=\"architecture-groups\">");
    for subgraph in &layout.subgraphs {
        let stroke = subgraph.style.stroke.as_deref().unwrap_or(GROUP_STROKE);
        let stroke_width = subgraph.style.stroke_width.unwrap_or(2.0);
        let dash_attr = subgraph
            .style
            .stroke_dasharray
            .as_ref()
            .map(|dash| format!(" stroke-dasharray=\"{}\"", dash))
            .unwrap_or_default();
        let group_id = sanitize_group_suffix(&subgraph.label);
        svg.push_str(&format!(
            "<rect id=\"group-{}\" class=\"node-bkg\" x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{} />",
            escape_xml(&group_id),
            subgraph.x,
            subgraph.y,
            subgraph.width,
            subgraph.height,
            escape_xml(stroke),
            stroke_width,
            dash_attr,
        ));
        let icon_x = subgraph.x + GROUP_ICON_OFFSET;
        let icon_y = subgraph.y + GROUP_ICON_OFFSET;
        svg.push_str(&format!(
            "<g transform=\"translate({:.3},{:.3})\">",
            icon_x, icon_y
        ));
        svg.push_str(&format!(
            "<rect width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"none\" />",
            GROUP_ICON_SIZE, GROUP_ICON_SIZE, ICON_FILL
        ));
        svg.push_str(&architecture_icon_svg(
            subgraph.icon.as_deref(),
            GROUP_ICON_SIZE,
            GROUP_ICON_SIZE,
            ICON_TEXT_FILL,
        ));
        svg.push_str("</g>");
        let label_x = subgraph.x + GROUP_ICON_SIZE + 4.0;
        let label_y = subgraph.y + GROUP_ICON_SIZE * 0.7;
        svg.push_str(&format!(
            "<text x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"start\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
            label_x,
            label_y,
            normalize_font_family(&theme.font_family),
            theme.font_size,
            escape_xml(&theme.primary_text_color),
            escape_xml(first_line(&subgraph.label))
        ));
    }
    svg.push_str("</g>");

    svg
}

fn render_venn(venn: &VennLayout, theme: &Theme, _config: &LayoutConfig) -> String {
    let mut svg = String::new();

    if let Some(ref title) = venn.title {
        svg.push_str(&format!(
            "<text class=\"venn-title\" font-size=\"16px\" text-anchor=\"middle\" dominant-baseline=\"middle\" x=\"{}\" y=\"{}\" style=\"fill: {};\">{}</text>",
            venn.width / 2.0,
            32.0 * 0.5,
            escape_xml(&theme.text_color),
            escape_xml(title)
        ));
    }

    svg.push_str(&format!(
        "<g transform=\"translate(0, {})\">",
        venn.title_height
    ));

    for (idx, circle) in venn.circles.iter().enumerate() {
        let path = venn_circle_path(circle.cx, circle.cy, circle.radius);
        svg.push_str(&format!(
            "<g class=\"venn-area venn-circle venn-set-{}\" data-venn-sets=\"{}\"><path d=\"{}\" style=\"fill-opacity: {}; fill: {}; stroke: {}; stroke-width: {}; stroke-opacity: {};\"/>",
            idx % 8,
            escape_xml(&circle.id),
            escape_xml(&path),
            circle.fill_opacity,
            escape_xml(&circle.color),
            escape_xml(&circle.stroke),
            circle.stroke_width,
            circle.stroke_opacity,
        ));
        if !circle.label.is_empty() {
            svg.push_str(&format!(
                "<text class=\"label\" text-anchor=\"middle\" dy=\".35em\" x=\"{}\" y=\"{}\" style=\"fill: {}; font-size: 24px;\"><tspan x=\"{}\" y=\"{}\" dy=\"0.35em\">{}</tspan></text>",
                circle.label_x.round(),
                circle.label_y.round(),
                escape_xml(&circle.text_color),
                circle.label_x.round(),
                circle.label_y.round(),
                escape_xml(&circle.label)
            ));
        }
        svg.push_str("</g>");
    }

    for intersection in &venn.intersections {
        let data_key = venn_data_sets_key(&intersection.set_ids);
        svg.push_str(&format!(
            "<g class=\"venn-area venn-intersection\" data-venn-sets=\"{}\">",
            escape_xml(&data_key)
        ));
        if let Some(ref path) = intersection.path {
            svg.push_str(&format!(
                "<path d=\"{}\" style=\"fill-opacity: {}; fill: {};\"/>",
                escape_xml(path),
                intersection.fill_opacity,
                escape_xml(&intersection.fill)
            ));
        }
        if let Some(ref label) = intersection.label {
            svg.push_str(&format!(
                "<text class=\"label\" text-anchor=\"middle\" dy=\".35em\" x=\"{}\" y=\"{}\" style=\"fill: {}; font-size: 24px;\"><tspan x=\"{}\" y=\"{}\" dy=\"0.35em\">{}</tspan></text>",
                intersection.cx.round(),
                intersection.cy.round(),
                escape_xml(&intersection.text_color),
                intersection.cx.round(),
                intersection.cy.round(),
                escape_xml(label)
            ));
        }
        svg.push_str("</g>");
    }

    if !venn.text_nodes.is_empty() {
        svg.push_str(
            "<g class=\"venn-text-nodes\"><g class=\"venn-text-area\" font-size=\"20px\">",
        );
        for node in &venn.text_nodes {
            svg.push_str(&format!(
                "<foreignObject class=\"venn-text-node-fo\" width=\"{}\" height=\"{}\" x=\"{}\" y=\"{}\" overflow=\"visible\"><span class=\"venn-text-node\" xmlns=\"http://www.w3.org/1999/xhtml\" style=\"display: flex; width: 100%; height: 100%; white-space: normal; align-items: center; justify-content: center; text-align: center; overflow-wrap: normal; word-break: normal;{}\">{}</span></foreignObject>",
                node.width,
                node.height,
                node.x,
                node.y,
                node
                    .color
                    .as_ref()
                    .map(|color| format!(" color: {};", escape_xml(color)))
                    .unwrap_or_default(),
                escape_xml(&node.label)
            ));
        }
        svg.push_str("</g></g>");
    }

    svg.push_str("</g>");
    svg
}

fn venn_circle_path(cx: f32, cy: f32, radius: f32) -> String {
    let diameter = radius * 2.0;
    format!(
        "M {cx} {cy} m -{radius} 0 a {radius} {radius} 0 1 0 {diameter} 0 a {radius} {radius} 0 1 0 -{diameter} 0"
    )
}

fn venn_data_sets_key(set_ids: &[String]) -> String {
    let mut ids = set_ids.to_vec();
    ids.sort();
    ids.join("_")
}

fn render_packet(packet: &PacketLayout) -> String {
    let mut svg = String::new();
    svg.push_str(
        "<style>.packetByte{font-size:10px;}.packetByte.start{fill:black;}.packetByte.end{fill:black;}.packetLabel{fill:black;font-size:12px;}.packetTitle{fill:black;font-size:14px;}.packetBlock{stroke:black;stroke-width:1;fill:#efefef;}</style>",
    );

    for block in &packet.blocks {
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" class=\"packetBlock\"/>",
            block.x, block.y, block.width, block.height
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" class=\"packetLabel\" dominant-baseline=\"middle\" text-anchor=\"middle\">{}</text>",
            block.x + block.width / 2.0,
            block.y + block.height / 2.0,
            escape_xml(&block.label)
        ));

        if packet.show_bits {
            let single = block.start == block.end;
            let bit_y = block.y - 2.0;
            let start_x = if single {
                block.x + block.width / 2.0
            } else {
                block.x
            };
            let start_anchor = if single { "middle" } else { "start" };
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" class=\"packetByte start\" dominant-baseline=\"auto\" text-anchor=\"{}\">{}</text>",
                start_x, bit_y, start_anchor, block.start
            ));
            if !single {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" class=\"packetByte end\" dominant-baseline=\"auto\" text-anchor=\"end\">{}</text>",
                    block.x + block.width,
                    bit_y,
                    block.end
                ));
            }
        }
    }

    if let Some(title) = &packet.title {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" dominant-baseline=\"middle\" text-anchor=\"middle\" class=\"packetTitle\">{}</text>",
            packet.title_x,
            packet.title_y,
            escape_xml(title)
        ));
    }

    svg
}

fn render_eventmodeling(layout: &EventModelingLayout) -> String {
    let mut svg = String::new();
    let swimlane_width = layout.max_r + 15.0;
    svg.push_str("<g/>");

    for swimlane in &layout.swimlanes {
        svg.push_str(&format!(
            "<g class=\"em-swimlane\"><rect x=\"0\" y=\"{:.3}\" rx=\"3\" width=\"{:.3}\" height=\"{:.3}\" fill=\"rgb(250,250,250)\" stroke=\"rgb(240,240,240)\"/><text font-weight=\"bold\" x=\"30\" y=\"{:.3}\">{}</text></g>",
            swimlane.y,
            swimlane_width,
            swimlane.height,
            swimlane.y + 30.0,
            escape_xml(&swimlane.label)
        ));
    }

    for box_layout in &layout.boxes {
        svg.push_str(&format!(
            "<g class=\"em-box\"><rect x=\"{:.3}\" y=\"{:.3}\" rx=\"3\" width=\"{:.3}\" height=\"{:.3}\" stroke=\"{}\" fill=\"{}\"/><foreignObject x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\"><div xmlns=\"http://www.w3.org/1999/xhtml\" style=\"display: table; height: 100%; width: 100%;\"><span style=\"display: table-cell; text-align: center; vertical-align: middle;\">{}</span></div></foreignObject></g>",
            box_layout.x,
            box_layout.y,
            box_layout.width,
            box_layout.height,
            box_layout.stroke,
            box_layout.fill,
            box_layout.x + 10.0,
            box_layout.y + 10.0,
            (box_layout.width - 20.0).max(1.0),
            (box_layout.height - 20.0).max(1.0),
            box_layout.html,
        ));
    }

    for relation in &layout.relations {
        let Some(source) = layout.boxes.get(relation.source_box) else {
            continue;
        };
        let Some(target) = layout.boxes.get(relation.target_box) else {
            continue;
        };
        let upwards = source.y > target.y;
        let source_x = source.x + (source.width * 2.0) / 3.0;
        let target_x = target.x + target.width / 3.0;
        let source_y = if upwards {
            source.y
        } else {
            source.y + source.height
        };
        let target_y = if upwards {
            target.y + target.height
        } else {
            target.y
        };
        svg.push_str(&format!(
            "<path class=\"em-relation\" fill=\"none\" stroke=\"#333333\" stroke-width=\"1\" marker-end=\"url(#em-arrowhead-my-svg)\" d=\"M{:.3} {:.3} L{:.3} {:.3}\"/>",
            source_x, source_y, target_x, target_y
        ));
    }

    svg.push_str("<defs><marker id=\"em-arrowhead-my-svg\" markerWidth=\"10\" markerHeight=\"7\" refX=\"10\" refY=\"3.5\" orient=\"auto\"><polygon points=\"0 0, 10 3.5, 0 7\" fill=\"#333333\"/></marker></defs>");
    svg
}

#[derive(Debug, Clone, Copy)]
struct CynefinDomainLayout {
    cx: f32,
    cy: f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

fn render_cynefin(layout: &CynefinLayout, theme: &Theme) -> String {
    let mut svg = String::new();
    let t = &theme.cynefin;
    let width = layout.diagram_width;
    let height = layout.diagram_height;
    let padding = layout.padding;
    let root_transform = format!("translate({:.3}, {:.3})", padding, padding);
    let domain_layouts = cynefin_domain_layouts(width, height);
    let seed = cynefin_hash_string("my-svg");

    svg.push_str(&format!(
        "<style>.cynefinDomain{{stroke:none;}}.cynefinDomainLabel{{font-size:{:.3}px;font-weight:bold;fill:{};}}.cynefinSubtitle{{font-size:{:.3}px;fill:{};font-style:italic;}}.cynefinItem{{fill-opacity:0.95;stroke:{};stroke-width:1;}}.cynefinItemText{{font-size:{:.3}px;fill:{};}}.cynefinItemOverflow{{fill-opacity:0.6;stroke:{};stroke-width:1;stroke-dasharray:3 2;}}.cynefinBoundary{{stroke:{};stroke-width:{:.3};stroke-dasharray:6 3;}}.cynefinCliff{{stroke:{};stroke-width:{:.3};}}.cynefinConfusion{{stroke:{};stroke-width:1.5;stroke-dasharray:4 2;}}.cynefinArrowLine{{stroke:{};stroke-width:{:.3};fill:none;}}.cynefinArrowHead{{fill:{};stroke:none;}}.cynefinArrowLabel{{font-size:{:.3}px;fill:{};}}.cynefinTitle{{font-size:{:.3}px;font-weight:bold;fill:{};}}</style>",
        t.domain_font_size,
        escape_xml(&t.label_color),
        t.item_font_size - 1.0,
        escape_xml(&t.text_color),
        escape_xml(&t.boundary_color),
        t.item_font_size,
        escape_xml(&t.text_color),
        escape_xml(&t.boundary_color),
        escape_xml(&t.boundary_color),
        t.boundary_width,
        escape_xml(&t.cliff_color),
        t.cliff_width,
        escape_xml(&t.boundary_color),
        escape_xml(&t.arrow_color),
        t.arrow_width,
        escape_xml(&t.arrow_color),
        t.item_font_size - 1.0,
        escape_xml(&t.text_color),
        t.domain_font_size + 2.0,
        escape_xml(&t.label_color),
    ));

    if !layout.transitions.is_empty() {
        svg.push_str(&format!(
            "<defs><marker id=\"cynefin-arrow-my-svg\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto-start-reverse\"><path d=\"M 0 0 L 10 5 L 0 10 z\" class=\"cynefinArrowHead\"/></marker></defs>"
        ));
    }

    svg.push_str(&format!("<g transform=\"{}\">", root_transform));
    svg.push_str("<g class=\"cynefin-backgrounds\">");
    for domain in [
        crate::ir::CynefinDomainName::Complex,
        crate::ir::CynefinDomainName::Complicated,
        crate::ir::CynefinDomainName::Chaotic,
        crate::ir::CynefinDomainName::Clear,
    ] {
        let l = cynefin_domain_layout(domain, &domain_layouts);
        svg.push_str(&format!(
            "<rect class=\"cynefinDomain\" x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" fill=\"{}\" fill-opacity=\"0.4\" stroke=\"none\"/>",
            l.x,
            l.y,
            l.w,
            l.h,
            escape_xml(cynefin_domain_bg(domain, t))
        ));
    }
    svg.push_str("</g>");

    svg.push_str("<g class=\"cynefin-boundaries\">");
    svg.push_str(&format!(
        "<path class=\"cynefinBoundary\" d=\"{}\" fill=\"none\"/>",
        cynefin_generate_fold_path(width, height, seed, layout.boundary_amplitude)
    ));
    svg.push_str(&format!(
        "<path class=\"cynefinBoundary\" d=\"{}\" fill=\"none\"/>",
        cynefin_generate_horizontal_boundary(
            width,
            height,
            seed.wrapping_add(100),
            layout.boundary_amplitude
        )
    ));
    svg.push_str(&format!(
        "<path class=\"cynefinCliff\" d=\"{}\" fill=\"none\"/>",
        cynefin_generate_cliff_path(width, height)
    ));
    svg.push_str("</g>");

    svg.push_str(&format!(
        "<path class=\"cynefinConfusion\" d=\"{}\" fill=\"{}\" fill-opacity=\"0.5\"/>",
        cynefin_generate_confusion_path(width / 2.0, height / 2.0, width * 0.15, height * 0.15),
        escape_xml(&t.confusion_bg)
    ));

    svg.push_str("<g class=\"cynefin-labels\">");
    for domain in [
        crate::ir::CynefinDomainName::Complex,
        crate::ir::CynefinDomainName::Complicated,
        crate::ir::CynefinDomainName::Chaotic,
        crate::ir::CynefinDomainName::Clear,
    ] {
        let l = cynefin_domain_layout(domain, &domain_layouts);
        let y = if layout.show_domain_descriptions {
            l.cy - 30.0
        } else {
            l.cy
        };
        svg.push_str(&format!(
            "<text class=\"cynefinDomainLabel\" x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"middle\" dominant-baseline=\"middle\">{}</text>",
            l.cx,
            y,
            domain.title()
        ));
    }
    let confusion_y = if layout.show_domain_descriptions {
        height / 2.0 - 10.0
    } else {
        height / 2.0
    };
    svg.push_str(&format!(
        "<text class=\"cynefinDomainLabel\" x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"middle\" dominant-baseline=\"middle\">Confusion</text>",
        width / 2.0,
        confusion_y
    ));
    svg.push_str("</g>");

    if layout.show_domain_descriptions {
        svg.push_str("<g class=\"cynefin-subtitles\">");
        for domain in [
            crate::ir::CynefinDomainName::Complex,
            crate::ir::CynefinDomainName::Complicated,
            crate::ir::CynefinDomainName::Chaotic,
            crate::ir::CynefinDomainName::Clear,
        ] {
            let l = cynefin_domain_layout(domain, &domain_layouts);
            let (model, practice) = cynefin_domain_meta(domain);
            svg.push_str(&format!(
                "<text class=\"cynefinSubtitle\" x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"middle\" dominant-baseline=\"middle\">{}</text>",
                l.cx,
                l.cy - 10.0,
                model
            ));
            svg.push_str(&format!(
                "<text class=\"cynefinSubtitle\" x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"middle\" dominant-baseline=\"middle\">{}</text>",
                l.cx,
                l.cy + 5.0,
                practice
            ));
        }
        svg.push_str(&format!(
            "<text class=\"cynefinSubtitle\" x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"middle\" dominant-baseline=\"middle\">Disorder</text>",
            width / 2.0,
            height / 2.0 + 8.0
        ));
        svg.push_str("</g>");
    }

    svg.push_str("<g class=\"cynefin-items\">");
    for domain in [
        crate::ir::CynefinDomainName::Complex,
        crate::ir::CynefinDomainName::Complicated,
        crate::ir::CynefinDomainName::Chaotic,
        crate::ir::CynefinDomainName::Clear,
        crate::ir::CynefinDomainName::Confusion,
    ] {
        let Some(items) = layout.domains.get(&domain) else {
            continue;
        };
        if items.is_empty() {
            continue;
        }
        render_cynefin_items(&mut svg, domain, items, layout, theme, &domain_layouts);
    }
    svg.push_str("</g>");

    if !layout.transitions.is_empty() {
        svg.push_str("<g class=\"cynefin-arrows\">");
        for transition in &layout.transitions {
            if transition.from == transition.to {
                continue;
            }
            let from = cynefin_domain_layout(transition.from, &domain_layouts);
            let to = cynefin_domain_layout(transition.to, &domain_layouts);
            let x1 = from.cx;
            let y1 = from.cy;
            let x2 = to.cx;
            let y2 = to.cy;
            let mx = (x1 + x2) / 2.0;
            let my = (y1 + y2) / 2.0;
            let dx = x2 - x1;
            let dy = y2 - y1;
            let len = (dx * dx + dy * dy).sqrt();
            if len <= f32::EPSILON {
                continue;
            }
            let offset_amount = len * 0.15;
            let nx = -dy / len;
            let ny = dx / len;
            let cpx = mx + nx * offset_amount;
            let cpy = my + ny * offset_amount;
            svg.push_str(&format!(
                "<path class=\"cynefinArrowLine\" d=\"M{:.3},{:.3} Q{:.3},{:.3} {:.3},{:.3}\" fill=\"none\" marker-end=\"url(#cynefin-arrow-my-svg)\"/>",
                x1, y1, cpx, cpy, x2, y2
            ));
            if let Some(label) = &transition.label {
                svg.push_str(&format!(
                    "<text class=\"cynefinArrowLabel\" x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"middle\" dominant-baseline=\"auto\">{}</text>",
                    cpx,
                    cpy - 6.0,
                    escape_xml(label)
                ));
            }
        }
        svg.push_str("</g>");
    }

    if let Some(title) = &layout.title {
        svg.push_str(&format!(
            "<text class=\"cynefinTitle\" x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"middle\" dominant-baseline=\"middle\">{}</text>",
            width / 2.0,
            -padding / 2.0,
            escape_xml(title)
        ));
    }
    svg.push_str("</g>");
    svg
}

fn render_cynefin_items(
    svg: &mut String,
    domain: crate::ir::CynefinDomainName,
    items: &[String],
    layout: &CynefinLayout,
    theme: &Theme,
    domain_layouts: &std::collections::HashMap<crate::ir::CynefinDomainName, CynefinDomainLayout>,
) {
    const MAX_CONFUSION_ITEMS: usize = 3;
    let t = &theme.cynefin;
    let l = cynefin_domain_layout(domain, domain_layouts);
    let is_confusion = domain == crate::ir::CynefinDomainName::Confusion;
    let render_count = if is_confusion {
        items.len().min(MAX_CONFUSION_ITEMS)
    } else {
        items.len()
    };
    let overflow_count = if is_confusion && items.len() > MAX_CONFUSION_ITEMS {
        items.len() - MAX_CONFUSION_ITEMS
    } else {
        0
    };
    let item_height = 26.0;
    let item_padding_x = 10.0;
    let start_y = if is_confusion {
        l.cy + if layout.show_domain_descriptions {
            22.0
        } else {
            14.0
        }
    } else {
        l.cy + if layout.show_domain_descriptions {
            25.0
        } else {
            15.0
        }
    };

    for (idx, item) in items.iter().take(render_count).enumerate() {
        let item_y = start_y + idx as f32 * (item_height + 4.0);
        let measured_width =
            text_metrics::measure_text_width(item, t.item_font_size, &theme.font_family)
                .unwrap_or_else(|| item.chars().count() as f32 * 7.0);
        let badge_width = measured_width + item_padding_x * 2.0;
        let item_x = l.cx - badge_width / 2.0;
        svg.push_str(&format!(
            "<g transform=\"translate({:.3}, {:.3})\"><rect class=\"cynefinItem\" x=\"0\" y=\"0\" width=\"{:.3}\" height=\"{:.3}\" rx=\"4\" ry=\"4\" fill=\"{}\" fill-opacity=\"0.95\"/><text class=\"cynefinItemText\" x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"middle\" dominant-baseline=\"central\">{}</text></g>",
            item_x,
            item_y,
            badge_width,
            item_height,
            escape_xml(cynefin_domain_bg(domain, t)),
            badge_width / 2.0,
            item_height / 2.0,
            escape_xml(item)
        ));
    }

    if overflow_count > 0 {
        let label = format!("+{} more", overflow_count);
        let overflow_y = start_y + render_count as f32 * (item_height + 4.0);
        let measured_width =
            text_metrics::measure_text_width(&label, t.item_font_size, &theme.font_family)
                .unwrap_or_else(|| label.chars().count() as f32 * 7.0);
        let badge_width = measured_width + item_padding_x * 2.0;
        let item_x = l.cx - badge_width / 2.0;
        svg.push_str(&format!(
            "<g transform=\"translate({:.3}, {:.3})\"><rect class=\"cynefinItemOverflow\" x=\"0\" y=\"0\" width=\"{:.3}\" height=\"{:.3}\" rx=\"4\" ry=\"4\" fill=\"{}\" fill-opacity=\"0.6\"/><text class=\"cynefinItemText\" x=\"{:.3}\" y=\"{:.3}\" text-anchor=\"middle\" dominant-baseline=\"central\">{}</text></g>",
            item_x,
            overflow_y,
            badge_width,
            item_height,
            escape_xml(cynefin_domain_bg(domain, t)),
            badge_width / 2.0,
            item_height / 2.0,
            escape_xml(&label)
        ));
    }
}

fn cynefin_domain_layouts(
    width: f32,
    height: f32,
) -> std::collections::HashMap<crate::ir::CynefinDomainName, CynefinDomainLayout> {
    let hw = width / 2.0;
    let hh = height / 2.0;
    [
        (
            crate::ir::CynefinDomainName::Complex,
            CynefinDomainLayout {
                cx: hw / 2.0,
                cy: hh / 2.0,
                x: 0.0,
                y: 0.0,
                w: hw,
                h: hh,
            },
        ),
        (
            crate::ir::CynefinDomainName::Complicated,
            CynefinDomainLayout {
                cx: hw + hw / 2.0,
                cy: hh / 2.0,
                x: hw,
                y: 0.0,
                w: hw,
                h: hh,
            },
        ),
        (
            crate::ir::CynefinDomainName::Chaotic,
            CynefinDomainLayout {
                cx: hw / 2.0,
                cy: hh + hh / 2.0,
                x: 0.0,
                y: hh,
                w: hw,
                h: hh,
            },
        ),
        (
            crate::ir::CynefinDomainName::Clear,
            CynefinDomainLayout {
                cx: hw + hw / 2.0,
                cy: hh + hh / 2.0,
                x: hw,
                y: hh,
                w: hw,
                h: hh,
            },
        ),
        (
            crate::ir::CynefinDomainName::Confusion,
            CynefinDomainLayout {
                cx: hw,
                cy: hh,
                x: hw * 0.7,
                y: hh * 0.7,
                w: hw * 0.6,
                h: hh * 0.6,
            },
        ),
    ]
    .into_iter()
    .collect()
}

fn cynefin_domain_layout(
    domain: crate::ir::CynefinDomainName,
    layouts: &std::collections::HashMap<crate::ir::CynefinDomainName, CynefinDomainLayout>,
) -> CynefinDomainLayout {
    layouts[&domain]
}

fn cynefin_domain_bg<'a>(
    domain: crate::ir::CynefinDomainName,
    theme: &'a crate::theme::CynefinTheme,
) -> &'a str {
    match domain {
        crate::ir::CynefinDomainName::Complex => &theme.complex_bg,
        crate::ir::CynefinDomainName::Complicated => &theme.complicated_bg,
        crate::ir::CynefinDomainName::Chaotic => &theme.chaotic_bg,
        crate::ir::CynefinDomainName::Clear => &theme.clear_bg,
        crate::ir::CynefinDomainName::Confusion => &theme.confusion_bg,
    }
}

fn cynefin_domain_meta(domain: crate::ir::CynefinDomainName) -> (&'static str, &'static str) {
    match domain {
        crate::ir::CynefinDomainName::Complex => ("Probe → Sense → Respond", "Emergent Practices"),
        crate::ir::CynefinDomainName::Complicated => {
            ("Sense → Analyse → Respond", "Good Practices")
        }
        crate::ir::CynefinDomainName::Clear => ("Sense → Categorise → Respond", "Best Practices"),
        crate::ir::CynefinDomainName::Chaotic => ("Act → Sense → Respond", "Novel Practices"),
        crate::ir::CynefinDomainName::Confusion => ("", "Disorder"),
    }
}

fn cynefin_hash_string(input: &str) -> i32 {
    let mut hash = 0_i32;
    for ch in input.encode_utf16() {
        hash = hash
            .wrapping_shl(5)
            .wrapping_sub(hash)
            .wrapping_add(ch as i32);
    }
    hash
}

fn cynefin_seeded_random(seed: i32) -> f32 {
    let mut t = seed.wrapping_add(0x6d2b79f5_u32 as i32);
    t = (t ^ ((t as u32 >> 15) as i32)).wrapping_mul(t | 1);
    t ^= t.wrapping_add((t ^ ((t as u32 >> 7) as i32)).wrapping_mul(t | 61));
    ((t ^ ((t as u32 >> 14) as i32)) as u32) as f32 / 4_294_967_296.0
}

fn cynefin_generate_fold_path(width: f32, height: f32, seed: i32, amplitude: f32) -> String {
    let cx = width / 2.0;
    let segments = 7;
    let seg_height = height / segments as f32;
    let mut points = Vec::new();
    for i in 0..=segments {
        let jitter =
            cynefin_seeded_random(seed.wrapping_add(i as i32 * 17)) * amplitude * 2.0 - amplitude;
        points.push((cx + jitter, i as f32 * seg_height));
    }
    let mut d = format!("M{:.3},{:.3}", points[0].0, points[0].1);
    for i in 0..segments {
        let p0 = points[i];
        let p1 = points[i + 1];
        let mid_y = (p0.1 + p1.1) / 2.0;
        let dir = if i % 2 == 0 { 1.0 } else { -1.0 };
        let offset =
            amplitude * 1.5 * dir * cynefin_seeded_random(seed.wrapping_add(i as i32 * 31 + 7));
        d.push_str(&format!(
            " C{:.3},{:.3} {:.3},{:.3} {:.3},{:.3}",
            p0.0 + offset,
            mid_y,
            p1.0 - offset,
            mid_y,
            p1.0,
            p1.1
        ));
    }
    d
}

fn cynefin_generate_horizontal_boundary(
    width: f32,
    height: f32,
    seed: i32,
    amplitude: f32,
) -> String {
    let cy = height / 2.0;
    let segments = 7;
    let seg_width = width / segments as f32;
    let mut points = Vec::new();
    for i in 0..=segments {
        let jitter =
            cynefin_seeded_random(seed.wrapping_add(i as i32 * 23)) * amplitude * 2.0 - amplitude;
        points.push((i as f32 * seg_width, cy + jitter));
    }
    let mut d = format!("M{:.3},{:.3}", points[0].0, points[0].1);
    for i in 0..segments {
        let p0 = points[i];
        let p1 = points[i + 1];
        let mid_x = (p0.0 + p1.0) / 2.0;
        let dir = if i % 2 == 0 { 1.0 } else { -1.0 };
        let offset =
            amplitude * 1.5 * dir * cynefin_seeded_random(seed.wrapping_add(i as i32 * 37 + 11));
        d.push_str(&format!(
            " C{:.3},{:.3} {:.3},{:.3} {:.3},{:.3}",
            mid_x,
            p0.1 + offset,
            mid_x,
            p1.1 - offset,
            p1.0,
            p1.1
        ));
    }
    d
}

fn cynefin_generate_cliff_path(width: f32, height: f32) -> String {
    let cx = width / 2.0;
    let top_y = height * 0.5;
    let bottom_y = height;
    let amplitude = width * 0.03;
    format!(
        "M{:.3},{:.3} C{:.3},{:.3} {:.3},{:.3} {:.3},{:.3} C{:.3},{:.3} {:.3},{:.3} {:.3},{:.3}",
        cx,
        top_y,
        cx + amplitude,
        top_y + (bottom_y - top_y) * 0.2,
        cx - amplitude * 1.5,
        top_y + (bottom_y - top_y) * 0.55,
        cx + amplitude * 0.5,
        top_y + (bottom_y - top_y) * 0.75,
        cx - amplitude,
        top_y + (bottom_y - top_y) * 0.85,
        cx + amplitude * 0.3,
        top_y + (bottom_y - top_y) * 0.95,
        cx,
        bottom_y
    )
}

fn cynefin_generate_confusion_path(cx: f32, cy: f32, rx: f32, ry: f32) -> String {
    format!(
        "M{:.3},{:.3} A{:.3},{:.3} 0 1,1 {:.3},{:.3} A{:.3},{:.3} 0 1,1 {:.3},{:.3} Z",
        cx - rx,
        cy,
        rx,
        ry,
        cx + rx,
        cy,
        rx,
        ry,
        cx - rx,
        cy
    )
}

fn render_pie(pie: &PieData, theme: &Theme, config: &LayoutConfig) -> String {
    let mut svg = String::new();
    let (cx, cy) = pie.center;
    let radius = pie.radius;
    if radius <= 0.0 {
        return svg;
    }

    let pie_cfg = &config.pie;
    let mut total: f32 = pie.legend.iter().map(|s| s.value.max(0.0)).sum();
    if total <= 0.0 {
        total = pie.slices.iter().map(|s| s.value.max(0.0)).sum();
    }

    let slice_stroke = theme.pie_stroke_color.as_str();
    let slice_stroke_width = theme.pie_stroke_width.max(1.2);

    for slice in &pie.slices {
        let span = (slice.end_angle - slice.start_angle).abs();
        if span <= 0.0001 {
            continue;
        }
        if span >= std::f32::consts::PI * 2.0 - 0.001 {
            svg.push_str(&format!(
                "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.3}\" opacity=\"{:.3}\"/>",
                cx,
                cy,
                radius,
                escape_xml(&slice.color),
                escape_xml(slice_stroke),
                slice_stroke_width,
                theme.pie_opacity
            ));
            continue;
        }
        let path = pie_slice_path(cx, cy, radius, slice.start_angle, slice.end_angle);
        svg.push_str(&format!(
            "<path d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.3}\" opacity=\"{:.3}\"/>",
            escape_xml(&path),
            escape_xml(&slice.color),
            escape_xml(slice_stroke),
            slice_stroke_width,
            theme.pie_opacity
        ));
    }

    if theme.pie_outer_stroke_width > 0.0 {
        let outer_radius = radius + theme.pie_outer_stroke_width / 2.0;
        svg.push_str(&format!(
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.3}\"/>",
            cx,
            cy,
            outer_radius,
            escape_xml(&theme.pie_outer_stroke_color),
            theme.pie_outer_stroke_width
        ));
    }

    // Add labels on slices (percent inside, category outside)
    #[derive(Clone)]
    struct PieLabel {
        text: String,
        font_size: f32,
        outside: bool,
        side: i32,
        x: f32,
        y: f32,
        edge_x: f32,
        edge_y: f32,
        line_color: String,
    }

    let mut labels: Vec<PieLabel> = Vec::new();
    let suppress_outside_labels = !pie.legend.is_empty();
    for slice in &pie.slices {
        let span = (slice.end_angle - slice.start_angle).abs();
        if span <= 0.0001 || total <= 0.0 {
            continue;
        }
        let percent = slice.value / total * 100.0;
        if percent < pie_cfg.min_percent {
            continue;
        }
        let percent_text = format!("{:.0}%", percent);
        let mid_angle = (slice.start_angle + slice.end_angle) / 2.0;
        let font_size = theme.pie_section_text_size;
        let arc_len = radius * span;
        let percent_width =
            text_metrics::get_computed_text_length(&percent_text, font_size, &theme.font_family);
        let outside = !suppress_outside_labels && (arc_len < percent_width * 1.35 || span < 0.4);
        let label_text = if outside {
            slice
                .label
                .lines
                .iter()
                .map(|l| l.text().into_owned())
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            percent_text.clone()
        };
        let (edge_x, edge_y) = pie_point(cx, cy, radius, mid_angle);
        let bump = (font_size * 1.6).max(radius * 0.18);
        let (label_x, label_y) = if outside {
            pie_point(cx, cy, radius + bump, mid_angle)
        } else {
            let label_radius = radius * pie_cfg.text_position;
            pie_point(cx, cy, label_radius, mid_angle)
        };
        labels.push(PieLabel {
            text: label_text,
            font_size,
            outside,
            side: if mid_angle.cos() >= 0.0 { 1 } else { -1 },
            x: label_x,
            y: label_y,
            edge_x,
            edge_y,
            line_color: slice.color.clone(),
        });
    }

    let min_y = cy - radius * 1.1;
    let max_y = cy + radius * 1.1;
    let min_gap = theme.pie_section_text_size * 1.2;

    let mut left: Vec<usize> = Vec::new();
    let mut right: Vec<usize> = Vec::new();
    for (idx, label) in labels.iter().enumerate() {
        if label.outside {
            if label.side >= 0 {
                right.push(idx);
            } else {
                left.push(idx);
            }
        }
    }

    let distribute = |indices: &mut Vec<usize>, labels: &mut [PieLabel]| {
        indices.sort_by(|&a, &b| {
            labels[a]
                .y
                .partial_cmp(&labels[b].y)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut prev = min_y - min_gap;
        for &idx in indices.iter() {
            let y = labels[idx].y.max(prev + min_gap);
            labels[idx].y = y;
            prev = y;
        }
        if let Some(&last_idx) = indices.last() {
            let overflow = labels[last_idx].y - max_y;
            if overflow > 0.0 {
                for &idx in indices.iter() {
                    labels[idx].y -= overflow;
                }
            }
        }
        if let Some(&first_idx) = indices.first() {
            let underflow = min_y - labels[first_idx].y;
            if underflow > 0.0 {
                for &idx in indices.iter() {
                    labels[idx].y += underflow;
                }
            }
        }
    };

    distribute(&mut left, &mut labels);
    distribute(&mut right, &mut labels);

    for label in labels {
        let mut anchor = "middle";
        let mut label_x = label.x;
        if label.outside {
            let bump = (label.font_size * 1.6).max(radius * 0.18);
            if label.side >= 0 {
                label_x = cx + radius + bump;
                anchor = "start";
            } else {
                label_x = cx - radius - bump;
                anchor = "end";
            }
            let elbow_x = if label.side >= 0 {
                label_x - 6.0
            } else {
                label_x + 6.0
            };
            svg.push_str(&format!(
                "<path d=\"M {sx:.2},{sy:.2} L {mx:.2},{ly:.2} L {lx:.2},{ly:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
                escape_xml(&label.line_color),
                sx = label.edge_x,
                sy = label.edge_y,
                mx = elbow_x,
                lx = label_x,
                ly = label.y
            ));
            let label_width = text_metrics::get_computed_text_length(
                label.text.as_str(),
                label.font_size,
                &theme.font_family,
            );
            let pad_x = (label.font_size * 0.35).max(4.0);
            let pad_y = (label.font_size * 0.25).max(2.5);
            let rect_w = label_width + pad_x * 2.0;
            let rect_h = label.font_size + pad_y * 2.0;
            let rect_x = if label.side >= 0 {
                label_x - pad_x
            } else {
                label_x - rect_w + pad_x
            };
            let rect_y = label.y - rect_h / 2.0;
            let bg = if theme.edge_label_background == "none" {
                theme.background.as_str()
            } else {
                theme.edge_label_background.as_str()
            };
            svg.push_str(&format!(
                "<rect x=\"{rect_x:.2}\" y=\"{rect_y:.2}\" width=\"{rect_w:.2}\" height=\"{rect_h:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" stroke=\"none\"/>",
                escape_xml(bg)
            ));
        }
        svg.push_str(&format!(
            "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"{}\" dominant-baseline=\"middle\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
            label_x,
            label.y,
            anchor,
            normalize_font_family(&theme.font_family),
            label.font_size,
            escape_xml(&theme.pie_section_text_color),
            label.text
        ));
    }

    for item in &pie.legend {
        let rect_x = item.x;
        let rect_y = item.y;
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.3}\"/>",
            rect_x,
            rect_y,
            item.marker_size,
            item.marker_size,
            escape_xml(&item.color),
            escape_xml(&item.color),
            theme.pie_stroke_width
        ));
        let label_x = rect_x + item.marker_size + pie_cfg.legend_spacing;
        let label_y = rect_y + item.marker_size - pie_cfg.legend_spacing;
        svg.push_str(&text_block_svg_with_font_size(
            label_x,
            label_y,
            &item.label,
            theme,
            config,
            theme.pie_legend_text_size,
            "start",
            Some(theme.pie_legend_text_color.as_str()),
            true,
        ));
    }

    if let Some(title) = &pie.title {
        svg.push_str(&text_block_svg_with_font_size(
            title.x,
            title.y,
            &title.text,
            theme,
            config,
            theme.pie_title_text_size,
            "middle",
            Some(theme.pie_title_text_color.as_str()),
            true,
        ));
    }

    svg
}

fn pie_slice_path(cx: f32, cy: f32, radius: f32, start_angle: f32, end_angle: f32) -> String {
    let (sx, sy) = pie_point(cx, cy, radius, start_angle);
    let (ex, ey) = pie_point(cx, cy, radius, end_angle);
    let large_arc = if (end_angle - start_angle).abs() > std::f32::consts::PI {
        1
    } else {
        0
    };
    let sweep = 1;
    format!(
        "M {cx:.2} {cy:.2} L {sx:.2} {sy:.2} A {radius:.2} {radius:.2} 0 {large_arc} {sweep} {ex:.2} {ey:.2} Z"
    )
}

fn pie_point(cx: f32, cy: f32, radius: f32, angle: f32) -> (f32, f32) {
    (cx + radius * angle.sin(), cy - radius * angle.cos())
}

fn render_quadrant(
    layout: &crate::layout::QuadrantLayout,
    theme: &Theme,
    config: &LayoutConfig,
) -> String {
    let mut svg = String::new();
    let grid_x = layout.grid_x;
    let grid_y = layout.grid_y;
    let w = layout.grid_width;
    let h = layout.grid_height;
    let half_w = w / 2.0;
    let half_h = h / 2.0;
    let quadrant_config = &config.quadrant;

    let fmt = |value: f32| -> String {
        if (value - value.round()).abs() < 0.01 {
            format!("{:.0}", value.round())
        } else {
            let formatted = format!("{:.2}", value);
            formatted
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        }
    };
    let text_svg = |text: &TextBlock,
                    fill: &str,
                    font_size: f32,
                    dominant_baseline: &str,
                    text_anchor: &str,
                    x: f32,
                    y: f32,
                    rotation: f32|
     -> String {
        format!(
            "<text x=\"0\" y=\"0\" fill=\"{}\" font-size=\"{}\" dominant-baseline=\"{}\" text-anchor=\"{}\" transform=\"translate({}, {}) rotate({})\">{}</text>",
            escape_xml(fill),
            fmt(font_size),
            dominant_baseline,
            text_anchor,
            fmt(x),
            fmt(y),
            fmt(rotation),
            escape_xml(&text_block_plain(text))
        )
    };

    // Draw 4 quadrant backgrounds
    // Q1 top-right, Q2 top-left, Q3 bottom-left, Q4 bottom-right
    svg.push_str(&format!(
        "<g class=\"main\"><g class=\"quadrants\"><g class=\"quadrant\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        fmt(grid_x + half_w),
        fmt(grid_y),
        fmt(half_w),
        fmt(half_h),
        escape_xml(&theme.quadrant.fills[0])
    ));
    svg.push_str(&format!(
        "</g><g class=\"quadrant\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        fmt(grid_x),
        fmt(grid_y),
        fmt(half_w),
        fmt(half_h),
        escape_xml(&theme.quadrant.fills[1])
    ));
    svg.push_str(&format!(
        "</g><g class=\"quadrant\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        fmt(grid_x),
        fmt(grid_y + half_h),
        fmt(half_w),
        fmt(half_h),
        escape_xml(&theme.quadrant.fills[2])
    ));
    svg.push_str(&format!(
        "</g><g class=\"quadrant\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        fmt(grid_x + half_w),
        fmt(grid_y + half_h),
        fmt(half_w),
        fmt(half_h),
        escape_xml(&theme.quadrant.fills[3])
    ));
    svg.push_str("</g></g>");

    // Draw Mermaid's external border and center dividers.
    let external_width = quadrant_config.quadrant_external_border_stroke_width;
    let internal_width = quadrant_config.quadrant_internal_border_stroke_width;
    let half_external = external_width / 2.0;
    let external_stroke = escape_xml(&theme.quadrant.external_border_stroke_fill);
    let internal_stroke = escape_xml(&theme.quadrant.internal_border_stroke_fill);
    svg.push_str("<g class=\"border\">");
    let lines = [
        (
            grid_x - half_external,
            grid_y,
            grid_x + w + half_external,
            grid_y,
            external_width,
            external_stroke.as_str(),
        ),
        (
            grid_x + w,
            grid_y + half_external,
            grid_x + w,
            grid_y + h - half_external,
            external_width,
            external_stroke.as_str(),
        ),
        (
            grid_x - half_external,
            grid_y + h,
            grid_x + w + half_external,
            grid_y + h,
            external_width,
            external_stroke.as_str(),
        ),
        (
            grid_x,
            grid_y + half_external,
            grid_x,
            grid_y + h - half_external,
            external_width,
            external_stroke.as_str(),
        ),
        (
            grid_x + half_w,
            grid_y + half_external,
            grid_x + half_w,
            grid_y + h - half_external,
            internal_width,
            internal_stroke.as_str(),
        ),
        (
            grid_x + half_external,
            grid_y + half_h,
            grid_x + w - half_external,
            grid_y + half_h,
            internal_width,
            internal_stroke.as_str(),
        ),
    ];
    for (x1, y1, x2, y2, stroke_width, stroke) in lines {
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" style=\"stroke: {}; stroke-width: {};\"/>",
            fmt(x1),
            fmt(y1),
            fmt(x2),
            fmt(y2),
            stroke,
            fmt(stroke_width)
        ));
    }
    svg.push_str("</g>");

    // Title
    if let Some(ref title) = layout.title {
        svg.push_str("<g class=\"title\">");
        svg.push_str(&text_svg(
            title,
            &theme.quadrant.title_fill,
            quadrant_config.title_font_size,
            "hanging",
            "middle",
            layout.width / 2.0,
            quadrant_config.title_padding,
            0.0,
        ));
        svg.push_str("</g>");
    }

    // Quadrant labels
    let q_label_top = !layout.points.is_empty();
    let q_baseline = if q_label_top { "hanging" } else { "middle" };
    let label_positions = [
        (
            grid_x + half_w + half_w / 2.0,
            if q_label_top {
                grid_y + quadrant_config.quadrant_text_top_padding
            } else {
                grid_y + half_h / 2.0
            },
        ),
        (
            grid_x + half_w / 2.0,
            if q_label_top {
                grid_y + quadrant_config.quadrant_text_top_padding
            } else {
                grid_y + half_h / 2.0
            },
        ),
        (
            grid_x + half_w / 2.0,
            if q_label_top {
                grid_y + half_h + quadrant_config.quadrant_text_top_padding
            } else {
                grid_y + half_h + half_h / 2.0
            },
        ),
        (
            grid_x + half_w + half_w / 2.0,
            if q_label_top {
                grid_y + half_h + quadrant_config.quadrant_text_top_padding
            } else {
                grid_y + half_h + half_h / 2.0
            },
        ),
    ];
    svg.push_str("<g class=\"labels\">");
    for (i, label_opt) in layout.quadrant_labels.iter().enumerate() {
        if let Some(label) = label_opt {
            let (lx, ly) = label_positions[i];
            svg.push_str(&text_svg(
                label,
                &theme.quadrant.text_fills[i],
                quadrant_config.quadrant_label_font_size,
                q_baseline,
                "middle",
                lx,
                ly,
                0.0,
            ));
        }
    }

    // Axis labels
    let has_points = !layout.points.is_empty();
    let x_axis_position = if has_points {
        "bottom"
    } else {
        quadrant_config.x_axis_position.as_str()
    };
    let title_space = if layout.title.is_some() {
        quadrant_config.title_font_size + quadrant_config.title_padding * 2.0
    } else {
        0.0
    };
    let draw_x_middle = layout.x_axis_right.is_some();
    let draw_y_middle = layout.y_axis_top.is_some();
    let x_label_y = if x_axis_position == "top" {
        quadrant_config.x_axis_label_padding + title_space
    } else {
        quadrant_config.x_axis_label_padding + grid_y + h + quadrant_config.quadrant_padding
    };
    let x_anchor = if draw_x_middle { "middle" } else { "start" };
    if let Some(ref x_left) = layout.x_axis_left {
        let x = grid_x + if draw_x_middle { half_w / 2.0 } else { 0.0 };
        svg.push_str(&text_svg(
            x_left,
            &theme.quadrant.x_axis_text_fill,
            quadrant_config.x_axis_label_font_size,
            "hanging",
            x_anchor,
            x,
            x_label_y,
            0.0,
        ));
    }
    if let Some(ref x_right) = layout.x_axis_right {
        let x = grid_x + half_w + if draw_x_middle { half_w / 2.0 } else { 0.0 };
        svg.push_str(&text_svg(
            x_right,
            &theme.quadrant.x_axis_text_fill,
            quadrant_config.x_axis_label_font_size,
            "hanging",
            x_anchor,
            x,
            x_label_y,
            0.0,
        ));
    }
    let y_axis_x = if quadrant_config.y_axis_position == "left" {
        quadrant_config.y_axis_label_padding
    } else {
        quadrant_config.y_axis_label_padding + grid_x + w + quadrant_config.quadrant_padding
    };
    let y_anchor = if draw_y_middle { "middle" } else { "start" };
    if let Some(ref y_bottom) = layout.y_axis_bottom {
        let y = grid_y + h - if draw_y_middle { half_h / 2.0 } else { 0.0 };
        svg.push_str(&text_svg(
            y_bottom,
            &theme.quadrant.y_axis_text_fill,
            quadrant_config.y_axis_label_font_size,
            "hanging",
            y_anchor,
            y_axis_x,
            y,
            -90.0,
        ));
    }
    if let Some(ref y_top) = layout.y_axis_top {
        let y = grid_y + half_h - if draw_y_middle { half_h / 2.0 } else { 0.0 };
        svg.push_str(&text_svg(
            y_top,
            &theme.quadrant.y_axis_text_fill,
            quadrant_config.y_axis_label_font_size,
            "hanging",
            y_anchor,
            y_axis_x,
            y,
            -90.0,
        ));
    }
    svg.push_str("</g>");

    // Data points
    svg.push_str("<g class=\"data-points\">");
    for point in &layout.points {
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>",
            fmt(point.x),
            fmt(point.y),
            fmt(point.radius),
            escape_xml(&point.color),
            escape_xml(&point.stroke_color),
            escape_xml(&point.stroke_width)
        ));
        svg.push_str(&text_svg(
            &point.label,
            &theme.quadrant.point_text_fill,
            quadrant_config.point_label_font_size,
            "hanging",
            "middle",
            point.x,
            point.y + quadrant_config.point_text_padding,
            0.0,
        ));
    }
    svg.push_str("</g></g>");

    svg
}

fn render_gantt(
    layout: &crate::layout::GanttLayout,
    _theme: &Theme,
    _config: &LayoutConfig,
) -> String {
    let mut svg = String::new();
    let font_family = "trebuchet ms,verdana,arial,sans-serif";
    let axis_y = layout.chart_y + layout.chart_height;
    let grid_top = 35.0;
    let grid_y2 = -axis_y + grid_top;

    for range in &layout.exclude_ranges {
        svg.push_str(&format!(
            "<rect x=\"{:.0}\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\" fill=\"#eeeeee\"/>",
            range.x, range.y, range.width, range.height
        ));
    }

    svg.push_str(&format!(
        "<g class=\"grid\" transform=\"translate({:.0}, {:.0})\" fill=\"none\" font-size=\"10\" font-family=\"sans-serif\" text-anchor=\"middle\">",
        layout.chart_x, axis_y
    ));
    svg.push_str(&format!(
        "<path class=\"domain\" stroke=\"currentColor\" d=\"M0.5,{:.0}V0.5H{:.1}V{:.0}\"/>",
        grid_y2,
        layout.chart_width + 0.5,
        grid_y2
    ));
    for tick in &layout.ticks {
        let local_x = tick.x - layout.chart_x;
        svg.push_str(&format!(
            "<g class=\"tick\" opacity=\"1\" transform=\"translate({:.1},0)\"><line stroke=\"currentColor\" y2=\"{:.0}\"/><text fill=\"#000\" y=\"3\" dy=\"1em\" stroke=\"none\" font-size=\"10\" style=\"text-anchor: middle;\">{}</text></g>",
            local_x,
            grid_y2,
            escape_xml(&tick.label)
        ));
    }
    svg.push_str("</g>");

    svg.push_str("<g>");
    let mut rendered_section_orders = HashSet::new();
    for task in &layout.tasks {
        if !rendered_section_orders.insert(task.order) {
            continue;
        }
        let fill = match task.section_index {
            0 => "#6666ff",
            2 => "#fff400",
            _ => "#ffffff",
        };
        let opacity = if task.section_index == 0 { 0.098 } else { 0.2 };
        svg.push_str(&format!(
            "<rect x=\"0\" y=\"{:.0}\" width=\"{:.1}\" height=\"{:.0}\" fill=\"{}\" opacity=\"{:.3}\" stroke=\"none\"/>",
            task.order as f32 * layout.row_height + layout.chart_y - 2.0,
            layout.chart_x + layout.chart_width + 37.5,
            layout.row_height,
            fill,
            opacity
        ));
    }
    svg.push_str("</g>");

    svg.push_str("<g>");
    for task in &layout.tasks {
        let (stroke, stroke_width) = gantt_task_stroke(task);
        let transform = if task.milestone {
            format!(
                " transform=\"translate({:.2} {:.2}) rotate(45) scale(0.8,0.8) translate({:.2} {:.2})\"",
                task.transform_origin_x,
                task.transform_origin_y,
                -task.transform_origin_x,
                -task.transform_origin_y
            )
        } else {
            String::new()
        };
        svg.push_str(&format!(
            "<rect id=\"my-svg-{}\" rx=\"3\" ry=\"3\" x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{} />",
            escape_xml(&task.id),
            task.x,
            task.y,
            task.width.max(0.0),
            task.height,
            task.color,
            stroke,
            stroke_width,
            transform
        ));
    }
    for task in &layout.tasks {
        let label_text = task
            .label
            .lines
            .iter()
            .find(|line| !line.text().trim().is_empty())
            .map(|line| line.text().into_owned())
            .unwrap_or_default();
        if label_text.is_empty() {
            continue;
        }
        let fill = gantt_task_text_fill(task);
        let font_size = if task.vert { 15.0 } else { 11.0 };
        let font_style = if task.milestone {
            " font-style=\"italic\""
        } else {
            ""
        };
        svg.push_str(&format!(
            "<text id=\"my-svg-{}-text\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"{}\" font-family=\"{}\" font-size=\"{:.0}\" fill=\"{}\"{}>{}</text>",
            escape_xml(&task.id),
            task.label_x,
            task.label_y,
            task.label_anchor,
            font_family,
            font_size,
            fill,
            font_style,
            escape_xml(&label_text)
        ));
    }
    svg.push_str("</g>");

    svg.push_str("<g>");
    for section in &layout.sections {
        let label_text = section
            .label
            .lines
            .iter()
            .find(|line| !line.text().trim().is_empty())
            .map(|line| line.text().into_owned())
            .unwrap_or_default();
        if label_text.is_empty() {
            continue;
        }
        svg.push_str(&format!(
            "<text dy=\"0em\" x=\"10\" y=\"{:.0}\" font-size=\"11\" fill=\"#333\" text-anchor=\"start\" font-family=\"{}\"><tspan alignment-baseline=\"central\" x=\"10\">{}</tspan></text>",
            section.y,
            font_family,
            escape_xml(&label_text)
        ));
    }
    svg.push_str("</g>");

    if let Some(today_x) = layout.today_x {
        svg.push_str(&format!(
            "<g class=\"today\"><line x1=\"{:.0}\" x2=\"{:.0}\" y1=\"25\" y2=\"{:.0}\" fill=\"none\" stroke=\"red\" stroke-width=\"2\"/></g>",
            today_x,
            today_x,
            axis_y + 25.0
        ));
    }

    if let Some(ref title) = layout.title {
        let title_text = title
            .lines
            .first()
            .map(|line| line.text().into_owned())
            .unwrap_or_default();
        svg.push_str(&format!(
            "<text x=\"{:.0}\" y=\"{:.0}\" text-anchor=\"middle\" font-size=\"18\" fill=\"#333\" font-family=\"{}\">{}</text>",
            layout.chart_x + layout.chart_width / 2.0,
            layout.title_y,
            font_family,
            escape_xml(&title_text)
        ));
    } else {
        svg.push_str(&format!(
            "<text x=\"{:.0}\" y=\"{:.0}\" text-anchor=\"middle\" font-size=\"18\" fill=\"#333\" font-family=\"{}\"/>",
            layout.chart_x + layout.chart_width / 2.0,
            layout.title_y,
            font_family
        ));
    }

    svg
}

fn gantt_task_stroke(task: &crate::layout::GanttTaskLayout) -> (&'static str, &'static str) {
    if task.vert {
        ("navy", "2")
    } else if task.crit {
        ("#ff8888", "2")
    } else if task.done {
        ("grey", "2")
    } else {
        ("#534fbc", "2")
    }
}

fn gantt_task_text_fill(task: &crate::layout::GanttTaskLayout) -> &'static str {
    if task.vert {
        "navy"
    } else if !task.label_inside {
        "black"
    } else if task.active || task.done {
        "black"
    } else {
        "white"
    }
}

fn render_xychart(
    layout: &crate::layout::XYChartLayout,
    theme: &Theme,
    config: &LayoutConfig,
) -> String {
    let mut svg = String::new();
    let chart_config = &config.xychart;
    let x_axis_config = &chart_config.x_axis;
    let y_axis_config = &chart_config.y_axis;
    let plot_bottom = layout.plot_y + layout.plot_height;
    let x_axis_line_y = plot_bottom + x_axis_config.axis_line_width / 2.0;
    let x_tick_start_y = plot_bottom
        + if x_axis_config.show_axis_line {
            x_axis_config.axis_line_width
        } else {
            0.0
        };
    let x_label_y = plot_bottom
        + x_axis_config.label_padding
        + if x_axis_config.show_tick {
            x_axis_config.tick_length
        } else {
            0.0
        }
        + if x_axis_config.show_axis_line {
            x_axis_config.axis_line_width
        } else {
            0.0
        };
    let y_axis_line_x = layout.plot_x - y_axis_config.axis_line_width / 2.0;
    let y_tick_start_x = layout.plot_x
        - if y_axis_config.show_axis_line {
            y_axis_config.axis_line_width
        } else {
            0.0
        };
    let y_label_x = layout.plot_x
        - if y_axis_config.show_label {
            y_axis_config.label_padding
        } else {
            0.0
        }
        - if y_axis_config.show_tick {
            y_axis_config.tick_length
        } else {
            0.0
        }
        - if y_axis_config.show_axis_line {
            y_axis_config.axis_line_width
        } else {
            0.0
        };

    svg.push_str(&format!(
        "<g class=\"main\"><rect width=\"{:.2}\" height=\"{:.2}\" class=\"background\" fill=\"{}\"/>",
        layout.width,
        layout.height,
        escape_xml(&theme.xy_chart.background_color)
    ));

    if let Some(ref title) = layout.title {
        svg.push_str("<g class=\"chart-title\">");
        svg.push_str(&format!(
            "<text x=\"0\" y=\"0\" fill=\"{}\" font-size=\"{:.0}\" dominant-baseline=\"middle\" text-anchor=\"middle\" transform=\"translate({:.2}, {:.2}) rotate(0)\">{}</text>",
            escape_xml(&theme.xy_chart.title_color),
            chart_config.title_font_size,
            layout.width / 2.0,
            layout.title_y,
            escape_xml(&text_block_plain(title))
        ));
        svg.push_str("</g>");
    }

    svg.push_str("<g class=\"plot\">");
    for bar in &layout.bars {
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"0\"/>",
            bar.x,
            bar.y,
            bar.width,
            bar.height,
            escape_xml(&bar.color),
            escape_xml(&bar.color)
        ));
    }
    if chart_config.show_data_label && !layout.bars.is_empty() {
        let label_size = xy_data_label_font_size(&layout.bars);
        for bar in &layout.bars {
            svg.push_str(&format!(
                "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" dominant-baseline=\"hanging\" fill=\"{}\" font-size=\"{:.0}px\">{}</text>",
                bar.x + bar.width / 2.0,
                bar.y + 10.0,
                escape_xml(&theme.xy_chart.data_label_color),
                label_size,
                escape_xml(&format_xy_value(bar.value))
            ));
        }
    }

    for line in &layout.lines {
        if line.points.len() >= 2 {
            let path: String = line
                .points
                .iter()
                .enumerate()
                .map(|(i, (x, y))| {
                    if i == 0 {
                        format!("M {:.2},{:.2}", x, y)
                    } else {
                        format!(" L {:.2},{:.2}", x, y)
                    }
                })
                .collect();
            svg.push_str(&format!(
                "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
                path,
                escape_xml(&line.color)
            ));
        }
    }
    svg.push_str("</g>");

    svg.push_str("<g class=\"bottom-axis\">");
    if x_axis_config.show_axis_line {
        svg.push_str(&format!(
            "<g class=\"axis-line\"><path d=\"M {:.2},{:.2} L {:.2},{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.0}\"/></g>",
            layout.plot_x,
            x_axis_line_y,
            layout.width,
            x_axis_line_y,
            escape_xml(&theme.xy_chart.x_axis_line_color),
            x_axis_config.axis_line_width
        ));
    }
    if x_axis_config.show_label {
        svg.push_str("<g class=\"label\">");
        for (label, x) in &layout.x_axis_categories {
            svg.push_str(&format!(
                "<text x=\"0\" y=\"0\" fill=\"{}\" font-size=\"{:.0}\" dominant-baseline=\"text-before-edge\" text-anchor=\"middle\" transform=\"translate({:.2}, {:.2}) rotate(0)\">{}</text>",
                escape_xml(&theme.xy_chart.x_axis_label_color),
                x_axis_config.label_font_size,
                x,
                x_label_y,
                escape_xml(label)
            ));
        }
        svg.push_str("</g>");
    }
    if x_axis_config.show_tick {
        svg.push_str("<g class=\"ticks\">");
        for (_, x) in &layout.x_axis_categories {
            svg.push_str(&format!(
                "<path d=\"M {:.2},{:.2} L {:.2},{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.0}\"/>",
                x,
                x_tick_start_y,
                x,
                x_tick_start_y + x_axis_config.tick_length,
                escape_xml(&theme.xy_chart.x_axis_tick_color),
                x_axis_config.tick_width
            ));
        }
        svg.push_str("</g>");
    }
    if x_axis_config.show_title
        && let Some(ref x_label) = layout.x_axis_label
    {
        svg.push_str(&format!(
            "<g class=\"title\"><text x=\"0\" y=\"0\" fill=\"{}\" font-size=\"{:.0}\" dominant-baseline=\"text-before-edge\" text-anchor=\"middle\" transform=\"translate({:.2}, {:.2}) rotate(0)\">{}</text></g>",
            escape_xml(&theme.xy_chart.x_axis_title_color),
            x_axis_config.title_font_size,
            layout.plot_x + layout.plot_width / 2.0,
            layout.x_axis_label_y,
            escape_xml(&text_block_plain(x_label))
        ));
    }
    svg.push_str("</g>");

    svg.push_str("<g class=\"left-axis\">");
    if y_axis_config.show_axis_line {
        svg.push_str(&format!(
            "<g class=\"axisl-line\"><path d=\"M {:.2},{:.2} L {:.2},{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.0}\"/></g>",
            y_axis_line_x,
            layout.plot_y,
            y_axis_line_x,
            plot_bottom,
            escape_xml(&theme.xy_chart.y_axis_line_color),
            y_axis_config.axis_line_width
        ));
    }
    if y_axis_config.show_label {
        svg.push_str("<g class=\"label\">");
        for (label, y) in &layout.y_axis_ticks {
            svg.push_str(&format!(
                "<text x=\"0\" y=\"0\" fill=\"{}\" font-size=\"{:.0}\" dominant-baseline=\"middle\" text-anchor=\"end\" transform=\"translate({:.2}, {:.2}) rotate(0)\">{}</text>",
                escape_xml(&theme.xy_chart.y_axis_label_color),
                y_axis_config.label_font_size,
                y_label_x,
                y,
                escape_xml(label)
            ));
        }
        svg.push_str("</g>");
    }
    if y_axis_config.show_tick {
        svg.push_str("<g class=\"ticks\">");
        for (_, y) in &layout.y_axis_ticks {
            svg.push_str(&format!(
                "<path d=\"M {:.2},{:.2} L {:.2},{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.0}\"/>",
                y_tick_start_x,
                y,
                y_tick_start_x - y_axis_config.tick_length,
                y,
                escape_xml(&theme.xy_chart.y_axis_tick_color),
                y_axis_config.tick_width
            ));
        }
        svg.push_str("</g>");
    }
    if y_axis_config.show_title
        && let Some(ref y_label) = layout.y_axis_label
    {
        svg.push_str(&format!(
            "<g class=\"title\"><text x=\"0\" y=\"0\" fill=\"{}\" font-size=\"{:.0}\" dominant-baseline=\"text-before-edge\" text-anchor=\"middle\" transform=\"translate({:.2}, {:.2}) rotate(270)\">{}</text></g>",
            escape_xml(&theme.xy_chart.y_axis_title_color),
            y_axis_config.title_font_size,
            layout.y_axis_label_x,
            layout.plot_y + layout.plot_height / 2.0,
            escape_xml(&text_block_plain(y_label))
        ));
    }
    svg.push_str("</g></g>");

    svg
}

fn text_block_plain(block: &TextBlock) -> String {
    block
        .lines
        .iter()
        .map(|line| line.text().into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

fn xy_data_label_font_size(bars: &[crate::layout::XYChartBarLayout]) -> f32 {
    let mut size: f32 = 25.0;
    for bar in bars {
        let label_len = format_xy_value(bar.value).chars().count().max(1) as f32;
        let width_limit = bar.width / (label_len * 0.72);
        let height_limit = (bar.height - 10.0).max(8.0);
        size = size.min(width_limit).min(height_limit);
    }
    size.floor().clamp(8.0, 25.0)
}

fn format_xy_value(value: f32) -> String {
    let normalized = if value.abs() < 0.000_001 { 0.0 } else { value };
    if (normalized - normalized.round()).abs() < 0.000_001 {
        return format!("{:.0}", normalized.round());
    }
    let mut text = format!("{:.6}", normalized);
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn render_timeline(
    layout: &crate::layout::TimelineLayout,
    theme: &Theme,
    _config: &LayoutConfig,
) -> String {
    let mut svg = String::new();
    let font_family = normalize_font_family(&theme.font_family);
    let font_size = theme.font_size;

    // ── Card shape constants (matching JS) ─────────────────────────────
    let card_path_w: f32 = 180.0;
    let card_line_w: f32 = 190.0;
    let card_path_h: f32 = 62.8;
    let card_line_y: f32 = 67.8;

    // ── HSL color system matching JS ───────────────────────────────────
    // Returns (fill_hsl, text_color, line_hsl) for a given section index.
    // Default HSL palette (matches JS CSS class colors).
    // idx=-1 → section--1 (first), idx=0 → section-0, etc.
    let default_colors: [(&str, &str, &str); 11] = [
        (
            "hsl(240, 100%, 76.2745098039%)",
            "#ffffff",
            "hsl(60, 100%, 86.2745098039%)",
        ),
        (
            "hsl(60, 100%, 73.5294117647%)",
            "black",
            "hsl(240, 100%, 83.5294117647%)",
        ),
        (
            "hsl(80, 100%, 76.2745098039%)",
            "black",
            "hsl(260, 100%, 86.2745098039%)",
        ),
        (
            "hsl(270, 100%, 76.2745098039%)",
            "#ffffff",
            "hsl(90, 100%, 86.2745098039%)",
        ),
        (
            "hsl(300, 100%, 76.2745098039%)",
            "black",
            "hsl(120, 100%, 86.2745098039%)",
        ),
        (
            "hsl(330, 100%, 76.2745098039%)",
            "black",
            "hsl(150, 100%, 86.2745098039%)",
        ),
        (
            "hsl(0, 100%, 76.2745098039%)",
            "black",
            "hsl(180, 100%, 86.2745098039%)",
        ),
        (
            "hsl(30, 100%, 76.2745098039%)",
            "black",
            "hsl(210, 100%, 86.2745098039%)",
        ),
        (
            "hsl(90, 100%, 76.2745098039%)",
            "black",
            "hsl(270, 100%, 86.2745098039%)",
        ),
        (
            "hsl(150, 100%, 76.2745098039%)",
            "black",
            "hsl(330, 100%, 86.2745098039%)",
        ),
        (
            "hsl(180, 100%, 76.2745098039%)",
            "black",
            "hsl(0, 100%, 86.2745098039%)",
        ),
    ];

    // Custom cScale colors from themeVariables override the default palette.
    let has_custom = !theme.cscale_colors.is_empty();
    let custom_colors: Vec<(String, &str, String)> = if has_custom {
        theme
            .cscale_colors
            .iter()
            .map(|c| {
                let text_color = "black";
                // JS applies the same section class to both path and line,
                // so the divider line uses the same fill color as the card.
                let line_color = c.clone();
                (c.clone(), text_color, line_color)
            })
            .collect()
    } else {
        Vec::new()
    };

    // Returns (fill, text_color, line_color) for a given section index.
    let section_colors = |idx: i32| -> (String, String, String) {
        if has_custom {
            let i = ((idx + 1).rem_euclid(custom_colors.len() as i32)) as usize;
            let (ref f, tc, ref lc) = custom_colors[i];
            (f.clone(), tc.to_string(), lc.clone())
        } else {
            let i = ((idx + 1).rem_euclid(11)) as usize;
            let (f, tc, lc) = default_colors[i];
            (f.to_string(), tc.to_string(), lc.to_string())
        }
    };

    // ── Arrowhead marker definition ────────────────────────────────────
    svg.push_str(
        "<defs><marker id=\"timeline-arrowhead\" refX=\"5\" refY=\"2\" \
         markerWidth=\"6\" markerHeight=\"4\" orient=\"auto\">\
         <path d=\"M 0,0 V 4 L6,2 Z\"/></marker></defs>",
    );

    // ── Section headers ────────────────────────────────────────────────
    for section in &layout.sections {
        let (fill, text_color, line_color) = section_colors(section.section_idx);
        let w = section.width - 10.0; // path internal width
        svg.push_str(&format!(
            "<g transform=\"translate({}, {})\"><g>\
             <path d=\"M0 {ch} v-{chm5} q0,-5 5,-5 h{w} q5,0 5,5 v{ch} H0 Z\" fill=\"{fill}\"/>\
             <line x1=\"0\" y1=\"{cly}\" x2=\"{lw}\" y2=\"{cly}\" stroke=\"{line_color}\" stroke-width=\"3\"/>\
             </g>\
             <g transform=\"translate({tx}, 10)\">\
             <text dy=\"1em\" alignment-baseline=\"middle\" dominant-baseline=\"middle\" \
             text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"{fs}\" fill=\"{text_color}\">\
             <tspan x=\"0\" dy=\"1em\">{label}</tspan></text></g></g>",
            section.x, section.y,
            ch = card_path_h,
            chm5 = card_path_h - 5.0,
            w = w,
            fill = fill,
            cly = card_line_y,
            lw = section.width,
            line_color = line_color,
            tx = section.width / 2.0,
            ff = font_family,
            fs = font_size,
            text_color = text_color,
            label = escape_xml(
                &section.label.lines.iter().map(|l| l.text().into_owned()).collect::<Vec<_>>().join(" ")
            ),
        ));
    }

    // ── Time period cards ──────────────────────────────────────────────
    for period in &layout.time_periods {
        let (fill, text_color, line_color) = section_colors(period.section_idx);
        let label_text = period
            .label
            .lines
            .iter()
            .map(|l| l.text().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        svg.push_str(&format!(
            "<g class=\"taskWrapper\" transform=\"translate({}, {})\">\
             <g>\
             <path d=\"M0 {ch} v-{chm5} q0,-5 5,-5 h{w} q5,0 5,5 v{ch} H0 Z\" fill=\"{fill}\"/>\
             <line x1=\"0\" y1=\"{cly}\" x2=\"{lw}\" y2=\"{cly}\" stroke=\"{line_color}\" stroke-width=\"3\"/>\
             </g>\
             <g transform=\"translate({tx}, 10)\">\
             <text dy=\"1em\" alignment-baseline=\"middle\" dominant-baseline=\"middle\" \
             text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"{fs}\" fill=\"{text_color}\">\
             <tspan x=\"0\" dy=\"1em\">{label}</tspan></text></g></g>",
            period.x, period.y,
            ch = card_path_h,
            chm5 = card_path_h - 5.0,
            w = card_path_w,
            fill = fill,
            cly = card_line_y,
            lw = card_line_w,
            line_color = line_color,
            tx = card_line_w / 2.0,
            ff = font_family,
            fs = font_size,
            text_color = text_color,
            label = escape_xml(&label_text),
        ));
    }

    // ── Dashed connectors (time card → event area) ─────────────────────
    for conn in &layout.connectors {
        svg.push_str(&format!(
            "<g class=\"lineWrapper\"><line x1=\"{x}\" y1=\"{sy}\" x2=\"{x}\" y2=\"{ey}\" \
             stroke-width=\"2\" stroke=\"black\" marker-end=\"url(#timeline-arrowhead)\" \
             stroke-dasharray=\"5,5\"/></g>",
            x = conn.x,
            sy = conn.start_y,
            ey = conn.end_y,
        ));
    }

    // ── Event cards (below axis) ───────────────────────────────────────
    for card in &layout.event_cards {
        let (fill, text_color, line_color) = section_colors(card.section_idx);
        // Event cards use the section fill + brightness(120%) filter.
        let card_h = card.height;
        let card_line = card_h + 5.0;

        // Build tspan text with wrapping
        let mut tspans = String::new();
        for (i, line) in card.lines.iter().enumerate() {
            let dy = if i == 0 { "1em" } else { "1.1em" };
            tspans.push_str(&format!(
                "<tspan x=\"0\" dy=\"{dy}\">{text}</tspan>",
                dy = dy,
                text = escape_xml(line),
            ));
        }

        svg.push_str(&format!(
            "<g class=\"eventWrapper\" transform=\"translate({x}, {y})\" style=\"filter: brightness(120%)\">\
             <g>\
             <path d=\"M0 {h} v-{hm5} q0,-5 5,-5 h{w} q5,0 5,5 v{h} H0 Z\" fill=\"{fill}\"/>\
             <line x1=\"0\" y1=\"{ly}\" x2=\"{lw}\" y2=\"{ly}\" stroke=\"{line_color}\" stroke-width=\"3\"/>\
             </g>\
             <g transform=\"translate({tx}, 10)\">\
             <text dy=\"1em\" alignment-baseline=\"middle\" dominant-baseline=\"middle\" \
             text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"{fs}\" fill=\"{text_color}\">\
             {tspans}</text></g></g>",
            x = card.x,
            y = card.y,
            h = card_h,
            hm5 = (card_h - 5.0_f32).max(0.0),
            w = card_path_w,
            fill = fill,
            ly = card_line,
            lw = card_line_w,
            line_color = line_color,
            tx = card_line_w / 2.0,
            ff = font_family,
            fs = font_size,
            text_color = text_color,
            tspans = tspans,
        ));
    }

    // ── Title ──────────────────────────────────────────────────────────
    if let Some(ref title) = layout.title {
        let title_text = title
            .lines
            .iter()
            .map(|l| l.text().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        svg.push_str(&format!(
            "<text x=\"{x}\" font-size=\"4ex\" font-weight=\"bold\" font-family=\"{ff}\" y=\"{y}\">{text}</text>",
            x = layout.title_x,
            y = layout.title_y,
            ff = font_family,
            text = escape_xml(&title_text),
        ));
    }

    // ── Horizontal timeline axis (with arrowhead) ──────────────────────
    svg.push_str(&format!(
        "<g class=\"lineWrapper\"><line x1=\"{x1}\" y1=\"{y}\" x2=\"{x2}\" y2=\"{y}\" \
         stroke-width=\"4\" stroke=\"black\" marker-end=\"url(#timeline-arrowhead)\"/></g>",
        x1 = layout.axis_start_x,
        y = layout.axis_y,
        x2 = layout.axis_end_x,
    ));

    svg
}

fn render_journey(layout: &JourneyLayout, theme: &Theme, config: &LayoutConfig) -> String {
    let mut svg = String::new();

    if let Some(ref title) = layout.title {
        let title_text = title
            .lines
            .iter()
            .map(|line| line.text())
            .collect::<Vec<_>>()
            .join(" ");
        let title_x = layout
            .sections
            .first()
            .map(|section| section.x)
            .unwrap_or(0.0);
        svg.push_str(&format!(
            "<text x=\"{title_x:.2}\" y=\"{:.2}\" font-size=\"4ex\" font-weight=\"bold\" font-family=\"{}\" fill=\"{}\">{}</text>",
            layout.title_y,
            normalize_font_family(&theme.font_family),
            escape_xml(&theme.primary_text_color),
            escape_xml(&title_text)
        ));
    }

    let mut actor_colors: HashMap<String, String> = HashMap::new();
    for actor in &layout.actors {
        actor_colors.insert(actor.name.clone(), actor.color.clone());
        svg.push_str(&format!(
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            actor.x,
            actor.y,
            actor.radius,
            actor.color,
            "#000"
        ));
        svg.push_str(&format!(
            "<text x=\"50\" y=\"{:.2}\" fill=\"#666\" font-family=\"{}\" font-size=\"{}\" text-anchor=\"start\">{}</text>",
            actor.y + 7.0,
            normalize_font_family(&theme.font_family),
            theme.font_size,
            escape_xml(&actor.name)
        ));
    }

    for section in &layout.sections {
        let fill = section.color.as_str();
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"3\" ry=\"3\" fill=\"{}\"/>",
            section.x,
            section.y,
            section.width,
            section.height,
            fill
        ));
        if !section.label.lines.is_empty()
            && !section
                .label
                .lines
                .iter()
                .all(|l| l.text().trim().is_empty())
        {
            let label_x = section.x + section.width / 2.0;
            let label_y = section.y + section.height / 2.0;
            svg.push_str(&text_block_svg(
                label_x,
                label_y,
                &section.label,
                theme,
                config,
                false,
                Some("#fff"),
            ));
        }
    }

    for task in &layout.tasks {
        let center_x = task.x + task.width / 2.0;
        svg.push_str(&format!(
            "<line x1=\"{center_x:.2}\" y1=\"{:.2}\" x2=\"{center_x:.2}\" y2=\"450\" stroke=\"#666\" stroke-width=\"1\" stroke-dasharray=\"4 2\"/>",
            task.y
        ));
        if let Some(score) = task.score {
            svg.push_str(&render_journey_face(center_x, task.score_y, score));
        }
        svg.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"3\" ry=\"3\" fill=\"{}\"/>",
            task.x,
            task.y,
            task.width,
            task.height,
            task.score_color
        ));
        if let Some(actor_y) = task.actor_y {
            let mut x_pos = task.x + 14.0;
            for actor in &task.actors {
                let color = actor_colors
                    .get(actor)
                    .map(|c| c.as_str())
                    .unwrap_or("#8FBC8F");
                svg.push_str(&format!(
                    "<circle cx=\"{x_pos:.2}\" cy=\"{actor_y:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"#000\" stroke-width=\"1\"><title>{}</title></circle>",
                    layout.actor_radius,
                    color,
                    escape_xml(actor)
                ));
                x_pos += 10.0;
            }
        }
        let label_x = task.x + task.width / 2.0;
        let label_y = task.y + task.height / 2.0;
        svg.push_str(&text_block_svg(
            label_x,
            label_y,
            &task.label,
            theme,
            config,
            false,
            Some("#fff"),
        ));
    }

    if let Some((x1, y, x2)) = layout.baseline {
        svg.push_str(&format!(
            "<line x1=\"{x1:.2}\" y1=\"{y:.2}\" x2=\"{x2:.2}\" y2=\"{y:.2}\" stroke=\"black\" stroke-width=\"4\"/>"
        ));
        svg.push_str(&format!(
            "<polygon points=\"{x2:.2},{y:.2} {ax:.2},{ay1:.2} {ax:.2},{ay2:.2}\" fill=\"black\"/>",
            ax = x2 - 6.0,
            ay1 = y - 4.0,
            ay2 = y + 4.0
        ));
    }

    svg
}

fn render_journey_face(cx: f32, cy: f32, score: f32) -> String {
    let radius = 15.0_f32;
    let mut svg = String::new();
    svg.push_str(&format!(
        "<circle cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{radius:.2}\" fill=\"#FFF8DC\" stroke=\"#999\" stroke-width=\"2\"/>"
    ));
    svg.push_str(&format!(
        "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"1.5\" fill=\"#666\" stroke=\"#666\" stroke-width=\"2\"/>",
        cx - radius / 3.0,
        cy - radius / 3.0
    ));
    svg.push_str(&format!(
        "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"1.5\" fill=\"#666\" stroke=\"#666\" stroke-width=\"2\"/>",
        cx + radius / 3.0,
        cy - radius / 3.0
    ));
    if score > 3.0 {
        svg.push_str(&format!(
            "<path d=\"M {:.2} {:.2} A 7.5 7.5 0 0 0 {:.2} {:.2}\" fill=\"none\" stroke=\"#666\" stroke-width=\"1\"/>",
            cx - 7.5,
            cy + 2.0,
            cx + 7.5,
            cy + 2.0
        ));
    } else if score < 3.0 {
        svg.push_str(&format!(
            "<path d=\"M {:.2} {:.2} A 7.5 7.5 0 0 1 {:.2} {:.2}\" fill=\"none\" stroke=\"#666\" stroke-width=\"1\"/>",
            cx - 7.5,
            cy + 7.0,
            cx + 7.5,
            cy + 7.0
        ));
    } else {
        svg.push_str(&format!(
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"#666\" stroke-width=\"1\"/>",
            cx - 5.0,
            cy + 7.0,
            cx + 5.0,
            cy + 7.0
        ));
    }
    svg
}

fn render_gitgraph(gitgraph: &GitGraphLayout, theme: &Theme, config: &LayoutConfig) -> String {
    let gg = &config.gitgraph;
    let mut svg = String::new();
    svg.push_str("<g>");

    if gg.show_branches {
        for branch in &gitgraph.branches {
            let (x1, y1, x2, y2) = match gitgraph.direction {
                crate::ir::Direction::TopDown => {
                    (branch.pos, gg.default_pos, branch.pos, gitgraph.max_pos)
                }
                crate::ir::Direction::BottomTop => {
                    (branch.pos, gitgraph.max_pos, branch.pos, gg.default_pos)
                }
                _ => (0.0, branch.pos - 2.0, gitgraph.max_pos, branch.pos - 2.0),
            };
            svg.push_str(&format!(
                "<line x1=\"{x1:.2}\" y1=\"{y1:.2}\" x2=\"{x2:.2}\" y2=\"{y2:.2}\" stroke=\"{}\" stroke-width=\"{}\" stroke-dasharray=\"{}\"/>",
                escape_xml(&theme.line_color),
                gg.branch_stroke_width,
                escape_xml(&gg.branch_dasharray)
            ));

            let color_idx = branch.index % theme.git_colors.len();
            let label_color = theme.git_colors[color_idx].as_str();
            let text_color = theme.git_branch_label_colors[color_idx].as_str();
            let label = &branch.label;

            svg.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" ry=\"{:.2}\" fill=\"{}\" stroke=\"none\"/>",
                label.bg_x,
                label.bg_y,
                label.bg_width,
                label.bg_height,
                gg.branch_label_corner_radius,
                gg.branch_label_corner_radius,
                escape_xml(label_color)
            ));

            let branch_font_size = if gg.branch_label_font_size > 0.0 {
                gg.branch_label_font_size
            } else {
                theme.font_size
            };
            svg.push_str(&render_gitgraph_multiline_text(
                label.text_x,
                label.text_y,
                &branch.name,
                &theme.font_family,
                branch_font_size,
                1.0,
                text_color,
            ));
        }
    }

    if !gitgraph.arrows.is_empty() {
        svg.push_str("<g class=\"commit-arrows\">");
        for arrow in &gitgraph.arrows {
            let color_idx = arrow.color_index % theme.git_colors.len();
            let stroke = theme.git_colors[color_idx].as_str();
            svg.push_str(&format!(
                "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-linecap=\"round\"/>",
                escape_xml(&arrow.path),
                escape_xml(stroke),
                gg.arrow_stroke_width
            ));
        }
        svg.push_str("</g>");
    }

    svg.push_str("<g class=\"commit-bullets\">");
    for commit in &gitgraph.commits {
        let color_idx = commit.branch_index % theme.git_colors.len();
        let color = theme.git_colors[color_idx].as_str();
        let highlight_color = theme.git_inv_colors[color_idx].as_str();
        let commit_symbol_type = commit.custom_type.unwrap_or(commit.commit_type);
        match commit_symbol_type {
            crate::ir::GitGraphCommitType::Highlight => {
                let outer_size = gg.highlight_outer_size;
                let inner_size = gg.highlight_inner_size;
                svg.push_str(&format!(
                    "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"{}\"/>",
                    commit.x - outer_size / 2.0,
                    commit.y - outer_size / 2.0,
                    outer_size,
                    outer_size,
                    escape_xml(highlight_color),
                    escape_xml(highlight_color)
                ));
                svg.push_str(&format!(
                    "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"{}\"/>",
                    commit.x - inner_size / 2.0,
                    commit.y - inner_size / 2.0,
                    inner_size,
                    inner_size,
                    escape_xml(&theme.primary_color),
                    escape_xml(&theme.primary_color)
                ));
            }
            crate::ir::GitGraphCommitType::CherryPick => {
                svg.push_str(&format!(
                    "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\"/>",
                    commit.x,
                    commit.y,
                    gg.commit_radius,
                    escape_xml(color),
                    escape_xml(color)
                ));
                let accent = escape_xml(&gg.cherry_pick_accent_color);
                svg.push_str(&format!(
                    "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"none\"/>",
                    commit.x - gg.cherry_pick_dot_offset_x,
                    commit.y + gg.cherry_pick_dot_offset_y,
                    gg.cherry_pick_dot_radius,
                    accent
                ));
                svg.push_str(&format!(
                    "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"none\"/>",
                    commit.x + gg.cherry_pick_dot_offset_x,
                    commit.y + gg.cherry_pick_dot_offset_y,
                    gg.cherry_pick_dot_radius,
                    accent
                ));
                svg.push_str(&format!(
                    "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{}\"/>",
                    commit.x + gg.cherry_pick_dot_offset_x,
                    commit.y + gg.cherry_pick_stem_start_offset_y,
                    commit.x,
                    commit.y + gg.cherry_pick_stem_end_offset_y,
                    accent,
                    gg.cherry_pick_stem_stroke_width
                ));
                svg.push_str(&format!(
                    "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{}\"/>",
                    commit.x - gg.cherry_pick_dot_offset_x,
                    commit.y + gg.cherry_pick_stem_start_offset_y,
                    commit.x,
                    commit.y + gg.cherry_pick_stem_end_offset_y,
                    accent,
                    gg.cherry_pick_stem_stroke_width
                ));
            }
            _ => {
                let radius = if commit.commit_type == crate::ir::GitGraphCommitType::Merge {
                    gg.merge_radius_outer
                } else {
                    gg.commit_radius
                };
                svg.push_str(&format!(
                    "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\"/>",
                    commit.x,
                    commit.y,
                    radius,
                    escape_xml(color),
                    escape_xml(color)
                ));
                if commit_symbol_type == crate::ir::GitGraphCommitType::Merge {
                    svg.push_str(&format!(
                        "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\"/>",
                        commit.x,
                        commit.y,
                        gg.merge_radius_inner,
                        escape_xml(&theme.primary_color),
                        escape_xml(&theme.primary_color)
                    ));
                }
                if commit_symbol_type == crate::ir::GitGraphCommitType::Reverse {
                    let size = gg.reverse_cross_size;
                    svg.push_str(&format!(
                        "<path d=\"M {x1:.2},{y1:.2} L {x2:.2},{y2:.2} M {x3:.2},{y3:.2} L {x4:.2},{y4:.2}\" stroke=\"{}\" stroke-width=\"{}\" fill=\"none\"/>",
                        escape_xml(&theme.primary_color),
                        gg.reverse_stroke_width,
                        x1 = commit.x - size,
                        y1 = commit.y - size,
                        x2 = commit.x + size,
                        y2 = commit.y + size,
                        x3 = commit.x - size,
                        y3 = commit.y + size,
                        x4 = commit.x + size,
                        y4 = commit.y - size,
                    ));
                }
            }
        }
    }
    svg.push_str("</g>");

    svg.push_str("<g class=\"commit-labels\">");
    for commit in &gitgraph.commits {
        if let Some(label) = &commit.label {
            let mut inner = String::new();
            inner.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" opacity=\"{}\"/>",
                label.bg_x,
                label.bg_y,
                label.bg_width,
                label.bg_height,
                escape_xml(&theme.git_commit_label_background),
                gg.commit_label_bg_opacity
            ));
            inner.push_str(&format!(
                "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"start\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                label.text_x,
                label.text_y,
                normalize_font_family(&theme.font_family),
                gg.commit_label_font_size,
                escape_xml(&theme.git_commit_label_color),
                escape_xml(&label.text)
            ));
            if let Some(transform) = &label.transform {
                svg.push_str(&format!(
                    "<g transform=\"translate({:.2}, {:.2}) rotate({:.2}, {:.2}, {:.2})\">{}</g>",
                    transform.translate_x,
                    transform.translate_y,
                    transform.rotate_deg,
                    transform.rotate_cx,
                    transform.rotate_cy,
                    inner
                ));
            } else {
                svg.push_str(&inner);
            }
        }

        if !commit.tags.is_empty() {
            for tag in &commit.tags {
                let points = tag
                    .points
                    .iter()
                    .map(|(x, y)| format!("{:.2},{:.2}", x, y))
                    .collect::<Vec<_>>()
                    .join(" ");
                let mut tag_inner = String::new();
                tag_inner.push_str(&format!(
                    "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\"/>",
                    points,
                    escape_xml(&theme.git_tag_label_background),
                    escape_xml(&theme.git_tag_label_border)
                ));
                tag_inner.push_str(&format!(
                    "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\"/>",
                    tag.hole_x,
                    tag.hole_y,
                    gg.tag_hole_radius,
                    escape_xml(&theme.text_color)
                ));
                tag_inner.push_str(&format!(
                    "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"start\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                    tag.text_x,
                    tag.text_y,
                    normalize_font_family(&theme.font_family),
                    gg.tag_label_font_size,
                    escape_xml(&theme.git_tag_label_color),
                    escape_xml(&tag.text)
                ));
                if let Some(transform) = &tag.transform {
                    svg.push_str(&format!(
                        "<g transform=\"translate({:.2}, {:.2}) rotate({:.2}, {:.2}, {:.2})\">{}</g>",
                        transform.translate_x,
                        transform.translate_y,
                        transform.rotate_deg,
                        transform.rotate_cx,
                        transform.rotate_cy,
                        tag_inner
                    ));
                } else {
                    svg.push_str(&tag_inner);
                }
            }
        }
    }
    svg.push_str("</g>");

    svg.push_str("</g>");
    svg
}

fn render_gitgraph_multiline_text(
    x: f32,
    y: f32,
    text: &str,
    font_family: &str,
    font_size: f32,
    line_height: f32,
    color: &str,
) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.is_empty() {
        return String::new();
    }
    let start_y = y + font_size;
    let mut out = String::new();
    out.push_str(&format!(
        "<text x=\"{x:.2}\" y=\"{start_y:.2}\" text-anchor=\"start\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">",
        normalize_font_family(font_family),
        font_size,
        escape_xml(color)
    ));
    let line_height_px = font_size * line_height;
    for (idx, line) in lines.iter().enumerate() {
        let dy = if idx == 0 { 0.0 } else { line_height_px };
        out.push_str(&format!(
            "<tspan x=\"{x:.2}\" dy=\"{dy:.2}\">{}</tspan>",
            escape_xml(line)
        ));
    }
    out.push_str("</text>");
    out
}

fn text_block_svg(
    x: f32,
    y: f32,
    label: &TextBlock,
    theme: &Theme,
    config: &LayoutConfig,
    _edge: bool,
    override_color: Option<&str>,
) -> String {
    text_block_svg_with_font_size(
        x,
        y,
        label,
        theme,
        config,
        theme.font_size,
        "middle",
        override_color,
        false,
    )
}

fn text_block_svg_anchor(
    x: f32,
    y: f32,
    label: &TextBlock,
    theme: &Theme,
    config: &LayoutConfig,
    anchor: &str,
    override_color: Option<&str>,
) -> String {
    text_block_svg_with_font_size(
        x,
        y,
        label,
        theme,
        config,
        theme.font_size,
        anchor,
        override_color,
        false,
    )
}

fn sequence_text_block_svg(
    x: f32,
    y: f32,
    label: &TextBlock,
    theme: &Theme,
    _edge: bool,
    override_color: Option<&str>,
) -> String {
    text_block_svg_with_font_size_and_line_height(
        x,
        y,
        label,
        theme,
        theme.font_size,
        "middle",
        override_color,
        false,
        SEQUENCE_TEXT_LINE_HEIGHT,
    )
}

fn sequence_text_block_svg_anchor(
    x: f32,
    y: f32,
    label: &TextBlock,
    theme: &Theme,
    anchor: &str,
    override_color: Option<&str>,
) -> String {
    text_block_svg_with_font_size_and_line_height(
        x,
        y,
        label,
        theme,
        theme.font_size,
        anchor,
        override_color,
        false,
        SEQUENCE_TEXT_LINE_HEIGHT,
    )
}

fn text_block_svg_with_font_size(
    x: f32,
    y: f32,
    label: &TextBlock,
    theme: &Theme,
    config: &LayoutConfig,
    font_size: f32,
    anchor: &str,
    override_color: Option<&str>,
    baseline: bool,
) -> String {
    text_block_svg_with_font_size_and_line_height(
        x,
        y,
        label,
        theme,
        font_size,
        anchor,
        override_color,
        baseline,
        config.label_line_height,
    )
}

fn text_block_svg_with_font_size_and_line_height(
    x: f32,
    y: f32,
    label: &TextBlock,
    theme: &Theme,
    font_size: f32,
    anchor: &str,
    override_color: Option<&str>,
    baseline: bool,
    line_height_factor: f32,
) -> String {
    let total_height = label.lines.len() as f32 * font_size * line_height_factor;
    let start_y = if baseline {
        y
    } else {
        y - total_height / 2.0 + font_size
    };
    let mut text = String::new();
    let default_fill = theme.primary_text_color.as_str();
    let fill = override_color.unwrap_or(default_fill);

    text.push_str(&format!(
        "<text x=\"{x:.2}\" y=\"{start_y:.2}\" text-anchor=\"{anchor}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">",
        normalize_font_family(&theme.font_family),
        font_size,
        fill
    ));

    let line_height = font_size * line_height_factor;
    for (idx, line) in label.lines.iter().enumerate() {
        let dy = if idx == 0 { 0.0 } else { line_height };
        let line_text = line.text();
        if is_divider_line(&line_text) {
            text.push_str(&format!("<tspan x=\"{x:.2}\" dy=\"{dy:.2}\"></tspan>",));
        } else if line.has_formatting() {
            render_formatted_tspans(&mut text, x, dy, line, true);
        } else {
            text.push_str(&format!(
                "<tspan x=\"{x:.2}\" dy=\"{dy:.2}\">{}</tspan>",
                escape_xml(&line_text)
            ));
        }
    }

    text.push_str("</text>");
    text
}

fn text_block_svg_with_font_size_weight(
    x: f32,
    y: f32,
    label: &TextBlock,
    theme: &Theme,
    config: &LayoutConfig,
    font_size: f32,
    anchor: &str,
    override_color: Option<&str>,
    font_weight: Option<&str>,
    baseline: bool,
) -> String {
    text_block_svg_with_font_attrs(
        x,
        y,
        label,
        theme,
        config,
        font_size,
        anchor,
        override_color,
        font_weight,
        None,
        baseline,
    )
}

fn text_block_svg_with_font_attrs(
    x: f32,
    y: f32,
    label: &TextBlock,
    theme: &Theme,
    config: &LayoutConfig,
    font_size: f32,
    anchor: &str,
    override_color: Option<&str>,
    font_weight: Option<&str>,
    font_style: Option<&str>,
    baseline: bool,
) -> String {
    let total_height = label.lines.len() as f32 * font_size * config.label_line_height;
    let start_y = if baseline {
        y
    } else {
        y - total_height / 2.0 + font_size
    };
    let mut text = String::new();
    let default_fill = theme.primary_text_color.as_str();
    let fill = override_color.unwrap_or(default_fill);
    let weight_attr = font_weight
        .filter(|w| !w.trim().is_empty())
        .map(|w| format!(" font-weight=\"{}\"", w))
        .unwrap_or_default();
    let style_attr = font_style
        .filter(|s| !s.trim().is_empty())
        .map(|s| format!(" font-style=\"{}\"", s))
        .unwrap_or_default();

    text.push_str(&format!(
        "<text x=\"{x:.2}\" y=\"{start_y:.2}\" text-anchor=\"{anchor}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\"{weight_attr}{style_attr}>",
        normalize_font_family(&theme.font_family),
        font_size,
        fill
    ));

    let line_height = font_size * config.label_line_height;
    for (idx, line) in label.lines.iter().enumerate() {
        let dy = if idx == 0 { 0.0 } else { line_height };
        let line_text = line.text();
        if is_divider_line(&line_text) {
            text.push_str(&format!("<tspan x=\"{x:.2}\" dy=\"{dy:.2}\"></tspan>",));
        } else if line.has_formatting() {
            render_formatted_tspans(&mut text, x, dy, line, true);
        } else {
            text.push_str(&format!(
                "<tspan x=\"{x:.2}\" dy=\"{dy:.2}\">{}</tspan>",
                escape_xml(&line_text)
            ));
        }
    }

    text.push_str("</text>");
    text
}

fn text_line_svg_with_font_size(
    x: f32,
    y: f32,
    text: &str,
    theme: &Theme,
    font_size: f32,
    fill: &str,
    anchor: &str,
) -> String {
    format!(
        "<text x=\"{x:.2}\" y=\"{y:.2}\" text-anchor=\"{anchor}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
        normalize_font_family(&theme.font_family),
        font_size,
        fill,
        escape_xml(text)
    )
}

fn text_line_svg(x: f32, y: f32, text: &str, theme: &Theme, fill: &str, anchor: &str) -> String {
    format!(
        "<text x=\"{x:.2}\" y=\"{y:.2}\" text-anchor=\"{anchor}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
        normalize_font_family(&theme.font_family),
        theme.font_size,
        fill,
        escape_xml(text)
    )
}

/// Emit `<tspan>` elements for a formatted `TextLine`. The first span gets
/// `x` + `dy` positioning; subsequent spans flow inline (no `x` reset).
fn render_formatted_tspans(
    out: &mut String,
    x: f32,
    dy: f32,
    line: &crate::layout::TextLine,
    set_x: bool,
) {
    for (i, span) in line.spans.iter().enumerate() {
        let mut attrs = String::new();
        if i == 0 {
            if set_x {
                attrs.push_str(&format!(" x=\"{x:.2}\""));
            }
            attrs.push_str(&format!(" dy=\"{dy:.2}\""));
        }
        if span.style.bold {
            attrs.push_str(" font-weight=\"bold\"");
        }
        if span.style.italic {
            attrs.push_str(" font-style=\"italic\"");
        }
        out.push_str(&format!(
            "<tspan{}>{}</tspan>",
            attrs,
            escape_xml(&span.text)
        ));
    }
}

const C4_PERSON_ICON: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADAAAAAwCAIAAADYYG7QAAACD0lEQVR4Xu2YoU4EMRCGT+4j8Ai8AhaH4QHgAUjQuFMECUgMIUgwJAgMhgQsAYUiJCiQIBBY+EITsjfTdme6V24v4c8vyGbb+ZjOtN0bNcvjQXmkH83WvYBWto6PLm6v7p7uH1/w2fXD+PBycX1Pv2l3IdDm/vn7x+dXQiAubRzoURa7gRZWd0iGRIiJbOnhnfYBQZNJjNbuyY2eJG8fkDE3bbG4ep6MHUAsgYxmE3nVs6VsBWJSGccsOlFPmLIViMzLOB7pCVO2AtHJMohH7Fh6zqitQK7m0rJvAVYgGcEpe//PLdDz65sM4pF9N7ICcXDKIB5Nv6j7tD0NoSdM2QrU9Gg0ewE1LqBhHR3BBdvj2vapnidjHxD/q6vd7Pvhr31AwcY8eXMTXAKECZZJFXuEq27aLgQK5uLMohCenGGuGewOxSjBvYBqeG6B+Nqiblggdjnc+ZXDy+FNFpFzw76O3UBAROuXh6FoiAcf5g9eTvUgzy0nWg6I8cXHRUpg5bOVBCo+KDpFajOf23GgPme7RSQ+lacIENUgJ6gg1k6HjgOlqnLqip4tEuhv0hNEMXUD0clyXE3p6pZA0S2nnvTlXwLJEZWlb7cTQH1+USgTN4VhAenm/wea1OCAOmqo6fE1WCb9WSKBah+rbUWPWAmE2Rvk0ApiB45eOyNAzU8xcTvj8KvkKEoOaIYeHNA3ZuygAvFMUO0AAAAASUVORK5CYII=";
const C4_EXTERNAL_PERSON_ICON: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADAAAAAwCAIAAADYYG7QAAAB6ElEQVR4Xu2YLY+EMBCG9+dWr0aj0Wg0Go1Go0+j8Xdv2uTCvv1gpt0ebHKPuhDaeW4605Z9mJvx4AdXUyTUdd08z+u6flmWZRnHsWkafk9DptAwDPu+f0eAYtu2PEaGWuj5fCIZrBAC2eLBAnRCsEkkxmeaJp7iDJ2QMDdHsLg8SxKFEJaAo8lAXnmuOFIhTMpxxKATebo4UiFknuNo4OniSIXQyRxEA3YsnjGCVEjVXD7yLUAqxBGUyPv/Y4W2beMgGuS7kVQIBycH0fD+oi5pezQETxdHKmQKGk1eQEYldK+jw5GxPfZ9z7Mk0Qnhf1W1m3w//EUn5BDmSZsbR44QQLBEqrBHqOrmSKaQAxdnLArCrxZcM7A7ZKs4ioRq8LFC+NpC3WCBJsvpVw5edm9iEXFuyNfxXAgSwfrFQ1c0iNda8AdejvUgnktOtJQQxmcfFzGglc5WVCj7oDgFqU18boeFSs52CUh8LE8BIVQDT1ABrB0HtgSEYlX5doJnCwv9TXocKCaKbnwhdDKPq4lf3SwU3HLq4V/+WYhHVMa/3b4IlfyikAduCkcBc7mQ3/z/Qq/cTuikhkzB12Ae/mcJC9U+Vo8Ej1gWAtgbeGgFsAMHr50BIWOLCbezvhpBFUdY6EJuJ/QDW0XoMX60zZ0AAAAASUVORK5CYII=";

fn render_c4(c4: &C4Layout, config: &LayoutConfig) -> String {
    let conf = &config.c4;
    let mut svg = String::new();

    svg.push_str("<defs><symbol id=\"computer\" width=\"24\" height=\"24\"><path transform=\"scale(.5)\" d=\"M2 2v13h20v-13h-20zm18 11h-16v-9h16v9zm-10.228 6l.466-1h3.524l.467 1h-4.457zm14.228 3h-24l2-6h2.104l-1.33 4h18.45l-1.297-4h2.073l2 6zm-5-10h-14v-7h14v7z\"/></symbol></defs>");
    svg.push_str("<defs><symbol id=\"database\" fill-rule=\"evenodd\" clip-rule=\"evenodd\"><path transform=\"scale(.5)\" d=\"M12.258.001l.256.004.255.005.253.008.251.01.249.012.247.015.246.016.242.019.241.02.239.023.236.024.233.027.231.028.229.031.225.032.223.034.22.036.217.038.214.04.211.041.208.043.205.045.201.046.198.048.194.05.191.051.187.053.183.054.18.056.175.057.172.059.168.06.163.061.16.063.155.064.15.066.074.033.073.033.071.034.07.034.069.035.068.035.067.035.066.035.064.036.064.036.062.036.06.036.06.037.058.037.058.037.055.038.055.038.053.038.052.038.051.039.05.039.048.039.047.039.045.04.044.04.043.04.041.04.04.041.039.041.037.041.036.041.034.041.033.042.032.042.03.042.029.042.027.042.026.043.024.043.023.043.021.043.02.043.018.044.017.043.015.044.013.044.012.044.011.045.009.044.007.045.006.045.004.045.002.045.001.045v17l-.001.045-.002.045-.004.045-.006.045-.007.045-.009.044-.011.045-.012.044-.013.044-.015.044-.017.043-.018.044-.02.043-.021.043-.023.043-.024.043-.026.043-.027.042-.029.042-.03.042-.032.042-.033.042-.034.041-.036.041-.037.041-.039.041-.04.041-.041.04-.043.04-.044.04-.045.04-.047.039-.048.039-.05.039-.051.039-.052.038-.053.038-.055.038-.055.038-.058.037-.058.037-.06.037-.06.036-.062.036-.064.036-.064.036-.066.035-.067.035-.068.035-.069.035-.07.034-.071.034-.073.033-.074.033-.15.066-.155.064-.16.063-.163.061-.168.06-.172.059-.175.057-.18.056-.183.054-.187.053-.191.051-.194.05-.198.048-.201.046-.205.045-.208.043-.211.041-.214.04-.217.038-.22.036-.223.034-.225.032-.229.031-.231.028-.233.027-.236.024-.239.023-.241.02-.242.019-.246.016-.247.015-.249.012-.251.01-.253.008-.255.005-.256.004-.258.001-.258-.001-.256-.004-.255-.005-.253-.008-.251-.01-.249-.012-.247-.015-.245-.016-.243-.019-.241-.02-.238-.023-.236-.024-.234-.027-.231-.028-.228-.031-.226-.032-.223-.034-.22-.036-.217-.038-.214-.04-.211-.041-.208-.043-.204-.045-.201-.046-.198-.048-.195-.05-.19-.051-.187-.053-.184-.054-.179-.056-.176-.057-.172-.059-.167-.06-.164-.061-.159-.063-.155-.064-.151-.066-.074-.033-.072-.033-.072-.034-.07-.034-.069-.035-.068-.035-.067-.035-.066-.035-.064-.036-.063-.036-.062-.036-.061-.036-.06-.037-.058-.037-.057-.037-.056-.038-.055-.038-.053-.038-.052-.038-.051-.039-.049-.039-.049-.039-.046-.039-.046-.04-.044-.04-.043-.04-.041-.04-.04-.041-.039-.041-.037-.041-.036-.041-.034-.041-.033-.042-.032-.042-.03-.042-.029-.042-.027-.042-.026-.043-.024-.043-.023-.043-.021-.043-.02-.043-.018-.044-.017-.043-.015-.044-.013-.044-.012-.044-.011-.045-.009-.044-.007-.045-.006-.045-.004-.045-.002-.045-.001-.045v-17l.001-.045.002-.045.004-.045.006-.045.007-.045.009-.044.011-.045.012-.044.013-.044.015-.044.017-.043.018-.044.02-.043.021-.043.023-.043.024-.043.026-.043.027-.042.029-.042.03-.042.032-.042.033-.042.034-.041.036-.041.037-.041.039-.041.04-.041.041-.04.043-.04.044-.04.046-.04.046-.039.049-.039.049-.039.051-.039.052-.038.053-.038.055-.038.056-.038.057-.037.058-.037.06-.037.061-.036.062-.036.063-.036.064-.036.066-.035.067-.035.068-.035.069-.035.07-.034.072-.034.072-.033.074-.033.151-.066.155-.064.159-.063.164-.061.167-.06.172-.059.176-.057.179-.056.184-.054.187-.053.19-.051.195-.05.198-.048.201-.046.204-.045.208-.043.211-.041.214-.04.217-.038.22-.036.223-.034.226-.032.228-.031.231-.028.234-.027.236-.024.238-.023.241-.02.243-.019.245-.016.247-.015.249-.012.251-.01.253-.008.255-.005.256-.004.258-.001.258.001z\"/></symbol></defs>");
    svg.push_str("<defs><symbol id=\"clock\" width=\"24\" height=\"24\"><path transform=\"scale(.5)\" d=\"M12 2c5.514 0 10 4.486 10 10s-4.486 10-10 10-10-4.486-10-10 4.486-10 10-10zm0-2c-6.627 0-12 5.373-12 12s5.373 12 12 12 12-5.373 12-12-5.373-12-12-12zm5.848 12.459c.202.038.202.333.001.372-1.907.361-6.045 1.111-6.547 1.111-.719 0-1.301-.582-1.301-1.301 0-.512.77-5.447 1.125-7.445.034-.192.312-.181.343.014l.985 6.238 5.394 1.011z\"/></symbol></defs>");

    for shape in &c4.shapes {
        svg.push_str(&render_c4_shape(shape, conf));
    }

    for boundary in &c4.boundaries {
        svg.push_str(&render_c4_boundary(boundary, conf));
    }

    svg.push_str("<defs><marker id=\"arrowhead\" refX=\"9\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"12\" markerHeight=\"12\" orient=\"auto\"><path d=\"M 0 0 L 10 5 L 0 10 z\"/></marker></defs>");
    svg.push_str("<defs><marker id=\"arrowend\" refX=\"1\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"12\" markerHeight=\"12\" orient=\"auto\"><path d=\"M 10 0 L 0 5 L 10 10 z\"/></marker></defs>");
    svg.push_str("<defs><marker id=\"crosshead\" markerWidth=\"15\" markerHeight=\"8\" orient=\"auto\" refX=\"16\" refY=\"4\"><path fill=\"black\" stroke=\"#000000\" stroke-width=\"1px\" d=\"M 9,2 V 6 L16,4 Z\" style=\"stroke-dasharray: 0, 0;\"/><path fill=\"none\" stroke=\"#000000\" stroke-width=\"1px\" d=\"M 0,1 L 6,7 M 6,1 L 0,7\" style=\"stroke-dasharray: 0, 0;\"/></marker></defs>");
    svg.push_str("<defs><marker id=\"filled-head\" refX=\"18\" refY=\"7\" markerWidth=\"20\" markerHeight=\"28\" orient=\"auto\"><path d=\"M 18,7 L9,13 L14,7 L9,1 Z\"/></marker></defs>");

    svg.push_str("<g>");
    for (idx, rel) in c4.rels.iter().enumerate() {
        svg.push_str(&render_c4_rel(rel, conf, idx == 0));
    }
    svg.push_str("</g>");

    if let Some(title) = &c4.title {
        let box_width = c4.viewbox_width - 2.0 * conf.diagram_margin_x;
        let title_x = box_width / 2.0 - 4.0 * conf.diagram_margin_x;
        let title_y = 2.0 * conf.diagram_margin_y;
        svg.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.0}\">{}</text>",
            title_x,
            title_y,
            escape_xml(title)
        ));
    }

    svg
}

fn render_c4_shape(shape: &C4ShapeLayout, conf: &crate::config::C4Config) -> String {
    let (default_fill, default_stroke) = c4_shape_colors(conf, shape.kind);
    let fill = shape.bg_color.as_deref().unwrap_or(default_fill);
    let stroke = shape.border_color.as_deref().unwrap_or(default_stroke);
    let font_color = shape.font_color.as_deref().unwrap_or("#FFFFFF");
    let fill = escape_xml(fill);
    let stroke = escape_xml(stroke);
    let font_color = escape_xml(font_color);
    let mut svg = String::new();
    svg.push_str("<g class=\"person-man\">");
    match shape.kind {
        crate::ir::C4ShapeKind::SystemDb
        | crate::ir::C4ShapeKind::ExternalSystemDb
        | crate::ir::C4ShapeKind::ContainerDb
        | crate::ir::C4ShapeKind::ExternalContainerDb
        | crate::ir::C4ShapeKind::ComponentDb
        | crate::ir::C4ShapeKind::ExternalComponentDb => {
            let half = shape.width / 2.0;
            let ellipse = conf.db_ellipse_height;
            svg.push_str(&format!(
                "<path fill=\"{}\" stroke-width=\"{}\" stroke=\"{}\" d=\"M{:.0},{:.0}c0,-{ellipse} {half:.0},-{ellipse} {half:.0},-{ellipse}c0,0 {half:.0},0 {half:.0},{ellipse}l0,{:.0}c0,{ellipse} -{half:.0},{ellipse} -{half:.0},{ellipse}c0,0 -{half:.0},0 -{half:.0},-{ellipse}l0,-{:.0}\"/>",
                fill,
                conf.shape_stroke_width,
                stroke,
                shape.x,
                shape.y,
                shape.height,
                shape.height
            ));
            svg.push_str(&format!(
                "<path fill=\"none\" stroke-width=\"{}\" stroke=\"{}\" d=\"M{:.0},{:.0}c0,{ellipse} {half:.0},{ellipse} {half:.0},{ellipse}c0,0 {half:.0},0 {half:.0},-{ellipse}\"/>",
                conf.shape_stroke_width,
                stroke,
                shape.x,
                shape.y,
            ));
        }
        crate::ir::C4ShapeKind::SystemQueue
        | crate::ir::C4ShapeKind::ExternalSystemQueue
        | crate::ir::C4ShapeKind::ContainerQueue
        | crate::ir::C4ShapeKind::ExternalContainerQueue
        | crate::ir::C4ShapeKind::ComponentQueue
        | crate::ir::C4ShapeKind::ExternalComponentQueue => {
            let half = shape.height / 2.0;
            let curve = conf.queue_curve_radius;
            svg.push_str(&format!(
                "<path fill=\"{}\" stroke-width=\"{}\" stroke=\"{}\" d=\"M{:.0},{:.0}l{:.0},0c{curve},0 {curve},{half} {curve},{half}c0,0 0,{half} -{curve},{half}l-{:.0},0c-{curve},0 -{curve},-{half} -{curve},-{half}c0,0 0,-{half} {curve},-{half}\"/>",
                fill,
                conf.shape_stroke_width,
                stroke,
                shape.x,
                shape.y,
                shape.width,
                shape.width
            ));
            svg.push_str(&format!(
                "<path fill=\"none\" stroke-width=\"{}\" stroke=\"{}\" d=\"M{:.0},{:.0}c-{curve},0 -{curve},{half} -{curve},{half}c0,{half} {curve},{half} {curve},{half}\"/>",
                conf.shape_stroke_width,
                stroke,
                shape.x + shape.width,
                shape.y,
            ));
        }
        _ => {
            svg.push_str(&format!(
                "<rect x=\"{:.0}\" y=\"{:.0}\" fill=\"{}\" stroke=\"{}\" width=\"{:.0}\" height=\"{:.0}\" rx=\"{:.1}\" ry=\"{:.1}\" stroke-width=\"{}\"/>",
                shape.x,
                shape.y,
                fill,
                stroke,
                shape.width,
                shape.height,
                conf.shape_corner_radius,
                conf.shape_corner_radius,
                conf.shape_stroke_width
            ));
        }
    }

    let type_font_size = c4_shape_font_size(conf, shape.kind) - 2.0;
    let type_font_family = c4_shape_font_family(conf, shape.kind);
    svg.push_str(&format!(
        "<text fill=\"{}\" font-family=\"{}\" font-size=\"{}\" font-style=\"italic\" lengthAdjust=\"spacing\" textLength=\"{:.0}\" x=\"{:.0}\" y=\"{:.0}\">{}</text>",
        font_color,
        normalize_font_family(type_font_family),
        type_font_size,
        shape.type_label.width.round(),
        shape.x + shape.width / 2.0 - shape.type_label.width / 2.0,
        shape.y + shape.type_label.y,
        escape_xml(&shape.type_label.text)
    ));

    if let Some(image_y) = shape.image_y
        && matches!(
            shape.kind,
            crate::ir::C4ShapeKind::Person | crate::ir::C4ShapeKind::ExternalPerson
        )
    {
        let icon = match shape.kind {
            crate::ir::C4ShapeKind::ExternalPerson => C4_EXTERNAL_PERSON_ICON,
            crate::ir::C4ShapeKind::Person => C4_PERSON_ICON,
            _ => C4_PERSON_ICON,
        };
        svg.push_str(&format!(
            "<image width=\"{:.0}\" height=\"{:.0}\" x=\"{:.0}\" y=\"{:.0}\" xlink:href=\"{}\"/>",
            conf.person_icon_size,
            conf.person_icon_size,
            shape.x + shape.width / 2.0 - conf.person_icon_size / 2.0,
            shape.y + image_y,
            icon
        ));
    }

    let label_font_size = c4_shape_font_size(conf, shape.kind) + 2.0;
    let label_font_family = c4_shape_font_family(conf, shape.kind);
    let label_font_weight = "bold";
    svg.push_str(&c4_text_svg(
        shape.x + shape.width / 2.0,
        shape.y + shape.label.y,
        &shape.label.lines,
        label_font_family,
        label_font_size,
        label_font_weight,
        &font_color,
        false,
    ));

    if let Some(type_or_techn) = &shape.type_or_techn {
        let font_family = c4_shape_font_family(conf, shape.kind);
        let font_weight = c4_shape_font_weight(conf, shape.kind);
        let font_size = c4_shape_font_size(conf, shape.kind);
        svg.push_str(&c4_text_svg(
            shape.x + shape.width / 2.0,
            shape.y + type_or_techn.y,
            &type_or_techn.lines,
            font_family,
            font_size,
            font_weight,
            &font_color,
            true,
        ));
    }

    if let Some(descr) = &shape.descr {
        let font_family = c4_shape_font_family(conf, shape.kind);
        let font_weight = c4_shape_font_weight(conf, shape.kind);
        let font_size = c4_shape_font_size(conf, shape.kind);
        svg.push_str(&c4_text_svg(
            shape.x + shape.width / 2.0,
            shape.y + descr.y,
            &descr.lines,
            font_family,
            font_size,
            font_weight,
            &font_color,
            false,
        ));
    }

    svg.push_str("</g>");
    svg
}

fn render_c4_boundary(boundary: &C4BoundaryLayout, conf: &crate::config::C4Config) -> String {
    let mut svg = String::new();
    svg.push_str("<g>");
    let fill = boundary.bg_color.as_deref().unwrap_or(&conf.boundary_fill);
    let stroke = boundary
        .border_color
        .as_deref()
        .unwrap_or(&conf.boundary_stroke);
    let font_color = boundary
        .font_color
        .as_deref()
        .unwrap_or(&conf.boundary_stroke);
    let fill_attr = escape_xml(fill);
    let stroke_attr = escape_xml(stroke);
    let font_color_attr = escape_xml(font_color);
    let mut rect_attrs = format!(
        "<rect x=\"{:.0}\" y=\"{:.0}\" fill=\"{}\" stroke=\"{}\" width=\"{:.0}\" height=\"{:.0}\" rx=\"{:.1}\" ry=\"{:.1}\" stroke-width=\"{}\"",
        boundary.x,
        boundary.y,
        fill_attr,
        stroke_attr,
        boundary.width,
        boundary.height,
        conf.boundary_corner_radius,
        conf.boundary_corner_radius,
        conf.boundary_stroke_width
    );
    if !conf.boundary_dasharray.is_empty() {
        rect_attrs.push_str(&format!(
            " stroke-dasharray=\"{}\"",
            escape_xml(&conf.boundary_dasharray)
        ));
    }
    if conf.boundary_fill != "none" && conf.boundary_fill_opacity < 1.0 {
        rect_attrs.push_str(&format!(
            " fill-opacity=\"{:.2}\"",
            conf.boundary_fill_opacity
        ));
    }
    rect_attrs.push_str("/>");
    svg.push_str(&rect_attrs);

    let label_font_size = conf.boundary_font_size + 2.0;
    svg.push_str(&c4_text_svg(
        boundary.x + boundary.width / 2.0,
        boundary.y + boundary.label.y,
        &boundary.label.lines,
        &conf.boundary_font_family,
        label_font_size,
        "bold",
        &font_color_attr,
        false,
    ));

    if let Some(boundary_type) = &boundary.boundary_type {
        svg.push_str(&c4_text_svg(
            boundary.x + boundary.width / 2.0,
            boundary.y + boundary_type.y,
            &boundary_type.lines,
            &conf.boundary_font_family,
            conf.boundary_font_size,
            &conf.boundary_font_weight,
            &font_color_attr,
            false,
        ));
    }

    if let Some(descr) = &boundary.descr {
        svg.push_str(&c4_text_svg(
            boundary.x + boundary.width / 2.0,
            boundary.y + descr.y,
            &descr.lines,
            &conf.boundary_font_family,
            conf.boundary_font_size - 2.0,
            &conf.boundary_font_weight,
            &font_color_attr,
            false,
        ));
    }

    svg.push_str("</g>");
    svg
}

fn render_c4_rel(rel: &C4RelLayout, conf: &crate::config::C4Config, straight: bool) -> String {
    let mut svg = String::new();
    let stroke = rel.line_color.as_deref().unwrap_or(&conf.boundary_stroke);
    if straight {
        let mut attrs = String::new();
        if rel.kind != crate::ir::C4RelKind::RelBack {
            attrs.push_str(" marker-end=\"url(#arrowhead)\"");
        }
        if matches!(
            rel.kind,
            crate::ir::C4RelKind::BiRel | crate::ir::C4RelKind::RelBack
        ) {
            attrs.push_str(" marker-start=\"url(#arrowend)\"");
        }
        svg.push_str(&format!(
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke-width=\"1\" stroke=\"{}\" style=\"fill: none;\"{attrs} />",
            rel.start.0,
            rel.start.1,
            rel.end.0,
            rel.end.1,
            escape_xml(stroke),
        ));
    } else {
        let control_x = rel.start.0 + (rel.end.0 - rel.start.0) / 4.0;
        let control_y = rel.start.1 + (rel.end.1 - rel.start.1) / 2.0;
        let mut path = format!(
            "<path fill=\"none\" stroke-width=\"1\" stroke=\"{}\" d=\"M{:.2},{:.2} Q{:.2},{:.2} {:.2},{:.2}\"",
            escape_xml(stroke),
            rel.start.0,
            rel.start.1,
            control_x,
            control_y,
            rel.end.0,
            rel.end.1
        );
        if rel.kind != crate::ir::C4RelKind::RelBack {
            path.push_str(" marker-end=\"url(#arrowhead)\"");
        }
        if matches!(
            rel.kind,
            crate::ir::C4RelKind::BiRel | crate::ir::C4RelKind::RelBack
        ) {
            path.push_str(" marker-start=\"url(#arrowend)\"");
        }
        path.push_str("/>");
        svg.push_str(&path);
    }

    let text_color = rel.text_color.as_deref().unwrap_or(&conf.boundary_stroke);
    let mid_x = rel.start.0.min(rel.end.0) + (rel.start.0 - rel.end.0).abs() / 2.0 + rel.offset_x;
    let mid_y = rel.start.1.min(rel.end.1) + (rel.start.1 - rel.end.1).abs() / 2.0 + rel.offset_y;
    let label_x = mid_x + rel.label.width / 2.0;
    svg.push_str(&c4_text_svg(
        label_x,
        mid_y,
        &rel.label.lines,
        &conf.message_font_family,
        conf.message_font_size,
        &conf.message_font_weight,
        text_color,
        false,
    ));
    if let Some(techn) = &rel.techn {
        let techn_lines = c4_bracketed_lines(&techn.lines);
        let techn_x = mid_x + rel.label.width.max(techn.width) / 2.0;
        svg.push_str(&c4_text_svg(
            techn_x,
            mid_y + conf.message_font_size + 5.0,
            &techn_lines,
            &conf.message_font_family,
            conf.message_font_size,
            &conf.message_font_weight,
            text_color,
            true,
        ));
    }
    svg
}

fn c4_bracketed_lines(lines: &[String]) -> Vec<String> {
    match lines {
        [] => Vec::new(),
        [line] => vec![format!("[{line}]")],
        _ => {
            let mut bracketed = lines.to_vec();
            if let Some(first) = bracketed.first_mut() {
                first.insert(0, '[');
            }
            if let Some(last) = bracketed.last_mut() {
                last.push(']');
            }
            bracketed
        }
    }
}

fn c4_text_svg(
    x: f32,
    y: f32,
    lines: &[String],
    font_family: &str,
    font_size: f32,
    font_weight: &str,
    fill: &str,
    italic: bool,
) -> String {
    let mut out = String::new();
    let line_count = lines.len() as f32;
    for (idx, line) in lines.iter().enumerate() {
        let dy = idx as f32 * font_size - font_size * (line_count - 1.0) / 2.0;
        out.push_str(&format!(
            "<text x=\"{x:.2}\" y=\"{y:.2}\" dominant-baseline=\"middle\" fill=\"{}\" style=\"text-anchor: middle; font-size: {}px; font-weight: {}; font-family: {}\"{}><tspan dy=\"{dy:.2}\" alignment-baseline=\"mathematical\">{}</tspan></text>",
            escape_xml(fill),
            font_size,
            escape_xml(font_weight),
            normalize_font_family(font_family),
            if italic { " font-style=\"italic\"" } else { "" },
            escape_xml(line)
        ));
    }
    out
}

fn c4_shape_colors(conf: &crate::config::C4Config, kind: crate::ir::C4ShapeKind) -> (&str, &str) {
    match kind {
        crate::ir::C4ShapeKind::Person => (&conf.person_bg_color, &conf.person_border_color),
        crate::ir::C4ShapeKind::ExternalPerson => (
            &conf.external_person_bg_color,
            &conf.external_person_border_color,
        ),
        crate::ir::C4ShapeKind::System => (&conf.system_bg_color, &conf.system_border_color),
        crate::ir::C4ShapeKind::SystemDb => {
            (&conf.system_db_bg_color, &conf.system_db_border_color)
        }
        crate::ir::C4ShapeKind::SystemQueue => {
            (&conf.system_queue_bg_color, &conf.system_queue_border_color)
        }
        crate::ir::C4ShapeKind::ExternalSystem => (
            &conf.external_system_bg_color,
            &conf.external_system_border_color,
        ),
        crate::ir::C4ShapeKind::ExternalSystemDb => (
            &conf.external_system_db_bg_color,
            &conf.external_system_db_border_color,
        ),
        crate::ir::C4ShapeKind::ExternalSystemQueue => (
            &conf.external_system_queue_bg_color,
            &conf.external_system_queue_border_color,
        ),
        crate::ir::C4ShapeKind::Container => {
            (&conf.container_bg_color, &conf.container_border_color)
        }
        crate::ir::C4ShapeKind::ContainerDb => {
            (&conf.container_db_bg_color, &conf.container_db_border_color)
        }
        crate::ir::C4ShapeKind::ContainerQueue => (
            &conf.container_queue_bg_color,
            &conf.container_queue_border_color,
        ),
        crate::ir::C4ShapeKind::ExternalContainer => (
            &conf.external_container_bg_color,
            &conf.external_container_border_color,
        ),
        crate::ir::C4ShapeKind::ExternalContainerDb => (
            &conf.external_container_db_bg_color,
            &conf.external_container_db_border_color,
        ),
        crate::ir::C4ShapeKind::ExternalContainerQueue => (
            &conf.external_container_queue_bg_color,
            &conf.external_container_queue_border_color,
        ),
        crate::ir::C4ShapeKind::Component => {
            (&conf.component_bg_color, &conf.component_border_color)
        }
        crate::ir::C4ShapeKind::ComponentDb => {
            (&conf.component_db_bg_color, &conf.component_db_border_color)
        }
        crate::ir::C4ShapeKind::ComponentQueue => (
            &conf.component_queue_bg_color,
            &conf.component_queue_border_color,
        ),
        crate::ir::C4ShapeKind::ExternalComponent => (
            &conf.external_component_bg_color,
            &conf.external_component_border_color,
        ),
        crate::ir::C4ShapeKind::ExternalComponentDb => (
            &conf.external_component_db_bg_color,
            &conf.external_component_db_border_color,
        ),
        crate::ir::C4ShapeKind::ExternalComponentQueue => (
            &conf.external_component_queue_bg_color,
            &conf.external_component_queue_border_color,
        ),
    }
}

fn c4_shape_font_family(conf: &crate::config::C4Config, kind: crate::ir::C4ShapeKind) -> &str {
    match kind {
        crate::ir::C4ShapeKind::Person => &conf.person_font_family,
        crate::ir::C4ShapeKind::ExternalPerson => &conf.external_person_font_family,
        crate::ir::C4ShapeKind::System => &conf.system_font_family,
        crate::ir::C4ShapeKind::SystemDb => &conf.system_db_font_family,
        crate::ir::C4ShapeKind::SystemQueue => &conf.system_queue_font_family,
        crate::ir::C4ShapeKind::ExternalSystem => &conf.external_system_font_family,
        crate::ir::C4ShapeKind::ExternalSystemDb => &conf.external_system_db_font_family,
        crate::ir::C4ShapeKind::ExternalSystemQueue => &conf.external_system_queue_font_family,
        crate::ir::C4ShapeKind::Container => &conf.container_font_family,
        crate::ir::C4ShapeKind::ContainerDb => &conf.container_db_font_family,
        crate::ir::C4ShapeKind::ContainerQueue => &conf.container_queue_font_family,
        crate::ir::C4ShapeKind::ExternalContainer => &conf.external_container_font_family,
        crate::ir::C4ShapeKind::ExternalContainerDb => &conf.external_container_db_font_family,
        crate::ir::C4ShapeKind::ExternalContainerQueue => {
            &conf.external_container_queue_font_family
        }
        crate::ir::C4ShapeKind::Component => &conf.component_font_family,
        crate::ir::C4ShapeKind::ComponentDb => &conf.component_db_font_family,
        crate::ir::C4ShapeKind::ComponentQueue => &conf.component_queue_font_family,
        crate::ir::C4ShapeKind::ExternalComponent => &conf.external_component_font_family,
        crate::ir::C4ShapeKind::ExternalComponentDb => &conf.external_component_db_font_family,
        crate::ir::C4ShapeKind::ExternalComponentQueue => {
            &conf.external_component_queue_font_family
        }
    }
}

fn c4_shape_font_size(conf: &crate::config::C4Config, kind: crate::ir::C4ShapeKind) -> f32 {
    match kind {
        crate::ir::C4ShapeKind::Person => conf.person_font_size,
        crate::ir::C4ShapeKind::ExternalPerson => conf.external_person_font_size,
        crate::ir::C4ShapeKind::System => conf.system_font_size,
        crate::ir::C4ShapeKind::SystemDb => conf.system_db_font_size,
        crate::ir::C4ShapeKind::SystemQueue => conf.system_queue_font_size,
        crate::ir::C4ShapeKind::ExternalSystem => conf.external_system_font_size,
        crate::ir::C4ShapeKind::ExternalSystemDb => conf.external_system_db_font_size,
        crate::ir::C4ShapeKind::ExternalSystemQueue => conf.external_system_queue_font_size,
        crate::ir::C4ShapeKind::Container => conf.container_font_size,
        crate::ir::C4ShapeKind::ContainerDb => conf.container_db_font_size,
        crate::ir::C4ShapeKind::ContainerQueue => conf.container_queue_font_size,
        crate::ir::C4ShapeKind::ExternalContainer => conf.external_container_font_size,
        crate::ir::C4ShapeKind::ExternalContainerDb => conf.external_container_db_font_size,
        crate::ir::C4ShapeKind::ExternalContainerQueue => conf.external_container_queue_font_size,
        crate::ir::C4ShapeKind::Component => conf.component_font_size,
        crate::ir::C4ShapeKind::ComponentDb => conf.component_db_font_size,
        crate::ir::C4ShapeKind::ComponentQueue => conf.component_queue_font_size,
        crate::ir::C4ShapeKind::ExternalComponent => conf.external_component_font_size,
        crate::ir::C4ShapeKind::ExternalComponentDb => conf.external_component_db_font_size,
        crate::ir::C4ShapeKind::ExternalComponentQueue => conf.external_component_queue_font_size,
    }
}

fn c4_shape_font_weight(conf: &crate::config::C4Config, kind: crate::ir::C4ShapeKind) -> &str {
    match kind {
        crate::ir::C4ShapeKind::Person => &conf.person_font_weight,
        crate::ir::C4ShapeKind::ExternalPerson => &conf.external_person_font_weight,
        crate::ir::C4ShapeKind::System => &conf.system_font_weight,
        crate::ir::C4ShapeKind::SystemDb => &conf.system_db_font_weight,
        crate::ir::C4ShapeKind::SystemQueue => &conf.system_queue_font_weight,
        crate::ir::C4ShapeKind::ExternalSystem => &conf.external_system_font_weight,
        crate::ir::C4ShapeKind::ExternalSystemDb => &conf.external_system_db_font_weight,
        crate::ir::C4ShapeKind::ExternalSystemQueue => &conf.external_system_queue_font_weight,
        crate::ir::C4ShapeKind::Container => &conf.container_font_weight,
        crate::ir::C4ShapeKind::ContainerDb => &conf.container_db_font_weight,
        crate::ir::C4ShapeKind::ContainerQueue => &conf.container_queue_font_weight,
        crate::ir::C4ShapeKind::ExternalContainer => &conf.external_container_font_weight,
        crate::ir::C4ShapeKind::ExternalContainerDb => &conf.external_container_db_font_weight,
        crate::ir::C4ShapeKind::ExternalContainerQueue => {
            &conf.external_container_queue_font_weight
        }
        crate::ir::C4ShapeKind::Component => &conf.component_font_weight,
        crate::ir::C4ShapeKind::ComponentDb => &conf.component_db_font_weight,
        crate::ir::C4ShapeKind::ComponentQueue => &conf.component_queue_font_weight,
        crate::ir::C4ShapeKind::ExternalComponent => &conf.external_component_font_weight,
        crate::ir::C4ShapeKind::ExternalComponentDb => &conf.external_component_db_font_weight,
        crate::ir::C4ShapeKind::ExternalComponentQueue => {
            &conf.external_component_queue_font_weight
        }
    }
}

fn is_class_annotation_label_text(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with('\u{00ab}') && trimmed.ends_with('\u{00bb}')
}

fn text_block_svg_class(
    node: &crate::layout::NodeLayout,
    theme: &Theme,
    config: &LayoutConfig,
    override_color: Option<&str>,
) -> String {
    let line_height = theme.font_size * config.class_diagram_label_line_height();
    let total_height = node.label.lines.len() as f32 * line_height;
    let start_y = node.y + node.height / 2.0 - total_height / 2.0 + theme.font_size;
    let center_x = node.x + node.width / 2.0;
    let left_x = node.x + config.node_padding_x.max(10.0);
    let fill = override_color.unwrap_or(theme.primary_text_color.as_str());

    let text_lines: Vec<String> = node
        .label
        .lines
        .iter()
        .map(|l| l.text().into_owned())
        .collect();
    let Some(divider_idx) = text_lines.iter().position(|line| is_divider_line(line)) else {
        let lines: Vec<(usize, &str)> = text_lines
            .iter()
            .enumerate()
            .map(|(idx, line)| (idx, line.as_str()))
            .collect();
        return text_lines_svg(
            &lines,
            center_x,
            start_y,
            line_height,
            "middle",
            theme,
            fill,
            false,
        );
    };

    let mut annotation_lines: Vec<(usize, &str)> = Vec::new();
    let mut title_lines: Vec<(usize, &str)> = Vec::new();
    for (idx, line) in text_lines.iter().enumerate().take(divider_idx) {
        if !line.trim().is_empty() {
            if is_class_annotation_label_text(line) {
                annotation_lines.push((idx, line.as_str()));
            } else {
                title_lines.push((idx, line.as_str()));
            }
        }
    }
    let mut member_lines: Vec<(usize, &str)> = Vec::new();
    for (idx, line) in text_lines.iter().enumerate().skip(divider_idx + 1) {
        if !line.trim().is_empty() && !is_divider_line(line) {
            member_lines.push((idx, line.as_str()));
        }
    }

    let mut svg = String::new();
    if !annotation_lines.is_empty() {
        svg.push_str(&text_lines_svg(
            &annotation_lines,
            center_x,
            start_y,
            line_height,
            "middle",
            theme,
            fill,
            false,
        ));
    }
    if !title_lines.is_empty() {
        svg.push_str(&text_lines_svg(
            &title_lines,
            center_x,
            start_y,
            line_height,
            "middle",
            theme,
            fill,
            true,
        ));
    }
    if !member_lines.is_empty() {
        svg.push_str(&text_lines_svg(
            &member_lines,
            left_x,
            start_y,
            line_height,
            "start",
            theme,
            fill,
            false,
        ));
    }
    svg
}

fn render_er_node_label(
    node: &crate::layout::NodeLayout,
    theme: &Theme,
    config: &LayoutConfig,
) -> Option<String> {
    let text_lines: Vec<String> = node
        .label
        .lines
        .iter()
        .map(|l| l.text().into_owned())
        .collect();
    let divider_idx = text_lines.iter().position(|line| is_divider_line(line))?;
    let line_height = theme.font_size * config.class_label_line_height();
    let total_height = node.label.lines.len() as f32 * line_height;
    let start_y = node.y + node.height / 2.0 - total_height / 2.0 + theme.font_size;
    let center_x = node.x + node.width / 2.0;
    let left_x = node.x + config.node_padding_x.max(10.0);
    let fill = node
        .style
        .text_color
        .as_deref()
        .unwrap_or(theme.primary_text_color.as_str());

    let mut title_lines: Vec<(usize, &str)> = Vec::new();
    for (idx, line) in text_lines.iter().enumerate().take(divider_idx) {
        if !line.trim().is_empty() {
            title_lines.push((idx, line.as_str()));
        }
    }
    let mut attr_lines: Vec<(usize, &str)> = Vec::new();
    for (idx, line) in text_lines.iter().enumerate().skip(divider_idx + 1) {
        if !line.trim().is_empty() && !is_divider_line(line) {
            attr_lines.push((idx, line.as_str()));
        }
    }

    let mut svg = String::new();
    if !title_lines.is_empty() {
        let divider_baseline = start_y + divider_idx as f32 * line_height;
        let header_bottom = divider_baseline - line_height * 0.3;
        let header_top = (start_y - line_height * 0.9).min(header_bottom);
        let header_height = (header_bottom - header_top).max(0.0);
        if header_height > 0.0 {
            svg.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"6\" ry=\"6\" fill=\"{}\" fill-opacity=\"0.22\" stroke=\"none\"/>",
                node.x + 0.5,
                header_top,
                (node.width - 1.0).max(0.0),
                header_height,
                theme.cluster_background
            ));
        }
        svg.push_str(&text_lines_svg(
            &title_lines,
            center_x,
            start_y,
            line_height,
            "middle",
            theme,
            fill,
            true,
        ));
        svg.push_str(&format!(
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1\" stroke-opacity=\"0.35\"/>",
            node.x + 0.8,
            header_bottom,
            node.x + node.width - 0.8,
            header_bottom,
            theme.primary_border_color
        ));
    }

    if !attr_lines.is_empty() {
        let mut parsed: Vec<(usize, String, String)> = Vec::new();
        let mut max_type_width: f32 = 0.0;
        let mut use_columns = true;
        for (idx, line) in &attr_lines {
            let trimmed = line.trim();
            let mut parts = trimmed.split_whitespace();
            let Some(first) = parts.next() else {
                continue;
            };
            let rest = trimmed[first.len()..].trim();
            if rest.is_empty() {
                use_columns = false;
                break;
            }
            let width =
                text_metrics::get_computed_text_length(first, theme.font_size, &theme.font_family);
            max_type_width = max_type_width.max(width);
            parsed.push((*idx, first.to_string(), rest.to_string()));
        }

        let pad_x = config.node_padding_x.max(10.0);
        let content_width = (node.width - pad_x * 2.0).max(0.0);
        let gap = theme.font_size * 0.65;
        let name_x = left_x + max_type_width + gap;
        let body_top = start_y + (divider_idx as f32 + 0.15) * line_height;
        let body_bottom = node.y + node.height - line_height * 0.25;

        for (row_idx, (idx, _)) in attr_lines.iter().enumerate() {
            if row_idx % 2 == 0 {
                let row_top = start_y + *idx as f32 * line_height - line_height * 0.85;
                let row_height = line_height;
                svg.push_str(&format!(
                    "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" fill-opacity=\"0.12\" stroke=\"none\"/>",
                    node.x + 0.5,
                    row_top,
                    (node.width - 1.0).max(0.0),
                    row_height,
                    theme.secondary_color
                ));
            }
        }

        if use_columns && name_x < node.x + pad_x + content_width {
            let divider_x = name_x - gap * 0.5;
            svg.push_str(&format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1\" stroke-opacity=\"0.2\"/>",
                divider_x,
                body_top,
                divider_x,
                body_bottom,
                theme.primary_border_color
            ));
            for (idx, ty, name) in parsed {
                let y = start_y + idx as f32 * line_height;
                svg.push_str(&format!(
                    "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"start\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\" fill-opacity=\"0.75\">{}</text>",
                    left_x,
                    y,
                    normalize_font_family(&theme.font_family),
                    theme.font_size,
                    fill,
                    escape_xml(&ty)
                ));
                svg.push_str(&format!(
                    "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"start\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                    name_x,
                    y,
                    normalize_font_family(&theme.font_family),
                    theme.font_size,
                    fill,
                    escape_xml(&name)
                ));
            }
        } else {
            svg.push_str(&text_lines_svg(
                &attr_lines,
                left_x,
                start_y,
                line_height,
                "start",
                theme,
                fill,
                false,
            ));
        }
    }

    Some(svg)
}

fn text_lines_svg(
    lines: &[(usize, &str)],
    x: f32,
    start_y: f32,
    line_height: f32,
    anchor: &str,
    theme: &Theme,
    fill: &str,
    bold_first: bool,
) -> String {
    let Some((first_idx, _)) = lines.first() else {
        return String::new();
    };
    let first_y = start_y + *first_idx as f32 * line_height;
    let mut text = String::new();
    text.push_str(&format!(
        "<text x=\"{x:.2}\" y=\"{first_y:.2}\" text-anchor=\"{anchor}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">",
        normalize_font_family(&theme.font_family),
        theme.font_size,
        fill
    ));

    let mut prev_idx = *first_idx;
    for (pos, (idx, line)) in lines.iter().enumerate() {
        let dy = if pos == 0 {
            0.0
        } else {
            (*idx - prev_idx) as f32 * line_height
        };
        let weight = if pos == 0 && bold_first {
            " font-weight=\"600\""
        } else {
            ""
        };
        text.push_str(&format!(
            "<tspan x=\"{x:.2}\" dy=\"{dy:.2}\"{weight}>{}</tspan>",
            escape_xml(line)
        ));
        prev_idx = *idx;
    }
    text.push_str("</text>");
    text
}

fn is_divider_line(line: &str) -> bool {
    line.trim() == "---"
}

fn is_divider_text_line(line: &crate::layout::TextLine) -> bool {
    is_divider_line(&line.text())
}

fn divider_lines_svg(
    node: &crate::layout::NodeLayout,
    theme: &Theme,
    line_height: f32,
    extend_to_border: bool,
    render_as_class_path: bool,
) -> String {
    if !node
        .label
        .lines
        .iter()
        .any(|line| is_divider_text_line(line))
    {
        return String::new();
    }

    let total_height = node.label.lines.len() as f32 * line_height;
    let start_y = node.y + node.height / 2.0 - total_height / 2.0 + theme.font_size;
    let stroke = node
        .style
        .stroke
        .as_ref()
        .unwrap_or(&theme.primary_border_color);
    let stroke_width = node
        .style
        .stroke_width
        .map(|value| value.to_string())
        .unwrap_or_else(|| {
            if render_as_class_path {
                "1.3".to_string()
            } else {
                "1.0".to_string()
            }
        });
    let dash = node
        .style
        .stroke_dasharray
        .as_ref()
        .map(|value| format!(" stroke-dasharray=\"{}\"", value))
        .unwrap_or_default();
    let (x1, x2) = if extend_to_border {
        (node.x, node.x + node.width)
    } else {
        (node.x + 6.0, node.x + node.width - 6.0)
    };

    let mut svg = String::new();
    for (idx, line) in node.label.lines.iter().enumerate() {
        if !is_divider_text_line(line) {
            continue;
        }
        let baseline_y = start_y + idx as f32 * line_height;
        let y = baseline_y - theme.font_size * 0.35;
        if render_as_class_path {
            let d = class_box_rough_line_path(x1, y, x2, y + 0.001);
            svg.push_str(&format!(
                "<g class=\"divider\"><path d=\"{d}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\" fill=\"none\"{dash}/></g>",
            ));
        } else {
            svg.push_str(&format!(
                "<line x1=\"{x1:.2}\" y1=\"{y:.2}\" x2=\"{x2:.2}\" y2=\"{y:.2}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{dash}/>",
            ));
        }
    }

    svg
}

#[derive(Debug, Clone)]
struct ErAttribute {
    name: String,
    data_type: String,
    keys: Vec<String>,
    comment: Option<String>,
}

fn split_er_attribute_comment(line: &str) -> (&str, Option<String>) {
    let Some(first_quote) = line.find('"') else {
        return (line, None);
    };
    let Some(last_quote) = line.rfind('"') else {
        return (line, None);
    };
    if last_quote <= first_quote {
        return (line, None);
    }
    let comment = line[first_quote + 1..last_quote].trim();
    let comment = if comment.is_empty() {
        None
    } else {
        Some(comment.to_string())
    };
    (line[..first_quote].trim_end(), comment)
}

fn parse_er_attributes(
    lines: &[crate::layout::TextLine],
) -> (crate::layout::TextLine, Vec<ErAttribute>) {
    let mut title = lines
        .first()
        .cloned()
        .unwrap_or_else(|| crate::layout::TextLine::plain(String::new()));
    let mut attrs = Vec::new();
    let mut in_body = false;
    for line in lines.iter().skip(1) {
        let line_str = line.text();
        if is_divider_line(&line_str) {
            in_body = true;
            continue;
        }
        if !in_body {
            if !line_str.trim().is_empty() {
                title = line.clone();
            }
            continue;
        }
        let trimmed = line_str.trim();
        if trimmed.is_empty() {
            continue;
        }
        let (attr_text, comment) = split_er_attribute_comment(trimmed);
        let mut keys = Vec::new();
        let mut parts: Vec<String> = Vec::new();
        for token in attr_text.split_whitespace() {
            let cleaned = token
                .trim_matches(|ch: char| ch == ',' || ch == ';')
                .to_ascii_uppercase();
            if cleaned == "PK" || cleaned == "FK" || cleaned == "UK" {
                keys.push(cleaned);
                continue;
            }
            if cleaned.contains(',') {
                let mut handled = false;
                for piece in cleaned.split(',') {
                    if piece == "PK" || piece == "FK" || piece == "UK" {
                        keys.push(piece.to_string());
                        handled = true;
                    }
                }
                if handled {
                    continue;
                }
            }
            parts.push(token.to_string());
        }
        if parts.is_empty() {
            continue;
        }
        let (data_type, name) = if parts.len() >= 2 {
            (parts[0].clone(), parts[1..].join(" "))
        } else {
            (String::new(), parts[0].clone())
        };
        attrs.push(ErAttribute {
            name,
            data_type,
            keys,
            comment,
        });
    }
    (title, attrs)
}

fn er_badge_svg(
    x: f32,
    y: f32,
    text: &str,
    font_size: f32,
    fill: &str,
    text_color: &str,
    font_family: &str,
) -> (String, f32) {
    let font_family = normalize_font_family(font_family);
    let pad_x = (font_size * 0.45).max(4.0);
    let text_width = text_metrics::measure_text_width(text, font_size * 0.72, &font_family)
        .unwrap_or(font_size * 0.9);
    let width = text_width + pad_x * 2.0;
    let height = (font_size * 0.9).max(10.0);
    let rect_y = y - height / 2.0;
    let rx = (height / 2.0).max(4.0);
    let mut svg = String::new();
    svg.push_str(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" ry=\"{:.2}\" fill=\"{}\"/>",
        x, rect_y, width, height, rx, rx, fill
    ));
    svg.push_str(&format!(
        "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{:.2}\" font-weight=\"600\" fill=\"{}\">{}</text>",
        x + width / 2.0,
        y + font_size * 0.26,
        font_family,
        font_size * 0.72,
        text_color,
        escape_xml(text)
    ));
    (svg, width)
}

fn render_er_node(
    node: &crate::layout::NodeLayout,
    theme: &Theme,
    config: &LayoutConfig,
) -> String {
    const ER_ROW_ODD_FILL: &str = "hsl(240, 100%, 100%)";
    const ER_ROW_EVEN_FILL: &str = "hsl(240, 100%, 97.2745098039%)";

    let (title, attrs) = parse_er_attributes(&node.label.lines);
    let font_size = theme.font_size;
    let line_height = font_size * config.label_line_height;
    let header_height = if attrs.is_empty() {
        node.height
    } else {
        crate::layout::ER_ATTRIBUTE_ROW_HEIGHT
    };

    let border = node
        .style
        .stroke
        .as_ref()
        .unwrap_or(&theme.primary_border_color);
    let custom_fill = node.style.fill.as_deref();
    let body_fill = custom_fill.unwrap_or(&theme.background);
    let header_fill = custom_fill.unwrap_or(theme.cluster_background.as_str());
    let grid_color = border.as_str();
    let header_text_color = node
        .style
        .text_color
        .as_deref()
        .unwrap_or(theme.primary_text_color.as_str());
    let name_text_color = header_text_color;
    let type_text_color = header_text_color;
    let stroke_width = node.style.stroke_width.unwrap_or(1.2);
    let grid_stroke_width = node.style.stroke_width.unwrap_or(1.0);
    let stroke_dasharray = node
        .style
        .stroke_dasharray
        .as_deref()
        .map(|dasharray| format!(" stroke-dasharray=\"{}\"", escape_xml(dasharray)))
        .unwrap_or_default();

    let x = node.x;
    let y = node.y;
    let w = node.width;
    let h = node.height;
    let radius = 0.0;

    let mut svg = String::new();
    svg.push_str(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" ry=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
        x,
        y,
        w,
        h,
        radius,
        radius,
        body_fill,
        border,
        stroke_width,
        stroke_dasharray
    ));

    svg.push_str(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" ry=\"{:.2}\" fill=\"{}\"/>",
        x,
        y,
        w,
        header_height,
        radius,
        radius,
        header_fill
    ));

    let header_label = TextBlock {
        lines: vec![title.clone()],
        width: 0.0,
        height: 0.0,
    };
    let header_y = y + header_height / 2.0;
    svg.push_str(&text_block_svg_anchor(
        x + w / 2.0,
        header_y,
        &header_label,
        theme,
        config,
        "middle",
        Some(header_text_color),
    ));

    if attrs.is_empty() {
        return svg;
    }

    let pad_x = (font_size * 0.8).max(10.0);
    let mut max_type_width = 0.0f32;
    let mut max_name_width = 0.0f32;
    let mut max_badge_width = 0.0f32;
    let mut max_comment_width = 0.0f32;
    for attr in &attrs {
        if !attr.data_type.is_empty() {
            if let Some(width) =
                text_metrics::measure_text_width(&attr.data_type, font_size, &theme.font_family)
            {
                max_type_width = max_type_width.max(width);
            }
        }
        if let Some(width) =
            text_metrics::measure_text_width(&attr.name, font_size, &theme.font_family)
        {
            max_name_width = max_name_width.max(width);
        }
        if !attr.keys.is_empty() {
            let mut row_badge_width = 0.0f32;
            for key in attr.keys.iter().take(2) {
                let text_width =
                    text_metrics::measure_text_width(key, font_size * 0.72, &theme.font_family)
                        .unwrap_or(font_size * 0.9);
                let badge_width = text_width + (font_size * 0.45).max(4.0) * 2.0;
                row_badge_width += badge_width + font_size * 0.4;
            }
            if row_badge_width > 0.0 {
                row_badge_width -= font_size * 0.4;
            }
            max_badge_width = max_badge_width.max(row_badge_width);
        }
        if let Some(comment) = attr.comment.as_deref()
            && let Some(width) =
                text_metrics::measure_text_width(comment, font_size, &theme.font_family)
        {
            max_comment_width = max_comment_width.max(width);
        }
    }

    let col_gap = font_size * 0.9;
    let type_x = x + pad_x;
    let name_x = if max_type_width > 0.0 {
        type_x + max_type_width + col_gap
    } else {
        type_x
    };
    let keys_x = name_x + max_name_width + col_gap;
    let comment_x = keys_x + max_badge_width + col_gap;
    let show_keys_col = max_badge_width > 0.0 && keys_x < x + w - pad_x;
    let show_comment_col = max_comment_width > 0.0 && comment_x < x + w - pad_x;

    let mut row_height = crate::layout::ER_ATTRIBUTE_ROW_HEIGHT;
    let body_height = (h - header_height).max(line_height);
    let needed = attrs.len() as f32 * row_height;
    if needed > body_height {
        row_height = body_height / attrs.len() as f32;
    }

    let row_even_fill = custom_fill.unwrap_or(ER_ROW_EVEN_FILL);
    for idx in 0..attrs.len() {
        let row_top = y + header_height + idx as f32 * row_height;
        let (row_class, row_fill) = if idx % 2 == 0 {
            ("odd", ER_ROW_ODD_FILL)
        } else {
            ("even", row_even_fill)
        };
        svg.push_str(&format!(
            "<rect class=\"row-rect-{}\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" ry=\"{:.2}\" fill=\"{}\"/>",
            row_class,
            x,
            row_top,
            w,
            row_height,
            radius,
            radius,
            row_fill
        ));
    }

    svg.push_str(&format!(
        "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{}\" stroke-opacity=\"0.6\"{}/>",
        x,
        y + header_height,
        x + w,
        y + header_height,
        grid_color,
        grid_stroke_width,
        stroke_dasharray
    ));

    if max_type_width > 0.0 {
        svg.push_str(&format!(
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{}\" stroke-opacity=\"0.45\"{}/>",
            name_x - col_gap * 0.45,
            y + header_height,
            name_x - col_gap * 0.45,
            y + h,
            grid_color,
            grid_stroke_width,
            stroke_dasharray
        ));
    }

    for (idx, attr) in attrs.iter().enumerate() {
        let row_top = y + header_height + idx as f32 * row_height;
        let row_center = row_top + row_height / 2.0;
        if idx > 0 {
            svg.push_str(&format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{}\" stroke-opacity=\"0.35\"{}/>",
                x,
                row_top,
                x + w,
                row_top,
                grid_color,
                grid_stroke_width,
                stroke_dasharray
            ));
        }

        if !attr.data_type.is_empty() {
            let type_label = TextBlock {
                lines: vec![crate::layout::TextLine::plain(attr.data_type.clone())],
                width: 0.0,
                height: 0.0,
            };
            svg.push_str(&text_block_svg_anchor(
                type_x,
                row_center,
                &type_label,
                theme,
                config,
                "start",
                Some(type_text_color),
            ));
        }

        let name_label = TextBlock {
            lines: vec![crate::layout::TextLine::plain(attr.name.clone())],
            width: 0.0,
            height: 0.0,
        };
        svg.push_str(&text_block_svg_anchor(
            name_x,
            row_center,
            &name_label,
            theme,
            config,
            "start",
            Some(name_text_color),
        ));

        if show_keys_col {
            let mut cursor_x = keys_x;
            for key in attr.keys.iter().take(2) {
                let fill = match key.as_str() {
                    "PK" => "#1D4ED8",
                    "FK" => "#0F766E",
                    "UK" => "#7C3AED",
                    _ => "#475569",
                };
                let (badge_svg, badge_width) = er_badge_svg(
                    cursor_x,
                    row_center,
                    key,
                    font_size,
                    fill,
                    "#FFFFFF",
                    &theme.font_family,
                );
                svg.push_str(&badge_svg);
                cursor_x += badge_width + font_size * 0.4;
            }
        }

        if show_comment_col && let Some(comment) = attr.comment.as_deref() {
            let comment_label = TextBlock {
                lines: vec![crate::layout::TextLine::plain(comment.to_string())],
                width: 0.0,
                height: 0.0,
            };
            svg.push_str(&text_block_svg_anchor(
                comment_x,
                row_center,
                &comment_label,
                theme,
                config,
                "start",
                Some(type_text_color),
            ));
        }
    }

    svg
}

pub fn write_output_svg(svg: &str, output: Option<&Path>) -> Result<()> {
    match output {
        Some(path) => {
            std::fs::write(path, svg)?;
        }
        None => {
            print!("{}", svg);
        }
    }
    Ok(())
}

#[cfg(feature = "png")]
pub fn write_output_png(
    svg: &str,
    output: &Path,
    render_cfg: &RenderConfig,
    theme: &Theme,
) -> Result<()> {
    let mut opt = usvg::Options {
        font_family: primary_font(&theme.font_family),
        default_size: usvg::Size::from_wh(render_cfg.width, render_cfg.height)
            .unwrap_or(usvg::Size::from_wh(800.0, 600.0).unwrap()),
        ..Default::default()
    };

    opt.fontdb_mut().load_system_fonts();
    #[cfg(target_os = "ios")]
    {
        opt.fontdb_mut().load_fonts_dir("/System/Library/Fonts");
        opt.fontdb_mut()
            .load_fonts_dir("/System/Library/Fonts/Core");
    }

    let tree = usvg::Tree::from_str(svg, &opt)?;
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or_else(|| anyhow::anyhow!("Failed to allocate pixmap"))?;
    if let Some(color) = parse_hex_color(&theme.background) {
        pixmap.fill(color);
    }

    let mut pixmap_mut = pixmap.as_mut();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap_mut,
    );
    pixmap.save_png(output)?;
    Ok(())
}

/// Render a sequence diagram actor/participant with the appropriate shape.
fn render_sequence_actor_shape(
    svg: &mut String,
    node: &crate::layout::NodeLayout,
    theme: &Theme,
    _config: &LayoutConfig,
    _is_footbox: bool,
) {
    use crate::ir::NodeShape;

    let hide_label = node
        .label
        .lines
        .iter()
        .all(|line| line.text().trim().is_empty())
        || node.id.starts_with("__start_")
        || node.id.starts_with("__end_");

    match node.shape {
        // mermaid.js renders `actor` as a stick figure. `boundary`,
        // `control`, and `entity` each have their own UML glyphs and
        // are handled in dedicated arms below. Queue/Collections also
        // get their own arms; `database` falls through to the cylinder
        // arm.
        NodeShape::StickFigure => {
            // Draw a stick figure above the label.
            let cx = node.x + node.width / 2.0;
            let top = node.y;
            let head_r = 10.0;
            let head_cy = top + head_r + 2.0;
            let body_top = head_cy + head_r;
            let body_bot = body_top + 16.0;
            let leg_bot = body_bot + 16.0;
            let arm_y = body_top + 6.0;
            let arm_half = 14.0;
            // Head
            svg.push_str(&format!(
                "<circle cx=\"{cx:.2}\" cy=\"{head_cy:.2}\" r=\"{head_r:.2}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                stroke = theme.sequence_actor_border
            ));
            // Body
            svg.push_str(&format!(
                "<line x1=\"{cx:.2}\" y1=\"{body_top:.2}\" x2=\"{cx:.2}\" y2=\"{body_bot:.2}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                stroke = theme.sequence_actor_border
            ));
            // Arms
            svg.push_str(&format!(
                "<line x1=\"{x1:.2}\" y1=\"{arm_y:.2}\" x2=\"{x2:.2}\" y2=\"{arm_y:.2}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                x1 = cx - arm_half, x2 = cx + arm_half, stroke = theme.sequence_actor_border
            ));
            // Legs
            svg.push_str(&format!(
                "<line x1=\"{cx:.2}\" y1=\"{body_bot:.2}\" x2=\"{x1:.2}\" y2=\"{leg_bot:.2}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                x1 = cx - 12.0, stroke = theme.sequence_actor_border
            ));
            svg.push_str(&format!(
                "<line x1=\"{cx:.2}\" y1=\"{body_bot:.2}\" x2=\"{x2:.2}\" y2=\"{leg_bot:.2}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                x2 = cx + 12.0, stroke = theme.sequence_actor_border
            ));
            // Label below the figure
            if !hide_label {
                let label_y = leg_bot + 8.0;
                svg.push_str(&sequence_text_block_svg(
                    cx,
                    label_y,
                    &node.label,
                    theme,
                    false,
                    node.style.text_color.as_deref(),
                ));
            }
        }
        NodeShape::Boundary => {
            // UML boundary glyph: vertical bar | + horizontal connector — + circle O.
            // Geometry mirrors mermaid JS drawActorTypeBoundary: torso line at
            // y=12 from cx-radius*2.5 to cx-15, vertical arms at cx-radius*2.5
            // (y=2..22), circle at cx with r=22.
            let cx = node.x + node.width / 2.0;
            let radius = 22.0_f32;
            let glyph_y = node.y + 12.0;
            let bar_x = cx - radius * 2.5;
            let stroke = &theme.sequence_actor_border;
            // Vertical bar (left)
            svg.push_str(&format!(
                "<line x1=\"{bar_x:.2}\" y1=\"{y1:.2}\" x2=\"{bar_x:.2}\" y2=\"{y2:.2}\" stroke=\"{stroke}\" stroke-width=\"2\"/>",
                y1 = glyph_y - 10.0, y2 = glyph_y + 10.0
            ));
            // Horizontal connector
            svg.push_str(&format!(
                "<line x1=\"{x1:.2}\" y1=\"{glyph_y:.2}\" x2=\"{x2:.2}\" y2=\"{glyph_y:.2}\" stroke=\"{stroke}\" stroke-width=\"2\"/>",
                x1 = bar_x, x2 = cx - 15.0
            ));
            // Circle (right)
            svg.push_str(&format!(
                "<circle cx=\"{cx:.2}\" cy=\"{glyph_y:.2}\" r=\"{radius:.2}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"2\"/>"
            ));
            if !hide_label {
                let label_y = glyph_y + radius + 16.0;
                svg.push_str(&sequence_text_block_svg(
                    cx,
                    label_y,
                    &node.label,
                    theme,
                    false,
                    node.style.text_color.as_deref(),
                ));
            }
        }
        NodeShape::Control => {
            // UML control glyph: filled circle (with a tiny arrow marker
            // at the top in JS — rendered here as just the circle since the
            // marker is zero-length and visually subtle). Mirrors mermaid JS
            // drawActorTypeControl: circle at (cx, actorY+32, r=22) with
            // stroke-width 1.2.
            let cx = node.x + node.width / 2.0;
            let radius = 22.0_f32;
            let cy = node.y + 32.0;
            let stroke = &theme.sequence_actor_border;
            let fill = &theme.sequence_actor_fill;
            svg.push_str(&format!(
                "<circle cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{radius:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.2\"/>"
            ));
            if !hide_label {
                let label_y = cy + radius + 12.0;
                svg.push_str(&sequence_text_block_svg(
                    cx,
                    label_y,
                    &node.label,
                    theme,
                    false,
                    node.style.text_color.as_deref(),
                ));
            }
        }
        NodeShape::Entity => {
            // UML entity glyph: circle with a horizontal underline below.
            // Mirrors mermaid JS drawActorTypeEntity: circle at
            // (cx, actorY+25, r=22) and a 2-px stroke line at y=cy+r from
            // x-r to x+r.
            let cx = node.x + node.width / 2.0;
            let radius = 22.0_f32;
            let cy = node.y + 25.0;
            let stroke = &theme.sequence_actor_border;
            let fill = &theme.sequence_actor_fill;
            svg.push_str(&format!(
                "<circle cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{radius:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.2\"/>"
            ));
            let underline_y = cy + radius;
            svg.push_str(&format!(
                "<line x1=\"{x1:.2}\" y1=\"{underline_y:.2}\" x2=\"{x2:.2}\" y2=\"{underline_y:.2}\" stroke=\"{stroke}\" stroke-width=\"2\"/>",
                x1 = cx - radius, x2 = cx + radius
            ));
            if !hide_label {
                let label_y = underline_y + 12.0;
                svg.push_str(&sequence_text_block_svg(
                    cx,
                    label_y,
                    &node.label,
                    theme,
                    false,
                    node.style.text_color.as_deref(),
                ));
            }
        }
        NodeShape::Collections => {
            // Collections: stacked-papers look — two offset rects.
            // JS draws a primary rect plus a back rect shifted by (-6, +6) so
            // the back rect peeks out at the bottom-left corner.
            let x = node.x;
            let y = node.y;
            let w = node.width;
            let h = node.height;
            // Primary rect drawn first.
            svg.push_str(&format!(
                "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.0\"/>",
                fill = theme.sequence_actor_fill, stroke = theme.sequence_actor_border
            ));
            // Back rect on top, shifted down-left — matches JS draw order so
            // the offset rect's left/bottom edges are visible as a "second
            // paper" silhouette.
            svg.push_str(&format!(
                "<rect x=\"{xb:.2}\" y=\"{yb:.2}\" width=\"{w:.2}\" height=\"{h:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.0\"/>",
                xb = x - 6.0, yb = y + 6.0,
                fill = theme.sequence_actor_fill, stroke = theme.sequence_actor_border
            ));
            if !hide_label {
                let cx = x + w / 2.0;
                let cy = y + h / 2.0;
                svg.push_str(&sequence_text_block_svg(
                    cx,
                    cy,
                    &node.label,
                    theme,
                    false,
                    node.style.text_color.as_deref(),
                ));
            }
        }
        NodeShape::Queue => {
            // Queue: horizontal pill with semi-elliptical caps on BOTH ends.
            // Matches mermaid.js queue actor shape (rx≈8.5 cap, full-height
            // ellipse caps). Body starts at node.x + cap_w so the LEFT bulge
            // (which extends leftward from path start) lands at node.x and the
            // queue body is centered on the lifeline at node.x + node.width/2.
            let y = node.y;
            let w = node.width;
            let h = node.height;
            let cap_w = (w * 0.057).max(6.0); // matches mermaid's 8.55 for w=150
            let body_w = w - cap_w * 2.0;
            let ry = h / 2.0;
            let cy = y + ry;
            let path_x = node.x + cap_w;
            // Single closed path: M(body left) -> arc-down (left cap, bulges
            //   LEFT into cap_w region) -> horizontal right -> arc-up (right
            //   cap, bulges RIGHT) -> horizontal left back.
            let d = format!(
                "M {x:.2},{y:.2} a {cap_w:.2},{ry:.2} 0 0,0 0,{h:.2} \
                 h {body_w:.2} a {cap_w:.2},{ry:.2} 0 0,0 0,-{h:.2} \
                 h -{body_w:.2} z",
                x = path_x,
                y = y,
                cap_w = cap_w,
                ry = ry,
                h = h,
                body_w = body_w,
            );
            svg.push_str(&format!(
                "<path d=\"{d}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.0\"/>",
                fill = theme.sequence_actor_fill,
                stroke = theme.sequence_actor_border
            ));
            if !hide_label {
                let cx = node.x + w / 2.0;
                svg.push_str(&sequence_text_block_svg(
                    cx,
                    cy,
                    &node.label,
                    theme,
                    false,
                    node.style.text_color.as_deref(),
                ));
            }
        }
        NodeShape::Cylinder => {
            // Database stereotype: small inset cylinder centered horizontally
            // with the label drawn BELOW it. Mirrors mermaid JS
            // drawActorTypeDatabase: cylinder w = h = actor.width / 3,
            // rx = w/2, ry = rx / (2.5 + w/50). Single closed path so no
            // internal horizontal stroke artifacts.
            let actor_cx = node.x + node.width / 2.0;
            let cyl_w = node.width / 3.0;
            let rx = cyl_w / 2.0;
            let ry = rx / (2.5 + cyl_w / 50.0);
            let cyl_h = cyl_w;
            let body_h = cyl_h - ry * 2.0;
            let cyl_x = actor_cx - cyl_w / 2.0;
            let cyl_top = node.y + ry;
            let d = format!(
                "M {cyl_x:.2},{cyl_top:.2} \
                 a {rx:.2},{ry:.2} 0 0 0 {cyl_w:.2},0 \
                 a {rx:.2},{ry:.2} 0 0 0 -{cyl_w:.2},0 \
                 l 0,{body_h:.2} \
                 a {rx:.2},{ry:.2} 0 0 0 {cyl_w:.2},0 \
                 l 0,-{body_h:.2}"
            );
            svg.push_str(&format!(
                "<path d=\"{d}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.0\"/>",
                fill = theme.sequence_actor_fill,
                stroke = theme.sequence_actor_border,
            ));
            if !hide_label {
                // Tight gap below cylinder front-arc bottom, matching JS
                // drawActorTypeDatabase which centers the label ~3 px below
                // the cylinder. text_block_svg adds ~4 px baseline pad on top
                // of this offset.
                let label_y = node.y + cyl_h + ry + 4.0;
                svg.push_str(&sequence_text_block_svg(
                    actor_cx,
                    label_y,
                    &node.label,
                    theme,
                    false,
                    node.style.text_color.as_deref(),
                ));
            }
        }
        // Default (ActorBox, Rectangle, etc.)
        _ => {
            svg.push_str(&format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"3\" ry=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.0\"/>",
                node.x, node.y, node.width, node.height,
                theme.sequence_actor_fill, theme.sequence_actor_border
            ));
            if !hide_label {
                let cx = node.x + node.width / 2.0;
                let cy = node.y + node.height / 2.0;
                svg.push_str(&sequence_text_block_svg(
                    cx,
                    cy,
                    &node.label,
                    theme,
                    false,
                    node.style.text_color.as_deref(),
                ));
            }
        }
    }
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn escape_xml_text_node(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(feature = "png")]
fn parse_hex_color(input: &str) -> Option<resvg::tiny_skia::Color> {
    let color = input.trim();
    let hex = color.strip_prefix('#')?;
    let (r, g, b, a) = match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            (r, g, b, 255)
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (r, g, b, 255)
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            (r, g, b, a)
        }
        _ => return None,
    };
    Some(resvg::tiny_skia::Color::from_rgba8(r, g, b, a))
}

fn link_attrs(link: &crate::ir::NodeLink) -> String {
    let url = escape_xml(&link.url);
    let mut attrs = format!("href=\"{}\" xlink:href=\"{}\"", url, url);
    if let Some(target) = link.target.as_deref() {
        let target = escape_xml(target);
        attrs.push_str(&format!(" target=\"{}\"", target));
        if target == "_blank" {
            attrs.push_str(" rel=\"noopener noreferrer\"");
        }
    }
    attrs
}

const CLASS_OPEN_MARKER_EXTENT: f32 = 17.25;
const CLASS_DEPENDENCY_MARKER_EXTENT: f32 = 6.0;
const CLASS_DECORATION_MARKER_EXTENT: f32 = 18.0;
const CLASS_GENERIC_MARKER_EXTENT: f32 = 8.0;
const CLASS_LOLLIPOP_MARKER_EXTENT: f32 = 6.0;

fn class_arrow_marker_extent(arrow: bool, kind: Option<crate::ir::EdgeArrowhead>) -> f32 {
    if !arrow {
        return 0.0;
    }
    match kind {
        Some(crate::ir::EdgeArrowhead::OpenTriangle) => CLASS_OPEN_MARKER_EXTENT,
        Some(crate::ir::EdgeArrowhead::ClassDependency) => CLASS_DEPENDENCY_MARKER_EXTENT,
        None => CLASS_GENERIC_MARKER_EXTENT,
    }
}

fn class_decoration_marker_extent(decoration: Option<crate::ir::EdgeDecoration>) -> f32 {
    match decoration {
        Some(crate::ir::EdgeDecoration::Diamond)
        | Some(crate::ir::EdgeDecoration::DiamondFilled) => CLASS_DECORATION_MARKER_EXTENT,
        Some(crate::ir::EdgeDecoration::Lollipop) => CLASS_LOLLIPOP_MARKER_EXTENT,
        Some(crate::ir::EdgeDecoration::Circle) | Some(crate::ir::EdgeDecoration::Cross) => {
            CLASS_GENERIC_MARKER_EXTENT
        }
        _ => 0.0,
    }
}

fn trim_polyline_endpoint(points: &mut [(f32, f32)], start: bool, amount: f32) {
    if points.len() < 2 || amount <= 0.0 {
        return;
    }
    let (endpoint_idx, neighbor_idx) = if start {
        (0, 1)
    } else {
        (points.len() - 1, points.len() - 2)
    };
    let endpoint = points[endpoint_idx];
    let neighbor = points[neighbor_idx];
    let dx = neighbor.0 - endpoint.0;
    let dy = neighbor.1 - endpoint.1;
    let length = (dx * dx + dy * dy).sqrt();
    if length <= 0.001 {
        return;
    }
    let trim = amount.min(length * 0.9);
    points[endpoint_idx] = (
        endpoint.0 + dx / length * trim,
        endpoint.1 + dy / length * trim,
    );
}

fn class_symbol_render_points(
    edge: &crate::layout::EdgeLayout,
    kind: crate::ir::DiagramKind,
) -> Vec<(f32, f32)> {
    let mut points = edge.points.clone();
    if kind != crate::ir::DiagramKind::Class || points.len() < 2 {
        return points;
    }

    let start_trim = class_arrow_marker_extent(edge.arrow_start, edge.arrow_start_kind)
        .max(class_decoration_marker_extent(edge.start_decoration));
    let end_trim = class_arrow_marker_extent(edge.arrow_end, edge.arrow_end_kind)
        .max(class_decoration_marker_extent(edge.end_decoration));
    trim_polyline_endpoint(&mut points, true, start_trim);
    trim_polyline_endpoint(&mut points, false, end_trim);
    points
}

fn edge_decoration_svg(
    point: (f32, f32),
    angle_deg: f32,
    decoration: crate::ir::EdgeDecoration,
    stroke: &str,
    stroke_width: f32,
    _at_start: bool,
) -> String {
    let (x, y) = point;
    let angle = angle_deg;
    let join = " stroke-linejoin=\"round\" stroke-linecap=\"round\"";
    let shape = match decoration {
        crate::ir::EdgeDecoration::Circle => format!(
            "<circle cx=\"0\" cy=\"0\" r=\"5\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>",
            stroke, stroke, stroke_width
        ),
        crate::ir::EdgeDecoration::Cross => format!(
            "<path d=\"M -5 -5 L 5 5 M -5 5 L 5 -5\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{join}/>",
            stroke, stroke_width
        ),
        crate::ir::EdgeDecoration::Lollipop => {
            let cx = if _at_start { -6.0 } else { 6.0 };
            format!(
                "<circle cx=\"{cx}\" cy=\"0\" r=\"6\" fill=\"#ECECFF\" stroke=\"{}\" stroke-width=\"{}\"/>",
                stroke, stroke_width
            )
        }
        crate::ir::EdgeDecoration::Diamond => {
            let points = "0,0 9,6 18,0 9,-6";
            format!(
                "<polygon points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{join}/>",
                points, stroke, stroke_width
            )
        }
        crate::ir::EdgeDecoration::DiamondFilled => {
            let points = "0,0 9,6 18,0 9,-6";
            format!(
                "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{join}/>",
                points, stroke, stroke, stroke_width
            )
        }
        // Crow's foot notation for ER diagrams
        crate::ir::EdgeDecoration::CrowsFootOne => format!(
            "<path d=\"M 9 -6 L 9 6 M 15 -6 L 15 6\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{join}/>",
            stroke, stroke_width
        ),
        crate::ir::EdgeDecoration::CrowsFootZeroOne => format!(
            "<g><circle cx=\"5\" cy=\"0\" r=\"4\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/><path d=\"M 13 -6 L 13 6\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{join}/></g>",
            stroke, stroke_width, stroke, stroke_width
        ),
        crate::ir::EdgeDecoration::CrowsFootMany => format!(
            "<path d=\"M 2 -6 L 2 6 M 2 0 L 10 -6 M 2 0 L 10 6\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{join}/>",
            stroke, stroke_width
        ),
        crate::ir::EdgeDecoration::CrowsFootZeroMany => format!(
            "<g><circle cx=\"5\" cy=\"0\" r=\"4\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/><path d=\"M 13 0 L 21 -6 M 13 0 L 21 6\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{join}/></g>",
            stroke, stroke_width, stroke, stroke_width
        ),
    };
    format!(
        "<g class=\"edgeDecoration\" data-edge-decoration=\"true\" transform=\"translate({x:.2} {y:.2}) rotate({angle:.2})\">{shape}</g>"
    )
}

fn default_edge_stroke_for_kind(kind: crate::ir::DiagramKind, theme: &Theme) -> String {
    if matches!(
        kind,
        crate::ir::DiagramKind::Block
            | crate::ir::DiagramKind::Class
            | crate::ir::DiagramKind::Flowchart
            | crate::ir::DiagramKind::Sequence
    ) && theme.line_color.eq_ignore_ascii_case("#2F3B4D")
    {
        "#333333".to_string()
    } else {
        theme.line_color.clone()
    }
}

fn color_with_opacity(color: &str, opacity: f32) -> String {
    let opacity = opacity.clamp(0.0, 1.0);
    if let Some((r, g, b)) = parse_hex_rgb(color.trim()) {
        format!("rgba({r}, {g}, {b}, {opacity})")
    } else {
        color.to_string()
    }
}

fn default_kanban_section_fill(index_zero_based: usize) -> &'static str {
    const COLORS: [&str; 11] = [
        "hsl(80, 100%, 86.2745098039%)",
        "hsl(270, 100%, 86.2745098039%)",
        "hsl(300, 100%, 86.2745098039%)",
        "hsl(330, 100%, 86.2745098039%)",
        "hsl(0, 100%, 86.2745098039%)",
        "hsl(30, 100%, 86.2745098039%)",
        "hsl(90, 100%, 86.2745098039%)",
        "hsl(150, 100%, 86.2745098039%)",
        "hsl(180, 100%, 86.2745098039%)",
        "hsl(210, 100%, 86.2745098039%)",
        "hsl(60, 100%, 83.5294117647%)",
    ];
    COLORS[index_zero_based % COLORS.len()]
}

fn parse_hex_rgb(value: &str) -> Option<(u8, u8, u8)> {
    let hex = value.strip_prefix('#')?;
    match hex.len() {
        3 => {
            let mut chars = hex.chars();
            let r = chars.next()?.to_digit(16)? as u8;
            let g = chars.next()?.to_digit(16)? as u8;
            let b = chars.next()?.to_digit(16)? as u8;
            Some((r * 17, g * 17, b * 17))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some((r, g, b))
        }
        _ => None,
    }
}

fn arrowhead_svg(point: (f32, f32), angle_deg: f32, stroke: &str, stroke_width: f32) -> String {
    let size = (stroke_width * 2.2 + 6.0).clamp(6.0, 14.0);
    let half = size * 0.6;
    let (x, y) = point;
    let join = " stroke-linejoin=\"round\" stroke-linecap=\"round\"";
    format!(
        "<g transform=\"translate({x:.2} {y:.2}) rotate({angle_deg:.2})\"><polygon points=\"0,0 {neg_size:.2},{half:.2} {neg_size:.2},{neg_half:.2}\" fill=\"{stroke}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{join}/></g>",
        neg_size = -size,
        half = half,
        neg_half = -half,
    )
}

fn edge_endpoint_angle(points: &[(f32, f32)], start: bool) -> f32 {
    if points.len() < 2 {
        return 0.0;
    }
    let (p0, p1) = if start {
        (points[0], points[1])
    } else {
        (points[points.len() - 2], points[points.len() - 1])
    };
    let dx = p1.0 - p0.0;
    let dy = p1.1 - p0.1;
    dy.atan2(dx).to_degrees()
}

fn render_kanban_item_node(
    node: &crate::layout::NodeLayout,
    theme: &Theme,
    config: &LayoutConfig,
) -> String {
    let parts = kanban_render_parts(&node.label);
    let fill = node.style.fill.as_deref().unwrap_or("#FFFFFF");
    let stroke = node.style.stroke.as_deref().unwrap_or("#9370DB");
    let stroke_width = node.style.stroke_width.unwrap_or(1.0);
    let mut out = String::new();

    out.push_str(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"5\" ry=\"5\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\" stroke-linejoin=\"round\" stroke-linecap=\"round\"/>",
        node.x, node.y, node.width, node.height, fill, stroke, stroke_width
    ));

    if !parts.title_lines.is_empty() {
        let title_block = TextBlock {
            width: node.label.width,
            height: node.label.height,
            lines: parts.title_lines,
        };
        let title_x = node.x + 10.0;
        let title_y = node.y + 4.0 + title_block.height / 2.0;
        out.push_str(&text_block_svg_with_font_size(
            title_x,
            title_y,
            &title_block,
            theme,
            config,
            theme.font_size,
            "start",
            node.style.text_color.as_deref(),
            false,
        ));
    }

    let footer_baseline = node.y + node.height - 12.0;
    if let Some(ticket) = parts.ticket.as_deref() {
        out.push_str(&format!(
            "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"start\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\" text-decoration=\"underline\"><tspan x=\"{:.2}\" dy=\"0.00\">{}</tspan></text>",
            node.x + 10.0,
            footer_baseline,
            normalize_font_family(&theme.font_family),
            theme.font_size,
            theme.primary_text_color,
            node.x + 10.0,
            escape_xml(ticket)
        ));
    }
    if let Some(assigned) = parts.assigned.as_deref() {
        let assigned_width =
            text_metrics::measure_text_width(assigned, theme.font_size, &theme.font_family)
                .unwrap_or_else(|| assigned.chars().count() as f32 * theme.font_size * 0.56);
        let assigned_x = node.x + node.width - assigned_width - 10.0;
        out.push_str(&format!(
            "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"start\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\"><tspan x=\"{:.2}\" dy=\"0.00\">{}</tspan></text>",
            assigned_x,
            footer_baseline,
            normalize_font_family(&theme.font_family),
            theme.font_size,
            theme.primary_text_color,
            assigned_x,
            escape_xml(assigned)
        ));
    }

    if let Some(color) = parts.priority.as_deref().and_then(kanban_priority_color) {
        let line_x = node.x + 2.0;
        out.push_str(&format!(
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke-width=\"4\" stroke=\"{}\"/>",
            line_x,
            node.y + 2.0,
            line_x,
            node.y + node.height - 2.0,
            color
        ));
    }

    out
}

struct KanbanRenderParts {
    title_lines: Vec<TextLine>,
    ticket: Option<String>,
    assigned: Option<String>,
    priority: Option<String>,
}

fn kanban_render_parts(label: &TextBlock) -> KanbanRenderParts {
    let mut title_lines = Vec::new();
    let mut ticket = None;
    let mut assigned = None;
    let mut priority = None;

    for line in &label.lines {
        let text = line.text();
        if is_kanban_metadata_line(&text) {
            for (key, value) in parse_kanban_metadata_pairs(&text) {
                match key.as_str() {
                    "ticket" => ticket = Some(value),
                    "assigned" => assigned = Some(value),
                    "priority" => priority = Some(value),
                    _ => {}
                }
            }
        } else {
            title_lines.push(line.clone());
        }
    }

    KanbanRenderParts {
        title_lines,
        ticket,
        assigned,
        priority,
    }
}

fn is_kanban_metadata_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("ticket:") || lower.contains("assigned:") || lower.contains("priority:")
}

fn parse_kanban_metadata_pairs(input: &str) -> Vec<(String, String)> {
    input
        .split([',', '\n'])
        .filter_map(|pair| {
            let (key, value) = pair.split_once(':')?;
            let key = key
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_ascii_lowercase();
            let value = value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            if key.is_empty() {
                None
            } else {
                Some((key, value))
            }
        })
        .collect()
}

fn kanban_priority_color(priority: &str) -> Option<&'static str> {
    match priority.to_ascii_lowercase().as_str() {
        "very high" => Some("red"),
        "high" => Some("orange"),
        "medium" => None,
        "low" => Some("blue"),
        "very low" => Some("lightblue"),
        _ => None,
    }
}

#[cfg(feature = "png")]
fn primary_font(fonts: &str) -> String {
    fonts
        .split(',')
        .map(|s| s.trim().trim_matches('"'))
        .find(|s| !s.is_empty())
        .unwrap_or("Inter")
        .to_string()
}

fn shape_svg(
    node: &crate::layout::NodeLayout,
    theme: &Theme,
    config: &LayoutConfig,
    diagram_kind: crate::ir::DiagramKind,
) -> String {
    if is_flowchart_icon_shape(node.shape) {
        let raw = flowchart_icon_shape_svg(node, theme);
        return if config.look == crate::ir::DiagramLook::HandDrawn {
            let seed = hand_drawn_seed(node.x, node.y, node.width, node.height);
            hand_drawn_path_jitter(&raw, 1.5, seed)
        } else {
            raw
        };
    }

    let mut raw = if diagram_kind == crate::ir::DiagramKind::Class {
        class_diagram_box_svg(node, theme)
            .unwrap_or_else(|| shape_svg_inner(node, theme, config, diagram_kind))
    } else {
        shape_svg_inner(node, theme, config, diagram_kind)
    };
    // If the node has an icon, render it inside the shape
    if let Some(icon_name) = &node.icon {
        if crate::icons::lookup_icon(icon_name).is_some() {
            let icon_size = node.height.min(node.width) * 0.5;
            let ix = node.x + (node.width - icon_size) / 2.0;
            let iy = node.y + (node.height - icon_size) / 2.0;
            let fill = node
                .style
                .stroke
                .as_ref()
                .unwrap_or(&theme.primary_border_color);
            raw.push_str(&crate::icons::render_icon_svg(
                icon_name, ix, iy, icon_size, fill,
            ));
        }
    }
    // If the node has an image, render it inside the shape
    if let Some(img_url) = &node.img {
        let iw = node.img_w.unwrap_or(60.0);
        let ih = node.img_h.unwrap_or(60.0);
        let ix = node.x + (node.width - iw) / 2.0;
        let iy = node.y + (node.height - ih) / 2.0;
        raw.push_str(&format!(
            "<image x=\"{ix:.2}\" y=\"{iy:.2}\" width=\"{iw:.2}\" height=\"{ih:.2}\" href=\"{img_url}\" preserveAspectRatio=\"xMidYMid meet\"/>",
        ));
    }
    if config.look == crate::ir::DiagramLook::HandDrawn {
        let seed = hand_drawn_seed(node.x, node.y, node.width, node.height);
        hand_drawn_path_jitter(&raw, 1.5, seed)
    } else {
        raw
    }
}

fn is_flowchart_icon_shape(shape: crate::ir::NodeShape) -> bool {
    matches!(
        shape,
        crate::ir::NodeShape::Icon
            | crate::ir::NodeShape::IconCircle
            | crate::ir::NodeShape::IconSquare
            | crate::ir::NodeShape::IconRounded
    )
}

fn flowchart_icon_visual_size(shape: crate::ir::NodeShape) -> f32 {
    match shape {
        crate::ir::NodeShape::IconCircle => {
            FLOWCHART_ICON_ASSET_SIZE * std::f32::consts::SQRT_2
                + FLOWCHART_ICON_CIRCLE_PADDING * 2.0
        }
        crate::ir::NodeShape::IconSquare | crate::ir::NodeShape::IconRounded => {
            FLOWCHART_ICON_ASSET_SIZE + FLOWCHART_ICON_SQUARE_PADDING * 2.0
        }
        crate::ir::NodeShape::Icon => FLOWCHART_ICON_ASSET_SIZE,
        _ => FLOWCHART_ICON_ASSET_SIZE,
    }
}

fn flowchart_icon_label_center_y(node: &crate::layout::NodeLayout) -> f32 {
    let icon_box = flowchart_icon_visual_size(node.shape);
    node.y
        + icon_box
        + FLOWCHART_ICON_LABEL_PADDING
        + FLOWCHART_ICON_LABEL_TOP_INSET
        + node.label.height / 2.0
}

fn treemap_shape_svg(node: &crate::layout::NodeLayout, theme: &Theme) -> String {
    let fill = node.style.fill.as_deref().unwrap_or(&theme.primary_color);
    let stroke = node.style.stroke.as_deref().unwrap_or(fill);
    let stroke_width =
        node.style
            .stroke_width
            .unwrap_or(if node.is_treemap_leaf { 3.0 } else { 2.0 });
    if node.is_treemap_leaf {
        format!(
            "<rect class=\"treemapLeaf\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" fill-opacity=\"0.3\" stroke=\"{}\" stroke-width=\"{}\"/>",
            node.x,
            node.y,
            node.width,
            node.height,
            escape_xml(fill),
            escape_xml(stroke),
            stroke_width
        )
    } else {
        format!(
            "<rect class=\"treemapSection\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" fill-opacity=\"0.6\" stroke=\"{}\" stroke-width=\"{}\" stroke-opacity=\"0.4\"/>",
            node.x,
            node.y,
            node.width,
            node.height,
            escape_xml(fill),
            escape_xml(stroke),
            stroke_width
        )
    }
}

fn treemap_section_label_svg(node: &crate::layout::NodeLayout, theme: &Theme) -> String {
    let label = first_text_line(&node.label);
    if label.trim().is_empty() {
        return String::new();
    }
    let fill = node
        .style
        .text_color
        .as_deref()
        .unwrap_or(theme.primary_text_color.as_str());
    let font_family = normalize_font_family(&theme.font_family);
    let y = node.y + 12.5;
    let mut out = format!(
        "<text class=\"treemapSectionLabel\" x=\"{:.2}\" y=\"{:.2}\" dominant-baseline=\"middle\" font-family=\"{}\" font-weight=\"bold\" style=\"dominant-baseline: middle; font-size: 12px; fill:{}; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;\">{}</text>",
        node.x + 6.0,
        y,
        font_family,
        escape_xml(fill),
        escape_xml(&label)
    );
    if let Some(sub_label) = node.sub_label.as_ref() {
        let value = first_text_line(sub_label);
        if !value.trim().is_empty() {
            out.push_str(&format!(
                "<text class=\"treemapSectionValue\" x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"end\" dominant-baseline=\"middle\" font-family=\"{}\" font-style=\"italic\" style=\"text-anchor: end; dominant-baseline: middle; font-size: 10px; fill:{}; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;\">{}</text>",
                node.x + node.width - 10.0,
                y,
                font_family,
                escape_xml(fill),
                escape_xml(&value)
            ));
        }
    }
    out
}

fn treemap_leaf_label_svg(node: &crate::layout::NodeLayout, theme: &Theme) -> String {
    let label = first_text_line(&node.label);
    if label.trim().is_empty() {
        return String::new();
    }
    let fill = node
        .style
        .text_color
        .as_deref()
        .unwrap_or(theme.primary_text_color.as_str());
    let base_fill = node.treemap_base_text_color.as_deref().unwrap_or(fill);
    let label_fill_style = if base_fill != fill {
        format!(
            "fill:{};fill:{} !important",
            escape_xml(base_fill),
            escape_xml(fill)
        )
    } else {
        format!("fill:{}", escape_xml(fill))
    };
    let value_fill_style = if base_fill != fill {
        format!("fill:{} !important", escape_xml(fill))
    } else {
        format!("fill:{}", escape_xml(fill))
    };
    let clip_id = format!("clip-my-svg-{}", node.id);
    let escaped_clip_id = escape_xml(&clip_id);
    let clip_width = (node.width - 4.0).max(0.0);
    let clip_height = (node.height - 4.0).max(0.0);
    let font_family = normalize_font_family(&theme.font_family);
    let center_x = node.x + node.width / 2.0;
    let center_y = node.y + node.height / 2.0;
    let mut out = format!(
        "<clipPath id=\"{escaped_clip_id}\"><rect x=\"{clip_x:.2}\" y=\"{clip_y:.2}\" width=\"{clip_width:.2}\" height=\"{clip_height:.2}\"/></clipPath>",
        clip_x = node.x,
        clip_y = node.y,
        clip_width = clip_width,
        clip_height = clip_height
    );

    let Some((label_font, value_font)) = treemap_leaf_font_sizes(node, &label, theme) else {
        out.push_str(&format!(
            "<text class=\"treemapLabel\" x=\"{:.2}\" y=\"{:.2}\" font-family=\"{}\" style=\"text-anchor: middle; dominant-baseline: middle; font-size: 38px; {}; display: none;\" clip-path=\"url(#{})\">{}</text>",
            center_x,
            center_y,
            font_family,
            label_fill_style,
            escaped_clip_id,
            escape_xml(&label)
        ));
        return out;
    };

    out.push_str(&format!(
        "<text class=\"treemapLabel\" x=\"{:.2}\" y=\"{:.2}\" font-family=\"{}\" style=\"text-anchor: middle; dominant-baseline: middle; font-size: {}px; {};\" clip-path=\"url(#{})\">{}</text>",
        center_x,
        center_y,
        font_family,
        label_font,
        label_fill_style,
        escaped_clip_id,
        escape_xml(&label),
    ));

    if let Some(sub_label) = node.sub_label.as_ref() {
        let value = first_text_line(sub_label);
        let value_y = center_y + label_font / 2.0 + 2.0;
        let available_width = (node.width - 8.0).max(0.0);
        let value_bottom = value_y + value_font;
        if !value.trim().is_empty() {
            let value_style_suffix = if treemap_estimated_text_width(&value, value_font, theme)
                <= available_width
                && value_bottom <= node.y + node.height - 4.0
            {
                String::new()
            } else {
                " display: none;".to_string()
            };
            out.push_str(&format!(
                "<text class=\"treemapValue\" x=\"{:.2}\" y=\"{:.2}\" font-family=\"{}\" style=\"text-anchor: middle; dominant-baseline: hanging; font-size: {}px; {};{}\" clip-path=\"url(#{})\">{}</text>",
                center_x,
                value_y,
                font_family,
                value_font,
                value_fill_style,
                value_style_suffix,
                escaped_clip_id,
                escape_xml(&value)
            ));
        }
    }

    out
}

fn treemap_leaf_font_sizes(
    node: &crate::layout::NodeLayout,
    label: &str,
    theme: &Theme,
) -> Option<(f32, f32)> {
    let available_width = node.width - 8.0;
    let available_height = node.height - 8.0;
    if available_width < 10.0 || available_height < 10.0 {
        return None;
    }

    let mut label_font = 38.0;
    while treemap_estimated_text_width(label, label_font, theme) > available_width
        && label_font > 8.0
    {
        label_font -= 1.0;
    }

    let mut value_font = treemap_value_font_size(label_font);
    while label_font + 2.0 + value_font > available_height && label_font > 8.0 {
        label_font -= 1.0;
        value_font = treemap_value_font_size(label_font);
    }

    if label_font < 8.0 || treemap_estimated_text_width(label, label_font, theme) > available_width
    {
        None
    } else {
        Some((label_font, value_font))
    }
}

fn treemap_value_font_size(label_font: f32) -> f32 {
    (label_font * 0.6).round().clamp(6.0, 28.0)
}

fn treemap_estimated_text_width(text: &str, font_size: f32, theme: &Theme) -> f32 {
    // Mermaid asks the browser's shaped SVG text node for getComputedTextLength().
    // Our ttf advance sum is a hair more conservative for some Trebuchet strings,
    // so keep the browser-style fitting loop from shrinking on sub-pixel drift.
    (text_metrics::get_computed_text_length(text, font_size, &theme.font_family) - 1.0).max(0.0)
}

fn first_text_line(label: &crate::layout::TextBlock) -> String {
    label
        .lines
        .first()
        .map(|line| line.text().into_owned())
        .unwrap_or_default()
}

fn class_diagram_box_svg(node: &crate::layout::NodeLayout, theme: &Theme) -> Option<String> {
    let (fill, stroke) = match node.shape {
        crate::ir::NodeShape::Rectangle => (
            node.style
                .fill
                .as_deref()
                .unwrap_or(&theme.primary_color)
                .to_string(),
            node.style
                .stroke
                .as_deref()
                .unwrap_or(&theme.primary_border_color)
                .to_string(),
        ),
        crate::ir::NodeShape::Note => (
            node.style
                .fill
                .as_deref()
                .unwrap_or(&theme.sequence_note_fill)
                .to_string(),
            node.style
                .stroke
                .as_deref()
                .unwrap_or(&theme.sequence_note_border)
                .to_string(),
        ),
        _ => return None,
    };
    let dash = node
        .style
        .stroke_dasharray
        .as_ref()
        .map(|value| format!(" stroke-dasharray=\"{}\"", value))
        .unwrap_or_else(|| " stroke-dasharray=\"0 0\"".to_string());
    let stroke_width = node
        .style
        .stroke_width
        .map(|value| value.to_string())
        .unwrap_or_else(|| "1.3".to_string());
    let fill_d = class_box_fill_path(node.x, node.y, node.width, node.height);
    let stroke_d = class_box_rough_rect_path(node.x, node.y, node.width, node.height);

    Some(format!(
        "<g class=\"basic label-container outer-path\"><path d=\"{fill_d}\" stroke=\"none\" stroke-width=\"0\" fill=\"{fill}\"/><path d=\"{stroke_d}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\" fill=\"none\"{dash}/></g>",
    ))
}

fn class_box_fill_path(x: f32, y: f32, width: f32, height: f32) -> String {
    let right = x + width;
    let bottom = y + height;
    format!("M{x:.2} {y:.2} L{right:.2} {y:.2} L{right:.2} {bottom:.2} L{x:.2} {bottom:.2}")
}

fn class_box_rough_rect_path(x: f32, y: f32, width: f32, height: f32) -> String {
    let right = x + width;
    let bottom = y + height;
    [
        class_box_rough_line_path(x, y, right, y),
        class_box_rough_line_path(right, y, right, bottom),
        class_box_rough_line_path(right, bottom, x, bottom),
        class_box_rough_line_path(x, bottom, x, y),
    ]
    .join(" ")
}

fn class_box_rough_line_path(x1: f32, y1: f32, x2: f32, y2: f32) -> String {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let c1a = (x1 + dx * 0.31, y1 + dy * 0.31);
    let c2a = (x1 + dx * 0.68, y1 + dy * 0.68);
    let c1b = (x1 + dx * 0.42, y1 + dy * 0.42);
    let c2b = (x1 + dx * 0.57, y1 + dy * 0.57);
    format!(
        "M{x1:.2} {y1:.2} C{:.2} {:.2}, {:.2} {:.2}, {x2:.2} {y2:.2} M{x1:.2} {y1:.2} C{:.2} {:.2}, {:.2} {:.2}, {x2:.2} {y2:.2}",
        c1a.0, c1a.1, c2a.0, c2a.1, c1b.0, c1b.1, c2b.0, c2b.1
    )
}

fn flowchart_icon_shape_svg(node: &crate::layout::NodeLayout, theme: &Theme) -> String {
    let fill = node.style.fill.as_ref().unwrap_or(&theme.primary_color);
    let stroke = match node.shape {
        crate::ir::NodeShape::IconCircle
        | crate::ir::NodeShape::IconSquare
        | crate::ir::NodeShape::IconRounded => fill,
        _ => node
            .style
            .stroke
            .as_ref()
            .unwrap_or(&theme.primary_border_color),
    };
    let icon_box = flowchart_icon_visual_size(node.shape);
    let icon_x = node.x + (node.width - FLOWCHART_ICON_ASSET_SIZE) / 2.0;
    let icon_y = node.y + (icon_box - FLOWCHART_ICON_ASSET_SIZE) / 2.0;
    let mut svg = String::new();

    match node.shape {
        crate::ir::NodeShape::IconCircle => {
            let cx = node.x + node.width / 2.0;
            let cy = node.y + icon_box / 2.0;
            let r = icon_box / 2.0;
            svg.push_str(&format!(
                "<circle cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{r:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.3\" stroke-linejoin=\"round\" stroke-linecap=\"round\"/>",
            ));
        }
        crate::ir::NodeShape::IconSquare | crate::ir::NodeShape::IconRounded => {
            let x = node.x + (node.width - icon_box) / 2.0;
            let y = node.y;
            let radius = if node.shape == crate::ir::NodeShape::IconRounded {
                icon_box * 0.1
            } else {
                0.0
            };
            svg.push_str(&format!(
                "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{icon_box:.2}\" height=\"{icon_box:.2}\" rx=\"{radius:.2}\" ry=\"{radius:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1\" stroke-linejoin=\"round\" stroke-linecap=\"round\"/>",
            ));
        }
        crate::ir::NodeShape::Icon => {}
        _ => {}
    }

    if node.icon.is_some() {
        svg.push_str(&crate::icons::render_unknown_icon_svg(
            icon_x,
            icon_y,
            FLOWCHART_ICON_ASSET_SIZE,
        ));
    }
    svg
}

fn left_inv_arrow_notch(width: f32, height: f32) -> f32 {
    (height * 0.5).min(width * 0.35)
}

fn flowchart_odd_notch(height: f32) -> f32 {
    height / 4.0
}

fn flowchart_curly_brace_radius(total_height: f32) -> f32 {
    if total_height <= 60.0 {
        5.0
    } else {
        total_height / 12.0
    }
}

fn flowchart_curly_circle_points(
    center_x: f32,
    center_y: f32,
    radius: f32,
    count: usize,
    start_angle: f32,
    end_angle: f32,
    invert: bool,
) -> Vec<(f32, f32)> {
    let start = start_angle.to_radians();
    let end = end_angle.to_radians();
    let step = (end - start) / (count.saturating_sub(1).max(1) as f32);
    (0..count)
        .map(|idx| {
            let angle = start + step * idx as f32;
            let x = center_x + radius * angle.cos();
            let y = center_y + radius * angle.sin();
            if invert { (-x, -y) } else { (x, y) }
        })
        .collect()
}

fn flowchart_curly_brace_left_points(x: f32, y: f32, width: f32, height: f32) -> Vec<(f32, f32)> {
    let radius = flowchart_curly_brace_radius(height);
    let body_width = (width - radius * 2.0).max(1.0);
    let body_height = (height - radius * 2.0).max(1.0);
    let center_x = x + width / 2.0;
    let center_y = y + height / 2.0;

    let mut points = Vec::new();
    points.extend(flowchart_curly_circle_points(
        body_width / 2.0,
        -body_height / 2.0,
        radius,
        30,
        -90.0,
        0.0,
        true,
    ));
    points.push((-body_width / 2.0 - radius, radius));
    points.extend(flowchart_curly_circle_points(
        body_width / 2.0 + radius * 2.0,
        -radius,
        radius,
        20,
        -180.0,
        -270.0,
        true,
    ));
    points.extend(flowchart_curly_circle_points(
        body_width / 2.0 + radius * 2.0,
        radius,
        radius,
        20,
        -90.0,
        -180.0,
        true,
    ));
    points.push((-body_width / 2.0 - radius, -body_height / 2.0));
    points.extend(flowchart_curly_circle_points(
        body_width / 2.0,
        body_height / 2.0,
        radius,
        20,
        0.0,
        90.0,
        true,
    ));

    points
        .into_iter()
        .map(|(px, py)| (center_x + px + radius, center_y + py))
        .collect()
}

fn flowchart_curly_brace_right_points(x: f32, y: f32, width: f32, height: f32) -> Vec<(f32, f32)> {
    let radius = flowchart_curly_brace_radius(height);
    let body_width = (width - radius * 2.0).max(1.0);
    let body_height = (height - radius * 2.0).max(1.0);
    let center_x = x + width / 2.0;
    let center_y = y + height / 2.0;

    let mut points = Vec::new();
    points.extend(flowchart_curly_circle_points(
        body_width / 2.0,
        -body_height / 2.0,
        radius,
        20,
        -90.0,
        0.0,
        false,
    ));
    points.push((body_width / 2.0 + radius, -radius));
    points.extend(flowchart_curly_circle_points(
        body_width / 2.0 + radius * 2.0,
        -radius,
        radius,
        20,
        -180.0,
        -270.0,
        false,
    ));
    points.extend(flowchart_curly_circle_points(
        body_width / 2.0 + radius * 2.0,
        radius,
        radius,
        20,
        -90.0,
        -180.0,
        false,
    ));
    points.push((body_width / 2.0 + radius, body_height / 2.0));
    points.extend(flowchart_curly_circle_points(
        body_width / 2.0,
        body_height / 2.0,
        radius,
        20,
        0.0,
        90.0,
        false,
    ));

    points
        .into_iter()
        .map(|(px, py)| (center_x + px - radius, center_y + py))
        .collect()
}

fn flowchart_curly_braces_points(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> (Vec<(f32, f32)>, Vec<(f32, f32)>) {
    let radius = flowchart_curly_brace_radius(height);
    let body_width = (width - radius * 2.5).max(1.0);
    let body_height = (height - radius * 2.0).max(1.0);
    let center_x = x + width / 2.0;
    let center_y = y + height / 2.0;
    let group_shift = radius - radius / 4.0;
    let to_absolute = |(px, py): (f32, f32)| (center_x + px + group_shift, center_y + py);

    let mut left_points = Vec::new();
    left_points.extend(flowchart_curly_circle_points(
        body_width / 2.0,
        -body_height / 2.0,
        radius,
        30,
        -90.0,
        0.0,
        true,
    ));
    left_points.push((-body_width / 2.0 - radius, radius));
    left_points.extend(flowchart_curly_circle_points(
        body_width / 2.0 + radius * 2.0,
        -radius,
        radius,
        20,
        -180.0,
        -270.0,
        true,
    ));
    left_points.extend(flowchart_curly_circle_points(
        body_width / 2.0 + radius * 2.0,
        radius,
        radius,
        20,
        -90.0,
        -180.0,
        true,
    ));
    left_points.push((-body_width / 2.0 - radius, -body_height / 2.0));
    left_points.extend(flowchart_curly_circle_points(
        body_width / 2.0,
        body_height / 2.0,
        radius,
        20,
        0.0,
        90.0,
        true,
    ));

    let mut right_points = Vec::new();
    right_points.extend(flowchart_curly_circle_points(
        -body_width / 2.0 + radius + radius / 2.0,
        -body_height / 2.0,
        radius,
        20,
        -90.0,
        -180.0,
        true,
    ));
    right_points.push((body_width / 2.0 - radius / 2.0, radius));
    right_points.extend(flowchart_curly_circle_points(
        -body_width / 2.0 - radius / 2.0,
        -radius,
        radius,
        20,
        0.0,
        90.0,
        true,
    ));
    right_points.extend(flowchart_curly_circle_points(
        -body_width / 2.0 - radius / 2.0,
        radius,
        radius,
        20,
        -90.0,
        0.0,
        true,
    ));
    right_points.push((body_width / 2.0 - radius / 2.0, -radius));
    right_points.extend(flowchart_curly_circle_points(
        -body_width / 2.0 + radius + radius / 2.0,
        body_height / 2.0,
        radius,
        30,
        -180.0,
        -270.0,
        true,
    ));

    (
        left_points.into_iter().map(to_absolute).collect(),
        right_points.into_iter().map(to_absolute).collect(),
    )
}

fn block_arrow_points(shape: crate::ir::NodeShape, width: f32, height: f32) -> Vec<(f32, f32)> {
    let midpoint = height / 2.0;
    let padding = 4.0;
    let raw = match shape {
        crate::ir::NodeShape::BlockArrowAll => vec![
            (0.0, 0.0),
            (midpoint, 0.0),
            (width / 2.0, 2.0 * padding),
            (width - midpoint, 0.0),
            (width, 0.0),
            (width, -height / 3.0),
            (width + 2.0 * padding, -height / 2.0),
            (width, -2.0 * height / 3.0),
            (width, -height),
            (width - midpoint, -height),
            (width / 2.0, -height - 2.0 * padding),
            (midpoint, -height),
            (0.0, -height),
            (0.0, -2.0 * height / 3.0),
            (-2.0 * padding, -height / 2.0),
            (0.0, -height / 3.0),
        ],
        crate::ir::NodeShape::BlockArrowXUp => vec![
            (midpoint, 0.0),
            (width - midpoint, 0.0),
            (width, -height / 2.0),
            (width - midpoint, -height),
            (midpoint, -height),
            (0.0, -height / 2.0),
        ],
        crate::ir::NodeShape::BlockArrowXDown => vec![
            (0.0, 0.0),
            (midpoint, -height),
            (width - midpoint, -height),
            (width, 0.0),
        ],
        crate::ir::NodeShape::BlockArrowYRight => vec![
            (0.0, 0.0),
            (width, -midpoint),
            (width, -height + midpoint),
            (0.0, -height),
        ],
        crate::ir::NodeShape::BlockArrowYLeft => vec![
            (width, 0.0),
            (0.0, -midpoint),
            (0.0, -height + midpoint),
            (width, -height),
        ],
        crate::ir::NodeShape::BlockArrowX => vec![
            (midpoint, 0.0),
            (midpoint, -padding),
            (width - midpoint, -padding),
            (width - midpoint, 0.0),
            (width, -height / 2.0),
            (width - midpoint, -height),
            (width - midpoint, -height + padding),
            (midpoint, -height + padding),
            (midpoint, -height),
            (0.0, -height / 2.0),
        ],
        crate::ir::NodeShape::BlockArrowY => vec![
            (width / 2.0, 0.0),
            (0.0, -padding),
            (midpoint, -padding),
            (midpoint, -height + padding),
            (0.0, -height + padding),
            (width / 2.0, -height),
            (width, -height + padding),
            (width - midpoint, -height + padding),
            (width - midpoint, -padding),
            (width, -padding),
        ],
        crate::ir::NodeShape::BlockArrowRightUp => {
            vec![(0.0, 0.0), (width, -midpoint), (0.0, -height)]
        }
        crate::ir::NodeShape::BlockArrowRightDown => {
            vec![(0.0, 0.0), (width, 0.0), (0.0, -height)]
        }
        crate::ir::NodeShape::BlockArrowLeftUp => {
            vec![(width, 0.0), (0.0, -midpoint), (width, -height)]
        }
        crate::ir::NodeShape::BlockArrowLeftDown => {
            vec![(width, 0.0), (0.0, 0.0), (width, -height)]
        }
        crate::ir::NodeShape::BlockArrowRight => vec![
            (midpoint, -padding),
            (midpoint, -padding),
            (width - midpoint, -padding),
            (width - midpoint, 0.0),
            (width, -height / 2.0),
            (width - midpoint, -height),
            (width - midpoint, -height + padding),
            (midpoint, -height + padding),
            (midpoint, -height + padding),
        ],
        crate::ir::NodeShape::BlockArrowLeft => vec![
            (midpoint, 0.0),
            (midpoint, -padding),
            (width - midpoint, -padding),
            (width - midpoint, -height + padding),
            (midpoint, -height + padding),
            (midpoint, -height),
            (0.0, -height / 2.0),
        ],
        crate::ir::NodeShape::BlockArrowUp => vec![
            (midpoint, -padding),
            (midpoint, -height + padding),
            (0.0, -height + padding),
            (width / 2.0, -height),
            (width, -height + padding),
            (width - midpoint, -height + padding),
            (width - midpoint, -padding),
        ],
        crate::ir::NodeShape::BlockArrowDown => vec![
            (width / 2.0, 0.0),
            (0.0, -padding),
            (midpoint, -padding),
            (midpoint, -height + padding),
            (width - midpoint, -height + padding),
            (width - midpoint, -padding),
            (width, -padding),
        ],
        _ => Vec::new(),
    };

    raw.into_iter().map(|(px, py)| (px, py + height)).collect()
}

fn shape_svg_inner(
    node: &crate::layout::NodeLayout,
    theme: &Theme,
    config: &LayoutConfig,
    diagram_kind: crate::ir::DiagramKind,
) -> String {
    let stroke = node
        .style
        .stroke
        .as_ref()
        .unwrap_or(&theme.primary_border_color);
    let fill = node.style.fill.as_ref().unwrap_or(&theme.primary_color);
    let dash = node
        .style
        .stroke_dasharray
        .as_ref()
        .map(|value| format!(" stroke-dasharray=\"{}\"", value))
        .unwrap_or_default();
    let join = " stroke-linejoin=\"round\" stroke-linecap=\"round\"";
    let x = node.x;
    let y = node.y;
    let w = node.width;
    let h = node.height;
    match node.shape {
        crate::ir::NodeShape::Rectangle => format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"0\" ry=\"0\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
            x,
            y,
            w,
            h,
            fill,
            stroke,
            node.style.stroke_width.unwrap_or(1.0)
        ),
        crate::ir::NodeShape::Text => format!(
            "<rect class=\"text\" x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\" rx=\"0\" ry=\"0\" fill=\"none\" stroke=\"none\" stroke-width=\"0\"/>",
        ),
        crate::ir::NodeShape::Note => {
            let note_fill = node
                .style
                .fill
                .as_deref()
                .unwrap_or(&theme.sequence_note_fill);
            let note_stroke = node
                .style
                .stroke
                .as_deref()
                .unwrap_or(&theme.sequence_note_border);
            format!(
                "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\" rx=\"0\" ry=\"0\" fill=\"{note_fill}\" stroke=\"{note_stroke}\" stroke-width=\"{}\"{dash}{join}/>",
                node.style.stroke_width.unwrap_or(1.0)
            )
        }
        crate::ir::NodeShape::ForkJoin => {
            let (render_y, render_h) = if diagram_kind == crate::ir::DiagramKind::State
                && h > STATE_FORK_JOIN_RENDER_HEIGHT
            {
                (
                    y + (h - STATE_FORK_JOIN_RENDER_HEIGHT) * 0.5,
                    STATE_FORK_JOIN_RENDER_HEIGHT,
                )
            } else {
                (y, h)
            };
            let (fork_fill, fork_stroke) = if diagram_kind == crate::ir::DiagramKind::State {
                (
                    theme.primary_text_color.as_str(),
                    theme.primary_text_color.as_str(),
                )
            } else {
                (fill.as_str(), stroke.as_str())
            };
            format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"2\" ry=\"2\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                x,
                render_y,
                w,
                render_h,
                fork_fill,
                fork_stroke,
                node.style.stroke_width.unwrap_or(1.0)
            )
        }
        crate::ir::NodeShape::ActorBox => format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"3\" ry=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
            x,
            y,
            w,
            h,
            fill,
            stroke,
            node.style.stroke_width.unwrap_or(1.0)
        ),
        crate::ir::NodeShape::Diamond => {
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            let points = format!(
                "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                cx,
                y,
                x + w,
                cy,
                cx,
                y + h,
                x,
                cy
            );
            format!(
                "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                points,
                fill,
                stroke,
                node.style.stroke_width.unwrap_or(1.0)
            )
        }
        crate::ir::NodeShape::Circle | crate::ir::NodeShape::DoubleCircle => {
            let label_empty = node
                .label
                .lines
                .iter()
                .all(|line| line.text().trim().is_empty());
            let is_state_start = node.id.starts_with("__start_");
            let is_state_end = node.id.starts_with("__end_");
            let (circle_fill, circle_stroke) = if is_state_start {
                (theme.line_color.as_str(), theme.line_color.as_str())
            } else if is_state_end {
                (
                    theme.primary_border_color.as_str(),
                    theme.primary_border_color.as_str(),
                )
            } else if label_empty {
                if node.shape == crate::ir::NodeShape::Circle {
                    (
                        theme.primary_text_color.as_str(),
                        theme.primary_text_color.as_str(),
                    )
                } else {
                    (
                        theme.primary_border_color.as_str(),
                        theme.background.as_str(),
                    )
                }
            } else {
                (fill.as_str(), stroke.as_str())
            };
            let stroke_width = node.style.stroke_width.unwrap_or(1.0);
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            let r = (w.min(h)) / 2.0;
            let mut svg = format!(
                "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                cx, cy, r, circle_fill, circle_stroke, stroke_width
            );
            if node.shape == crate::ir::NodeShape::DoubleCircle {
                let inner_gap = if label_empty || is_state_end {
                    4.0
                } else {
                    5.0
                };
                let r2 = r - inner_gap;
                if r2 > 0.0 {
                    let inner_fill = if label_empty || is_state_end {
                        theme.background.as_str()
                    } else {
                        "none"
                    };
                    let inner_stroke = if label_empty || is_state_end {
                        theme.background.as_str()
                    } else {
                        circle_stroke
                    };
                    let inner_stroke_width = if label_empty || is_state_end {
                        1.2
                    } else {
                        1.0
                    };
                    svg.push_str(&format!(
                        "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{join}/>",
                        cx, cy, r2, inner_fill, inner_stroke, inner_stroke_width
                    ));
                }
            }
            svg
        }
        crate::ir::NodeShape::Stadium => format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"{:.2}\" ry=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
            x,
            y,
            w,
            h,
            h / 2.0,
            h / 2.0,
            fill,
            stroke,
            node.style.stroke_width.unwrap_or(1.0)
        ),
        crate::ir::NodeShape::RoundRect => format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"5\" ry=\"5\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
            x,
            y,
            w,
            h,
            fill,
            stroke,
            node.style.stroke_width.unwrap_or(1.0)
        ),
        crate::ir::NodeShape::Cylinder => {
            let stroke_width = node.style.stroke_width.unwrap_or(1.0);
            let rx = w / 2.0;
            let ry = rx / (2.5 + w / 50.0);
            let y_top = y + ry;
            let y_bot = y + h - ry;
            let x_right = x + w;
            let cylinder_d = format!(
                "M {x:.2},{y_top:.2} A {rx:.2},{ry:.2} 0 0,0 {x_right:.2},{y_top:.2} A {rx:.2},{ry:.2} 0 0,0 {x:.2},{y_top:.2} L {x:.2},{y_bot:.2} A {rx:.2},{ry:.2} 0 0,0 {x_right:.2},{y_bot:.2} L {x_right:.2},{y_top:.2}"
            );
            format!(
                "<path d=\"{cylinder_d}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{dash}{join}/>"
            )
        }
        crate::ir::NodeShape::Subroutine => {
            let stroke_width = node.style.stroke_width.unwrap_or(1.0);
            let inset = 6.0;
            let mut svg = format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                x, y, w, h, fill, stroke, stroke_width
            );
            let y1 = y + 2.0;
            let y2 = y + h - 2.0;
            let x1 = x + inset;
            let x2 = x + w - inset;
            svg.push_str(&format!(
                "<line x1=\"{x1:.2}\" y1=\"{y1:.2}\" x2=\"{x1:.2}\" y2=\"{y2:.2}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{join}/>"
            ));
            svg.push_str(&format!(
                "<line x1=\"{x2:.2}\" y1=\"{y1:.2}\" x2=\"{x2:.2}\" y2=\"{y2:.2}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{join}/>"
            ));
            svg
        }
        crate::ir::NodeShape::Hexagon => {
            let notch = (h / 4.0).min(w / 2.0);
            let x1 = x + notch;
            let x2 = x + w - notch;
            let y_mid = y + h / 2.0;
            let points = format!(
                "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                x1,
                y,
                x2,
                y,
                x + w,
                y_mid,
                x2,
                y + h,
                x1,
                y + h,
                x,
                y_mid
            );
            format!(
                "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                points,
                fill,
                stroke,
                node.style.stroke_width.unwrap_or(1.0)
            )
        }
        crate::ir::NodeShape::Parallelogram | crate::ir::NodeShape::ParallelogramAlt => {
            let offset = h / 2.0;
            let (p1, p2, p3, p4) = if node.shape == crate::ir::NodeShape::Parallelogram {
                (
                    (x + offset, y),
                    (x + w, y),
                    (x + w - offset, y + h),
                    (x, y + h),
                )
            } else {
                (
                    (x, y),
                    (x + w - offset, y),
                    (x + w, y + h),
                    (x + offset, y + h),
                )
            };
            let points = format!(
                "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                p1.0, p1.1, p2.0, p2.1, p3.0, p3.1, p4.0, p4.1
            );
            format!(
                "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                points,
                fill,
                stroke,
                node.style.stroke_width.unwrap_or(1.0)
            )
        }
        crate::ir::NodeShape::Trapezoid | crate::ir::NodeShape::TrapezoidAlt => {
            let offset = h / 2.0;
            let (p1, p2, p3, p4) = if node.shape == crate::ir::NodeShape::Trapezoid {
                (
                    (x + offset, y),
                    (x + w - offset, y),
                    (x + w, y + h),
                    (x, y + h),
                )
            } else {
                (
                    (x, y),
                    (x + w, y),
                    (x + w - offset, y + h),
                    (x + offset, y + h),
                )
            };
            let points = format!(
                "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                p1.0, p1.1, p2.0, p2.1, p3.0, p3.1, p4.0, p4.1
            );
            format!(
                "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                points,
                fill,
                stroke,
                node.style.stroke_width.unwrap_or(1.0)
            )
        }
        crate::ir::NodeShape::BlockArrowRight
        | crate::ir::NodeShape::BlockArrowLeft
        | crate::ir::NodeShape::BlockArrowUp
        | crate::ir::NodeShape::BlockArrowDown
        | crate::ir::NodeShape::BlockArrowX
        | crate::ir::NodeShape::BlockArrowY
        | crate::ir::NodeShape::BlockArrowXUp
        | crate::ir::NodeShape::BlockArrowXDown
        | crate::ir::NodeShape::BlockArrowYRight
        | crate::ir::NodeShape::BlockArrowYLeft
        | crate::ir::NodeShape::BlockArrowRightUp
        | crate::ir::NodeShape::BlockArrowRightDown
        | crate::ir::NodeShape::BlockArrowLeftUp
        | crate::ir::NodeShape::BlockArrowLeftDown
        | crate::ir::NodeShape::BlockArrowAll => {
            let points = block_arrow_points(node.shape, w, h)
                .into_iter()
                .map(|(px, py)| format!("{:.2},{:.2}", x + px, y + py))
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                points,
                fill,
                stroke,
                node.style.stroke_width.unwrap_or(1.0)
            )
        }
        crate::ir::NodeShape::Asymmetric => {
            let notch = left_inv_arrow_notch(w, h);
            let points = format!(
                "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                x,
                y,
                x + w,
                y,
                x + w,
                y + h,
                x,
                y + h,
                x + notch,
                y + h / 2.0
            );
            format!(
                "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                points,
                fill,
                stroke,
                node.style.stroke_width.unwrap_or(1.0)
            )
        }
        crate::ir::NodeShape::MindmapDefault => {
            let rd = config
                .mindmap
                .default_corner_radius
                .max(theme.font_size * 0.55)
                .max(4.0);
            let inner_h = (h - 2.0 * rd).max(0.0);
            let inner_w = (w - 2.0 * rd).max(0.0);
            let rect_path = format!(
                "M{:.2} {:.2} v{:.2} q0,-{rd:.2} {rd:.2},-{rd:.2} h{:.2} q{rd:.2},0 {rd:.2},{rd:.2} v{:.2} q0,{rd:.2} -{rd:.2},{rd:.2} h{:.2} q-{rd:.2},0 -{rd:.2},-{rd:.2} Z",
                x,
                y + h - rd,
                -inner_h,
                inner_w,
                inner_h,
                -inner_w
            );
            let stroke_width = node.style.stroke_width.unwrap_or(1.0);
            let mut svg = format!(
                "<path d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
                rect_path, fill, stroke, stroke_width
            );
            let line_color = node.style.line_color.as_ref().unwrap_or(stroke);
            let line_width = config.mindmap.divider_line_width;
            let line_y = y + h - stroke_width.max(0.8);
            svg.push_str(&format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-opacity=\"0.35\"/>",
                x,
                line_y,
                x + w,
                line_y,
                line_color,
                line_width
            ));
            svg
        }
        crate::ir::NodeShape::Document => {
            // Mermaid's `doc` is a wave-edged rectangle: flat top/sides,
            // with a sine wave along the bottom.
            let sw = node.style.stroke_width.unwrap_or(1.3);
            let wave_amplitude = flowchart_wave_document_amplitude(h);
            let body_height = h - wave_amplitude * 2.0;
            let wave_baseline = y + body_height + wave_amplitude;
            let mut d = format!("M {x:.2} {wave_baseline:.2}");
            let steps = 50;
            let cycles = 0.8;
            let frequency = 2.0 * std::f32::consts::PI * cycles / w;
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let px = x + t * w;
                let py = wave_baseline + wave_amplitude * (frequency * (px - x)).sin();
                d.push_str(&format!(" L {px:.2} {py:.2}"));
            }
            d.push_str(&format!(" L {:.2} {y:.2} L {x:.2} {y:.2} Z", x + w));
            format!(
                "<path d=\"{d}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>"
            )
        }
        crate::ir::NodeShape::StackedDocument => {
            // Two offset curved-trapezoid document shapes.
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let off = 4.0;
            let bw = w - off;
            let bh = h - off;
            let doc_path = |sx: f32, sy: f32, dw: f32, dh: f32| -> String {
                let radius = dh / 2.0;
                let rw = (dw - radius).max(0.0);
                let tw = dh / 4.0;
                format!(
                    "<path d=\"M {rx:.2} {sy:.2} L {lx:.2} {sy:.2} L {sx:.2} {my:.2} \
                     L {lx:.2} {by:.2} L {rx:.2} {by:.2} \
                     A {r:.2} {r:.2} 0 0 0 {rx:.2} {sy:.2} Z\" \
                     fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>",
                    rx = sx + rw,
                    lx = sx + tw,
                    my = sy + dh / 2.0,
                    by = sy + dh,
                    r = radius,
                )
            };
            let back = doc_path(x + off, y, bw, bh);
            let front = doc_path(x, y + off, bw, bh);
            format!("{back}{front}")
        }
        crate::ir::NodeShape::NotchRect => {
            // Rectangle with a notched (cut) top-left corner.
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let notch = (w.min(h) * 0.15).min(12.0);
            format!(
                "<path d=\"M {x1:.2} {y:.2} h {w1:.2} v {h:.2} h {nw:.2} v {nh:.2} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>",
                x1 = x + notch,
                w1 = w - notch,
                nw = -(w),
                nh = -(h - notch),
            )
        }
        crate::ir::NodeShape::TagRect => {
            // Rectangle with a folded corner (document tag) in the
            // bottom-right, matching mermaid-js tag-rect shape.
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let fold = (w * 0.15).min(h * 0.3).max(6.0);
            // Outer rectangle (with fold cutout at bottom-right).
            let rect_path = format!(
                "M {x:.2} {y:.2} h {w:.2} v {hf:.2} l {nf:.2} {f:.2} h {nwf:.2} Z",
                hf = h - fold,
                nf = -fold,
                f = fold,
                nwf = -(w - fold),
            );
            // Fold triangle overlay.
            let fx = x + w - fold;
            let fy = y + h - fold;
            let fold_path = format!(
                "M {fx:.2} {fy2:.2} h {f:.2} v {nf:.2} Z",
                fy2 = y + h,
                f = fold,
                nf = -fold,
            );
            format!(
                "<path d=\"{rect_path}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>\
                 <path d=\"{fold_path}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\" stroke-linejoin=\"round\"/>",
            )
        }
        crate::ir::NodeShape::Flag => {
            // Flag shape: rectangle with a notched right side (pennant).
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let indent = w * 0.18;
            format!(
                "<path d=\"M {x:.2} {y:.2} h {w:.2} l {ni:.2} {hh:.2} l {ind:.2} {hh:.2} h {nw:.2} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>",
                ni = -indent,
                hh = h / 2.0,
                ind = indent,
                nw = -w,
            )
        }
        crate::ir::NodeShape::Hourglass => {
            // Hourglass: two triangles meeting at the center.
            let sw = node.style.stroke_width.unwrap_or(1.3);
            format!(
                "<polygon points=\"{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>",
                x,
                y,
                x + w,
                y,
                x,
                y + h,
                x + w,
                y + h,
            )
        }
        crate::ir::NodeShape::LightningBolt => {
            let sw = node.style.stroke_width.unwrap_or(1.3);
            let gap = 7.0;
            let bolt_height = h / 2.0;
            format!(
                "<polygon points=\"{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>",
                x + w,
                y,
                x,
                y + bolt_height + gap / 2.0,
                x + w - 2.0 * gap,
                y + bolt_height + gap / 2.0,
                x,
                y + 2.0 * bolt_height,
                x + w,
                y + bolt_height - gap / 2.0,
                x + 2.0 * gap,
                y + bolt_height - gap / 2.0,
            )
        }
        crate::ir::NodeShape::WindowPane => {
            let sw = node.style.stroke_width.unwrap_or(1.3);
            let mx = x + FLOWCHART_WINDOW_PANE_OFFSET;
            let my = y + FLOWCHART_WINDOW_PANE_OFFSET;
            let rect = format!(
                "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\" rx=\"0\" ry=\"0\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>"
            );
            let vert = format!(
                "<line x1=\"{mx:.2}\" y1=\"{y:.2}\" x2=\"{mx:.2}\" y2=\"{y2:.2}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}/>",
                y2 = y + h,
            );
            let horiz = format!(
                "<line x1=\"{x:.2}\" y1=\"{my:.2}\" x2=\"{x2:.2}\" y2=\"{my:.2}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}/>",
                x2 = x + w,
            );
            format!("{rect}{vert}{horiz}")
        }
        crate::ir::NodeShape::LeanRight => {
            // Lean right: parallelogram leaning right.
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let offset = if diagram_kind == crate::ir::DiagramKind::Flowchart {
                h / 2.0
            } else {
                w * 0.15
            };
            let points = format!(
                "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                x + offset,
                y,
                x + w,
                y,
                x + w - offset,
                y + h,
                x,
                y + h
            );
            format!(
                "<polygon points=\"{points}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>"
            )
        }
        crate::ir::NodeShape::LeanLeft => {
            // Lean left: parallelogram leaning left.
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let offset = if diagram_kind == crate::ir::DiagramKind::Flowchart {
                h / 2.0
            } else {
                w * 0.15
            };
            let points = format!(
                "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                x,
                y,
                x + w - offset,
                y,
                x + w,
                y + h,
                x + offset,
                y + h
            );
            format!(
                "<polygon points=\"{points}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>"
            )
        }
        crate::ir::NodeShape::LinedCylinder => {
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let rx = w / 2.0;
            let ry = flowchart_cylinder_ry(w);
            let body_h = (h - ry * 2.0).max(0.0);
            let outer_offset = body_h * 0.1;
            format!(
                "<path d=\"M{x:.2},{y1:.2} a{rx:.2},{ry:.2} 0,0,0 {w:.2},0 a{rx:.2},{ry:.2} 0,0,0 {neg_w:.2},0 l0,{body_h:.2} a{rx:.2},{ry:.2} 0,0,0 {w:.2},0 l0,{neg_body_h:.2} M{x:.2},{inner_y:.2} a{rx:.2},{ry:.2} 0,0,0 {w:.2},0\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>",
                y1 = y + ry,
                neg_w = -w,
                neg_body_h = -body_h,
                inner_y = y + ry + outer_offset,
            )
        }
        crate::ir::NodeShape::Comment => {
            // Callout comment: rectangle with a folded corner.
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let fold = (w.min(h) * 0.15).min(12.0);
            format!(
                "<path d=\"M {x:.2} {y:.2} h {w1:.2} v {fold:.2} h {nfold:.2} v {h1:.2} h {nw:.2} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>",
                w1 = w - fold,
                nfold = -fold,
                h1 = h - fold,
                nw = -(w - fold) - fold,
            )
        }
        crate::ir::NodeShape::OddShape => {
            // Mermaid's `odd` shape is `rect_left_inv_arrow`: a rectangle
            // with an inverted arrow notch on the left.
            let sw = node.style.stroke_width.unwrap_or(1.3);
            let notch = flowchart_odd_notch(h);
            let points = format!(
                "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                x,
                y,
                x + w,
                y,
                x + w,
                y + h,
                x,
                y + h,
                x + notch,
                y + h / 2.0,
            );
            format!(
                "<polygon points=\"{points}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>"
            )
        }
        crate::ir::NodeShape::BraceLeft => {
            let sw = node.style.stroke_width.unwrap_or(1.3);
            let d = points_to_path(&flowchart_curly_brace_left_points(x, y, w, h));
            format!(
                "<path d=\"{d}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>"
            )
        }
        crate::ir::NodeShape::BraceRight => {
            let sw = node.style.stroke_width.unwrap_or(1.3);
            let d = points_to_path(&flowchart_curly_brace_right_points(x, y, w, h));
            format!(
                "<path d=\"{d}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>"
            )
        }
        crate::ir::NodeShape::BraceBoth => {
            let sw = node.style.stroke_width.unwrap_or(1.3);
            let (left_points, right_points) = flowchart_curly_braces_points(x, y, w, h);
            let left_d = points_to_path(&left_points);
            let right_d = points_to_path(&right_points);
            format!(
                "<path d=\"{left_d}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/><path d=\"{right_d}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>"
            )
        }
        crate::ir::NodeShape::LinedDocument => {
            let sw = node.style.stroke_width.unwrap_or(1.3);
            let wave_amplitude = flowchart_wave_document_amplitude(h);
            let body_height = h - wave_amplitude * 2.0;
            let body_width = w / 1.1;
            let side_overhang = (w - body_width) / 2.0;
            let wave_baseline = y + body_height + wave_amplitude;
            let x_right = x + w;
            let mut d = format!("M {x:.2} {y:.2} L {x:.2} {wave_baseline:.2}");
            let steps = 50;
            let cycles = 0.8;
            let frequency = 2.0 * std::f32::consts::PI * cycles / w.max(f32::EPSILON);
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let px = x + t * w;
                let py = wave_baseline + wave_amplitude * (frequency * (px - x)).sin();
                d.push_str(&format!(" L {px:.2} {py:.2}"));
            }
            d.push_str(&format!(" L {x_right:.2} {y:.2} L {x:.2} {y:.2} Z"));

            let line_x = x + side_overhang;
            let line_bottom = y + (body_height + wave_amplitude) * 1.05;
            format!(
                "<path d=\"{d}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>\
                 <line x1=\"{line_x:.2}\" y1=\"{y:.2}\" x2=\"{line_x:.2}\" y2=\"{line_bottom:.2}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}/>"
            )
        }
        crate::ir::NodeShape::TagDocument => {
            let sw = node.style.stroke_width.unwrap_or(1.3);
            let wave_amplitude = flowchart_wave_document_amplitude(h);
            let body_height = h - wave_amplitude * 2.0;
            let body_width = w / 1.1;
            let final_height = body_height + wave_amplitude;
            let center_x = x + w / 2.0;
            let center_y = y + h / 2.0;
            let to_abs = |px: f32, py: f32| -> (f32, f32) {
                (center_x + px, center_y + py - wave_amplitude / 2.0)
            };

            let left = -body_width * 0.55;
            let right = body_width * 0.55;
            let top = -final_height / 2.0;
            let baseline = final_height / 2.0;
            let steps = 50;
            let cycles = 0.8;
            let frequency = 2.0 * std::f32::consts::PI * cycles / (right - left).max(f32::EPSILON);
            let (sx, sy) = to_abs(left, baseline);
            let mut body_path = format!("M {sx:.2} {sy:.2}");
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let px = left + t * (right - left);
                let py = baseline + wave_amplitude * (frequency * (px - left)).sin();
                let (ax, ay) = to_abs(px, py);
                body_path.push_str(&format!(" L {ax:.2} {ay:.2}"));
            }
            let (right_top_x, right_top_y) = to_abs(right, top);
            let (left_top_x, left_top_y) = to_abs(left, top);
            body_path.push_str(&format!(
                " L {right_top_x:.2} {right_top_y:.2} L {left_top_x:.2} {left_top_y:.2} Z"
            ));

            let tag_width = body_width * 0.2;
            let tag_height = body_height * 0.2;
            let tag_x = -body_width / 2.0 + body_width * 0.05;
            let tag_y = -final_height / 2.0 - tag_height * 0.4;
            let tag_bottom = tag_y + body_height;
            let tag_left = tag_x + body_width - tag_width;
            let tag_right = tag_x + body_width;
            let tag_sine_start_y = tag_bottom * 1.25;
            let tag_sine_end_y = tag_bottom * 1.3;
            let tag_delta_x = tag_left - tag_right;
            let tag_delta_y = tag_sine_end_y - tag_sine_start_y;
            let tag_frequency = 2.0 * std::f32::consts::PI * 0.5 / tag_delta_x;
            let tag_mid_y = tag_sine_start_y + tag_delta_y / 2.0;
            let (p1x, p1y) = to_abs(tag_left, tag_bottom * 1.3);
            let (p2x, p2y) = to_abs(tag_right, tag_bottom - tag_height);
            let (p3x, p3y) = to_abs(tag_right, tag_bottom * 0.9);
            let mut tag_path =
                format!("M {p1x:.2} {p1y:.2} L {p2x:.2} {p2y:.2} L {p3x:.2} {p3y:.2}");
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let px = tag_right + t * tag_delta_x;
                let py = tag_mid_y - body_height * 0.02 * (tag_frequency * (px - tag_right)).sin();
                let (ax, ay) = to_abs(px, py);
                tag_path.push_str(&format!(" L {ax:.2} {ay:.2}"));
            }
            tag_path.push_str(" Z");

            format!(
                "<path d=\"{body_path}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>\
                 <path d=\"{tag_path}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>"
            )
        }
        crate::ir::NodeShape::CurvedTrapezoid => {
            // Mermaid's display shape: left trapezoid plus right semicircle.
            let sw = node.style.stroke_width.unwrap_or(1.3);
            let radius = h / 2.0;
            let rw = w - radius;
            let tw = h / 4.0;
            format!(
                "<path d=\"M {rw_x:.2} {y:.2} L {tw_x:.2} {y:.2} L {x:.2} {mid_y:.2} L {tw_x:.2} {bottom_y:.2} L {rw_x:.2} {bottom_y:.2} A {radius:.2} {radius:.2} 0 0 0 {rw_x:.2} {y:.2} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>",
                rw_x = x + rw,
                tw_x = x + tw,
                mid_y = y + h / 2.0,
                bottom_y = y + h,
            )
        }
        crate::ir::NodeShape::Cloud => {
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let cw = w * 0.78;
            let ch = h * 0.42;
            let sx = x + w * 0.10;
            let sy = y + h * 0.32;
            let r1 = 0.15 * cw;
            let r2 = 0.25 * cw;
            let r3 = 0.35 * cw;
            let r4 = 0.20 * cw;
            format!(
                "<path d=\"M {sx:.2} {sy:.2} \
                a {r1:.2} {r1:.2} 0 0 1 {dx1:.2} {dy1:.2} \
                a {r3:.2} {r3:.2} 1 0 1 {dx2:.2} {dy2:.2} \
                a {r2:.2} {r2:.2} 1 0 1 {dx3:.2} {dy3:.2} \
                a {r1:.2} {r1:.2} 1 0 1 {dx4:.2} {dy4:.2} \
                a {r4:.2} {r4:.2} 1 0 1 {dx5:.2} {dy5:.2} \
                a {r2:.2} {r1:.2} 1 0 1 {dx6:.2} {dy6:.2} \
                a {r3:.2} {r3:.2} 1 0 1 {dx7:.2} 0 \
                a {r1:.2} {r1:.2} 1 0 1 {dx8:.2} {dy8:.2} \
                a {r1:.2} {r1:.2} 1 0 1 {dx9:.2} {dy9:.2} \
                a {r4:.2} {r4:.2} 1 0 1 {dx10:.2} {dy10:.2} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>",
                dx1 = cw * 0.25,
                dy1 = -cw * 0.10,
                dx2 = cw * 0.40,
                dy2 = -cw * 0.10,
                dx3 = cw * 0.35,
                dy3 = cw * 0.20,
                dx4 = cw * 0.15,
                dy4 = ch * 0.35,
                dx5 = -cw * 0.15,
                dy5 = ch * 0.65,
                dx6 = -cw * 0.25,
                dy6 = cw * 0.15,
                dx7 = -cw * 0.50,
                dx8 = -cw * 0.25,
                dy8 = -cw * 0.15,
                dx9 = -cw * 0.10,
                dy9 = -ch * 0.35,
                dx10 = cw * 0.10,
                dy10 = -ch * 0.65,
            )
        }
        crate::ir::NodeShape::Bang => {
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let effective_width = w / FLOWCHART_BANG_BBOX_SCALE;
            let effective_height = h / FLOWCHART_BANG_BBOX_SCALE;
            let sx = x + effective_width * 0.1;
            let sy = y + effective_height * 0.1;
            let r = 0.15 * effective_width;
            let r_small = r * 0.8;
            format!(
                "<path d=\"M {sx:.2} {sy:.2} \
                a {r:.2} {r:.2} 1 0 0 {dx1:.2} {dy1:.2} \
                a {r:.2} {r:.2} 1 0 0 {dx1:.2} 0 \
                a {r:.2} {r:.2} 1 0 0 {dx1:.2} 0 \
                a {r:.2} {r:.2} 1 0 0 {dx1:.2} {dy4:.2} \
                a {r:.2} {r:.2} 1 0 0 {dx5:.2} {dy5:.2} \
                a {r_small:.2} {r_small:.2} 1 0 0 0 {dy6:.2} \
                a {r:.2} {r:.2} 1 0 0 {dx7:.2} {dy5:.2} \
                a {r:.2} {r:.2} 1 0 0 {dx8:.2} {dy8:.2} \
                a {r:.2} {r:.2} 1 0 0 {dx8:.2} 0 \
                a {r:.2} {r:.2} 1 0 0 {dx8:.2} 0 \
                a {r:.2} {r:.2} 1 0 0 {dx8:.2} {dy11:.2} \
                a {r:.2} {r:.2} 1 0 0 {dx12:.2} {dy12:.2} \
                a {r_small:.2} {r_small:.2} 1 0 0 0 {dy13:.2} \
                a {r:.2} {r:.2} 1 0 0 {dx14:.2} {dy12:.2} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>",
                dx1 = effective_width * 0.25,
                dy1 = -effective_height * 0.10,
                dy4 = effective_height * 0.10,
                dx5 = effective_width * 0.15,
                dy5 = effective_height * 0.33,
                dy6 = effective_height * 0.34,
                dx7 = -effective_width * 0.15,
                dx8 = -effective_width * 0.25,
                dy8 = effective_height * 0.15,
                dy11 = -effective_height * 0.15,
                dx12 = -effective_width * 0.10,
                dy12 = -effective_height * 0.33,
                dy13 = -effective_height * 0.34,
                dx14 = effective_width * 0.10,
            )
        }
        crate::ir::NodeShape::Triangle => {
            // Triangle pointing up.
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let points = format!(
                "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                x + w / 2.0,
                y,
                x + w,
                y + h,
                x,
                y + h,
            );
            format!(
                "<polygon points=\"{points}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>"
            )
        }
        crate::ir::NodeShape::FlippedTriangle => {
            // Triangle pointing down.
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let points = format!(
                "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                x,
                y,
                x + w,
                y,
                x + w / 2.0,
                y + h,
            );
            format!(
                "<polygon points=\"{points}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>"
            )
        }
        crate::ir::NodeShape::SmallCircle => {
            // Circle with smaller default radius.
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let r = w.min(h) / 2.0;
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            format!(
                "<circle cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{r:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>"
            )
        }
        crate::ir::NodeShape::FilledCircle => {
            // Solid-fill circle, no label.
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let r = w.min(h) / 2.0;
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            format!(
                "<circle cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{r:.2}\" fill=\"{stroke}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>"
            )
        }
        crate::ir::NodeShape::HalfRoundedRect => {
            // Rectangle with rounded right side (delay shape).
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let r = h / 2.0;
            format!(
                "<path d=\"M {x:.2} {y:.2} h {w1:.2} a {r:.2} {r:.2} 0 0 1 0 {h:.2} h {nw1:.2} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>",
                w1 = w - r,
                nw1 = -(w - r),
            )
        }
        crate::ir::NodeShape::SlopedRect => {
            // Rectangle with sloped top edge (manual input).
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let slope = h / 3.0;
            format!(
                "<path d=\"M {x:.2} {y1:.2} L {x2:.2} {y:.2} v {h:.2} h {nw:.2} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>",
                y1 = y + slope,
                x2 = x + w,
                nw = -w,
            )
        }
        crate::ir::NodeShape::NotchedPentagon => {
            // Mermaid's loop-limit "trapezoidal pentagon": narrow top with
            // upper side shoulders.
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let points = format!(
                "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                x + w * 0.1,
                y,
                x + w * 0.9,
                y,
                x + w,
                y + h * 0.2,
                x + w,
                y + h,
                x,
                y + h,
                x,
                y + h * 0.2,
            );
            format!(
                "<polygon points=\"{points}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>",
            )
        }
        crate::ir::NodeShape::StackedRect => {
            // Rectangle with offset rectangles behind it (stacked processes).
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let off = 4.0;
            let bw = w - off;
            let bh = h - off;
            let back2 = format!(
                "<rect x=\"{:.2}\" y=\"{y:.2}\" width=\"{bw:.2}\" height=\"{bh:.2}\" rx=\"0\" ry=\"0\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>",
                x + 2.0 * off,
            );
            let back1 = format!(
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{bw:.2}\" height=\"{bh:.2}\" rx=\"0\" ry=\"0\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>",
                x + off,
                y + off,
            );
            let front = format!(
                "<rect x=\"{x:.2}\" y=\"{:.2}\" width=\"{bw:.2}\" height=\"{bh:.2}\" rx=\"0\" ry=\"0\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>",
                y + 2.0 * off,
            );
            format!("{back2}{back1}{front}")
        }
        crate::ir::NodeShape::BowTieRect => {
            // Rectangle with curved (concave) left side (stored data).
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let curve = w * 0.1;
            format!(
                "<path d=\"M {x1:.2} {y:.2} h {w1:.2} v {h:.2} h {nw1:.2} q {qx:.2} {qy:.2} 0 {nqh:.2} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>",
                x1 = x + curve,
                w1 = w - curve,
                nw1 = -(w - curve),
                qx = curve,
                qy = h / 2.0,
                nqh = -h,
            )
        }
        crate::ir::NodeShape::FramedCircle => {
            // Circle inside a circle (framed/stop).
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let r = w.min(h) / 2.0;
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            let outer = format!(
                "<circle cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{r:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>"
            );
            let inner_r = r * 0.7;
            let inner = format!(
                "<circle cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{inner_r:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>"
            );
            format!("{outer}{inner}")
        }
        crate::ir::NodeShape::CrossedCircle => {
            // Circle with an X through it (summary).
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let r = w.min(h) / 2.0;
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            let circ = format!(
                "<circle cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{r:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>"
            );
            let d = r * 0.707; // cos(45°)
            let line1 = format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
                cx - d,
                cy - d,
                cx + d,
                cy + d,
            );
            let line2 = format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
                cx + d,
                cy - d,
                cx - d,
                cy + d,
            );
            format!("{circ}{line1}{line2}")
        }
        crate::ir::NodeShape::HorizontalCylinder => {
            // Matches Mermaid's tiltedCylinder path: the visible bbox is
            // body width plus one radius of arc overshoot on each side.
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let rx = flowchart_tilted_cylinder_rx(h);
            let ry = h / 2.0;
            let body_w = (w - 2.0 * rx).max(10.0);
            let start_x = x + rx;
            let bottom_y = y + h;
            let top_y = y;
            let inner_x = start_x + body_w;
            let neg_h = -h;
            let neg_body_w = -body_w;
            format!(
                "<path d=\"M{start_x:.2},{bottom_y:.2} a{rx:.2},{ry:.2} 0,0,1 0,{neg_h:.2} l{body_w:.2},0 a{rx:.2},{ry:.2} 0,0,1 0,{h:.2} M{inner_x:.2},{top_y:.2} a{rx:.2},{ry:.2} 0,0,0 0,{h:.2} l{neg_body_w:.2},0\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>"
            )
        }
        crate::ir::NodeShape::DividedRect => {
            // Mermaid computes an inner height, then adds a 20% header strip.
            // The visible divider is therefore 1/6 of the total rendered height.
            let sw = node.style.stroke_width.unwrap_or(1.3);
            let div_y = y + flowchart_divided_rect_offset(h);
            format!(
                "<path d=\"M {x:.2} {div_y:.2} L {x2:.2} {div_y:.2} L {x2:.2} {y2:.2} L {x:.2} {y2:.2} L {x:.2} {y:.2} L {x2:.2} {y:.2} L {x2:.2} {div_y:.2} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\" fill-rule=\"evenodd\"{dash}{join}/>",
                x2 = x + w,
                y2 = y + h,
            )
        }
        crate::ir::NodeShape::LinedRect => {
            // Rectangle with vertical lines (lined process).
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let inset = w.min(h) * 0.12;
            let rect = format!(
                "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\" rx=\"0\" ry=\"0\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>"
            );
            let line1 = format!(
                "<line x1=\"{:.2}\" y1=\"{y:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
                x + inset,
                x + inset,
                y + h,
            );
            let line2 = format!(
                "<line x1=\"{:.2}\" y1=\"{y:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
                x + w - inset,
                x + w - inset,
                y + h,
            );
            format!("{rect}{line1}{line2}")
        }
        crate::ir::NodeShape::WavyRect => {
            // Paper tape: rectangle with wavy top and bottom edges.
            let sw = node.style.stroke_width.unwrap_or(1.0);
            let wave = h / 12.0;
            let top_baseline = y + wave;
            let bottom_baseline = y + h - wave;
            format!(
                "<path d=\"M {x:.2} {bottom_baseline:.2} q {q1x:.2} {q2y_dn:.2} {qmx:.2} 0 q {q2x:.2} {q1y_up:.2} {qmx:.2} 0 V {top_baseline:.2} q {nq1x:.2} {q1y_up:.2} {nqmx:.2} 0 q {nq2x:.2} {q2y_dn:.2} {nqmx:.2} 0 Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}{join}/>",
                q1x = w * 0.25,
                q1y_up = -wave * 2.0,
                qmx = w * 0.5,
                q2x = w * 0.25,
                q2y_dn = wave * 2.0,
                nq1x = -(w * 0.25),
                nqmx = -(w * 0.5),
                nq2x = -(w * 0.25),
            )
        }
        _ => format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{dash}{join}/>",
            x,
            y,
            w,
            h,
            fill,
            stroke,
            node.style.stroke_width.unwrap_or(1.0)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LayoutConfig;
    use crate::ir::{Direction, Graph};
    use crate::layout::compute_layout;
    use crate::parser::parse_mermaid;

    fn svg_text_y_for_label(svg: &str, label: &str) -> f32 {
        let label_pos = svg
            .find(label)
            .unwrap_or_else(|| panic!("missing label {label} in svg"));
        let text_pos = svg[..label_pos]
            .rfind("<text ")
            .unwrap_or_else(|| panic!("missing text tag for label {label}"));
        let tag_end = svg[text_pos..]
            .find('>')
            .map(|offset| text_pos + offset)
            .unwrap_or_else(|| panic!("unterminated text tag for label {label}"));
        let tag = &svg[text_pos..tag_end];
        let y_start = tag
            .find(" y=\"")
            .map(|offset| offset + 4)
            .unwrap_or_else(|| panic!("missing y attribute for label {label}: {tag}"));
        let y_end = tag[y_start..]
            .find('"')
            .map(|offset| y_start + offset)
            .unwrap_or_else(|| panic!("unterminated y attribute for label {label}: {tag}"));
        tag[y_start..y_end]
            .parse::<f32>()
            .unwrap_or_else(|err| panic!("invalid y attribute for label {label}: {err}"))
    }

    #[test]
    fn render_svg_basic() {
        let mut graph = Graph::new();
        graph.direction = Direction::LeftRight;
        graph.ensure_node(
            "A",
            Some("Alpha".to_string()),
            Some(crate::ir::NodeShape::Rectangle),
        );
        graph.ensure_node(
            "B",
            Some("Beta".to_string()),
            Some(crate::ir::NodeShape::Rectangle),
        );
        graph.edges.push(crate::ir::Edge {
            from: "A".to_string(),
            to: "B".to_string(),
            label: Some("go".to_string()),
            start_label: None,
            end_label: None,
            directed: true,
            arrow_start: false,
            arrow_end: true,
            arrow_start_kind: None,
            arrow_end_kind: None,
            start_decoration: None,
            end_decoration: None,
            sequence_arrow_end: None,
            sequence_arrow_start: None,
            style: crate::ir::EdgeStyle::Solid,
            markdown_label: false,
            id: None,
            curve: None,
            arch_port_from: None,
            arch_port_to: None,
        });
        let layout = compute_layout(&graph, &Theme::modern(), &LayoutConfig::default());
        let svg = render_svg(&layout, &Theme::modern(), &LayoutConfig::default());
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Alpha"));
        assert!(svg.contains("id=\"edge-0\""));
        assert!(svg.contains("data-edge-id=\"edge-0\""));
        assert!(svg.contains("data-label-kind=\"center\""));
    }

    #[test]
    fn render_svg_declares_and_embeds_mermaid_font() {
        let parsed = parse_mermaid("flowchart LR\nA-->B").unwrap();
        let theme = Theme::mermaid_default();
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        let svg = render_svg(&layout, &theme, &config);

        assert!(svg.contains("svg{font-family:\"trebuchet ms\",verdana,arial,sans-serif;"));
        assert!(svg.contains("font-size:16px;fill:#333;"));
        if crate::text_metrics::embedded_font_data(&theme.font_family).is_some() {
            assert!(svg.contains("@font-face"));
            assert!(svg.contains("data:font/"));
            assert!(svg.contains("base64,"));
        }
    }

    #[test]
    fn cynefin_embeds_bold_mermaid_font_face_for_bold_labels() {
        let parsed = parse_mermaid("cynefin-beta\ncomplex").unwrap();
        let theme = Theme::mermaid_default();
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        let svg = render_svg(&layout, &theme, &config);

        if crate::text_metrics::embedded_font_data_with_weight(&theme.font_family, 700).is_some() {
            assert!(svg.contains("font-weight:700;font-style:normal;"));
        }
    }

    #[test]
    fn sequence_default_colors_match_mermaid_element_defaults() {
        let parsed = parse_mermaid(
            "sequenceDiagram\nAlice->>+John: Hello\nNote over Alice: Remember\nJohn-->>-Alice: Back",
        )
        .unwrap();
        let theme = Theme::mermaid_default();
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        let svg = render_svg(&layout, &theme, &config);

        assert!(svg.contains("stroke=\"#9370DB\" stroke-width=\"0.5\""));
        assert!(svg.contains("fill=\"#ECECFF\" stroke=\"#9370DB\""));
        assert!(svg.contains("fill=\"#FFF5AD\" stroke=\"#AAAA33\""));
        assert!(svg.contains("fill=\"#f4f4f4\" stroke=\"#666\""));
        assert!(svg.contains("stroke=\"#333333\" stroke-width=\"1.5\""));
        assert!(svg.contains("id=\"arrow-seq-0\""));
        assert!(svg.contains("fill=\"#333333\" stroke=\"#333333\""));
        assert!(!svg.contains("#2F3B4D"));
        assert!(!svg.contains("stroke=\"#999\""));
    }

    #[test]
    fn sequence_loop_frames_use_mermaid_label_box_defaults() {
        let parsed = parse_mermaid(
            "sequenceDiagram\nAlice->>John: Begin\nloop every tick\nAlice->>John: Ping\nend",
        )
        .unwrap();
        let theme = Theme::mermaid_default();
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        let svg = render_svg(&layout, &theme, &config);

        let seq = match &layout.diagram {
            DiagramData::Sequence(seq) => seq,
            _ => panic!("expected sequence layout"),
        };
        let frame = seq.frames.first().expect("expected one loop frame");
        assert_eq!(frame.label_box.2, 50.0);
        assert_eq!(frame.label_box.3, 20.0);
        assert!(svg.contains("stroke=\"#9370DB\" stroke-width=\"2.0\" stroke-dasharray=\"2 2\""));
        assert!(svg.contains("fill=\"#ECECFF\" stroke=\"#9370DB\" stroke-width=\"1.1\""));
    }

    #[test]
    fn render_tree_view_uses_mermaid_tree_shell_and_annotations() {
        let parsed = parse_mermaid(
            "treeView-beta\n    src/\n        App.tsx :::highlight icon(react) ## main component\n        config.toml\n        secret icon()",
        )
        .unwrap();
        let theme = Theme::mermaid_default();
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        let svg = render_svg(&layout, &theme, &config);

        assert!(svg.starts_with("<svg id=\"my-svg\""));
        assert!(svg.contains("xmlns:xlink=\"http://www.w3.org/1999/xlink\""));
        assert!(svg.contains("aria-roledescription=\"treeView\""));
        assert!(svg.contains("<g class=\"tree-view\">"));
        assert!(svg.contains("id=\"tv-icon-my-svg-folder\""));
        assert!(svg.contains("id=\"tv-icon-my-svg-react\""));
        assert!(svg.contains("id=\"tv-icon-my-svg-config\""));
        assert!(!svg.contains("xlink:href=\"#tv-icon-my-svg-folder\""));
        assert!(svg.contains("class=\"treeView-highlight-bg\""));
        assert!(svg.contains("class=\"treeView-node-description\""));
        assert!(svg.contains(">main component</text>"));
        assert!(!svg.contains("id=\"arrow-0\""));
    }

    #[test]
    fn sankey_text_nodes_preserve_quote_glyphs() {
        let parsed =
            parse_mermaid("sankey-beta\nA,\"B \"\"quoted\"\"\",1\nAgricultural 'waste',C,1")
                .unwrap();
        let theme = Theme::mermaid_default();
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        let svg = render_svg(&layout, &theme, &config);

        assert!(svg.contains("B \"quoted\"\n1"));
        assert!(svg.contains("Agricultural 'waste'\n1"));
        assert!(!svg.contains("B &quot;quoted&quot;"));
        assert!(!svg.contains("Agricultural &apos;waste&apos;"));
    }

    #[test]
    fn sankey_svg_shell_matches_mermaid_setup() {
        let parsed = parse_mermaid("sankey-beta\nA,B,1").unwrap();
        let theme = Theme::mermaid_default();
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        let svg = render_svg(&layout, &theme, &config);

        assert!(svg.starts_with("<svg id=\"my-svg\" xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(svg.contains("xmlns:xlink=\"http://www.w3.org/1999/xlink\""));
        assert!(svg.contains("role=\"graphics-document document\""));
        assert!(svg.contains("aria-roledescription=\"sankey\""));
        assert!(svg.contains("</style><g/><g class=\"nodes\">"));
    }

    #[test]
    fn eventmodeling_data_blocks_use_xml_safe_nbsp() {
        let parsed = parse_mermaid(concat!(
            "eventmodeling\n",
            "tf 01 cmd AddItem [[AddItem01]]\n",
            "data AddItem01 {\n",
            "  productId: 7\n",
            "  quantity: 2\n",
            "}\n",
        ))
        .unwrap();
        let theme = Theme::mermaid_default();
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        let svg = render_svg(&layout, &theme, &config);

        assert!(!svg.contains("&nbsp;"));
        assert!(svg.contains("productId:\u{00a0}7"));
        assert!(svg.contains("quantity:\u{00a0}2"));
        assert!(svg.contains("productId:\u{00a0}7\n\u{00a0}quantity:\u{00a0}2"));
        assert!(!svg.contains("productId:\u{00a0}7\n\u{00a0}\u{00a0}quantity:\u{00a0}2"));
    }

    #[test]
    fn cynefin_renders_framework_regions_items_and_transitions() {
        let parsed = parse_mermaid(
            "cynefin-beta\n\
title Decision space\n\
complex\n\
  \"Probe\"\n\
complicated\n\
  \"Analyse\"\n\
confusion\n\
  \"Unknown\"\n\
  \"Mixed\"\n\
  \"Ambiguous\"\n\
  \"Overflow\"\n\
complex --> complicated: \"clarify\"",
        )
        .unwrap();
        let theme = Theme::mermaid_default();
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        let svg = render_svg(&layout, &theme, &config);

        assert!(svg.starts_with("<svg id=\"my-svg\" xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(svg.contains("aria-roledescription=\"cynefin\""));
        assert!(svg.contains("viewBox=\"0 0 880 680\""));
        assert!(svg.contains("class=\"cynefinBoundary\""));
        assert!(svg.contains("class=\"cynefinCliff\""));
        assert!(svg.contains(">Probe → Sense → Respond</text>"));
        assert!(svg.contains(">+1 more</text>"));
        assert!(svg.contains("marker-end=\"url(#cynefin-arrow-my-svg)\""));
        assert!(svg.contains(">clarify</text>"));
    }

    #[test]
    fn kanban_metadata_renders_footer_fields_not_body_text() {
        let parsed = parse_mermaid(
            "kanban\n  todo[Todo]\n    id3[Update Database Function]@{ ticket: MC-2037, assigned: 'knsv', priority: 'High' }",
        )
        .unwrap();
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let svg = render_svg(&layout, &Theme::modern(), &LayoutConfig::default());

        assert!(svg.contains(">MC-2037</tspan>"));
        assert!(svg.contains(">knsv</tspan>"));
        assert!(svg.contains("y=\"103.00\""));
        assert!(!svg.contains("y=\"99.00\""));
        assert!(svg.contains("stroke=\"orange\""));
        assert!(!svg.contains("ticket: MC-2037"));
        assert!(!svg.contains("assigned:"));
        assert!(!svg.contains("priority:"));
    }

    #[test]
    fn kanban_section_label_stays_inside_header_band() {
        let parsed =
            parse_mermaid("kanban\n  column1[Column Title]\n    task1[Task Description]").unwrap();
        let theme = Theme::mermaid_default();
        let layout = compute_layout(&parsed.graph, &theme, &LayoutConfig::default());
        let svg = render_svg(&layout, &theme, &LayoutConfig::default());

        assert!(svg.contains("<text x=\"110.00\" y=\"26.00\""));
        assert!(svg.contains("<rect x=\"17.50\" y=\"35.00\" width=\"185.00\""));
        assert!(!svg.contains("<text x=\"110.00\" y=\"38.00\""));
    }

    #[test]
    fn flowchart_cluster_label_stays_inside_title_band() {
        let source = r#"flowchart TB
    md[("hello-fed MD<br/>(replicated on both members)")]
    subgraph FED0["fed0 deploy controller"]
        d0[reconcile vc-a1<br/>OWNED -> deploy]
        d0x[reconcile vc-b1<br/>FOREIGN -> skip]
    end
    subgraph FED1["fed1 deploy controller"]
        d1x[reconcile vc-a1<br/>FOREIGN -> skip]
        d1[reconcile vc-b1<br/>OWNED -> deploy]
    end
    md --> d0 & d0x & d1 & d1x
    d0 ==>|kubectl apply| pa["pod: hello-world<br/>default-fedten-vc-a1-default<br/>fed0cluster1"]
    d1 ==>|kubectl apply| pb["pod: hello-world<br/>default-fedten-vc-b1-default<br/>fed1cluster1"]"#;
        let parsed = parse_mermaid(source).expect("failed to parse realworld phase-08 fixture");
        let theme = Theme::mermaid_default();
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        let svg = render_svg(&layout, &theme, &config);

        for (label, first_child_id) in [
            ("fed0 deploy controller", "d0"),
            ("fed1 deploy controller", "d1x"),
        ] {
            let subgraph = layout
                .subgraphs
                .iter()
                .find(|subgraph| subgraph.label == label)
                .unwrap_or_else(|| panic!("missing subgraph {label}"));
            let child = layout
                .nodes
                .get(first_child_id)
                .unwrap_or_else(|| panic!("missing child {first_child_id}"));
            let rendered_label_y = svg_text_y_for_label(&svg, label);
            let expected_label_y = subgraph.y + theme.font_size;

            assert!(
                (rendered_label_y - expected_label_y).abs() <= 0.05,
                "flowchart cluster labels should render inside Mermaid's top title band; got label y {rendered_label_y:.2}, expected {expected_label_y:.2}"
            );
            assert!(
                rendered_label_y + 4.0 < child.y,
                "flowchart cluster label {label} should not overlap first child {first_child_id}; label y {rendered_label_y:.2}, child top {:.2}",
                child.y
            );
        }
    }

    #[test]
    fn flowchart_cluster_rects_use_classic_square_corners() {
        let parsed = parse_mermaid("flowchart TB\nsubgraph A[Group]\n  b[Node]\nend").unwrap();
        let theme = Theme::mermaid_default();
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        let svg = render_svg(&layout, &theme, &config);

        assert!(svg.contains("rx=\"0\" ry=\"0\" fill=\"#FFFFDE\" stroke=\"#AAAA33\""));
        assert!(!svg.contains("rx=\"10\" ry=\"10\" fill=\"#FFFFDE\" stroke=\"#AAAA33\""));
    }

    #[test]
    fn flowchart_normal_edges_use_mermaid_common_stroke_width() {
        let parsed = parse_mermaid("flowchart LR\n  A --> B\n  B ==> C").unwrap();
        let theme = Theme::mermaid_default();
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        let svg = render_svg(&layout, &theme, &config);
        let edge0 = svg
            .split("<path")
            .find(|part| part.contains("data-edge-id=\"edge-0\""))
            .expect("missing normal flowchart edge");
        let edge1 = svg
            .split("<path")
            .find(|part| part.contains("data-edge-id=\"edge-1\""))
            .expect("missing thick flowchart edge");

        assert!(edge0.contains("stroke-width=\"1\""));
        assert!(!edge0.contains("stroke-width=\"2\""));
        assert!(edge1.contains("stroke-width=\"3.5\""));
    }

    #[test]
    fn class_inline_annotation_renders_as_annotation_row() {
        let parsed = parse_mermaid("classDiagram\nclass Shape <<interface>>").unwrap();
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let svg = render_svg(&layout, &Theme::modern(), &LayoutConfig::default());

        assert!(svg.contains(">\u{00ab}interface\u{00bb}</tspan>"));
        assert!(svg.contains(">Shape</tspan>"));
        assert!(!svg.contains("&lt;&lt;interface&gt;&gt;"));
        assert!(!svg.contains("Shape &lt;&lt;"));
    }

    #[test]
    fn er_entity_labels_render_markdown() {
        let parsed = parse_mermaid("erDiagram\n\"This **is** _Markdown_\"").unwrap();
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let svg = render_svg(&layout, &Theme::modern(), &LayoutConfig::default());

        assert!(svg.contains("font-weight=\"bold\""));
        assert!(svg.contains("font-style=\"italic\""));
        assert!(!svg.contains("**is**"));
        assert!(!svg.contains("_Markdown_"));
    }

    #[test]
    fn er_frontmatter_title_renders_above_diagram() {
        let parsed = parse_mermaid(
            "---\ntitle: Order example\n---\nerDiagram\nCUSTOMER ||--o{ ORDER : places",
        )
        .unwrap();
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let svg = render_svg(&layout, &Theme::modern(), &LayoutConfig::default());

        assert!(svg.contains("class=\"erDiagramTitleText\""));
        assert!(svg.contains(">Order example</text>"));
        assert!(svg.contains("viewBox=\"0 -48"));
    }

    #[test]
    fn er_attribute_rows_render_with_mermaid_row_height() {
        let parsed =
            parse_mermaid("erDiagram\nCUSTOMER {\nstring name\nstring custNumber\n}").unwrap();
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let svg = render_svg(&layout, &Theme::modern(), &LayoutConfig::default());

        assert!(svg.contains("height=\"42.75\""));
        assert!(svg.contains("height=\"128.25\""));
    }

    #[test]
    fn er_attribute_rows_render_zebra_backgrounds() {
        let parsed = parse_mermaid(
            "erDiagram\nCUSTOMER {\nstring name\nstring custNumber\nstring sector\n}",
        )
        .unwrap();
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let svg = render_svg(&layout, &Theme::modern(), &LayoutConfig::default());

        assert!(svg.contains("class=\"row-rect-odd\""));
        assert!(svg.contains("class=\"row-rect-even\""));
        assert!(svg.contains("fill=\"hsl(240, 100%, 100%)\""));
        assert!(svg.contains("fill=\"hsl(240, 100%, 97.2745098039%)\""));

        let row_pos = svg.find("class=\"row-rect-odd\"").expect("row rect");
        let label_pos = svg.find(">name</tspan>").expect("attribute label");
        assert!(
            row_pos < label_pos,
            "attribute row backgrounds should render underneath text"
        );
    }

    #[test]
    fn er_inline_styles_apply_to_box_and_label() {
        let parsed = parse_mermaid(
            "erDiagram\nid1 ||--|| id2 : label\nstyle id1 fill:#f9f,stroke:#333,stroke-width:4px\nstyle id2 fill:#bbf,stroke:#f66,stroke-width:2px,color:#fff,stroke-dasharray: 5 5",
        )
        .unwrap();
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let svg = render_svg(&layout, &Theme::modern(), &LayoutConfig::default());

        assert!(
            svg.contains(
                "fill=\"#bbf\" stroke=\"#f66\" stroke-width=\"2\" stroke-dasharray=\"5 5\""
            )
        );
        assert!(svg.contains("fill=\"#fff\"><tspan"));
        assert!(
            !svg.contains("fill=\"#FFFFDE\""),
            "styled empty ER entities should not be repainted with the default header fill"
        );
    }

    #[test]
    fn er_inline_stroke_styles_apply_to_attribute_dividers() {
        let parsed = parse_mermaid(
            "erDiagram\nPERSON {\nstring name\nstring email\n}\nstyle PERSON fill:#bbf,stroke:#f66,stroke-width:2px,stroke-dasharray: 5 5",
        )
        .unwrap();
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let svg = render_svg(&layout, &Theme::modern(), &LayoutConfig::default());

        let header_divider = svg
            .split("/>")
            .find(|element| {
                element.contains("<line ")
                    && element.contains("y1=\"50.75\"")
                    && element.contains("y2=\"50.75\"")
                    && element.contains("stroke-opacity=\"0.6\"")
            })
            .expect("header divider line should be rendered");
        assert!(header_divider.contains("stroke=\"#f66\""));
        assert!(header_divider.contains("stroke-width=\"2\""));
        assert!(header_divider.contains("stroke-dasharray=\"5 5\""));
    }

    #[test]
    fn er_default_class_stroke_width_keeps_dividers_on_border_color() {
        let parsed = parse_mermaid(
            "erDiagram\nCAR {\nstring registrationNumber\nstring make\n}\nclassDef default fill:#f9f,stroke-width:4px",
        )
        .unwrap();
        let theme = Theme::modern();
        let border = theme.primary_border_color.clone();
        let layout = compute_layout(&parsed.graph, &theme, &LayoutConfig::default());
        let svg = render_svg(&layout, &theme, &LayoutConfig::default());

        assert!(svg.contains(&format!(
            "fill=\"#f9f\" stroke=\"{}\" stroke-width=\"4\"",
            border
        )));
        assert!(svg.contains("y1=\"50.75\" x2=\""));
        assert!(svg.contains(&format!(
            "y2=\"50.75\" stroke=\"{}\" stroke-width=\"4\"",
            border
        )));
        assert!(
            !svg.contains("stroke=\"#AAAA33\" stroke-width=\"4\""),
            "ER default stroke-width styles should not turn divider lines into cluster-colored strokes"
        );
    }

    #[test]
    fn er_endpoint_decorations_render_above_nodes_and_away_from_target() {
        let parsed = parse_mermaid("erDiagram\nid1 ||--|| id2 : label").unwrap();
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let svg = render_svg(&layout, &Theme::modern(), &LayoutConfig::default());

        let id2_pos = svg.find(">id2</tspan>").expect("id2 label");
        let overlay_pos = svg
            .find("class=\"erEdgeDecorations\"")
            .expect("ER decoration overlay");

        assert!(
            overlay_pos > id2_pos,
            "ER endpoint decorations should be painted after entity nodes"
        );
        assert!(
            svg.contains("rotate(270.00)"),
            "target-end ER marker should point away from the target entity"
        );
        assert!(
            svg.contains("M 9 -6 L 9 6 M 15 -6 L 15 6"),
            "ER only-one bars should sit clear of the entity boundary"
        );
    }

    #[test]
    fn class_divider_lines_extend_to_node_border() {
        let node = crate::layout::NodeLayout {
            id: "ClassA".to_string(),
            x: 10.0,
            y: 20.0,
            width: 120.0,
            height: 90.0,
            label: crate::layout::TextBlock {
                lines: vec![
                    crate::layout::TextLine::plain("ClassA".to_string()),
                    crate::layout::TextLine::plain("---".to_string()),
                    crate::layout::TextLine::plain("+field".to_string()),
                    crate::layout::TextLine::plain("---".to_string()),
                    crate::layout::TextLine::plain("+method()".to_string()),
                ],
                width: 120.0,
                height: 90.0,
            },
            shape: crate::ir::NodeShape::Rectangle,
            style: crate::ir::NodeStyle::default(),
            link: None,
            anchor_subgraph: None,
            hidden: false,
            icon: None,
            img: None,
            img_w: None,
            img_h: None,
            sub_label: None,
            is_treemap_leaf: false,
            treemap_base_text_color: None,
        };

        let svg = divider_lines_svg(&node, &Theme::modern(), 20.0, true, true);

        let expected_y = 20.0 + 90.0 / 2.0 - (5.0 * 20.0) / 2.0 + 14.0 + 20.0 - 14.0 * 0.35;
        assert!(svg.contains(&format!("M10.00 {expected_y:.2}")));
        assert!(svg.contains(&format!("130.00 {expected_y:.2}")));
        assert!(!svg.contains("M16.00"));
        assert!(!svg.contains("124.00"));
        assert!(!svg.contains("<line"));
    }

    #[test]
    fn rendered_class_diagram_dividers_touch_class_box_edges() {
        fn attr_str<'a>(element: &'a str, attr: &str) -> &'a str {
            let needle = format!("{attr}=\"");
            let start = element.find(&needle).unwrap() + needle.len();
            let end = start + element[start..].find('"').unwrap();
            &element[start..end]
        }

        fn elements<'a>(svg: &'a str, tag: &str) -> Vec<&'a str> {
            let needle = format!("<{tag} ");
            let mut out = Vec::new();
            let mut offset = 0;
            while let Some(relative_start) = svg[offset..].find(&needle) {
                let start = offset + relative_start;
                let end = start + svg[start..].find('>').unwrap() + 1;
                out.push(&svg[start..end]);
                offset = end;
            }
            out
        }

        fn divider_path_elements(svg: &str) -> Vec<&str> {
            let needle = "<g class=\"divider\"><path ";
            let mut out = Vec::new();
            let mut offset = 0;
            while let Some(relative_start) = svg[offset..].find(needle) {
                let start = offset + relative_start + "<g class=\"divider\">".len();
                let end = start + svg[start..].find('>').unwrap() + 1;
                out.push(&svg[start..end]);
                offset = end;
            }
            out
        }

        let parsed = crate::parser::parse_mermaid(
            "classDiagram\nclass BankAccount\nBankAccount : +String owner\nBankAccount : +deposit(amount)",
        )
        .unwrap();
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let svg = render_svg(&layout, &Theme::modern(), &LayoutConfig::default());

        let class_node = layout.nodes.get("BankAccount").expect("class node");
        let rect_x = class_node.x;
        let rect_right = class_node.x + class_node.width;
        let divider_paths = divider_path_elements(&svg);

        assert_eq!(divider_paths.len(), 2);
        for path in divider_paths {
            let d = attr_str(path, "d");
            assert!(d.contains(&format!("M{rect_x:.2} ")), "{path}");
            assert!(d.contains(&format!("{rect_right:.2} ")), "{path}");
        }
        assert!(
            elements(&svg, "line").is_empty(),
            "class dividers should render as Mermaid classBox paths"
        );
    }

    #[test]
    fn class_divider_lines_inherit_node_stroke_style() {
        let node = crate::layout::NodeLayout {
            id: "StyledClass".to_string(),
            x: 10.0,
            y: 20.0,
            width: 120.0,
            height: 90.0,
            label: crate::layout::TextBlock {
                lines: vec![
                    crate::layout::TextLine::plain("StyledClass".to_string()),
                    crate::layout::TextLine::plain("---".to_string()),
                    crate::layout::TextLine::plain("---".to_string()),
                ],
                width: 120.0,
                height: 90.0,
            },
            shape: crate::ir::NodeShape::Rectangle,
            style: crate::ir::NodeStyle {
                stroke: Some("#f66".to_string()),
                stroke_width: Some(2.0),
                stroke_dasharray: Some("5 5".to_string()),
                ..crate::ir::NodeStyle::default()
            },
            link: None,
            anchor_subgraph: None,
            hidden: false,
            icon: None,
            img: None,
            img_w: None,
            img_h: None,
            sub_label: None,
            is_treemap_leaf: false,
            treemap_base_text_color: None,
        };

        let svg = divider_lines_svg(&node, &Theme::modern(), 20.0, true, true);

        assert_eq!(svg.matches("stroke=\"#f66\"").count(), 2);
        assert_eq!(svg.matches("stroke-width=\"2\"").count(), 2);
        assert_eq!(svg.matches("stroke-dasharray=\"5 5\"").count(), 2);
    }

    #[test]
    fn rendered_class_style_lines_apply_class_box_and_text_styles() {
        let parsed = crate::parser::parse_mermaid(
            "classDiagram\nclass Animal\nclass Mineral\nstyle Animal fill:#f9f,stroke:#333,stroke-width:4px\nstyle Mineral fill:#bbf,stroke:#f66,stroke-width:2px,color:#fff,stroke-dasharray: 5 5",
        )
        .unwrap();
        let theme = Theme::base();
        let layout = compute_layout(&parsed.graph, &theme, &LayoutConfig::default());
        let svg = render_svg(&layout, &theme, &LayoutConfig::default());

        assert!(svg.contains("fill=\"#f9f\""));
        assert!(svg.contains("stroke=\"#333\""));
        assert!(svg.contains("stroke-width=\"4\""));
        assert!(svg.contains("fill=\"#bbf\""));
        assert!(svg.contains("stroke=\"#f66\""));
        assert!(svg.contains("stroke-width=\"2\""));
        assert!(svg.contains("stroke-dasharray=\"5 5\""));
        assert!(svg.contains("fill=\"#fff\""));
    }

    #[test]
    fn class_notes_render_as_yellow_notes_with_connector() {
        let parsed = crate::parser::parse_mermaid(
            "classDiagram\nnote \"This is a general note\"\nnote for MyClass \"This is a note for a class\"\nclass MyClass",
        )
        .unwrap();
        let theme = Theme::base();
        let layout = compute_layout(&parsed.graph, &theme, &LayoutConfig::default());
        let svg = render_svg(&layout, &theme, &LayoutConfig::default());

        assert!(svg.contains("This is a general note"));
        assert!(svg.contains("This is a note for a class"));
        let lower_svg = svg.to_ascii_lowercase();
        assert!(lower_svg.contains("fill=\"#fff5ad\""));
        assert!(lower_svg.contains("stroke=\"#aaaa33\""));
        assert!(svg.contains("stroke-dasharray=\"2\""));
    }

    #[test]
    fn state_notes_render_mermaid_note_edge_connectors() {
        let parsed = crate::parser::parse_mermaid(
            "stateDiagram-v2\nState1: The state with a note\nnote right of State1\nImportant information! You can write\nnotes.\nend note\nState1 --> State2\nnote left of State2 : This is a note on State2.",
        )
        .unwrap();
        let theme = Theme::mermaid_default();
        let layout = compute_layout(&parsed.graph, &theme, &LayoutConfig::default());
        let svg = render_svg(&layout, &theme, &LayoutConfig::default());

        assert_eq!(svg.matches("transition note-edge").count(), 2);
        assert_eq!(svg.matches("stroke-dasharray=\"5\"").count(), 2);
        assert!(!svg.contains("stroke=\"#AAAA33\" stroke-width=\"1\" stroke-dasharray=\"5 3\""));
    }

    #[test]
    fn class_symbol_render_points_trim_endpoint_to_leave_marker_visible() {
        let edge = crate::layout::EdgeLayout {
            from: "A".to_string(),
            to: "B".to_string(),
            label: None,
            start_label: None,
            end_label: None,
            label_anchor: None,
            start_label_anchor: None,
            end_label_anchor: None,
            points: vec![(0.0, 0.0), (0.0, 100.0)],
            directed: true,
            arrow_start: false,
            arrow_end: true,
            arrow_start_kind: None,
            arrow_end_kind: Some(crate::ir::EdgeArrowhead::OpenTriangle),
            start_decoration: None,
            end_decoration: None,
            sequence_arrow_end: None,
            sequence_arrow_start: None,
            style: crate::ir::EdgeStyle::Solid,
            override_style: crate::ir::EdgeStyleOverride::default(),
            curve: None,
        };

        let points = class_symbol_render_points(&edge, crate::ir::DiagramKind::Class);
        assert_eq!(points[0], (0.0, 0.0));
        assert!((points[1].1 - 82.75).abs() < 0.001);

        let flowchart_points = class_symbol_render_points(&edge, crate::ir::DiagramKind::Flowchart);
        assert_eq!(flowchart_points[1], (0.0, 100.0));
    }

    #[test]
    fn flowchart_two_point_basis_edges_get_dagre_midpoint() {
        let points = flowchart_d3_basis_points(&[(0.0, 0.0), (0.0, 60.0)]);
        assert_eq!(points, vec![(0.0, 0.0), (0.0, 30.0), (0.0, 60.0)]);

        let d = points_to_d3_basis_path(&points);
        assert!(d.starts_with("M 0.000,0.000 L 0.000,5.000 C"));
        assert!(d.ends_with("L 0.000,60.000"));
    }

    #[test]
    fn flowchart_arrow_paths_are_shortened_like_mermaid_line_with_offset() {
        let edge = crate::layout::EdgeLayout {
            from: "A".to_string(),
            to: "B".to_string(),
            label: None,
            start_label: None,
            end_label: None,
            label_anchor: None,
            start_label_anchor: None,
            end_label_anchor: None,
            points: vec![(0.0, 0.0), (100.0, 0.0)],
            directed: true,
            arrow_start: false,
            arrow_end: true,
            arrow_start_kind: None,
            arrow_end_kind: None,
            start_decoration: None,
            end_decoration: None,
            sequence_arrow_end: None,
            sequence_arrow_start: None,
            style: crate::ir::EdgeStyle::Solid,
            override_style: crate::ir::EdgeStyleOverride::default(),
            curve: None,
        };

        let end_only = flowchart_marker_offset_render_points(&edge.points, &edge);
        assert_eq!(end_only, vec![(0.0, 0.0), (96.0, 0.0)]);

        let mut both_edge = edge.clone();
        both_edge.arrow_start = true;
        let both = flowchart_marker_offset_render_points(&both_edge.points, &both_edge);
        assert_eq!(both, vec![(4.0, 0.0), (96.0, 0.0)]);

        let mut vertical_edge = edge.clone();
        vertical_edge.points = vec![(0.0, 0.0), (0.0, 100.0)];
        let vertical = flowchart_marker_offset_render_points(&vertical_edge.points, &vertical_edge);
        assert_eq!(vertical, vec![(0.0, 0.0), (0.0, 96.0)]);
    }

    #[test]
    fn class_open_end_marker_uses_extension_end_shape() {
        let parsed = crate::parser::parse_mermaid("classDiagram\nA --|> B : inherits").unwrap();
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let svg = render_svg(&layout, &Theme::modern(), &LayoutConfig::default());

        assert!(svg.contains("id=\"arrow-class-open-0\""));
        assert!(svg.contains("<path d=\"M 1 1 V 13 L 18 7 Z\""));
        assert!(svg.contains("marker-end=\"url(#arrow-class-open-0)\""));
    }

    #[test]
    fn class_two_way_extension_renders_both_open_markers() {
        let parsed = crate::parser::parse_mermaid("classDiagram\nAnimal <|--|> Zebra").unwrap();
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let svg = render_svg(&layout, &Theme::modern(), &LayoutConfig::default());

        assert!(svg.contains("marker-start=\"url(#arrow-class-open-start-0)\""));
        assert!(svg.contains("marker-end=\"url(#arrow-class-open-0)\""));
    }

    #[test]
    fn center_label_background_hidden_when_path_is_clear() {
        let points = vec![(0.0, 0.0), (120.0, 0.0)];
        let touching = LabelRect {
            x: 40.0,
            y: -5.0,
            width: 24.0,
            height: 10.0,
        };
        assert!(edge_label_background_visible(
            crate::ir::DiagramKind::Flowchart,
            EdgeLabelKind::Center,
            &points,
            touching
        ));

        let detached = LabelRect {
            x: 40.0,
            y: -30.0,
            width: 24.0,
            height: 10.0,
        };
        assert!(!edge_label_background_visible(
            crate::ir::DiagramKind::Flowchart,
            EdgeLabelKind::Center,
            &points,
            detached
        ));
    }

    #[test]
    fn endpoint_label_background_prefers_no_box_when_not_touching() {
        let points = vec![(0.0, 0.0), (120.0, 0.0)];
        let detached = LabelRect {
            x: 8.0,
            y: -14.0,
            width: 16.0,
            height: 8.0,
        };
        assert!(!edge_label_background_visible(
            crate::ir::DiagramKind::Class,
            EdgeLabelKind::Start,
            &points,
            detached
        ));

        let touching = LabelRect {
            x: 8.0,
            y: -4.0,
            width: 16.0,
            height: 8.0,
        };
        assert!(!edge_label_background_visible(
            crate::ir::DiagramKind::Class,
            EdgeLabelKind::Start,
            &points,
            touching
        ));
        assert!(edge_label_background_visible(
            crate::ir::DiagramKind::Sequence,
            EdgeLabelKind::Start,
            &points,
            touching
        ));
    }

    #[test]
    fn sequence_center_label_background_visible_for_near_clearance() {
        let points = vec![(0.0, 0.0), (120.0, 0.0)];
        let near = LabelRect {
            x: 40.0,
            y: -11.5,
            width: 24.0,
            height: 10.0,
        };
        assert!(edge_label_background_visible(
            crate::ir::DiagramKind::Sequence,
            EdgeLabelKind::Center,
            &points,
            near
        ));
        assert!(!edge_label_background_visible(
            crate::ir::DiagramKind::Flowchart,
            EdgeLabelKind::Center,
            &points,
            near
        ));
    }

    #[test]
    fn sequence_endpoint_label_background_visible_for_small_non_touch_gap() {
        let points = vec![(0.0, 0.0), (120.0, 0.0)];
        let near = LabelRect {
            x: 8.0,
            y: -8.9,
            width: 16.0,
            height: 8.0,
        };
        assert!(edge_label_background_visible(
            crate::ir::DiagramKind::Sequence,
            EdgeLabelKind::Start,
            &points,
            near
        ));
        assert!(!edge_label_background_visible(
            crate::ir::DiagramKind::Class,
            EdgeLabelKind::Start,
            &points,
            near
        ));
    }

    #[test]
    fn sloped_rect_top_extension_matches_mermaid_manual_input() {
        let node = crate::layout::NodeLayout {
            id: "A".to_string(),
            x: 8.0,
            y: 8.0,
            width: 39.4375,
            height: 81.0,
            label: crate::layout::TextBlock {
                lines: vec![crate::layout::TextLine::plain("A".to_string())],
                width: 9.4375,
                height: 24.0,
            },
            shape: crate::ir::NodeShape::SlopedRect,
            style: crate::ir::NodeStyle::default(),
            link: None,
            anchor_subgraph: None,
            hidden: false,
            icon: None,
            img: None,
            img_w: None,
            img_h: None,
            sub_label: None,
            is_treemap_leaf: false,
            treemap_base_text_color: None,
        };
        let svg = shape_svg(
            &node,
            &Theme::modern(),
            &LayoutConfig::default(),
            crate::ir::DiagramKind::Flowchart,
        );

        assert!(svg.contains("M 8.00 35.00 L 47.44 8.00 v 81.00 h -39.44 Z"));
    }

    #[test]
    fn notched_pentagon_points_match_mermaid_loop_limit() {
        let node = crate::layout::NodeLayout {
            id: "A".to_string(),
            x: 8.0,
            y: 8.0,
            width: 39.4375,
            height: 54.0,
            label: crate::layout::TextBlock {
                lines: vec![crate::layout::TextLine::plain("A".to_string())],
                width: 9.4375,
                height: 24.0,
            },
            shape: crate::ir::NodeShape::NotchedPentagon,
            style: crate::ir::NodeStyle::default(),
            link: None,
            anchor_subgraph: None,
            hidden: false,
            icon: None,
            img: None,
            img_w: None,
            img_h: None,
            sub_label: None,
            is_treemap_leaf: false,
            treemap_base_text_color: None,
        };
        let svg = shape_svg(
            &node,
            &Theme::modern(),
            &LayoutConfig::default(),
            crate::ir::DiagramKind::Flowchart,
        );

        assert!(svg.contains(
            "points=\"11.94,8.00 43.49,8.00 47.44,18.80 47.44,62.00 8.00,62.00 8.00,18.80\""
        ));
    }

    #[test]
    fn curly_braces_both_sides_match_mermaid_bounds() {
        let (left_points, right_points) = flowchart_curly_braces_points(8.0, 8.0, 36.9375, 49.0);
        let all_points = left_points
            .iter()
            .chain(right_points.iter())
            .copied()
            .collect::<Vec<_>>();
        let min_x = all_points.iter().map(|(x, _)| *x).fold(f32::MAX, f32::min);
        let max_x = all_points.iter().map(|(x, _)| *x).fold(f32::MIN, f32::max);
        let min_y = all_points.iter().map(|(_, y)| *y).fold(f32::MAX, f32::min);
        let max_y = all_points.iter().map(|(_, y)| *y).fold(f32::MIN, f32::max);

        assert!((min_x - 8.0).abs() <= 0.01);
        assert!((max_x - 44.9375).abs() <= 0.01);
        assert!((min_y - 8.0).abs() <= 0.01);
        assert!((max_y - 57.0).abs() <= 0.01);
    }

    #[test]
    fn braces_both_sides_render_two_open_curly_paths() {
        let node = crate::layout::NodeLayout {
            id: "A".to_string(),
            x: 8.0,
            y: 8.0,
            width: 36.9375,
            height: 49.0,
            label: crate::layout::TextBlock {
                lines: vec![crate::layout::TextLine::plain("A".to_string())],
                width: 9.4375,
                height: 24.0,
            },
            shape: crate::ir::NodeShape::BraceBoth,
            style: crate::ir::NodeStyle::default(),
            link: None,
            anchor_subgraph: None,
            hidden: false,
            icon: None,
            img: None,
            img_w: None,
            img_h: None,
            sub_label: None,
            is_treemap_leaf: false,
            treemap_base_text_color: None,
        };
        let svg = shape_svg(
            &node,
            &Theme::modern(),
            &LayoutConfig::default(),
            crate::ir::DiagramKind::Flowchart,
        );

        assert_eq!(svg.matches("<path ").count(), 2);
        assert_eq!(svg.matches("fill=\"none\"").count(), 2);
        assert!(!svg.contains(" h "));
    }

    #[test]
    fn block_arrow_right_points_match_mermaid_compact_blank_arrow() {
        let node = crate::layout::NodeLayout {
            id: "blockArrowId6".to_string(),
            x: 0.0,
            y: 0.0,
            width: 24.0,
            height: 16.0,
            label: crate::layout::TextBlock {
                lines: vec![crate::layout::TextLine::plain(" ".to_string())],
                width: 0.0,
                height: 24.0,
            },
            shape: crate::ir::NodeShape::BlockArrowRight,
            style: crate::ir::NodeStyle::default(),
            link: None,
            anchor_subgraph: None,
            hidden: false,
            icon: None,
            img: None,
            img_w: None,
            img_h: None,
            sub_label: None,
            is_treemap_leaf: false,
            treemap_base_text_color: None,
        };
        let svg = shape_svg(
            &node,
            &Theme::modern(),
            &LayoutConfig::default(),
            crate::ir::DiagramKind::Block,
        );

        assert!(svg.contains(
            "points=\"8.00,12.00 8.00,12.00 16.00,12.00 16.00,16.00 24.00,8.00 16.00,0.00 16.00,4.00 8.00,4.00 8.00,4.00\""
        ));
    }

    #[test]
    fn flowchart_lean_shape_points_match_mermaid_data_io_geometry() {
        fn node(shape: crate::ir::NodeShape) -> crate::layout::NodeLayout {
            crate::layout::NodeLayout {
                id: "A".to_string(),
                x: 8.0,
                y: 8.0,
                width: 63.4375,
                height: 39.0,
                label: crate::layout::TextBlock {
                    lines: vec![crate::layout::TextLine::plain("A".to_string())],
                    width: 9.4375,
                    height: 24.0,
                },
                shape,
                style: crate::ir::NodeStyle::default(),
                link: None,
                anchor_subgraph: None,
                hidden: false,
                icon: None,
                img: None,
                img_w: None,
                img_h: None,
                sub_label: None,
                is_treemap_leaf: false,
                treemap_base_text_color: None,
            }
        }

        let left_svg = shape_svg(
            &node(crate::ir::NodeShape::LeanLeft),
            &Theme::modern(),
            &LayoutConfig::default(),
            crate::ir::DiagramKind::Flowchart,
        );
        assert!(left_svg.contains("points=\"8.00,8.00 51.94,8.00 71.44,47.00 27.50,47.00\""));

        let right_svg = shape_svg(
            &node(crate::ir::NodeShape::LeanRight),
            &Theme::modern(),
            &LayoutConfig::default(),
            crate::ir::DiagramKind::Flowchart,
        );
        assert!(right_svg.contains("points=\"27.50,8.00 71.44,8.00 51.94,47.00 8.00,47.00\""));
    }

    #[test]
    fn wavy_rect_bottom_edge_stays_within_bounds() {
        // Regression: paper-tape (WavyRect) bottom wave went right instead of
        // left, causing the shape to stretch into neighbouring nodes.
        let mut graph = Graph::new();
        graph.direction = Direction::RightLeft;
        graph.ensure_node(
            "E",
            Some("Paper Records".to_string()),
            Some(crate::ir::NodeShape::WavyRect),
        );
        let layout = compute_layout(&graph, &Theme::modern(), &LayoutConfig::default());
        let svg = render_svg(&layout, &Theme::modern(), &LayoutConfig::default());

        let (_, node) = layout
            .nodes
            .iter()
            .find(|(id, _)| id.as_str() == "E")
            .unwrap();
        let node_right = (node.x + node.width + 1.0) as f64;

        let paper_idx = svg.find("Paper Records").unwrap();
        let path_before = &svg[..paper_idx];
        let d_start = path_before.rfind("d=\"").unwrap() + 3;
        let d_end = path_before[d_start..].find('"').unwrap() + d_start;
        let d_attr = &svg[d_start..d_end];

        let mut abs_x = 0.0_f64;
        let mut max_x = f64::MIN;
        let nums: Vec<f64> = d_attr
            .split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
            .filter(|s| !s.is_empty())
            .map(|s| s.parse::<f64>().unwrap())
            .collect();

        abs_x = nums[0];
        max_x = max_x.max(abs_x);

        let remainder = d_attr.trim_start_matches(|c: char| {
            c == 'M' || c == ' ' || c.is_ascii_digit() || c == '.' || c == '-'
        });
        for segment in
            remainder.split_inclusive(|c: char| c.is_ascii_uppercase() || c.is_ascii_lowercase())
        {
            let cmd = segment.chars().last().unwrap_or(' ');
            let seg_nums: Vec<f64> = segment
                .split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
                .filter(|s| !s.is_empty())
                .map(|s| s.parse::<f64>().unwrap())
                .collect();
            match cmd {
                'q' => {
                    if seg_nums.len() >= 4 {
                        abs_x += seg_nums[2];
                        max_x = max_x.max(abs_x);
                    }
                }
                'v' | 'Z' => {}
                _ => {}
            }
        }

        assert!(
            max_x <= node_right,
            "WavyRect path extends to x={max_x:.1} but node right edge is {node_right:.1}; \
             bottom wave likely goes in wrong direction. d=\"{d_attr}\""
        );
    }
}

// ── TreeView renderer ───────────────────────────────────────────────────

const TREE_VIEW_FILE_ICON: &str =
    "M13,9V3.5L18.5,9M6,2C4.89,2 4,2.89 4,4V20A2,2 0 0,0 6,22H18A2,2 0 0,0 20,20V8L14,2H6Z";
const TREE_VIEW_FOLDER_ICON: &str =
    "M10,4H4C2.89,4 2,4.89 2,6V18A2,2 0 0,0 4,20H20A2,2 0 0,0 22,18V8C22,6.89 21.1,6 20,6H12L10,4Z";
const TREE_VIEW_CONFIG_ICON: &str = "M12,15.5A3.5,3.5 0 0,1 8.5,12A3.5,3.5 0 0,1 12,8.5A3.5,3.5 0 0,1 15.5,12A3.5,3.5 0 0,1 12,15.5M19.43,12.97C19.47,12.65 19.5,12.33 19.5,12C19.5,11.67 19.47,11.34 19.43,11L21.54,9.37C21.73,9.22 21.78,8.95 21.66,8.73L19.66,5.27C19.54,5.05 19.27,4.96 19.05,5.05L16.56,6.05C16.04,5.66 15.5,5.32 14.87,5.07L14.5,2.42C14.46,2.18 14.25,2 14,2H10C9.75,2 9.54,2.18 9.5,2.42L9.13,5.07C8.5,5.32 7.96,5.66 7.44,6.05L4.95,5.05C4.73,4.96 4.46,5.05 4.34,5.27L2.34,8.73C2.21,8.95 2.27,9.22 2.46,9.37L4.57,11C4.53,11.34 4.5,11.67 4.5,12C4.5,12.33 4.53,12.65 4.57,12.97L2.46,14.63C2.27,14.78 2.21,15.05 2.34,15.27L4.34,18.73C4.46,18.95 4.73,19.03 4.95,18.95L7.44,17.94C7.96,18.34 8.5,18.68 9.13,18.93L9.5,21.58C9.54,21.82 9.75,22 10,22H14C14.25,22 14.46,21.82 14.5,21.58L14.87,18.93C15.5,18.67 16.04,18.34 16.56,17.94L19.05,18.95C19.27,19.03 19.54,18.95 19.66,18.73L21.66,15.27C21.78,15.05 21.73,14.78 21.54,14.63L19.43,12.97Z";
const TREE_VIEW_MARKDOWN_ICON: &str = "M20.56 18H3.44C2.65 18 2 17.37 2 16.59V7.41C2 6.63 2.65 6 3.44 6H20.56C21.35 6 22 6.63 22 7.41V16.59C22 17.37 21.35 18 20.56 18M6.81 15.19V11.53L8.73 13.88L10.65 11.53V15.19H12.58V8.81H10.65L8.73 11.16L6.81 8.81H4.89V15.19H6.81M19.69 12H17.77V8.81H15.85V12H13.92L16.81 15.28L19.69 12Z";
const TREE_VIEW_RUST_ICON: &str = "M21.9 11.7L21 11.2V11L21.7 10.3C21.8 10.2 21.8 10 21.7 9.9L21.6 9.8L20.7 9.5C20.7 9.4 20.7 9.3 20.6 9.3L21.2 8.5C21.3 8.4 21.3 8.2 21.1 8.1C21.1 8.1 21 8.1 21 8L20 7.8C20 7.7 19.9 7.7 19.9 7.6L20.3 6.7V6.4C20.2 6.3 20.1 6.3 20 6.3H19C19 6.3 19 6.2 18.9 6.2L19.1 5.2C19.1 5 19 4.9 18.9 4.9H18.8L17.8 5.1C17.8 5 17.7 5 17.6 4.9V3.9C17.6 3.7 17.5 3.6 17.3 3.6H17.2L16.3 4H16.2L16 3C16 2.8 15.8 2.7 15.7 2.8H15.6L14.8 3.4C14.7 3.4 14.6 3.4 14.6 3.3L14.3 2.4C14.2 2.3 14.1 2.2 13.9 2.2C13.9 2.2 13.8 2.2 13.8 2.3L13 3H12.8L12.3 2.2C12.2 2 12 2 11.8 2L11.7 2.1L11.2 3H11L10.3 2.3C10.2 2.2 10 2.2 9.9 2.3L9.8 2.4L9.5 3.3C9.4 3.3 9.3 3.3 9.3 3.4L8.5 2.8C8.3 2.7 8.1 2.7 8 2.9V3L7.8 4C7.8 4 7.7 4 7.6 4.1L6.7 3.7C6.6 3.6 6.4 3.7 6.3 3.8V4.9C6.3 5 6.2 5 6.2 5.1L5.2 4.9C5 4.8 4.9 4.9 4.9 5.1V5.2L5.1 6.2C5 6.2 5 6.3 4.9 6.3H3.9C3.7 6.3 3.6 6.4 3.6 6.6V6.7L4 7.6V7.8L3 8C2.8 8 2.7 8.2 2.7 8.3V8.4L3.3 9.2C3.3 9.3 3.3 9.4 3.2 9.4L2.4 9.8C2.3 9.9 2.2 10 2.2 10.2C2.2 10.2 2.2 10.3 2.3 10.3L3 11V11.2L2.2 11.7C2 11.8 2 12 2 12.1L2.1 12.2L3 12.8V13L2.3 13.7C2.2 13.8 2.2 14 2.3 14.1L2.4 14.2L3.3 14.5C3.3 14.6 3.3 14.7 3.4 14.7L2.8 15.5C2.7 15.6 2.7 15.8 2.9 15.9C2.9 15.9 3 15.9 3 16L4 16.2C4 16.3 4.1 16.3 4.1 16.4L3.7 17.3C3.6 17.4 3.7 17.6 3.8 17.7H4.9C5 17.7 5 17.8 5.1 17.8L4.9 18.8C4.9 19 5 19.1 5.1 19.1H5.2L6.2 18.9C6.2 19 6.3 19 6.4 19.1V20.1C6.4 20.3 6.5 20.4 6.7 20.4H6.8L7.7 20H7.8L8 21C8 21.2 8.2 21.3 8.3 21.2H8.4L9.2 20.6C9.3 20.6 9.4 20.6 9.4 20.7L9.7 21.6C9.8 21.7 9.9 21.8 10.1 21.8C10.1 21.8 10.2 21.8 10.2 21.7L11 21H11.2L11.7 21.8C11.8 21.9 12 22 12.1 21.9L12.2 21.8L12.7 21H12.9L13.6 21.7C13.7 21.8 13.9 21.8 14 21.7L14.1 21.6L14.4 20.7C14.5 20.7 14.6 20.7 14.6 20.6L15.4 21.2C15.5 21.3 15.7 21.3 15.8 21.1C15.8 21.1 15.8 21 15.9 21L16.1 20C16.2 20 16.2 19.9 16.3 19.9L17.2 20.3C17.3 20.4 17.5 20.3 17.6 20.2V19.1L17.8 18.9L18.8 19.1C19 19.1 19.1 19 19.1 18.9V18.8L18.9 17.8L19.1 17.6H20.1C20.3 17.6 20.4 17.5 20.4 17.3V17.2L20 16.3C20 16.2 20.1 16.2 20.1 16.1L21.1 15.9C21.3 15.9 21.4 15.7 21.3 15.6V15.5L20.7 14.7L20.8 14.5L21.7 14.2C21.8 14.1 21.9 14 21.9 13.8C21.9 13.8 21.9 13.7 21.8 13.7L21 13V12.8L21.8 12.3C22 12.2 22 12 21.9 11.7C21.9 11.8 21.9 11.8 21.9 11.7M16.2 18.7C15.9 18.6 15.7 18.3 15.7 18C15.8 17.7 16.1 17.5 16.4 17.5C16.7 17.6 16.9 17.9 16.9 18.2C16.9 18.6 16.6 18.8 16.2 18.7M16 16.8C15.7 16.7 15.4 16.9 15.4 17.2L15 18.6C14.1 19 13.1 19.2 12 19.2C10.9 19.2 9.9 19 8.9 18.5L8.6 17.1C8.5 16.8 8.3 16.6 8 16.7L6.8 17C6.6 16.8 6.4 16.5 6.2 16.3H12.2C12.3 16.3 12.3 16.3 12.3 16.2V14.1C12.3 14 12.3 14 12.2 14H10.5V12.7H12.4C12.6 12.7 13.3 12.7 13.6 13.7C13.7 14 13.8 15 14 15.3C14.1 15.6 14.6 16.3 15.1 16.3H18.2C18 16.6 17.8 16.8 17.5 17.1L16 16.8M7.7 18.7C7.4 18.8 7.1 18.6 7 18.2C6.9 17.9 7.1 17.6 7.5 17.5S8.1 17.6 8.2 18C8.2 18.3 8 18.6 7.7 18.7M5.4 9.5C5.5 9.8 5.4 10.2 5.1 10.3C4.8 10.4 4.4 10.3 4.3 10C4.2 9.7 4.3 9.3 4.6 9.2C5 9.1 5.3 9.2 5.4 9.5M4.7 11.1L6 10.6C6.3 10.5 6.4 10.2 6.3 9.9L6 9.3H7V14H5C4.7 13 4.6 12.1 4.7 11.1M10.3 10.7V9.3H12.8C12.9 9.3 13.7 9.4 13.7 10C13.7 10.5 13.1 10.7 12.6 10.7H10.3M19.3 11.9V12.4H18.5C18.4 12.4 18.4 12.4 18.4 12.5V12.8C18.4 13.6 17.9 13.8 17.5 13.8C17.1 13.8 16.7 13.6 16.6 13.4C16.4 12.1 16 11.9 15.4 11.4C16.1 10.9 16.9 10.2 16.9 9.3C16.9 8.3 16.2 7.7 15.8 7.4C15.1 7 14.4 6.9 14.2 6.9H6.6C7.7 5.7 9.1 4.9 10.7 4.6L11.6 5.6C11.8 5.8 12.1 5.8 12.4 5.6L13.4 4.6C15.5 5 17.3 6.3 18.4 8.2L17.7 9.8C17.6 10.1 17.7 10.4 18 10.5L19.3 11.1V11.9M11.6 3.9C11.8 3.7 12.2 3.7 12.4 3.9C12.6 4.1 12.6 4.5 12.4 4.7C12.1 5 11.8 5 11.5 4.7C11.3 4.5 11.4 4.2 11.6 3.9M18.5 9.5C18.6 9.2 19 9.1 19.3 9.2C19.6 9.3 19.7 9.7 19.6 10C19.5 10.3 19.1 10.4 18.8 10.3C18.5 10.2 18.4 9.8 18.5 9.5Z";

fn tree_view_icon_path(icon_id: &str) -> &'static str {
    match icon_id {
        "folder" => TREE_VIEW_FOLDER_ICON,
        "file" => TREE_VIEW_FILE_ICON,
        "rust" => TREE_VIEW_RUST_ICON,
        "config" => TREE_VIEW_CONFIG_ICON,
        "markdown" => TREE_VIEW_MARKDOWN_ICON,
        "database" => {
            "M12,3C7.58,3 4,4.79 4,7C4,9.21 7.58,11 12,11C16.42,11 20,9.21 20,7C20,4.79 16.42,3 12,3M4,9V12C4,14.21 7.58,16 12,16C16.42,16 20,14.21 20,12V9C20,11.21 16.42,13 12,13C7.58,13 4,11.21 4,9M4,14V17C4,19.21 7.58,21 12,21C16.42,21 20,19.21 20,17V14C20,16.21 16.42,18 12,18C7.58,18 4,16.21 4,14Z"
        }
        "json" => {
            "M5,3H7V5H5V10A2,2 0 0,1 3,12A2,2 0 0,1 5,14V19H7V21H5C3.93,20.73 3,20.1 3,19V15A2,2 0 0,0 1,13H0V11H1A2,2 0 0,0 3,9V5A2,2 0 0,1 5,3M19,3A2,2 0 0,1 21,5V9A2,2 0 0,0 23,11H24V13H23A2,2 0 0,0 21,15V19A2,2 0 0,1 19,21H17V19H19V14A2,2 0 0,1 21,12A2,2 0 0,1 19,10V5H17V3H19M12,15A1,1 0 0,1 13,16A1,1 0 0,1 12,17A1,1 0 0,1 11,16A1,1 0 0,1 12,15M8,15A1,1 0 0,1 9,16A1,1 0 0,1 8,17A1,1 0 0,1 7,16A1,1 0 0,1 8,15M16,15A1,1 0 0,1 17,16A1,1 0 0,1 16,17A1,1 0 0,1 15,16A1,1 0 0,1 16,15Z"
        }
        "lock" => {
            "M12,17A2,2 0 0,0 14,15C14,13.89 13.1,13 12,13A2,2 0 0,0 10,15A2,2 0 0,0 12,17M18,8A2,2 0 0,1 20,10V20A2,2 0 0,1 18,22H6A2,2 0 0,1 4,20V10C4,8.89 4.9,8 6,8H7V6A5,5 0 0,1 12,1A5,5 0 0,1 17,6V8H18M12,3A3,3 0 0,0 9,6V8H15V6A3,3 0 0,0 12,3Z"
        }
        "terminal" => {
            "M20,19V7H4V19H20M20,3A2,2 0 0,1 22,5V19A2,2 0 0,1 20,21H4A2,2 0 0,1 2,19V5C2,3.89 2.9,3 4,3H20M13,17V15H18V17H13M9.58,13L5.57,9H8.4L11.7,12.3C12.09,12.69 12.09,13.33 11.7,13.72L8.42,17H5.59L9.58,13Z"
        }
        "git" => {
            "M2.6,10.59L8.38,4.8L10.07,6.5C9.83,7.35 10.22,8.28 11,8.73V14.27C10.4,14.61 10,15.26 10,16A2,2 0 0,0 12,18A2,2 0 0,0 14,16C14,15.26 13.6,14.61 13,14.27V9.41L15.07,11.5C15,11.65 15,11.82 15,12A2,2 0 0,0 17,14A2,2 0 0,0 19,12A2,2 0 0,0 17,10C16.82,10 16.65,10 16.5,10.07L13.93,7.5C14.19,6.57 13.71,5.55 12.78,5.16C12.35,5 11.9,4.96 11.5,5.07L9.8,3.38L10.59,2.6C11.37,1.81 12.63,1.81 13.41,2.6L21.4,10.59C22.19,11.37 22.19,12.63 21.4,13.41L13.41,21.4C12.63,22.19 11.37,22.19 10.59,21.4L2.6,13.41C1.81,12.63 1.81,11.37 2.6,10.59Z"
        }
        _ => TREE_VIEW_FILE_ICON,
    }
}

fn tree_view_show_icon(node: &crate::layout::TreeViewNodeLayout) -> Option<&str> {
    node.icon_id.as_deref().filter(|icon_id| *icon_id != "none")
}

fn render_tree_view_icon_defs(layout: &crate::layout::TreeViewLayout) -> String {
    let mut used_icons: Vec<&str> = Vec::new();
    for node in &layout.nodes {
        let Some(icon_id) = tree_view_show_icon(node) else {
            continue;
        };
        if !used_icons.contains(&icon_id) {
            used_icons.push(icon_id);
        }
    }
    if used_icons.is_empty() {
        return String::new();
    }

    let mut svg = String::from("<defs>");
    for icon_id in used_icons {
        svg.push_str(&format!(
            "<symbol id=\"tv-icon-my-svg-{icon_id}\" viewBox=\"0 0 24 24\"><path d=\"{path}\"/></symbol>",
            icon_id = escape_xml(icon_id),
            path = tree_view_icon_path(icon_id)
        ));
    }
    svg.push_str("</defs>");
    svg
}

fn render_tree_view(layout: &crate::layout::TreeViewLayout, theme: &Theme) -> String {
    let mut svg = String::new();
    let font_family = normalize_font_family(&theme.font_family);
    let font_size = theme.font_size;

    svg.push_str(&format!(
        "<style>.treeView-node-label{{font-size:{font_size}px;fill:black;}}.treeView-node-dir{{font-weight:bold;}}.treeView-node-line{{stroke:black;}}.treeView-node-icon{{fill:#546e7a;}}.treeView-node-description{{font-size:{font_size}px;fill:#6a9955;font-style:italic;}}.treeView-highlight-bg{{fill:rgba(255,193,7,0.15);stroke:#ffc107;stroke-width:1;}}</style>",
    ));
    svg.push_str(&render_tree_view_icon_defs(layout));
    svg.push_str("<g class=\"tree-view\">");

    for line in &layout.lines {
        svg.push_str(&format!(
            "<line class=\"treeView-node-line\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" \
             stroke=\"black\" stroke-width=\"1\"/>",
            line.x1, line.y1, line.x2, line.y2,
        ));
    }

    for node in &layout.nodes {
        svg.push_str("<g>");
        if let Some(highlight_width) = node.highlight_width {
            svg.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"3\" class=\"treeView-highlight-bg\"/>",
                node.x,
                node.y + 1.0,
                highlight_width,
                node.height - 2.0,
            ));
        }

        // Mermaid CLI output keeps icon symbols and reserves this gutter, but
        // the serialized comparison SVGs do not include visible <use> nodes.

        let mut class = String::from("treeView-node-label");
        if node.node_type == crate::ir::TreeViewNodeType::Directory {
            class.push_str(" treeView-node-dir");
        }
        if let Some(css_class) = &node.css_class {
            class.push(' ');
            class.push_str(css_class);
        }
        let class_attr = escape_xml(&class);
        svg.push_str(&format!(
            "<text class=\"{class}\" x=\"{:.1}\" y=\"{:.1}\" \
             dominant-baseline=\"middle\" font-family=\"{ff}\" font-size=\"{fs}\" fill=\"black\">{text}</text>",
            node.label_x,
            node.y + node.height / 2.0,
            class = class_attr,
            ff = font_family,
            fs = font_size,
            text = escape_xml(&node.name),
        ));

        if let (Some(description), Some(description_x)) = (&node.description, node.description_x) {
            svg.push_str(&format!(
                "<text class=\"treeView-node-description\" x=\"{:.1}\" y=\"{:.1}\" dominant-baseline=\"middle\" font-family=\"{ff}\" font-size=\"{fs}\" fill=\"#6a9955\" font-style=\"italic\">{text}</text>",
                description_x,
                node.y + node.height / 2.0,
                ff = font_family,
                fs = font_size,
                text = escape_xml(description),
            ));
        }

        svg.push_str("</g>");
    }
    svg.push_str("</g>");

    if let Some(ref title) = layout.title {
        svg.push_str(&format!(
            "<text x=\"5\" y=\"-10\" font-family=\"{ff}\" font-size=\"{fs}\" \
             font-weight=\"bold\" fill=\"black\">{text}</text>",
            ff = font_family,
            fs = font_size * 1.2,
            text = escape_xml(title),
        ));
    }

    svg
}

// ── Ishikawa (fishbone) renderer ────────────────────────────────────────

/// Character-count word wrapping matching JS ishikawa wrapText().
/// Greedy packing: adds words to current line if they fit within max_chars.
fn wrap_by_chars(text: &str, max_chars: usize) -> Vec<String> {
    if text.len() <= max_chars {
        return vec![text.to_string()];
    }
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        if let Some(last) = lines.last_mut() {
            if last.len() + 1 + word.len() <= max_chars {
                last.push(' ');
                last.push_str(word);
                continue;
            }
        }
        lines.push(word.to_string());
    }
    if lines.is_empty() {
        vec![text.to_string()]
    } else {
        lines
    }
}

fn render_ishikawa(layout: &crate::layout::IshikawaLayout, theme: &Theme) -> String {
    let mut svg = String::new();
    let font_family = normalize_font_family(&theme.font_family);
    let font_size = theme.font_size;
    let line_color = "#333333";
    let box_fill = "#ECECFF"; // JS uses this for head and label boxes

    svg.push_str("<g class=\"ishikawa\">");

    // Arrow marker definition (pointing toward spine = cause→effect)
    svg.push_str(&format!(
        "<defs><marker id=\"ishikawa-arrow\" viewBox=\"0 0 10 10\" refX=\"0\" refY=\"5\" \
         markerWidth=\"6\" markerHeight=\"6\" orient=\"auto\">\
         <path d=\"M 10 0 L 0 5 L 10 10 Z\" class=\"ishikawa-arrow\" fill=\"{lc}\"/></marker></defs>",
        lc = line_color,
    ));

    // Spine
    svg.push_str(&format!(
        "<line class=\"ishikawa-spine\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" \
         stroke=\"{lc}\" stroke-width=\"{sw}\" fill=\"none\"/>",
        layout.spine.x1,
        layout.spine.y1,
        layout.spine.x2,
        layout.spine.y2,
        lc = line_color,
        sw = layout.spine.stroke_width,
    ));

    // Fish head path — in a group translated to spine center (matching JS).
    if !layout.head_path.is_empty() {
        svg.push_str(&format!(
            "<g class=\"ishikawa-head-group\" transform=\"translate(0,{hy})\">\
             <path class=\"ishikawa-head\" d=\"{path}\" fill=\"{fill}\" stroke=\"{lc}\" stroke-width=\"2\"/>\
             </g>",
            hy = layout.head_y,
            path = layout.head_path,
            fill = box_fill,
            lc = line_color,
        ));
    }

    // Branches (primary = stroke-width 2 with arrow, sub = stroke-width 1 with arrow)
    for branch in &layout.branches {
        let class = if branch.stroke_width >= 2.0 {
            "ishikawa-branch"
        } else {
            "ishikawa-sub-branch"
        };
        svg.push_str(&format!(
            "<line class=\"{class}\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" \
             stroke=\"{lc}\" stroke-width=\"{sw}\" fill=\"none\" \
             marker-start=\"url(#ishikawa-arrow)\"/>",
            branch.x1,
            branch.y1,
            branch.x2,
            branch.y2,
            lc = line_color,
            sw = branch.stroke_width,
        ));
    }

    // Label boxes first (behind text)
    for label in &layout.labels {
        if label.has_box {
            svg.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
                 class=\"ishikawa-label-box\" fill=\"{fill}\" stroke=\"{lc}\" stroke-width=\"2\"/>",
                label.box_x,
                label.box_y,
                label.box_w,
                label.box_h,
                fill = box_fill,
                lc = line_color,
            ));
        }
    }

    // Label text
    for label in &layout.labels {
        if label.font_weight == "600" {
            // Head label: rendered inside the <g> group centered on spine.
            // Use local coordinates (y=0 = spine center).
            // JS: text at x=0, y=-8.4, transform="translate(tx, ty)"
            //   tx = (w - textWidth)/2 - textBBox.x + 3
            //   ty = -textBBox.y - textHeight/2
            let head_fs = 14.0_f32;
            let lines = if label.lines.is_empty() {
                vec![label.text.clone()]
            } else {
                label.lines.clone()
            };

            let line_h = head_fs * 1.2; // 16.8 at 14px

            // Match JS exactly:
            // <text x="0" y="-8.4" transform="translate(tx, ty)">
            //   <tspan x="0" dy="0">Line1</tspan>
            //   <tspan x="0" dy="16.8">Line2</tspan>
            // y = -fontSize * 0.6 = -8.4
            // ty ≈ small positive value to nudge center down
            // tx = label.x (from layout, ≈ head_q_extent * 0.23 ≈ 33)
            let text_y = -head_fs * 0.6; // -8.4
            let ty = 1.34375_f32;

            let mut tspans = String::new();
            for (i, line) in lines.iter().enumerate() {
                let dy = if i == 0 { 0.0 } else { line_h };
                tspans.push_str(&format!(
                    "<tspan x=\"0\" dy=\"{dy}\">{}</tspan>",
                    escape_xml(line),
                ));
            }
            // Render head label in its own group at spine center
            svg.push_str(&format!(
                "<g transform=\"translate(0,{hy})\">\
                 <text class=\"ishikawa-head-label\" text-anchor=\"start\" x=\"0\" y=\"{text_y:.1}\" \
                 transform=\"translate({tx:.1},{ty:.1})\" \
                 font-family=\"{ff}\" font-size=\"{fs}\" font-weight=\"600\" \
                 fill=\"{tc}\">{tspans}</text></g>",
                hy = layout.head_y,
                text_y = text_y,
                tx = label.x,
                ty = ty,
                ff = font_family,
                fs = head_fs,
                tc = theme.primary_text_color,
                tspans = tspans,
            ));
        } else {
            // Mermaid wraps sub-bone labels before measuring/drawing, but cause
            // labels are left as a single line.
            let wrapped = if !label.lines.is_empty() {
                label.lines.clone()
            } else if label.has_box {
                vec![label.text.clone()]
            } else {
                wrap_by_chars(&label.text, 15)
            };
            if wrapped.len() <= 1 {
                let class = if label.has_box {
                    "ishikawa-label cause"
                } else {
                    "ishikawa-label align"
                };
                svg.push_str(&format!(
                    "<text class=\"{class}\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"{anchor}\" \
                     dominant-baseline=\"middle\" font-family=\"{ff}\" font-size=\"{fs}\" \
                     fill=\"{tc}\">{text}</text>",
                    label.x,
                    label.y,
                    anchor = label.anchor,
                    ff = font_family,
                    fs = font_size,
                    tc = theme.primary_text_color,
                    text = escape_xml(wrapped.first().map(String::as_str).unwrap_or(&label.text)),
                ));
            } else {
                let class = if label.has_box {
                    "ishikawa-label cause"
                } else {
                    "ishikawa-label align"
                };
                // Multi-line bone label with tspans
                let line_h = font_size * 1.2;
                let mut tspans = String::new();
                for (i, line) in wrapped.iter().enumerate() {
                    let dy = if i == 0 { 0.0 } else { line_h };
                    tspans.push_str(&format!(
                        "<tspan x=\"{:.1}\" dy=\"{dy}\">{}</tspan>",
                        label.x,
                        escape_xml(line),
                    ));
                }
                // Shift up by half the extra lines to keep vertically centered
                let shift_y = -((wrapped.len() as f32 - 1.0) * line_h) / 2.0;
                svg.push_str(&format!(
                    "<text class=\"{class}\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"{anchor}\" \
                     dominant-baseline=\"middle\" font-family=\"{ff}\" font-size=\"{fs}\" \
                     fill=\"{tc}\">{tspans}</text>",
                    label.x,
                    label.y + shift_y,
                    anchor = label.anchor,
                    ff = font_family,
                    fs = font_size,
                    tc = theme.primary_text_color,
                    tspans = tspans,
                ));
            }
        }
    }

    svg.push_str("</g>");
    svg
}

// ── Wardley map renderer ────────────────────────────────────────────────

fn render_wardley(layout: &crate::layout::WardleyLayout, theme: &Theme) -> String {
    let mut svg = String::new();
    let font_family = normalize_font_family(&theme.font_family);
    let axis_color = "#000";
    let evolution_color = "#dc3545";
    let node_fill = "#fff";

    // Background
    svg.push_str(&format!(
        "<rect width=\"{w}\" height=\"{h}\" fill=\"{bg}\"/>",
        w = layout.canvas_width,
        h = layout.canvas_height,
        bg = theme.background,
    ));

    // Title
    if let Some(ref title) = layout.title {
        svg.push_str(&format!(
            "<text x=\"{x}\" y=\"24\" text-anchor=\"middle\" font-family=\"{ff}\" \
             font-size=\"16\" font-weight=\"bold\" fill=\"{tc}\">{text}</text>",
            x = layout.canvas_width / 2.0,
            ff = font_family,
            tc = theme.primary_text_color,
            text = escape_xml(title),
        ));
    }

    // Axes
    let (cx, cy, cw, ch) = (
        layout.chart_x,
        layout.chart_y,
        layout.chart_width,
        layout.chart_height,
    );

    // X-axis (bottom)
    svg.push_str(&format!(
        "<line x1=\"{cx}\" y1=\"{y}\" x2=\"{x2}\" y2=\"{y}\" stroke=\"{c}\" stroke-width=\"1\"/>",
        cx = cx,
        y = cy + ch,
        x2 = cx + cw,
        c = axis_color,
    ));
    // Y-axis (left)
    svg.push_str(&format!(
        "<line x1=\"{cx}\" y1=\"{cy}\" x2=\"{cx}\" y2=\"{y2}\" stroke=\"{c}\" stroke-width=\"1\"/>",
        cx = cx,
        cy = cy,
        y2 = cy + ch,
        c = axis_color,
    ));

    // X-axis label
    svg.push_str(&format!(
        "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" font-family=\"{ff}\" \
         font-size=\"12\" fill=\"{tc}\">{label}</text>",
        x = cx + cw / 2.0,
        y = cy + ch + 40.0,
        ff = font_family,
        tc = theme.primary_text_color,
        label = escape_xml(&layout.x_label),
    ));
    // Y-axis label (rotated)
    svg.push_str(&format!(
        "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" font-family=\"{ff}\" \
         font-size=\"12\" fill=\"{tc}\" transform=\"rotate(-90, {x}, {y})\">{label}</text>",
        x = cx - 32.0,
        y = cy + ch / 2.0,
        ff = font_family,
        tc = theme.primary_text_color,
        label = escape_xml(&layout.y_label),
    ));

    // Stage dividers and labels
    for (i, stage) in layout.stages.iter().enumerate() {
        if i > 0 {
            svg.push_str(&format!(
                "<line x1=\"{x}\" y1=\"{cy}\" x2=\"{x}\" y2=\"{y2}\" \
                 stroke=\"{c}\" stroke-width=\"1\" stroke-dasharray=\"5 5\"/>",
                x = stage.divider_x,
                cy = cy,
                y2 = cy + ch,
                c = axis_color,
            ));
        }
        svg.push_str(&format!(
            "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" font-family=\"{ff}\" \
             font-size=\"10\" fill=\"{tc}\">{label}</text>",
            x = stage.label_x,
            y = cy + ch + 20.0,
            ff = font_family,
            tc = theme.primary_text_color,
            label = escape_xml(&stage.label),
        ));
    }

    // Arrow markers
    svg.push_str(
        "<defs>\
         <marker id=\"wardley-trend-arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" \
         markerWidth=\"6\" markerHeight=\"6\" orient=\"auto\">\
         <path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"#dc3545\"/></marker>\
         <marker id=\"wardley-link-arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" \
         markerWidth=\"6\" markerHeight=\"6\" orient=\"auto\">\
         <path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"#000\"/></marker>\
         </defs>",
    );

    // Links
    for link in &layout.links {
        let dash = if link.dashed {
            " stroke-dasharray=\"6 6\""
        } else {
            ""
        };
        let marker = match link.flow {
            Some(crate::ir::WardleyFlow::Forward) => " marker-end=\"url(#wardley-link-arrow)\"",
            Some(crate::ir::WardleyFlow::Backward) => " marker-start=\"url(#wardley-link-arrow)\"",
            Some(crate::ir::WardleyFlow::Bidirectional) => {
                " marker-start=\"url(#wardley-link-arrow)\" marker-end=\"url(#wardley-link-arrow)\""
            }
            None => "",
        };
        svg.push_str(&format!(
            "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" \
             stroke=\"{c}\" stroke-width=\"1\"{dash}{marker}/>",
            link.x1,
            link.y1,
            link.x2,
            link.y2,
            c = axis_color,
            dash = dash,
            marker = marker,
        ));
        if let Some(ref label) = link.label {
            let mx = (link.x1 + link.x2) / 2.0;
            let my = (link.y1 + link.y2) / 2.0 - 8.0;
            svg.push_str(&format!(
                "<text x=\"{mx:.1}\" y=\"{my:.1}\" text-anchor=\"middle\" \
                 font-family=\"{ff}\" font-size=\"10\" fill=\"{tc}\">{text}</text>",
                ff = font_family,
                tc = theme.primary_text_color,
                text = escape_xml(label),
            ));
        }
    }

    // Trends (red dashed arrows)
    for trend in &layout.trends {
        svg.push_str(&format!(
            "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" \
             stroke=\"{c}\" stroke-width=\"1\" stroke-dasharray=\"4 4\" \
             marker-end=\"url(#wardley-trend-arrow)\"/>",
            trend.x1,
            trend.y1,
            trend.x2,
            trend.y2,
            c = evolution_color,
        ));
    }

    // Nodes
    for node in &layout.nodes {
        // Strategy overlay
        if let Some(strategy) = node.strategy {
            let overlay_r = node.radius * 2.0;
            let fill = match strategy {
                crate::ir::WardleyStrategy::Build => "#eee",
                crate::ir::WardleyStrategy::Buy => "#ccc",
                crate::ir::WardleyStrategy::Outsource => "#666",
                crate::ir::WardleyStrategy::Market => "#fff",
            };
            svg.push_str(&format!(
                "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" \
                 fill=\"{fill}\" stroke=\"{c}\" stroke-width=\"1\"/>",
                node.cx,
                node.cy,
                overlay_r,
                fill = fill,
                c = axis_color,
            ));
        }

        // Main node circle
        svg.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{r}\" \
             fill=\"{fill}\" stroke=\"{c}\" stroke-width=\"1\"/>",
            node.cx,
            node.cy,
            r = node.radius,
            fill = node_fill,
            c = axis_color,
        ));

        // Inertia indicator
        if node.inertia {
            let ix = node.cx + node.radius + 15.0;
            svg.push_str(&format!(
                "<line x1=\"{ix:.1}\" y1=\"{:.1}\" x2=\"{ix:.1}\" y2=\"{:.1}\" \
                 stroke=\"{c}\" stroke-width=\"6\"/>",
                node.cy - node.radius,
                node.cy + node.radius,
                c = axis_color,
            ));
        }

        // Label
        let weight = if node.is_anchor {
            " font-weight=\"bold\""
        } else {
            ""
        };
        svg.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" font-family=\"{ff}\" \
             font-size=\"10\"{weight} fill=\"{tc}\">{text}</text>",
            node.label_x,
            node.label_y,
            ff = font_family,
            weight = weight,
            tc = theme.primary_text_color,
            text = escape_xml(&node.label),
        ));
    }

    // Notes
    for (text, x, y) in &layout.notes {
        svg.push_str(&format!(
            "<text x=\"{x:.1}\" y=\"{y:.1}\" font-family=\"{ff}\" \
             font-size=\"11\" font-weight=\"bold\" fill=\"{tc}\">{text}</text>",
            ff = font_family,
            tc = theme.primary_text_color,
            text = escape_xml(text),
        ));
    }

    svg
}
