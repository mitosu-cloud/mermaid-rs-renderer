use super::*;

const KANBAN_SECTION_WIDTH: f32 = 200.0;
const KANBAN_ITEM_WIDTH: f32 = 185.0;
const KANBAN_ITEM_LABEL_WIDTH: f32 = 175.0;
const KANBAN_SECTION_TOP_TO_ITEMS: f32 = 25.0;
const KANBAN_ITEM_GAP: f32 = 5.0;
const KANBAN_SECTION_EXTRA_HEIGHT: f32 = 30.0;
const KANBAN_VIEWBOX_PAD: f32 = 10.0;
const KANBAN_FOOTER_HEIGHT: f32 = 24.0;
const KANBAN_ITEM_LABEL_PADDING_Y: f32 = 10.0;

pub(super) fn compute_kanban_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
    stage_metrics: Option<&mut LayoutStageMetrics>,
) -> Layout {
    if !graph.edges.is_empty() {
        return compute_flowchart_layout(graph, theme, config, stage_metrics);
    }

    let mut nodes = if graph.kind == crate::ir::DiagramKind::Kanban {
        build_kanban_node_layouts(graph, theme, config)
    } else {
        build_graph_node_layouts(graph, theme, config)
    };
    if graph.kind == crate::ir::DiagramKind::Requirement {
        for node in nodes.values_mut() {
            if node.style.fill.is_none() {
                node.style.fill = Some(config.requirement.fill.clone());
            }
            if node.style.stroke.is_none() {
                node.style.stroke = Some(config.requirement.box_stroke.clone());
            }
            if node.style.stroke_width.is_none() {
                node.style.stroke_width = Some(config.requirement.box_stroke_width);
            }
            if node.style.text_color.is_none() {
                node.style.text_color = Some(config.requirement.label_color.clone());
            }
        }
    }

    let node_gap = if graph.kind == crate::ir::DiagramKind::Kanban {
        KANBAN_ITEM_GAP
    } else {
        (theme.font_size * 0.45).max(4.0)
    };
    let column_gap = if graph.kind == crate::ir::DiagramKind::Kanban {
        5.0
    } else {
        (theme.font_size * 0.3).max(3.0)
    };
    let origin_x = 6.0;
    let origin_y = 6.0;
    let mut column_x = origin_x;
    let mut assigned: HashSet<String> = HashSet::new();

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

        let (column_width, pad_x, top_padding) = if graph.kind == crate::ir::DiagramKind::Kanban {
            (
                KANBAN_SECTION_WIDTH,
                (KANBAN_SECTION_WIDTH - KANBAN_ITEM_WIDTH) / 2.0,
                KANBAN_SECTION_TOP_TO_ITEMS,
            )
        } else {
            let label_empty = sub.label.trim().is_empty();
            let mut label_block = measure_label(&sub.label, theme, config);
            if label_empty {
                label_block.width = 0.0;
                label_block.height = 0.0;
            }
            let (pad_x, _pad_y, top_padding) =
                subgraph_padding_from_label(graph, sub, theme, &label_block);

            let max_node_width = column_nodes
                .iter()
                .filter_map(|id| nodes.get(id).map(|n| n.width))
                .fold(0.0_f32, f32::max);
            let inner_width = max_node_width.max(label_block.width);
            (inner_width + pad_x * 2.0, pad_x, top_padding)
        };

        let mut y_cursor = origin_y + top_padding;
        let last_idx = column_nodes.len().saturating_sub(1);
        for (idx, node_id) in column_nodes.iter().enumerate() {
            if let Some(node) = nodes.get_mut(node_id) {
                node.x = column_x + pad_x;
                node.y = y_cursor;
                y_cursor += node.height;
                if idx < last_idx {
                    y_cursor += node_gap;
                }
            }
        }

        column_x += column_width + column_gap;
    }

    let mut free_x = column_x;
    for node in nodes.values_mut() {
        if assigned.contains(&node.id) {
            continue;
        }
        node.x = free_x;
        node.y = origin_y;
        free_x += node.width + column_gap;
    }

    let mut edges: Vec<EdgeLayout> = Vec::new();
    let mut subgraphs = if graph.kind == crate::ir::DiagramKind::Kanban {
        build_kanban_subgraph_layouts(graph, &nodes, theme, config)
    } else {
        build_subgraph_layouts(graph, &nodes, theme, config)
    };
    normalize_layout(&mut nodes, edges.as_mut_slice(), &mut subgraphs);
    if graph.kind == crate::ir::DiagramKind::Kanban {
        shift_kanban_to_mermaid_padding(&mut nodes, &mut subgraphs);
    }

    let (max_x, max_y) = bounds_without_padding(&nodes, &subgraphs);
    let end_pad = if graph.kind == crate::ir::DiagramKind::Kanban {
        KANBAN_VIEWBOX_PAD
    } else {
        6.0
    };
    let width = max_x + end_pad;
    let height = max_y + end_pad;

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

