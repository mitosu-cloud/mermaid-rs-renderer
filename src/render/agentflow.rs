use super::*;
use crate::layout::NodeLayout;
use serde_json::Value;

/// ELK supplies orthogonal sections; round each bend locally instead of
/// replacing the entire routed section with a spline through obstacles.
pub(super) fn rounded_path(points: &[(f32, f32)]) -> String {
    rounded_path_with_endpoint_gaps(points, 0.0, 0.0)
}

/// Neo flowcharts mask four pixels of the stroke underneath each arrow tip.
/// Move only the painted endpoints, retaining the original bend geometry.
pub(super) fn rounded_path_with_endpoint_gaps(
    points: &[(f32, f32)],
    start_gap: f32,
    end_gap: f32,
) -> String {
    let points = dedupe_points(points);
    let Some(first) = points.first() else {
        return String::new();
    };
    let inset = |point: (f32, f32), neighbor: Option<&(f32, f32)>, gap: f32| {
        let Some(neighbor) = neighbor else {
            return point;
        };
        let (dx, dy) = (neighbor.0 - point.0, neighbor.1 - point.1);
        let length = dx.hypot(dy);
        if length < 0.001 {
            return point;
        }
        let amount = gap.max(0.0).min(length / 2.0) / length;
        (point.0 + dx * amount, point.1 + dy * amount)
    };
    let first = inset(*first, points.get(1), start_gap);
    let mut path = format!("M {:.3},{:.3}", first.0, first.1);
    for index in 1..points.len().saturating_sub(1) {
        let (a, b, c) = (points[index - 1], points[index], points[index + 1]);
        let incoming = ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
        let outgoing = ((c.0 - b.0).powi(2) + (c.1 - b.1).powi(2)).sqrt();
        if incoming < 0.001 || outgoing < 0.001 {
            continue;
        }
        let dot = ((b.0 - a.0) * (c.0 - b.0) + (b.1 - a.1) * (c.1 - b.1)) / (incoming * outgoing);
        let angle = dot.clamp(-1.0, 1.0).acos();
        // ELK can include dummy-node points in a straight channel. Mermaid
        // leaves those straight and rounds only actual turns, with radius 5.
        if angle < 0.00001 || (std::f32::consts::PI - angle).abs() < 0.00001 {
            path.push_str(&format!(" L {:.3},{:.3}", b.0, b.1));
            continue;
        }
        let radius = (5.0 / (angle / 2.0).sin())
            .min(incoming / 2.0)
            .min(outgoing / 2.0);
        let p = (
            b.0 + (a.0 - b.0) * radius / incoming,
            b.1 + (a.1 - b.1) * radius / incoming,
        );
        let q = (
            b.0 + (c.0 - b.0) * radius / outgoing,
            b.1 + (c.1 - b.1) * radius / outgoing,
        );
        path.push_str(&format!(
            " L {:.3},{:.3} Q {:.3},{:.3} {:.3},{:.3}",
            p.0, p.1, b.0, b.1, q.0, q.1
        ));
    }
    if let Some(last) = points.last() {
        let last = inset(*last, points.get(points.len().saturating_sub(2)), end_gap);
        path.push_str(&format!(" L {:.3},{:.3}", last.0, last.1));
    }
    path
}

pub(super) fn shape(node: &NodeLayout, theme: &Theme, options: &Value) -> Option<String> {
    let (x, y, w, h) = (node.x, node.y, node.width, node.height);
    let fill = node.style.fill.as_deref().unwrap_or(&theme.primary_color);
    let stroke = node
        .style
        .stroke
        .as_deref()
        .unwrap_or(&theme.primary_border_color);
    let sw = node.style.stroke_width.unwrap_or(1.0);
    let dash = node
        .style
        .stroke_dasharray
        .as_ref()
        .map(|dash| format!(" stroke-dasharray=\"{}\"", escape_xml(dash)))
        .unwrap_or_default();
    match node.shape {
        crate::ir::NodeShape::ReferenceDocument => {
            let body = node.label.height
                + if crate::agentflow::neo(options) {
                    24.0
                } else {
                    30.0
                };
            let amplitude = body
                / if crate::agentflow::neo(options) {
                    4.0
                } else {
                    8.0
                };
            let baseline = y + body + amplitude;
            let right = x + w;
            let margin = x + w / 22.0;
            let mut path = format!("M {x:.3},{y:.3} V {baseline:.3}");
            for step in 0..=50 {
                let t = step as f32 / 50.0;
                path.push_str(&format!(
                    " L {:.3},{:.3}",
                    x + w * t,
                    baseline + amplitude * (std::f32::consts::TAU * 0.8 * t).sin()
                ));
            }
            path.push_str(&format!(" L {right:.3},{y:.3} H {x:.3} Z"));
            Some(format!(
                "<path d=\"{path}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}/><path d=\"M {margin:.3},{y:.3} V {baseline:.3}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>"
            ))
        }
        crate::ir::NodeShape::Hexagon => {
            let cut = h / if crate::agentflow::neo(options) {
                3.5
            } else {
                4.0
            };
            Some(format!(
                "<polygon points=\"{:.2},{y:.2} {:.2},{y:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {x:.2},{:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}/>",
                x + cut,
                x + w - cut,
                x + w,
                y + h / 2.0,
                x + w - cut,
                y + h,
                x + cut,
                y + h,
                y + h / 2.0
            ))
        }
        crate::ir::NodeShape::CollapsedGroup => {
            let separator_y = y + h - 28.0;
            let indicator = options
                .get("themeVariables")
                .and_then(|v| {
                    v.get("flowContainerStroke")
                        .or_else(|| v.get("secondaryBorderColor"))
                })
                .and_then(Value::as_str)
                .unwrap_or(if crate::usecase::redux(options) {
                    "#B3B3B3"
                } else {
                    &theme.cluster_border
                });
            let mut svg = format!(
                "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\" rx=\"10\" ry=\"10\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}/><line x1=\"{:.2}\" y1=\"{separator_y:.2}\" x2=\"{:.2}\" y2=\"{separator_y:.2}\" stroke=\"{indicator}\" stroke-width=\"0.75\" stroke-dasharray=\"3,3\"/>",
                x + 8.0,
                x + w - 8.0
            );
            for offset in [-10.0, 0.0, 10.0] {
                svg.push_str(&format!(
                    "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"2.5\" fill=\"{}\" opacity=\"0.5\"/>",
                    x + w / 2.0 + offset,
                    separator_y + 10.0,
                    theme.primary_border_color
                ));
            }
            Some(svg)
        }
        _ => None,
    }
}
