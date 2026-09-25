use super::*;

const BLOCK_VIEWBOX_PADDING: f32 = 5.0;
const BLOCK_GRID_GAP: f32 = 8.0;
const BLOCK_MARKER_OFFSET: f32 = 4.0;

pub(super) fn measure_block_label(text: &str, theme: &Theme, config: &LayoutConfig) -> TextBlock {
    // Block labels use their rendered width and only explicit line breaks.
    // The generic fast estimate and character-count floor widen short labels.
    let mut label = measure_label_no_wrap(text, theme, config);
    label.width = label
        .lines
        .iter()
        .map(|line| text_width(&line.text(), theme.font_size, &theme.font_family, false))
        .fold(0.0, f32::max);
    // Chromium's HTML layout rounds label widths up to 1/64 of a CSS pixel.
    label.width = (label.width * 64.0).ceil() / 64.0;
    label.height = label.lines.len() as f32 * theme.font_size * config.label_line_height;
    label
}

pub(super) fn compute_block_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    let mut nodes = build_graph_node_layouts(graph, theme, config);

    let node_gap = BLOCK_GRID_GAP;
    let column_gap = BLOCK_GRID_GAP;
    let origin_x = 6.0;
    let origin_y = 6.0;

    let mut edges: Vec<EdgeLayout> = Vec::new();

    let Some(block) = graph.block.as_ref() else {
        let mut subgraphs = build_subgraph_layouts(graph, &nodes, theme, config);
        normalize_layout_with_padding(
            &mut nodes,
            edges.as_mut_slice(),
            &mut subgraphs,
            BLOCK_VIEWBOX_PADDING,
        );
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
            },
        };
    };

    let (placement_nodes, inferred_columns) = if block.nodes.is_empty() {
        infer_block_grid(graph)
    } else {
        (block.nodes.clone(), 0)
    };
    let columns = block.columns.unwrap_or_else(|| {
        if placement_nodes.is_empty() {
            1
        } else if inferred_columns > 0 {
            inferred_columns
        } else {
            placement_nodes.iter().map(|node| node.span.max(1)).sum()
        }
    });
    let mut column_widths = vec![0.0f32; columns];
    let mut column_x = vec![0.0f32; columns];
    let mut row_y = Vec::<f32>::new();

    let mut row = 0usize;
    let mut col = 0usize;
    let mut row_heights: Vec<f32> = vec![0.0];

    for node in &placement_nodes {
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
        if !node.is_space
            && let Some(layout) = nodes.get(&node.id)
        {
            let per_col = layout.width / span as f32;
            for i in 0..span {
                let idx = col + i;
                if idx < columns {
                    column_widths[idx] = column_widths[idx].max(per_col);
                }
            }
            row_heights[row] = row_heights[row].max(layout.height);
        }
        col += span;
    }

    // Mermaid's block grid uses the widest normalized child for every column,
    // including invisible space cells, and the tallest child for every row.
    let column_width = column_widths.iter().copied().fold(0.0, f32::max);
    let row_height = row_heights.iter().copied().fold(0.0, f32::max);
    column_widths.fill(column_width);
    row_heights.fill(row_height);

    column_x[0] = origin_x;
    for i in 1..columns {
        column_x[i] = column_x[i - 1] + column_widths[i - 1] + column_gap;
    }

    let mut y_cursor = origin_y;
    for h in &row_heights {
        row_y.push(y_cursor);
        y_cursor += *h + node_gap;
    }

    row = 0;
    col = 0;
    for node in &placement_nodes {
        if col >= columns {
            col = 0;
            row += 1;
        }
        let span = node.span.max(1).min(columns);
        if col + span > columns {
            col = 0;
            row += 1;
        }
        if !node.is_space
            && let Some(layout) = nodes.get_mut(&node.id)
        {
            let start_x = column_x[col];
            let mut span_width = 0.0;
            for i in 0..span {
                let idx = col + i;
                if idx < columns {
                    span_width += column_widths[idx];
                    if i + 1 < span {
                        span_width += column_gap;
                    }
                }
            }
            let (width, height) =
                crate::block_shapes::positioned_size(layout, span_width, row_heights[row], span);
            layout.width = width;
            layout.height = height;
            layout.x = start_x + (span_width - width) / 2.0;
            layout.y = row_y[row] + (row_heights[row] - height) / 2.0;
        }
        col += span;
    }

    for edge in &graph.edges {
        let Some(from_layout) = nodes.get(&edge.from) else {
            continue;
        };
        let Some(to_layout) = nodes.get(&edge.to) else {
            continue;
        };
        let from_center = (
            from_layout.x + from_layout.width / 2.0,
            from_layout.y + from_layout.height / 2.0,
        );
        let to_center = (
            to_layout.x + to_layout.width / 2.0,
            to_layout.y + to_layout.height / 2.0,
        );
        let midpoint = (
            (from_center.0 + to_center.0) / 2.0,
            (from_center.1 + to_center.1) / 2.0,
        );
        let mut start = block_boundary_point(from_layout, to_center);
        let mut end = block_boundary_point(to_layout, from_center);
        if edge.arrow_start {
            start = trim_block_endpoint(start, midpoint);
        }
        if edge.arrow_end {
            end = trim_block_endpoint(end, midpoint);
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

    let mut subgraphs = build_subgraph_layouts(graph, &nodes, theme, config);
    normalize_layout_with_padding(
        &mut nodes,
        edges.as_mut_slice(),
        &mut subgraphs,
        BLOCK_VIEWBOX_PADDING,
    );

    let (width, height) = crop_block_canvas(&mut nodes, &mut edges, &mut subgraphs);

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
        },
    }
}

