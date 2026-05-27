use super::*;

// Constants matching JS ishikawaRenderer.ts
const BONE_STUB: f32 = 30.0;
const BONE_BASE: f32 = 60.0;
const BONE_PER_CHILD: f32 = 5.0;
const SPINE_BASE_LENGTH: f32 = 250.0;
const ANGLE_DEG: f32 = 82.0;
const PAIR_START_OFFSET: f32 = 20.0;
const DIAGRAM_PADDING: f32 = 20.0;
const LABEL_BOX_HEIGHT: f32 = 23.0;
const HEAD_LABEL_FONT_SIZE: f32 = 14.0;

#[derive(Debug, Clone)]
struct LabelEntry {
    lines: Vec<String>,
    depth: usize,
    parent_index: isize,
    child_count: usize,
}

#[derive(Debug, Clone, Copy)]
struct BoneInfo {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    child_count: usize,
    children_drawn: usize,
}

#[derive(Debug, Clone, Copy)]
struct IshikawaBounds {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl IshikawaBounds {
    fn new() -> Self {
        Self {
            min_x: f32::MAX,
            min_y: f32::MAX,
            max_x: f32::MIN,
            max_y: f32::MIN,
        }
    }

    fn include_point(&mut self, x: f32, y: f32) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    fn include_line(&mut self, line: &IshikawaLineLayout) {
        self.include_point(line.x1, line.y1);
        self.include_point(line.x2, line.y2);
    }

    fn include_box(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.include_point(x, y);
        self.include_point(x + w, y + h);
    }

    fn include_text(
        &mut self,
        x: f32,
        y: f32,
        anchor: &str,
        lines: &[String],
        font_size: f32,
        font_family: &str,
    ) {
        let width = max_line_width(lines, font_size, font_family);
        let height = multiline_height(lines.len().max(1), font_size);
        let left = text_left(x, anchor, width);
        self.include_box(left, y - height / 2.0, width, height);
    }

