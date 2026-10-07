use super::*;

fn measure_kanban_label(text: &str, max_width: f32, theme: &Theme) -> TextBlock {
    let mut lines = Vec::new();
    let mut width = 0.0_f32;
    for line in text.lines() {
        for wrapped in
            crate::text_metrics::wrap_text(line, max_width, theme.font_size, &theme.font_family)
        {
            let measured = crate::text_metrics::measure_text_width_with_kerning(
                &wrapped,
                theme.font_size,
                &theme.font_family,
            )
            .unwrap_or_else(|| {
                crate::text_metrics::get_computed_text_length(
                    &wrapped,
                    theme.font_size,
                    &theme.font_family,
                )
            });
            width = width.max(measured);
            lines.push(TextLine::plain(wrapped));
        }
    }
    let height = lines.len() as f32 * theme.font_size * 1.5;
    TextBlock {
        lines,
        width,
        height,
    }
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

    const COLUMN_WIDTH: f32 = 200.0;
    const CARD_WIDTH: f32 = 185.0;
    const CARD_GAP: f32 = 5.0;
    const COLUMN_TOP: f32 = -300.0;
    let column_labels: Vec<_> = graph
        .subgraphs
        .iter()
        .map(|column| measure_kanban_label(&column.label, COLUMN_WIDTH, theme))
        .collect();
    let header_height = column_labels
        .iter()
        .map(|label| label.height)
        .fold(25.0_f32, f32::max);
    let mut nodes = BTreeMap::new();
    let mut cards = BTreeMap::new();
    let mut columns = Vec::new();
    let mut max_height = 50.0_f32;

    for (index, (column, label_block)) in graph.subgraphs.iter().zip(column_labels).enumerate() {
        let center_x = COLUMN_WIDTH * (index + 1) as f32 + index as f32 * CARD_GAP;
        let top = COLUMN_TOP + header_height;
        let mut y = top;
        for id in &column.nodes {
            let Some(node) = graph.nodes.get(id) else {
                continue;
            };
            // Mermaid reserves a fixed-width card and wraps its title against
            // the 175 px label limit rather than sizing the card to its text.
            let label = measure_kanban_label(&node.label, CARD_WIDTH - 10.0, theme);
            let metadata = graph.kanban_tasks.get(id).cloned().unwrap_or_default();
            let ticket = measure_kanban_label(&metadata.ticket, CARD_WIDTH - 10.0, theme);
            let assigned = measure_kanban_label(&metadata.assigned, CARD_WIDTH - 10.0, theme);
            let height = label.height + 20.0 + ticket.height.max(assigned.height) / 2.0;
            let ticket_url = config
                .kanban
                .ticket_base_url
                .as_ref()
                .filter(|url| !url.is_empty() && !metadata.ticket.is_empty())
                .map(|url| url.replace("#TICKET#", &metadata.ticket));
            cards.insert(
                id.clone(),
                KanbanCardLayout {
                    ticket,
                    assigned,
                    priority: metadata.priority,
                    ticket_url,
                },
            );
            let style = resolve_node_style(id, graph);
            let mut item = build_node_layout(node, label, CARD_WIDTH, height, style, graph);
            item.x = center_x - CARD_WIDTH / 2.0;
            item.y = y;
            nodes.insert(id.clone(), item);
            y += height + CARD_GAP;
        }
        // The reference includes the trailing card gap in its column height.
        let height = (y - top + 30.0).max(50.0) + header_height - 25.0;
        max_height = max_height.max(height);
        columns.push(SubgraphLayout {
            label: column.label.clone(),
            label_block,
            nodes: column.nodes.clone(),
            x: center_x - COLUMN_WIDTH / 2.0,
            y: COLUMN_TOP,
            width: COLUMN_WIDTH,
            height,
            style: crate::ir::NodeStyle::default(),
            icon: column.icon.clone(),
        });
    }

    let count = columns.len();
    Layout {
        kind: graph.kind,
        nodes,
        edges: Vec::new(),
        subgraphs: columns,
        width: (count as f32 * COLUMN_WIDTH + count.saturating_sub(1) as f32 * CARD_GAP + 20.0)
            .max(1.0),
        height: max_height + 20.0,
        acc_title: None,
        acc_descr: None,
        diagram: DiagramData::Kanban(cards),
    }
}
