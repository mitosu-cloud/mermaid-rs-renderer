use super::*;

fn tree_view_has_css_class(css_class: &Option<String>, token: &str) -> bool {
    css_class
        .as_deref()
        .is_some_and(|classes| classes.split_whitespace().any(|class| class == token))
}

pub(super) fn compute_tree_view_layout(
    graph: &Graph,
    theme: &Theme,
    _config: &LayoutConfig,
) -> Layout {
    let font_size = theme.font_size;
    let row_indent: f32 = 10.0;
    let padding_x: f32 = 5.0;
    let padding_y: f32 = 5.0;
    let icon_offset: f32 = 14.0 + 4.0;
    let desc_gap: f32 = 16.0;
    let line_thickness: f32 = 1.0;
    let label_height = font_size * 1.1875;
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
        icon_offset: f32,
        line_thickness: f32,
        label_height: f32,
        is_virtual_root: bool,
    ) {
        let x = depth as f32 * (row_indent + padding_x);
        let label_w =
            crate::text_metrics::get_computed_text_length(&node.name, font_size, font_family);
        let show_icon = !is_virtual_root
            && node
                .icon_id
                .as_deref()
                .is_some_and(|icon_id| icon_id != "none");
        let node_icon_offset = if show_icon { icon_offset } else { 0.0 };
        let node_w = label_w + padding_x * 2.0 + node_icon_offset;
        let node_h = label_height + padding_y * 2.0;
        let y = *total_height;
        let mid_y = y + node_h / 2.0;
        let label_x = x + padding_x + node_icon_offset;

        nodes_out.push(TreeViewNodeLayout {
            name: node.name.clone(),
            node_type: node.node_type,
            icon_id: node.icon_id.clone(),
            css_class: node.css_class.clone(),
            description: node.description.clone(),
            x,
            y,
            label_x,
            label_right_edge: label_x + label_w,
            description_x: None,
            highlight_width: None,
            width: node_w,
            height: node_h,
        });

        lines_out.push(TreeViewLineLayout {
            x1: x - row_indent,
            y1: mid_y,
            x2: x,
            y2: mid_y,
        });

        *total_height += node_h;
        *total_width = total_width.max(x + node_w);

        let mut direct_child_indices = Vec::new();
        let my_connector_x = x + padding_x;

        for child in &node.children {
            direct_child_indices.push(nodes_out.len());
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
                icon_offset,
                line_thickness,
                label_height,
                false,
            );
        }

        // Vertical connector spanning all children
        if let Some(last_child_idx) = direct_child_indices.last().copied() {
            let last_direct_child_mid =
                nodes_out[last_child_idx].y + nodes_out[last_child_idx].height / 2.0;
            lines_out.push(TreeViewLineLayout {
                x1: my_connector_x,
                y1: y + node_h,
                x2: my_connector_x,
                y2: last_direct_child_mid + line_thickness / 2.0,
            });
        }
    }

    let virtual_root = crate::ir::TreeViewNode {
        name: "/".to_string(),
        node_type: crate::ir::TreeViewNodeType::Directory,
        icon_id: None,
        css_class: None,
        description: None,
        children: graph.tree_view.root.clone(),
    };
    visit(
        &virtual_root,
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
        icon_offset,
        line_thickness,
        label_height,
        true,
    );

    if nodes_out.iter().any(|node| node.description.is_some()) {
        let max_label_right = nodes_out
            .iter()
            .map(|node| node.label_right_edge)
            .fold(0.0_f32, f32::max);
        let description_x = max_label_right + desc_gap;

        for node in &mut nodes_out {
            let Some(description) = &node.description else {
                continue;
            };
            node.description_x = Some(description_x);
            let desc_w = crate::text_metrics::get_computed_text_length(
                description,
                font_size,
                font_family_str,
            );
            total_width = total_width.max(description_x + desc_w + padding_x);
        }
    }

    for node in &mut nodes_out {
        if tree_view_has_css_class(&node.css_class, "highlight") {
            let highlight_width = total_width - node.x + 8.0;
            node.highlight_width = Some(highlight_width);
            total_width = total_width.max(node.x + highlight_width + 2.0);
        }
    }

    let width = total_width.max(1.0);
    let height = total_height.max(1.0);

    let mut nodes = BTreeMap::new();
    nodes.insert(
        "__tree_view_content".to_string(),
        NodeLayout {
            id: "__tree_view_content".to_string(),
            x: 0.0,
            y: 0.0,
            width,
            height,
            label: TextBlock {
                lines: vec![TextLine::plain(String::new())],
                width: 0.0,
                height: 0.0,
            },
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
            treemap_base_text_color: None,
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
