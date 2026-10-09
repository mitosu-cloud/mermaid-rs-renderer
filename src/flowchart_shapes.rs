//! Geometry shared by measurement and painting for cloud and brace symbols.
use crate::{
    ir::{DiagramKind, Graph, NodeShape, NodeStyle},
    layout::{NodeLayout, TextBlock},
    theme::Theme,
};
use serde_json::Value;

mod documents;

pub(crate) fn uses_scoped_theme(graph: &Graph) -> bool {
    matches!(graph.kind, DiagramKind::Er | DiagramKind::Flowchart)
        || crate::class_lollipop::enabled(graph)
}

pub(crate) fn theme_for_graph(graph: &Graph, theme: &Theme) -> Option<Theme> {
    uses_scoped_theme(graph).then(|| crate::agentflow::theme(&graph.appearance_config, theme))
}

pub(crate) fn node_style(graph: &Graph, id: &str) -> NodeStyle {
    let mut style = NodeStyle::default();
    if graph.kind != DiagramKind::Flowchart || !crate::usecase::redux(&graph.appearance_config) {
        return style;
    }
    let color = |key: &str, default: &str| {
        graph
            .appearance_config
            .get("themeVariables")
            .and_then(|v| v.get(key))
            .and_then(Value::as_str)
            .unwrap_or(default)
            .to_string()
    };
    style.fill = Some(color("mainBkg", "#ffffff"));
    style.stroke = Some(color("nodeBorder", "#28253D"));
    // Redux's CSS overrides roughjs's presentation attributes, including the
    // 1.3px path stroke and stateEnd's black stroke / filled inner circle.
    style.stroke_width = Some(2.0);
    if graph
        .nodes
        .get(id)
        .is_some_and(|n| n.shape == NodeShape::Rectangle)
    {
        style.corner_radius = Some(0.0);
    }
    style
}

fn neo(options: &Value) -> bool {
    options.get("look").and_then(Value::as_str).unwrap_or("neo") == "neo"
}

struct Cloud {
    width: f32,
    height: f32,
    bounds: [f32; 4],
    path: String,
}

