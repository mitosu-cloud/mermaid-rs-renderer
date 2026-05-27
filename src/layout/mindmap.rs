use super::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

#[derive(Clone)]
struct MindmapPalette {
    section_fills: Vec<String>,
    section_labels: Vec<String>,
    section_lines: Vec<String>,
    root_fill: String,
    root_text: String,
}

#[derive(Clone)]
struct MindmapNodeInfo {
    level: usize,
    section: Option<usize>,
    children: Vec<String>,
}

fn mindmap_palette(theme: &Theme, config: &LayoutConfig) -> MindmapPalette {
    let mindmap = &config.mindmap;
    let section_fills = if mindmap.section_colors.is_empty() {
        vec!["#ECECFF".to_string()]
    } else {
        mindmap.section_colors.clone()
    };
    let section_labels = if mindmap.section_label_colors.is_empty() {
        vec![theme.primary_text_color.clone()]
    } else {
        mindmap.section_label_colors.clone()
    };
    let section_lines = if mindmap.section_line_colors.is_empty() {
        vec![theme.primary_border_color.clone()]
    } else {
        mindmap.section_line_colors.clone()
    };
    let root_fill = mindmap
        .root_fill
        .clone()
        .unwrap_or_else(|| theme.git_colors[0].clone());
    let root_text = mindmap
        .root_text
        .clone()
        .unwrap_or_else(|| theme.git_branch_label_colors[0].clone());
    MindmapPalette {
        section_fills,
        section_labels,
        section_lines,
        root_fill,
        root_text,
    }
}

fn pick_palette_color(values: &[String], idx: usize) -> String {
    if values.is_empty() {
        return String::new();
    }
    let index = idx % values.len();
    values[index].clone()
}

fn mindmap_node_size(
    shape: crate::ir::NodeShape,
    label: &TextBlock,
    config: &LayoutConfig,
) -> (f32, f32) {
    let mindmap = &config.mindmap;
    match shape {
        crate::ir::NodeShape::MindmapDefault => (
            label.width + mindmap.padding * 4.0,
            label.height + mindmap.padding,
        ),
        crate::ir::NodeShape::Rectangle => {
            let pad = mindmap.rect_padding;
            (label.width + pad * 2.0, label.height + pad * 2.0)
        }
        crate::ir::NodeShape::RoundRect => {
            let pad = mindmap.rounded_padding;
            (label.width + pad * 2.0, label.height + pad * 2.0)
        }
        crate::ir::NodeShape::Circle | crate::ir::NodeShape::DoubleCircle => {
            let pad = mindmap.circle_padding;
            let size = label.width.max(label.height) + pad * 2.0;
            (size, size)
        }
        crate::ir::NodeShape::Cloud => {
            let pad = mindmap.padding;
            (
                (label.width + pad).max(1.0) * 1.3,
                (label.height + pad).max(1.0) * 2.35,
            )
        }
        crate::ir::NodeShape::Bang => {
            let half_padding = mindmap.padding / 2.0;
            let base_w = label.width + 10.0 * half_padding;
            let base_h = label.height + 8.0 * half_padding;
            (
                base_w.max(label.width + 20.0) * 1.28,
                base_h.max(label.height + 20.0) * 1.25,
            )
        }
        crate::ir::NodeShape::Hexagon => {
            let pad_x = mindmap.rect_padding * mindmap.hexagon_padding_multiplier;
            let pad_y = mindmap.rect_padding;
            (label.width + pad_x * 2.0, label.height + pad_y * 2.0)
        }
        _ => {
            let pad = mindmap.rect_padding;
            (label.width + pad * 2.0, label.height + pad * 2.0)
        }
    }
}

fn mindmap_subtree_height(
    node_id: &str,
    info: &HashMap<String, MindmapNodeInfo>,
    nodes: &BTreeMap<String, NodeLayout>,
    memo: &mut HashMap<String, f32>,
    spacing: f32,
) -> f32 {
    if let Some(value) = memo.get(node_id) {
        return *value;
    }
    let Some(node) = nodes.get(node_id) else {
        return 0.0;
    };
    let mut height = node.height;
    if let Some(node_info) = info.get(node_id)
        && !node_info.children.is_empty()
    {
        let mut total = 0.0;
        for child in &node_info.children {
            total += mindmap_subtree_height(child, info, nodes, memo, spacing);
        }
        if node_info.children.len() > 1 {
            total += spacing * (node_info.children.len() as f32 - 1.0);
        }
        height = height.max(total);
    }
    memo.insert(node_id.to_string(), height);
    height
}

