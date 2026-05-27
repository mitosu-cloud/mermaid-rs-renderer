use super::*;

const BLOCK_LAYOUT_PADDING: f32 = 8.0;
const BLOCK_VIEWBOX_PADDING: f32 = 5.0;

#[derive(Clone)]
struct BlockItemMeasure {
    id: String,
    is_space: bool,
    is_group: bool,
    width: f32,
    height: f32,
}

struct BlockGridMeasure {
    columns: usize,
    column_widths: Vec<f32>,
    row_heights: Vec<f32>,
    cells: Vec<BlockGridCell>,
    width: f32,
    height: f32,
}

struct BlockGridCell {
    row: usize,
    col: usize,
    span: usize,
    item: BlockItemMeasure,
}

pub(super) fn compute_block_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    let mut nodes = build_graph_node_layouts(graph, theme, config);

    let mut edges: Vec<EdgeLayout> = Vec::new();

    let Some(block) = graph.block.as_ref() else {
        let mut subgraphs = build_subgraph_layouts(graph, &nodes, theme, config);
        normalize_block_layout(&mut nodes, edges.as_mut_slice(), &mut subgraphs);
        let (max_x, max_y) = bounds_without_padding(&nodes, &subgraphs);
        return Layout {
            kind: graph.kind,
            nodes,
            edges,
            subgraphs,
            width: max_x + BLOCK_VIEWBOX_PADDING,
            height: max_y + BLOCK_VIEWBOX_PADDING,
            acc_title: None,
            acc_descr: None,
            diagram: DiagramData::Graph {
                state_notes: Vec::new(),
                title: None,
            },
        };
    };

    let (placement_nodes, inferred_columns) = if block.nodes.is_empty() && block.groups.is_empty() {
        infer_block_grid(graph)
    } else {
        (block.nodes.clone(), 0)
    };
    let root_columns = if inferred_columns > 0 {
        Some(inferred_columns)
    } else {
        block.columns
    };
    layout_block_items(
        &placement_nodes,
        root_columns,
        BLOCK_VIEWBOX_PADDING,
        BLOCK_VIEWBOX_PADDING,
        None,
        &mut nodes,
        block,
    );

    let mut subgraphs = build_subgraph_layouts(graph, &nodes, theme, config);

    for edge in &graph.edges {
        let Some(from_box) = block_endpoint_box(&edge.from, &nodes, graph) else {
            continue;
        };
        let Some(to_box) = block_endpoint_box(&edge.to, &nodes, graph) else {
            continue;
        };
        let from_center = from_box.center();
        let to_center = to_box.center();
        let midpoint = (
            (from_center.0 + to_center.0) / 2.0,
            (from_center.1 + to_center.1) / 2.0,
        );
        let mut start = block_box_intersection(from_box, midpoint);
        let mut end = block_box_intersection(to_box, midpoint);
        if edge.arrow_start {
            start = trim_point_towards(start, midpoint, BLOCK_MARKER_OFFSET);
        }
        if edge.arrow_end {
            end = trim_point_towards(end, midpoint, BLOCK_MARKER_OFFSET);
        }
        let label = edge.label.as_ref().map(|l| measure_label(l, theme, config));
        let start_label = edge
            .start_label
            .as_ref()
            .map(|l| measure_label(l, theme, config));
        let end_label = edge
            .end_label
            .as_ref()
            .map(|l| measure_label(l, theme, config));
        let mut override_style = resolve_edge_style(edges.len(), graph);
        if edge.style == crate::ir::EdgeStyle::Dotted && override_style.dasharray.is_none() {
            override_style.dasharray = Some("3 3".to_string());
        }
        edges.push(EdgeLayout {
            from: edge.from.clone(),
            to: edge.to.clone(),
            label,
            start_label,
            end_label,
            label_anchor: None,
            start_label_anchor: None,
            end_label_anchor: None,
            points: vec![start, midpoint, end],
            directed: edge.directed,
            arrow_start: edge.arrow_start,
            arrow_end: edge.arrow_end,
            arrow_start_kind: edge.arrow_start_kind,
            arrow_end_kind: edge.arrow_end_kind,
            start_decoration: edge.start_decoration,
            end_decoration: edge.end_decoration,
            sequence_arrow_end: edge.sequence_arrow_end,
            sequence_arrow_start: edge.sequence_arrow_start,
            style: edge.style,
            override_style,
            curve: Some(crate::ir::CurveType::Basis),
        });
    }

    normalize_block_layout(&mut nodes, edges.as_mut_slice(), &mut subgraphs);

    let (max_x, max_y) = bounds_with_edges_capped(&nodes, &subgraphs, &edges, Some(0.0));
    let width = max_x + BLOCK_VIEWBOX_PADDING;
    let height = max_y + BLOCK_VIEWBOX_PADDING;

    Layout {
        kind: graph.kind,
        nodes,
        edges,
        subgraphs,
        width,
        height,
        acc_title: None,
        acc_descr: None,
        diagram: DiagramData::Graph {
            state_notes: Vec::new(),
            title: None,
        },
    }
}

