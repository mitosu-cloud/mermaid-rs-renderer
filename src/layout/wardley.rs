use super::*;

pub(super) fn compute_wardley_layout(
    graph: &Graph,
    _theme: &Theme,
    _config: &LayoutConfig,
) -> Layout {
    let data = &graph.wardley;

    let (canvas_w, canvas_h) = data.size.unwrap_or((900.0, 600.0));
    let padding: f32 = 48.0;
    let chart_x = padding;
    let chart_y = padding;
    let chart_w = canvas_w - padding * 2.0;
    let chart_h = canvas_h - padding * 2.0;
    let node_radius: f32 = 6.0;
    let label_offset: f32 = 8.0;

    // Project coordinates (visibility=Y inverted, evolution=X)
    let project_x = |evo: f32| -> f32 { chart_x + (evo / 100.0) * chart_w };
    let project_y = |vis: f32| -> f32 { canvas_h - padding - (vis / 100.0) * chart_h };

    // Layout nodes
    let mut nodes_out = Vec::new();
    for node in &data.nodes {
        let cx = project_x(node.evolution);
        let cy = project_y(node.visibility);
        let (dx, dy) = node.label_offset.unwrap_or((label_offset, -label_offset));
        nodes_out.push(WardleyNodeLayout {
            id: node.id.clone(),
            label: node.label.clone(),
            cx,
            cy,
            radius: node_radius,
            is_anchor: node.is_anchor,
            label_x: cx + dx,
            label_y: cy + dy,
            strategy: node.strategy,
            inertia: node.inertia,
        });
    }

    // Build node position lookup
    let node_pos: std::collections::HashMap<&str, (f32, f32)> = nodes_out
        .iter()
        .map(|n| (n.id.as_str(), (n.cx, n.cy)))
        .collect();

    // Layout links
    let mut links_out = Vec::new();
    for link in &data.links {
        if let (Some(&(sx, sy)), Some(&(tx, ty))) =
            (node_pos.get(link.source.as_str()), node_pos.get(link.target.as_str()))
        {
            links_out.push(WardleyLinkLayout {
                x1: sx,
                y1: sy,
                x2: tx,
                y2: ty,
                dashed: link.dashed,
                label: link.label.clone(),
                flow: link.flow,
            });
        }
    }

    // Layout trends (evolution arrows)
    let mut trends_out = Vec::new();
    for trend in &data.trends {
        if let Some(&(cx, cy)) = node_pos.get(trend.node_id.as_str()) {
            let target_x = project_x(trend.target_evolution);
            trends_out.push(WardleyTrendLayout {
                x1: cx + node_radius + 2.0,
                y1: cy,
                x2: target_x - node_radius - 2.0,
                y2: cy,
            });
        }
    }

    // Layout stages
    let mut stages_out = Vec::new();
    let n_stages = data.stages.len().max(1);
    for (i, stage) in data.stages.iter().enumerate() {
        let stage_start = i as f32 / n_stages as f32;
        let stage_end = (i + 1) as f32 / n_stages as f32;
        let divider_x = if i > 0 {
            chart_x + stage_start * chart_w
        } else {
            chart_x
        };
        let label_x = chart_x + (stage_start + stage_end) / 2.0 * chart_w;

        stages_out.push(WardleyStageLayout {
            label: stage.clone(),
            divider_x,
            label_x,
        });
    }

    // Notes
    let notes_out: Vec<(String, f32, f32)> = data
        .notes
        .iter()
        .map(|n| (n.text.clone(), project_x(n.x), project_y(n.y)))
        .collect();

    let mut nodes = BTreeMap::new();
    nodes.insert(
        "__wardley_content".to_string(),
        NodeLayout {
            id: "__wardley_content".to_string(),
            x: 0.0, y: 0.0, width: canvas_w, height: canvas_h,
            label: TextBlock { lines: vec![TextLine::plain(String::new())], width: 0.0, height: 0.0 },
            shape: crate::ir::NodeShape::Rectangle,
            style: crate::ir::NodeStyle::default(),
            link: None, anchor_subgraph: None, hidden: false,
            icon: None, img: None, img_w: None, img_h: None, sub_label: None, is_treemap_leaf: false,
            kanban_ticket: None, kanban_assigned: None, kanban_priority: None,
        },
    );

    Layout {
        kind: graph.kind,
        nodes,
        edges: Vec::new(),
        subgraphs: Vec::new(),
        acc_title: None,
        acc_descr: None,
        diagram: DiagramData::Wardley(WardleyLayout {
            title: data.title.clone(),
            canvas_width: canvas_w,
            canvas_height: canvas_h,
            padding,
            chart_x,
            chart_y,
            chart_width: chart_w,
            chart_height: chart_h,
            nodes: nodes_out,
            links: links_out,
            trends: trends_out,
            stages: stages_out,
            x_label: "Evolution".to_string(),
            y_label: "Visibility".to_string(),
            notes: notes_out,
            width: canvas_w,
            height: canvas_h,
        }),
        width: canvas_w,
        height: canvas_h,
    }
}
