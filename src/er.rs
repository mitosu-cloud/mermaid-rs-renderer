//! Shared ER table measurements: layout and drawing must use identical cells.
use crate::{layout::TextLine, text_metrics, theme::Theme};

pub(crate) struct Table {
    pub title: String,
    pub rows: Vec<[String; 4]>,
    pub columns: [f32; 4],
    pub width: f32,
    pub height: f32,
    pub row_height: f32,
    pub padding: f32,
}

pub(crate) fn text_width(text: &str, theme: &Theme) -> f32 {
    let width =
        text_metrics::measure_text_width_with_kerning(text, theme.font_size, &theme.font_family)
            .unwrap_or(text.chars().count() as f32 * theme.font_size * 0.5);
    // Browser HTML labels expose widths on a 1/64 px grid.
    (width * 64.0).ceil() / 64.0
}

pub(crate) fn table(lines: &[TextLine], theme: &Theme, line_height: f32) -> Table {
    let title = lines
        .first()
        .map(|line| line.text().into_owned())
        .unwrap_or_default();
    let mut rows = Vec::new();
    let mut in_body = false;
    for line in lines.iter().skip(1) {
        let text = line.text();
        if text.trim() == "---" {
            in_body = true;
            continue;
        }
        if !in_body || text.trim().is_empty() {
            continue;
        }
        let (fields, comment) = if let Some(start) = text.find('"') {
            (
                &text[..start],
                text[start..].trim().trim_matches('"').to_string(),
            )
        } else {
            (text.as_ref(), String::new())
        };
        let mut words = fields.split_whitespace();
        let data_type = words.next().unwrap_or_default().to_string();
        let name = words.next().unwrap_or_default().to_string();
        let keys = words
            .flat_map(|word| word.split(','))
            .filter(|key| matches!(*key, "PK" | "FK" | "UK"))
            .collect::<Vec<_>>()
            .join(",");
        rows.push([data_type, name, keys, comment]);
    }
    // erBox.ts expands SVG-mode padding by 1.25; the default reference
    // takes this branch while createText still emits HTML line boxes.
    let padding = 25.0;
    let row_height = theme.font_size * line_height + 18.75;
    let mut columns = [0.0_f32; 4];
    for row in &rows {
        for (index, value) in row.iter().enumerate() {
            if index < 2 || !value.is_empty() {
                columns[index] = columns[index].max(text_width(value, theme) + padding);
            }
        }
    }
    let column_width: f32 = columns.iter().sum();
    let width = column_width.max(text_width(&title, theme) + padding * 2.0);
    let count = columns.iter().filter(|width| **width > 0.0).count();
    if count > 0 {
        for column in &mut columns {
            if *column > 0.0 {
                *column += (width - column_width) / count as f32;
            }
        }
    }
    let height = row_height * (rows.len() + 1) as f32;
    Table {
        title,
        rows,
        columns,
        width,
        height,
        row_height,
        padding,
    }
}