fn shift_kanban_to_mermaid_padding(
    nodes: &mut BTreeMap<String, NodeLayout>,
    subgraphs: &mut [SubgraphLayout],
) {
    let shift = KANBAN_VIEWBOX_PAD - LAYOUT_BOUNDARY_PAD;
    if shift.abs() < 1e-3 {
        return;
    }
    for node in nodes.values_mut() {
        node.x += shift;
        node.y += shift;
    }
    for subgraph in subgraphs {
        subgraph.x += shift;
        subgraph.y += shift;
    }
}

fn build_kanban_node_layouts(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> BTreeMap<String, NodeLayout> {
    let mut nodes = BTreeMap::new();
    for node in graph.nodes.values() {
        let parts = kanban_label_parts(&node.label);
        let title_block = measure_label_with_font_size_and_wrap_width(
            &parts.title,
            theme.font_size.max(16.0),
            config,
            true,
            theme.font_family.as_str(),
            Some(KANBAN_ITEM_LABEL_WIDTH),
        );

        let mut lines = title_block.lines.clone();
        if let Some(raw_meta) = parts.raw_meta.as_ref() {
            lines.push(TextLine::plain(raw_meta.clone()));
        }
        let label = TextBlock {
            lines,
            width: title_block.width,
            height: title_block.height,
        };

        let has_footer = parts.ticket.is_some() || parts.assigned.is_some();
        let footer_half = if has_footer {
            KANBAN_FOOTER_HEIGHT / 2.0
        } else {
            0.0
        };
        let height = (title_block.height + KANBAN_ITEM_LABEL_PADDING_Y * 2.0)
            .max(theme.font_size * KANBAN_MIN_HEIGHT_SCALE)
            + footer_half;

        nodes.insert(
            node.id.clone(),
            NodeLayout {
                id: node.id.clone(),
                x: 0.0,
                y: 0.0,
                width: KANBAN_ITEM_WIDTH,
                height,
                label,
                shape: node.shape,
                style: resolve_node_style(node.id.as_str(), graph),
                link: graph.node_links.get(&node.id).cloned(),
                anchor_subgraph: None,
                hidden: false,
                icon: node.icon.clone(),
                img: node.img.clone(),
                img_w: node.img_w,
                img_h: node.img_h,
                sub_label: None,
                is_treemap_leaf: false,
                treemap_base_text_color: None,
            },
        );
    }
    nodes
}

fn build_kanban_subgraph_layouts(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) -> Vec<SubgraphLayout> {
    let mut subgraphs = Vec::new();
    for sub in &graph.subgraphs {
        let column_nodes: Vec<&NodeLayout> =
            sub.nodes.iter().filter_map(|id| nodes.get(id)).collect();
        if column_nodes.is_empty() {
            continue;
        }

        let min_x = column_nodes
            .iter()
            .map(|node| node.x)
            .fold(f32::MAX, f32::min);
        let min_y = column_nodes
            .iter()
            .map(|node| node.y)
            .fold(f32::MAX, f32::min);
        let total_card_height = column_nodes.iter().map(|node| node.height).sum::<f32>();
        let height = total_card_height
            + KANBAN_ITEM_GAP * column_nodes.len() as f32
            + KANBAN_SECTION_EXTRA_HEIGHT;
        let label_block = measure_label(&sub.label, theme, config);
        let style = resolve_subgraph_style(sub, graph);

        subgraphs.push(SubgraphLayout {
            label: sub.label.clone(),
            label_block,
            nodes: sub.nodes.clone(),
            x: min_x - (KANBAN_SECTION_WIDTH - KANBAN_ITEM_WIDTH) / 2.0,
            y: min_y - KANBAN_SECTION_TOP_TO_ITEMS,
            width: KANBAN_SECTION_WIDTH,
            height,
            style,
            icon: sub.icon.clone(),
        });
    }
    subgraphs
}

#[derive(Default)]
struct KanbanLabelParts {
    title: String,
    raw_meta: Option<String>,
    ticket: Option<String>,
    assigned: Option<String>,
}

fn kanban_label_parts(label: &str) -> KanbanLabelParts {
    let mut parts = KanbanLabelParts::default();
    let mut title_lines = Vec::new();
    let mut raw_meta = Vec::new();

    for line in label.lines() {
        if is_kanban_metadata_line(line) {
            raw_meta.push(line.trim().to_string());
            for (key, value) in parse_kanban_metadata_pairs(line) {
                match key.as_str() {
                    "ticket" => parts.ticket = Some(value),
                    "assigned" => parts.assigned = Some(value),
                    "label" => title_lines = vec![value],
                    _ => {}
                }
            }
        } else {
            title_lines.push(line.to_string());
        }
    }

    parts.title = title_lines.join("\n");
    if !raw_meta.is_empty() {
        parts.raw_meta = Some(raw_meta.join(", "));
    }
    parts
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
