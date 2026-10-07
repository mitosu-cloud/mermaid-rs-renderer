use super::*;

const BLOCK_VIEWBOX_PADDING: f32 = 5.0;
const BLOCK_GRID_GAP: f32 = 8.0;
const BLOCK_MARKER_OFFSET: f32 = 4.0;

pub(super) fn measure_block_label(text: &str, theme: &Theme, config: &LayoutConfig) -> TextBlock {
    // Block labels use their rendered width and only explicit line breaks.
    // The generic fast estimate and character-count floor widen short labels.
    let mut label = measure_label_no_wrap(text, theme, config);
    // HTML collapses whitespace-only labels to a zero-size box. In particular,
    // blank arrow labels must not reserve a full line of text.
    if text.trim().is_empty() {
        label.width = 0.0;
        label.height = 0.0;
        return label;
    }
    label.width = label
        .lines
        .iter()
        .map(|line| {
            crate::text_metrics::measure_text_width_with_kerning(
                &line.text(),
                theme.font_size,
                &theme.font_family,
            )
            .unwrap_or_else(|| text_width(&line.text(), theme.font_size, &theme.font_family, false))
        })
        .fold(0.0, f32::max);
    // Chromium's HTML layout rounds label widths up to 1/64 of a CSS pixel.
    label.width = (label.width * 64.0).ceil() / 64.0;
    label.height = label.lines.len() as f32 * theme.font_size * config.label_line_height;
    label
}

pub(super) fn compute_block_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    let mut nodes = build_graph_node_layouts(graph, theme, config);

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
                usecase: None,
                state_notes: Vec::new(),
            },
        };
    };

    let (placement_nodes, inferred_columns) = if block.nodes.is_empty() && block.groups.is_empty() {
        infer_block_grid(graph)
    } else {
        (block.nodes.clone(), 0)
    };
    let mut root = BlockCell {
        id: String::new(),
        span: 1,
        is_space: false,
        is_group: true,
        columns: if inferred_columns > 0 {
            Some(inferred_columns)
        } else {
            block.columns
        },
        bounds: BlockBounds::default(),
        children: placement_nodes
            .iter()
            .map(|item| build_block_cell(item, block, &nodes))
            .collect(),
    };
    size_block_cells(&mut root, 0.0, 0.0);
    root.bounds.x = -root.bounds.width / 2.0;
    root.bounds.y = -root.bounds.height / 2.0;
    position_block_cells(&mut root);
    let mut subgraphs = Vec::new();
    let mut group_bounds = HashMap::new();
    for child in &root.children {
        place_block_cell(child, graph, &mut nodes, &mut subgraphs, &mut group_bounds);
    }

    for edge in &graph.edges {
        let Some(from_box) = block_endpoint_bounds(&edge.from, &nodes, &group_bounds) else {
            continue;
        };
        let Some(to_box) = block_endpoint_bounds(&edge.to, &nodes, &group_bounds) else {
            continue;
        };
        let from_center = from_box.center();
        let to_center = to_box.center();
        let midpoint = (
            (from_center.0 + to_center.0) / 2.0,
            (from_center.1 + to_center.1) / 2.0,
        );
        let mut start = nodes.get(&edge.from).map_or_else(
            || from_box.intersect(to_center),
            |node| block_boundary_point(node, to_center),
        );
        let mut end = nodes.get(&edge.to).map_or_else(
            || to_box.intersect(from_center),
            |node| block_boundary_point(node, from_center),
        );
        if edge.arrow_start {
            start = trim_block_endpoint(start, midpoint);
        }
        if edge.arrow_end {
            end = trim_block_endpoint(end, midpoint);
        }
        let label = edge
            .label
            .as_ref()
            .map(|l| measure_block_label(l, theme, config));
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
            label_anchor: Some(midpoint),
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
            usecase: None,
            state_notes: Vec::new(),
        },
    }
}

