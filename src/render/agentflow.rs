use super::*;
use crate::layout::NodeLayout;
use serde_json::Value;

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
