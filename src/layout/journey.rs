use super::*;

const JOURNEY_DIAGRAM_MARGIN_X: f32 = 50.0;
const JOURNEY_DIAGRAM_MARGIN_Y: f32 = 10.0;
const JOURNEY_LEFT_MARGIN: f32 = 150.0;
const JOURNEY_TASK_WIDTH: f32 = 150.0;
const JOURNEY_TASK_HEIGHT: f32 = 50.0;
const JOURNEY_TASK_MARGIN: f32 = 50.0;
const JOURNEY_SECTION_Y: f32 = 50.0;
const JOURNEY_TASK_Y: f32 = 110.0;
const JOURNEY_ACTIVITY_Y: f32 = 200.0;
const JOURNEY_SCORE_BASE_Y: f32 = 300.0;
const JOURNEY_SCORE_STEP_Y: f32 = 30.0;
const JOURNEY_TASK_LINE_BOTTOM_Y: f32 = 450.0;
const JOURNEY_TITLE_Y: f32 = 25.0;
const JOURNEY_TITLE_EXTRA_HEIGHT: f32 = 70.0;
const JOURNEY_ACTOR_RADIUS: f32 = 7.0;
const JOURNEY_FACE_RADIUS: f32 = 15.0;
const JOURNEY_ACTOR_COLORS: [&str; 6] = [
    "#8FBC8F", "#7CFC00", "#00FFFF", "#20B2AA", "#B0E0E6", "#FFFFE0",
];
const JOURNEY_SECTION_FILLS: [&str; 7] = [
    "#191970", "#8B008B", "#4B0082", "#2F4F4F", "#800000", "#8B4513", "#00008B",
];

fn parse_journey_task_label(label: &str) -> (String, Vec<String>) {
    let mut lines = split_lines(label);
    if lines.is_empty() {
        return (String::new(), Vec::new());
    }
    let title = lines.remove(0).trim().to_string();
    let mut actors = Vec::new();
    for line in lines {
        for part in line.split(',') {
            let actor = part.trim();
            if !actor.is_empty() {
                actors.push(actor.to_string());
            }
        }
    }
    (title, actors)
}

