use super::*;

const TREEMAP_SECTION_HEADER_HEIGHT: f32 = 25.0;
const TREEMAP_SECTION_INNER_PADDING: f32 = 10.0;
const TREEMAP_SQUARIFY_RATIO: f32 = 1.618_034;
const TREEMAP_VALUE_EPSILON: f32 = 1e-3;
const TREEMAP_SCALE_COLORS: [&str; 12] = [
    "hsl(240, 100%, 76.2745098039%)",
    "hsl(60, 100%, 73.5294117647%)",
    "hsl(80, 100%, 76.2745098039%)",
    "hsl(270, 100%, 76.2745098039%)",
    "hsl(300, 100%, 76.2745098039%)",
    "hsl(330, 100%, 76.2745098039%)",
    "hsl(0, 100%, 76.2745098039%)",
    "hsl(30, 100%, 76.2745098039%)",
    "hsl(90, 100%, 76.2745098039%)",
    "hsl(150, 100%, 76.2745098039%)",
    "hsl(180, 100%, 76.2745098039%)",
    "hsl(210, 100%, 76.2745098039%)",
];
const TREEMAP_SCALE_PEER_COLORS: [&str; 12] = [
    "hsl(240, 100%, 61.2745098039%)",
    "hsl(60, 100%, 48.5294117647%)",
    "hsl(80, 100%, 61.2745098039%)",
    "hsl(270, 100%, 61.2745098039%)",
    "hsl(300, 100%, 61.2745098039%)",
    "hsl(330, 100%, 61.2745098039%)",
    "hsl(0, 100%, 61.2745098039%)",
    "hsl(30, 100%, 61.2745098039%)",
    "hsl(90, 100%, 61.2745098039%)",
    "hsl(150, 100%, 61.2745098039%)",
    "hsl(180, 100%, 61.2745098039%)",
    "hsl(210, 100%, 61.2745098039%)",
];
const TREEMAP_SCALE_LABEL_COLORS: [&str; 12] = [
    "#ffffff", "black", "black", "#ffffff", "black", "black", "black", "black", "black", "black",
    "black", "black",
];
const TREEMAP_FOREST_SCALE_COLORS: [&str; 2] = [
    "hsl(78.1578947368, 58.4615384615%, 64.5098039216%)",
    "hsl(98.961038961, 100%, 74.9019607843%)",
];
const TREEMAP_FOREST_SCALE_PEER_COLORS: [&str; 2] = [
    "hsl(78.1578947368, 58.4615384615%, 39.5098039216%)",
    "hsl(98.961038961, 100%, 39.9019607843%)",
];

