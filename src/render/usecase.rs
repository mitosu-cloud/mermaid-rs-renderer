use super::*;
use crate::ir::{NodeShape, UseCaseActorType, UseCaseData};
use crate::layout::{NodeLayout, SubgraphLayout};

pub(super) fn render_node(
    node: &NodeLayout,
    data: &UseCaseData,
    theme: &Theme,
    config: &LayoutConfig,
) -> String {
    let theme = crate::usecase::node_theme(data, &node.id, theme);
    let fill = node.style.fill.as_deref().unwrap_or(&theme.primary_color);
    let stroke = node
        .style
        .stroke
        .as_deref()
        .unwrap_or(&theme.primary_border_color);
    let stroke_width = node
        .style
        .stroke_width
        .unwrap_or(if crate::usecase::redux(&data.config) {
            2.0
        } else {
            1.0
        });
    let dash = node
        .style
        .stroke_dasharray
        .as_ref()
        .map(|value| format!(" stroke-dasharray=\"{}\"", escape_xml(value)))
        .unwrap_or_default();
    let cx = node.x + node.width / 2.0;
    let cy = node.y + node.height / 2.0;
    let name = node
        .label
        .lines
        .iter()
        .map(|line| line.text().into_owned())
        .collect::<Vec<_>>()
        .join(" ");
    let weight = node
        .style
        .font_weight
        .clone()
        .or_else(|| crate::usecase::font_weight(data, &node.id));
    let weight_attr = weight
        .as_ref()
        .map(|weight| format!(" font-weight=\"{}\"", escape_xml(weight)))
        .unwrap_or_default();
    let details = data.nodes.get(&node.id);
    let mut accessible_name = Vec::new();
    if details.is_some_and(|details| details.business) {
        accessible_name.push("Business".to_string());
    }
    if details.is_some_and(|details| details.actor_type.is_some()) {
        accessible_name.push("Actor".to_string());
    }
    if let Some(stereotype) = details.and_then(|details| details.stereotype.as_ref()) {
        accessible_name.push(format!("«{stereotype}»"));
    }
    accessible_name.push(name);
    let mut svg = format!(
        "<g id=\"usecase-{}\" role=\"img\" aria-label=\"{}\"{weight_attr}>",
        escape_xml(&node.id),
        escape_xml(&accessible_name.join(" "))
    );
    if let Some(rows) = data.json_tables.get(&node.id) {
        let border_width = node.style.stroke_width.unwrap_or(1.0);
        let metrics =
            crate::usecase::table_metrics(rows, &node.label, &theme, config, border_width);
        let fill = node.style.fill.as_deref().unwrap_or_else(|| {
            data.config
                .get("themeVariables")
                .and_then(|v| v.get("mainBkg"))
                .and_then(|v| v.as_str())
                .unwrap_or(
                    if data
                        .config
                        .get("theme")
                        .and_then(|v| v.as_str())
                        .unwrap_or("redux-color")
                        .starts_with("redux")
                    {
                        "#ffffff"
                    } else {
                        &theme.primary_color
                    },
                )
        });
        svg.push_str(&format!("<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{stroke_width}\"{dash}/>", node.x, node.y, node.width, node.height, fill, stroke));
        let inset = border_width.max(0.0);
        let x = node.x + inset;
        let mut top = node.y + inset;
        svg.push_str(&format!("<rect x=\"{x:.2}\" y=\"{top:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"/>", node.width - inset * 2.0, node.height - inset * 2.0));
        svg.push_str(&text_block_svg(
            cx,
            top + metrics.title_height / 2.0,
            &node.label,
            &theme,
            config,
            false,
            node.style.text_color.as_deref(),
        ));
        top += metrics.title_height;
        let separator_x = x + metrics.key_width;
        svg.push_str(&format!("<line x1=\"{separator_x:.2}\" y1=\"{top:.2}\" x2=\"{separator_x:.2}\" y2=\"{:.2}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"/>", node.y + node.height - inset));
        for ((key, value), height) in rows.iter().zip(metrics.row_heights) {
            svg.push_str(&format!("<line x1=\"{x:.2}\" y1=\"{top:.2}\" x2=\"{:.2}\" y2=\"{top:.2}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"/>", node.x + node.width - inset));
            for (text, center) in [
                (key, x + metrics.key_width / 2.0),
                (
                    value,
                    separator_x + (node.width - inset * 2.0 - metrics.key_width) / 2.0,
                ),
            ] {
                let block = crate::usecase::text_block(text, &theme, config);
                svg.push_str(&text_block_svg_with_font_size(
                    center,
                    top + height / 2.0,
                    &block,
                    &theme,
                    config,
                    theme.font_size,
                    "middle",
                    node.style.text_color.as_deref(),
                    false,
                ));
            }
            top += height;
        }
    } else if let Some(actor) = data.nodes.get(&node.id).and_then(|node| node.actor_type) {
        let glyph_y = node.y + 10.0 + 36.0;
        svg.push_str(&format!("<g transform=\"translate({cx:.2} {glyph_y:.2})\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{dash}>"));
        match actor {
            UseCaseActorType::Normal => svg.push_str("<path d=\"M 0 -12 C 6.627 -12 12 -17.373 12 -24 C 12 -30.627 6.627 -36 0 -36 C -6.627 -36 -12 -30.627 -12 -24 C -12 -17.373 -6.627 -12 0 -12 Z M 0 -12 V 8 M -17 -5 H 17 M 0 8 L -15 28 M 0 8 L 15 28\"/>"),
            UseCaseActorType::Hollow => svg.push_str("<circle cx=\"0\" cy=\"-23\" r=\"9\" fill=\"none\"/><path d=\"M -22 -10 H 22 V 0 H 6 L 22 17 L 13 28 L 0 13 L -13 28 L -22 17 L -6 0 H -22 Z\" fill=\"none\"/>"),
            UseCaseActorType::Awesome => svg.push_str("<path d=\"M 0 -34 C 7.18 -34 13 -28.18 13 -21 C 13 -13.82 7.18 -8 0 -8 C -7.18 -8 -13 -13.82 -13 -21 C -13 -28.18 -7.18 -34 0 -34 Z M -24 25 C -24 7 -14 -3 0 -3 C 14 -3 24 7 24 25 C 24 28 21 30 18 30 H -18 C -21 30 -24 28 -24 25 Z\"/>"),
            UseCaseActorType::Icon => {
                // usecaseActorIcon centers a 42px symbol in a 52px frame at y=-2.
                svg.push_str(&format!("<rect class=\"usecase-actor-icon-frame\" x=\"-26\" y=\"-28\" width=\"52\" height=\"52\" rx=\"4\" ry=\"4\" fill=\"{fill}\" stroke=\"{}\" stroke-width=\"{stroke_width}\"/>", theme.primary_border_color));
                let name = node.icon.as_deref().unwrap_or("");
                if crate::icons::lookup_icon(name).is_some() {
                    svg.push_str("<g stroke=\"none\" aria-hidden=\"true\">");
                    svg.push_str(&crate::icons::render_icon_svg(name, -21.0, -23.0, 42.0, stroke));
                    svg.push_str("</g>");
                } else {
                    // Mermaid's generic unknown Iconify symbol; no authored identifier text.
                    svg.push_str("<g class=\"usecase-actor-icon-fallback\" aria-hidden=\"true\" transform=\"translate(-21 -23) scale(0.525)\"><rect width=\"80\" height=\"80\" fill=\"#087ebf\" stroke-width=\"0\"/><text x=\"21.16\" y=\"64.67\" fill=\"#fff\" font-family=\"ArialMT, Arial\" font-size=\"67.75\">?</text></g>");
                }
            },
        }
        if data.nodes.get(&node.id).is_some_and(|node| node.business) {
            let (head_y, radius) = if actor == UseCaseActorType::Hollow {
                (-23.0_f32, 9.0_f32)
            } else {
                (-24.0, 12.0)
            };
            let center_x = radius * 0.6 * 3.0_f32.sqrt() / 2.0;
            let center_y = head_y + radius * 0.3;
            let dx = radius * 0.4;
            let dy = -radius * 0.4 * 3.0_f32.sqrt();
            svg.push_str(&format!(
                "<path d=\"M {:.2} {:.2} L {:.2} {:.2}\" fill=\"none\"/>",
                center_x - dx,
                center_y - dy,
                center_x + dx,
                center_y + dy
            ));
        }
        svg.push_str("</g>");
        let mut top = node.y + 10.0 + 72.0 + 8.0;
        if let Some(stereotype) = node.sub_label.as_ref() {
            svg.push_str(&text_block_svg(
                cx,
                top + stereotype.height / 2.0,
                stereotype,
                &theme,
                config,
                false,
                node.style.text_color.as_deref(),
            ));
            top += stereotype.height + 2.0;
        }
        svg.push_str(&text_block_svg(
            cx,
            top + node.label.height / 2.0,
            &node.label,
            &theme,
            config,
            false,
            node.style.text_color.as_deref(),
        ));
    } else {
        if node.shape == NodeShape::Ellipse {
            svg.push_str(&format!("<ellipse cx=\"{cx:.2}\" cy=\"{cy:.2}\" rx=\"{:.2}\" ry=\"{:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{dash}/>", node.width / 2.0, node.height / 2.0));
            if data.nodes.get(&node.id).is_some_and(|node| node.business) {
                let rx = node.width / 2.0;
                let ry = node.height / 2.0;
                let start_x = node.label.width.max(
                    node.sub_label
                        .as_ref()
                        .map(|label| label.width)
                        .unwrap_or(0.0),
                ) / 2.0
                    + 2.0;
                let end_x = rx - 2.0;
                let start_y = ry * (1.0 - (start_x / rx).powi(2)).max(0.0).sqrt();
                let end_y = -ry * (1.0 - (end_x / rx).powi(2)).max(0.0).sqrt();
                svg.push_str(&format!("<path d=\"M {:.2} {:.2} L {:.2} {:.2}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"/>", cx + start_x, cy + start_y, cx + end_x, cy + end_y));
            }
        } else {
            svg.push_str(&shape_svg(
                node,
                &theme,
                config,
                crate::ir::DiagramKind::UseCase,
            ));
        }
        let total = node.label.height
            + node
                .sub_label
                .as_ref()
                .map(|label| label.height + 2.0)
                .unwrap_or(0.0);
        let mut top = cy - total / 2.0;
        if let Some(stereotype) = node.sub_label.as_ref() {
            svg.push_str(&text_block_svg(
                cx,
                top + stereotype.height / 2.0,
                stereotype,
                &theme,
                config,
                false,
                node.style.text_color.as_deref(),
            ));
            top += stereotype.height + 2.0;
        }
        svg.push_str(&text_block_svg(
            cx,
            top + node.label.height / 2.0,
            &node.label,
            &theme,
            config,
            false,
            node.style.text_color.as_deref(),
        ));
    }
    svg.push_str("</g>");
    svg
}

pub(super) fn render_package(
    subgraph: &SubgraphLayout,
    theme: &Theme,
    config: &LayoutConfig,
) -> String {
    let fill = subgraph
        .style
        .fill
        .as_deref()
        .unwrap_or(&theme.cluster_background);
    let stroke = subgraph
        .style
        .stroke
        .as_deref()
        .unwrap_or(&theme.cluster_border);
    let width = (subgraph.label_block.width + 24.0).min(subgraph.width);
    let header = subgraph.label_block.height + 16.0;
    let body_y = subgraph.y + header;
    let mut svg = format!(
        "<path d=\"M {:.2} {:.2} H {:.2} V {body_y:.2} H {:.2} V {:.2} H {:.2} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{}\"/>",
        subgraph.x,
        subgraph.y,
        subgraph.x + width,
        subgraph.x + subgraph.width,
        subgraph.y + subgraph.height,
        subgraph.x,
        subgraph.style.stroke_width.unwrap_or(1.0)
    );
    svg.push_str(&format!("<line x1=\"{:.2}\" y1=\"{body_y:.2}\" x2=\"{:.2}\" y2=\"{body_y:.2}\" stroke=\"{stroke}\"/>", subgraph.x, subgraph.x + width));
    svg.push_str(&text_block_svg(
        subgraph.x + width / 2.0,
        subgraph.y + header / 2.0,
        &subgraph.label_block,
        theme,
        config,
        false,
        subgraph.style.text_color.as_deref(),
    ));
    svg
}