/// SVG endpoint-to-center conversion, including arc extrema. Measuring the
/// actual ten arcs prevents labels and neighboring nodes using a smaller box.
fn cloud(label: &TextBlock) -> Cloud {
    let w = label.width.max(120.0) + 15.0;
    let h = label.height + 15.0;
    let arcs = [
        (0.15 * w, 0.15 * w, 0.25 * w, -0.1 * w, false),
        (0.35 * w, 0.35 * w, 0.4 * w, -0.1 * w, true),
        (0.25 * w, 0.25 * w, 0.35 * w, 0.2 * w, true),
        (0.15 * w, 0.15 * w, 0.15 * w, 0.35 * h, true),
        (0.2 * w, 0.2 * w, -0.15 * w, 0.65 * h, true),
        (0.25 * w, 0.15 * w, -0.25 * w, 0.15 * w, true),
        (0.35 * w, 0.35 * w, -0.5 * w, 0.0, true),
        (0.15 * w, 0.15 * w, -0.25 * w, -0.15 * w, true),
        (0.15 * w, 0.15 * w, -0.1 * w, -0.35 * h, true),
        (0.2 * w, 0.2 * w, 0.1 * w, -0.65 * h, true),
    ];
    let mut path = String::from("M 0 0");
    let (mut x, mut y) = (0.0_f32, 0.0_f32);
    let mut bounds = [0.0_f32; 4];
    for (mut rx, mut ry, dx, dy, rotated) in arcs {
        path.push_str(&format!(
            " a {rx:.5},{ry:.5} {} 0,1 {dx:.5},{dy:.5}",
            if rotated { 1 } else { 0 }
        ));
        // The third SVG arc parameter is a one-degree rotation, not the
        // large-arc flag. Every Mermaid cloud lobe uses the short clockwise arc.
        let rotation = if rotated { 1.0_f32.to_radians() } else { 0.0 };
        let (sin, cos) = rotation.sin_cos();
        let (px, py) = (
            -dx * cos / 2.0 - dy * sin / 2.0,
            dx * sin / 2.0 - dy * cos / 2.0,
        );
        let scale = (px * px / (rx * rx) + py * py / (ry * ry)).sqrt().max(1.0);
        rx *= scale;
        ry *= scale;
        let denominator = rx * rx * py * py + ry * ry * px * px;
        let factor = if denominator > 0.0 {
            ((rx * rx * ry * ry - denominator).max(0.0) / denominator).sqrt()
        } else {
            0.0
        };
        let (cx, cy) = (factor * rx * py / ry, -factor * ry * px / rx);
        let center = (
            x + dx / 2.0 + cx * cos - cy * sin,
            y + dy / 2.0 + cx * sin + cy * cos,
        );
        let start = ((py - cy) / ry).atan2((px - cx) / rx);
        let end = ((-py - cy) / ry).atan2((-px - cx) / rx);
        let sweep = (end - start).rem_euclid(std::f32::consts::TAU);
        let mut include = |a: f32| {
            let p = (
                center.0 + rx * a.cos() * cos - ry * a.sin() * sin,
                center.1 + rx * a.cos() * sin + ry * a.sin() * cos,
            );
            bounds[0] = bounds[0].min(p.0);
            bounds[1] = bounds[1].min(p.1);
            bounds[2] = bounds[2].max(p.0);
            bounds[3] = bounds[3].max(p.1);
        };
        include(start);
        include(end);
        let extrema = [(-ry * sin).atan2(rx * cos), (ry * cos).atan2(rx * sin)];
        for a in extrema
            .into_iter()
            .flat_map(|a| [a, a + std::f32::consts::PI])
        {
            if (a - start).rem_euclid(std::f32::consts::TAU) <= sweep + 0.0001 {
                include(a);
            }
        }
        x += dx;
        y += dy;
    }
    path.push_str(" H 0 V 0 Z");
    Cloud {
        width: w,
        height: h,
        bounds,
        path,
    }
}

pub(crate) fn size(shape: NodeShape, label: &TextBlock, options: &Value) -> Option<(f32, f32)> {
    if let Some(geometry) = documents::geometry(shape, label, neo(options)) {
        return Some((
            geometry.bounds[2] - geometry.bounds[0],
            geometry.bounds[3] - geometry.bounds[1],
        ));
    }
    match shape {
        NodeShape::Hourglass => Some((30.0, 30.0)),
        NodeShape::LightningBolt => Some((35.0, 70.0)),
        NodeShape::SmallCircle | NodeShape::FilledCircle | NodeShape::FramedCircle => {
            Some((14.0, 14.0))
        }
        NodeShape::CrossedCircle => Some((60.0, 60.0)),
        NodeShape::DoubleCircle => {
            let diameter = label.width.hypot(label.height) + if neo(options) { 56.0 } else { 40.0 };
            Some((diameter, diameter))
        }
        NodeShape::Rectangle | NodeShape::RoundRect => Some((
            label.width + if neo(options) { 32.0 } else { 60.0 },
            label.height + if neo(options) { 24.0 } else { 30.0 },
        )),
        NodeShape::Cloud => {
            let c = cloud(label);
            Some((c.bounds[2] - c.bounds[0], c.bounds[3] - c.bounds[1]))
        }
        NodeShape::Comment | NodeShape::BraceLeft | NodeShape::BraceRight => {
            let left = shape == NodeShape::BraceLeft;
            let w = label.width.max(120.0)
                + if neo(options) {
                    if left { 18.0 } else { 36.0 }
                } else {
                    15.0
                };
            let h = label.height
                + if neo(options) {
                    if left { 12.0 } else { 24.0 }
                } else {
                    15.0
                };
            let r = (h * 0.1).max(5.0);
            Some((
                w + if shape == NodeShape::Comment {
                    r * 2.5
                } else if left {
                    (w * 0.1).max(r * 2.0)
                } else {
                    r * 2.0
                },
                h + r * 2.0,
            ))
        }
        _ => None,
    }
}