pub(super) fn compute_treemap_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> Layout {
    let mut nodes: BTreeMap<String, NodeLayout> = BTreeMap::new();
    let edges = Vec::new();
    let subgraphs = Vec::new();

    let internal_width = config.treemap.width.max(1.0);
    let internal_height = config.treemap.height.max(1.0);
    let viewport_padding = config.treemap.diagram_padding.max(0.0);
    let viewport_x = TREEMAP_SECTION_INNER_PADDING - viewport_padding;
    let viewport_y =
        TREEMAP_SECTION_HEADER_HEIGHT + TREEMAP_SECTION_INNER_PADDING - viewport_padding;
    let width =
        (internal_width - TREEMAP_SECTION_INNER_PADDING * 2.0 + viewport_padding * 2.0).max(1.0);
    let height =
        (internal_height - TREEMAP_SECTION_HEADER_HEIGHT - TREEMAP_SECTION_INNER_PADDING * 2.0
            + viewport_padding * 2.0)
            .max(1.0);
    let root_rect = TreemapRect::new(0.0, 0.0, internal_width, internal_height);

    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    let mut parents: HashMap<String, String> = HashMap::new();
    for edge in &graph.edges {
        children
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        parents.insert(edge.to.clone(), edge.from.clone());
    }

    let mut roots: Vec<String> = graph
        .nodes
        .keys()
        .filter(|id| !parents.contains_key(*id))
        .cloned()
        .collect();
    roots.sort_by_key(|id| graph.node_order.get(id).copied().unwrap_or(usize::MAX));

    if !roots.is_empty() {
        let mut tiles = vec![TreemapTileNode {
            id: None,
            parent: None,
            children: Vec::new(),
            value: 0.0,
            x0: root_rect.x,
            y0: root_rect.y,
            x1: root_rect.x + root_rect.w,
            y1: root_rect.y + root_rect.h,
        }];
        for root_id in roots {
            let child_idx =
                build_treemap_tile_node(&root_id, Some(0), graph, &children, &mut tiles);
            tiles[0].value += tiles[child_idx].value;
            tiles[0].children.push(child_idx);
        }
        sort_treemap_tile_children(0, &mut tiles, graph);
        let mut padding_stack = vec![0.0];
        position_treemap_tile_node(
            0,
            0,
            &mut tiles,
            &mut padding_stack,
            config.treemap.gap.max(0.0),
        );
        round_treemap_tiles(&mut tiles);
        insert_treemap_layout_nodes(
            &tiles, viewport_x, viewport_y, graph, theme, config, &mut nodes,
        );
    }

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

#[derive(Debug, Clone, Copy)]
struct TreemapRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl TreemapRect {
    fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
}

#[derive(Debug, Clone)]
struct TreemapTileNode {
    id: Option<String>,
    parent: Option<usize>,
    children: Vec<usize>,
    value: f32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

fn build_treemap_tile_node(
    id: &str,
    parent: Option<usize>,
    graph: &Graph,
    children: &HashMap<String, Vec<String>>,
    tiles: &mut Vec<TreemapTileNode>,
) -> usize {
    let idx = tiles.len();
    tiles.push(TreemapTileNode {
        id: Some(id.to_string()),
        parent,
        children: Vec::new(),
        value: 0.0,
        x0: 0.0,
        y0: 0.0,
        x1: 0.0,
        y1: 0.0,
    });

    let mut value = graph
        .nodes
        .get(id)
        .and_then(|node| node.value)
        .unwrap_or(0.0)
        .max(0.0);

    if let Some(child_ids) = children.get(id) {
        let mut sorted_child_ids = child_ids.clone();
        sorted_child_ids.sort_by_key(|child_id| {
            graph
                .node_order
                .get(child_id)
                .copied()
                .unwrap_or(usize::MAX)
        });
        for child_id in sorted_child_ids {
            let child_idx = build_treemap_tile_node(&child_id, Some(idx), graph, children, tiles);
            value += tiles[child_idx].value;
            tiles[idx].children.push(child_idx);
        }
    }

    tiles[idx].value = value;
    idx
}

fn sort_treemap_tile_children(idx: usize, tiles: &mut [TreemapTileNode], graph: &Graph) {
    let child_indices = tiles[idx].children.clone();
    for child_idx in child_indices {
        sort_treemap_tile_children(child_idx, tiles, graph);
    }

    let mut child_indices = std::mem::take(&mut tiles[idx].children);
    child_indices.sort_by(|a, b| {
        tiles[*b]
            .value
            .partial_cmp(&tiles[*a].value)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                treemap_tile_order(*a, tiles, graph).cmp(&treemap_tile_order(*b, tiles, graph))
            })
            .then_with(|| treemap_tile_id(*a, tiles).cmp(treemap_tile_id(*b, tiles)))
    });
    tiles[idx].children = child_indices;
}

fn treemap_tile_order(idx: usize, tiles: &[TreemapTileNode], graph: &Graph) -> usize {
    tiles[idx]
        .id
        .as_ref()
        .and_then(|id| graph.node_order.get(id))
        .copied()
        .unwrap_or(usize::MAX)
}

fn treemap_tile_id(idx: usize, tiles: &[TreemapTileNode]) -> &str {
    tiles[idx].id.as_deref().unwrap_or("")
}

