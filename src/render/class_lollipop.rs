use super::*;

pub(super) fn circle(point: (f32, f32), angle: f32, start: bool, stroke: &str) -> String {
    let theta = angle.to_radians();
    let offset = if start { -6.0 } else { 6.0 };
    format!(
        "<circle cx=\"{:.3}\" cy=\"{:.3}\" r=\"6\" fill=\"white\" stroke=\"{stroke}\" stroke-width=\"2\"/>",
        point.0 + offset * theta.cos(),
        point.1 + offset * theta.sin()
    )
}

pub(super) fn node(
    node: &crate::layout::NodeLayout,
    theme: &Theme,
    config: &LayoutConfig,
) -> String {
    if node.shape == crate::ir::NodeShape::Text {
        return text_block_svg(
            node.x + node.width / 2.0,
            node.y + node.height / 2.0,
            &node.label,
            theme,
            config,
            false,
            node.style.text_color.as_deref(),
        );
    }
    if node.shape == crate::ir::NodeShape::Note {
        return format!(
            "{}{}",
            shape_svg(node, theme, config, crate::ir::DiagramKind::Class),
            text_block_svg(
                node.x + node.width / 2.0,
                node.y + node.height / 2.0,
                &node.label,
                theme,
                config,
                false,
                node.style.text_color.as_deref()
            )
        );
    }
    let groups = crate::class_lollipop::sections(&node.label);
    let heights = crate::class_lollipop::heights(&groups, theme);
    let fill = node.style.fill.as_deref().unwrap_or(&theme.primary_color);
    let stroke = node
        .style
        .stroke
        .as_deref()
        .unwrap_or(&theme.primary_border_color);
    let color = node
        .style
        .text_color
        .as_deref()
        .unwrap_or(&theme.primary_text_color);
    let sw = node.style.stroke_width.unwrap_or(2.0);
    let mut svg = format!(
        "<rect x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" fill=\"black\" opacity=\"0.06\"/><rect x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        node.x + 4.0,
        node.y + 4.0,
        node.width,
        node.height,
        node.x,
        node.y,
        node.width,
        node.height
    );
    let mut top = node.y;
    for (i, group) in groups.iter().enumerate() {
        if i > 0 {
            svg.push_str(&format!("<line x1=\"{:.3}\" y1=\"{top:.3}\" x2=\"{:.3}\" y2=\"{top:.3}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",node.x,node.x+node.width));
        }
        let baseline = top
            + (heights[i] - group.len() as f32 * theme.font_size * 1.5) / 2.0
            + theme.font_size * 1.5 / 2.0
            + crate::text_metrics::centered_baseline_offset(theme.font_size, &theme.font_family)
                .unwrap_or(theme.font_size * 0.35);
        let lines: Vec<_> = group
            .iter()
            .enumerate()
            .map(|(i, s)| (i, s.as_str()))
            .collect();
        svg.push_str(&text_lines_svg(
            &lines,
            if i == 0 {
                node.x + node.width / 2.0
            } else {
                node.x + 12.0
            },
            baseline,
            theme.font_size * 1.5,
            if i == 0 { "middle" } else { "start" },
            theme,
            color,
            i == 0,
        ));
        top += heights[i];
    }
    svg
}