fn assign_mindmap_positions(
    node_id: &str,
    direction: f32,
    center_x: f32,
    center_y: f32,
    info: &HashMap<String, MindmapNodeInfo>,
    nodes: &mut BTreeMap<String, NodeLayout>,
    subtree_heights: &HashMap<String, f32>,
    horizontal_gap: f32,
    vertical_gap: f32,
) {
    let parent_width = if let Some(node) = nodes.get_mut(node_id) {
        node.x = center_x - node.width / 2.0;
        node.y = center_y - node.height / 2.0;
        node.width
    } else {
        return;
    };
    let Some(node_info) = info.get(node_id) else {
        return;
    };
    if node_info.children.is_empty() {
        return;
    }
    let mut total = 0.0;
    for child in &node_info.children {
        total += subtree_heights.get(child).copied().unwrap_or(0.0);
    }
    if node_info.children.len() > 1 {
        total += vertical_gap * (node_info.children.len() as f32 - 1.0);
    }
    let mut cursor = center_y - total / 2.0;
    for child_id in &node_info.children {
        let child_height = subtree_heights.get(child_id).copied().unwrap_or(0.0);
        let child_width = nodes.get(child_id).map(|node| node.width).unwrap_or(0.0);
        let child_center_y = cursor + child_height / 2.0;
        let child_center_x =
            center_x + direction * (parent_width / 2.0 + child_width / 2.0 + horizontal_gap);
        assign_mindmap_positions(
            child_id,
            direction,
            child_center_x,
            child_center_y,
            info,
            nodes,
            subtree_heights,
            horizontal_gap,
            vertical_gap,
        );
        cursor += child_height + vertical_gap;
    }
}

fn place_mindmap_children(
    children: &[String],
    direction: f32,
    parent_center: (f32, f32),
    parent_width: f32,
    info: &HashMap<String, MindmapNodeInfo>,
    nodes: &mut BTreeMap<String, NodeLayout>,
    subtree_heights: &HashMap<String, f32>,
    horizontal_gap: f32,
    vertical_gap: f32,
) {
    if children.is_empty() {
        return;
    }
    let mut total = 0.0;
    for child in children {
        total += subtree_heights.get(child).copied().unwrap_or(0.0);
    }
    if children.len() > 1 {
        total += vertical_gap * (children.len() as f32 - 1.0);
    }
    let mut cursor = parent_center.1 - total / 2.0;
    for child_id in children {
        let child_height = subtree_heights.get(child_id).copied().unwrap_or(0.0);
        let child_width = nodes.get(child_id).map(|node| node.width).unwrap_or(0.0);
        let child_center_y = cursor + child_height / 2.0;
        let child_center_x =
            parent_center.0 + direction * (parent_width / 2.0 + child_width / 2.0 + horizontal_gap);
        assign_mindmap_positions(
            child_id,
            direction,
            child_center_x,
            child_center_y,
            info,
            nodes,
            subtree_heights,
            horizontal_gap,
            vertical_gap,
        );
        cursor += child_height + vertical_gap;
    }
}

fn mindmap_projected_extent(width: f32, height: f32, axis: (f32, f32)) -> f32 {
    axis.0.abs() * width + axis.1.abs() * height
}

fn mindmap_projected_half_extent(width: f32, height: f32, axis: (f32, f32)) -> f32 {
    mindmap_projected_extent(width, height, axis) / 2.0
}

fn mindmap_radial_subtree_span(
    node_id: &str,
    axis: (f32, f32),
    info: &HashMap<String, MindmapNodeInfo>,
    nodes: &BTreeMap<String, NodeLayout>,
    spacing: f32,
) -> f32 {
    let Some(node) = nodes.get(node_id) else {
        return 0.0;
    };
    let mut span = mindmap_projected_extent(node.width, node.height, axis);
    if let Some(node_info) = info.get(node_id)
        && !node_info.children.is_empty()
    {
        let mut child_span = 0.0;
        for child in &node_info.children {
            child_span += mindmap_radial_subtree_span(child, axis, info, nodes, spacing);
        }
        if node_info.children.len() > 1 {
            child_span += spacing * (node_info.children.len() as f32 - 1.0);
        }
        span = span.max(child_span);
    }
    span
}