fn position_treemap_tile_node(
    idx: usize,
    depth: usize,
    tiles: &mut [TreemapTileNode],
    padding_stack: &mut Vec<f32>,
    inner_padding: f32,
) {
    let p = padding_stack.get(depth).copied().unwrap_or(0.0);
    let mut x0 = tiles[idx].x0 + p;
    let mut y0 = tiles[idx].y0 + p;
    let mut x1 = tiles[idx].x1 - p;
    let mut y1 = tiles[idx].y1 - p;
    if x1 < x0 {
        let x = (x0 + x1) / 2.0;
        x0 = x;
        x1 = x;
    }
    if y1 < y0 {
        let y = (y0 + y1) / 2.0;
        y0 = y;
        y1 = y;
    }

    tiles[idx].x0 = x0;
    tiles[idx].y0 = y0;
    tiles[idx].x1 = x1;
    tiles[idx].y1 = y1;

    if !tiles[idx].children.is_empty() {
        let child_padding = inner_padding / 2.0;
        if padding_stack.len() <= depth + 1 {
            padding_stack.resize(depth + 2, 0.0);
        }
        padding_stack[depth + 1] = child_padding;

        let mut child_x0 = x0 + TREEMAP_SECTION_INNER_PADDING - child_padding;
        let mut child_y0 =
            y0 + TREEMAP_SECTION_HEADER_HEIGHT + TREEMAP_SECTION_INNER_PADDING - child_padding;
        let mut child_x1 = x1 - TREEMAP_SECTION_INNER_PADDING + child_padding;
        let mut child_y1 = y1 - TREEMAP_SECTION_INNER_PADDING + child_padding;
        if child_x1 < child_x0 {
            let x = (child_x0 + child_x1) / 2.0;
            child_x0 = x;
            child_x1 = x;
        }
        if child_y1 < child_y0 {
            let y = (child_y0 + child_y1) / 2.0;
            child_y0 = y;
            child_y1 = y;
        }
        squarify_treemap_tile(idx, child_x0, child_y0, child_x1, child_y1, tiles);
    }

    let child_indices = tiles[idx].children.clone();
    for child_idx in child_indices {
        position_treemap_tile_node(child_idx, depth + 1, tiles, padding_stack, inner_padding);
    }
}

fn squarify_treemap_tile(
    parent_idx: usize,
    mut x0: f32,
    mut y0: f32,
    x1: f32,
    y1: f32,
    tiles: &mut [TreemapTileNode],
) {
    let children = tiles[parent_idx].children.clone();
    let mut i0 = 0;
    let mut i1 = 0;
    let n = children.len();
    let mut value = tiles[parent_idx].value;

    while i0 < n {
        let dx = x1 - x0;
        let dy = y1 - y0;
        if dx <= 0.0 && dy <= 0.0 {
            for child_idx in &children[i0..] {
                tiles[*child_idx].x0 = x0;
                tiles[*child_idx].y0 = y0;
                tiles[*child_idx].x1 = x0;
                tiles[*child_idx].y1 = y0;
            }
            return;
        }
        if value <= TREEMAP_VALUE_EPSILON {
            let row = &children[i0..];
            if dx < dy {
                treemap_dice(row, x0, y0, x1, y1, 0.0, tiles);
            } else {
                treemap_slice(row, x0, y0, x1, y1, 0.0, tiles);
            }
            return;
        }

        let mut sum_value;
        loop {
            if i1 >= n {
                return;
            }
            sum_value = tiles[children[i1]].value;
            i1 += 1;
            if sum_value > 0.0 || i1 >= n {
                break;
            }
        }
        if sum_value <= 0.0 {
            let row = &children[i0..];
            if dx < dy {
                treemap_dice(row, x0, y0, x1, y1, 0.0, tiles);
            } else {
                treemap_slice(row, x0, y0, x1, y1, 0.0, tiles);
            }
            return;
        }

        let mut min_value = sum_value;
        let mut max_value = sum_value;
        let alpha = (dy / dx).max(dx / dy) / (value * TREEMAP_SQUARIFY_RATIO);
        let mut beta = sum_value * sum_value * alpha;
        let mut min_ratio = (max_value / beta).max(beta / min_value);

        while i1 < n {
            let node_value = tiles[children[i1]].value;
            sum_value += node_value;
            min_value = min_value.min(node_value);
            max_value = max_value.max(node_value);
            beta = sum_value * sum_value * alpha;
            let new_ratio = (max_value / beta).max(beta / min_value);
            if new_ratio > min_ratio {
                sum_value -= node_value;
                break;
            }
            min_ratio = new_ratio;
            i1 += 1;
        }

        let row = &children[i0..i1];
        if dx < dy {
            let row_y1 = y0 + dy * sum_value / value;
            treemap_dice(row, x0, y0, x1, row_y1, sum_value, tiles);
            y0 = row_y1;
        } else {
            let row_x1 = x0 + dx * sum_value / value;
            treemap_slice(row, x0, y0, row_x1, y1, sum_value, tiles);
            x0 = row_x1;
        }
        value -= sum_value;
        i0 = i1;
    }
}

