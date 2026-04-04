use super::*;

pub(super) fn compute_timeline_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> Layout {
    let data = &graph.timeline;
    let font_size = theme.font_size;

    // ── Constants matching JS timelineRenderer ──────────────────────────
    let card_path_width: f32 = 180.0; // internal path width
    let card_visible_width: f32 = 190.0; // path + line overshoot
    let card_spacing: f32 = 200.0; // center-to-center horizontal
    let card_line_y: f32 = 67.8; // divider line y within card
    let zone_gap: f32 = 50.0; // gap between vertical zones
    let axis_to_events: f32 = 82.2; // gap from axis to first event card
    let event_stack_offset: f32 = 60.0; // fixed y offset between stacked cards (JS behavior)
    let base_y: f32 = 50.0; // top of first zone
    let left_margin: f32 = 50.0; // JS LEFT_MARGIN
    let content_start_x: f32 = 200.0; // first card x=200 (matches all JS outputs)
    let axis_left_pad: f32 = 150.0; // JS axis x1 = LEFT_MARGIN, but within viewBox offset
    let connector_bottom_pad: f32 = 60.0; // extra depth below deepest event

    // Font metrics for text wrapping inside cards.
    let wrap_width = card_path_width - 10.0; // ~170px usable text width
    let event_line_height: f32 = font_size * 1.1; // dy="1.1em" for subsequent lines
    let event_first_line_extra: f32 = font_size; // dy="1em" for first line
    let event_text_pad: f32 = 20.0; // top+bottom padding inside event card

    let has_title = data.title.is_some();
    let title = data.title.as_ref().map(|t| measure_label(t, theme, config));

    // ── Group events by section ────────────────────────────────────────
    let has_sections = !data.sections.is_empty();
    let section_events: Vec<(&str, Vec<&crate::ir::TimelineEvent>)> = if has_sections {
        data.sections
            .iter()
            .map(|s| {
                let evts: Vec<_> = data
                    .events
                    .iter()
                    .filter(|e| e.section.as_deref() == Some(s.as_str()))
                    .collect();
                (s.as_str(), evts)
            })
            .collect()
    } else {
        // No sections — each event is its own "section" (auto-indexed).
        data.events.iter().map(|e| ("", vec![e])).collect()
    };

    // ── Wrap event text & compute per-card heights ─────────────────────
    struct WrappedEvent {
        lines: Vec<String>,
        height: f32,
    }

    let wrap_event = |text: &str| -> WrappedEvent {
        let wrapped = wrap_line(
            text,
            wrap_width,
            font_size,
            &theme.font_family,
            config.fast_text_metrics,
        );
        let num_lines = wrapped.len().max(1) as f32;
        let text_h = event_first_line_extra + (num_lines - 1.0) * event_line_height;
        let h = (text_h + event_text_pad).max(45.0);
        WrappedEvent {
            lines: wrapped,
            height: h,
        }
    };

    // ── Compute Y coordinates for vertical zones ───────────────────────
    let section_y = base_y;
    let time_y = if has_sections {
        section_y + card_line_y + zone_gap // 50 + 67.8 + 50 = 167.8
    } else {
        base_y
    };
    let axis_y = time_y + card_line_y + zone_gap;
    let events_start_y = axis_y + axis_to_events;

    // ── Layout all elements left-to-right ──────────────────────────────
    let mut sections_layout = Vec::new();
    let mut time_periods = Vec::new();
    let mut event_cards = Vec::new();
    let mut connectors = Vec::new();

    let mut x_cursor = content_start_x;
    let mut global_section_idx: i32 = -1; // JS starts at -1
    let mut max_event_bottom: f32 = events_start_y;
    let mut rightmost_card_x: f32 = content_start_x; // track rightmost card position

    for (group_idx, (sec_name, evts)) in section_events.iter().enumerate() {
        let num_periods = evts.len().max(1);
        let section_width = (num_periods as f32) * card_spacing - 10.0;

        // Section header (only when explicit sections exist)
        if has_sections && !sec_name.is_empty() {
            let label = measure_label(sec_name, theme, config);
            sections_layout.push(TimelineSectionLayout {
                label,
                x: x_cursor,
                y: section_y,
                width: section_width,
                height: card_line_y,
                section_idx: global_section_idx,
            });
        }

        // Time period cards + event cards for each event in this section
        for (evt_idx, evt) in evts.iter().enumerate() {
            let period_x = x_cursor + evt_idx as f32 * card_spacing;
            let center_x = period_x + card_visible_width / 2.0;

            let sec_idx = if has_sections {
                global_section_idx
            } else {
                -1 + group_idx as i32
            };

            // Time period card (above axis)
            let time_label = measure_label(&evt.time, theme, config);
            time_periods.push(TimelineTimePeriodLayout {
                label: time_label,
                x: period_x,
                y: time_y,
                width: card_visible_width,
                height: card_line_y,
                section_idx: sec_idx,
            });

            // Event cards (below axis), one per event description.
            // JS uses fixed 60px offset between stacked cards.
            let mut event_y = events_start_y;
            for event_text in &evt.events {
                let wrapped = wrap_event(event_text);
                event_cards.push(TimelineEventCardLayout {
                    lines: wrapped.lines,
                    x: period_x,
                    y: event_y,
                    width: card_visible_width,
                    height: wrapped.height,
                    section_idx: sec_idx,
                });
                event_y += event_stack_offset;
            }

            max_event_bottom = max_event_bottom.max(event_y);
            rightmost_card_x = rightmost_card_x.max(period_x);

            connectors.push(TimelineConnectorLayout {
                x: center_x,
                start_y: time_y + card_line_y,
                end_y: 0.0, // placeholder
            });
        }

        if has_sections {
            global_section_idx += 1;
        }
        x_cursor += section_width + 10.0;
    }

    // Connector depth: extend past deepest event + padding (matching JS).
    let connector_end = max_event_bottom + connector_bottom_pad;
    for conn in &mut connectors {
        conn.end_y = connector_end;
    }

    // ── Compute total dimensions ─────────────────────────────────────
    // JS: setupGraphViewbox() uses getBBox() + padding.
    // axis x2 = box.width + 3 * LEFT_MARGIN.
    // We approximate the bounding box from x_cursor (rightmost content edge).
    // JS: axis x2 = box.width + 3 * LEFT_MARGIN where box comes from getBBox.
    // From comparing outputs, axis extends ~40px past the rightmost card edge.
    // JS: axis x2 = box.width + 3 * LEFT_MARGIN ≈ rightmost_card_right + ~50px.
    // JS: axis x2 = box.width + 3 * LEFT_MARGIN, where box is from getBBox().
    // The getBBox includes the axis line starting at LEFT_MARGIN (=50 in viewBox coords)
    // plus content spanning to the last card. In practice, the axis extends ~250px past
    // the last card's left edge. We approximate by adding card_spacing/2 + 3*left_margin.
    let last_card_right = rightmost_card_x + card_visible_width;
    let axis_end_x = last_card_right + card_spacing / 2.0 + 2.0 * left_margin;
    let total_width = axis_end_x;
    let total_height = connector_end + 50.0;

    // Title: JS uses `box.width / 2 - LEFT_MARGIN` where box is getBBox().
    // From JS outputs, title_x = (num_periods * 100) - 155 consistently.
    // This is equivalent to: center_of_all_cards - LEFT_MARGIN.
    let total_periods: usize = section_events.iter().map(|(_, evts)| evts.len()).sum();
    let title_x = (total_periods as f32) * 100.0 - 155.0;
    let title_y = 20.0;

    // ── Build dummy node for metrics ───────────────────────────────────
    let mut nodes = BTreeMap::new();
    nodes.insert(
        "__timeline_metrics_content".to_string(),
        NodeLayout {
            id: "__timeline_metrics_content".to_string(),
            x: 0.0,
            y: 0.0,
            width: total_width.max(1.0),
            height: total_height.max(1.0),
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
        diagram: DiagramData::Timeline(TimelineLayout {
            title,
            title_x,
            title_y,
            sections: sections_layout,
            time_periods,
            event_cards,
            connectors,
            axis_y,
            axis_start_x: axis_left_pad,
            axis_end_x,
            width: total_width,
            height: total_height,
        }),
        width: total_width,
        height: total_height,
    }
}
