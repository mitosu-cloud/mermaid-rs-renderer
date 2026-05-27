use std::collections::BTreeMap;

use crate::config::LayoutConfig;
use crate::ir::{EventModelingEntityType, EventModelingFrame, Graph};
use crate::text_metrics;
use crate::theme::Theme;

use super::{
    DiagramData, EventModelingBoxLayout, EventModelingLayout, EventModelingRelationLayout,
    EventModelingSwimlaneLayout, Layout,
};

const SWIMLANE_MIN_HEIGHT: f32 = 70.0;
const SWIMLANE_PADDING: f32 = 15.0;
const SWIMLANE_GAP: f32 = 10.0;
const BOX_PADDING: f32 = 10.0;
const BOX_OVERLAP: f32 = 90.0;
const BOX_MIN_WIDTH: f32 = 80.0;
const BOX_MAX_WIDTH: f32 = 450.0;
const BOX_MIN_HEIGHT: f32 = 80.0;
const BOX_MAX_HEIGHT: f32 = 750.0;
const CONTENT_START_X: f32 = 250.0;
const TEXT_MAX_WIDTH: f32 = 430.0;
const FONT_SIZE: f32 = 16.0;
const FONT_WEIGHT: u16 = 700;
const FONT_FAMILY: &str = "\"trebuchet ms\", verdana, arial, sans-serif";
const SVG_PADDING: f32 = 30.0;

#[derive(Debug, Clone)]
struct TextProps {
    html: String,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone)]
struct SwimlaneProps {
    index: i32,
    label: String,
}

pub fn compute_eventmodeling_layout(
    graph: &Graph,
    _theme: &Theme,
    _config: &LayoutConfig,
) -> Layout {
    let mut boxes: Vec<EventModelingBoxLayout> = Vec::new();
    let mut swimlanes: BTreeMap<i32, EventModelingSwimlaneLayout> = BTreeMap::new();
    let mut relations = Vec::new();
    let mut previous_swimlane_number: Option<i32> = None;
    let mut max_r = 0.0_f32;

    for (index, frame) in graph.eventmodeling.frames.iter().enumerate() {
        let text = calculate_text_props(frame, graph);
        let swimlane_props = calculate_swimlane_props(frame, &swimlanes);
        swimlanes
            .entry(swimlane_props.index)
            .or_insert_with(|| EventModelingSwimlaneLayout {
                index: swimlane_props.index,
                label: swimlane_props.label.clone(),
                namespace: None,
                r: 0.0,
                y: swimlane_props.index as f32 * SWIMLANE_MIN_HEIGHT + SWIMLANE_GAP,
                height: SWIMLANE_MIN_HEIGHT,
                max_height: SWIMLANE_MIN_HEIGHT,
            });

        let dimension_width = (text.width + 2.0 * BOX_PADDING).clamp(BOX_MIN_WIDTH, BOX_MAX_WIDTH)
            + 2.0 * BOX_PADDING;
        let dimension_height = (text.height + 2.0 * BOX_PADDING)
            .clamp(BOX_MIN_HEIGHT, BOX_MAX_HEIGHT)
            + 2.0 * BOX_PADDING;

        let last_box = boxes.last();
        let previous_swimlane =
            previous_swimlane_number.and_then(|idx| swimlanes.get(&idx).cloned());
        let current_swimlane = swimlanes
            .get(&swimlane_props.index)
            .cloned()
            .unwrap_or_else(|| EventModelingSwimlaneLayout {
                index: swimlane_props.index,
                label: swimlane_props.label.clone(),
                namespace: None,
                r: 0.0,
                y: 0.0,
                height: SWIMLANE_MIN_HEIGHT,
                max_height: SWIMLANE_MIN_HEIGHT,
            });
        let x = calculate_x(&current_swimlane, previous_swimlane.as_ref(), last_box);
        let r = x + dimension_width + BOX_PADDING;
        max_r = max_r.max(calculate_max_right(&swimlanes, r));

        let swimlane = swimlanes
            .get_mut(&swimlane_props.index)
            .expect("eventmodeling swimlane should exist");
        swimlane.r = x + dimension_width;
        swimlane.max_height = swimlane.max_height.max(dimension_height);
        swimlane.height = swimlane.max_height.max(SWIMLANE_MIN_HEIGHT) + 2.0 * SWIMLANE_PADDING;

        let (fill, stroke) = visual_props(frame.entity_type);
        boxes.push(EventModelingBoxLayout {
            frame_name: frame.name.clone(),
            frame_index: index,
            swimlane_index: swimlane.index,
            x,
            y: swimlane.y + SWIMLANE_PADDING,
            r,
            width: dimension_width,
            height: dimension_height,
            fill: fill.to_string(),
            stroke: stroke.to_string(),
            html: text.html,
        });

        previous_swimlane_number = Some(swimlane_props.index);
        recalculate_swimlane_y(&mut swimlanes);

        for relation in relation_for_frame(index, frame, &boxes, &graph.eventmodeling.frames) {
            relations.push(relation);
        }
    }

    recalculate_swimlane_y(&mut swimlanes);
    update_box_y(&mut boxes, &swimlanes);

    let swimlanes_vec: Vec<_> = swimlanes.values().cloned().collect();
    let content_width = max_r + SWIMLANE_PADDING;
    let content_height = swimlanes_vec
        .last()
        .map(|swimlane| swimlane.y + swimlane.height)
        .unwrap_or(1.0);
    let render_width = (content_width + SVG_PADDING * 2.0).max(1.0);
    let render_height = (content_height + SVG_PADDING * 2.0).max(1.0);

    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width: render_width,
        height: render_height,
        diagram: DiagramData::EventModeling(EventModelingLayout {
            width: render_width,
            height: render_height,
            viewbox_x: -SVG_PADDING,
            viewbox_y: -SVG_PADDING,
            viewbox_width: render_width,
            viewbox_height: render_height,
            max_r,
            swimlanes: swimlanes_vec,
            boxes,
            relations,
            use_max_width: true,
        }),
        acc_title: None,
        acc_descr: None,
    }
}