    fn pad(self, pad: f32) -> Self {
        Self {
            min_x: self.min_x - pad,
            min_y: self.min_y - pad,
            max_x: self.max_x + pad,
            max_y: self.max_y + pad,
        }
    }
}

pub(super) fn compute_ishikawa_layout(
    graph: &Graph,
    theme: &Theme,
    _config: &LayoutConfig,
) -> Layout {
    let font_size = if theme.font_size > 0.0 {
        theme.font_size
    } else {
        14.0
    };
    let angle = ANGLE_DEG * std::f32::consts::PI / 180.0;
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    let root = match &graph.ishikawa.root {
        Some(r) => r,
        None => return empty_layout(graph),
    };

    let causes: &[crate::ir::IshikawaNode] = &root.children;
    if causes.is_empty() {
        return empty_layout(graph);
    }

    let upper_causes: Vec<&crate::ir::IshikawaNode> = causes
        .iter()
        .enumerate()
        .filter_map(|(i, c)| (i % 2 == 0).then_some(c))
        .collect();
    let lower_causes: Vec<&crate::ir::IshikawaNode> = causes
        .iter()
        .enumerate()
        .filter_map(|(i, c)| (i % 2 == 1).then_some(c))
        .collect();

    let upper_stats = side_stats(&upper_causes);
    let lower_stats = side_stats(&lower_causes);
    let descendant_total = upper_stats.0 + lower_stats.0;

    let mut upper_len = SPINE_BASE_LENGTH;
    let mut lower_len = SPINE_BASE_LENGTH;
    if descendant_total > 0 {
        let pool = SPINE_BASE_LENGTH * 2.0;
        let min_len = SPINE_BASE_LENGTH * 0.3;
        upper_len = (pool * (upper_stats.0 as f32 / descendant_total as f32)).max(min_len);
        lower_len = (pool * (lower_stats.0 as f32 / descendant_total as f32)).max(min_len);
    }

    let min_spacing = font_size * 2.0;
    upper_len = upper_len.max(upper_stats.1 as f32 * min_spacing);
    lower_len = lower_len.max(lower_stats.1 as f32 * min_spacing);

    let spine_y = upper_len.max(SPINE_BASE_LENGTH);
    let font_family = &theme.font_family;

    // JS wraps the head with fontSize-based character count, then sizes the head
    // from the browser text bbox. The visible quadratic bbox only reaches half of
    // the `Q w*2.4` control point, so use `w*1.2` for layout bounds.
    let head_max_chars = 6_usize.max((110.0 / (font_size * 0.6)).floor() as usize);
    let head_lines = wrap_text_lines(&root.text, head_max_chars);
    let head_text_w = max_line_width(&head_lines, HEAD_LABEL_FONT_SIZE, font_family);
    let head_text_h = multiline_height(head_lines.len().max(1), font_size);
    let head_w = 60.0_f32.max(head_text_w + 6.0);
    let head_h = 40.0_f32.max(head_text_h * 2.0 + 40.0);
    let head_half_h = head_h / 2.0;
    let head_q_control = head_w * 2.4;
    let head_visible_max_x = head_w * 1.2;
    // Mermaid measures the head label before the final CSS classes apply in
    // this renderer path, so the checked SVGs place the label at half the
    // minimum head width plus the 3px nudge.
    let head_label_x = head_w / 2.0 + 3.0;

    let head_path = format!(
        "M 0 {} L 0 {} Q {} 0 0 {} Z",
        -head_half_h, head_half_h, head_q_control, -head_half_h,
    );

    let mut branches = Vec::new();
    let mut labels = Vec::new();
    let mut bounds = IshikawaBounds::new();

    bounds.include_box(0.0, spine_y - head_half_h, head_visible_max_x, head_h);
    bounds.include_text(
        head_label_x,
        spine_y,
        "start",
        &head_lines,
        HEAD_LABEL_FONT_SIZE,
        font_family,
    );

    let mut spine_x = -PAIR_START_OFFSET;
    let pair_count = causes.len().div_ceil(2);
    for pair_idx in 0..pair_count {
        let mut pair_left = f32::MAX;

        if let Some(cause) = causes.get(pair_idx * 2) {
            add_branch(
                cause,
                spine_x,
                spine_y,
                -1,
                upper_len,
                font_size,
                font_family,
                cos_a,
                sin_a,
                &mut branches,
                &mut labels,
                &mut bounds,
                &mut pair_left,
            );
        }

        if let Some(cause) = causes.get(pair_idx * 2 + 1) {
            add_branch(
                cause,
                spine_x,
                spine_y,
                1,
                lower_len,
                font_size,
                font_family,
                cos_a,
                sin_a,
                &mut branches,
                &mut labels,
                &mut bounds,
                &mut pair_left,
            );
        }

        if pair_left.is_finite() {
            spine_x = pair_left;
        }
    }

    let spine = IshikawaLineLayout {
        x1: spine_x,
        y1: spine_y,
        x2: 0.0,
        y2: spine_y,
        stroke_width: 2.0,
    };
    bounds.include_line(&spine);

    labels.push(IshikawaLabelLayout {
        text: root.text.clone(),
        lines: head_lines,
        x: head_label_x,
        y: spine_y,
        anchor: "start".to_string(),
        font_weight: "600".to_string(),
        has_box: false,
        box_x: 0.0,
        box_y: 0.0,
        box_w: 0.0,
        box_h: 0.0,
    });

    let padded = bounds.pad(DIAGRAM_PADDING);
    let min_x = padded.min_x;
    let min_y = padded.min_y;
    let width = (padded.max_x - padded.min_x).max(1.0);
    let height = (padded.max_y - padded.min_y).max(1.0);

    let mut nodes = BTreeMap::new();
    nodes.insert(
        "__ishikawa_content".to_string(),
        NodeLayout {
            id: "__ishikawa_content".to_string(),
            x: min_x,
            y: min_y,
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
        diagram: DiagramData::Ishikawa(IshikawaLayout {
            head_path,
            head_x: 0.0,
            head_y: spine_y,
            spine,
            branches,
            labels,
            width,
            height,
        }),
        width,
        height,
    }
}

fn add_branch(
    node: &crate::ir::IshikawaNode,
    start_x: f32,
    start_y: f32,
    direction: i32,
    length: f32,
    font_size: f32,
    font_family: &str,
    cos_a: f32,
    sin_a: f32,
    branches: &mut Vec<IshikawaLineLayout>,
    labels: &mut Vec<IshikawaLabelLayout>,
    bounds: &mut IshikawaBounds,
    pair_left: &mut f32,
) {
    let dir = direction as f32;
    let children = &node.children;
    let line_len = length * if children.is_empty() { 0.2 } else { 1.0 };
    let dx = -cos_a * line_len;
    let dy = sin_a * line_len * dir;
    let end_x = start_x + dx;
    let end_y = start_y + dy;

    let branch = IshikawaLineLayout {
        x1: start_x,
        y1: start_y,
        x2: end_x,
        y2: end_y,
        stroke_width: 2.0,
    };
    bounds.include_line(&branch);
    branches.push(branch);

    let cause_lines = vec![node.text.clone()];
    let cause_w = max_line_width(&cause_lines, font_size, font_family);
    let label_y = end_y + 11.0 * dir;
    *pair_left = pair_left.min(text_left(end_x, "middle", cause_w));

    let box_x = end_x - cause_w / 2.0 - 20.0;
    let box_y = label_y - font_size * 0.800_781_25;
    let box_w = cause_w + 40.0;
    bounds.include_box(box_x, box_y, box_w, LABEL_BOX_HEIGHT);
    bounds.include_text(
        end_x,
        label_y,
        "middle",
        &cause_lines,
        font_size,
        font_family,
    );

    labels.push(IshikawaLabelLayout {
        text: node.text.clone(),
        lines: Vec::new(),
        x: end_x,
        y: label_y,
        anchor: "middle".to_string(),
        font_weight: "normal".to_string(),
        has_box: true,
        box_x,
        box_y,
        box_w,
        box_h: LABEL_BOX_HEIGHT,
    });

    if children.is_empty() {
        return;
    }

    let (entries, y_order) = flatten_tree(children, direction);
    let entry_count = entries.len();
    let mut ys = vec![0.0; entry_count];
    for (slot, entry_idx) in y_order.iter().enumerate() {
        ys[*entry_idx] = start_y + dy * ((slot as f32 + 1.0) / (entry_count as f32 + 1.0));
    }

    let mut root_bone = BoneInfo {
        x0: start_x,
        y0: start_y,
        x1: end_x,
        y1: end_y,
        child_count: children.len(),
        children_drawn: 0,
    };
    let mut bones = vec![None; entry_count];
    let diagonal_x = -cos_a;
    let diagonal_y = sin_a * dir;

    for (i, entry) in entries.iter().enumerate() {
        let y = ys[i];
        let parent = if entry.parent_index < 0 {
            root_bone
        } else {
            bones[entry.parent_index as usize].expect("parent bone must exist")
        };

        let (bx0, by0, bx1) = if entry.depth % 2 == 0 {
            let dy_parent = parent.y1 - parent.y0;
            let t = if dy_parent.abs() > f32::EPSILON {
                (y - parent.y0) / dy_parent
            } else {
                0.5
            };
            let bx0 = lerp(parent.x0, parent.x1, t);
            let bx1 = bx0
                - if entry.child_count > 0 {
                    BONE_BASE + entry.child_count as f32 * BONE_PER_CHILD
                } else {
                    BONE_STUB
                };
            (bx0, y, bx1)
        } else {
            let k = if entry.parent_index < 0 {
                let drawn = root_bone.children_drawn;
                root_bone.children_drawn += 1;
                drawn
            } else {
                let parent_bone = bones[entry.parent_index as usize]
                    .as_mut()
                    .expect("parent bone must exist");
                let drawn = parent_bone.children_drawn;
                parent_bone.children_drawn += 1;
                drawn
            };
            let t = (parent.child_count - k) as f32 / (parent.child_count + 1) as f32;
            let bx0 = lerp(parent.x0, parent.x1, t);
            let by0 = parent.y0;
            let bx1 = bx0 + diagonal_x * ((y - by0) / diagonal_y);
            (bx0, by0, bx1)
        };

        let sub_branch = IshikawaLineLayout {
            x1: bx0,
            y1: by0,
            x2: bx1,
            y2: y,
            stroke_width: 1.0,
        };
        bounds.include_line(&sub_branch);
        branches.push(sub_branch);

        let text_w = max_line_width(&entry.lines, font_size, font_family);
        *pair_left = pair_left.min(text_left(bx1, "end", text_w));
        bounds.include_text(bx1, y, "end", &entry.lines, font_size, font_family);
        labels.push(IshikawaLabelLayout {
            text: entry.lines.join("\n"),
            lines: entry.lines.clone(),
            x: bx1,
            y,
            anchor: "end".to_string(),
            font_weight: "normal".to_string(),
            has_box: false,
            box_x: 0.0,
            box_y: 0.0,
            box_w: 0.0,
            box_h: 0.0,
        });

        if entry.child_count > 0 {
            bones[i] = Some(BoneInfo {
                x0: bx0,
                y0: by0,
                x1: bx1,
                y1: y,
                child_count: entry.child_count,
                children_drawn: 0,
            });
        }
    }
}

fn side_stats(nodes: &[&crate::ir::IshikawaNode]) -> (usize, usize) {
    nodes.iter().fold((0, 0), |(total, max), node| {
        let descendants = count_descendants(node);
        (total + descendants, max.max(descendants))
    })
}

fn count_descendants(node: &crate::ir::IshikawaNode) -> usize {
    node.children
        .iter()
        .map(|child| 1 + count_descendants(child))
        .sum()
}

fn flatten_tree(
    children: &[crate::ir::IshikawaNode],
    direction: i32,
) -> (Vec<LabelEntry>, Vec<usize>) {
    fn walk(
        nodes: &[crate::ir::IshikawaNode],
        parent_index: isize,
        depth: usize,
        direction: i32,
        entries: &mut Vec<LabelEntry>,
        y_order: &mut Vec<usize>,
    ) {
        let indexes: Vec<usize> = if direction < 0 {
            (0..nodes.len()).rev().collect()
        } else {
            (0..nodes.len()).collect()
        };

        for node_index in indexes {
            let child = &nodes[node_index];
            let idx = entries.len();
            entries.push(LabelEntry {
                lines: wrap_text_lines(&child.text, 15),
                depth,
                parent_index,
                child_count: child.children.len(),
            });
            if depth % 2 == 0 {
                y_order.push(idx);
                if !child.children.is_empty() {
                    walk(
                        &child.children,
                        idx as isize,
                        depth + 1,
                        direction,
                        entries,
                        y_order,
                    );
                }
            } else {
                if !child.children.is_empty() {
                    walk(
                        &child.children,
                        idx as isize,
                        depth + 1,
                        direction,
                        entries,
                        y_order,
                    );
                }
                y_order.push(idx);
            }
        }
    }

    let mut entries = Vec::new();
    let mut y_order = Vec::new();
    walk(children, -1, 2, direction, &mut entries, &mut y_order);
    (entries, y_order)
}

fn wrap_text_lines(text: &str, max_chars: usize) -> Vec<String> {
    if text.len() <= max_chars {
        return vec![text.to_string()];
    }

    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        if let Some(last) = lines.last_mut() {
            if last.len() + 1 + word.len() <= max_chars {
                last.push(' ');
                last.push_str(word);
                continue;
            }
        }
        lines.push(word.to_string());
    }

    if lines.is_empty() {
        vec![text.to_string()]
    } else {
        lines
    }
}

fn max_line_width(lines: &[String], font_size: f32, font_family: &str) -> f32 {
    lines
        .iter()
        .map(|line| crate::text_metrics::get_computed_text_length(line, font_size, font_family))
        .fold(0.0_f32, f32::max)
}

fn multiline_height(line_count: usize, font_size: f32) -> f32 {
    if line_count <= 1 {
        font_size
    } else {
        font_size + (line_count as f32 - 1.0) * font_size * 1.05
    }
}

fn text_left(x: f32, anchor: &str, width: f32) -> f32 {
    match anchor {
        "middle" => x - width / 2.0,
        "end" => x - width,
        _ => x,
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn empty_layout(graph: &Graph) -> Layout {
    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        acc_title: None,
        acc_descr: None,
        diagram: DiagramData::Ishikawa(IshikawaLayout {
            head_path: String::new(),
            head_x: 0.0,
            head_y: 0.0,
            spine: IshikawaLineLayout {
                x1: 0.0,
                y1: 0.0,
                x2: 0.0,
                y2: 0.0,
                stroke_width: 0.0,
            },
            branches: Vec::new(),
            labels: Vec::new(),
            width: 100.0,
            height: 50.0,
        }),
        width: 100.0,
        height: 50.0,
    }
}
