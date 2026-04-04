use super::*;

pub(super) fn compute_tree_view_layout(
    graph: &Graph,
    theme: &Theme,
    _config: &LayoutConfig,
) -> Layout {
    let font_size = theme.font_size;
    let row_indent: f32 = 10.0;
    let padding_x: f32 = 5.0;
    let padding_y: f32 = 5.0;
    let font_family_str = theme.font_family.as_str();

    let mut nodes_out = Vec::new();
    let mut lines_out = Vec::new();
    let mut total_height: f32 = 0.0;
    let mut total_width: f32 = 0.0;

    // Recursive depth-first traversal
    fn visit(
        node: &crate::ir::TreeViewNode,
        depth: usize,
        total_height: &mut f32,
        total_width: &mut f32,
        nodes_out: &mut Vec<TreeViewNodeLayout>,
        lines_out: &mut Vec<TreeViewLineLayout>,
        font_size: f32,
        font_family: &str,
        row_indent: f32,
        padding_x: f32,
        padding_y: f32,
        parent_x: Option<f32>,
    ) {
        let x = depth as f32 * (row_indent + padding_x);
        let label_w = crate::text_metrics::get_computed_text_length(&node.name, font_size, font_family);
        let node_w = label_w + padding_x * 2.0;
        let node_h = font_size + padding_y * 2.0;
        let y = *total_height;
        let mid_y = y + node_h / 2.0;

        nodes_out.push(TreeViewNodeLayout {
            name: node.name.clone(),
            x,
            y,
            width: node_w,
            height: node_h,
        });

        // Horizontal connector from parent indent to this node
        if let Some(px) = parent_x {
            lines_out.push(TreeViewLineLayout {
                x1: px,
                y1: mid_y,
                x2: x,
                y2: mid_y,
            });
        }

        *total_height += node_h;
        *total_width = total_width.max(x + node_w);

        let child_start_idx = nodes_out.len();
        let my_connector_x = x + padding_x;

        for child in &node.children {
            visit(
                child,
                depth + 1,
                total_height,
                total_width,
                nodes_out,
                lines_out,
                font_size,
                font_family,
                row_indent,
                padding_x,
                padding_y,
                Some(my_connector_x),
            );
        }

        // Vertical connector spanning all children
        if !node.children.is_empty() {
            let first_child_mid = nodes_out[child_start_idx].y + nodes_out[child_start_idx].height / 2.0;
            let last_child_idx = nodes_out.len() - 1;
            // Find the last DIRECT child (not grandchild) — it's the one at child_start_idx + direct children count - 1
            // Actually we just connect from our bottom to the last direct child's mid
            let mut last_direct_child_mid = first_child_mid;
            let child_depth = depth + 1;
            for n in &nodes_out[child_start_idx..] {
                // Approximate: look at nodes at the direct child depth level that start at our connector x
                let child_x = child_depth as f32 * (row_indent + padding_x);
                if (n.x - child_x).abs() < 1.0 {
                    last_direct_child_mid = n.y + n.height / 2.0;
                }
            }

            lines_out.push(TreeViewLineLayout {
                x1: my_connector_x,
                y1: y + node_h,
                x2: my_connector_x,
                y2: last_direct_child_mid,
            });
        }
    }

    for root_node in &graph.tree_view.root {
        visit(
            root_node,
            0,
            &mut total_height,
            &mut total_width,
            &mut nodes_out,
            &mut lines_out,
            font_size,
            font_family_str,
            row_indent,
            padding_x,
            padding_y,
            None,
        );
    }

    let width = total_width.max(100.0);
    let height = total_height.max(50.0);

    let mut nodes = BTreeMap::new();
    nodes.insert(
        "__tree_view_content".to_string(),
        NodeLayout {
            id: "__tree_view_content".to_string(),
            x: 0.0,
            y: 0.0,
            width,
            height,
            label: TextBlock { lines: vec![TextLine::plain(String::new())], width: 0.0, height: 0.0 },
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
            kanban_ticket: None,
            kanban_assigned: None,
            kanban_priority: None,
        },
    );

    Layout {
        kind: graph.kind,
        nodes,
        edges: Vec::new(),
        subgraphs: Vec::new(),
        acc_title: None,
        acc_descr: None,
        diagram: DiagramData::TreeView(TreeViewLayout {
            title: graph.tree_view.title.clone(),
            nodes: nodes_out,
            lines: lines_out,
            width,
            height,
        }),
        width,
        height,
    }
}