fn calculate_x(
    swimlane: &EventModelingSwimlaneLayout,
    previous_swimlane: Option<&EventModelingSwimlaneLayout>,
    last_box: Option<&EventModelingBoxLayout>,
) -> f32 {
    let Some(previous_swimlane) = previous_swimlane else {
        return CONTENT_START_X;
    };
    if previous_swimlane.index == swimlane.index && swimlane.r != 0.0 {
        return swimlane.r + BOX_PADDING;
    }
    let Some(last_box) = last_box else {
        return CONTENT_START_X;
    };
    last_box.r - BOX_OVERLAP + BOX_PADDING
}

fn calculate_max_right(
    swimlanes: &BTreeMap<i32, EventModelingSwimlaneLayout>,
    swimlane_r: f32,
) -> f32 {
    swimlanes
        .values()
        .map(|swimlane| swimlane.r)
        .fold(swimlane_r, f32::max)
}

fn recalculate_swimlane_y(swimlanes: &mut BTreeMap<i32, EventModelingSwimlaneLayout>) {
    let mut y = 0.0;
    let mut first = true;
    for swimlane in swimlanes.values_mut() {
        if first {
            swimlane.y = 0.0;
            first = false;
        } else {
            swimlane.y = y;
        }
        y = swimlane.y + swimlane.height + SWIMLANE_GAP;
    }
}

fn update_box_y(
    boxes: &mut [EventModelingBoxLayout],
    swimlanes: &BTreeMap<i32, EventModelingSwimlaneLayout>,
) {
    for box_layout in boxes {
        if let Some(swimlane) = swimlanes.get(&box_layout.swimlane_index) {
            box_layout.y = swimlane.y + SWIMLANE_PADDING;
        }
    }
}

fn relation_for_frame(
    index: usize,
    frame: &EventModelingFrame,
    boxes: &[EventModelingBoxLayout],
    frames: &[EventModelingFrame],
) -> Vec<EventModelingRelationLayout> {
    if frame.reset || (index == 0 && frame.source_frames.is_empty()) {
        return Vec::new();
    }
    let Some(target_box) = find_box_by_frame_name(boxes, &frame.name) else {
        return Vec::new();
    };

    if !frame.source_frames.is_empty() {
        return frame
            .source_frames
            .iter()
            .filter_map(|source_name| {
                find_box_by_frame_name(boxes, source_name).map(|source_box| {
                    EventModelingRelationLayout {
                        source_box,
                        target_box,
                    }
                })
            })
            .collect();
    }

    let target_swimlane = boxes[target_box].swimlane_index;
    let source_box = find_box_by_line_index(boxes, target_swimlane, index.saturating_sub(1));
    if source_box.is_none() && frames.get(index).is_some() {
        return Vec::new();
    }
    source_box
        .map(|source_box| EventModelingRelationLayout {
            source_box,
            target_box,
        })
        .into_iter()
        .collect()
}

