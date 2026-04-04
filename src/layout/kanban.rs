use super::*;

/// Fixed column width matching mermaid-js kanban default (conf.kanban.sectionWidth || 200).
const KANBAN_SECTION_WIDTH: f32 = 200.0;
/// Padding between and around elements, matching mermaid-js kanban (padding = 10).
const KANBAN_PADDING: f32 = 10.0;

/// Hues for kanban section backgrounds, matching mermaid-js default theme.
/// JS generates these via cScale with hue rotations from the primary color.
const KANBAN_SECTION_HUES: [f32; 12] = [
    60.0, 80.0, 270.0, 300.0, 330.0, 0.0, 30.0, 90.0, 150.0, 180.0, 210.0, 120.0,
];
const KANBAN_SECTION_LIGHTNESS: f32 = 86.3;
const KANBAN_SECTION_SATURATION: f32 = 100.0;

/// Return an HSL color string for kanban section at given index.
fn kanban_section_color(idx: usize) -> String {
    let hue = KANBAN_SECTION_HUES[idx % KANBAN_SECTION_HUES.len()];
    format!(
        "hsl({:.0}, {:.0}%, {:.1}%)",
        hue, KANBAN_SECTION_SATURATION, KANBAN_SECTION_LIGHTNESS
    )
}

pub(super) fn compute_kanban_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
    stage_metrics: Option<&mut LayoutStageMetrics>,
) -> Layout {
    if !graph.edges.is_empty() {
        return compute_flowchart_layout(graph, theme, config, stage_metrics);
    }

    let font_size = theme.font_size.max(16.0);
    let section_width = KANBAN_SECTION_WIDTH;
    let padding = KANBAN_PADDING;
    // Card width = sectionWidth - 1.5 * padding (matches JS: item.width = WIDTH - 1.5 * padding)
    let card_width = section_width - 1.5 * padding;
    // Text area within card (leaving padding on each side)
    let card_text_width = card_width - padding * 2.0;
    let card_pad_y = padding;

    // Build node layouts with text wrapped to card width
    let line_height = font_size * config.label_line_height;
    let mut nodes = BTreeMap::new();
    for node in graph.nodes.values() {
        let label = measure_label_wrapped_to_px_width(
            &node.label,
            font_size,
            card_text_width,
            config,
            &theme.font_family,
        );
        // Add height for metadata row (ticket/assigned) if present
        let has_meta_row = node.kanban_ticket.is_some() || node.kanban_assigned.is_some();
        let meta_row_height = if has_meta_row { line_height } else { 0.0 };
        let height = (label.height + meta_row_height + card_pad_y * 2.0).max(font_size * 2.6);
        let width = card_width;
        let style = resolve_node_style(node.id.as_str(), graph);
        nodes.insert(
            node.id.clone(),
            build_node_layout(node, label, width, height, style, graph),
        );
    }

    let column_gap = padding / 2.0; // JS: columns separated by padding/2
    let card_gap = padding / 2.0; // JS: cards separated by padding/2
    let origin_x = padding;
    let origin_y = padding;
    let mut column_x = origin_x;
    let mut assigned: HashSet<String> = HashSet::new();

    // First pass: measure all column label heights to find max (like JS: maxLabelHeight)
    let mut max_label_height: f32 = 25.0; // JS default minimum
    for sub in &graph.subgraphs {
        if !sub.label.trim().is_empty() {
            let label_block = measure_label(&sub.label, theme, config);
            max_label_height = max_label_height.max(label_block.height);
        }
    }

    // Second pass: position columns and cards
    for sub in &graph.subgraphs {
        let column_nodes: Vec<String> = sub
            .nodes
            .iter()
            .filter(|id| nodes.contains_key(*id))
            .cloned()
            .collect();
        if column_nodes.is_empty() {
            continue;
        }
        assigned.extend(column_nodes.iter().cloned());

        // Position cards vertically within the column
        // JS: top = (-WIDTH * 3) / 2 + maxLabelHeight; y starts at top
        // For our layout, y_cursor starts after the column header area
        let top_offset = max_label_height + padding;
        let mut y_cursor = origin_y + top_offset;

        let last_idx = column_nodes.len().saturating_sub(1);
        for (idx, node_id) in column_nodes.iter().enumerate() {
            if let Some(node) = nodes.get_mut(node_id) {
                // Center card horizontally within section
                let card_x_offset = (section_width - card_width) / 2.0;
                node.x = column_x + card_x_offset;
                node.y = y_cursor;
                y_cursor += node.height;
                if idx < last_idx {
                    y_cursor += card_gap;
                }
            }
        }

        column_x += section_width + column_gap;
    }

    // Place unassigned nodes to the right
    let mut free_x = column_x;
    for node in nodes.values_mut() {
        if assigned.contains(&node.id) {
            continue;
        }
        node.x = free_x;
        node.y = origin_y;
        free_x += node.width + column_gap;
    }

    // Build subgraph layouts — these wrap each column with a border rectangle.
    // We override sizes to use fixed section_width and proper heights.
    // Each column gets a different color from the theme's color scale (matching JS cScale).
    let mut subgraphs = Vec::new();
    let mut col_x = origin_x;
    let mut col_idx: usize = 0;
    for sub in &graph.subgraphs {
        let column_nodes: Vec<String> = sub
            .nodes
            .iter()
            .filter(|id| nodes.contains_key(*id))
            .cloned()
            .collect();
        if column_nodes.is_empty() {
            continue;
        }

        let label_empty = sub.label.trim().is_empty();
        let label_block = if label_empty {
            TextBlock {
                lines: vec![TextLine::plain(String::new())],
                width: 0.0,
                height: 0.0,
            }
        } else {
            measure_label(&sub.label, theme, config)
        };

        // Compute column height from card positions
        let mut max_card_bottom: f32 = 0.0;
        for node_id in &column_nodes {
            if let Some(node) = nodes.get(node_id) {
                max_card_bottom = max_card_bottom.max(node.y + node.height);
            }
        }
        // Column height: from origin_y to bottom of last card + padding
        // Minimum height of 50px matching JS
        let col_height = (max_card_bottom - origin_y + padding * 1.5).max(50.0);

        // Assign per-column color cycling through distinct hues.
        // Matches JS kanban styles: each section gets a pastel color at
        // 100% saturation, ~86% lightness, with hue rotating in 30° steps.
        let mut style = resolve_subgraph_style(sub, graph);
        if style.fill.is_none() {
            let section_fill = kanban_section_color(col_idx);
            style.stroke = Some(section_fill.clone());
            style.fill = Some(section_fill);
        }
        col_idx += 1;
        subgraphs.push(SubgraphLayout {
            label: sub.label.clone(),
            label_block,
            nodes: sub.nodes.clone(),
            x: col_x,
            y: origin_y,
            width: section_width,
            height: col_height,
            style,
            icon: sub.icon.clone(),
        });

        col_x += section_width + column_gap;
    }

    // Calculate canvas bounds
    let (max_x, max_y) = bounds_without_padding(&nodes, &subgraphs);
    let width = max_x + padding;
    let height = max_y + padding;

    Layout {
        kind: graph.kind,
        nodes,
        edges: Vec::new(),
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