fn treemap_dice(
    row: &[usize],
    mut x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    row_value: f32,
    tiles: &mut [TreemapTileNode],
) {
    let k = if row_value > 0.0 {
        (x1 - x0) / row_value
    } else {
        0.0
    };
    for child_idx in row {
        let child_x1 = x0 + tiles[*child_idx].value * k;
        tiles[*child_idx].x0 = x0;
        tiles[*child_idx].y0 = y0;
        tiles[*child_idx].x1 = child_x1;
        tiles[*child_idx].y1 = y1;
        x0 = child_x1;
    }
}

fn treemap_slice(
    row: &[usize],
    x0: f32,
    mut y0: f32,
    x1: f32,
    y1: f32,
    row_value: f32,
    tiles: &mut [TreemapTileNode],
) {
    let k = if row_value > 0.0 {
        (y1 - y0) / row_value
    } else {
        0.0
    };
    for child_idx in row {
        let child_y1 = y0 + tiles[*child_idx].value * k;
        tiles[*child_idx].x0 = x0;
        tiles[*child_idx].y0 = y0;
        tiles[*child_idx].x1 = x1;
        tiles[*child_idx].y1 = child_y1;
        y0 = child_y1;
    }
}

fn round_treemap_tiles(tiles: &mut [TreemapTileNode]) {
    for tile in tiles {
        tile.x0 = tile.x0.round();
        tile.y0 = tile.y0.round();
        tile.x1 = tile.x1.round();
        tile.y1 = tile.y1.round();
    }
}

fn insert_treemap_layout_nodes(
    tiles: &[TreemapTileNode],
    viewport_x: f32,
    viewport_y: f32,
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
    nodes_out: &mut BTreeMap<String, NodeLayout>,
) {
    let branch_color_indices = treemap_branch_color_indices(tiles);
    let leaf_label_indices = treemap_leaf_label_indices(tiles, branch_color_indices.len());

    for (tile_idx, tile) in tiles.iter().enumerate().skip(1) {
        let Some(id) = tile.id.as_ref() else {
            continue;
        };
        let node_rect = TreemapRect::new(
            tile.x0 - viewport_x,
            tile.y0 - viewport_y,
            (tile.x1 - tile.x0).max(0.0),
            (tile.y1 - tile.y0).max(0.0),
        );
        let is_leaf = tile.children.is_empty();
        if let Some(node) = graph.nodes.get(id) {
            let mut style = resolve_node_style(id, graph);
            let (default_fill, default_stroke, default_text, default_stroke_width) =
                treemap_default_style(
                    tile_idx,
                    tiles,
                    &branch_color_indices,
                    &leaf_label_indices,
                    theme,
                    is_leaf,
                );
            let base_text_color = default_text.clone();
            if style.fill.is_none() {
                style.fill = Some(default_fill);
            }
            if style.stroke.is_none() {
                style.stroke = Some(default_stroke);
            }
            if style.stroke_width.is_none() {
                style.stroke_width = Some(default_stroke_width);
            }
            if style.text_color.is_none() {
                style.text_color = Some(default_text);
            }

            let label = measure_label(&node.label, theme, config);
            let pad_x = config.treemap.label_padding_x;
            let pad_y = config.treemap.label_padding_y;
            let fits = label.width <= (node_rect.w - pad_x * 2.0).max(0.0)
                && label.height <= (node_rect.h - pad_y * 2.0).max(0.0);
            let area = node_rect.w * node_rect.h;
            let label = if is_leaf || (fits && area >= config.treemap.min_label_area) {
                label
            } else {
                TextBlock {
                    lines: vec![TextLine::plain(String::new())],
                    width: 0.0,
                    height: 0.0,
                }
            };

            let sub_label = if config.treemap.show_values && tile.value > TREEMAP_VALUE_EPSILON {
                Some(measure_label(
                    &format_treemap_value(tile.value, &config.treemap.value_format),
                    theme,
                    config,
                ))
            } else {
                None
            };

            nodes_out.insert(
                id.clone(),
                NodeLayout {
                    id: node.id.clone(),
                    x: node_rect.x,
                    y: node_rect.y,
                    width: node_rect.w,
                    height: node_rect.h,
                    label,
                    shape: crate::ir::NodeShape::Rectangle,
                    style,
                    link: graph.node_links.get(id).cloned(),
                    anchor_subgraph: None,
                    hidden: false,
                    icon: None,
                    img: None,
                    img_w: None,
                    img_h: None,
                    sub_label,
                    is_treemap_leaf: is_leaf,
                    treemap_base_text_color: Some(base_text_color),
                },
            );
        }
    }
}