fn normalize_block_layout(
    nodes: &mut BTreeMap<String, NodeLayout>,
    edges: &mut [EdgeLayout],
    subgraphs: &mut [SubgraphLayout],
) {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    for node in nodes.values() {
        min_x = min_x.min(node.x);
        min_y = min_y.min(node.y);
    }
    for sub in subgraphs.iter() {
        min_x = min_x.min(sub.x);
        min_y = min_y.min(sub.y);
    }
    for edge in edges.iter() {
        for point in &edge.points {
            min_x = min_x.min(point.0);
            min_y = min_y.min(point.1);
        }
    }

    if !min_x.is_finite() || !min_y.is_finite() {
        return;
    }

    let shift_x = BLOCK_VIEWBOX_PADDING - min_x;
    let shift_y = BLOCK_VIEWBOX_PADDING - min_y;
    if shift_x.abs() < 1e-3 && shift_y.abs() < 1e-3 {
        return;
    }

    for node in nodes.values_mut() {
        node.x += shift_x;
        node.y += shift_y;
    }
    for edge in edges.iter_mut() {
        for point in edge.points.iter_mut() {
            point.0 += shift_x;
            point.1 += shift_y;
        }
        if let Some(anchor) = edge.label_anchor.as_mut() {
            anchor.0 += shift_x;
            anchor.1 += shift_y;
        }
        if let Some(anchor) = edge.start_label_anchor.as_mut() {
            anchor.0 += shift_x;
            anchor.1 += shift_y;
        }
        if let Some(anchor) = edge.end_label_anchor.as_mut() {
            anchor.0 += shift_x;
            anchor.1 += shift_y;
        }
    }
    for sub in subgraphs.iter_mut() {
        sub.x += shift_x;
        sub.y += shift_y;
    }
}

