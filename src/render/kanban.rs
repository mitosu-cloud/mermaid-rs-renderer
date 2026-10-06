use std::fmt::Write;

use crate::config::LayoutConfig;
use crate::layout::Layout;
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

pub(super) fn render_kanban(layout: &Layout, theme: &Theme, config: &LayoutConfig) -> String {
    let mut svg = String::new();
    let mut label_config = config.clone();
    label_config.label_line_height = 1.5;
    let first_line_baseline = theme.font_size * 0.75
        + crate::text_metrics::centered_baseline_offset(theme.font_size, &theme.font_family)
            .unwrap_or(theme.font_size * 0.375);
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
            let _ = write!(
                svg,
                "<rect x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" rx=\"5\" ry=\"5\" fill=\"{background}\" stroke=\"{border}\" stroke-width=\"1\"/>",
                card.x, card.y, card.width, card.height
            );
            svg.push_str(&text_block_svg_with_font_size(
                card.x + 10.0,
                card.y + 10.0 + first_line_baseline,
                &card.label,
                theme,
                &label_config,
                theme.font_size,
                "start",
                Some(&theme.text_color),
                true,
            ));
        }
    }
    svg
}