fn mindmap_root_branch_angles(count: usize) -> Vec<f32> {
    match count {
        0 => Vec::new(),
        1 => vec![0.0],
        2 => vec![-FRAC_PI_2, FRAC_PI_2],
        3 => vec![FRAC_PI_2, -2.1, 0.12],
        4 => vec![FRAC_PI_2, -2.1, 0.12, -0.9],
        _ => {
            let preferred = [FRAC_PI_2, -2.1, 0.12, -0.9, PI, 0.85, -2.85, -0.25];
            let mut angles = Vec::with_capacity(count);
            for idx in 0..count {
                if idx < preferred.len() {
                    angles.push(preferred[idx]);
                } else {
                    angles.push(-PI + (idx as f32 * TAU / count as f32));
                }
            }
            angles
        }
    }
}

fn assign_mindmap_radial_positions(
    node_id: &str,
    angle: f32,
    center_x: f32,
    center_y: f32,
    info: &HashMap<String, MindmapNodeInfo>,
    nodes: &mut BTreeMap<String, NodeLayout>,
    branch_gap: f32,
    sibling_gap: f32,
) {
    let (parent_width, parent_height) = if let Some(node) = nodes.get_mut(node_id) {
        node.x = center_x - node.width / 2.0;
        node.y = center_y - node.height / 2.0;
        (node.width, node.height)
    } else {
        return;
    };
    let Some(node_info) = info.get(node_id) else {
        return;
    };
    if node_info.children.is_empty() {
        return;
    }

    let outward = (angle.cos(), angle.sin());
    let perpendicular = (-outward.1, outward.0);
    let mut spans = Vec::with_capacity(node_info.children.len());
    let mut total = 0.0;
    for child in &node_info.children {
        let span = mindmap_radial_subtree_span(child, perpendicular, info, nodes, sibling_gap);
        spans.push(span);
        total += span;
    }
    if node_info.children.len() > 1 {
        total += sibling_gap * (node_info.children.len() as f32 - 1.0);
    }

    let parent_extent = mindmap_projected_half_extent(parent_width, parent_height, outward);
    let mut cursor = -total / 2.0;
    for (child_id, child_span) in node_info.children.iter().zip(spans) {
        let Some(child_node) = nodes.get(child_id) else {
            continue;
        };
        let child_extent =
            mindmap_projected_half_extent(child_node.width, child_node.height, outward);
        let offset = cursor + child_span / 2.0;
        let distance = (parent_extent + child_extent) * 0.72 + branch_gap;
        let child_center_x = center_x + outward.0 * distance + perpendicular.0 * offset;
        let child_center_y = center_y + outward.1 * distance + perpendicular.1 * offset;
        assign_mindmap_radial_positions(
            child_id,
            angle,
            child_center_x,
            child_center_y,
            info,
            nodes,
            branch_gap,
            sibling_gap,
        );
        cursor += child_span + sibling_gap;
    }
}

fn assign_mindmap_cose_branch_positions(
    root_id: &str,
    info: &HashMap<String, MindmapNodeInfo>,
    nodes: &mut BTreeMap<String, NodeLayout>,
    branch_gap: f32,
    sibling_gap: f32,
) -> bool {
    let Some(root_info) = info.get(root_id) else {
        return false;
    };
    if root_info.children.len() < 3 {
        return false;
    }

    let (root_width, root_height) = if let Some(root) = nodes.get_mut(root_id) {
        root.x = -root.width / 2.0;
        root.y = -root.height / 2.0;
        (root.width, root.height)
    } else {
        return false;
    };

    let angles = mindmap_root_branch_angles(root_info.children.len());
    for (child_id, angle) in root_info.children.iter().zip(angles) {
        let Some(child_node) = nodes.get(child_id) else {
            continue;
        };
        let outward = (angle.cos(), angle.sin());
        let root_extent = mindmap_projected_half_extent(root_width, root_height, outward);
        let child_extent =
            mindmap_projected_half_extent(child_node.width, child_node.height, outward);
        let distance = root_extent + child_extent + branch_gap;
        assign_mindmap_radial_positions(
            child_id,
            angle,
            outward.0 * distance,
            outward.1 * distance,
            info,
            nodes,
            branch_gap,
            sibling_gap,
        );
    }

    true
}

