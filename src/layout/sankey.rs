use std::collections::{BTreeMap, HashMap, HashSet};

use crate::config::{LayoutConfig, SankeyNodeAlignment};
use crate::ir::Graph;
use crate::theme::Theme;

use super::text::measure_label;
use super::{
    DiagramData, EdgeLayout, Layout, NodeLayout, SankeyLayout, SankeyLinkLayout, SankeyNodeLayout,
};

const SANKEY_ITERATIONS: usize = 6;
const SANKEY_PALETTE: [&str; 10] = [
    "#4e79a7", "#f28e2c", "#e15759", "#76b7b2", "#59a14f", "#edc949", "#af7aa1", "#ff9da7",
    "#9c755f", "#bab0ab",
];

#[derive(Debug, Clone)]
struct SankeyNodeData {
    id: String,
    label: String,
    value: f64,
    depth: usize,
    height: usize,
    layer: usize,
    source_links: Vec<usize>,
    target_links: Vec<usize>,
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
    color: String,
}

#[derive(Debug, Clone)]
struct SankeyLinkData {
    source: usize,
    target: usize,
    value: f64,
    width: f64,
    y0: f64,
    y1: f64,
    index: usize,
}

pub(super) fn compute_sankey_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    let sankey_config = &config.sankey;
    let node_width = f64::from(sankey_config.node_width.max(0.0));
    let width = f64::from(sankey_config.width).max(node_width);
    let height = f64::from(sankey_config.height.max(1.0));
    let node_padding = f64::from(sankey_config.node_padding.max(0.0))
        + if sankey_config.show_values { 15.0 } else { 0.0 };

    let mut node_ids: Vec<String> = graph.nodes.keys().cloned().collect();
    node_ids.sort_by(|a, b| {
        let order_a = graph.node_order.get(a).copied().unwrap_or(usize::MAX);
        let order_b = graph.node_order.get(b).copied().unwrap_or(usize::MAX);
        order_a.cmp(&order_b).then_with(|| a.cmp(b))
    });

    let mut id_to_idx: HashMap<String, usize> = HashMap::new();
    for (idx, id) in node_ids.iter().enumerate() {
        id_to_idx.insert(id.clone(), idx);
    }

    let mut nodes: Vec<SankeyNodeData> = node_ids
        .iter()
        .enumerate()
        .map(|(idx, id)| {
            let label = graph
                .nodes
                .get(id)
                .map(|node| node.label.clone())
                .unwrap_or_else(|| id.clone());
            SankeyNodeData {
                id: id.clone(),
                label,
                value: 0.0,
                depth: 0,
                height: 0,
                layer: 0,
                source_links: Vec::new(),
                target_links: Vec::new(),
                x0: 0.0,
                x1: node_width,
                y0: 0.0,
                y1: 0.0,
                color: SANKEY_PALETTE[idx % SANKEY_PALETTE.len()].to_string(),
            }
        })
        .collect();

    let mut links = Vec::new();
    for edge in &graph.edges {
        let Some(&source) = id_to_idx.get(&edge.from) else {
            continue;
        };
        let Some(&target) = id_to_idx.get(&edge.to) else {
            continue;
        };
        let raw_value = edge
            .label
            .as_deref()
            .and_then(|text| text.parse::<f64>().ok())
            .unwrap_or(1.0);
        let value = raw_value.max(0.0);
        let index = links.len();
        links.push(SankeyLinkData {
            source,
            target,
            value,
            width: 0.0,
            y0: 0.0,
            y1: 0.0,
            index,
        });
        nodes[source].source_links.push(index);
        nodes[target].target_links.push(index);
    }

    compute_node_values(&mut nodes, &links);
    compute_node_depths(&mut nodes, &links);
    compute_node_heights(&mut nodes, &links);
    let mut columns = compute_node_layers(
        &mut nodes,
        &links,
        width,
        node_width,
        &sankey_config.node_alignment,
    );
    compute_node_breadths(&mut nodes, &mut links, &mut columns, height, node_padding);
    compute_link_breadths(&nodes, &mut links);

    let mut layout_nodes = BTreeMap::new();
    let mut sankey_nodes = Vec::with_capacity(nodes.len());
    for node in &nodes {
        let mut style = crate::ir::NodeStyle::default();
        style.fill = Some(node.color.clone());
        style.stroke = Some("none".to_string());
        style.stroke_width = Some(0.0);
        layout_nodes.insert(
            node.id.clone(),
            NodeLayout {
                id: node.id.clone(),
                x: node.x0 as f32,
                y: node.y0 as f32,
                width: (node.x1 - node.x0) as f32,
                height: (node.y1 - node.y0) as f32,
                label: measure_label(&node.label, theme, config),
                shape: crate::ir::NodeShape::Rectangle,
                style,
                link: graph.node_links.get(&node.id).cloned(),
                anchor_subgraph: None,
                hidden: false,
                icon: None,
                img: None,
                img_w: None,
                img_h: None,
                sub_label: None,
                is_treemap_leaf: false,
                treemap_base_text_color: None,
            },
        );
        sankey_nodes.push(SankeyNodeLayout {
            id: node.id.clone(),
            label: node.label.clone(),
            total: node.value as f32,
            rank: node.layer,
            x: node.x0 as f32,
            y: node.y0 as f32,
            width: (node.x1 - node.x0) as f32,
            height: (node.y1 - node.y0) as f32,
            color: node.color.clone(),
        });
    }

    let mut layout_edges = Vec::with_capacity(links.len());
    let mut sankey_links = Vec::with_capacity(links.len());
    for link in &links {
        let source = &nodes[link.source];
        let target = &nodes[link.target];
        let start = (source.x1 as f32, link.y0 as f32);
        let end = (target.x0 as f32, link.y1 as f32);
        let thickness = link.width as f32;
        let gradient_id = format!("linearGradient-{}", nodes.len() + link.index + 1);

        layout_edges.push(EdgeLayout {
            from: source.id.clone(),
            to: target.id.clone(),
            label: None,
            start_label: None,
            end_label: None,
            label_anchor: None,
            start_label_anchor: None,
            end_label_anchor: None,
            points: vec![start, end],
            directed: false,
            arrow_start: false,
            arrow_end: false,
            arrow_start_kind: None,
            arrow_end_kind: None,
            start_decoration: None,
            end_decoration: None,
            sequence_arrow_end: None,
            sequence_arrow_start: None,
            style: crate::ir::EdgeStyle::Solid,
            override_style: crate::ir::EdgeStyleOverride {
                stroke: Some(source.color.clone()),
                stroke_width: Some(thickness.max(1.0)),
                dasharray: None,
                label_color: None,
            },
            curve: None,
        });
        sankey_links.push(SankeyLinkLayout {
            source: source.id.clone(),
            target: target.id.clone(),
            value: link.value as f32,
            thickness,
            start,
            end,
            color_start: source.color.clone(),
            color_end: target.color.clone(),
            gradient_id,
        });
    }

    Layout {
        kind: graph.kind,
        nodes: layout_nodes,
        edges: layout_edges,
        subgraphs: Vec::new(),
        width: width as f32,
        height: height as f32,
        acc_title: None,
        acc_descr: None,
        diagram: DiagramData::Sankey(SankeyLayout {
            width: width as f32,
            height: height as f32,
            node_width: node_width as f32,
            show_values: sankey_config.show_values,
            prefix: sankey_config.prefix.clone(),
            suffix: sankey_config.suffix.clone(),
            link_color: sankey_config.link_color.clone(),
            use_max_width: sankey_config.use_max_width,
            nodes: sankey_nodes,
            links: sankey_links,
        }),
    }
}