pub(crate) fn label_center(node: &NodeLayout, options: &Value) -> Option<(f32, f32)> {
    if let Some(geometry) = documents::geometry(node.shape, &node.label, neo(options)) {
        return Some((
            node.x - geometry.bounds[0] + geometry.label.0,
            node.y - geometry.bounds[1] + geometry.label.1,
        ));
    }
    if matches!(
        node.shape,
        NodeShape::Comment | NodeShape::BraceLeft | NodeShape::BraceRight
    ) {
        let left = node.shape == NodeShape::BraceLeft;
        let px = if neo(options) {
            if left { 18.0 } else { 36.0 }
        } else {
            15.0
        };
        let py = if neo(options) {
            if left { 12.0 } else { 24.0 }
        } else {
            15.0
        };
        let body_w = node.label.width.max(120.0) + px;
        let r = ((node.label.height + py) * 0.1).max(5.0);
        let dx = if left {
            (body_w * 0.1).max(r * 2.0) / 2.0 - px / 2.0
        } else {
            7.5 - px / 2.0
        };
        return Some((
            node.x + node.width / 2.0 + dx,
            node.y + node.height / 2.0 + 7.5 - py / 2.0,
        ));
    }
    if node.shape != NodeShape::Cloud {
        return None;
    }
    let c = cloud(&node.label);
    Some((
        node.x - c.bounds[0] + c.width / 2.0,
        node.y - c.bounds[1] + c.height / 2.0,
    ))
}

