use super::*;

type Rect = (f32, f32, f32, f32);

const SEQUENCE_LABEL_PAD_X: f32 = 3.0;
const SEQUENCE_LABEL_PAD_Y: f32 = 2.0;
const SEQUENCE_ENDPOINT_LABEL_PAD_X: f32 = 2.5;
const SEQUENCE_ENDPOINT_LABEL_PAD_Y: f32 = 1.5;
const SEQUENCE_LABEL_TOUCH_EPS: f32 = 0.5;
const SEQUENCE_ENDPOINT_LABEL_GAP_TARGET: f32 = 2.5;
const SEQUENCE_ENDPOINT_LABEL_GAP_MIN: f32 = 1.0;
const SEQUENCE_ENDPOINT_LABEL_GAP_MAX: f32 = 6.0;
const SEQUENCE_ENDPOINT_LABEL_FAR_GAP: f32 = 10.0;
const SEQUENCE_CENTER_LABEL_TANGENT_LINEAR_WEIGHT: f32 = 0.22;
const SEQUENCE_CENTER_LABEL_TANGENT_QUAD_WEIGHT: f32 = 0.95;
const SEQUENCE_CENTER_LABEL_TANGENT_SOFT_LIMIT: f32 = 1.2;
const SEQUENCE_CENTER_LABEL_TANGENT_FAR_WEIGHT: f32 = 3.2;

#[derive(Clone, Copy)]
enum SequenceLabelPlacementMode {
    Center,
    Endpoint,
}

