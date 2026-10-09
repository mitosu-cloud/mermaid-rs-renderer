//! Mermaid's unified C4 shape geometry, shared by measurement and drawing.
use crate::{
    config::C4Config,
    ir::{C4Shape, C4ShapeKind},
    layout::{C4ShapeLayout, C4TextLayout},
};

pub(crate) fn person(kind: C4ShapeKind) -> bool {
    matches!(kind, C4ShapeKind::Person | C4ShapeKind::ExternalPerson)
}
pub(crate) fn database(kind: C4ShapeKind) -> bool {
    matches!(
        kind,
        C4ShapeKind::SystemDb
            | C4ShapeKind::ExternalSystemDb
            | C4ShapeKind::ContainerDb
            | C4ShapeKind::ExternalContainerDb
            | C4ShapeKind::ComponentDb
            | C4ShapeKind::ExternalComponentDb
    )
}
pub(crate) fn queue(kind: C4ShapeKind) -> bool {
    matches!(
        kind,
        C4ShapeKind::SystemQueue
            | C4ShapeKind::ExternalSystemQueue
            | C4ShapeKind::ContainerQueue
            | C4ShapeKind::ExternalContainerQueue
            | C4ShapeKind::ComponentQueue
            | C4ShapeKind::ExternalComponentQueue
    )
}