fn compute_node_values(nodes: &mut [SankeyNodeData], links: &[SankeyLinkData]) {
    for idx in 0..nodes.len() {
        let source_total = nodes[idx]
            .source_links
            .iter()
            .map(|link_idx| links[*link_idx].value)
            .sum::<f64>();
        let target_total = nodes[idx]
            .target_links
            .iter()
            .map(|link_idx| links[*link_idx].value)
            .sum::<f64>();
        nodes[idx].value = source_total.max(target_total);
    }
}

fn compute_node_depths(nodes: &mut [SankeyNodeData], links: &[SankeyLinkData]) {
    let n = nodes.len();
    let mut current: Vec<usize> = (0..n).collect();
    let mut depth = 0usize;
    while !current.is_empty() {
        let mut next = Vec::new();
        let mut seen = HashSet::new();
        for &node_idx in &current {
            nodes[node_idx].depth = depth;
            for &link_idx in &nodes[node_idx].source_links {
                let target = links[link_idx].target;
                if seen.insert(target) {
                    next.push(target);
                }
            }
        }
        depth += 1;
        if depth > n {
            break;
        }
        current = next;
    }
}

fn compute_node_heights(nodes: &mut [SankeyNodeData], links: &[SankeyLinkData]) {
    let n = nodes.len();
    let mut current: Vec<usize> = (0..n).collect();
    let mut height = 0usize;
    while !current.is_empty() {
        let mut next = Vec::new();
        let mut seen = HashSet::new();
        for &node_idx in &current {
            nodes[node_idx].height = height;
            for &link_idx in &nodes[node_idx].target_links {
                let source = links[link_idx].source;
                if seen.insert(source) {
                    next.push(source);
                }
            }
        }
        height += 1;
        if height > n {
            break;
        }
        current = next;
    }
}