pub(super) fn compute_journey_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> Layout {
    let mut section_defs: Vec<(String, Vec<String>)> = Vec::new();
    let mut assigned: HashSet<String> = HashSet::new();
    if graph.subgraphs.is_empty() {
        let mut ordered: Vec<String> = graph.nodes.keys().cloned().collect();
        ordered.sort_by_key(|id| graph.node_order.get(id).copied().unwrap_or(usize::MAX));
        section_defs.push((String::new(), ordered));
    } else {
        for sub in &graph.subgraphs {
            let mut nodes = Vec::new();
            for id in &sub.nodes {
                if graph.nodes.contains_key(id) {
                    nodes.push(id.clone());
                    assigned.insert(id.clone());
                }
            }
            section_defs.push((sub.label.clone(), nodes));
        }
        let mut extras: Vec<String> = graph
            .nodes
            .keys()
            .filter(|id| !assigned.contains(*id))
            .cloned()
            .collect();
        if !extras.is_empty() {
            extras.sort_by_key(|id| graph.node_order.get(id).copied().unwrap_or(usize::MAX));
            section_defs.push(("Other".to_string(), extras));
        }
    }

    struct TaskData {
        id: String,
        label: TextBlock,
        score: Option<f32>,
        actors: Vec<String>,
        section_idx: usize,
        order_idx: usize,
    }

    let mut tasks_data: Vec<TaskData> = Vec::new();
    let mut section_ranges: Vec<(usize, usize)> = Vec::new();
    let mut order_idx = 0usize;
    for (section_idx, (_label, nodes)) in section_defs.iter().enumerate() {
        let start_idx = order_idx;
        for node_id in nodes {
            if let Some(node) = graph.nodes.get(node_id) {
                let (title, actors) = parse_journey_task_label(&node.label);
                let title_text = if title.is_empty() {
                    node.label.clone()
                } else {
                    title
                };
                let label = measure_label(&title_text, theme, config);
                tasks_data.push(TaskData {
                    id: node_id.clone(),
                    label,
                    score: node.value,
                    actors,
                    section_idx,
                    order_idx,
                });
                order_idx += 1;
            }
        }
        let end_idx = order_idx.saturating_sub(1);
        section_ranges.push((start_idx, end_idx));
    }

    let mut actor_order: Vec<String> = Vec::new();
    let mut actor_set: HashSet<String> = HashSet::new();
    for task in &tasks_data {
        for actor in &task.actors {
            if actor_set.insert(actor.clone()) {
                actor_order.push(actor.clone());
            }
        }
    }
    actor_order.sort();

    let mut max_actor_label_width = 0.0_f32;
    for actor in &actor_order {
        let label = measure_label(actor, theme, config);
        if label.width > max_actor_label_width && label.width > JOURNEY_LEFT_MARGIN - label.width {
            max_actor_label_width = label.width;
        }
    }
    let left_margin = JOURNEY_LEFT_MARGIN + max_actor_label_width;

    let title_block = graph
        .journey_title
        .as_ref()
        .map(|title| measure_label(title, theme, config));

    let actors = actor_order
        .iter()
        .enumerate()
        .map(|(idx, actor)| JourneyActorLayout {
            name: actor.clone(),
            color: JOURNEY_ACTOR_COLORS[idx % JOURNEY_ACTOR_COLORS.len()].to_string(),
            x: 20.0,
            y: 60.0 + idx as f32 * 20.0,
            radius: JOURNEY_ACTOR_RADIUS,
        })
        .collect::<Vec<_>>();

    let total_tasks = tasks_data.len();

    let mut tasks = Vec::new();
    for task in &tasks_data {
        let x = left_margin + task.order_idx as f32 * (JOURNEY_TASK_WIDTH + JOURNEY_TASK_MARGIN);
        let score = task.score.unwrap_or(0.0);
        let score_y = JOURNEY_SCORE_BASE_Y + (5.0 - score) * JOURNEY_SCORE_STEP_Y;
        let section_color =
            JOURNEY_SECTION_FILLS[task.section_idx % JOURNEY_SECTION_FILLS.len()].to_string();
        tasks.push(JourneyTaskLayout {
            id: task.id.clone(),
            label: task.label.clone(),
            x,
            y: JOURNEY_TASK_Y,
            width: JOURNEY_TASK_WIDTH,
            height: JOURNEY_TASK_HEIGHT,
            score: task.score,
            score_color: section_color,
            score_y,
            actors: task.actors.clone(),
            actor_y: Some(JOURNEY_TASK_Y),
            section_idx: task.section_idx,
        });
    }

    let mut sections = Vec::new();
    for (section_idx, (label, _nodes)) in section_defs.iter().enumerate() {
        let (start_idx, end_idx) = section_ranges.get(section_idx).copied().unwrap_or((0, 0));
        if start_idx > end_idx || total_tasks == 0 {
            continue;
        }
        let x = left_margin + start_idx as f32 * (JOURNEY_TASK_WIDTH + JOURNEY_TASK_MARGIN);
        let span = end_idx.saturating_sub(start_idx) + 1;
        let width = span as f32 * JOURNEY_TASK_WIDTH
            + (span.saturating_sub(1)) as f32 * JOURNEY_DIAGRAM_MARGIN_X;
        let label_block = measure_label(label, theme, config);
        let color = JOURNEY_SECTION_FILLS[section_idx % JOURNEY_SECTION_FILLS.len()].to_string();
        sections.push(JourneySectionLayout {
            label: label_block,
            x,
            y: JOURNEY_SECTION_Y,
            width,
            height: JOURNEY_TASK_HEIGHT,
            color,
        });
    }

    let baseline = if total_tasks > 0 {
        let stop_x = left_margin
            + (total_tasks.saturating_sub(1)) as f32 * (JOURNEY_TASK_WIDTH + JOURNEY_TASK_MARGIN)
            + JOURNEY_DIAGRAM_MARGIN_X
            + JOURNEY_TASK_MARGIN;
        let width = left_margin + stop_x + 2.0 * JOURNEY_DIAGRAM_MARGIN_X;
        Some((left_margin, JOURNEY_ACTIVITY_Y, width - left_margin - 4.0))
    } else {
        None
    };

    let stop_x = if total_tasks > 0 {
        left_margin
            + (total_tasks.saturating_sub(1)) as f32 * (JOURNEY_TASK_WIDTH + JOURNEY_TASK_MARGIN)
            + JOURNEY_DIAGRAM_MARGIN_X
            + JOURNEY_TASK_MARGIN
    } else {
        left_margin
    };
    let width = (left_margin + stop_x + 2.0 * JOURNEY_DIAGRAM_MARGIN_X).max(1.0);
    let content_stop_y = JOURNEY_TASK_LINE_BOTTOM_Y.max(actor_order.len() as f32 * 50.0);
    let mut height = content_stop_y + 2.0 * JOURNEY_DIAGRAM_MARGIN_Y;
    if title_block.is_some() {
        height += JOURNEY_TITLE_EXTRA_HEIGHT;
    }
    height = height.max(1.0);

    let mut nodes = BTreeMap::new();
    nodes.insert(
        "__journey_metrics_content".to_string(),
        NodeLayout {
            id: "__journey_metrics_content".to_string(),
            x: 0.0,
            y: -25.0,
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
        diagram: DiagramData::Journey(JourneyLayout {
            title: title_block,
            title_y: JOURNEY_TITLE_Y,
            actors,
            actor_label_y: 0.0,
            tasks,
            sections,
            baseline,
            score_radius: JOURNEY_FACE_RADIUS,
            actor_radius: JOURNEY_ACTOR_RADIUS,
            actor_gap: 13.0,
            card_gap_y: 0.0,
            width,
            height,
        }),
        width,
        height,
    }
}