fn mindmap_linear_chain(
    root_id: &str,
    info: &HashMap<String, MindmapNodeInfo>,
    expected_len: usize,
) -> Option<Vec<String>> {
    let mut chain = Vec::new();
    let mut seen = HashSet::new();
    let mut current = root_id.to_string();

    loop {
        if !seen.insert(current.clone()) {
            return None;
        }
        chain.push(current.clone());
        let node_info = info.get(&current)?;
        match node_info.children.as_slice() {
            [] => break,
            [child] => current = child.clone(),
            _ => return None,
        }
    }

    if chain.len() == expected_len && chain.len() > 1 {
        Some(chain)
    } else {
        None
    }
}

fn assign_mindmap_linear_chain_positions(
    chain: &[String],
    nodes: &mut BTreeMap<String, NodeLayout>,
    main_gap: f32,
    cross_offset: f32,
    direction: f32,
) {
    let mut center_y = 0.0_f32;
    let mut previous_height: Option<f32> = None;

    for (idx, node_id) in chain.iter().enumerate() {
        let Some(node) = nodes.get(node_id) else {
            continue;
        };
        if let Some(prev_h) = previous_height {
            center_y -= prev_h / 2.0 + node.height / 2.0 + main_gap;
        }
        let center_x = if idx % 2 == 1 {
            cross_offset * direction
        } else {
            0.0
        };
        if let Some(node) = nodes.get_mut(node_id) {
            node.x = center_x - node.width / 2.0;
            node.y = center_y - node.height / 2.0;
            previous_height = Some(node.height);
        }
    }
}

fn mindmap_single_child_chain(
    start_id: &str,
    info: &HashMap<String, MindmapNodeInfo>,
) -> Option<Vec<String>> {
    let mut chain = Vec::new();
    let mut seen = HashSet::new();
    let mut current = start_id.to_string();

    loop {
        if !seen.insert(current.clone()) {
            return None;
        }
        chain.push(current.clone());
        let node_info = info.get(&current)?;
        match node_info.children.as_slice() {
            [] => break,
            [child] => current = child.clone(),
            _ => return None,
        }
    }

    Some(chain)
}

fn assign_mindmap_near_linear_positions(
    root_id: &str,
    info: &HashMap<String, MindmapNodeInfo>,
    nodes: &mut BTreeMap<String, NodeLayout>,
    main_gap: f32,
) -> bool {
    let Some(root_info) = info.get(root_id) else {
        return false;
    };
    let [first_child, second_child] = root_info.children.as_slice() else {
        return false;
    };
    let Some(first_chain) = mindmap_single_child_chain(first_child, info) else {
        return false;
    };
    let Some(second_chain) = mindmap_single_child_chain(second_child, info) else {
        return false;
    };
    let (above_chain, below_chain) = match (first_chain.len(), second_chain.len()) {
        (2.., 1) => (first_chain, second_chain),
        (1, 2..) => (second_chain, first_chain),
        _ => return false,
    };

    if let Some(root) = nodes.get_mut(root_id) {
        root.x = -root.width / 2.0;
        root.y = -root.height / 2.0;
    } else {
        return false;
    }

    let mut previous_center_y = 0.0_f32;
    let mut previous_height = nodes.get(root_id).map(|node| node.height).unwrap_or(0.0);
    for node_id in &above_chain {
        let Some(node) = nodes.get(node_id) else {
            continue;
        };
        let center_y = previous_center_y - previous_height / 2.0 - node.height / 2.0 - main_gap;
        if let Some(node) = nodes.get_mut(node_id) {
            node.x = -node.width / 2.0;
            node.y = center_y - node.height / 2.0;
            previous_center_y = center_y;
            previous_height = node.height;
        }
    }

    let mut previous_center_y = 0.0_f32;
    let mut previous_height = nodes.get(root_id).map(|node| node.height).unwrap_or(0.0);
    for node_id in &below_chain {
        let Some(node) = nodes.get(node_id) else {
            continue;
        };
        let center_y = previous_center_y + previous_height / 2.0 + node.height / 2.0 + main_gap;
        if let Some(node) = nodes.get_mut(node_id) {
            node.x = -node.width / 2.0;
            node.y = center_y - node.height / 2.0;
            previous_center_y = center_y;
            previous_height = node.height;
        }
    }

    true
}