fn layout_block_items(
    items: &[crate::ir::BlockNode],
    columns_override: Option<usize>,
    origin_x: f32,
    origin_y: f32,
    target_width: Option<f32>,
    nodes: &mut std::collections::BTreeMap<String, NodeLayout>,
    block: &crate::ir::BlockDiagram,
) -> (f32, f32) {
    let mut measure = measure_block_items(items, columns_override, nodes, block);
    if let Some(target_width) = target_width {
        if target_width > measure.width && measure.columns > 0 {
            let gap_total = BLOCK_LAYOUT_PADDING * measure.columns.saturating_sub(1) as f32;
            let per_col = ((target_width - gap_total) / measure.columns as f32).max(0.0);
            for width in &mut measure.column_widths {
                *width = width.max(per_col);
            }
            measure.width = measure.column_widths.iter().sum::<f32>() + gap_total;
        }
    }

    let mut column_x = vec![origin_x; measure.columns];
    for i in 1..measure.columns {
        column_x[i] = column_x[i - 1] + measure.column_widths[i - 1] + BLOCK_LAYOUT_PADDING;
    }
    let mut row_y = vec![origin_y; measure.row_heights.len()];
    for i in 1..measure.row_heights.len() {
        row_y[i] = row_y[i - 1] + measure.row_heights[i - 1] + BLOCK_LAYOUT_PADDING;
    }

    for cell in &measure.cells {
        if cell.item.is_space {
            continue;
        }
        let start_x = column_x.get(cell.col).copied().unwrap_or(origin_x);
        let mut span_width = 0.0;
        for i in 0..cell.span {
            let idx = cell.col + i;
            if idx < measure.column_widths.len() {
                span_width += measure.column_widths[idx];
                if i + 1 < cell.span {
                    span_width += BLOCK_LAYOUT_PADDING;
                }
            }
        }
        let row_height = measure
            .row_heights
            .get(cell.row)
            .copied()
            .unwrap_or(cell.item.height);
        let y = row_y.get(cell.row).copied().unwrap_or(origin_y);

        if cell.item.is_group {
            let group_y = y + (row_height - cell.item.height).max(0.0) / 2.0;
            let child_x = start_x + BLOCK_LAYOUT_PADDING;
            let child_y = group_y + BLOCK_LAYOUT_PADDING;
            let child_w = (span_width - 2.0 * BLOCK_LAYOUT_PADDING).max(0.0);
            if let Some(group) = block.groups.get(&cell.item.id) {
                layout_block_items(
                    &group.nodes,
                    group.columns,
                    child_x,
                    child_y,
                    Some(child_w),
                    nodes,
                    block,
                );
            }
        } else if let Some(layout) = nodes.get_mut(&cell.item.id) {
            if block_node_uses_positioned_size(layout.shape) {
                layout.width = layout.width.max(span_width);
                layout.height = layout.height.max(row_height);
            }
            layout.x = start_x + (span_width - layout.width) / 2.0;
            layout.y = y + (row_height - layout.height) / 2.0;
        }
    }

    (measure.width, measure.height)
}

fn measure_block_items(
    items: &[crate::ir::BlockNode],
    columns_override: Option<usize>,
    nodes: &std::collections::BTreeMap<String, NodeLayout>,
    block: &crate::ir::BlockDiagram,
) -> BlockGridMeasure {
    let columns = resolve_block_columns(items, columns_override);
    let mut column_widths = vec![0.0f32; columns];
    let mut row_heights: Vec<f32> = vec![0.0];
    let mut cells = Vec::new();
    let mut row = 0usize;
    let mut col = 0usize;
    let measured_items = items
        .iter()
        .map(|node| measure_block_item(node, nodes, block))
        .collect::<Vec<_>>();
    let parent_max_height = measured_items
        .iter()
        .filter(|item| !item.is_space)
        .map(|item| item.height)
        .fold(0.0_f32, f32::max);

    for (node, item) in items.iter().zip(measured_items.into_iter()) {
        if col >= columns {
            col = 0;
            row += 1;
            row_heights.push(0.0);
        }
        let span = node.span.max(1).min(columns);
        if col + span > columns {
            col = 0;
            row += 1;
            row_heights.push(0.0);
        }
        if item.is_space {
            if let Some(row_height) = row_heights.get_mut(row) {
                *row_height = (*row_height).max(parent_max_height);
            }
        } else {
            let per_col = item.width / span as f32;
            for i in 0..span {
                let idx = col + i;
                if idx < columns {
                    column_widths[idx] = column_widths[idx].max(per_col);
                }
            }
            if let Some(row_height) = row_heights.get_mut(row) {
                let layout_height = if item.is_group {
                    item.height
                } else {
                    parent_max_height
                };
                *row_height = (*row_height).max(layout_height);
            }
        }
        cells.push(BlockGridCell {
            row,
            col,
            span,
            item,
        });
        col += span;
    }

    let max_column_width = column_widths.iter().copied().fold(0.0_f32, f32::max);
    if max_column_width > 0.0 {
        for width in &mut column_widths {
            *width = max_column_width;
        }
    }

    let width =
        column_widths.iter().sum::<f32>() + BLOCK_LAYOUT_PADDING * columns.saturating_sub(1) as f32;
    let height = row_heights.iter().sum::<f32>()
        + BLOCK_LAYOUT_PADDING * row_heights.len().saturating_sub(1) as f32;

    BlockGridMeasure {
        columns,
        column_widths,
        row_heights,
        cells,
        width,
        height,
    }
}

