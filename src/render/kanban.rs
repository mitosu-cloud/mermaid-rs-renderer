use std::collections::BTreeMap;
use std::fmt::Write;

use crate::config::LayoutConfig;
use crate::layout::{KanbanCardLayout, Layout};
use crate::theme::{Theme, adjust_color};

use super::{escape_xml, text_block_svg_with_font_size};

fn column_fill(index: usize, theme: &Theme) -> String {
    // Mermaid numbers the first column section-1; section-i uses cScale(i+1).
    const HUES: [u16; 12] = [240, 60, 80, 270, 300, 330, 0, 30, 90, 150, 180, 210];
    let color_index = (index + 2) % HUES.len();
    let default = format!("hsl({}, 100%, 76.2745098039%)", HUES[color_index]);
    let base = theme.cscale_colors.get(color_index).unwrap_or(&default);
    adjust_color(base, 0.0, 0.0, 10.0)
}

pub(super) fn render_kanban(
    layout: &Layout,
    cards: &BTreeMap<String, KanbanCardLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) -> String {
    let mut svg = String::new();
    let mut label_config = config.clone();
    label_config.label_line_height = 1.5;
    let first_line_baseline = (theme.font_size * 0.75
        + crate::text_metrics::centered_baseline_offset(theme.font_size, &theme.font_family)
            .unwrap_or(theme.font_size * 0.375))
    .floor();
    let background = escape_xml(&theme.background);
    let border = escape_xml(&theme.primary_border_color);

    for (index, column) in layout.subgraphs.iter().enumerate() {
        let fill = escape_xml(&column_fill(index, theme));
        let _ = write!(
            svg,
            "<rect x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" rx=\"5\" ry=\"5\" fill=\"{fill}\" stroke=\"{fill}\" stroke-width=\"1\"/>",
            column.x, column.y, column.width, column.height
        );
        svg.push_str(&text_block_svg_with_font_size(
            column.x + column.width / 2.0,
            column.y + first_line_baseline,
            &column.label_block,
            theme,
            &label_config,
            theme.font_size,
            "middle",
            Some(&theme.text_color),
            true,
        ));
    }
    for column in &layout.subgraphs {
        for id in &column.nodes {
            let Some(card) = layout.nodes.get(id) else {
                continue;
            };
            let Some(metadata) = cards.get(id) else {
                continue;
            };
            let _ = write!(
                svg,
                "<rect x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" rx=\"5\" ry=\"5\" fill=\"{background}\" stroke=\"{border}\" stroke-width=\"1\"/>",
                card.x, card.y, card.width, card.height
            );
            // Mermaid centers title + footer using half the footer's line
            // height as an adjustment; the title is not simply top-padded.
            let height_adjustment = metadata.ticket.height.max(metadata.assigned.height) / 2.0;
            let title_top =
                card.y + card.height / 2.0 - height_adjustment - card.label.height / 2.0;
            svg.push_str(&text_block_svg_with_font_size(
                card.x + 10.0,
                title_top + first_line_baseline,
                &card.label,
                theme,
                &label_config,
                theme.font_size,
                "start",
                Some(&theme.text_color),
                true,
            ));
            let footer_y = title_top + card.label.height + first_line_baseline;
            if !metadata.ticket.lines.is_empty() {
                if let Some(url) = &metadata.ticket_url {
                    let _ = write!(
                        svg,
                        "<a href=\"{}\" target=\"_blank\" text-decoration=\"underline\">",
                        escape_xml(url)
                    );
                }
                svg.push_str(&text_block_svg_with_font_size(
                    card.x + 10.0,
                    footer_y,
                    &metadata.ticket,
                    theme,
                    &label_config,
                    theme.font_size,
                    "start",
                    Some(&theme.text_color),
                    true,
                ));
                if metadata.ticket_url.is_some() {
                    svg.push_str("</a>");
                }
            }
            if !metadata.assigned.lines.is_empty() {
                svg.push_str(&text_block_svg_with_font_size(
                    card.x + card.width - 10.0,
                    footer_y,
                    &metadata.assigned,
                    theme,
                    &label_config,
                    theme.font_size,
                    "end",
                    Some(&theme.text_color),
                    true,
                ));
            }
            let priority_color = match metadata.priority.as_deref() {
                Some("Very High") => Some("red"),
                Some("High") => Some("orange"),
                Some("Low") => Some("blue"),
                Some("Very Low") => Some("lightblue"),
                _ => None,
            };
            if let Some(color) = priority_color {
                let _ = write!(
                    svg,
                    "<line x1=\"{:.3}\" y1=\"{:.3}\" x2=\"{:.3}\" y2=\"{:.3}\" stroke-width=\"4\" stroke=\"{color}\"/>",
                    card.x + 2.0,
                    card.y + 2.0,
                    card.x + 2.0,
                    card.y + card.height - 2.0
                );
            }
        }
    }
    svg
}