pub(super) fn compute_sequence_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> Layout {
    let mut nodes = BTreeMap::new();
    let mut edges = Vec::new();
    let subgraphs = Vec::new();

    let mut participants = graph.sequence_participants.clone();
    for id in graph.nodes.keys() {
        if !participants.contains(id) {
            participants.push(id.clone());
        }
    }

    let mut label_blocks: HashMap<String, TextBlock> = HashMap::new();
    let mut max_label_height: f32 = 0.0;
    let min_actor_width = (theme.font_size * 9.0).max(150.0);
    let mut participant_widths: HashMap<String, f32> = HashMap::new();
    let mut width_total = 0.0f32;
    // Sequence actor labels honor explicit <br/> line breaks but should not
    // be auto-wrapped by the global character cap; upstream mermaid sizes
    // each actor box to fit the user-provided lines on their own.
    let measure_actor_label =
        |text: &str| -> TextBlock { super::text::measure_label_no_wrap(text, theme, config) };
    for id in &participants {
        let node = graph.nodes.get(id).expect("participant missing");
        let label = measure_actor_label(&node.label);
        max_label_height = max_label_height.max(label.height);
        // Same 0.855 scaling as messages — our char-table over-measures vs JS
        // canvas measureText. Only matters for actor labels longer than ~14
        // chars where the box would otherwise need to widen past the 150
        // minimum (e.g. "BookingService" in break-statement). Round to
        // integer to mirror JS `Math.round(bBox.width)` (mermaid utils.ts:731).
        let scaled_label_w = (label.width * 0.855).round();
        let width = (scaled_label_w + theme.font_size * 2.5).max(min_actor_width);
        participant_widths.insert(id.clone(), width);
        width_total += width;
        label_blocks.insert(id.clone(), label);
    }

    let participant_count = participants.len();
    // Match upstream mermaid: 65px actor box height for 1- or 2-line labels,
    // expanding only when labels actually need more vertical room.
    // Stick-figure actors render the label BELOW the figure so they need
    // additional vertical room to keep the label inside the actor envelope
    // (otherwise the lifeline starts inside the label text).
    let has_stick_actor = participants.iter().any(|id| {
        graph.nodes.get(id).is_some_and(|n| {
            matches!(
                n.shape,
                crate::ir::NodeShape::StickFigure
                    | crate::ir::NodeShape::Boundary
                    | crate::ir::NodeShape::Control
                    | crate::ir::NodeShape::Entity
                    | crate::ir::NodeShape::Cylinder
            )
        })
    });
    let stick_extra = if has_stick_actor { 16.0 } else { 0.0 };
    let actor_height = (max_label_height + theme.font_size * 1.0).max(65.0) + stick_extra;
    let avg_actor_width = if participant_count > 0 {
        width_total / participant_count as f32
    } else {
        min_actor_width
    };
    // Mermaid.js sequence-renderer constants (sequenceRenderer.ts:1631-1639,
    // schemas/config.schema.yaml: actorMargin=50, wrapPadding=10).
    // Per-pair gap = max(messageWidth + actorMargin - actor.w/2 - next.w/2,
    //                    actorMargin)
    // where messageWidth = label_width + 2*wrapPadding.
    // Only ADJACENT-actor messages contribute; multi-span messages overflow
    // visually (matches mermaid.js behavior).
    const ACTOR_MARGIN: f32 = 50.0;
    const WRAP_PADDING: f32 = 10.0;
    let _ = avg_actor_width; // formula is per-pair, average no longer needed

    let actor_index: HashMap<&str, usize> = participants
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();
    let actor_widths: Vec<f32> = participants
        .iter()
        .map(|id| {
            participant_widths
                .get(id)
                .copied()
                .unwrap_or(min_actor_width)
        })
        .collect();
    let mut gap_widths: Vec<f32> = vec![ACTOR_MARGIN; participant_count.saturating_sub(1)];
    for edge in &graph.edges {
        let Some(&from_idx) = actor_index.get(edge.from.as_str()) else {
            continue;
        };
        let Some(&to_idx) = actor_index.get(edge.to.as_str()) else {
            continue;
        };
        if from_idx == to_idx {
            continue;
        }
        let lo = from_idx.min(to_idx);
        let hi = from_idx.max(to_idx);
        // Only adjacent-pair messages widen a gap (mermaid.js convention).
        if hi - lo != 1 {
            continue;
        }
        let mut max_label_w = 0.0f32;
        let mut has_html_entity = false;
        for label in [
            edge.label.as_ref(),
            edge.start_label.as_ref(),
            edge.end_label.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            let measured = super::text::measure_label_no_wrap(label, theme, config);
            max_label_w = max_label_w.max(measured.width);
            // HTML entity refs like #9829; render in JS as wider than our
            // table predicts (entities expand to symbols ♥/∞ but JS renders
            // the literal `#NNNN;` text). Our raw width happens to match JS
            // for these; scaling shrinks too aggressively. Skip scaling when
            // an entity ref is present.
            if label.contains('#') && label.contains(';') {
                has_html_entity = true;
            }
        }
        if max_label_w <= 0.0 {
            continue;
        }
        // Our char-table width estimate for message labels runs ~15% wider
        // than mermaid-cli's canvas measureText for the default trebuchet stack.
        // Scale only for gap sizing (not for rendering) so our gaps match JS
        // layout while labels continue to render at the measured visual width.
        // Round to integer to mirror JS `Math.round(bBox.width)` in
        // mermaid utils.ts:731 (calculateTextDimensions).
        const MESSAGE_GAP_MEASURE_SCALE: f32 = 0.855;
        let scaled_label_w = if has_html_entity {
            max_label_w.round()
        } else {
            (max_label_w * MESSAGE_GAP_MEASURE_SCALE).round()
        };
        let message_w = scaled_label_w + 2.0 * WRAP_PADDING;
        let lo_w = actor_widths[lo];
        let hi_w = actor_widths[hi];
        let required = message_w + ACTOR_MARGIN - lo_w / 2.0 - hi_w / 2.0;
        let required = required.max(ACTOR_MARGIN);
        if let Some(slot) = gap_widths.get_mut(lo) {
            if *slot < required {
                *slot = required;
            }
        }
    }
    // Box-transition padding: JS adds extra padding to actor gaps when
    // crossing box boundaries (sequenceRenderer.ts:752-766):
    //   - prev in box A, next in different box B: +boxMargin(10) + 2*boxTextMargin(5) = +20
    //   - prev in box, next not in box: +boxMargin(10) + boxTextMargin(5) = +15
    //   - prev not in box, next in box: +boxTextMargin(5) = +5
    if !graph.sequence_boxes.is_empty() {
        let box_of: HashMap<&str, usize> = graph
            .sequence_boxes
            .iter()
            .enumerate()
            .flat_map(|(i, b)| b.participants.iter().map(move |p| (p.as_str(), i)))
            .collect();
        for i in 0..gap_widths.len() {
            let prev = box_of.get(participants[i].as_str()).copied();
            let next = box_of.get(participants[i + 1].as_str()).copied();
            let extra = match (prev, next) {
                (Some(a), Some(b)) if a != b => 20.0,
                (Some(_), None) => 15.0,
                (None, Some(_)) => 5.0,
                _ => 0.0,
            };
            gap_widths[i] += extra;
        }
    }
    let any_gap_widened = gap_widths.iter().any(|g| *g > ACTOR_MARGIN + 0.5);

    // Add consistent margins to center the diagram
    let margin = 8.0;
    // Mermaid.js sequenceRenderer.ts L1070-1075: when `box` groupings have
    // titles, the cursor is bumped by boxMargin + boxTextMaxHeight BEFORE
    // top actors are drawn — top actors and everything below shift down,
    // making room for the box title text above. Same convention here.
    // When sequence boxes have titles, mermaid.js reserves boxMargin +
    // boxTextMaxHeight (~10 + 14 + 8 ≈ 32px) above the actor row so the
    // box title centers in that gap above the top actor boxes. Without
    // enough space the title text overlaps the actor rectangles.
    let actor_y_offset = if graph.sequence_boxes.iter().any(|b| b.label.is_some()) {
        theme.font_size + 16.0
    } else {
        0.0
    };
    let actor_top_y = margin + actor_y_offset;
    let mut cursor_x = margin;
    // Mermaid.js widens the gap before a CREATED actor by `actor.width / 2`
    // (sequenceRenderer.ts addActorRenderingData: `prevMargin += actor.width/2`
    // when createdActors.get(actor.name)). This leaves room for the new
    // actor's box, which is centered on the create-message's line.
    let created_set: std::collections::HashSet<&str> = graph
        .sequence_lifecycle
        .iter()
        .filter(|e| matches!(e.kind, crate::ir::SequenceLifecycleKind::Create))
        .map(|e| e.participant.as_str())
        .collect();
    for (idx, id) in participants.iter().enumerate() {
        let node = graph.nodes.get(id).expect("participant missing");
        let actor_width = participant_widths
            .get(id)
            .copied()
            .unwrap_or(min_actor_width);
        if idx > 0 && created_set.contains(id.as_str()) {
            cursor_x += actor_width / 2.0;
        }
        let label = label_blocks.get(id).cloned().unwrap_or_else(|| TextBlock {
            lines: vec![TextLine::plain(id.clone())],
            width: 0.0,
            height: 0.0,
        });
        nodes.insert(
            id.clone(),
            NodeLayout {
                id: id.clone(),
                x: cursor_x,
                y: actor_top_y,
                width: actor_width,
                height: actor_height,
                label,
                shape: node.shape,
                style: resolve_node_style(id.as_str(), graph),
                link: graph.node_links.get(id).cloned(),
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
        cursor_x += actor_width;
        if let Some(gap) = gap_widths.get(idx) {
            cursor_x += *gap;
        }
    }

    // Matches mermaid.js: per-message vertical advance ≈ messageMargin (35)
    // + boxMargin (10) - small constant ≈ 44px for 1-line labels with the
    // default 16px font. Computed empirically against the basic-sequence
    // diagram (line ys 109, 153, 197 = 44px apart).
    let base_spacing = (theme.font_size * 2.75).max(35.0);
    let message_row_spacing: Vec<f32> = graph
        .edges
        .iter()
        .map(|edge| {
            let mut row_h = 0.0f32;
            // Sequence message labels honor only explicit <br/> — never auto-
            // wrap (actor spacing is sized to fit each label on one line).
            // Using `measure_label` with wrap=true here over-estimates the
            // row height for long single-line labels and inflates the gap
            // between consecutive messages.
            if let Some(label) = &edge.label {
                row_h = row_h.max(super::text::measure_label_no_wrap(label, theme, config).height);
            }
            if let Some(label) = &edge.start_label {
                row_h = row_h.max(super::text::measure_label_no_wrap(label, theme, config).height);
            }
            if let Some(label) = &edge.end_label {
                row_h = row_h.max(super::text::measure_label_no_wrap(label, theme, config).height);
            }
            let base = base_spacing.max(row_h + 20.0);
            // Self-messages need extra room for the loopback path. Mermaid.js
            // adds ~30px (sequenceRenderer.ts boundMessage: `totalOffset += 30`
            // when startx === stopx).
            if edge.from == edge.to {
                base + 30.0
            } else {
                base
            }
        })
        .collect();
    let note_gap_y = (theme.font_size * 0.55).max(5.0);
    let note_gap_x = (theme.font_size * 0.65).max(7.0);
    let note_padding_x = (theme.font_size * 0.75).max(7.0);
    // JS notes use noteMargin=8 vertical padding. For default font_size=16,
    // 0.46875 yields 7.5px padding (= 15 total) so a 1-line note renders at
    // 39px height matching JS exactly.
    let note_padding_y = (theme.font_size * 0.46875).max(4.0);
    let mut extra_before = vec![0.0; graph.edges.len()];
    let frame_end_pad = base_spacing * 0.25;
    // Padding to add AFTER the last message when an outermost frame extends
    // through the end of the message list. Matches mermaid.js's drawLoop
    // bottom region: the frame box continues past the last message before
    // the bottom actor row begins. ~box_margin (10) per outermost frame.
    let mut frame_tail_pad = 0.0f32;
    for frame in &graph.sequence_frames {
        // Rect frames (background-highlighting `rect rgb(...)`) have no title
        // text, so JS's adjustLoopHeightForWrap uses (boxMargin, boxMargin)
        // for them — only ~20 extra. Loop/critical/etc. titled frames get
        // the full base_spacing (44).
        let frame_start_extra = match frame.kind {
            crate::ir::SequenceFrameKind::Rect => 32.0,
            // Break frames wrap their title label (`break <long condition>`)
            // to fit the actor span; mermaid.js's adjustLoopHeightForWrap
            // then allocates boxMargin + (boxMargin + textMargin + 2*lineHeight)
            // ≈ 61 for a 2-line title. Our default 44 under-counts by ~17.
            crate::ir::SequenceFrameKind::Break => 58.0,
            _ => base_spacing,
        };
        if frame.start_idx < extra_before.len() {
            extra_before[frame.start_idx] += frame_start_extra;
        }
        for section in frame.sections.iter().skip(1) {
            if section.start_idx < extra_before.len() {
                extra_before[section.start_idx] += base_spacing;
            }
        }
        if frame.end_idx < extra_before.len() {
            // Par/Rect/Alt frames need an extra +1 vs base frame_end_pad to
            // match JS's bumpVerticalPos behavior at frame close. Break/Opt
            // already match JS at the base value.
            // For nested Rect frames (background-highlighting case), skip
            // the +1 bonus on the INNER frame — JS's bumpVerticalPos doesn't
            // double-count nested rect closes.
            let is_nested_inner = graph.sequence_frames.iter().any(|other| {
                !std::ptr::eq(other, frame)
                    && other.start_idx < frame.start_idx
                    && other.end_idx >= frame.end_idx
            });
            let extra = match frame.kind {
                crate::ir::SequenceFrameKind::Par | crate::ir::SequenceFrameKind::Alt => 1.0,
                crate::ir::SequenceFrameKind::Rect => {
                    if is_nested_inner {
                        0.0
                    } else {
                        1.0
                    }
                }
                _ => 0.0,
            };
            extra_before[frame.end_idx] += frame_end_pad + extra;
        } else {
            // Critical frames have wider per-section vertical extent than
            // loop/par due to wrapped section labels (boxTextMargin + label
            // height padding accumulates per section in mermaid.js).
            // Empirical fit: critical needs 14 + (sections-1)*24 vs ~10 for
            // other outer frames.
            if matches!(frame.kind, crate::ir::SequenceFrameKind::Critical) {
                let sections = frame.sections.len().max(1) as f32;
                frame_tail_pad += 25.0 + (sections - 1.0) * 24.0;
            } else {
                // JS bumps the cursor by ~boxMargin+1 after a frame ends; we
                // were short by 1px on loops/parallel-flows/opt-bearing diagrams.
                // Nested frames (contained by another frame) get an extra +1
                // per nesting level — JS's bumpVerticalPos accumulates per depth.
                let is_nested = graph.sequence_frames.iter().any(|other| {
                    !std::ptr::eq(other, frame)
                        && other.start_idx < frame.start_idx
                        && other.end_idx >= frame.end_idx
                });
                frame_tail_pad += if is_nested { 13.0 } else { 11.0 };
            }
        }
    }
    // Lifecycle events (`create X` / `destroy X`) attach to a message index;
    // mermaid.js bumps the cursor by `actor.height/2` AFTER processing that
    // message (sequenceRenderer.ts adjustCreatedDestroyedData). This pushes
    // the next message ~32px further down to leave room for the new actor's
    // box (which is centered on the create-msg's y), or for the destroyed
    // actor's bottom box (which sits at the destroy-msg's y).
    let mut lifecycle_extra_after = vec![0.0; graph.edges.len()];
    for event in &graph.sequence_lifecycle {
        if event.index < lifecycle_extra_after.len() {
            lifecycle_extra_after[event.index] += actor_height / 2.0;
        }
    }

    let mut notes_by_index = vec![Vec::new(); graph.edges.len().saturating_add(1)];
    for note in &graph.sequence_notes {
        let idx = note.index.min(graph.edges.len());
        notes_by_index[idx].push(note);
    }

    // Cursor starts right below the actor box. The first message gets
    // base_spacing (44) of clearance; the first note before any message gets
    // a much smaller note_gap_y (~9). Mermaid.js places a note immediately
    // below the actor (~10px gap) but a message at +44px.
    //
    // When a note follows a message, mermaid.js places the note ~boxMargin
    // (10) below the message line — NOT below the full message_row_spacing.
    // We collapse the cursor for the first note in a bucket whose preceding
    // item was a message, so the note tucks against the message line instead
    // of opening a fresh row below it.
    let box_margin = 10.0;
    let mut message_cursor = actor_top_y + actor_height;
    let mut applied_initial_message_offset = false;
    let mut message_ys = Vec::new();
    let mut sequence_notes = Vec::new();
    let mut last_message_y: Option<f32> = None;
    // True if the previous message reserved its own intrinsic row of vertical
    // space (e.g. self-messages need +30px for the loopback curve below the
    // message line). When true, do not collapse the next note onto it.
    let mut prev_msg_needs_full_row = false;
    let mut last_note_bottom_for_msg_gap: Option<f32> = None;
    for idx in 0..=graph.edges.len() {
        if let Some(bucket) = notes_by_index.get(idx) {
            for (note_pos_in_bucket, note) in bucket.iter().enumerate() {
                if note_pos_in_bucket == 0 {
                    if let Some(prev_y) = last_message_y {
                        if prev_msg_needs_full_row {
                            // JS positions a note after a self-msg-in-loop at:
                            //   line + 30 (self insert extension below line)
                            //   + nesting*boxMargin (loop close pump, ~10/level)
                            //   + boxMargin (drawNote leading gap)  =  ~80 from line.
                            // Our row_spacing[self] (base+30=74) already advanced
                            // cursor past line+30, so we only need a small leading
                            // gap. note_gap_y(~8.8) overshoots by ~2px; using a
                            // slightly tighter value closes autonumber's residual
                            // +1.80 height gap (the only self-msg fixture in our
                            // sequenceDiagram-* set).
                            message_cursor += note_gap_y - 1.8;
                        } else {
                            let target = prev_y + box_margin;
                            if target < message_cursor {
                                message_cursor = target;
                            }
                        }
                    } else {
                        // First note before any message: JS uses boxMargin (10)
                        // when the note is NOT inside a frame, but our smaller
                        // note_gap_y (~9) when it is (matches background-
                        // highlighting where the note sits inside a rect frame).
                        let in_frame = graph
                            .sequence_frames
                            .iter()
                            .any(|frame| frame.start_idx <= idx && idx < frame.end_idx);
                        let gap = if in_frame { note_gap_y } else { box_margin };
                        message_cursor += gap;
                    }
                } else {
                    message_cursor += note_gap_y;
                }
                let label = measure_label(&note.label, theme, config);
                // Mermaid.js notes use `conf.width` (default 150) as the
                // minimum width; label only widens the note past that. See
                // sequenceRenderer.ts: `rect.width = noteModel.width || conf.width`.
                let mut width = (label.width + note_padding_x * 2.0).max(150.0);
                let height = label.height + note_padding_y * 2.0;
                let mut lifeline_xs = note
                    .participants
                    .iter()
                    .filter_map(|id| nodes.get(id))
                    .map(|node| node.x + node.width / 2.0)
                    .collect::<Vec<_>>();
                if lifeline_xs.is_empty() {
                    lifeline_xs.push(0.0);
                }
                let base_x = lifeline_xs[0];
                let min_x = lifeline_xs.iter().copied().fold(f32::INFINITY, f32::min);
                let max_x = lifeline_xs
                    .iter()
                    .copied()
                    .fold(f32::NEG_INFINITY, f32::max);
                if note.position == crate::ir::SequenceNotePosition::Over
                    && note.participants.len() > 1
                {
                    let span = (max_x - min_x).abs();
                    // Mermaid.js sequence config noteMargin = 25 — used as the
                    // horizontal padding between an Over-spanning note's edges
                    // and the outer participants' lifelines. Scale with font:
                    // 1.5625 * 16 = 25 for the default font size.
                    let note_span_pad_x = (theme.font_size * 1.5625).max(16.0);
                    width = width.max(span + note_span_pad_x * 2.0);
                }
                // Mermaid.js positions LeftOf/RightOf notes by `(actor.width
                // + actorMargin) / 2` from the actor's anchor (sequenceRenderer.ts
                // L1702/L1710). Since our `base_x` is actor center, this becomes
                // `actorMargin / 2` from center. We use ACTOR_MARGIN (50) as the
                // baseline; falls back to the smaller `note_gap_x` when actor
                // margin isn't applicable (no actor lookup).
                let actor_w = nodes
                    .get(&note.participants[0])
                    .map(|n| n.width)
                    .unwrap_or(150.0);
                let side_offset = ((actor_w + ACTOR_MARGIN) / 2.0 - actor_w / 2.0).max(note_gap_x);
                let x = match note.position {
                    crate::ir::SequenceNotePosition::LeftOf => base_x - side_offset - width,
                    crate::ir::SequenceNotePosition::RightOf => base_x + side_offset,
                    crate::ir::SequenceNotePosition::Over => (min_x + max_x) / 2.0 - width / 2.0,
                };
                let y = message_cursor;
                sequence_notes.push(SequenceNoteLayout {
                    x,
                    y,
                    width,
                    height,
                    label,
                    position: note.position,
                    participants: note.participants.clone(),
                    index: note.index,
                });
                message_cursor += height + note_gap_y;
                last_note_bottom_for_msg_gap = Some(y + height);
            }
        }
        if idx < graph.edges.len() {
            // If a note immediately precedes a NON-first msg (i.e., a msg
            // already placed earlier), ensure base_spacing of room past the
            // note bottom. Skipped for first-msg case since that's handled
            // by target_first_message_y.
            let note_floor_bumped = if last_message_y.is_some() {
                if let Some(note_bot) = last_note_bottom_for_msg_gap.take() {
                    let target = note_bot + base_spacing;
                    if message_cursor < target {
                        message_cursor = target;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                last_note_bottom_for_msg_gap = None;
                false
            };
            // Add base_spacing offset for the first message only if we
            // haven't already advanced past the actor (e.g. via notes).
            if !applied_initial_message_offset {
                applied_initial_message_offset = true;
                // For multi-line message labels, mermaid.js bumps the cursor
                // by lineHeight per line BEFORE drawing the line. Use the
                // first message's row spacing as a floor for the initial
                // offset so multi-line labels get extra vertical room.
                let first_msg_offset = base_spacing.max(message_row_spacing[idx] - 12.0);
                let target_first_message_y = actor_top_y + actor_height + first_msg_offset;
                if message_cursor < target_first_message_y {
                    message_cursor = target_first_message_y;
                }
            }
            // If we already bumped the cursor past a preceding note's bottom
            // by base_spacing, skip the frame_end_pad portion of extra_before
            // to avoid double-adding (both represent frame-close padding).
            let pre_extra = if note_floor_bumped {
                (extra_before[idx] - frame_end_pad).max(0.0)
            } else {
                extra_before[idx]
            };
            message_cursor += pre_extra;
            message_ys.push(message_cursor);
            last_message_y = Some(message_cursor);
            prev_msg_needs_full_row = graph.edges[idx].from == graph.edges[idx].to;
            message_cursor += message_row_spacing[idx] + lifecycle_extra_after[idx];
        }
    }

    for (idx, edge) in graph.edges.iter().enumerate() {
        let from = nodes.get(&edge.from).expect("from node missing");
        let to = nodes.get(&edge.to).expect("to node missing");
        let y = message_ys.get(idx).copied().unwrap_or(message_cursor);
        // Sequence message labels honor only explicit <br/> breaks; never
        // auto-wrap. Actor spacing was sized to fit each label on a single
        // physical line.
        let label = edge
            .label
            .as_ref()
            .map(|l| super::text::measure_label_no_wrap(l, theme, config));
        let start_label = edge
            .start_label
            .as_ref()
            .map(|l| super::text::measure_label_no_wrap(l, theme, config));
        let end_label = edge
            .end_label
            .as_ref()
            .map(|l| super::text::measure_label_no_wrap(l, theme, config));

        let points = if edge.from == edge.to {
            let pad = config.node_spacing.max(20.0) * 0.6;
            let x = from.x + from.width / 2.0;
            vec![(x, y), (x + pad, y), (x + pad, y + pad), (x, y + pad)]
        } else {
            let from_x = from.x + from.width / 2.0;
            let to_x = to.x + to.width / 2.0;
            // Subtract a small margin from each endpoint that has an arrow
            // so the arrowhead tip lands ON the destination lifeline rather
            // than past it. Mirrors mermaid.js's per-side `arrowSize` shrink.
            const ARROW_MARGIN: f32 = 4.0;
            // Activation half-width: when a participant is currently in an
            // activation block at this message, its endpoint should land on
            // the activation rectangle's edge (offset toward the other side
            // by activation_width/2 = 5px) rather than crossing through the
            // activation rect to the lifeline center.
            const ACTIVATION_OFFSET: f32 = 5.0;
            let direction = if to_x >= from_x { 1.0 } else { -1.0 };
            let has_end_arrow = edge.arrow_end || edge.sequence_arrow_end.is_some();
            let has_start_arrow = edge.arrow_start || edge.sequence_arrow_start.is_some();
            let from_active = is_actor_active_at(&graph.sequence_activations, &edge.from, idx);
            let to_active = is_actor_active_at(&graph.sequence_activations, &edge.to, idx);
            // Source side moves toward destination if active.
            let from_x = if from_active {
                from_x + direction * ACTIVATION_OFFSET
            } else {
                from_x
            };
            // Destination side moves toward source if active.
            let to_x = if to_active {
                to_x - direction * ACTIVATION_OFFSET
            } else {
                to_x
            };
            let adjusted_to_x = if has_end_arrow {
                to_x - direction * ARROW_MARGIN
            } else {
                to_x
            };
            let adjusted_from_x = if has_start_arrow {
                from_x + direction * ARROW_MARGIN
            } else {
                from_x
            };
            vec![(adjusted_from_x, y), (adjusted_to_x, y)]
        };

        let mut override_style = resolve_edge_style(idx, graph);
        if edge.style == crate::ir::EdgeStyle::Dotted && override_style.dasharray.is_none() {
            override_style.dasharray = Some("3 3".to_string());
        }
        edges.push(EdgeLayout {
            from: edge.from.clone(),
            to: edge.to.clone(),
            label,
            start_label,
            end_label,
            label_anchor: None,
            start_label_anchor: None,
            end_label_anchor: None,
            points,
            directed: edge.directed,
            arrow_start: edge.arrow_start,
            arrow_end: edge.arrow_end,
            arrow_start_kind: edge.arrow_start_kind,
            arrow_end_kind: edge.arrow_end_kind,
            start_decoration: edge.start_decoration,
            end_decoration: edge.end_decoration,
            sequence_arrow_end: edge.sequence_arrow_end,
            sequence_arrow_start: edge.sequence_arrow_start,
            style: edge.style,
            override_style,
            curve: None,
        });
    }

    let mut sequence_frames = Vec::new();
    if !graph.sequence_frames.is_empty() && !message_ys.is_empty() {
        let mut frames = graph.sequence_frames.clone();
        frames.sort_by(|a, b| {
            a.start_idx
                .cmp(&b.start_idx)
                .then_with(|| b.end_idx.cmp(&a.end_idx))
        });
        let frames_ref = frames.clone();
        for frame in frames {
            if frame.start_idx >= frame.end_idx || frame.start_idx >= message_ys.len() {
                continue;
            }
            // Mermaid.js loop/critical/par frames span from actor CENTER to
            // actor center, not from actor.x to actor.x+width. The frame
            // border lines cross through the actor boxes (cutting them in half
            // visually). See sequenceRenderer.ts loopWidths uses actor center.
            let mut min_x = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            for edge in graph
                .edges
                .iter()
                .skip(frame.start_idx)
                .take(frame.end_idx.saturating_sub(frame.start_idx))
            {
                if let Some(node) = nodes.get(&edge.from) {
                    let cx = node.x + node.width / 2.0;
                    min_x = min_x.min(cx);
                    max_x = max_x.max(cx);
                    // Self-messages reserve actor.width/2 on EACH side of the
                    // lifeline in JS (calculateLoopBounds uses from.x ±
                    // msgModel.width/2 with msgModel.width defaulting to
                    // conf.width=150, so the loopback widens the frame by
                    // ~node.width/2 on both sides). Without this, frames
                    // containing self-messages render too narrow on the left.
                    // JS's `activationBounds` uses `center+1` as the right
                    // edge (and inserts span startx-dx to stopx+dx), so the
                    // envelope is asymmetric: -74 left, +76 right of center
                    // for default node.width=150.
                    if edge.from == edge.to {
                        min_x = min_x.min(cx - node.width / 2.0 + 1.0);
                        max_x = max_x.max(cx + node.width / 2.0 + 1.0);
                    }
                }
                if let Some(node) = nodes.get(&edge.to) {
                    let cx = node.x + node.width / 2.0;
                    min_x = min_x.min(cx);
                    max_x = max_x.max(cx);
                }
            }
            if !min_x.is_finite() || !max_x.is_finite() {
                for node in nodes.values() {
                    let cx = node.x + node.width / 2.0;
                    min_x = min_x.min(cx);
                    max_x = max_x.max(cx);
                }
            }
            if !min_x.is_finite() || !max_x.is_finite() {
                continue;
            }
            // JS uses fixed boxMargin=10 for frame nesting padding, NOT a
            // font-size-relative value. Match that to avoid +2.4px overshoot
            // on critical-region with self-msgs.
            // Nesting: each frame that this frame STRICTLY contains adds an
            // extra 10px outward so nested frames are visually inset rather
            // than coincident with their parent's borders. Equivalent to JS:
            // outer loop pad ≈ 21, inner alt/opt pad ≈ 11 — difference 10.
            // Count frames CONTAINED in this one (allowing shared endpoints,
            // but not the frame itself). Strict-on-both-ends missed cases like
            // a nested `par` whose last message is also the outer frame's last
            // message — both rects then collapsed to the same right/bottom
            // borders, making the nesting invisible.
            let nesting_below = frames_ref
                .iter()
                .filter(|other| {
                    other.start_idx >= frame.start_idx
                        && other.end_idx <= frame.end_idx
                        && (other.start_idx != frame.start_idx || other.end_idx != frame.end_idx)
                })
                .count() as f32;
            let frame_pad_x = 10.0 + nesting_below.min(2.0) * 10.0;
            let mut frame_width = (max_x - min_x) + frame_pad_x * 2.0;
            // Expand frame to fit any section title text that exceeds the
            // actor-center span. Mermaid.js's loopWidths accounts for the
            // section title in the loop's bounding box, with wrapPadding
            // (10) on each side.
            // Predict the labelBox width (mirrors the calc later at frame
            // construction). The FIRST section's label sits to the right of
            // the labelBox, so frame_width must accommodate
            //   labelBox_w + label_width + horizontal_pads
            // or the section label spills past the frame's right border.
            let predicted_label_box_w = {
                let label_text = match frame.kind {
                    crate::ir::SequenceFrameKind::Alt => "alt",
                    crate::ir::SequenceFrameKind::Opt => "opt",
                    crate::ir::SequenceFrameKind::Loop => "loop",
                    crate::ir::SequenceFrameKind::Par => "par",
                    crate::ir::SequenceFrameKind::Rect => "rect",
                    crate::ir::SequenceFrameKind::Critical => "critical",
                    crate::ir::SequenceFrameKind::Break => "break",
                };
                let label_w = super::text::measure_label_no_wrap(label_text, theme, config).width;
                (label_w + theme.font_size * 2.0).max(theme.font_size * 3.0)
            };
            const FRAME_TITLE_PAD: f32 = 16.0; // wrapPadding(10) + ~6 visual pad
            for (section_idx, section) in frame.sections.iter().enumerate() {
                if let Some(label_text) = &section.label {
                    // Rect frames don't render the section label as visible
                    // text (the "label" here is the rgb color expression).
                    // Skip width expansion so nested Rect frames keep their
                    // 10px horizontal inset rather than collapsing to the
                    // same width.
                    if matches!(frame.kind, crate::ir::SequenceFrameKind::Rect) {
                        continue;
                    }
                    let display = format!("[{}]", label_text);
                    let measured = super::text::measure_label_no_wrap(&display, theme, config);
                    // First section sits to the right of the labelBox; reserve
                    // labelBox width on the left. Other sections center in the
                    // full frame width.
                    let needed = if section_idx == 0 {
                        predicted_label_box_w + measured.width + frame_pad_x * 2.0 + FRAME_TITLE_PAD
                    } else {
                        measured.width + frame_pad_x * 2.0 + FRAME_TITLE_PAD
                    };
                    if needed > frame_width {
                        frame_width = needed;
                    }
                }
            }
            let frame_x = (min_x + max_x) / 2.0 - frame_width / 2.0;

            let first_y = message_ys
                .get(frame.start_idx)
                .copied()
                .unwrap_or(message_cursor);
            let last_idx = frame.end_idx.saturating_sub(1);
            let last_y = message_ys.get(last_idx).copied().unwrap_or(first_y);
            // Self-loop messages extend `pad` (≥30px) below their message Y
            // for the loopback. Frame must enclose the loopback PLUS the
            // arrow marker glyph (markerHeight ≈ 12 px, so ~6 px past line
            // endpoint), or the rendered arrowhead tip nearly touches the
            // frame bottom border. JS leaves ~40 px between hook bottom and
            // frame bottom; we mirror that with `node_spacing*0.6 + font*0.8`
            // (≈ 42 px for default config), then bottom_offset adds another
            // ~9.6 on top.
            let last_self_loop_pad = if graph
                .edges
                .get(last_idx)
                .map(|e| e.from == e.to)
                .unwrap_or(false)
            {
                config.node_spacing.max(20.0) * 0.6 + theme.font_size * 0.8
            } else {
                0.0
            };
            let mut min_y = first_y;
            let mut max_y = last_y + last_self_loop_pad;
            for note in &sequence_notes {
                // Notes whose `index` equals frame.end_idx are positioned AFTER
                // the frame's `end` keyword in source order — JS's drawNote runs
                // after LOOP_END, so the note is OUTSIDE the loop visually. Use
                // `<` (not `<=`) to keep such notes from inflating the frame
                // rect (e.g. autonumber: post-loop note was wrapped by loop).
                if note.index >= frame.start_idx && note.index < frame.end_idx {
                    min_y = min_y.min(note.y);
                    max_y = max_y.max(note.y + note.height);
                }
            }
            let header_offset = theme.font_size * 0.6;
            // Rect (background highlight) frames have no label box, so they
            // hug the message rows tightly. Other frame kinds (loop, alt,
            // critical, par, opt) reserve room above the first message for
            // the label box.
            let top_offset = if matches!(frame.kind, crate::ir::SequenceFrameKind::Rect) {
                // Rect frames hug content tightly. When the first enclosed
                // element is a NOTE, min_y is already the note's top edge and
                // a small header_offset suffices. When it's a MESSAGE, the
                // message LABEL sits ABOVE its line by ~font_size*0.9 — we
                // must reserve enough headroom or the label escapes the
                // highlight rect. JS observed: ~font*1.5 above msg line.
                let first_is_message = (min_y - first_y).abs() < 0.5;
                if first_is_message {
                    theme.font_size * 1.5
                } else {
                    header_offset
                }
            } else {
                (2.0 * base_spacing - header_offset).max(base_spacing)
            };
            // Nested frames: outer frame bottom needs extra clearance so its
            // bottom border doesn't coincide with the inner frame's bottom
            // border (which would visually merge the two rects).
            let bottom_offset = header_offset + nesting_below.min(2.0) * 10.0;
            let frame_y = min_y - top_offset;
            let frame_height = (max_y - min_y).max(0.0) + top_offset + bottom_offset;

            let frame_label_text = match frame.kind {
                crate::ir::SequenceFrameKind::Alt => "alt",
                crate::ir::SequenceFrameKind::Opt => "opt",
                crate::ir::SequenceFrameKind::Loop => "loop",
                crate::ir::SequenceFrameKind::Par => "par",
                crate::ir::SequenceFrameKind::Rect => "rect",
                crate::ir::SequenceFrameKind::Critical => "critical",
                crate::ir::SequenceFrameKind::Break => "break",
            };
            let label_block = measure_label(frame_label_text, theme, config);
            let label_box_w =
                (label_block.width + theme.font_size * 2.0).max(theme.font_size * 3.0);
            let label_box_h = theme.font_size * 1.25;
            let label_box_x = frame_x;
            let label_box_y = frame_y;
            let label = SequenceLabel {
                x: label_box_x + label_box_w / 2.0,
                y: label_box_y + label_box_h / 2.0,
                text: label_block,
            };

            let mut dividers = Vec::new();
            let divider_offset = theme.font_size * 0.9;
            // Self-loop messages extend `pad` (= node_spacing*0.6 ≥ 30) below
            // their message Y for the loopback path. The divider must sit
            // below the loopback's bottom edge or it bisects the loop.
            let self_loop_pad = config.node_spacing.max(20.0) * 0.6;
            for window in frame.sections.windows(2) {
                let prev_end = window[0].end_idx;
                let last_idx = prev_end.saturating_sub(1);
                let base_y = message_ys.get(last_idx).copied().unwrap_or(first_y);
                let last_is_self_loop = graph
                    .edges
                    .get(last_idx)
                    .map(|e| e.from == e.to)
                    .unwrap_or(false);
                let extra = if last_is_self_loop {
                    self_loop_pad
                } else {
                    0.0
                };
                dividers.push(base_y + extra + divider_offset);
            }

            let mut section_labels = Vec::new();
            // Distance from divider line to section-label CENTER. text_block_svg
            // renders the baseline at center + 4 with default text metrics, and
            // the glyph top is `baseline - font*0.8` above the baseline. To
            // achieve JS's ~10px glyph-top-to-divider clearance with a 16px
            // font, center must sit ~font*1.18 below the divider (baseline ≈
            // divider + 22.8 → glyph top ≈ divider + 10).
            let label_offset = theme.font_size * 1.2;
            for (section_idx, section) in frame.sections.iter().enumerate() {
                if let Some(label) = &section.label {
                    let display = format!("[{}]", label);
                    // Section labels are single-line in mermaid.js — never wrap
                    // (wrapping pushes the second line into the section's first
                    // message row and creates a "text too close to a line"
                    // collision the user has flagged).
                    let block = super::text::measure_label_no_wrap(&display, theme, config);
                    let label_y = if section_idx == 0 {
                        // Keep the first section label close to the frame header
                        // so it does not collide with the first message row.
                        frame_y + label_box_h + theme.font_size * 0.45
                    } else {
                        // Place else/and section labels BELOW the divider with full
                        // label_offset clearance, matching mermaid.js. The labelBox
                        // sits across the dashed divider; the text baseline lands
                        // ~font*0.7 below the line so it doesn't crowd it.
                        dividers
                            .get(section_idx - 1)
                            .copied()
                            .unwrap_or(frame_y + label_offset)
                            + label_offset
                    };
                    let side_pad = theme.font_size * 0.45;
                    let label_x = if section_idx == 0 {
                        // First section label sits in the space to the right of
                        // the frame's labelBox: centered between labelBox right
                        // edge and frame right edge (mermaid.js convention).
                        let preferred = frame_x + (label_box_w + frame_width) / 2.0;
                        let min_x =
                            frame_x + label_box_w + block.width / 2.0 + theme.font_size * 0.4;
                        let max_x =
                            frame_x + frame_width - block.width / 2.0 - theme.font_size * 0.4;
                        // Guard: if the label is wider than the available
                        // space (min_x > max_x), fall back to the preferred
                        // center to avoid a clamp panic.
                        if min_x <= max_x {
                            preferred.clamp(min_x, max_x)
                        } else {
                            preferred
                        }
                    } else {
                        // else/and/or section labels: centered in the frame
                        // (mermaid.js convention). Falls back to side_pad if
                        // the label is too wide for the frame.
                        let preferred = frame_x + frame_width / 2.0;
                        let min_x = frame_x + block.width / 2.0 + side_pad;
                        let max_x = frame_x + frame_width - block.width / 2.0 - side_pad;
                        if min_x <= max_x {
                            preferred.clamp(min_x, max_x)
                        } else {
                            preferred
                        }
                    };
                    section_labels.push(SequenceLabel {
                        x: label_x,
                        y: label_y,
                        text: block,
                    });
                }
            }

            // For `rect <color>` background-highlight blocks, the first
            // section's label is the literal color expression. Capture it
            // for the renderer.
            let fill_color = if matches!(frame.kind, crate::ir::SequenceFrameKind::Rect) {
                frame
                    .sections
                    .first()
                    .and_then(|s| s.label.as_ref())
                    .map(|s| s.clone())
            } else {
                None
            };
            sequence_frames.push(SequenceFrameLayout {
                kind: frame.kind,
                x: frame_x,
                y: frame_y,
                width: frame_width,
                height: frame_height,
                label_box: (label_box_x, label_box_y, label_box_w, label_box_h),
                label,
                section_labels,
                dividers,
                fill_color,
            });
        }
    }

    let lifeline_start = actor_top_y + actor_height;
    let mut last_message_y = message_ys
        .last()
        .copied()
        .unwrap_or(lifeline_start + base_spacing);
    for note in &sequence_notes {
        last_message_y = last_message_y.max(note.y + note.height);
    }
    // Lifecycle events on the LAST message also extend the diagram tail.
    if let Some(last_idx) = graph.edges.len().checked_sub(1)
        && last_idx < lifecycle_extra_after.len()
    {
        last_message_y += lifecycle_extra_after[last_idx];
    }
    last_message_y += frame_tail_pad;
    // Mermaid.js renders `control` actor type with a body symbol (circle + icon)
    // above text; the text label sits ~12px below where a regular actor-box's
    // text would sit. `database` (cylinder) similarly extends a few px beyond
    // the actor.height envelope. Other actor-man-like types (boundary, entity,
    // queue, stick) keep their text within the actor.height envelope.
    let has_control_actor = participants.iter().any(|id| {
        graph
            .nodes
            .get(id)
            .map(|n| matches!(n.shape, crate::ir::NodeShape::Control))
            .unwrap_or(false)
    });
    let has_database_actor = participants.iter().any(|id| {
        graph
            .nodes
            .get(id)
            .map(|n| matches!(n.shape, crate::ir::NodeShape::Cylinder))
            .unwrap_or(false)
    });
    let actor_man_extra = if has_control_actor {
        19.0
    } else if has_database_actor {
        5.0
    } else {
        0.0
    };
    let footbox_gap = (theme.font_size * 1.25).max(16.0) + actor_man_extra;
    let lifeline_end = last_message_y + footbox_gap;

    // Resolve `create`/`destroy` lifecycle events into per-participant
    // y-coordinates. The convention (matches mermaid.js):
    //   - `create X` before message N → X first appears at message_ys[N];
    //     the top actor box is centered on that y, the lifeline starts at
    //     the bottom of that box.
    //   - `destroy X` before message N → X's lifeline ends at message_ys[N];
    //     the bottom actor box top sits at that y, an X-cross is drawn at
    //     the same y on the lifeline.
    let mut lifecycle_create: HashMap<String, f32> = HashMap::new();
    let mut lifecycle_destroy: HashMap<String, f32> = HashMap::new();
    for event in &graph.sequence_lifecycle {
        let y = message_ys.get(event.index).copied().unwrap_or(
            if matches!(event.kind, crate::ir::SequenceLifecycleKind::Create) {
                lifeline_start
            } else {
                lifeline_end
            },
        );
        match event.kind {
            crate::ir::SequenceLifecycleKind::Create => {
                lifecycle_create.insert(event.participant.clone(), y);
            }
            crate::ir::SequenceLifecycleKind::Destroy => {
                lifecycle_destroy.insert(event.participant.clone(), y);
            }
        }
    }

    // Re-position the top actor box for created participants so it sits
    // centered on its create-message y.
    for (id, &create_y) in &lifecycle_create {
        if let Some(node) = nodes.get_mut(id) {
            let new_y = (create_y - node.height / 2.0).max(margin);
            node.y = new_y;
        }
    }

    let mut lifelines = participants
        .iter()
        .filter_map(|id| nodes.get(id))
        .map(|node| {
            let y1 = if let Some(&cy) = lifecycle_create.get(&node.id) {
                // Lifeline starts just below the (repositioned) top actor box.
                node.y + node.height
            } else {
                lifeline_start
            };
            let y2 = lifecycle_destroy
                .get(&node.id)
                .copied()
                .unwrap_or(lifeline_end);
            Lifeline {
                id: node.id.clone(),
                x: node.x + node.width / 2.0,
                y1,
                y2,
            }
        })
        .collect::<Vec<_>>();

    // Destroyed actor footer rects sit BELOW the destroy y, not on it. JS
    // leaves ~one message-row of clearance so the destroy message's crosshead
    // arrow tip and the footer rect's top border don't overlap. Without this
    // pad the rect's top stroke runs through the crosshead marker.
    let destroy_footer_pad = theme.font_size * 1.5;
    let mut sequence_footboxes = participants
        .iter()
        .filter_map(|id| nodes.get(id))
        .map(|node| {
            let mut foot = node.clone();
            foot.y = match lifecycle_destroy.get(&node.id).copied() {
                Some(destroy_y) => destroy_y + destroy_footer_pad,
                None => lifeline_end,
            };
            foot
        })
        .collect::<Vec<_>>();

    // X-cross markers at each destroy y on the participant's lifeline x.
    let destroy_markers: Vec<(f32, f32)> = participants
        .iter()
        .filter_map(|id| {
            let y = lifecycle_destroy.get(id).copied()?;
            let node = nodes.get(id)?;
            Some((node.x + node.width / 2.0, y))
        })
        .collect();

    let mut sequence_boxes = Vec::new();
    if !graph.sequence_boxes.is_empty() {
        // JS box horizontal padding = actorMargin / 2 = 25px (sequenceRenderer
        // drawBackgroundRect: leftmost actor x − actorMargin/2). Without this
        // adjacent boxes show a visible gap where JS has them touching.
        let pad_x = 25.0_f32;
        // JS box bottom padding: noteMargin (8) + boxMargin/2 + ~1px ≈ 11px
        // for 16px font. Our prior 9.6 left grouping-with-box short by 1.4px.
        let pad_y = theme.font_size * 0.6875;
        let bottom = sequence_footboxes
            .iter()
            .map(|foot| foot.y + foot.height)
            .fold(lifeline_end, f32::max);
        for seq_box in &graph.sequence_boxes {
            let mut min_x = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            for participant in &seq_box.participants {
                if let Some(node) = nodes.get(participant) {
                    min_x = min_x.min(node.x);
                    max_x = max_x.max(node.x + node.width);
                }
            }
            if !min_x.is_finite() || !max_x.is_finite() {
                continue;
            }
            let x = min_x - pad_x;
            let y = 0.0;
            let width = (max_x - min_x) + pad_x * 2.0;
            let height = bottom + pad_y;
            let label = seq_box
                .label
                .as_ref()
                .map(|text| measure_label(text, theme, config));
            sequence_boxes.push(SequenceBoxLayout {
                x,
                y,
                width,
                height,
                label,
                color: seq_box.color.clone(),
            });
        }
    }
    // Mermaid.js uses a fixed 10px-wide activation rect with 5px stack
    // offset. Match those dimensions exactly.
    let activation_width = 10.0_f32;
    let activation_offset = 5.0_f32;
    let activation_end_default = message_ys
        .last()
        .copied()
        .unwrap_or(lifeline_start + base_spacing * 0.5)
        + base_spacing * 0.6;
    let mut sequence_activations = Vec::new();
    let mut activation_stacks: HashMap<String, Vec<(f32, usize)>> = HashMap::new();
    let mut events = graph
        .sequence_activations
        .iter()
        .cloned()
        .enumerate()
        .map(|(order, event)| (event.index, order, event))
        .collect::<Vec<_>>();
    events.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let activation_y_for = |idx: usize| {
        if idx < message_ys.len() {
            message_ys[idx]
        } else {
            activation_end_default
        }
    };
    for (_, _, event) in events {
        let y = activation_y_for(event.index);
        let stack = activation_stacks
            .entry(event.participant.clone())
            .or_default();
        match event.kind {
            crate::ir::SequenceActivationKind::Activate => {
                let depth = stack.len();
                stack.push((y, depth));
            }
            crate::ir::SequenceActivationKind::Deactivate => {
                if let Some((start_y, depth)) = stack.pop()
                    && let Some(node) = nodes.get(&event.participant)
                {
                    let base_x = node.x + node.width / 2.0 - activation_width / 2.0;
                    let x = base_x + depth as f32 * activation_offset;
                    let mut y0 = start_y.min(y);
                    let mut height = (y - start_y).abs();
                    if height < base_spacing * 0.6 {
                        height = base_spacing * 0.6;
                    }
                    if y0 < lifeline_start {
                        y0 = lifeline_start;
                    }
                    sequence_activations.push(SequenceActivationLayout {
                        x,
                        y: y0,
                        width: activation_width,
                        height,
                        participant: event.participant.clone(),
                        depth,
                    });
                }
            }
        }
    }
    for (participant, stack) in activation_stacks {
        for (start_y, depth) in stack {
            if let Some(node) = nodes.get(&participant) {
                let base_x = node.x + node.width / 2.0 - activation_width / 2.0;
                let x = base_x + depth as f32 * activation_offset;
                let mut y0 = start_y.min(activation_end_default);
                let mut height = (activation_end_default - start_y).abs();
                if height < base_spacing * 0.6 {
                    height = base_spacing * 0.6;
                }
                if y0 < lifeline_start {
                    y0 = lifeline_start;
                }
                sequence_activations.push(SequenceActivationLayout {
                    x,
                    y: y0,
                    width: activation_width,
                    height,
                    participant: participant.clone(),
                    depth,
                });
            }
        }
    }

    let mut sequence_numbers = Vec::new();
    if let Some(start) = graph.sequence_autonumber {
        let mut value = start;
        for (idx, edge) in graph.edges.iter().enumerate() {
            if let (Some(from), Some(y)) = (nodes.get(&edge.from), message_ys.get(idx).copied()) {
                let from_x = from.x + from.width / 2.0;
                let to_x = nodes
                    .get(&edge.to)
                    .map(|node| node.x + node.width / 2.0)
                    .unwrap_or(from_x);
                // Mermaid.js places the sequence-number circle exactly at
                // the source-actor's lifeline x (the line's start point),
                // not offset along the line.
                let number_y = y;
                sequence_numbers.push(SequenceNumberLayout {
                    x: from_x,
                    y: number_y,
                    value,
                });
                value += 1;
            }
        }
    }

    place_sequence_label_anchors(
        &mut edges,
        &nodes,
        &sequence_footboxes,
        &sequence_frames,
        &sequence_notes,
        &sequence_activations,
        &sequence_numbers,
        theme,
    );

    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for node in nodes.values() {
        extend_bounds(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            node.x,
            node.y,
            node.width,
            node.height,
        );
    }
    for footbox in &sequence_footboxes {
        extend_bounds(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            footbox.x,
            footbox.y,
            footbox.width,
            footbox.height,
        );
    }
    for seq_box in &sequence_boxes {
        // JS extends bounds with INTERNAL box (boxTextMargin=5 each side
        // beyond actor edges), NOT the drawn rect. The drawn rect (with
        // pad_x ≈ 12.8 each side) extends visually beyond viewBox slightly.
        // Without this distinction grouping-with-box overshoots width.
        const BOX_BOUNDS_INSET: f32 = 5.0;
        let pad_x = theme.font_size * 0.8;
        let inset = pad_x - BOX_BOUNDS_INSET;
        extend_bounds(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            seq_box.x + inset,
            seq_box.y,
            (seq_box.width - 2.0 * inset).max(0.0),
            seq_box.height,
        );
    }
    for frame in &sequence_frames {
        extend_bounds(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            frame.x,
            frame.y,
            frame.width,
            frame.height,
        );
    }
    for note in &sequence_notes {
        extend_bounds(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            note.x,
            note.y,
            note.width,
            note.height,
        );
    }
    for activation in &sequence_activations {
        extend_bounds(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            activation.x,
            activation.y,
            activation.width,
            activation.height,
        );
    }
    for number in &sequence_numbers {
        extend_bounds(
            &mut min_x, &mut min_y, &mut max_x, &mut max_y, number.x, number.y, 0.0, 0.0,
        );
    }
    for edge in &edges {
        for point in &edge.points {
            extend_bounds(
                &mut min_x, &mut min_y, &mut max_x, &mut max_y, point.0, point.1, 0.0, 0.0,
            );
        }
        if let (Some(label), Some((x, y))) = (&edge.label, edge.label_anchor) {
            extend_bounds(
                &mut min_x,
                &mut min_y,
                &mut max_x,
                &mut max_y,
                x - label.width / 2.0 - SEQUENCE_LABEL_PAD_X,
                y - label.height / 2.0 - SEQUENCE_LABEL_PAD_Y,
                label.width + 2.0 * SEQUENCE_LABEL_PAD_X,
                label.height + 2.0 * SEQUENCE_LABEL_PAD_Y,
            );
        }
        if let (Some(label), Some((x, y))) = (&edge.start_label, edge.start_label_anchor) {
            extend_bounds(
                &mut min_x,
                &mut min_y,
                &mut max_x,
                &mut max_y,
                x - label.width / 2.0 - SEQUENCE_ENDPOINT_LABEL_PAD_X,
                y - label.height / 2.0 - SEQUENCE_ENDPOINT_LABEL_PAD_Y,
                label.width + 2.0 * SEQUENCE_ENDPOINT_LABEL_PAD_X,
                label.height + 2.0 * SEQUENCE_ENDPOINT_LABEL_PAD_Y,
            );
        }
        if let (Some(label), Some((x, y))) = (&edge.end_label, edge.end_label_anchor) {
            extend_bounds(
                &mut min_x,
                &mut min_y,
                &mut max_x,
                &mut max_y,
                x - label.width / 2.0 - SEQUENCE_ENDPOINT_LABEL_PAD_X,
                y - label.height / 2.0 - SEQUENCE_ENDPOINT_LABEL_PAD_Y,
                label.width + 2.0 * SEQUENCE_ENDPOINT_LABEL_PAD_X,
                label.height + 2.0 * SEQUENCE_ENDPOINT_LABEL_PAD_Y,
            );
        }
    }
    if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
        min_x = 0.0;
        min_y = 0.0;
        max_x = 1.0;
        max_y = 1.0;
    }

    // Mermaid.js sequence diagrams use 50px horizontal padding around content
    // (viewBox attributes like `-50 -10 W H`). Our width formula is
    // `(max_x_shifted - old_min_x) + 2*margin`, which expands to
    // `extent + 3*margin - old_min_x`, so margin=36 gives effective 100px
    // padding matching JS (since old_min_x=8). Messages are scaled in gap
    // calculations to match JS text measurement, so content extent matches
    // JS and margin=36 is appropriate for both minimum-content and
    // message-widened diagrams.
    let _ = any_gap_widened;
    let margin = 36.0;
    // JS vertical padding: 10px top, 11px bottom (21 total).
    // Our formula `extent + 3*margin_y - old_min_y` (old_min_y = 8) applies
    // the same 3x amplification as the x-axis due to the asymmetric min_y
    // shift. Solving `3*margin_y - 8 = 21` yields margin_y ≈ 9.667 for
    // exact JS parity on minimum-content diagrams.
    let margin_y = 29.0 / 3.0;
    // Shift to position content's leftmost edge at `margin` from viewBox 0.
    // Cap min_x at the initial cursor margin (8.0) — this prevents over-shifting
    // when content extends LEFT of the typical cursor start (e.g. self-message
    // frame extension widens min_x leftward). Without the cap, the formula
    // `width = max_x - min_x + 2*margin` would inflate by |min_x - 8|, since
    // only max_x gets the shift while min_x stays in the original frame.
    let shift_x = margin - min_x.max(8.0);
    let shift_y = margin_y - min_y;
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
            if let Some((x, y)) = edge.label_anchor {
                edge.label_anchor = Some((x + shift_x, y + shift_y));
            }
            if let Some((x, y)) = edge.start_label_anchor {
                edge.start_label_anchor = Some((x + shift_x, y + shift_y));
            }
            if let Some((x, y)) = edge.end_label_anchor {
                edge.end_label_anchor = Some((x + shift_x, y + shift_y));
            }
        }
        for lifeline in &mut lifelines {
            lifeline.x += shift_x;
            lifeline.y1 += shift_y;
            lifeline.y2 += shift_y;
        }
        for footbox in &mut sequence_footboxes {
            footbox.x += shift_x;
            footbox.y += shift_y;
        }
        for seq_box in &mut sequence_boxes {
            seq_box.x += shift_x;
            seq_box.y += shift_y;
        }
        for frame in &mut sequence_frames {
            frame.x += shift_x;
            frame.y += shift_y;
            frame.label_box.0 += shift_x;
            frame.label_box.1 += shift_y;
            frame.label.x += shift_x;
            frame.label.y += shift_y;
            for label in &mut frame.section_labels {
                label.x += shift_x;
                label.y += shift_y;
            }
            for divider in &mut frame.dividers {
                *divider += shift_y;
            }
        }
        for note in &mut sequence_notes {
            note.x += shift_x;
            note.y += shift_y;
        }
        for activation in &mut sequence_activations {
            activation.x += shift_x;
            activation.y += shift_y;
        }
        for number in &mut sequence_numbers {
            number.x += shift_x;
            number.y += shift_y;
        }
        max_x += shift_x;
        max_y += shift_y;
    }
    // destroy_markers are computed pre-shift from raw node coords; apply
    // the same shift_x/shift_y so they land on the rendered lifeline.
    let destroy_markers: Vec<(f32, f32)> = destroy_markers
        .into_iter()
        .map(|(x, y)| (x + shift_x, y + shift_y))
        .collect();

    let width = (max_x - min_x + margin * 2.0).max(1.0);
    let height = (max_y - min_y + margin_y * 2.0).max(1.0);

    Layout {
        kind: graph.kind,
        nodes,
        edges,
        subgraphs,
        width,
        height,
        acc_title: None,
        acc_descr: None,
        diagram: DiagramData::Sequence(SequenceData {
            lifelines,
            footboxes: sequence_footboxes,
            boxes: sequence_boxes,
            frames: sequence_frames,
            notes: sequence_notes,
            activations: sequence_activations,
            numbers: sequence_numbers,
            destroy_markers,
        }),
    }
}

/// True if `participant` has an active activation block at message index
/// `msg_idx`. Activations are inclusive on both ends: the activate event
/// at idx X starts activation at message X, and the deactivate event at
/// idx Y ends activation AFTER message Y (so message Y is still inside
/// the activation block).
fn is_actor_active_at(
    activations: &[crate::ir::SequenceActivation],
    participant: &str,
    msg_idx: usize,
) -> bool {
    let mut count: i32 = 0;
    for ev in activations {
        if ev.index > msg_idx {
            break;
        }
        if ev.participant != participant {
            continue;
        }
        match ev.kind {
            crate::ir::SequenceActivationKind::Activate => count += 1,
            crate::ir::SequenceActivationKind::Deactivate => {
                if ev.index < msg_idx {
                    count -= 1;
                }
            }
        }
    }
    count > 0
}

fn place_sequence_label_anchors(
    edges: &mut [EdgeLayout],
    nodes: &BTreeMap<String, NodeLayout>,
    footboxes: &[NodeLayout],
    frames: &[SequenceFrameLayout],
    notes: &[SequenceNoteLayout],
    activations: &[SequenceActivationLayout],
    numbers: &[SequenceNumberLayout],
    theme: &Theme,
) {
    if edges.is_empty() {
        return;
    }

    let mut occupied: Vec<Rect> = Vec::new();
    for node in nodes.values() {
        occupied.push((node.x, node.y, node.width, node.height));
    }
    for footbox in footboxes {
        occupied.push((footbox.x, footbox.y, footbox.width, footbox.height));
    }
    for frame in frames {
        occupied.push(frame.label_box);
        let section_pad_x = (theme.font_size * 0.18).max(1.5);
        let section_pad_y = (theme.font_size * 0.15).max(1.2);
        for label in &frame.section_labels {
            occupied.push((
                label.x - label.text.width / 2.0 - section_pad_x,
                label.y - label.text.height / 2.0 - section_pad_y,
                label.text.width + section_pad_x * 2.0,
                label.text.height + section_pad_y * 2.0,
            ));
        }
    }
    for note in notes {
        occupied.push((note.x, note.y, note.width, note.height));
    }
    for activation in activations {
        occupied.push((
            activation.x,
            activation.y,
            activation.width,
            activation.height,
        ));
    }
    let number_r = (theme.font_size * 0.45).max(6.0);
    for number in numbers {
        occupied.push((
            number.x - number_r,
            number.y - number_r,
            number_r * 2.0,
            number_r * 2.0,
        ));
    }

    let edge_paths: Vec<Vec<(f32, f32)>> = edges.iter().map(|edge| edge.points.clone()).collect();
    for idx in 0..edges.len() {
        if let Some(label) = edges[idx].label.clone() {
            let anchor = choose_sequence_center_label_anchor(
                &edge_paths[idx],
                &label,
                &occupied,
                &edge_paths,
                idx,
                theme,
            );
            edges[idx].label_anchor = Some(anchor);
            occupied.push(label_rect(
                anchor,
                &label,
                SEQUENCE_LABEL_PAD_X,
                SEQUENCE_LABEL_PAD_Y,
            ));
        }

        if let Some(label) = edges[idx].start_label.clone() {
            let anchor = choose_sequence_endpoint_label_anchor(
                &edge_paths[idx],
                &label,
                true,
                &occupied,
                &edge_paths,
                idx,
                theme,
            );
            edges[idx].start_label_anchor = anchor;
            if let Some(center) = anchor {
                occupied.push(label_rect(
                    center,
                    &label,
                    SEQUENCE_ENDPOINT_LABEL_PAD_X,
                    SEQUENCE_ENDPOINT_LABEL_PAD_Y,
                ));
            }
        }

        if let Some(label) = edges[idx].end_label.clone() {
            let anchor = choose_sequence_endpoint_label_anchor(
                &edge_paths[idx],
                &label,
                false,
                &occupied,
                &edge_paths,
                idx,
                theme,
            );
            edges[idx].end_label_anchor = anchor;
            if let Some(center) = anchor {
                occupied.push(label_rect(
                    center,
                    &label,
                    SEQUENCE_ENDPOINT_LABEL_PAD_X,
                    SEQUENCE_ENDPOINT_LABEL_PAD_Y,
                ));
            }
        }
    }
}

fn choose_sequence_center_label_anchor(
    points: &[(f32, f32)],
    label: &TextBlock,
    occupied: &[Rect],
    edge_paths: &[Vec<(f32, f32)>],
    edge_idx: usize,
    theme: &Theme,
) -> (f32, f32) {
    let (anchor, dir) = edge_midpoint_with_direction(points);
    let normal = (-dir.1, dir.0);
    let normal_step = (label.height * 0.5 + SEQUENCE_LABEL_PAD_Y).max(6.0);
    let tangent_step = (label.width + theme.font_size * 0.35).max(10.0) * 0.24;
    // Path-first search: keep center labels on their own message path and slide
    // along the path before moving off-path.
    let tangent_offsets_primary = [
        0.0, -0.25, 0.25, -0.55, 0.55, -0.95, 0.95, -1.45, 1.45, -2.1, 2.1, -2.9, 2.9, -3.8, 3.8,
        -4.9, 4.9, -6.2, 6.2,
    ];
    let tangent_offsets_wide = [
        0.0, -0.35, 0.35, -0.75, 0.75, -1.3, 1.3, -2.0, 2.0, -2.9, 2.9, -4.0, 4.0, -5.3, 5.3, -6.8,
        6.8, -8.4, 8.4,
    ];
    let normal_offsets_on_path = [0.0, -0.06, 0.06, -0.12, 0.12];
    let normal_offsets_near_path = [-0.28, 0.28, -0.46, 0.46, -0.68, 0.68];
    let normal_offsets_fallback = [-0.9, 0.9, -1.2, 1.2, -1.55, 1.55, -1.95, 1.95];
    let mut best = anchor;
    let mut best_score = f32::INFINITY;

    let mut evaluate_band = |tangent_offsets: &[f32], normal_offsets: &[f32]| {
        for t in tangent_offsets {
            for n in normal_offsets {
                let center = (
                    anchor.0 + dir.0 * tangent_step * *t + normal.0 * normal_step * *n,
                    anchor.1 + dir.1 * tangent_step * *t + normal.1 * normal_step * *n,
                );
                let rect = label_rect(center, label, SEQUENCE_LABEL_PAD_X, SEQUENCE_LABEL_PAD_Y);
                let mut score = sequence_label_penalty(
                    rect,
                    center,
                    anchor,
                    points,
                    occupied,
                    SequenceLabelPlacementMode::Center,
                );
                score += sequence_edge_overlap_penalty(rect, edge_paths, edge_idx);
                let own_dist = point_to_polyline_distance(center, points);
                score += own_dist * 0.045;
                // Keep center labels near message midpoint. We still allow drift
                // when required to resolve overlaps, but large tangent shifts are
                // strongly discouraged versus vertical escape.
                let tangent_abs = t.abs();
                score += tangent_abs * SEQUENCE_CENTER_LABEL_TANGENT_LINEAR_WEIGHT;
                score += tangent_abs * tangent_abs * SEQUENCE_CENTER_LABEL_TANGENT_QUAD_WEIGHT;
                if tangent_abs > SEQUENCE_CENTER_LABEL_TANGENT_SOFT_LIMIT {
                    score += (tangent_abs - SEQUENCE_CENTER_LABEL_TANGENT_SOFT_LIMIT)
                        * SEQUENCE_CENTER_LABEL_TANGENT_FAR_WEIGHT;
                }
                if dir.0.abs() > dir.1.abs() && center.1 > anchor.1 {
                    // Keep horizontal message labels out of the row below.
                    score += 0.3;
                }
                if score < best_score {
                    best_score = score;
                    best = center;
                }
            }
        }
    };

    evaluate_band(&tangent_offsets_primary, &normal_offsets_on_path);
    evaluate_band(&tangent_offsets_primary, &normal_offsets_near_path);
    evaluate_band(&tangent_offsets_wide, &normal_offsets_fallback);

    best
}

fn choose_sequence_endpoint_label_anchor(
    points: &[(f32, f32)],
    label: &TextBlock,
    start: bool,
    occupied: &[Rect],
    edge_paths: &[Vec<(f32, f32)>],
    edge_idx: usize,
    theme: &Theme,
) -> Option<(f32, f32)> {
    let ((anchor_x, anchor_y), dir) = sequence_endpoint_base(points, start, theme)?;
    let normal = (-dir.1, dir.0);
    let base_step = (theme.font_size * 0.45).max(6.0);
    let tangent_offsets = [0.0, 0.6, -0.6, 1.2, -1.2, 2.0, -2.0, 2.9, -2.9];
    let normal_offsets = [0.35, -0.35, 0.75, -0.75, 1.1, -1.1, 1.45, -1.45, 1.8, -1.8];
    let anchor = (anchor_x, anchor_y);
    let mut best = anchor;
    let mut best_score = f32::INFINITY;

    for t in tangent_offsets {
        for n in normal_offsets {
            let center = (
                anchor.0 + dir.0 * base_step * t + normal.0 * base_step * n,
                anchor.1 + dir.1 * base_step * t + normal.1 * base_step * n,
            );
            let rect = label_rect(
                center,
                label,
                SEQUENCE_ENDPOINT_LABEL_PAD_X,
                SEQUENCE_ENDPOINT_LABEL_PAD_Y,
            );
            let mut score = sequence_label_penalty(
                rect,
                center,
                anchor,
                points,
                occupied,
                SequenceLabelPlacementMode::Endpoint,
            );
            score += sequence_edge_overlap_penalty(rect, edge_paths, edge_idx);
            score += distance(center, anchor) * 0.05;
            if score < best_score {
                best_score = score;
                best = center;
            }
        }
    }

    Some(best)
}

fn sequence_endpoint_base(
    points: &[(f32, f32)],
    start: bool,
    theme: &Theme,
) -> Option<((f32, f32), (f32, f32))> {
    if points.len() < 2 {
        return None;
    }
    let (p0, p1) = if start {
        (points[0], points[1])
    } else {
        (points[points.len() - 1], points[points.len() - 2])
    };
    let dx = p1.0 - p0.0;
    let dy = p1.1 - p0.1;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= f32::EPSILON {
        return None;
    }
    let dir = (dx / len, dy / len);
    let offset = (theme.font_size * 0.45).max(6.0);
    let anchor = (p0.0 + dir.0 * offset * 1.4, p0.1 + dir.1 * offset * 1.4);
    Some((anchor, dir))
}

fn edge_midpoint_with_direction(points: &[(f32, f32)]) -> ((f32, f32), (f32, f32)) {
    if points.len() < 2 {
        let point = points.first().copied().unwrap_or((0.0, 0.0));
        return (point, (1.0, 0.0));
    }
    let mut lengths = Vec::with_capacity(points.len().saturating_sub(1));
    let mut total = 0.0f32;
    for segment in points.windows(2) {
        let len = distance(segment[0], segment[1]);
        lengths.push(len);
        total += len;
    }
    if total <= f32::EPSILON {
        let dx = points[1].0 - points[0].0;
        let dy = points[1].1 - points[0].1;
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        return (points[0], (dx / len, dy / len));
    }
    let target = total * 0.5;
    let mut acc = 0.0f32;
    for (idx, len) in lengths.iter().copied().enumerate() {
        if acc + len >= target {
            let seg = (points[idx], points[idx + 1]);
            let local_t = ((target - acc) / len.max(1e-6)).clamp(0.0, 1.0);
            let point = (
                seg.0.0 + (seg.1.0 - seg.0.0) * local_t,
                seg.0.1 + (seg.1.1 - seg.0.1) * local_t,
            );
            let dx = seg.1.0 - seg.0.0;
            let dy = seg.1.1 - seg.0.1;
            let dlen = (dx * dx + dy * dy).sqrt().max(1e-6);
            return (point, (dx / dlen, dy / dlen));
        }
        acc += len;
    }
    let last = points[points.len() - 1];
    let prev = points[points.len() - 2];
    let dx = last.0 - prev.0;
    let dy = last.1 - prev.1;
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    (last, (dx / len, dy / len))
}

fn sequence_label_penalty(
    rect: Rect,
    center: (f32, f32),
    anchor: (f32, f32),
    own_points: &[(f32, f32)],
    occupied: &[Rect],
    mode: SequenceLabelPlacementMode,
) -> f32 {
    let mut overlap_area_sum = 0.0f32;
    for obstacle in occupied {
        overlap_area_sum += rect_overlap_area(rect, *obstacle);
    }
    let own_gap = polyline_rect_gap(own_points, rect);
    let gap_penalty = match mode {
        SequenceLabelPlacementMode::Center => {
            if !own_gap.is_finite() {
                140.0
            } else if own_gap <= SEQUENCE_LABEL_TOUCH_EPS {
                0.0
            } else {
                let over = own_gap - SEQUENCE_LABEL_TOUCH_EPS;
                let mut penalty = over * over * 10.0 + over * 1.8;
                if own_gap > 8.0 {
                    penalty += (own_gap - 8.0) * 1.4;
                }
                penalty
            }
        }
        SequenceLabelPlacementMode::Endpoint => {
            let mut penalty = 0.0f32;
            if own_gap <= SEQUENCE_LABEL_TOUCH_EPS {
                penalty += 120.0 + (SEQUENCE_LABEL_TOUCH_EPS - own_gap).max(0.0) * 30.0;
            } else if own_gap < SEQUENCE_ENDPOINT_LABEL_GAP_MIN {
                let delta = (SEQUENCE_ENDPOINT_LABEL_GAP_MIN - own_gap)
                    / SEQUENCE_ENDPOINT_LABEL_GAP_MIN.max(1e-3);
                penalty += delta * delta * 14.0;
            } else if own_gap <= SEQUENCE_ENDPOINT_LABEL_GAP_MAX {
                let delta = (own_gap - SEQUENCE_ENDPOINT_LABEL_GAP_TARGET)
                    / SEQUENCE_ENDPOINT_LABEL_GAP_TARGET.max(1e-3);
                penalty += delta * delta * 0.9;
            } else {
                let far = own_gap - SEQUENCE_ENDPOINT_LABEL_GAP_MAX;
                penalty += far * far * 2.4 + far * 0.4;
                if own_gap > SEQUENCE_ENDPOINT_LABEL_FAR_GAP {
                    penalty += (own_gap - SEQUENCE_ENDPOINT_LABEL_FAR_GAP) * 0.9;
                }
            }
            penalty
        }
    };
    let anchor_weight = match mode {
        SequenceLabelPlacementMode::Center => 0.018,
        SequenceLabelPlacementMode::Endpoint => 0.025,
    };
    overlap_area_sum * 0.01 + gap_penalty + distance(center, anchor) * anchor_weight
}

fn sequence_edge_overlap_penalty(
    rect: Rect,
    edge_paths: &[Vec<(f32, f32)>],
    edge_idx: usize,
) -> f32 {
    let mut hits = 0usize;
    for (idx, points) in edge_paths.iter().enumerate() {
        if idx == edge_idx || points.len() < 2 {
            continue;
        }
        if points
            .windows(2)
            .any(|segment| segment_intersects_rect(segment[0], segment[1], rect))
        {
            hits += 1;
        }
    }
    hits as f32 * 3.0
}

fn label_rect(center: (f32, f32), label: &TextBlock, pad_x: f32, pad_y: f32) -> Rect {
    (
        center.0 - label.width / 2.0 - pad_x,
        center.1 - label.height / 2.0 - pad_y,
        label.width + pad_x * 2.0,
        label.height + pad_y * 2.0,
    )
}

fn rect_overlap_area(a: Rect, b: Rect) -> f32 {
    let x1 = a.0.max(b.0);
    let y1 = a.1.max(b.1);
    let x2 = (a.0 + a.2).min(b.0 + b.2);
    let y2 = (a.1 + a.3).min(b.1 + b.3);
    if x2 <= x1 || y2 <= y1 {
        return 0.0;
    }
    (x2 - x1) * (y2 - y1)
}

fn point_to_polyline_distance(point: (f32, f32), points: &[(f32, f32)]) -> f32 {
    if points.is_empty() {
        return 0.0;
    }
    if points.len() == 1 {
        return distance(point, points[0]);
    }
    points
        .windows(2)
        .map(|segment| point_to_segment_distance(point, segment[0], segment[1]))
        .fold(f32::INFINITY, f32::min)
}

fn point_rect_distance(point: (f32, f32), rect: Rect) -> f32 {
    let min_x = rect.0;
    let min_y = rect.1;
    let max_x = rect.0 + rect.2;
    let max_y = rect.1 + rect.3;
    let dx = if point.0 < min_x {
        min_x - point.0
    } else if point.0 > max_x {
        point.0 - max_x
    } else {
        0.0
    };
    let dy = if point.1 < min_y {
        min_y - point.1
    } else if point.1 > max_y {
        point.1 - max_y
    } else {
        0.0
    };
    (dx * dx + dy * dy).sqrt()
}

fn segment_rect_distance(a: (f32, f32), b: (f32, f32), rect: Rect) -> f32 {
    if segment_intersects_rect(a, b, rect) {
        return 0.0;
    }
    let mut best = point_rect_distance(a, rect).min(point_rect_distance(b, rect));
    let corners = [
        (rect.0, rect.1),
        (rect.0 + rect.2, rect.1),
        (rect.0 + rect.2, rect.1 + rect.3),
        (rect.0, rect.1 + rect.3),
    ];
    for corner in corners {
        best = best.min(point_to_segment_distance(corner, a, b));
    }
    best
}

fn polyline_rect_gap(points: &[(f32, f32)], rect: Rect) -> f32 {
    if points.len() < 2 {
        return f32::INFINITY;
    }
    points
        .windows(2)
        .map(|segment| segment_rect_distance(segment[0], segment[1], rect))
        .fold(f32::INFINITY, f32::min)
}

fn point_to_segment_distance(point: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let ab = (b.0 - a.0, b.1 - a.1);
    let len_sq = ab.0 * ab.0 + ab.1 * ab.1;
    if len_sq <= f32::EPSILON {
        return distance(point, a);
    }
    let ap = (point.0 - a.0, point.1 - a.1);
    let t = ((ap.0 * ab.0 + ap.1 * ab.1) / len_sq).clamp(0.0, 1.0);
    let proj = (a.0 + ab.0 * t, a.1 + ab.1 * t);
    distance(point, proj)
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

fn segment_intersects_rect(a: (f32, f32), b: (f32, f32), rect: Rect) -> bool {
    let (x, y, w, h) = rect;
    let min_x = a.0.min(b.0);
    let max_x = a.0.max(b.0);
    let min_y = a.1.min(b.1);
    let max_y = a.1.max(b.1);
    if max_x < x || min_x > x + w || max_y < y || min_y > y + h {
        return false;
    }
    if point_in_rect(a, rect) || point_in_rect(b, rect) {
        return true;
    }
    let corners = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
    for i in 0..4 {
        let c = corners[i];
        let d = corners[(i + 1) % 4];
        if segments_intersect(a, b, c, d) {
            return true;
        }
    }
    false
}

fn point_in_rect(point: (f32, f32), rect: Rect) -> bool {
    point.0 >= rect.0
        && point.0 <= rect.0 + rect.2
        && point.1 >= rect.1
        && point.1 <= rect.1 + rect.3
}

fn segments_intersect(a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32)) -> bool {
    const EPS: f32 = 1e-6;
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);

    if o1.abs() < EPS && on_segment(a, b, c) {
        return true;
    }
    if o2.abs() < EPS && on_segment(a, b, d) {
        return true;
    }
    if o3.abs() < EPS && on_segment(c, d, a) {
        return true;
    }
    if o4.abs() < EPS && on_segment(c, d, b) {
        return true;
    }
    (o1 > 0.0) != (o2 > 0.0) && (o3 > 0.0) != (o4 > 0.0)
}