fn compute_node_layers(
    nodes: &mut [SankeyNodeData],
    links: &[SankeyLinkData],
    width: f64,
    node_width: f64,
    alignment: &SankeyNodeAlignment,
) -> Vec<Vec<usize>> {
    let column_count = nodes.iter().map(|node| node.depth).max().unwrap_or(0) + 1;
    let kx = if column_count > 1 {
        (width - node_width) / (column_count - 1) as f64
    } else {
        0.0
    };
    let mut columns = vec![Vec::new(); column_count];

    for node_idx in 0..nodes.len() {
        let aligned = match alignment {
            SankeyNodeAlignment::Left => nodes[node_idx].depth as isize,
            SankeyNodeAlignment::Right => {
                column_count as isize - 1 - nodes[node_idx].height as isize
            }
            SankeyNodeAlignment::Center => {
                if !nodes[node_idx].target_links.is_empty() {
                    nodes[node_idx].depth as isize
                } else if !nodes[node_idx].source_links.is_empty() {
                    nodes[node_idx]
                        .source_links
                        .iter()
                        .map(|link_idx| nodes[links[*link_idx].target].depth)
                        .min()
                        .unwrap_or(1)
                        .saturating_sub(1) as isize
                } else {
                    0
                }
            }
            SankeyNodeAlignment::Justify => {
                if nodes[node_idx].source_links.is_empty() {
                    column_count as isize - 1
                } else {
                    nodes[node_idx].depth as isize
                }
            }
        };
        let layer = aligned.clamp(0, column_count.saturating_sub(1) as isize) as usize;
        nodes[node_idx].layer = layer;
        nodes[node_idx].x0 = layer as f64 * kx;
        nodes[node_idx].x1 = nodes[node_idx].x0 + node_width;
        columns[layer].push(node_idx);
    }

    columns
}

fn compute_node_breadths(
    nodes: &mut [SankeyNodeData],
    links: &mut [SankeyLinkData],
    columns: &mut [Vec<usize>],
    height: f64,
    node_padding: f64,
) -> f64 {
    let max_column_len = columns.iter().map(|column| column.len()).max().unwrap_or(0);
    let py = if max_column_len > 1 {
        node_padding.min(height / (max_column_len - 1) as f64)
    } else {
        node_padding
    };
    initialize_node_breadths(nodes, links, columns, height, py);
    for i in 0..SANKEY_ITERATIONS {
        let alpha = 0.99_f64.powi(i as i32);
        let beta = (1.0 - alpha).max((i + 1) as f64 / SANKEY_ITERATIONS as f64);
        relax_right_to_left(nodes, links, columns, height, py, alpha, beta);
        relax_left_to_right(nodes, links, columns, height, py, alpha, beta);
    }
    py
}

fn initialize_node_breadths(
    nodes: &mut [SankeyNodeData],
    links: &mut [SankeyLinkData],
    columns: &mut [Vec<usize>],
    height: f64,
    py: f64,
) {
    let mut ky = f64::INFINITY;
    for column in columns.iter() {
        let column_total = column.iter().map(|idx| nodes[*idx].value).sum::<f64>();
        if column_total <= 0.0 {
            continue;
        }
        let available = height - column.len().saturating_sub(1) as f64 * py;
        ky = ky.min(available / column_total);
    }
    if !ky.is_finite() {
        ky = 1.0;
    }

    for column in columns.iter() {
        let mut y = 0.0;
        for &node_idx in column {
            nodes[node_idx].y0 = y;
            nodes[node_idx].y1 = y + nodes[node_idx].value * ky;
            y = nodes[node_idx].y1 + py;
            for &link_idx in &nodes[node_idx].source_links {
                links[link_idx].width = links[link_idx].value * ky;
            }
        }
        y = (height - y + py) / (column.len() + 1) as f64;
        for (idx, &node_idx) in column.iter().enumerate() {
            let dy = y * (idx + 1) as f64;
            nodes[node_idx].y0 += dy;
            nodes[node_idx].y1 += dy;
        }
        reorder_links(column, nodes, links);
    }
}