fn find_box_by_frame_name(boxes: &[EventModelingBoxLayout], frame_name: &str) -> Option<usize> {
    boxes
        .iter()
        .position(|box_layout| box_layout.frame_name == frame_name)
}

fn find_box_by_line_index(
    boxes: &[EventModelingBoxLayout],
    target_swimlane: i32,
    line_index: usize,
) -> Option<usize> {
    for i in (0..=line_index).rev() {
        let box_layout = boxes.get(i)?;
        if box_layout.swimlane_index != target_swimlane {
            return Some(i);
        }
    }
    None
}

fn calculate_swimlane_props(
    frame: &EventModelingFrame,
    swimlanes: &BTreeMap<i32, EventModelingSwimlaneLayout>,
) -> SwimlaneProps {
    let namespace = extract_namespace(&frame.entity_identifier);
    let existing = find_swimlane_by_namespace(swimlanes, namespace.as_deref());
    match frame.entity_type {
        EventModelingEntityType::Ui | EventModelingEntityType::Processor => {
            if let Some(swimlane) = existing {
                SwimlaneProps {
                    index: swimlane.index,
                    label: swimlane
                        .namespace
                        .clone()
                        .unwrap_or_else(|| "UI/Automation".to_string()),
                }
            } else if let Some(namespace) = namespace {
                SwimlaneProps {
                    index: find_next_available_index(swimlanes, 0, 100),
                    label: format!("UI/A: {namespace}"),
                }
            } else {
                SwimlaneProps {
                    index: 0,
                    label: "UI/Automation".to_string(),
                }
            }
        }
        EventModelingEntityType::ReadModel | EventModelingEntityType::Command => {
            if let Some(swimlane) = existing {
                SwimlaneProps {
                    index: swimlane.index,
                    label: swimlane
                        .namespace
                        .clone()
                        .unwrap_or_else(|| "Command/Read Model".to_string()),
                }
            } else if let Some(namespace) = namespace {
                SwimlaneProps {
                    index: find_next_available_index(swimlanes, 100, 200),
                    label: format!("C/RM: {namespace}"),
                }
            } else {
                SwimlaneProps {
                    index: 100,
                    label: "Command/Read Model".to_string(),
                }
            }
        }
        EventModelingEntityType::Event => {
            if let Some(swimlane) = existing {
                SwimlaneProps {
                    index: swimlane.index,
                    label: swimlane
                        .namespace
                        .clone()
                        .unwrap_or_else(|| "Events".to_string()),
                }
            } else if let Some(namespace) = namespace {
                SwimlaneProps {
                    index: find_next_available_index(swimlanes, 200, 300),
                    label: format!("Stream: {namespace}"),
                }
            } else {
                SwimlaneProps {
                    index: 200,
                    label: "Events".to_string(),
                }
            }
        }
    }
}

fn find_swimlane_by_namespace<'a>(
    swimlanes: &'a BTreeMap<i32, EventModelingSwimlaneLayout>,
    namespace: Option<&str>,
) -> Option<&'a EventModelingSwimlaneLayout> {
    let namespace = namespace.filter(|value| !value.is_empty())?;
    swimlanes
        .values()
        .find(|swimlane| swimlane.namespace.as_deref() == Some(namespace))
}

fn find_next_available_index(
    swimlanes: &BTreeMap<i32, EventModelingSwimlaneLayout>,
    boundary_min: i32,
    boundary_max: i32,
) -> i32 {
    swimlanes
        .keys()
        .filter(|index| **index > boundary_min && **index < boundary_max)
        .copied()
        .fold(boundary_min, i32::max)
        + 1
}

fn visual_props(entity_type: EventModelingEntityType) -> (&'static str, &'static str) {
    match entity_type {
        EventModelingEntityType::Ui => ("white", "#dbdada"),
        EventModelingEntityType::Processor => ("#edb3f6", "#b88cbf"),
        EventModelingEntityType::ReadModel => ("#d3f1a2", "#a3b732"),
        EventModelingEntityType::Command => ("#bcd6fe", "#679ac3"),
        EventModelingEntityType::Event => ("#ffb778", "#c19a0f"),
    }
}