fn orient(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn on_segment(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    const EPS: f32 = 1e-6;
    c.0 >= a.0.min(b.0) - EPS
        && c.0 <= a.0.max(b.0) + EPS
        && c.1 >= a.1.min(b.1) - EPS
        && c.1 <= a.1.max(b.1) + EPS
}

fn extend_bounds(
    min_x: &mut f32,
    min_y: &mut f32,
    max_x: &mut f32,
    max_y: &mut f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    *min_x = (*min_x).min(x);
    *min_y = (*min_y).min(y);
    *max_x = (*max_x).max(x + w);
    *max_y = (*max_y).max(y + h);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_center_label_prefers_touching_own_path() {
        let points = vec![(0.0, 0.0), (140.0, 0.0)];
        let label = TextBlock {
            lines: vec![TextLine::plain("msg".to_string())],
            width: 36.0,
            height: 14.0,
        };
        let theme = Theme::mermaid_default();
        let anchor = choose_sequence_center_label_anchor(
            &points,
            &label,
            &[],
            std::slice::from_ref(&points),
            0,
            &theme,
        );
        let rect = label_rect(anchor, &label, SEQUENCE_LABEL_PAD_X, SEQUENCE_LABEL_PAD_Y);
        let gap = polyline_rect_gap(&points, rect);
        assert!(
            gap <= SEQUENCE_LABEL_TOUCH_EPS + 1e-3,
            "expected touching center label, got gap {:.3}",
            gap
        );
    }

    #[test]
    fn sequence_center_label_moves_off_path_when_path_is_blocked() {
        let points = vec![(0.0, 0.0), (140.0, 0.0)];
        let label = TextBlock {
            lines: vec![TextLine::plain("msg".to_string())],
            width: 36.0,
            height: 14.0,
        };
        let theme = Theme::mermaid_default();
        let occupied = vec![(-20.0, -10.0, 180.0, 20.0)];
        let anchor = choose_sequence_center_label_anchor(
            &points,
            &label,
            &occupied,
            std::slice::from_ref(&points),
            0,
            &theme,
        );
        assert!(
            anchor.1.abs() > 4.0,
            "expected off-path fallback for blocked corridor, got y={:.2}",
            anchor.1
        );
        assert!(
            (anchor.0 - 70.0).abs() <= 8.0,
            "expected blocked fallback to stay near midpoint, got x={:.2}",
            anchor.0
        );
    }
}
