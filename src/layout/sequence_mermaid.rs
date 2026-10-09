//! Port of Mermaid's sequenceRenderer cursor, bounds stack, actor margins and
//! activation endpoints. No generic graph router participates in this layout.
use super::*;
use crate::ir::{
    SequenceActivationKind as Active, SequenceEvent as Event, SequenceFrameKind as Kind,
    SequenceLifecycleKind as Life, SequenceNotePosition as Position,
};
use crate::sequence::{self, Metrics};

#[derive(Clone)]
struct Frame {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    sections: Vec<(f32, usize)>,
}
struct Bounds {
    x1: f32,
    x2: f32,
    y2: f32,
    stack: Vec<Frame>,
    active: Vec<SequenceActivationLayout>,
    margin: f32,
}
impl Bounds {
    fn insert(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        let (x1, x2, y1, y2) = (x1.min(x2), x1.max(x2), y1.min(y2), y1.max(y2));
        self.x1 = self.x1.min(x1);
        self.x2 = self.x2.max(x2);
        self.y2 = self.y2.max(y2);
        let len = self.stack.len();
        for (i, frame) in self.stack.iter_mut().enumerate() {
            let pad = (len - i) as f32 * self.margin;
            frame.x1 = frame.x1.min(x1 - pad);
            frame.x2 = frame.x2.max(x2 + pad);
            frame.y1 = frame.y1.min(y1 - pad);
            frame.y2 = frame.y2.max(y2 + pad);
            self.x1 = self.x1.min(x1 - pad);
            self.x2 = self.x2.max(x2 + pad);
            self.y2 = self.y2.max(y2 + pad);
        }
        // Upstream continues the same counter after visiting frame bounds.
        // Activation rectangles update their vertical extent, never frame X.
        for (i, activation) in self.active.iter_mut().enumerate() {
            activation.y = activation.y.min(y1 + i as f32 * self.margin);
        }
    }
    fn actor_bounds(&self, id: &str, node: &NodeLayout) -> (f32, f32) {
        let center = node.x + node.width / 2.0;
        self.active
            .iter()
            .filter(|a| a.participant == id)
            .fold((center - 1.0, center + 1.0), |(left, right), a| {
                (left.min(a.x), right.max(a.x + a.width))
            })
    }
}

fn note_geometry(
    note: &crate::ir::SequenceNote,
    nodes: &BTreeMap<String, NodeLayout>,
    metrics: &Metrics,
    width: f32,
    margin: f32,
    padding: f32,
) -> Option<(f32, f32, TextBlock)> {
    let first = nodes.get(note.participants.first()?)?;
    let last = nodes.get(note.participants.last()?)?;
    let label = metrics.label(&note.label, Some(width));
    let mut w = width.max(label.width + 2.0 * padding);
    let x = match note.position {
        Position::RightOf => {
            w = (first.width / 2.0 + last.width / 2.0).max(w);
            first.x + (first.width + margin) / 2.0
        }
        Position::LeftOf => {
            w = (first.width / 2.0 + last.width / 2.0).max(w);
            first.x - w + (first.width - margin) / 2.0
        }
        Position::Over if first.id == last.id => {
            w = first.width.max(w);
            first.x + (first.width - w) / 2.0
        }
        Position::Over => {
            let left = first.x + first.width / 2.0;
            let right = last.x + last.width / 2.0;
            w = (left - right).abs() + margin;
            left.min(right) - margin / 2.0
        }
    };
    let label = metrics.label(&note.label, Some(w - 2.0 * padding));
    Some((x, w, label))
}