fn treemap_branch_color_indices(tiles: &[TreemapTileNode]) -> HashMap<usize, usize> {
    let mut branches = Vec::new();
    collect_treemap_branches(0, tiles, &mut branches);
    branches
        .into_iter()
        .filter(|idx| *idx != 0)
        .enumerate()
        .map(|(color_idx, tile_idx)| (tile_idx, color_idx))
        .collect()
}

fn treemap_leaf_label_indices(
    tiles: &[TreemapTileNode],
    first_leaf_label_index: usize,
) -> HashMap<usize, usize> {
    let mut leaves = Vec::new();
    collect_treemap_leaves(0, tiles, &mut leaves);
    leaves
        .into_iter()
        .enumerate()
        .map(|(leaf_idx, tile_idx)| (tile_idx, first_leaf_label_index + leaf_idx))
        .collect()
}

fn collect_treemap_branches(idx: usize, tiles: &[TreemapTileNode], out: &mut Vec<usize>) {
    if !tiles[idx].children.is_empty() {
        out.push(idx);
    }
    for child_idx in &tiles[idx].children {
        collect_treemap_branches(*child_idx, tiles, out);
    }
}

fn collect_treemap_leaves(idx: usize, tiles: &[TreemapTileNode], out: &mut Vec<usize>) {
    if tiles[idx].children.is_empty() {
        out.push(idx);
        return;
    }
    for child_idx in &tiles[idx].children {
        collect_treemap_leaves(*child_idx, tiles, out);
    }
}

fn treemap_default_style(
    tile_idx: usize,
    tiles: &[TreemapTileNode],
    branch_color_indices: &HashMap<usize, usize>,
    leaf_label_indices: &HashMap<usize, usize>,
    theme: &Theme,
    is_leaf: bool,
) -> (String, String, String, f32) {
    if is_leaf {
        let fill_idx = tiles[tile_idx]
            .parent
            .and_then(|parent_idx| branch_color_indices.get(&parent_idx).copied())
            .unwrap_or_else(|| leaf_label_indices.get(&tile_idx).copied().unwrap_or(0));
        let label_idx = leaf_label_indices
            .get(&tile_idx)
            .copied()
            .unwrap_or(fill_idx);
        let fill = treemap_scale_color(fill_idx, theme);
        let text = treemap_label_color(label_idx, theme);
        (fill.clone(), fill, text, 3.0)
    } else {
        let color_idx = branch_color_indices.get(&tile_idx).copied().unwrap_or(0);
        (
            treemap_scale_color(color_idx, theme),
            treemap_scale_peer_color(color_idx, theme),
            treemap_label_color(color_idx, theme),
            2.0,
        )
    }
}

fn treemap_scale_color(idx: usize, theme: &Theme) -> String {
    theme.cscale_colors.get(idx).cloned().unwrap_or_else(|| {
        if is_forest_treemap_theme(theme) && idx < TREEMAP_FOREST_SCALE_COLORS.len() {
            TREEMAP_FOREST_SCALE_COLORS[idx].to_string()
        } else {
            TREEMAP_SCALE_COLORS[idx % TREEMAP_SCALE_COLORS.len()].to_string()
        }
    })
}

fn treemap_scale_peer_color(idx: usize, theme: &Theme) -> String {
    if let Some(custom) = theme.cscale_colors.get(idx) {
        return custom.clone();
    }
    if is_forest_treemap_theme(theme) && idx < TREEMAP_FOREST_SCALE_PEER_COLORS.len() {
        return TREEMAP_FOREST_SCALE_PEER_COLORS[idx].to_string();
    }
    TREEMAP_SCALE_PEER_COLORS[idx % TREEMAP_SCALE_PEER_COLORS.len()].to_string()
}

fn treemap_label_color(idx: usize, theme: &Theme) -> String {
    if is_forest_treemap_theme(theme) {
        return "black".to_string();
    }
    TREEMAP_SCALE_LABEL_COLORS[idx % TREEMAP_SCALE_LABEL_COLORS.len()].to_string()
}

fn is_forest_treemap_theme(theme: &Theme) -> bool {
    theme.primary_color.eq_ignore_ascii_case("#cde498")
        && theme.secondary_color.eq_ignore_ascii_case("#cdffb2")
}