fn measure_block_item(
    node: &crate::ir::BlockNode,
    nodes: &std::collections::BTreeMap<String, NodeLayout>,
    block: &crate::ir::BlockDiagram,
) -> BlockItemMeasure {
    if node.is_space {
        return BlockItemMeasure {
            id: node.id.clone(),
            is_space: true,
            is_group: false,
            width: 0.0,
            height: 0.0,
        };
    }

    if let Some(group) = block.groups.get(&node.id) {
        let measured = measure_block_items(&group.nodes, group.columns, nodes, block);
        return BlockItemMeasure {
            id: node.id.clone(),
            is_space: false,
            is_group: true,
            width: measured.width + 2.0 * BLOCK_LAYOUT_PADDING,
            height: measured.height + 2.0 * BLOCK_LAYOUT_PADDING,
        };
    }

    let (width, height) = nodes
        .get(&node.id)
        .map(|layout| (layout.width, layout.height))
        .unwrap_or((0.0, 0.0));
    BlockItemMeasure {
        id: node.id.clone(),
        is_space: false,
        is_group: false,
        width,
        height,
    }
}

fn block_node_uses_positioned_size(shape: crate::ir::NodeShape) -> bool {
    matches!(
        shape,
        crate::ir::NodeShape::Rectangle | crate::ir::NodeShape::RoundRect
    )
}

fn resolve_block_columns(items: &[crate::ir::BlockNode], columns_override: Option<usize>) -> usize {
    columns_override
        .filter(|columns| *columns > 0)
        .unwrap_or_else(|| {
            items
                .iter()
                .map(|node| node.span.max(1))
                .sum::<usize>()
                .max(1)
        })
}

const BLOCK_MARKER_OFFSET: f32 = 4.0;