fn section(
    text: &str,
    font: f32,
    family: &str,
    bold: bool,
    width: f32,
    wrap: bool,
) -> C4TextLayout {
    let advance = |s: &str| {
        crate::text_metrics::measure_styled_text_width(s, font, family, bold, false)
            .unwrap_or(s.chars().count() as f32 * font * 0.5)
    };
    let mut lines = Vec::new();
    for raw in text
        .replace("<br/>", "\n")
        .replace("<br>", "\n")
        .split('\n')
    {
        let mut line = String::new();
        for word in raw.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if wrap && !line.is_empty() && advance(&candidate) > width {
                lines.push(line);
                line = word.to_string();
            } else {
                line = candidate;
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
    }
    let height = if lines.is_empty() {
        0.0
    } else {
        crate::text_metrics::svg_text_height(font, family).unwrap_or(font * 1.35)
            + (lines.len() - 1) as f32 * font * 1.1
    };
    let width = lines.iter().map(|s| advance(s)).fold(0.0_f32, f32::max);
    C4TextLayout {
        text: text.into(),
        lines,
        width,
        height,
        y: 0.0,
    }
}

pub(crate) fn measure(shape: &C4Shape, conf: &C4Config, font: f32, family: &str) -> C4ShapeLayout {
    let padding = conf.c4_shape_padding;
    let wrap_width =
        (conf.width - if database(shape.kind) { padding } else { 0.0 } - 2.0 * padding).max(32.0);
    let kind = shape
        .kind
        .as_str()
        .trim_start_matches("external_")
        .trim_end_matches("_db")
        .trim_end_matches("_queue");
    let stereotype = match kind {
        "person" => "Person",
        "system" => "Software System",
        "container" => "Container",
        "component" => "Component",
        other => other,
    };
    let type_text = if let Some(techn) = &shape.techn {
        format!("[{stereotype}: {techn}]")
    } else {
        format!("[{stereotype}]")
    };
    let mut name = section(&shape.label, font, family, true, wrap_width, conf.wrap);
    let mut typ = section(
        &type_text,
        font * 0.75,
        family,
        false,
        wrap_width,
        conf.wrap,
    );
    let mut description = shape
        .descr
        .as_ref()
        .filter(|s| !s.is_empty())
        .map(|s| section(s, font * 0.82, family, false, wrap_width, conf.wrap));
    let mut top = 0.0;
    for text in std::iter::once(&mut name)
        .chain(std::iter::once(&mut typ))
        .chain(description.iter_mut())
    {
        text.y = top;
        top += text.height + 3.0;
    }
    let label_height = (top - 3.0).max(0.0);
    let label_width = name
        .width
        .max(typ.width)
        .max(description.as_ref().map(|t| t.width).unwrap_or(0.0));
    let mut width = conf.width.max(label_width + 2.0 * padding);
    let mut height = label_height + 2.0 * padding;
    let mut label_top = padding;
    if person(shape.kind) {
        let r = (width * 0.23).clamp(16.0, 56.0);
        height += r * 1.73;
        label_top += r * 1.73;
    } else if database(shape.kind) {
        width = conf.width.max(label_width + padding);
        let ry = width / 2.0 / (2.5 + width / 50.0);
        height = label_height + padding + 3.0 * ry;
        label_top = (height - label_height) / 2.0 + padding / 1.5;
    } else if queue(shape.kind) {
        width = conf.width.max(label_width + 2.0 * padding);
    }
    name.y += label_top;
    typ.y += label_top;
    if let Some(t) = &mut description {
        t.y += label_top;
    }
    C4ShapeLayout {
        id: shape.id.clone(),
        kind: shape.kind,
        bg_color: shape.bg_color.clone(),
        border_color: shape.border_color.clone(),
        font_color: shape.font_color.clone(),
        x: 0.0,
        y: 0.0,
        width,
        height,
        margin: conf.c4_shape_margin,
        type_label: section("", font, family, false, wrap_width, false),
        label: name,
        type_or_techn: Some(typ),
        descr: description,
        image_y: None,
    }
}

pub(crate) fn intersect(node: &C4ShapeLayout, target: (f32, f32)) -> (f32, f32) {
    let center = (node.x + node.width / 2.0, node.y + node.height / 2.0);
    let d = (target.0 - center.0, target.1 - center.1);
    if d.0.abs() + d.1.abs() < 0.0001 {
        return center;
    }
    let t = (node.width / 2.0 / d.0.abs()).min(node.height / 2.0 / d.1.abs());
    let mut p = (center.0 + d.0 * t, center.1 + d.1 * t);
    if database(node.kind) {
        let rx = node.width / 2.0;
        let ry = rx / (2.5 + node.width / 50.0);
        let x = p.0 - center.0;
        if x.abs() < rx || (x.abs() == rx && (p.1 - center.1).abs() > node.height / 2.0 - ry) {
            let y = ry - (ry * ry * (1.0 - x * x / (rx * rx))).max(0.0).sqrt();
            p.1 += if d.1 > 0.0 { -y } else { y };
        }
    } else if person(node.kind) {
        let r = (node.width * 0.23).clamp(16.0, 56.0);
        let body_top = node.y + r * 1.73;
        let body_height = node.height - r * 1.73;
        let br = (node.width * 0.177).min(body_height * 0.45);
        let mut paths = Vec::new();
        let mut body = Vec::new();
        for (cx, cy, angle) in [
            (node.x + br, body_top + br, 180.0_f32),
            (node.x + node.width - br, body_top + br, 270.0),
            (node.x + node.width - br, node.y + node.height - br, 0.0),
            (node.x + br, node.y + node.height - br, 90.0),
        ] {
            for i in 0..=12 {
                let a = (angle + i as f32 * 90.0 / 12.0).to_radians();
                body.push((cx + br * a.cos(), cy + br * a.sin()));
            }
        }
        paths.push(body);
        paths.push(
            (0..=48)
                .map(|i| {
                    let a = i as f32 * std::f32::consts::TAU / 48.0;
                    (center.0 + r * a.cos(), node.y + r + r * a.sin())
                })
                .collect(),
        );
        let mut furthest = 0.0_f32;
        for outline in paths {
            for i in 0..outline.len() {
                let a = outline[i];
                let b = outline[(i + 1) % outline.len()];
                let v = (b.0 - a.0, b.1 - a.1);
                let den = d.0 * v.1 - d.1 * v.0;
                if den.abs() < 0.0001 {
                    continue;
                }
                let q = (a.0 - center.0, a.1 - center.1);
                let t = (q.0 * v.1 - q.1 * v.0) / den;
                let u = (q.0 * d.1 - q.1 * d.0) / den;
                if t >= 0.0 && (0.0..=1.0).contains(&u) {
                    furthest = furthest.max(t);
                }
            }
        }
        p = (center.0 + d.0 * furthest, center.1 + d.1 * furthest);
    }
    p
}