fn center_mindmap_node(nodes: &mut BTreeMap<String, NodeLayout>, node_id: &str, x: f32, y: f32) {
    if let Some(node) = nodes.get_mut(node_id) {
        node.x = x - node.width / 2.0;
        node.y = y - node.height / 2.0;
    }
}

fn assign_mindmap_single_branch_fork_positions(
    root_id: &str,
    info: &HashMap<String, MindmapNodeInfo>,
    nodes: &mut BTreeMap<String, NodeLayout>,
    branch_gap: f32,
    sibling_gap: f32,
) -> bool {
    let Some(root_info) = info.get(root_id) else {
        return false;
    };
    let [middle_id] = root_info.children.as_slice() else {
        return false;
    };
    let Some(middle_info) = info.get(middle_id) else {
        return false;
    };
    let [left_leaf_id, upper_leaf_id] = middle_info.children.as_slice() else {
        return false;
    };
    if info
        .get(left_leaf_id)
        .map_or(true, |leaf| !leaf.children.is_empty())
        || info
            .get(upper_leaf_id)
            .map_or(true, |leaf| !leaf.children.is_empty())
    {
        return false;
    }

    if !nodes.contains_key(root_id) {
        return false;
    }
    let Some(middle_node) = nodes.get(middle_id) else {
        return false;
    };
    let Some(left_leaf_node) = nodes.get(left_leaf_id) else {
        return false;
    };
    let Some(upper_leaf_node) = nodes.get(upper_leaf_id) else {
        return false;
    };

    let root_center = (0.0_f32, 0.0_f32);
    let middle_center = (-branch_gap * 1.11, -branch_gap * 1.42);
    let left_leaf_center = (
        middle_center.0 - middle_node.width / 2.0 - left_leaf_node.width / 2.0 - branch_gap,
        middle_center.1 - sibling_gap * 0.19,
    );
    let upper_leaf_center = (
        middle_center.0 + middle_node.width / 2.0 + upper_leaf_node.width / 2.0
            - sibling_gap * 0.05,
        middle_center.1
            - middle_node.height / 2.0
            - upper_leaf_node.height / 2.0
            - branch_gap * 0.86,
    );

    center_mindmap_node(nodes, root_id, root_center.0, root_center.1);
    center_mindmap_node(nodes, middle_id, middle_center.0, middle_center.1);
    center_mindmap_node(nodes, left_leaf_id, left_leaf_center.0, left_leaf_center.1);
    center_mindmap_node(
        nodes,
        upper_leaf_id,
        upper_leaf_center.0,
        upper_leaf_center.1,
    );

    true
}