#[derive(Clone, Copy)]
struct BlockEndpointBox {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl BlockEndpointBox {
    fn center(self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}

fn block_endpoint_box(
    id: &str,
    nodes: &std::collections::BTreeMap<String, NodeLayout>,
    graph: &Graph,
) -> Option<BlockEndpointBox> {
    if let Some(layout) = nodes.get(id) {
        return Some(BlockEndpointBox {
            x: layout.x,
            y: layout.y,
            width: layout.width,
            height: layout.height,
        });
    }

    let subgraph = graph
        .subgraphs
        .iter()
        .find(|subgraph| subgraph.id.as_deref() == Some(id))?;
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for node_id in &subgraph.nodes {
        if let Some(node) = nodes.get(node_id) {
            min_x = min_x.min(node.x);
            min_y = min_y.min(node.y);
            max_x = max_x.max(node.x + node.width);
            max_y = max_y.max(node.y + node.height);
        }
    }
    if min_x == f32::MAX {
        return None;
    }
    Some(BlockEndpointBox {
        x: min_x - BLOCK_LAYOUT_PADDING,
        y: min_y - BLOCK_LAYOUT_PADDING,
        width: (max_x - min_x) + 2.0 * BLOCK_LAYOUT_PADDING,
        height: (max_y - min_y) + 2.0 * BLOCK_LAYOUT_PADDING,
    })
}

fn block_box_intersection(endpoint: BlockEndpointBox, point: (f32, f32)) -> (f32, f32) {
    let center = endpoint.center();
    let dx = point.0 - center.0;
    let dy = point.1 - center.1;
    if dx.abs() < 0.001 && dy.abs() < 0.001 {
        return center;
    }

    let half_width = endpoint.width / 2.0;
    let half_height = endpoint.height / 2.0;
    if dy.abs() * half_width > dx.abs() * half_height {
        let scale = half_height / dy.abs().max(0.001);
        (center.0 + dx * scale, center.1 + half_height * dy.signum())
    } else {
        let scale = half_width / dx.abs().max(0.001);
        (center.0 + half_width * dx.signum(), center.1 + dy * scale)
    }
}

fn trim_point_towards(point: (f32, f32), target: (f32, f32), amount: f32) -> (f32, f32) {
    let dx = target.0 - point.0;
    let dy = target.1 - point.1;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= amount || len < 0.001 {
        return point;
    }
    let scale = amount / len;
    (point.0 + dx * scale, point.1 + dy * scale)
}

fn infer_block_grid(graph: &Graph) -> (Vec<crate::ir::BlockNode>, usize) {
    let mut ids: Vec<String> = graph.nodes.keys().cloned().collect();
    ids.sort_by(|a, b| {
        let ao = graph.node_order.get(a).copied().unwrap_or(usize::MAX);
        let bo = graph.node_order.get(b).copied().unwrap_or(usize::MAX);
        ao.cmp(&bo).then_with(|| a.cmp(b))
    });
    if ids.is_empty() {
        return (Vec::new(), 1);
    }

    let mut indegree: HashMap<String, usize> = ids.iter().cloned().map(|id| (id, 0usize)).collect();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for edge in &graph.edges {
        if edge.from == edge.to {
            continue;
        }
        if !indegree.contains_key(&edge.from) || !indegree.contains_key(&edge.to) {
            continue;
        }
        outgoing
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        if let Some(value) = indegree.get_mut(&edge.to) {
            *value += 1;
        }
    }
    for children in outgoing.values_mut() {
        children.sort_by(|a, b| {
            let ao = graph.node_order.get(a).copied().unwrap_or(usize::MAX);
            let bo = graph.node_order.get(b).copied().unwrap_or(usize::MAX);
            ao.cmp(&bo).then_with(|| a.cmp(b))
        });
        children.dedup();
    }

    let mut queue: Vec<String> = ids
        .iter()
        .filter(|id| indegree.get(*id).copied().unwrap_or(0) == 0)
        .cloned()
        .collect();
    let mut rank: HashMap<String, usize> = HashMap::new();
    let mut head = 0usize;
    while head < queue.len() {
        let id = queue[head].clone();
        head += 1;
        let base_rank = rank.get(&id).copied().unwrap_or(0);
        if let Some(children) = outgoing.get(&id) {
            for child in children {
                rank.entry(child.clone())
                    .and_modify(|r| *r = (*r).max(base_rank + 1))
                    .or_insert(base_rank + 1);
                if let Some(value) = indegree.get_mut(child) {
                    *value = value.saturating_sub(1);
                    if *value == 0 {
                        queue.push(child.clone());
                    }
                }
            }
        }
    }

    if rank.len() < ids.len() {
        for id in &ids {
            if rank.contains_key(id) {
                continue;
            }
            let mut inferred_rank = None;
            for edge in &graph.edges {
                if edge.to != *id {
                    continue;
                }
                if let Some(parent_rank) = rank.get(&edge.from).copied() {
                    inferred_rank = Some(
                        inferred_rank.map_or(parent_rank + 1, |r: usize| r.max(parent_rank + 1)),
                    );
                }
            }
            rank.insert(id.clone(), inferred_rank.unwrap_or(0));
        }
    }

    let mut rows: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for id in ids {
        let row = rank.get(&id).copied().unwrap_or(0);
        rows.entry(row).or_default().push(id);
    }
    for row_ids in rows.values_mut() {
        row_ids.sort_by(|a, b| {
            let ao = graph.node_order.get(a).copied().unwrap_or(usize::MAX);
            let bo = graph.node_order.get(b).copied().unwrap_or(usize::MAX);
            ao.cmp(&bo).then_with(|| a.cmp(b))
        });
    }

    let columns = rows.values().map(Vec::len).max().unwrap_or(1).max(1);
    let mut block_nodes = Vec::new();
    for row_ids in rows.values() {
        for id in row_ids {
            block_nodes.push(crate::ir::BlockNode {
                id: id.clone(),
                span: 1,
                is_space: false,
            });
        }
        let missing = columns.saturating_sub(row_ids.len());
        for _ in 0..missing {
            block_nodes.push(crate::ir::BlockNode {
                id: "__space".to_string(),
                span: 1,
                is_space: true,
            });
        }
    }
    (block_nodes, columns)
}
