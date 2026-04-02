use super::*;

pub(super) fn compute_architecture_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> Layout {
    const MARGIN: f32 = 24.0;
    const SERVICE_SIZE: f32 = 64.0;
    const SERVICE_GAP: f32 = 72.0;
    const GROUP_PAD_X: f32 = 40.0;
    const GROUP_PAD_TOP: f32 = 44.0;
    const GROUP_PAD_BOTTOM: f32 = 52.0;
    const GROUP_GAP_Y: f32 = 48.0;
    const GROUP_STROKE: &str = "hsl(240, 60%, 86.2745098039%)";
    const ICON_FILL: &str = "#087ebf";

    let mut nodes = BTreeMap::new();

    for node in graph.nodes.values() {
        let label = measure_label(&node.label, theme, config);
        let mut style = resolve_node_style(node.id.as_str(), graph);
        if style.fill.is_none() {
            style.fill = Some(ICON_FILL.to_string());
        }
        if style.stroke.is_none() {
            style.stroke = Some("none".to_string());
        }
        if style.stroke_width.is_none() {
            style.stroke_width = Some(0.0);
        }
        let is_junction = graph
            .node_classes
            .get(&node.id)
            .map(|c| c.iter().any(|s| s == "__junction__"))
            .unwrap_or(false);
        let (nw, nh) = if is_junction {
            (1.0, 1.0)
        } else {
            (SERVICE_SIZE, SERVICE_SIZE)
        };
        let mut nl = build_node_layout(node, label, nw, nh, style, graph);
        nl.shape = crate::ir::NodeShape::Rectangle;
        nl.icon = node.icon.clone();
        nl.hidden = is_junction;
        nodes.insert(node.id.clone(), nl);
    }

    // Build spatial grid from port constraints (like mermaid-js).
    // BFS from the first connected node, using port directions to
    // determine grid offsets.
    let mut grid: HashMap<String, (i32, i32)> = HashMap::new();
    if !graph.edges.is_empty() {
        // Build adjacency list with direction info.
        let mut adj: HashMap<String, Vec<(String, Option<crate::ir::ArchPort>, Option<crate::ir::ArchPort>)>> =
            HashMap::new();
        for edge in &graph.edges {
            adj.entry(edge.from.clone())
                .or_default()
                .push((edge.to.clone(), edge.arch_port_from, edge.arch_port_to));
            adj.entry(edge.to.clone())
                .or_default()
                .push((edge.from.clone(), edge.arch_port_to, edge.arch_port_from));
        }

        // BFS to assign grid coordinates.
        let start = graph.edges[0].from.clone();
        grid.insert(start.clone(), (0, 0));
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start);

        while let Some(id) = queue.pop_front() {
            let (cx, cy) = grid[&id];
            if let Some(neighbors) = adj.get(&id) {
                for (neighbor, my_port, _their_port) in neighbors {
                    if grid.contains_key(neighbor) {
                        continue;
                    }
                    // Shift grid position based on MY port direction:
                    // If I connect from my Left, neighbor is to my Left (dx=-1)
                    // If I connect from my Right, neighbor is to my Right (dx=+1)
                    // If I connect from my Top, neighbor is above (dy=-1)
                    // If I connect from my Bottom, neighbor is below (dy=+1)
                    let (dx, dy) = match my_port {
                        Some(crate::ir::ArchPort::Left) => (-1, 0),
                        Some(crate::ir::ArchPort::Right) => (1, 0),
                        Some(crate::ir::ArchPort::Top) => (0, -1),
                        Some(crate::ir::ArchPort::Bottom) => (0, 1),
                        None => (1, 0), // default: place to the right
                    };
                    grid.insert(neighbor.clone(), (cx + dx, cy + dy));
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }

    // Assign any unpositioned nodes to the grid.
    let mut next_col = grid.values().map(|(x, _)| *x).max().unwrap_or(0) + 1;
    for node_id in nodes.keys() {
        if !grid.contains_key(node_id) {
            grid.insert(node_id.clone(), (next_col, 0));
            next_col += 1;
        }
    }

    // Convert grid coords to pixel positions.
    let min_gx = grid.values().map(|(x, _)| *x).min().unwrap_or(0);
    let min_gy = grid.values().map(|(_, y)| *y).min().unwrap_or(0);
    let cell = SERVICE_SIZE + SERVICE_GAP;

    let mut assigned: HashSet<String> = HashSet::new();
    let mut subgraphs = Vec::new();

    for sub in &graph.subgraphs {
        let mut group_nodes: Vec<String> = sub
            .nodes
            .iter()
            .filter(|id| nodes.contains_key(*id))
            .cloned()
            .collect();
        if group_nodes.is_empty() {
            continue;
        }
        group_nodes.sort_by(|a, b| {
            let order_a = graph.node_order.get(a).copied().unwrap_or(usize::MAX);
            let order_b = graph.node_order.get(b).copied().unwrap_or(usize::MAX);
            order_a.cmp(&order_b).then_with(|| a.cmp(b))
        });
        assigned.extend(group_nodes.iter().cloned());

        // Position nodes using grid coordinates.
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for node_id in &group_nodes {
            let (gx, gy) = grid.get(node_id).copied().unwrap_or((0, 0));
            let px = MARGIN + GROUP_PAD_X + (gx - min_gx) as f32 * cell;
            let py = MARGIN + GROUP_PAD_TOP + (gy - min_gy) as f32 * cell;
            if let Some(node) = nodes.get_mut(node_id) {
                // Center small nodes (junctions) in the grid cell so
                // their midpoint aligns with service midpoints.
                node.x = px + (SERVICE_SIZE - node.width) / 2.0;
                node.y = py + (SERVICE_SIZE - node.height) / 2.0;
            }
            min_x = min_x.min(px);
            min_y = min_y.min(py);
            max_x = max_x.max(px + SERVICE_SIZE);
            max_y = max_y.max(py + SERVICE_SIZE);
        }

        let group_x = min_x - GROUP_PAD_X;
        let group_y = min_y - GROUP_PAD_TOP;
        // max_x/max_y already include SERVICE_SIZE (line 136-137)
        let group_width = (max_x - min_x) + GROUP_PAD_X * 2.0;
        let group_height = (max_y - min_y) + GROUP_PAD_TOP + GROUP_PAD_BOTTOM;

        let label_block = measure_label(&sub.label, theme, config);
        let mut style = resolve_subgraph_style(sub, graph);
        style.fill = Some("none".to_string());
        style.stroke = Some(GROUP_STROKE.to_string());
        style.stroke_width = Some(2.0);
        style.stroke_dasharray = Some("8".to_string());
        if style.text_color.is_none() {
            style.text_color = Some(theme.primary_text_color.clone());
        }

        subgraphs.push(SubgraphLayout {
            label: sub.label.clone(),
            label_block,
            nodes: group_nodes,
            x: group_x,
            y: group_y,
            width: group_width,
            height: group_height,
            style,
            icon: sub.icon.clone(),
        });

    }

    // Free nodes: position using grid coordinates.
    let mut free_nodes: Vec<String> = nodes
        .keys()
        .filter(|id| !assigned.contains(*id))
        .cloned()
        .collect();
    free_nodes.sort_by(|a, b| {
        let order_a = graph.node_order.get(a).copied().unwrap_or(usize::MAX);
        let order_b = graph.node_order.get(b).copied().unwrap_or(usize::MAX);
        order_a.cmp(&order_b).then_with(|| a.cmp(b))
    });
    if !free_nodes.is_empty() {
        for node_id in &free_nodes {
            let (gx, gy) = grid.get(node_id).copied().unwrap_or((0, 0));
            let px = MARGIN + GROUP_PAD_X + (gx - min_gx) as f32 * cell;
            let py = MARGIN + GROUP_PAD_TOP + (gy - min_gy) as f32 * cell;
            if let Some(node) = nodes.get_mut(node_id) {
                node.x = px + (SERVICE_SIZE - node.width) / 2.0;
                node.y = py + (SERVICE_SIZE - node.height) / 2.0;
            }
        }
    }

    let mut edges = Vec::new();
    for (idx, edge) in graph.edges.iter().enumerate() {
        let Some(from) = nodes.get(&edge.from) else {
            continue;
        };
        let Some(to) = nodes.get(&edge.to) else {
            continue;
        };
        // Use port constraints to determine edge sides. Fall back to
        // geometric heuristic if no ports specified.
        let start_side = match edge.arch_port_from {
            Some(crate::ir::ArchPort::Left) => EdgeSide::Left,
            Some(crate::ir::ArchPort::Right) => EdgeSide::Right,
            Some(crate::ir::ArchPort::Top) => EdgeSide::Top,
            Some(crate::ir::ArchPort::Bottom) => EdgeSide::Bottom,
            None => edge_sides(from, to, graph.direction).0,
        };
        let end_side = match edge.arch_port_to {
            Some(crate::ir::ArchPort::Left) => EdgeSide::Left,
            Some(crate::ir::ArchPort::Right) => EdgeSide::Right,
            Some(crate::ir::ArchPort::Top) => EdgeSide::Top,
            Some(crate::ir::ArchPort::Bottom) => EdgeSide::Bottom,
            None => edge_sides(from, to, graph.direction).1,
        };
        // For hidden nodes (junctions), use their center as the
        // connection point.  For visible services, use the port anchor.
        let start = if from.hidden {
            (from.x + from.width / 2.0, from.y + from.height / 2.0)
        } else {
            anchor_point_for_node(from, start_side, 0.0)
        };
        let end = if to.hidden {
            (to.x + to.width / 2.0, to.y + to.height / 2.0)
        } else {
            anchor_point_for_node(to, end_side, 0.0)
        };

        // Build orthogonal path (horizontal + vertical segments only).
        let points = if (start.0 - end.0).abs() < 1e-3 || (start.1 - end.1).abs() < 1e-3 {
            // Already aligned — straight line.
            vec![start, end]
        } else {
            // L-shaped orthogonal routing: decide bend direction from
            // the port sides.
            if side_is_vertical(start_side) {
                // Start exits horizontally → go horizontal first, then vertical.
                vec![start, (end.0, start.1), end]
            } else {
                // Start exits vertically → go vertical first, then horizontal.
                vec![start, (start.0, end.1), end]
            }
        };
        let mut override_style = resolve_edge_style(idx, graph);
        if override_style.stroke.is_none() {
            override_style.stroke = Some(theme.line_color.clone());
        }
        override_style.stroke_width = Some(override_style.stroke_width.unwrap_or(3.0));

        edges.push(EdgeLayout {
            from: edge.from.clone(),
            to: edge.to.clone(),
            label: None,
            start_label: None,
            end_label: None,
            label_anchor: None,
            start_label_anchor: None,
            end_label_anchor: None,
            points: compress_path(&points),
            directed: false,
            arrow_start: false,
            arrow_end: false,
            arrow_start_kind: None,
            arrow_end_kind: None,
            start_decoration: None,
            end_decoration: None,
            style: edge.style,
            override_style,
            curve: None,
        });
    }

    let (max_x, max_y) = bounds_with_edges(&nodes, &subgraphs, &edges);
    let width = (max_x + MARGIN).max(200.0);
    let height = (max_y + MARGIN).max(200.0);

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