fn format_treemap_value(value: f32, format_str: &str) -> String {
    if !value.is_finite() {
        return String::new();
    }

    let format_str = match format_str.trim() {
        "" => ",",
        other => other,
    };
    if let Some(rest) = format_str.strip_prefix('$') {
        return format!("${}", format_treemap_number(value, rest));
    }
    format_treemap_number(value, format_str)
}

fn format_treemap_number(value: f32, format_str: &str) -> String {
    if format_str.contains('%') {
        let precision = format_precision(format_str).unwrap_or(0);
        let mut text = format_fixed_number(value * 100.0, Some(precision));
        if format_str.contains(',') {
            text = group_number_text(&text);
        }
        text.push('%');
        return text;
    }

    let mut text = format_fixed_number(value, format_precision(format_str));
    if format_str.contains(',') {
        text = group_number_text(&text);
    }
    text
}

fn format_precision(format_str: &str) -> Option<usize> {
    let dot = format_str.find('.')?;
    let digits = format_str[dot + 1..]
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<usize>().ok()
    }
}

fn format_fixed_number(value: f32, precision: Option<usize>) -> String {
    let negative = value.is_sign_negative();
    let value = value.abs();
    let text = if let Some(precision) = precision {
        format!("{value:.precision$}", precision = precision)
    } else if (value - value.round()).abs() <= TREEMAP_VALUE_EPSILON {
        format!("{:.0}", value.round())
    } else {
        format!("{value:.6}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    };
    if negative && text != "0" {
        format!("-{text}")
    } else {
        text
    }
}

fn group_number_text(text: &str) -> String {
    let (sign, unsigned) = text
        .strip_prefix('-')
        .map_or(("", text), |rest| ("-", rest));
    let decimal = unsigned.find('.');
    let int_end = unsigned.find('.').unwrap_or(unsigned.len());
    let int_part = &unsigned[..int_end];
    let mut grouped = String::new();
    for (idx, ch) in int_part.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let mut grouped = grouped.chars().rev().collect::<String>();
    if let Some(dot_idx) = decimal {
        grouped.push_str(&unsigned[dot_idx..]);
    }
    if sign == "-" && grouped != "0" {
        grouped.insert(0, '-');
    }
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_mermaid;

    fn assert_rect(layout: &Layout, id: &str, x: f32, y: f32, width: f32, height: f32) {
        let node = layout.nodes.get(id).unwrap_or_else(|| panic!("{id} node"));
        assert!(
            (node.x - x).abs() <= 0.01
                && (node.y - y).abs() <= 0.01
                && (node.width - width).abs() <= 0.01
                && (node.height - height).abs() <= 0.01,
            "{id} rect was ({:.2}, {:.2}, {:.2}, {:.2}), expected ({x:.2}, {y:.2}, {width:.2}, {height:.2})",
            node.x,
            node.y,
            node.width,
            node.height
        );
    }

    #[test]
    fn default_treemap_layout_matches_mermaid_d3_squarify_cells() {
        let source = r#"treemap-beta
"Category A"
 "Item A1": 10
 "Item A2": 20
"Category B"
 "Item B1": 15
 "Item B2": 25
"#;
        let parsed = parse_mermaid(source).expect("failed to parse treemap fixture");
        let layout =
            compute_treemap_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        assert_eq!(layout.width, 996.0);
        assert_eq!(layout.height, 371.0);
        assert_rect(&layout, "treemap_3", 8.0, 8.0, 556.0, 355.0);
        assert_rect(&layout, "treemap_0", 574.0, 8.0, 414.0, 355.0);
        assert_rect(&layout, "treemap_5", 18.0, 43.0, 331.0, 310.0);
        assert_rect(&layout, "treemap_4", 359.0, 43.0, 195.0, 310.0);
        assert_rect(&layout, "treemap_2", 584.0, 43.0, 259.0, 310.0);
        assert_rect(&layout, "treemap_1", 853.0, 43.0, 125.0, 310.0);
    }

    #[test]
    fn zero_value_treemap_items_keep_mermaid_degenerate_row_extent() {
        let source = r#"treemap-beta
"Products"
 "Electronics"
 "Phones": 50
 "Computers": 30
 "Accessories": 20
 "Clothing"
 "Men's": 40
 "Women's": 40
"#;
        let parsed = parse_mermaid(source).expect("failed to parse treemap fixture");
        let layout =
            compute_treemap_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        assert_rect(&layout, "treemap_1", 719.0, 358.0, 259.0, 0.0);
        assert_rect(&layout, "treemap_5", 719.0, 358.0, 259.0, 0.0);
    }

    #[test]
    fn treemap_values_use_mermaid_comma_formatting() {
        let source = r#"treemap-beta
"Budget"
 "Salaries": 700000
 "Equipment": 200000
"#;
        let parsed = parse_mermaid(source).expect("failed to parse treemap fixture");
        let layout =
            compute_treemap_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let budget = layout.nodes.get("treemap_0").expect("budget node");
        let salaries = layout.nodes.get("treemap_1").expect("salaries node");

        assert_eq!(
            budget
                .sub_label
                .as_ref()
                .and_then(|label| label.lines.first())
                .map(|line| line.text().into_owned()),
            Some("900,000".to_string())
        );
        assert_eq!(
            salaries
                .sub_label
                .as_ref()
                .and_then(|label| label.lines.first())
                .map(|line| line.text().into_owned()),
            Some("700,000".to_string())
        );
        assert_eq!(format_treemap_value(0.35, ","), "0.35");
        assert_eq!(format_treemap_value(0.35, ".1%"), "35.0%");
        assert_eq!(format_treemap_value(0.35, "$.1%"), "$35.0%");
        assert_eq!(format_treemap_value(1_500_000.0, "$0,0"), "$1,500,000");
    }

    #[test]
    fn treemap_nodes_use_mermaid_section_and_leaf_palette() {
        let source = r#"treemap-beta
"Category A"
 "Item A1": 10
 "Item A2": 20
"Category B"
 "Item B1": 15
 "Item B2": 25
"#;
        let parsed = parse_mermaid(source).expect("failed to parse treemap fixture");
        let layout =
            compute_treemap_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let category_b = layout.nodes.get("treemap_3").expect("Category B");
        assert_eq!(
            category_b.style.fill.as_deref(),
            Some("hsl(240, 100%, 76.2745098039%)")
        );
        assert_eq!(
            category_b.style.stroke.as_deref(),
            Some("hsl(240, 100%, 61.2745098039%)")
        );
        assert_eq!(category_b.style.text_color.as_deref(), Some("#ffffff"));
        assert_eq!(category_b.style.stroke_width, Some(2.0));

        let category_a = layout.nodes.get("treemap_0").expect("Category A");
        assert_eq!(
            category_a.style.fill.as_deref(),
            Some("hsl(60, 100%, 73.5294117647%)")
        );
        assert_eq!(category_a.style.text_color.as_deref(), Some("black"));

        let item_b2 = layout.nodes.get("treemap_5").expect("Item B2");
        assert_eq!(
            item_b2.style.fill.as_deref(),
            Some("hsl(240, 100%, 76.2745098039%)")
        );
        assert_eq!(
            item_b2.style.stroke.as_deref(),
            Some("hsl(240, 100%, 76.2745098039%)")
        );
        assert_eq!(item_b2.style.stroke_width, Some(3.0));
    }

    #[test]
    fn forest_treemap_nodes_use_mermaid_forest_scale_palette() {
        let source = r#"treemap-beta
"Category A"
 "Item A1": 10
 "Item A2": 20
"Category B"
 "Item B1": 15
 "Item B2": 25
"#;
        let parsed = parse_mermaid(source).expect("failed to parse treemap fixture");
        let layout =
            compute_treemap_layout(&parsed.graph, &Theme::forest(), &LayoutConfig::default());

        let category_b = layout.nodes.get("treemap_3").expect("Category B");
        assert_eq!(
            category_b.style.fill.as_deref(),
            Some("hsl(78.1578947368, 58.4615384615%, 64.5098039216%)")
        );
        assert_eq!(
            category_b.style.stroke.as_deref(),
            Some("hsl(78.1578947368, 58.4615384615%, 39.5098039216%)")
        );
        assert_eq!(category_b.style.text_color.as_deref(), Some("black"));

        let category_a = layout.nodes.get("treemap_0").expect("Category A");
        assert_eq!(
            category_a.style.fill.as_deref(),
            Some("hsl(98.961038961, 100%, 74.9019607843%)")
        );
        assert_eq!(
            category_a.style.stroke.as_deref(),
            Some("hsl(98.961038961, 100%, 39.9019607843%)")
        );

        let item_b1 = layout.nodes.get("treemap_4").expect("Item B1");
        assert_eq!(item_b1.style.text_color.as_deref(), Some("black"));
    }
}
