//! Sequence SVG painting from the shared Mermaid cursor layout.
use super::{Theme, escape_xml};
use crate::{
    ir::{NodeShape, SequenceArrowHead, SequenceFrameKind},
    layout::{Layout, NodeLayout, SequenceData, TextBlock},
    sequence,
};
use std::fmt::Write;

fn text(
    svg: &mut String,
    block: &TextBlock,
    x: f32,
    y: f32,
    baseline: &str,
    color: &str,
    size: f32,
    step: f32,
) {
    for (i, line) in block.lines.iter().enumerate() {
        let _ = write!(
            svg,
            "<text x=\"{x:.2}\" y=\"{:.2}\" text-anchor=\"middle\" {} fill=\"{}\" font-size=\"{size:.2}\" font-weight=\"400\">{}</text>",
            y + i as f32 * step,
            if baseline.is_empty() {
                String::new()
            } else {
                format!("dominant-baseline=\"{baseline}\" alignment-baseline=\"{baseline}\"")
            },
            escape_xml(color),
            escape_xml(&line.text())
        );
    }
}
fn centered_text(svg: &mut String, node: &NodeLayout, x: f32, y: f32, theme: &Theme) {
    text(
        svg,
        &node.label,
        x,
        y - theme.font_size * (node.label.lines.len().saturating_sub(1)) as f32 / 2.0,
        "central",
        &theme.text_color,
        theme.font_size,
        theme.font_size,
    );
}
fn actor(svg: &mut String, node: &NodeLayout, theme: &Theme, neo: bool, footer: bool) {
    let (x, y, w, h) = (node.x, node.y, node.width, node.height);
    let cx = x + w / 2.0;
    let fill = node
        .style
        .fill
        .as_deref()
        .unwrap_or(&theme.sequence_actor_fill);
    let stroke = node
        .style
        .stroke
        .as_deref()
        .unwrap_or(&theme.sequence_actor_border);
    let shadow = if neo {
        " filter=\"url(#sequence-shadow)\""
    } else {
        ""
    };
    let glyph_bottom = if footer {
        y + 50.0
    } else {
        y + h - 12.0 - node.label.height
    };
    let label_center = if footer {
        glyph_bottom + 6.0 + node.label.height / 2.0
    } else {
        y + h - 6.0 - node.label.height / 2.0
    };
    let _ = write!(
        svg,
        "<g data-id=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\">",
        escape_xml(&node.id),
        escape_xml(fill),
        escape_xml(stroke),
        if neo { 2 } else { 1 }
    );
    match node.shape {
        NodeShape::StickFigure => {
            let scale = if neo { 44.0 / 65.0 } else { 1.0 };
            let gy = |v: f32| {
                if neo {
                    glyph_bottom - (60.0 - v) * scale
                } else {
                    y + v
                }
            };
            let _ = write!(
                svg,
                "<g stroke-width=\"2\"><path d=\"M {cx},{a} V {b} M {left},{arms} H {right} M {left},{feet} L {cx},{b} L {right2},{feet}\" fill=\"none\"/><circle cx=\"{cx}\" cy=\"{head}\" r=\"{radius}\"/></g>",
                a = gy(25.0),
                b = gy(45.0),
                left = cx - 18.0 * scale,
                right = cx + 18.0 * scale,
                right2 = cx + 16.0 * scale,
                arms = gy(33.0),
                feet = gy(60.0),
                head = gy(10.0),
                radius = 15.0 * scale
            );
            centered_text(svg, node, cx, label_center, theme);
        }
        NodeShape::Boundary | NodeShape::Control | NodeShape::Entity => {
            let cy = if neo { glyph_bottom - 22.0 } else { y + 25.0 };
            let _ = write!(
                svg,
                "<g stroke-width=\"2\"{shadow}><circle cx=\"{cx}\" cy=\"{cy}\" r=\"22\"/>"
            );
            if node.shape == NodeShape::Boundary {
                let _ = write!(
                    svg,
                    "<path d=\"M {},{} H {} M {},{} V {}\" fill=\"none\"/>",
                    cx - 55.0,
                    cy,
                    cx - 15.0,
                    cx - 55.0,
                    cy - 10.0,
                    cy + 10.0
                );
            } else if node.shape == NodeShape::Entity {
                let _ = write!(
                    svg,
                    "<path d=\"M {},{} H {}\" fill=\"none\"/>",
                    cx - 22.0,
                    cy + 22.0,
                    cx + 22.0
                );
            } else {
                // Mermaid attaches a marker to a zero-length line. Express its
                // rotation and stroke-width scaling as a group so each actor
                // keeps its own palette without duplicate global marker IDs.
                let _ = write!(
                    svg,
                    "<g transform=\"translate({cx},{}) rotate(172.5) scale(2) translate(-11,-5.8)\"><path d=\"M 14.4 5.6 L 7.2 10.4 L 8.8 5.6 L 7.2 0.8 Z\" stroke-width=\"1.2\"/></g>",
                    cy - 22.0
                );
            }
            svg.push_str("</g>");
            centered_text(svg, node, cx, label_center, theme);
        }
        NodeShape::Cylinder => {
            let cw = w / 3.0;
            let rx = cw / 2.0;
            let ry = rx / (2.5 + cw / 50.0);
            let height = if neo { 44.0 } else { cw };
            let left = cx - cw / 2.0;
            let top = if neo {
                glyph_bottom - height + ry
            } else {
                y + 2.0 * ry
            };
            let _ = write!(
                svg,
                "<path d=\"M {left},{top} a {rx},{ry} 0 0 0 {cw},0 a {rx},{ry} 0 0 0 -{cw},0 l 0,{body} a {rx},{ry} 0 0 0 {cw},0 l 0,-{body}\"{shadow}/>",
                body = height - 2.0 * ry
            );
            centered_text(svg, node, cx, label_center, theme);
        }
        NodeShape::Collections => {
            let height = if neo { h - 6.0 } else { h };
            let _ = write!(
                svg,
                "<g{shadow}><rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{height}\"/><rect x=\"{}\" y=\"{}\" width=\"{w}\" height=\"{height}\"/></g>",
                x - 6.0,
                y + 6.0
            );
            centered_text(svg, node, cx - 6.0, y + 6.0 + height / 2.0, theme);
        }
        NodeShape::Queue => {
            let ry = h / 2.0;
            let rx = ry / (2.5 + h / 50.0);
            let left = x + rx;
            let width = w - 2.0 * rx;
            let _ = write!(
                svg,
                "<path d=\"M {left},{y} a {rx},{ry} 0 0 0 0,{h} h {width} a {rx},{ry} 0 0 0 0,-{h} Z\"{shadow}/><path d=\"M {},{y} a {rx},{ry} 0 0 0 0,{h}\" fill=\"none\" stroke-width=\"1.5\"/>",
                x + w - rx
            );
            centered_text(svg, node, cx, y + h / 2.0, theme);
        }
        _ => {
            let radius = if neo { 6 } else { 3 };
            let _ = write!(
                svg,
                "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"{radius}\" ry=\"{radius}\"{shadow}/>"
            );
            centered_text(svg, node, cx, y + h / 2.0, theme);
        }
    }
    svg.push_str("</g>");
}
fn marker(head: SequenceArrowHead) -> Option<&'static str> {
    match head {
        SequenceArrowHead::Filled => Some("sequence-arrow"),
        SequenceArrowHead::Open => Some("sequence-point"),
        SequenceArrowHead::Cross => Some("sequence-cross"),
        SequenceArrowHead::None => None,
    }
}
pub(super) fn render(layout: &Layout, data: &SequenceData, theme: &Theme) -> String {
    let options = data.appearance.as_ref().unwrap();
    let neo = sequence::neo(options);
    let (vx, vy, w, h) = data.viewbox.unwrap();
    let mut svg = String::new();
    let _ = write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.2}\" height=\"{h:.2}\" viewBox=\"{vx:.2} {vy:.2} {w:.2} {h:.2}\" role=\"img\"><style>"
    );
    if theme.font_family.contains("Recursive") {
        svg.push_str(include_str!("../fonts/Recursive.css"));
    }
    let _ = write!(
        svg,
        "text{{font-family:{};stroke:none}}</style><rect x=\"{vx}\" y=\"{vy}\" width=\"{w}\" height=\"{h}\" fill=\"{}\"/><defs><filter id=\"sequence-shadow\" height=\"130%\" width=\"130%\"><feDropShadow dx=\"4\" dy=\"4\" stdDeviation=\"0\" flood-opacity=\"0.06\"/></filter>",
        escape_xml(&theme.font_family),
        escape_xml(&theme.background)
    );
    let color = escape_xml(&theme.line_color);
    let _ = write!(
        svg,
        "<marker id=\"sequence-arrow\" refX=\"7.9\" refY=\"5\" markerUnits=\"userSpaceOnUse\" markerWidth=\"12\" markerHeight=\"12\" orient=\"auto-start-reverse\"><path d=\"M -1 0 L 10 5 L 0 10 Z\" fill=\"{color}\" stroke=\"{color}\"/></marker><marker id=\"sequence-point\" refX=\"15.5\" refY=\"7\" markerWidth=\"20\" markerHeight=\"28\" orient=\"auto-start-reverse\"><path d=\"M 18,7 L9,13 L14,7 L9,1 Z\"/></marker><marker id=\"sequence-cross\" markerWidth=\"15\" markerHeight=\"8\" orient=\"auto-start-reverse\" refX=\"4\" refY=\"4.5\"><path d=\"M 1,2 L 6,7 M 6,2 L 1,7\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1pt\"/></marker></defs>"
    );
    let metrics = sequence::Metrics::new(options, theme);
    // Backgrounds precede lifelines and messages just as the lowered JS groups do.
    for group in &data.boxes {
        let _ = write!(
            svg,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"rgba(0,0,0,0.5)\"/>",
            group.x,
            group.y,
            group.width,
            group.height,
            escape_xml(group.color.as_deref().unwrap_or("transparent"))
        );
        if let Some(label) = &group.label {
            text(
                &mut svg,
                label,
                group.x + group.width / 2.0,
                group.y
                    + sequence::number(options, "boxTextMargin", 5.0)
                    + label.height / 2.0
                    + 5.0,
                "",
                &theme.text_color,
                theme.font_size,
                metrics.painted_line_height,
            );
        }
    }
    for frame in data
        .frames
        .iter()
        .rev()
        .filter(|f| f.kind == SequenceFrameKind::Rect)
    {
        let _ = write!(
            svg,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            frame.x,
            frame.y,
            frame.width,
            frame.height,
            escape_xml(
                frame
                    .fill_color
                    .as_deref()
                    .unwrap_or(&theme.sequence_actor_fill)
            )
        );
    }
    for line in &data.lifelines {
        let _ = write!(
            svg,
            "<line class=\"actor-line\" x1=\"{}\" x2=\"{}\" y1=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
            line.x,
            line.x,
            line.y1,
            line.y2,
            escape_xml(&theme.sequence_actor_line)
        );
    }
    for activation in &data.activations {
        let node = &layout.nodes[&activation.participant];
        let _ = write!(
            svg,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            activation.x,
            activation.y,
            activation.width,
            activation.height,
            escape_xml(
                node.style
                    .fill
                    .as_deref()
                    .unwrap_or(&theme.sequence_activation_fill)
            ),
            escape_xml(
                node.style
                    .stroke
                    .as_deref()
                    .unwrap_or(&theme.sequence_activation_border)
            )
        );
    }
    for note in &data.notes {
        let _ = write!(
            svg,
            "<rect class=\"note\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\"/>",
            note.x,
            note.y,
            note.width,
            note.height,
            escape_xml(&theme.sequence_note_fill),
            escape_xml(&theme.sequence_note_border)
        );
        let note_margin = sequence::number(options, "noteMargin", 10.0);
        text(
            &mut svg,
            &note.label,
            (note.x + note.width / 2.0).round(),
            (note.y + note_margin / 2.0).round() + theme.font_size,
            "middle",
            &theme.text_color,
            theme.font_size,
            metrics.painted_line_height,
        );
    }
    let border = sequence::variable(options, "labelBoxBorderColor", &theme.sequence_actor_border);
    let label_fill = sequence::variable(options, "labelBoxBkgColor", &theme.sequence_actor_fill);
    for frame in data
        .frames
        .iter()
        .filter(|f| f.kind != SequenceFrameKind::Rect)
    {
        let _ = write!(
            svg,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\" stroke-dasharray=\"2,2\"/>",
            frame.x,
            frame.y,
            frame.width,
            frame.height,
            escape_xml(&border)
        );
        for y in &frame.dividers {
            let _ = write!(
                svg,
                "<line x1=\"{}\" x2=\"{}\" y1=\"{y}\" y2=\"{y}\" stroke=\"{}\" stroke-width=\"2\" stroke-dasharray=\"3,3\"/>",
                frame.x,
                frame.x + frame.width,
                escape_xml(&border)
            );
        }
        let (x, y, w, h) = frame.label_box;
        let _ = write!(
            svg,
            "<polygon points=\"{x},{y} {},{y} {},{} {},{} {x},{}\" fill=\"{}\" stroke=\"{}\"{}/>",
            x + w,
            x + w,
            y + h - 7.0,
            x + w - 8.4,
            y + h,
            y + h,
            escape_xml(&label_fill),
            escape_xml(&border),
            ""
        );
        let name = match frame.kind {
            SequenceFrameKind::Alt => "alt",
            SequenceFrameKind::Opt => "opt",
            SequenceFrameKind::Loop => "loop",
            SequenceFrameKind::Par => "par",
            SequenceFrameKind::Critical => "critical",
            SequenceFrameKind::Break => "break",
            SequenceFrameKind::Rect => "",
        };
        let label = TextBlock {
            lines: vec![crate::layout::TextLine::plain(name.into())],
            width: w,
            height: theme.font_size,
        };
        text(
            &mut svg,
            &label,
            (x + w / 2.0).round(),
            (y + h / 2.0 + 2.5).round(),
            "middle",
            &theme.text_color,
            theme.font_size,
            theme.font_size,
        );
        if frame.label.text.lines.iter().any(|l| !l.text().is_empty()) {
            text(
                &mut svg,
                &frame.label.text,
                frame.label.x,
                (frame.label.y + 2.5).round(),
                "",
                &theme.text_color,
                theme.font_size,
                metrics.painted_line_height,
            );
        }
        for label in &frame.section_labels {
            if label.text.lines.iter().any(|l| !l.text().is_empty()) {
                text(
                    &mut svg,
                    &label.text,
                    label.x,
                    (label.y + 2.5).round(),
                    "",
                    &theme.text_color,
                    theme.font_size,
                    metrics.painted_line_height,
                );
            }
        }
    }
    for node in layout.nodes.values() {
        actor(&mut svg, node, theme, neo, false);
    }
    for edge in &layout.edges {
        let mut start = edge.points[0];
        let end = *edge.points.last().unwrap();
        if !data.numbers.is_empty() && edge.from != edge.to {
            start.0 += if edge.arrow_start {
                if start.0 < end.0 { 12.0 } else { -6.0 }
            } else {
                6.0
            };
        }
        let dashed = if edge.style == crate::ir::EdgeStyle::Dotted {
            " stroke-dasharray=\"3,3\""
        } else {
            ""
        };
        let mut markers = String::new();
        if edge.arrow_end
            && let Some(head) = marker(edge.sequence_arrow_end.unwrap_or(SequenceArrowHead::Filled))
        {
            let _ = write!(markers, " marker-end=\"url(#{head})\"");
        }
        if edge.arrow_start
            && let Some(head) = marker(
                edge.sequence_arrow_start
                    .unwrap_or(SequenceArrowHead::Filled),
            )
        {
            let _ = write!(markers, " marker-start=\"url(#{head})\"");
        }
        if edge.from == edge.to && edge.points.len() == 4 {
            let a = edge.points[1];
            let b = edge.points[2];
            let path = if edge.curve.is_some() {
                format!(
                    "M {},{} C {},{} {},{} {},{}",
                    start.0, start.1, a.0, a.1, b.0, b.1, end.0, end.1
                )
            } else {
                format!(
                    "M {},{} H {} V {} H {}",
                    start.0,
                    start.1,
                    start.0 + 75.0,
                    start.1 + 25.0,
                    start.0
                )
            };
            let _ = write!(
                svg,
                "<path d=\"{path}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1.5\"{dashed}{markers}/>"
            );
        } else {
            let _ = write!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{color}\" stroke-width=\"1.5\"{dashed}{markers}/>",
                start.0, start.1, end.0, end.1
            );
        }
        for (marked, id, source) in [
            (edge.start_decoration, &edge.from, true),
            (edge.end_decoration, &edge.to, false),
        ] {
            if marked == Some(crate::ir::EdgeDecoration::Circle) {
                let node = &layout.nodes[id];
                let offset = if source && !data.numbers.is_empty() {
                    let from = &layout.nodes[&edge.from];
                    let to = &layout.nodes[&edge.to];
                    if from.x <= to.x { 16.5 } else { -16.5 }
                } else {
                    0.0
                };
                let _ = write!(
                    svg,
                    "<circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{color}\"/>",
                    node.x + node.width / 2.0 + offset,
                    start.1
                );
            }
        }
        if let (Some(label), Some((x, y))) = (&edge.label, edge.label_anchor) {
            text(
                &mut svg,
                label,
                x,
                y,
                "middle",
                &theme.text_color,
                theme.font_size,
                metrics.painted_line_height,
            );
        }
    }
    for number in &data.numbers {
        let _ = write!(
            svg,
            "<circle cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"{color}\"/><text x=\"{}\" y=\"{}\" text-anchor=\"middle\" fill=\"white\" font-size=\"{}\" style=\"font-family:sans-serif\">{}</text>",
            number.x,
            number.y,
            number.x,
            number.y + 4.0,
            if number.value.to_string().len() > 5 {
                7
            } else if number.value.to_string().len() > 3 {
                9
            } else {
                12
            },
            number.value
        );
    }
    for node in &data.footboxes {
        actor(&mut svg, node, theme, neo, true);
    }
    for (x, y) in &data.destroy_markers {
        let _ = write!(
            svg,
            "<path d=\"M {},{} l 10,10 m -10,0 l 10,-10\" fill=\"none\" stroke=\"{color}\" stroke-width=\"2\"/>",
            x - 5.0,
            y - 5.0
        );
    }
    if let Some(title) = options.get("title").and_then(|v| v.as_str()) {
        let label = TextBlock {
            lines: vec![crate::layout::TextLine::plain(title.into())],
            width: w,
            height: theme.font_size,
        };
        text(
            &mut svg,
            &label,
            (w - 100.0) / 2.0 - 100.0,
            -25.0,
            "",
            &theme.text_color,
            theme.font_size,
            theme.font_size,
        );
    }
    svg.push_str("</svg>");
    svg
}