fn relax_left_to_right(
    nodes: &mut [SankeyNodeData],
    links: &[SankeyLinkData],
    columns: &mut [Vec<usize>],
    height: f64,
    py: f64,
    alpha: f64,
    beta: f64,
) {
    for column_idx in 1..columns.len() {
        let column_nodes = columns[column_idx].clone();
        for target in column_nodes {
            let mut y = 0.0;
            let mut w = 0.0;
            for &link_idx in &nodes[target].target_links {
                let source = links[link_idx].source;
                let v = links[link_idx].value
                    * (nodes[target].layer as f64 - nodes[source].layer as f64);
                y += target_top(source, target, nodes, links, py) * v;
                w += v;
            }
            if w <= 0.0 {
                continue;
            }
            let dy = (y / w - nodes[target].y0) * alpha;
            nodes[target].y0 += dy;
            nodes[target].y1 += dy;
            reorder_node_links(target, nodes, links);
        }
        columns[column_idx].sort_by(|a, b| compare_node_breadth(nodes, *a, *b));
        resolve_collisions(&columns[column_idx], nodes, height, py, beta);
    }
}

fn relax_right_to_left(
    nodes: &mut [SankeyNodeData],
    links: &[SankeyLinkData],
    columns: &mut [Vec<usize>],
    height: f64,
    py: f64,
    alpha: f64,
    beta: f64,
) {
    if columns.len() < 2 {
        return;
    }
    for column_idx in (0..columns.len() - 1).rev() {
        let column_nodes = columns[column_idx].clone();
        for source in column_nodes {
            let mut y = 0.0;
            let mut w = 0.0;
            for &link_idx in &nodes[source].source_links {
                let target = links[link_idx].target;
                let v = links[link_idx].value
                    * (nodes[target].layer as f64 - nodes[source].layer as f64);
                y += source_top(source, target, nodes, links, py) * v;
                w += v;
            }
            if w <= 0.0 {
                continue;
            }
            let dy = (y / w - nodes[source].y0) * alpha;
            nodes[source].y0 += dy;
            nodes[source].y1 += dy;
            reorder_node_links(source, nodes, links);
        }
        columns[column_idx].sort_by(|a, b| compare_node_breadth(nodes, *a, *b));
        resolve_collisions(&columns[column_idx], nodes, height, py, beta);
    }
}

fn resolve_collisions(
    column: &[usize],
    nodes: &mut [SankeyNodeData],
    height: f64,
    py: f64,
    alpha: f64,
) {
    if column.is_empty() {
        return;
    }
    let middle = column.len() >> 1;
    let subject = column[middle];
    let subject_top = nodes[subject].y0;
    let subject_bottom = nodes[subject].y1;
    resolve_collisions_bottom_to_top(
        column,
        subject_top - py,
        middle as isize - 1,
        nodes,
        py,
        alpha,
    );
    resolve_collisions_top_to_bottom(column, subject_bottom + py, middle + 1, nodes, py, alpha);
    resolve_collisions_bottom_to_top(column, height, column.len() as isize - 1, nodes, py, alpha);
    resolve_collisions_top_to_bottom(column, 0.0, 0, nodes, py, alpha);
}

fn resolve_collisions_top_to_bottom(
    column: &[usize],
    mut y: f64,
    mut idx: usize,
    nodes: &mut [SankeyNodeData],
    py: f64,
    alpha: f64,
) {
    while idx < column.len() {
        let node_idx = column[idx];
        let dy = (y - nodes[node_idx].y0) * alpha;
        if dy > 1e-6 {
            nodes[node_idx].y0 += dy;
            nodes[node_idx].y1 += dy;
        }
        y = nodes[node_idx].y1 + py;
        idx += 1;
    }
}