pub(super) fn compute(graph: &Graph, original: &Theme, _config: &LayoutConfig) -> Layout {
    let options = &graph.appearance_config;
    let theme = sequence::theme(options, original);
    let metrics = Metrics::new(options, &theme);
    let neo = sequence::neo(options);
    let actor_width = sequence::number(options, "width", 150.0);
    let actor_margin = sequence::number(options, "actorMargin", 50.0);
    let margin = sequence::number(options, "boxMargin", 10.0);
    let text_margin = sequence::number(options, "boxTextMargin", 5.0);
    let padding = sequence::number(options, "wrapPadding", 10.0);
    let note_margin = sequence::number(options, "noteMargin", 10.0);
    let activation_width = sequence::number(options, "activationWidth", 10.0);
    let label_height = sequence::number(options, "labelBoxHeight", 20.0);
    let label_width = sequence::number(options, "labelBoxWidth", 50.0);
    let mirror = sequence::flag(options, "mirrorActors", true);
    let mut participants = graph.sequence_participants.clone();
    for id in graph.nodes.keys() {
        if !participants.contains(id) {
            participants.push(id.clone());
        }
    }
    if sequence::flag(options, "hideUnusedParticipants", false) {
        participants.retain(|id| graph.edges.iter().any(|e| &e.from == id || &e.to == id));
    }
    let mut nodes = BTreeMap::new();
    let mut actor_height = sequence::number(options, "height", 65.0);
    for (slot, id) in participants.iter().enumerate() {
        let node = &graph.nodes[id];
        let label = metrics.label(&node.label, Some(actor_width - 2.0 * padding));
        actor_height = actor_height.max(if neo {
            56.0 + label.height
        } else {
            label.height
        });
        let width = actor_width.max(if metrics.wrap {
            0.0
        } else {
            label.width + 2.0 * padding
        });
        let (fill, stroke) = sequence::actor_colors(options, &theme, slot);
        let mut style = resolve_node_style(id, graph);
        style.fill = Some(fill);
        style.stroke = Some(stroke);
        nodes.insert(
            id.clone(),
            NodeLayout {
                id: id.clone(),
                x: 0.0,
                y: 0.0,
                width,
                height: actor_height,
                label,
                shape: node.shape,
                style,
                link: graph.node_links.get(id).cloned(),
                anchor_subgraph: None,
                hidden: false,
                icon: node.icon.clone(),
                img: None,
                img_w: None,
                img_h: None,
                img_pos: None,
                sub_label: None,
                is_treemap_leaf: false,
            },
        );
    }
    let index: HashMap<_, _> = participants
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();
    let mut max_width = vec![0.0f32; participants.len()];
    for edge in &graph.edges {
        let (Some(&from), Some(&to)) = (index.get(edge.from.as_str()), index.get(edge.to.as_str()))
        else {
            continue;
        };
        let w = metrics
            .label(
                edge.label.as_deref().unwrap_or(""),
                Some(actor_width - 2.0 * padding),
            )
            .width
            + 2.0 * padding;
        if from.abs_diff(to) == 1 {
            max_width[from.min(to)] = max_width[from.min(to)].max(w);
        } else if from == to {
            max_width[from] = max_width[from].max(w / 2.0);
        }
    }
    for note in &graph.sequence_notes {
        let (Some(from), Some(to)) = (
            note.participants
                .first()
                .and_then(|id| index.get(id.as_str())),
            note.participants
                .last()
                .and_then(|id| index.get(id.as_str())),
        ) else {
            continue;
        };
        let w = metrics
            .label(&note.label, Some(actor_width - 2.0 * padding))
            .width
            + 2.0 * padding;
        match note.position {
            Position::RightOf => max_width[*from] = max_width[*from].max(w),
            Position::LeftOf if *to > 0 => max_width[*to - 1] = max_width[*to - 1].max(w),
            Position::Over => {
                if *to > 0 {
                    max_width[*to - 1] = max_width[*to - 1].max(w / 2.0);
                }
                if *to + 1 < participants.len() {
                    max_width[*from] = max_width[*from].max(w / 2.0);
                }
            }
            _ => {}
        }
    }
    let has_boxes = !graph.sequence_boxes.is_empty();
    let has_box_titles = graph
        .sequence_boxes
        .iter()
        .any(|b| b.label.as_ref().is_some_and(|s| !s.is_empty()));
    let box_title_height = if has_box_titles {
        graph
            .sequence_boxes
            .iter()
            .filter_map(|b| b.label.as_ref())
            .map(|s| metrics.label(s, None).height)
            .fold(0.0, f32::max)
    } else {
        0.0
    };
    let top = if has_boxes {
        margin + box_title_height
    } else {
        0.0
    };
    let mut cursor = top + actor_height;
    let mut x = 0.0;
    let mut previous_box = None;
    for (i, id) in participants.iter().enumerate() {
        let group = graph
            .sequence_boxes
            .iter()
            .position(|b| b.participants.contains(id));
        if group != previous_box {
            if previous_box.is_some() {
                x += margin + text_margin;
            }
            if group.is_some() {
                x += text_margin;
            }
        }
        if graph
            .sequence_lifecycle
            .iter()
            .any(|l| &l.participant == id && l.kind == Life::Create)
        {
            x += nodes[id].width / 2.0;
        }
        let width = nodes[id].width;
        let next_width = participants
            .get(i + 1)
            .map(|n| nodes[n].width)
            .unwrap_or(0.0);
        let node = nodes.get_mut(id).unwrap();
        node.x = x;
        node.y = top;
        node.height = actor_height;
        x += width + actor_margin.max(max_width[i] + actor_margin - width / 2.0 - next_width / 2.0);
        previous_box = group;
    }
    let mut bounds = Bounds {
        x1: 0.0,
        x2: participants
            .last()
            .map(|id| nodes[id].x + nodes[id].width)
            .unwrap_or(0.0),
        y2: cursor,
        stack: Vec::new(),
        active: Vec::new(),
        margin,
    };
    let mut notes = Vec::new();
    let mut activations = Vec::new();
    let mut frames = Vec::new();
    let mut edges = Vec::new();
    let mut numbers = Vec::new();
    let mut destroy_markers = Vec::new();
    let mut destroyed: HashMap<String, f32> = HashMap::new();
    let note_models: Vec<_> = graph
        .sequence_notes
        .iter()
        .map(|n| note_geometry(n, &nodes, &metrics, actor_width, actor_margin, note_margin))
        .collect();
    // This preliminary pass mirrors calculateLoopBounds: wrap frame headings
    // to their content span, rather than to the unrelated global character cap.
    let mut frame_widths = vec![0.0f32; graph.sequence_frames.len()];
    let mut frame_from = vec![f32::INFINITY; graph.sequence_frames.len()];
    let mut frame_to = vec![f32::NEG_INFINITY; graph.sequence_frames.len()];
    let mut frame_stack = Vec::new();
    let mut preview = Bounds {
        x1: 0.0,
        x2: 0.0,
        y2: 0.0,
        stack: Vec::new(),
        active: Vec::new(),
        margin,
    };
    for (event_index, event) in graph.sequence_events.iter().enumerate() {
        match *event {
            Event::FrameStart(id) => frame_stack.push(id),
            Event::FrameEnd(_) => {
                frame_stack.pop();
            }
            Event::Activation(id) => {
                let a = &graph.sequence_activations[id];
                if let Some(node) = nodes.get(&a.participant) {
                    if a.kind == Active::Activate {
                        let depth = preview
                            .active
                            .iter()
                            .filter(|v| v.participant == a.participant)
                            .count();
                        preview.active.push(SequenceActivationLayout {
                            x: node.x
                                + node.width / 2.0
                                + (depth as f32 - 1.0) * activation_width / 2.0,
                            y: 0.0,
                            width: activation_width,
                            height: 0.0,
                            participant: a.participant.clone(),
                            depth,
                        });
                    } else if let Some(i) = preview
                        .active
                        .iter()
                        .rposition(|v| v.participant == a.participant)
                    {
                        preview.active.remove(i);
                    }
                }
            }
            Event::Message(id) => {
                let e = &graph.edges[id];
                if let (Some(from), Some(to)) = (nodes.get(&e.from), nodes.get(&e.to)) {
                    let (fl, fr) = preview.actor_bounds(&e.from, from);
                    let (tl, tr) = preview.actor_bounds(&e.to, to);
                    let right = fl <= tl;
                    let sign = if right { 1.0 } else { -1.0 };
                    let mut start = if right { fr } else { fl };
                    let mut stop = if right { tl } else { tr };
                    let plain = e.sequence_arrow_end == Some(crate::ir::SequenceArrowHead::None);
                    if neo && !(plain && e.style != crate::ir::EdgeStyle::Dotted) {
                        stop -= 3.0 * sign;
                    }
                    if neo && e.arrow_start {
                        start += 3.0 * sign;
                    }
                    let central_source =
                        e.start_decoration == Some(crate::ir::EdgeDecoration::Circle);
                    let central_target =
                        e.end_decoration == Some(crate::ir::EdgeDecoration::Circle);
                    if central_source {
                        start += 4.0;
                        if e.arrow_start && !right {
                            start -= 6.0;
                        }
                    }
                    let self_message = e.from == e.to;
                    if self_message {
                        stop = start;
                    } else {
                        let activates =
                            graph
                                .sequence_events
                                .get(event_index + 1)
                                .is_some_and(|event| match event {
                                    Event::Activation(a) => {
                                        graph.sequence_activations[*a].kind == Active::Activate
                                            && graph.sequence_activations[*a].participant == e.to
                                    }
                                    _ => false,
                                });
                        if (activates || central_target) && (tr - tl).abs() <= 2.0 {
                            stop -= (activation_width / 2.0 - 1.0) * sign;
                        }
                        if !plain {
                            stop -= 3.0 * sign;
                        }
                        if e.arrow_start {
                            start += 3.0 * sign;
                        }
                    }
                    let label = metrics.label(
                        e.label.as_deref().unwrap_or(""),
                        Some((start - stop).abs().max(actor_width)),
                    );
                    let w = actor_width.max((start - stop).abs() + 2.0 * padding).max(
                        if metrics.wrap {
                            0.0
                        } else {
                            label.width + 2.0 * padding
                        },
                    );
                    for f in &frame_stack {
                        if self_message {
                            frame_from[*f] = frame_from[*f]
                                .min(from.x - w / 2.0)
                                .min(from.x - from.width / 2.0);
                            frame_to[*f] = frame_to[*f]
                                .max(to.x + w / 2.0)
                                .max(to.x + from.width / 2.0);
                            frame_widths[*f] = frame_widths[*f]
                                .max((frame_to[*f] - frame_from[*f]).abs())
                                - label_width;
                        } else {
                            frame_from[*f] = frame_from[*f].min(start);
                            frame_to[*f] = frame_to[*f].max(stop);
                            frame_widths[*f] = frame_widths[*f].max(w) - label_width;
                        }
                    }
                }
            }
            Event::Note(id) => {
                if let Some((x, w, _)) = &note_models[id] {
                    for f in &frame_stack {
                        frame_from[*f] = frame_from[*f].min(*x);
                        frame_to[*f] = frame_to[*f].max(x + w);
                        frame_widths[*f] = frame_widths[*f]
                            .max((frame_to[*f] - frame_from[*f]).abs())
                            - label_width;
                    }
                }
            }
            _ => {}
        }
    }
    let events: Vec<_> = if graph.sequence_events.is_empty() {
        (0..graph.edges.len()).map(Event::Message).collect()
    } else {
        graph.sequence_events.clone()
    };
    for (event_index, event) in events.iter().enumerate() {
        match *event {
            Event::Note(id) => {
                let Some((x, width, label)) = note_models[id].clone() else {
                    continue;
                };
                cursor += margin;
                let y = cursor;
                let height =
                    metrics.painted_line_height * label.lines.len() as f32 + 2.0 * note_margin;
                cursor += height;
                bounds.insert(x, y, x + width, cursor);
                let note = &graph.sequence_notes[id];
                notes.push(SequenceNoteLayout {
                    x,
                    y,
                    width,
                    height,
                    label,
                    position: note.position,
                    participants: note.participants.clone(),
                    index: note.index,
                });
            }
            Event::Activation(id) => {
                let a = &graph.sequence_activations[id];
                let Some(node) = nodes.get(&a.participant) else {
                    continue;
                };
                if a.kind == Active::Activate {
                    let depth = bounds
                        .active
                        .iter()
                        .filter(|v| v.participant == a.participant)
                        .count();
                    bounds.active.push(SequenceActivationLayout {
                        x: node.x
                            + node.width / 2.0
                            + (depth as f32 - 1.0) * activation_width / 2.0,
                        y: cursor + 2.0,
                        width: activation_width,
                        height: 0.0,
                        participant: a.participant.clone(),
                        depth,
                    });
                } else if let Some(i) = bounds
                    .active
                    .iter()
                    .rposition(|v| v.participant == a.participant)
                {
                    let mut a = bounds.active.remove(i);
                    let mut end = cursor;
                    if a.y + 18.0 > end {
                        a.y = end - 6.0;
                        end += 12.0;
                    }
                    a.height = end - a.y;
                    bounds.insert(a.x, end - 10.0, a.x + a.width, end);
                    activations.push(a);
                }
            }
            Event::FrameStart(id) => {
                cursor += margin;
                bounds.stack.push(Frame {
                    x1: f32::INFINITY,
                    x2: f32::NEG_INFINITY,
                    y1: cursor,
                    y2: cursor,
                    sections: Vec::new(),
                });
                let frame = &graph.sequence_frames[id];
                cursor += if frame.kind == Kind::Rect {
                    margin
                } else {
                    let label = metrics.frame_label(
                        frame.sections[0].label.as_deref(),
                        frame_widths[id] - 2.0 * padding,
                    );
                    margin
                        + text_margin
                        + if frame.sections[0].label.is_some() {
                            label.height.max(label_height)
                        } else {
                            0.0
                        }
                };
            }
            Event::FrameSection(id, section) => {
                cursor += margin + text_margin;
                if let Some(frame) = bounds.stack.last_mut() {
                    frame.sections.push((cursor, section));
                }
                let section = &graph.sequence_frames[id].sections[section];
                let label =
                    metrics.frame_label(section.label.as_deref(), frame_widths[id] - 2.0 * padding);
                cursor += margin
                    + if section.label.is_some() {
                        label.height.max(label_height)
                    } else {
                        0.0
                    };
            }
            Event::FrameEnd(id) => {
                let Some(frame) = bounds.stack.pop() else {
                    continue;
                };
                let input = &graph.sequence_frames[id];
                cursor = frame.y2.max(cursor);
                let x = if frame.x1.is_finite() { frame.x1 } else { 0.0 };
                let width = if frame.x2.is_finite() {
                    frame.x2 - x
                } else {
                    actor_width
                };
                let label = SequenceLabel {
                    x: x + label_width / 2.0 + width / 2.0,
                    y: frame.y1 + margin + text_margin,
                    text: metrics.frame_label(
                        input.sections[0].label.as_deref(),
                        frame_widths[id] - 2.0 * padding,
                    ),
                };
                let section_labels = frame
                    .sections
                    .iter()
                    .map(|(y, i)| SequenceLabel {
                        x: x + width / 2.0,
                        y: y + margin + text_margin,
                        text: metrics.frame_label(
                            input.sections[*i].label.as_deref(),
                            frame_widths[id] - 2.0 * padding,
                        ),
                    })
                    .collect();
                frames.push(SequenceFrameLayout {
                    kind: input.kind,
                    x,
                    y: frame.y1,
                    width,
                    height: cursor - frame.y1,
                    label_box: (
                        x,
                        frame.y1,
                        label_width.max(50.0),
                        label_height + if neo { 15.0 } else { 0.0 },
                    ),
                    label,
                    section_labels,
                    dividers: frame.sections.iter().map(|v| v.0).collect(),
                    fill_color: if input.kind == Kind::Rect {
                        input.sections[0].label.clone()
                    } else {
                        None
                    },
                });
            }
            Event::Message(id) => {
                let e = &graph.edges[id];
                let (Some(from), Some(to)) = (nodes.get(&e.from), nodes.get(&e.to)) else {
                    continue;
                };
                let (fl, fr) = bounds.actor_bounds(&e.from, from);
                let (tl, tr) = bounds.actor_bounds(&e.to, to);
                let right = fl <= tl;
                let sign = if right { 1.0 } else { -1.0 };
                let mut start = if right { fr } else { fl };
                let mut stop = if right { tl } else { tr };
                let head = e
                    .sequence_arrow_end
                    .unwrap_or(crate::ir::SequenceArrowHead::Filled);
                let plain = head == crate::ir::SequenceArrowHead::None;
                if neo && !(plain && e.style != crate::ir::EdgeStyle::Dotted) {
                    stop -= 3.0 * sign;
                }
                if neo && e.arrow_start {
                    start += 3.0 * sign;
                }
                let central_source = e.start_decoration == Some(crate::ir::EdgeDecoration::Circle);
                let central_target = e.end_decoration == Some(crate::ir::EdgeDecoration::Circle);
                if central_source {
                    start += 4.0;
                    if e.arrow_start && !right {
                        start -= 6.0;
                    }
                }
                let self_message = e.from == e.to;
                if self_message {
                    stop = start;
                } else {
                    let activates = events
                        .get(event_index + 1)
                        .is_some_and(|event| match event {
                            Event::Activation(a) => {
                                graph.sequence_activations[*a].kind == Active::Activate
                                    && graph.sequence_activations[*a].participant == e.to
                            }
                            _ => false,
                        });
                    if (activates || central_target) && (tr - tl).abs() <= 2.0 {
                        stop -= (activation_width / 2.0 - 1.0) * sign;
                    }
                    if !plain {
                        stop -= 3.0 * sign;
                    }
                    if e.arrow_start {
                        start += 3.0 * sign;
                    }
                }
                let label = metrics.label(
                    e.label.as_deref().unwrap_or(""),
                    Some((start - stop).abs().max(actor_width)),
                );
                let model_start = cursor;
                cursor += 10.0 + metrics.line_height;
                let mut offset = label.height - 10.0;
                let right_angles = sequence::flag(options, "rightAngles", false);
                if !self_message || !right_angles {
                    offset += margin;
                }
                let line_y = cursor + offset;
                if self_message {
                    offset += 30.0;
                    let dx = (label.width / 2.0).max(actor_width / 2.0);
                    bounds.insert(
                        start - dx,
                        cursor - 10.0 + offset,
                        start + dx,
                        cursor + 30.0 + offset,
                    );
                } else {
                    bounds.insert(start, line_y - 10.0, stop, line_y);
                }
                cursor += offset;
                bounds.insert(fl.min(tl), model_start, fr.max(tr), cursor - 10.0);
                for lifecycle in graph.sequence_lifecycle.iter().filter(|l| l.index == id) {
                    if lifecycle.kind == Life::Create && lifecycle.participant == e.to {
                        let node = nodes.get_mut(&e.to).unwrap();
                        node.y = line_y - actor_height / 2.0;
                        stop -= sign * (node.width / 2.0 + 3.0);
                        cursor += actor_height / 2.0;
                        bounds.insert(
                            start,
                            model_start,
                            stop + sign * (node.width + 6.0),
                            cursor + note_margin,
                        );
                    } else if lifecycle.kind == Life::Destroy {
                        let node = &nodes[&lifecycle.participant];
                        let y = line_y - actor_height / 2.0;
                        destroyed.insert(lifecycle.participant.clone(), y);
                        if mirror {
                            let adjust = node.width / 2.0
                                + if lifecycle.participant == e.to {
                                    3.0
                                } else {
                                    0.0
                                };
                            if lifecycle.participant == e.to {
                                stop -= sign * adjust;
                            } else {
                                start += sign * adjust;
                            }
                        } else {
                            destroy_markers.push((node.x + node.width / 2.0, line_y));
                        }
                        cursor += actor_height / 2.0;
                    }
                }
                let points = if self_message {
                    vec![
                        (start, line_y),
                        (start + 60.0, line_y - 10.0),
                        (start + 60.0, line_y + 30.0),
                        (start, line_y + 20.0),
                    ]
                } else {
                    vec![(start, line_y), (stop, line_y)]
                };
                if let Some(number) = graph.sequence_autonumber {
                    numbers.push(SequenceNumberLayout {
                        x: if self_message || right {
                            fl + 1.0
                        } else {
                            fr - 1.0
                        },
                        y: line_y,
                        value: number + id,
                    });
                }
                edges.push(EdgeLayout {
                    from: e.from.clone(),
                    to: e.to.clone(),
                    label: if e.label.is_some() { Some(label) } else { None },
                    label_anchor: Some((
                        ((start + stop) / 2.0).round(),
                        model_start + 10.0 + (padding / 2.0).round() + theme.font_size,
                    )),
                    start_label: None,
                    end_label: None,
                    start_label_anchor: None,
                    end_label_anchor: None,
                    points,
                    directed: e.directed,
                    arrow_start: e.arrow_start,
                    arrow_end: e.arrow_end,
                    arrow_start_kind: e.arrow_start_kind,
                    arrow_end_kind: e.arrow_end_kind,
                    start_decoration: e.start_decoration,
                    end_decoration: e.end_decoration,
                    sequence_arrow_end: e.sequence_arrow_end,
                    sequence_arrow_start: e.sequence_arrow_start,
                    style: e.style,
                    override_style: crate::ir::EdgeStyleOverride::default(),
                    curve: if self_message && !right_angles {
                        Some(crate::ir::CurveType::Basis)
                    } else {
                        None
                    },
                });
            }
        }
        bounds.y2 = bounds.y2.max(cursor);
    }
    for mut activation in std::mem::take(&mut bounds.active) {
        activation.height = (cursor - activation.y).max(1.0);
        activations.push(activation);
    }
    let footer_y = cursor + 2.0 * margin;
    let mut footboxes = Vec::new();
    let mut lifelines = Vec::new();
    for id in &participants {
        let node = &nodes[id];
        let end = destroyed
            .get(id)
            .copied()
            .unwrap_or(if mirror { footer_y } else { cursor });
        lifelines.push(Lifeline {
            id: id.clone(),
            x: node.x + node.width / 2.0,
            y1: node.y + actor_height,
            y2: end,
        });
        if mirror {
            let mut footer = node.clone();
            footer.y = end;
            footboxes.push(footer);
        }
    }
    if mirror {
        cursor = footer_y + actor_height + margin;
    }
    let mut boxes = Vec::new();
    for group in &graph.sequence_boxes {
        let members: Vec<_> = group
            .participants
            .iter()
            .filter_map(|id| nodes.get(id))
            .collect();
        if members.is_empty() {
            continue;
        }
        let x1 =
            members.iter().map(|n| n.x).fold(f32::INFINITY, f32::min) - text_margin - 2.0 * margin;
        let x2 = members
            .iter()
            .map(|n| n.x + n.width)
            .fold(f32::NEG_INFINITY, f32::max)
            + text_margin
            + 2.0 * margin;
        // Bounds cover the raw box; only its painted rectangle is padded.
        bounds.x1 = bounds.x1.min(x1 + 2.0 * margin);
        bounds.x2 = bounds.x2.max(x2 - 2.0 * margin);
        boxes.push(SequenceBoxLayout {
            x: x1,
            y: -margin / 2.0,
            width: x2 - x1,
            height: cursor + 1.5 * margin,
            label: group.label.as_ref().map(|s| metrics.label(s, None)),
            color: group.color.clone(),
        });
    }
    if has_boxes {
        cursor += margin;
    }
    bounds.y2 = bounds.y2.max(cursor);
    let title = options
        .get("title")
        .and_then(|v| v.as_str())
        .is_some_and(|s| !s.is_empty());
    let extra_title = if title { 40.0 } else { 0.0 };
    let mx = sequence::number(options, "diagramMarginX", 50.0);
    let my = sequence::number(options, "diagramMarginY", 10.0);
    let width = (bounds.x2 - bounds.x1 + 2.0 * mx).max(1.0);
    let height = bounds.y2
        + 2.0 * my
        + if mirror {
            -margin + sequence::number(options, "bottomMarginAdj", 1.0)
        } else {
            0.0
        }
        + if neo && !nodes.is_empty() { 30.0 } else { 0.0 }
        + extra_title;
    Layout {
        kind: graph.kind,
        nodes,
        edges,
        subgraphs: Vec::new(),
        width,
        height,
        acc_title: None,
        acc_descr: None,
        diagram: DiagramData::Sequence(SequenceData {
            appearance: Some(options.clone()),
            viewbox: Some((bounds.x1 - mx, -my - extra_title, width, height)),
            lifelines,
            footboxes,
            boxes,
            frames,
            notes,
            activations,
            numbers,
            destroy_markers,
        }),
    }
}