pub(crate) fn svg(
    node: &NodeLayout,
    theme: &Theme,
    options: &Value,
    with_inner_shadow: bool,
) -> Option<String> {
    let stroke = node
        .style
        .stroke
        .as_deref()
        .unwrap_or(&theme.primary_border_color);
    let fill = node.style.fill.as_deref().unwrap_or(&theme.primary_color);
    let sw = node.style.stroke_width.unwrap_or(1.3);
    let dash = node
        .style
        .stroke_dasharray
        .as_ref()
        .map(|d| format!(" stroke-dasharray=\"{}\"", crate::render::escape_xml(d)))
        .unwrap_or_default();
    if let Some(geometry) = documents::geometry(node.shape, &node.label, neo(options)) {
        return Some(geometry.svg(node.x, node.y, fill, stroke, sw, &dash));
    }
    match node.shape {
        NodeShape::Rectangle => Some(format!(
            "<rect x=\"{:.5}\" y=\"{:.5}\" width=\"{:.5}\" height=\"{:.5}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}/>",
            node.x, node.y, node.width, node.height
        )),
        NodeShape::SmallCircle
        | NodeShape::FilledCircle
        | NodeShape::FramedCircle
        | NodeShape::CrossedCircle
        | NodeShape::DoubleCircle => {
            let cx = node.x + node.width / 2.0;
            let cy = node.y + node.height / 2.0;
            let radius = node.width / 2.0;
            let fill = if node.shape == NodeShape::FilledCircle {
                stroke
            } else {
                fill
            };
            let mut svg = format!(
                "<circle cx=\"{cx:.5}\" cy=\"{cy:.5}\" r=\"{radius:.5}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}/>"
            );
            match node.shape {
                NodeShape::DoubleCircle => {
                    let inner = radius - if neo(options) { 12.0 } else { 5.0 };
                    // Mermaid filters the two rings independently. Paint the
                    // inner shadow after the outer fill so it stays visible.
                    if with_inner_shadow && neo(options) {
                        svg.push_str(&format!("<circle cx=\"{:.5}\" cy=\"{:.5}\" r=\"{inner:.5}\" fill=\"black\" stroke=\"black\" stroke-width=\"{sw}\" opacity=\"0.06\"{dash}/>",cx+4.0,cy+4.0));
                    }
                    svg.push_str(&format!("<circle cx=\"{cx:.5}\" cy=\"{cy:.5}\" r=\"{inner:.5}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}/>"));
                }
                NodeShape::FramedCircle => {
                    let inner = radius * 5.0 / 14.0;
                    svg.push_str(&format!("<circle cx=\"{cx:.5}\" cy=\"{cy:.5}\" r=\"{inner:.5}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}/>"));
                }
                NodeShape::CrossedCircle => {
                    let d = radius / 2.0_f32.sqrt();
                    svg.push_str(&format!("<path d=\"M {:.5},{:.5} L {:.5},{:.5} M {:.5},{:.5} L {:.5},{:.5}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}/>",cx-d,cy-d,cx+d,cy+d,cx-d,cy+d,cx+d,cy-d));
                }
                _ => {}
            }
            Some(svg)
        }
        NodeShape::LightningBolt => Some(format!(
            "<polygon points=\"{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\" stroke-linejoin=\"miter\"{dash}/>",
            node.x + node.width,
            node.y,
            node.x,
            node.y + node.height / 2.0 + 3.5,
            node.x + node.width - 14.0,
            node.y + node.height / 2.0 + 3.5,
            node.x,
            node.y + node.height,
            node.x + node.width,
            node.y + node.height / 2.0 - 3.5,
            node.x + 14.0,
            node.y + node.height / 2.0 - 3.5,
        )),
        NodeShape::Hourglass => Some(format!(
            "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\" stroke-linejoin=\"miter\"{dash}/>",
            node.x,
            node.y,
            node.x + node.width,
            node.y,
            node.x,
            node.y + node.height,
            node.x + node.width,
            node.y + node.height
        )),
        NodeShape::Cloud => {
            let c = cloud(&node.label);
            Some(format!(
                "<path d=\"{}\" transform=\"translate({:.5} {:.5})\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}/>",
                c.path,
                node.x - c.bounds[0],
                node.y - c.bounds[1]
            ))
        }
        NodeShape::Comment | NodeShape::BraceLeft | NodeShape::BraceRight => {
            let h = node.label.height
                + if neo(options) {
                    if node.shape == NodeShape::BraceLeft {
                        12.0
                    } else {
                        24.0
                    }
                } else {
                    15.0
                };
            let r = (h * 0.1).max(5.0);
            let top = node.y;
            let bottom = node.y + node.height;
            let middle = (top + bottom) / 2.0;
            let mut path = String::new();
            for right in [false, true] {
                if (right && node.shape == NodeShape::BraceLeft)
                    || (!right && node.shape == NodeShape::BraceRight)
                {
                    continue;
                }
                let (mut x, sign) = if right {
                    (node.x + node.width, -1.0)
                } else {
                    (node.x, 1.0)
                };
                if node.shape == NodeShape::BraceLeft {
                    let w = node.label.width.max(120.0) + if neo(options) { 18.0 } else { 15.0 };
                    x += (w * 0.1 - r * 2.0).max(0.0);
                }
                let a = x + sign * r;
                let b = x + sign * 2.0 * r;
                path.push_str(&format!(" M {b:.5},{top:.5} Q {a:.5},{top:.5} {a:.5},{:.5} L {a:.5},{:.5} Q {a:.5},{middle:.5} {x:.5},{middle:.5} Q {a:.5},{middle:.5} {a:.5},{:.5} L {a:.5},{:.5} Q {a:.5},{bottom:.5} {b:.5},{bottom:.5}",top+r,middle-r,middle+r,bottom-r));
            }
            Some(format!(
                "<path d=\"{path}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}/>"
            ))
        }
        _ => None,
    }
}

/// The same measured outline is used for painting and clipping ELK ports.
pub(crate) fn outline(node: &NodeLayout, options: &Value) -> Option<Vec<(f32, f32)>> {
    let geometry = documents::geometry(node.shape, &node.label, neo(options))?;
    Some(
        geometry
            .outline
            .iter()
            .map(|&(x, y)| {
                (
                    node.x - geometry.bounds[0] + x,
                    node.y - geometry.bounds[1] + y,
                )
            })
            .collect(),
    )
}
