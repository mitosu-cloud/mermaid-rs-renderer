use super::*;

// Constants matching JS ishikawaRenderer.ts
const BONE_STUB: f32 = 30.0;
const BONE_BASE: f32 = 60.0;
const BONE_PER_CHILD: f32 = 5.0;
const SPINE_BASE_LENGTH: f32 = 250.0;
const ANGLE_DEG: f32 = 82.0;
const PAIR_START_OFFSET: f32 = 20.0; // first pair distance from head

pub(super) fn compute_ishikawa_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> Layout {
    let font_size = if theme.font_size > 0.0 {
        theme.font_size
    } else {
        14.0
    };
    let angle = ANGLE_DEG * std::f32::consts::PI / 180.0;
    let cos_a = angle.cos(); // ≈ 0.139
    let sin_a = angle.sin(); // ≈ 0.990

    let root = match &graph.ishikawa.root {
        Some(r) => r,
        None => return empty_layout(graph),
    };

    let causes: &[crate::ir::IshikawaNode] = &root.children;
    if causes.is_empty() {
        return empty_layout(graph);
    }

    // Split causes into upper (even-indexed) and lower (odd-indexed), matching JS
    let upper: Vec<&crate::ir::IshikawaNode> = causes.iter().step_by(2).collect();
    let lower: Vec<&crate::ir::IshikawaNode> = causes.iter().skip(1).step_by(2).collect();

    // Group into pairs: (upper_cause, optional_lower_cause)
    let num_pairs = upper.len();

    // Spine length: JS uses proportional allocation based on descendant counts.
    // For 2 pairs (4 causes): JS spine ≈ 331, pair spacing ≈ 151.
    // For 1 pair (2 causes): JS spine ≈ 170.
    fn count_descendants(node: &crate::ir::IshikawaNode) -> usize {
        1 + node
            .children
            .iter()
            .map(|c| count_descendants(c))
            .sum::<usize>()
    }
    let upper_desc: usize = upper.iter().map(|c| count_descendants(c)).sum();
    let lower_desc: usize = lower.iter().map(|c| count_descendants(c)).sum();
    let total_desc = (upper_desc + lower_desc).max(1);

    // From JS output: 2 pairs → spine≈331, 1 pair → spine≈170.
    // Roughly: spine_length ≈ 20 + num_pairs * 155
    let spine_length = (PAIR_START_OFFSET + num_pairs as f32 * 155.0).max(150.0);
    let pair_spacing = if num_pairs > 1 {
        (spine_length - PAIR_START_OFFSET) / num_pairs as f32
    } else {
        spine_length - PAIR_START_OFFSET
    };

    // Compute head size dynamically from wrapped root text (matching JS).
    // JS: maxChars = max(6, floor(110 / (fontSize * 0.6)))
    let max_chars = 6_usize.max((110.0 / (font_size * 0.6)).floor() as usize);
    let head_text = &root.text;
    // Wrap using same char-count algorithm as JS
    let head_lines: Vec<String> = {
        let mut lines: Vec<String> = Vec::new();
        if head_text.len() <= max_chars {
            lines.push(head_text.clone());
        } else {
            for word in head_text.split_whitespace() {
                if let Some(last) = lines.last_mut() {
                    if last.len() + 1 + word.len() <= max_chars {
                        last.push(' ');
                        last.push_str(word);
                        continue;
                    }
                }
                lines.push(word.to_string());
            }
        }
        lines
    };
    let num_head_lines = head_lines.len().max(1) as f32;
    // Approximate text bbox: width from longest line, height from line count
    let head_text_w = head_lines
        .iter()
        .map(|l| crate::text_metrics::get_computed_text_length(l, font_size, &theme.font_family))
        .fold(0.0_f32, f32::max);
    let head_text_h = num_head_lines * font_size * 1.2;
    // JS: w = max(60, tb.width + 6), h = max(40, tb.height * 2 + 40)
    // The Q curve's max x = Q_extent/2 = w*1.2. Text starts at Q*0.23 = w*0.55.
    // So text must fit in w*1.2 - w*0.55 = w*0.65. Ensure w >= text_w/0.65.
    let head_w = (head_text_w / 0.65).max((head_text_w + 6.0).max(60.0));
    let head_h = (head_text_h * 2.0 + 40.0).max(40.0);
    let head_half_h = head_h / 2.0;
    let head_q_extent = head_w * 2.4;

    // Spine center Y — the vertical midpoint of the diagram
    let spine_y = head_half_h + 260.0; // enough room for branches above

    // Compute branch length: long enough for the diagonal to reach well above/below spine
    let branch_vertical_reach = spine_y - 10.0; // branches reach near top of diagram
    let branch_length = branch_vertical_reach / sin_a; // actual line length

    let mut branches = Vec::new();
    let mut labels = Vec::new();
    let font_family = &theme.font_family;

    // For each pair, draw upper and lower branches from the same spine attachment point
    for pair_idx in 0..num_pairs {
        let attach_x = -(PAIR_START_OFFSET + pair_idx as f32 * pair_spacing);

        // Upper branch (if exists)
        if let Some(cause) = upper.get(pair_idx) {
            let bone_len = BONE_BASE + cause.children.len() as f32 * BONE_PER_CHILD;
            let actual_len = branch_length.min(bone_len.max(branch_vertical_reach / sin_a));
            let dx = -cos_a * actual_len;
            let dy = -sin_a * actual_len; // negative = upward
            let end_x = attach_x + dx;
            let end_y = spine_y + dy;

            // Primary branch line
            branches.push(IshikawaLineLayout {
                x1: attach_x,
                y1: spine_y,
                x2: end_x,
                y2: end_y,
                stroke_width: 2.0,
            });

            // Cause label at branch endpoint
            let label_w =
                crate::text_metrics::get_computed_text_length(&cause.text, font_size, font_family);
            labels.push(IshikawaLabelLayout {
                text: cause.text.clone(),
                lines: Vec::new(),
                x: end_x,
                y: end_y - 12.0,
                anchor: "middle".to_string(),
                font_weight: "normal".to_string(),
                has_box: true,
                box_x: end_x - label_w / 2.0 - 10.0,
                box_y: end_y - 24.0,
                box_w: label_w + 20.0,
                box_h: 23.0,
            });

            // Sub-causes: horizontal bones from points along the diagonal
            let n_sub = cause.children.len();
            for (j, sub) in cause.children.iter().enumerate() {
                let t = (j as f32 + 1.0) / (n_sub as f32 + 1.0);
                let sub_x = attach_x + dx * t;
                let sub_y = spine_y + dy * t;
                let sub_len = if sub.children.is_empty() {
                    BONE_STUB
                } else {
                    BONE_BASE
                };

                branches.push(IshikawaLineLayout {
                    x1: sub_x,
                    y1: sub_y,
                    x2: sub_x - sub_len,
                    y2: sub_y,
                    stroke_width: 1.0,
                });

                labels.push(IshikawaLabelLayout {
                    text: sub.text.clone(),
                    lines: Vec::new(),
                    x: sub_x - sub_len - 4.0,
                    y: sub_y,
                    anchor: "end".to_string(),
                    font_weight: "normal".to_string(),
                    has_box: false,
                    box_x: 0.0,
                    box_y: 0.0,
                    box_w: 0.0,
                    box_h: 0.0,
                });
            }
        }

        // Lower branch (if exists)
        if let Some(cause) = lower.get(pair_idx) {
            let bone_len = BONE_BASE + cause.children.len() as f32 * BONE_PER_CHILD;
            let actual_len = branch_length.min(bone_len.max(branch_vertical_reach / sin_a));
            let dx = -cos_a * actual_len;
            let dy = sin_a * actual_len; // positive = downward
            let end_x = attach_x + dx;
            let end_y = spine_y + dy;

            branches.push(IshikawaLineLayout {
                x1: attach_x,
                y1: spine_y,
                x2: end_x,
                y2: end_y,
                stroke_width: 2.0,
            });

            let label_w =
                crate::text_metrics::get_computed_text_length(&cause.text, font_size, font_family);
            labels.push(IshikawaLabelLayout {
                text: cause.text.clone(),
                lines: Vec::new(),
                x: end_x,
                y: end_y + 12.0,
                anchor: "middle".to_string(),
                font_weight: "normal".to_string(),
                has_box: true,
                box_x: end_x - label_w / 2.0 - 10.0,
                box_y: end_y + 2.0,
                box_w: label_w + 20.0,
                box_h: 23.0,
            });

            // Sub-causes: horizontal bones along the lower diagonal
            let n_sub = cause.children.len();
            for (j, sub) in cause.children.iter().enumerate() {
                let t = (j as f32 + 1.0) / (n_sub as f32 + 1.0);
                let sub_x = attach_x + dx * t;
                let sub_y = spine_y + dy * t;
                let sub_len = if sub.children.is_empty() {
                    BONE_STUB
                } else {
                    BONE_BASE
                };

                branches.push(IshikawaLineLayout {
                    x1: sub_x,
                    y1: sub_y,
                    x2: sub_x - sub_len,
                    y2: sub_y,
                    stroke_width: 1.0,
                });

                labels.push(IshikawaLabelLayout {
                    text: sub.text.clone(),
                    lines: Vec::new(),
                    x: sub_x - sub_len - 4.0,
                    y: sub_y,
                    anchor: "end".to_string(),
                    font_weight: "normal".to_string(),
                    has_box: false,
                    box_x: 0.0,
                    box_y: 0.0,
                    box_w: 0.0,
                    box_h: 0.0,
                });
            }
        }
    }

    // Head label: center text within the logical head width.
    // JS: tx = (w - tb.width) / 2 - tb.x + 3
    // tb.x ≈ 0 for text-anchor start, so tx ≈ (w - text_w) / 2 + 3
    let head_label_x = (head_w - head_text_w) / 2.0 + 3.0;
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

    // Fish head path: local coordinates centered at y=0 (spine center).
    // The renderer wraps this in a <g transform="translate(0, spine_y)">.
    let head_path = format!(
        "M 0 {} L 0 {} Q {} 0 0 {} Z",
        -head_half_h, head_half_h, head_q_extent, -head_half_h,
    );

    // Spine line
    let spine = IshikawaLineLayout {
        x1: -spine_length,
        y1: spine_y,
        x2: 0.0,
        y2: spine_y,
        stroke_width: 2.0,
    };

    // Compute bounding box
    let mut min_x: f32 = -spine_length;
    let mut max_x: f32 = head_q_extent;
    let mut min_y: f32 = spine_y - head_half_h;
    let mut max_y: f32 = spine_y + head_half_h;

    for b in &branches {
        min_x = min_x.min(b.x1).min(b.x2);
        max_x = max_x.max(b.x1).max(b.x2);
        min_y = min_y.min(b.y1).min(b.y2);
        max_y = max_y.max(b.y1).max(b.y2);
    }
    for l in &labels {
        if l.anchor == "end" {
            let tw = crate::text_metrics::get_computed_text_length(&l.text, font_size, font_family);
            min_x = min_x.min(l.x - tw);
        }
        if l.has_box {
            min_x = min_x.min(l.box_x);
            min_y = min_y.min(l.box_y);
            max_y = max_y.max(l.box_y + l.box_h);
        }
        min_y = min_y.min(l.y - font_size);
        max_y = max_y.max(l.y + font_size);
    }

    let pad = 20.0;
    min_x -= pad;
    min_y -= pad;
    max_x += pad;
    max_y += pad;
    let width = max_x - min_x;
    let height = max_y - min_y;

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