fn calculate_text_props(frame: &EventModelingFrame, graph: &Graph) -> TextProps {
    let name = sanitize_text(&extract_name(&frame.entity_identifier));
    let wrapped_name = wrap_label(&name, TEXT_MAX_WIDTH);
    let mut html = format!("<b>{wrapped_name}</b>");
    let mut measure_html = html.clone();
    let mut rendered_data = frame.data_inline_value.as_deref().map(clean_inline_data);
    let mut data_is_reference = false;

    if let Some(reference) = &frame.data_reference
        && let Some(data) = graph
            .eventmodeling
            .data_entities
            .iter()
            .find(|entity| entity.name == *reference)
    {
        rendered_data = Some(data.value.clone());
        data_is_reference = true;
    }

    let has_data = rendered_data.is_some();
    if let Some(data) = rendered_data {
        let wrapped = wrap_label(&sanitize_text(&data), TEXT_MAX_WIDTH);
        let rendered = html_nonbreaking_spaces(&wrapped);
        let measured = html_entity_spaces(&wrapped);
        let trailing_break = if data_is_reference { "<br/>" } else { "" };
        html.push_str(&format!(
            "<br/><br/><code style=\"text-align: left; display: block;max-width:{TEXT_MAX_WIDTH}px\">{rendered}{trailing_break}</code>"
        ));
        measure_html.push_str(&format!(
            "<br/><br/><code style=\"text-align: left; display: block;max-width:{TEXT_MAX_WIDTH}px\">{measured}{trailing_break}</code>"
        ));
    }

    let dimensions = calculate_text_dimensions(&measure_html);
    let width = if has_data {
        // Mermaid measures the literal HTML source here and applies this
        // temporary divide-by-three hack before rendering it as HTML.
        dimensions.0 / 3.0
    } else {
        dimensions.0
    };
    TextProps {
        html,
        width,
        height: dimensions.1,
    }
}

fn html_nonbreaking_spaces(input: &str) -> String {
    input.replace(' ', "\u{00a0}")
}

fn html_entity_spaces(input: &str) -> String {
    input.replace(' ', "&nbsp;")
}

fn calculate_text_dimensions(html: &str) -> (f32, f32) {
    let lines: Vec<&str> = html.split("<br/>").collect();
    let mut width = 0.0_f32;
    let mut height = 0.0_f32;
    for line in lines {
        let line = if line.is_empty() { "\u{200b}" } else { line };
        let sans = text_metrics::measure_text_width_with_weight(
            line,
            FONT_SIZE,
            "sans-serif",
            FONT_WEIGHT,
        )
        .unwrap_or_else(|| line.chars().count() as f32 * FONT_SIZE * 0.56);
        let preferred =
            text_metrics::measure_text_width_with_weight(line, FONT_SIZE, FONT_FAMILY, FONT_WEIGHT)
                .unwrap_or(sans);
        width = width.max(preferred.round());
        height += 19.0;
    }
    (width, height)
}

fn wrap_label(label: &str, max_width: f32) -> String {
    if label.contains("<br") {
        return label.to_string();
    }
    let words: Vec<_> = label.split(' ').filter(|word| !word.is_empty()).collect();
    if words.is_empty() {
        return label.to_string();
    }
    let mut completed = Vec::new();
    let mut next_line = String::new();
    for (idx, word) in words.iter().enumerate() {
        let word_length = text_width(&format!("{word} "));
        let next_line_length = text_width(&next_line);
        if next_line_length + word_length >= max_width && !next_line.is_empty() {
            completed.push(next_line);
            next_line = (*word).to_string();
        } else {
            if !next_line.is_empty() {
                next_line.push(' ');
            }
            next_line.push_str(word);
        }
        if idx + 1 == words.len() {
            completed.push(next_line.clone());
        }
    }
    completed
        .into_iter()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("<br/>")
}

fn text_width(text: &str) -> f32 {
    let sans =
        text_metrics::measure_text_width_with_weight(text, FONT_SIZE, "sans-serif", FONT_WEIGHT)
            .unwrap_or_else(|| text.chars().count() as f32 * FONT_SIZE * 0.56);
    text_metrics::measure_text_width_with_weight(text, FONT_SIZE, FONT_FAMILY, FONT_WEIGHT)
        .unwrap_or(sans)
}

fn clean_inline_data(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') && trimmed.len() >= 2 {
        return trimmed[1..trimmed.len() - 1].trim().to_string();
    }
    trimmed.to_string()
}

fn sanitize_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn extract_namespace(entity_identifier: &str) -> Option<String> {
    let parts: Vec<_> = entity_identifier.split('.').collect();
    if parts.len() == 2 {
        Some(parts[0].to_string())
    } else {
        None
    }
}

fn extract_name(entity_identifier: &str) -> String {
    let parts: Vec<_> = entity_identifier.split('.').collect();
    if parts.len() == 2 {
        parts[1].to_string()
    } else {
        entity_identifier.to_string()
    }
}