fn resolve_collisions_bottom_to_top(
    column: &[usize],
    mut y: f64,
    mut idx: isize,
    nodes: &mut [SankeyNodeData],
    py: f64,
    alpha: f64,
) {
    while idx >= 0 {
        let node_idx = column[idx as usize];
        let dy = (nodes[node_idx].y1 - y) * alpha;
        if dy > 1e-6 {
            nodes[node_idx].y0 -= dy;
            nodes[node_idx].y1 -= dy;
        }
        y = nodes[node_idx].y0 - py;
        idx -= 1;
    }
}

fn reorder_node_links(node_idx: usize, nodes: &mut [SankeyNodeData], links: &[SankeyLinkData]) {
    let source_nodes: Vec<usize> = nodes[node_idx]
        .target_links
        .iter()
        .map(|link_idx| links[*link_idx].source)
        .collect();
    for source_idx in source_nodes {
        sort_source_links(source_idx, nodes, links);
    }

    let target_nodes: Vec<usize> = nodes[node_idx]
        .source_links
        .iter()
        .map(|link_idx| links[*link_idx].target)
        .collect();
    for target_idx in target_nodes {
        sort_target_links(target_idx, nodes, links);
    }
}

fn reorder_links(column: &[usize], nodes: &mut [SankeyNodeData], links: &[SankeyLinkData]) {
    for &node_idx in column {
        sort_source_links(node_idx, nodes, links);
        sort_target_links(node_idx, nodes, links);
    }
}

fn sort_source_links(node_idx: usize, nodes: &mut [SankeyNodeData], links: &[SankeyLinkData]) {
    let mut ordered = nodes[node_idx].source_links.clone();
    ordered.sort_by(|a, b| {
        compare_float(nodes[links[*a].target].y0, nodes[links[*b].target].y0)
            .then_with(|| links[*a].index.cmp(&links[*b].index))
    });
    nodes[node_idx].source_links = ordered;
}

fn sort_target_links(node_idx: usize, nodes: &mut [SankeyNodeData], links: &[SankeyLinkData]) {
    let mut ordered = nodes[node_idx].target_links.clone();
    ordered.sort_by(|a, b| {
        compare_float(nodes[links[*a].source].y0, nodes[links[*b].source].y0)
            .then_with(|| links[*a].index.cmp(&links[*b].index))
    });
    nodes[node_idx].target_links = ordered;
}

fn target_top(
    source: usize,
    target: usize,
    nodes: &[SankeyNodeData],
    links: &[SankeyLinkData],
    py: f64,
) -> f64 {
    let mut y =
        nodes[source].y0 - nodes[source].source_links.len().saturating_sub(1) as f64 * py / 2.0;
    for &link_idx in &nodes[source].source_links {
        if links[link_idx].target == target {
            break;
        }
        y += links[link_idx].width + py;
    }
    for &link_idx in &nodes[target].target_links {
        if links[link_idx].source == source {
            break;
        }
        y -= links[link_idx].width;
    }
    y
}

fn source_top(
    source: usize,
    target: usize,
    nodes: &[SankeyNodeData],
    links: &[SankeyLinkData],
    py: f64,
) -> f64 {
    let mut y =
        nodes[target].y0 - nodes[target].target_links.len().saturating_sub(1) as f64 * py / 2.0;
    for &link_idx in &nodes[target].target_links {
        if links[link_idx].source == source {
            break;
        }
        y += links[link_idx].width + py;
    }
    for &link_idx in &nodes[source].source_links {
        if links[link_idx].target == target {
            break;
        }
        y -= links[link_idx].width;
    }
    y
}

fn compute_link_breadths(nodes: &[SankeyNodeData], links: &mut [SankeyLinkData]) {
    for node in nodes {
        let mut y0 = node.y0;
        for &link_idx in &node.source_links {
            links[link_idx].y0 = y0 + links[link_idx].width / 2.0;
            y0 += links[link_idx].width;
        }

        let mut y1 = node.y0;
        for &link_idx in &node.target_links {
            links[link_idx].y1 = y1 + links[link_idx].width / 2.0;
            y1 += links[link_idx].width;
        }
    }
}

fn compare_node_breadth(nodes: &[SankeyNodeData], a: usize, b: usize) -> std::cmp::Ordering {
    compare_float(nodes[a].y0, nodes[b].y0)
}

fn compare_float(a: f64, b: f64) -> std::cmp::Ordering {
    a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
}