fn crop_block_canvas(
    nodes: &mut BTreeMap<String, NodeLayout>,
    edges: &mut [EdgeLayout],
    subgraphs: &mut [SubgraphLayout],
) -> (f32, f32) {
    let (mut max_x, mut max_y) = bounds_with_edges(nodes, subgraphs, edges);
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    if edges.is_empty() && subgraphs.is_empty() {
        max_x = f32::NEG_INFINITY;
        max_y = f32::NEG_INFINITY;
    }
    for node in nodes.values() {
        let (left, top, right, bottom) = crate::block_shapes::visible_bounds(node);
        min_x = min_x.min(left);
        min_y = min_y.min(top);
        max_x = max_x.max(right);
        max_y = max_y.max(bottom);
    }
    for sub in subgraphs.iter() {
        min_x = min_x.min(sub.x);
        min_y = min_y.min(sub.y);
    }
    for edge in edges.iter() {
        for &(x, y) in &edge.points {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
        }
    }
    if !min_x.is_finite() || !min_y.is_finite() {
        return (BLOCK_VIEWBOX_PADDING, BLOCK_VIEWBOX_PADDING);
    }
    let dx = BLOCK_VIEWBOX_PADDING - min_x;
    let dy = BLOCK_VIEWBOX_PADDING - min_y;
    for node in nodes.values_mut() {
        node.x += dx;
        node.y += dy;
    }
    for sub in subgraphs.iter_mut() {
        sub.x += dx;
        sub.y += dy;
    }
    for edge in edges.iter_mut() {
        for point in &mut edge.points {
            point.0 += dx;
            point.1 += dy;
        }
        for anchor in [
            &mut edge.label_anchor,
            &mut edge.start_label_anchor,
            &mut edge.end_label_anchor,
        ]
        .into_iter()
        .flatten()
        {
            anchor.0 += dx;
            anchor.1 += dy;
        }
    }
    (
        max_x - min_x + 2.0 * BLOCK_VIEWBOX_PADDING,
        max_y - min_y + 2.0 * BLOCK_VIEWBOX_PADDING,
    )
}

fn block_boundary_point(node: &NodeLayout, target: (f32, f32)) -> (f32, f32) {
    let center = (node.x + node.width / 2.0, node.y + node.height / 2.0);
    let direction = (target.0 - center.0, target.1 - center.1);
    if direction.0.abs() < 0.001 && direction.1.abs() < 0.001 {
        return center;
    }
    if matches!(
        node.shape,
        crate::ir::NodeShape::Circle | crate::ir::NodeShape::DoubleCircle
    ) && let Some(point) = ray_ellipse_intersection(
        center,
        direction,
        center,
        node.width / 2.0,
        node.height / 2.0,
    ) {
        return point;
    }
    if let Some(polygon) =
        crate::block_shapes::polygon_points(node).or_else(|| shape_polygon_points(node))
        && let Some(point) = ray_polygon_intersection(center, direction, &polygon)
    {
        return point;
    }
    let half_width = node.width / 2.0;
    let half_height = node.height / 2.0;
    let scale = if direction.1.abs() * half_width > direction.0.abs() * half_height {
        half_height / direction.1.abs()
    } else {
        half_width / direction.0.abs()
    };
    (
        center.0 + direction.0 * scale,
        center.1 + direction.1 * scale,
    )
}

fn trim_block_endpoint(point: (f32, f32), target: (f32, f32)) -> (f32, f32) {
    let dx = target.0 - point.0;
    let dy = target.1 - point.1;
    let length = dx.hypot(dy);
    if length <= BLOCK_MARKER_OFFSET {
        return point;
    }
    let scale = BLOCK_MARKER_OFFSET / length;
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