#[derive(Clone, Copy, Default)]
struct BlockBounds {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl BlockBounds {
    fn center(self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    fn intersect(self, target: (f32, f32)) -> (f32, f32) {
        let center = self.center();
        let direction = (target.0 - center.0, target.1 - center.1);
        if direction.0.abs() < 0.001 && direction.1.abs() < 0.001 {
            return center;
        }
        let scale = if direction.1.abs() * self.width > direction.0.abs() * self.height {
            self.height / 2.0 / direction.1.abs()
        } else {
            self.width / 2.0 / direction.0.abs()
        };
        (
            center.0 + direction.0 * scale,
            center.1 + direction.1 * scale,
        )
    }
}

struct BlockCell {
    id: String,
    span: usize,
    is_space: bool,
    is_group: bool,
    columns: Option<usize>,
    bounds: BlockBounds,
    children: Vec<BlockCell>,
}

fn build_block_cell(
    item: &crate::ir::BlockNode,
    block: &crate::ir::BlockDiagram,
    nodes: &BTreeMap<String, NodeLayout>,
) -> BlockCell {
    let group = block.groups.get(&item.id);
    let (width, height) = nodes
        .get(&item.id)
        .map(|node| {
            let (left, top, right, bottom) = crate::block_shapes::visible_bounds(node);
            (right - left, bottom - top)
        })
        .unwrap_or((0.0, 0.0));
    BlockCell {
        id: item.id.clone(),
        span: item.span.max(1),
        is_space: item.is_space,
        is_group: group.is_some(),
        columns: group.and_then(|g| g.columns),
        bounds: BlockBounds {
            x: 0.0,
            y: 0.0,
            width,
            height,
        },
        children: group
            .map(|g| {
                g.nodes
                    .iter()
                    .map(|child| build_block_cell(child, block, nodes))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// Mermaid first measures each nested grid, assigns shared sibling sizes, and
/// then expands the grids to their allocated widths. Group columns stay local.
fn size_block_cells(cell: &mut BlockCell, sibling_width: f32, sibling_height: f32) {
    let p = BLOCK_GRID_GAP;
    if cell.bounds.width == 0.0 {
        cell.bounds.width = sibling_width;
        cell.bounds.height = sibling_height;
    }
    if cell.children.is_empty() {
        return;
    }
    for child in &mut cell.children {
        size_block_cells(child, 0.0, 0.0);
    }
    let mut max_width = 0.0f32;
    let mut max_height = 0.0f32;
    for child in &cell.children {
        if !child.is_space {
            max_width = max_width.max(child.bounds.width / child.span as f32);
            max_height = max_height.max(child.bounds.height);
        }
    }
    for child in &mut cell.children {
        child.bounds.width = max_width * child.span as f32 + p * (child.span - 1) as f32;
        child.bounds.height = max_height;
    }
    for child in &mut cell.children {
        size_block_cells(child, max_width, max_height);
    }
    let num_items: usize = cell.children.iter().map(|child| child.span).sum();
    let x_size = cell
        .columns
        .filter(|&cols| cols > 0 && cols < num_items)
        .unwrap_or(cell.children.len());
    let y_size = num_items.div_ceil(x_size);
    let mut width = x_size as f32 * (max_width + p) + p;
    let mut height = y_size as f32 * (max_height + p) + p;
    if width < sibling_width {
        width = sibling_width;
        height = sibling_height;
        let child_width = (width - x_size as f32 * p - p) / x_size as f32;
        let child_height = (height - y_size as f32 * p - p) / y_size as f32;
        for child in &mut cell.children {
            child.bounds.width = child_width.max(0.0);
            child.bounds.height = child_height.max(0.0);
        }
    }
    if width < cell.bounds.width {
        width = cell.bounds.width;
        let num = cell
            .columns
            .map_or(cell.children.len(), |cols| cols.min(cell.children.len()))
            .max(1);
        let child_width = (width - num as f32 * p - p) / num as f32;
        for child in &mut cell.children {
            child.bounds.width = child_width.max(0.0);
        }
    }
    cell.bounds.width = width;
    cell.bounds.height = height;
}

fn block_row_and_advance(columns: Option<usize>, position: usize, span: usize) -> (usize, usize) {
    match columns.filter(|&cols| cols > 0) {
        Some(cols) => (position / cols, span.min(cols - position % cols)),
        None => (0, span),
    }
}

fn position_block_cells(cell: &mut BlockCell) {
    let p = BLOCK_GRID_GAP;
    let mut row_heights = BTreeMap::<usize, f32>::new();
    let mut position = 0;
    for child in &cell.children {
        let (row, advance) = block_row_and_advance(cell.columns, position, child.span);
        row_heights
            .entry(row)
            .and_modify(|h| *h = h.max(child.bounds.height))
            .or_insert(child.bounds.height);
        position += advance;
    }
    let mut row_offsets = BTreeMap::new();
    let mut offset = 0.0;
    for (&row, &height) in &row_heights {
        row_offsets.insert(row, offset);
        offset += height + p;
    }
    let (_, cy) = cell.bounds.center();
    let left = if cell.id.is_empty() {
        -p
    } else {
        cell.bounds.x
    };
    let mut cursor = left;
    let mut previous_row = 0;
    position = 0;
    for child in &mut cell.children {
        let (row, advance) = block_row_and_advance(cell.columns, position, child.span);
        if row != previous_row {
            cursor = left;
            previous_row = row;
        }
        child.bounds.x = cursor + p;
        child.bounds.y = cy - cell.bounds.height / 2.0
            + row_offsets[&row]
            + (row_heights[&row] - child.bounds.height) / 2.0
            + p;
        cursor = child.bounds.x + child.bounds.width;
        position_block_cells(child);
        position += advance;
    }
}

fn place_block_cell(
    cell: &BlockCell,
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    subgraphs: &mut Vec<SubgraphLayout>,
    groups: &mut HashMap<String, BlockBounds>,
) {
    if cell.is_space {
        return;
    }
    if cell.is_group {
        groups.insert(cell.id.clone(), cell.bounds);
        let members = graph
            .subgraphs
            .iter()
            .find(|sub| sub.id.as_deref() == Some(cell.id.as_str()))
            .map(|sub| sub.nodes.clone())
            .unwrap_or_default();
        subgraphs.push(SubgraphLayout {
            id: Some(cell.id.clone()),
            label: String::new(),
            label_block: TextBlock {
                lines: Vec::new(),
                width: 0.0,
                height: 0.0,
            },
            nodes: members,
            x: cell.bounds.x,
            y: cell.bounds.y,
            width: cell.bounds.width,
            height: cell.bounds.height,
            style: resolve_node_style(&cell.id, graph),
            icon: None,
        });
        for child in &cell.children {
            place_block_cell(child, graph, nodes, subgraphs, groups);
        }
    } else if let Some(node) = nodes.get_mut(&cell.id) {
        let (width, height) = crate::block_shapes::positioned_size(
            node,
            cell.bounds.width,
            cell.bounds.height,
            cell.span,
        );
        node.x = cell.bounds.x + (cell.bounds.width - width) / 2.0;
        node.y = cell.bounds.y + (cell.bounds.height - height) / 2.0;
        node.width = width;
        node.height = height;
    }
}

fn block_endpoint_bounds(
    id: &str,
    nodes: &BTreeMap<String, NodeLayout>,
    groups: &HashMap<String, BlockBounds>,
) -> Option<BlockBounds> {
    nodes
        .get(id)
        .map(|node| BlockBounds {
            x: node.x,
            y: node.y,
            width: node.width,
            height: node.height,
        })
        .or_else(|| groups.get(id).copied())
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