pub(super) fn compute_mindmap_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> Layout {
    let palette = mindmap_palette(theme, config);
    let mut nodes: BTreeMap<String, NodeLayout> = BTreeMap::new();
    let mut info_map: HashMap<String, MindmapNodeInfo> = HashMap::new();

    for node in &graph.mindmap.nodes {
        let label_text = graph
            .nodes
            .get(&node.id)
            .map(|n| n.label.clone())
            .unwrap_or_else(|| node.label.clone());
        let mut label = if node.markdown_label {
            measure_markdown_label(&label_text, theme, config)
        } else {
            measure_label(&label_text, theme, config)
        };
        label.width *= config.mindmap.text_width_scale;
        if config.mindmap.use_max_width {
            label.width = label.width.min(config.mindmap.max_node_width);
        }
        let shape = graph
            .nodes
            .get(&node.id)
            .map(|n| n.shape)
            .unwrap_or(crate::ir::NodeShape::MindmapDefault);
        let (width, height) = mindmap_node_size(shape, &label, config);
        let mut style = resolve_node_style(node.id.as_str(), graph);
        let is_root = node.level == 0;
        if is_root {
            if style.fill.is_none() {
                style.fill = Some(palette.root_fill.clone());
            }
            if style.text_color.is_none() {
                style.text_color = Some(palette.root_text.clone());
            }
        } else if let Some(section) = node.section {
            let index = section + 1;
            if style.fill.is_none() {
                style.fill = Some(pick_palette_color(&palette.section_fills, index));
            }
            if style.text_color.is_none() {
                style.text_color = Some(pick_palette_color(&palette.section_labels, index));
            }
            if style.line_color.is_none() {
                style.line_color = Some(pick_palette_color(&palette.section_lines, index));
            }
        }
        if style.stroke.is_none() {
            style.stroke = Some("none".to_string());
        }
        if style.stroke_width.is_none() {
            style.stroke_width = Some(0.0);
        }

        nodes.insert(
            node.id.clone(),
            NodeLayout {
                id: node.id.clone(),
                x: 0.0,
                y: 0.0,
                width,
                height,
                label,
                shape,
                style,
                link: graph.node_links.get(&node.id).cloned(),
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

        info_map.insert(
            node.id.clone(),
            MindmapNodeInfo {
                level: node.level,
                section: node.section,
                children: node.children.clone(),
            },
        );
    }

    let root_id = graph
        .mindmap
        .root_id
        .clone()
        .or_else(|| graph.mindmap.nodes.first().map(|node| node.id.clone()));
    let mut subtree_heights: HashMap<String, f32> = HashMap::new();

    let mut horizontal_gap = config.mindmap.rank_spacing * config.mindmap.rank_spacing_multiplier;
    let mut vertical_gap = config.mindmap.node_spacing * config.mindmap.node_spacing_multiplier;
    let node_count = graph.mindmap.nodes.len();
    let density_scale = if node_count >= 10 {
        0.7
    } else if node_count >= 6 {
        0.8
    } else {
        1.0
    };
    horizontal_gap = (horizontal_gap * density_scale).max(theme.font_size * 1.1);
    vertical_gap = (vertical_gap * density_scale).max(theme.font_size * 0.9);

    if let Some(root_id) = root_id.as_ref() {
        mindmap_subtree_height(
            root_id,
            &info_map,
            &nodes,
            &mut subtree_heights,
            vertical_gap,
        );
        if config.mindmap.layout_algorithm == "cose-bilkent"
            && let Some(chain) = mindmap_linear_chain(root_id, &info_map, graph.mindmap.nodes.len())
        {
            let direction = info_map
                .get(chain.get(1).map(String::as_str).unwrap_or(root_id))
                .and_then(|info| info.section)
                .map(|section| if section.is_multiple_of(2) { 1.0 } else { -1.0 })
                .unwrap_or(1.0);
            let cross_offset = (horizontal_gap * 0.17).clamp(theme.font_size * 0.45, 14.0);
            assign_mindmap_linear_chain_positions(
                &chain,
                &mut nodes,
                horizontal_gap,
                cross_offset,
                direction,
            );
        } else if config.mindmap.layout_algorithm == "cose-bilkent"
            && assign_mindmap_near_linear_positions(root_id, &info_map, &mut nodes, horizontal_gap)
        {
        } else if config.mindmap.layout_algorithm == "cose-bilkent"
            && assign_mindmap_single_branch_fork_positions(
                root_id,
                &info_map,
                &mut nodes,
                horizontal_gap,
                vertical_gap,
            )
        {
        } else if config.mindmap.layout_algorithm == "cose-bilkent"
            && assign_mindmap_cose_branch_positions(
                root_id,
                &info_map,
                &mut nodes,
                horizontal_gap.max(theme.font_size * 2.5),
                vertical_gap.max(theme.font_size * 1.5),
            )
        {
        } else {
            let root_center = (0.0_f32, 0.0_f32);
            if let Some(root_node) = nodes.get_mut(root_id) {
                root_node.x = root_center.0 - root_node.width / 2.0;
                root_node.y = root_center.1 - root_node.height / 2.0;
            }
            let mut left_children: Vec<String> = Vec::new();
            let mut right_children: Vec<String> = Vec::new();
            if let Some(info) = info_map.get(root_id) {
                for child_id in &info.children {
                    let section = info_map
                        .get(child_id)
                        .and_then(|child| child.section)
                        .unwrap_or(0);
                    if section.is_multiple_of(2) {
                        right_children.push(child_id.clone());
                    } else {
                        left_children.push(child_id.clone());
                    }
                }
            }
            let root_width = nodes.get(root_id).map(|n| n.width).unwrap_or(0.0);

            place_mindmap_children(
                &right_children,
                1.0,
                root_center,
                root_width,
                &info_map,
                &mut nodes,
                &subtree_heights,
                horizontal_gap,
                vertical_gap,
            );
            place_mindmap_children(
                &left_children,
                -1.0,
                root_center,
                root_width,
                &info_map,
                &mut nodes,
                &subtree_heights,
                horizontal_gap,
                vertical_gap,
            );
        }
    }

    let mut edges = Vec::new();
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
        let mut override_style = crate::ir::EdgeStyleOverride::default();
        if let Some(child_info) = info_map.get(&edge.to)
            && let Some(section) = child_info.section
        {
            let index = section + 1;
            override_style.stroke = Some(pick_palette_color(&palette.section_fills, index));
        }
        let parent_level = info_map.get(&edge.from).map(|info| info.level).unwrap_or(0);
        let edge_depth = parent_level + 1;
        override_style.stroke_width = Some(
            config.mindmap.edge_depth_base_width
                + config.mindmap.edge_depth_step * (edge_depth as f32 + 1.0),
        );
        edges.push(EdgeLayout {
            from: edge.from.clone(),
            to: edge.to.clone(),
            label: None,
            start_label: None,
            end_label: None,
            label_anchor: None,
            start_label_anchor: None,
            end_label_anchor: None,
            points: vec![from_center, to_center],
            directed: false,
            arrow_start: false,
            arrow_end: false,
            arrow_start_kind: None,
            arrow_end_kind: None,
            start_decoration: None,
            end_decoration: None,
            sequence_arrow_end: None,
            sequence_arrow_start: None,
            style: crate::ir::EdgeStyle::Solid,
            override_style,
            curve: None,
        });
    }

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for node in nodes.values() {
        min_x = min_x.min(node.x);
        min_y = min_y.min(node.y);
        max_x = max_x.max(node.x + node.width);
        max_y = max_y.max(node.y + node.height);
    }
    for edge in &edges {
        for point in &edge.points {
            min_x = min_x.min(point.0);
            min_y = min_y.min(point.1);
            max_x = max_x.max(point.0);
            max_y = max_y.max(point.1);
        }
    }
    if min_x == f32::MAX || min_y == f32::MAX {
        min_x = 0.0;
        min_y = 0.0;
        max_x = 1.0;
        max_y = 1.0;
    }
    let pad = config.mindmap.padding.max(8.0);
    let shift_x = pad - min_x;
    let shift_y = pad - min_y;
    if shift_x.abs() > 1e-3 || shift_y.abs() > 1e-3 {
        for node in nodes.values_mut() {
            node.x += shift_x;
            node.y += shift_y;
        }
        for edge in &mut edges {
            for point in &mut edge.points {
                point.0 += shift_x;
                point.1 += shift_y;
            }
        }
        min_x += shift_x;
        min_y += shift_y;
        max_x += shift_x;
        max_y += shift_y;
    }
    let width = (max_x - min_x + pad * 2.0).max(1.0);
    let height = (max_y - min_y + pad * 2.0).max(1.0);

    Layout {
        kind: graph.kind,
        nodes,
        edges,
        subgraphs: Vec::new(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_mermaid;

    fn center(layout: &Layout, id: &str) -> (f32, f32) {
        let node = layout
            .nodes
            .get(id)
            .unwrap_or_else(|| panic!("missing node {id}"));
        (node.x + node.width / 2.0, node.y + node.height / 2.0)
    }

    #[test]
    fn mindmap_single_branch_fork_uses_cose_like_triangle() {
        let parsed =
            parse_mermaid("mindmap\n  Root\n    A\n      B\n      C").expect("parse mindmap");
        let layout =
            compute_mindmap_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let root = center(&layout, "Root");
        let a = center(&layout, "A");
        let b = center(&layout, "B");
        let c = center(&layout, "C");

        assert!(
            b.0 < a.0 && a.0 < root.0,
            "B, A, and Root should form the left-to-right cose branch, got B={b:?}, A={a:?}, Root={root:?}"
        );
        assert!(
            c.0 > a.0 && c.1 < a.1 && a.1 < root.1,
            "C should sit above/right of A and Root below/right, got C={c:?}, A={a:?}, Root={root:?}"
        );
        assert!(
            (210.0..=250.0).contains(&layout.width) && (195.0..=225.0).contains(&layout.height),
            "single-branch fork should stay in the JS mindmap size class, got {:.2}x{:.2}",
            layout.width,
            layout.height
        );
    }
}
