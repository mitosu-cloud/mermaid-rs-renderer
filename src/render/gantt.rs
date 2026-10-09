use super::*;
use crate::layout::NativeGanttLayout;

pub(super) fn render(
    data: &NativeGanttLayout,
    title: Option<&TextBlock>,
    _theme: &Theme,
) -> String {
    let var = |key: &str, default: &str| {
        data.options
            .get("themeVariables")
            .and_then(|v| v.get(key))
            .and_then(serde_json::Value::as_str)
            .unwrap_or(default)
            .to_string()
    };
    let num = |key: &str, default: f32| {
        data.options
            .get("gantt")
            .and_then(|v| v.get(key))
            .and_then(serde_json::Value::as_f64)
            .map(|v| v as f32)
            .unwrap_or(default)
    };
    let family = data
        .options
        .get("fontFamily")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("\"trebuchet ms\", verdana, arial, sans-serif")
        .trim_end_matches(';');
    let font = num("fontSize", 11.0);
    let section_font = num("sectionFontSize", 11.0);
    let text = |x: f32, y: f32, label: &str, size: f32, anchor: &str, color: &str, extra: &str| {
        format!(
            "<text x=\"{x}\" y=\"{y}\" font-family=\"{}\" font-size=\"{size}\" text-anchor=\"{anchor}\" fill=\"{}\" {extra}>{}</text>",
            escape_xml(family),
            escape_xml(color),
            escape_xml(label)
        )
    };
    let mut svg = String::new();
    let grid_top = num("gridLineStartPadding", 35.0);
    for &(start, end) in &data.excluded {
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{grid_top}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            data.x(start),
            data.x(end) - data.x(start),
            data.height - data.top - grid_top,
            escape_xml(&var("excludeBkgColor", "#eeeeee"))
        ));
    }
    let top_axis = data
        .options
        .get("gantt")
        .and_then(|v| v.get("topAxis"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    for tick in &data.ticks {
        svg.push_str(&format!("<line x1=\"{}\" x2=\"{}\" y1=\"{}\" y2=\"{}\" stroke=\"black\" opacity=\"0.8\" shape-rendering=\"crispEdges\"/>",tick.x,tick.x,data.height-50.0,num("gridLineStartPadding",35.0)));
        svg.push_str(&text(
            tick.x,
            data.height - 37.0,
            &tick.label,
            10.0,
            "middle",
            &var("textColor", "#333"),
            "opacity=\"0.8\"",
        ));
        if top_axis {
            svg.push_str(&format!("<line x1=\"{}\" x2=\"{}\" y1=\"{}\" y2=\"{}\" stroke=\"black\" opacity=\"0.8\" shape-rendering=\"crispEdges\"/>", tick.x,tick.x,data.top,data.height-grid_top));
            svg.push_str(&text(
                tick.x,
                data.top - 6.0,
                &tick.label,
                10.0,
                "middle",
                &var("textColor", "#333"),
                "opacity=\"0.8\"",
            ));
        }
    }
    for (section, (label, first, count)) in data.sections.iter().enumerate() {
        let color = match section % 4 {
            0 => var("sectionBkgColor", "rgba(102,102,255,0.49)"),
            2 => var("sectionBkgColor2", "#fff400"),
            _ => var("altSectionBkgColor", "white"),
        };
        svg.push_str(&format!(
            "<rect x=\"0\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{color}\" opacity=\"0.2\"/>",
            data.top + *first as f32 * data.gap - 2.0,
            data.width - data.right / 2.0,
            *count as f32 * data.gap
        ));
        svg.push_str(&text(
            10.0,
            data.top + (*first as f32 + *count as f32 / 2.0) * data.gap,
            label,
            section_font,
            "start",
            &var("titleColor", "#333"),
            "dominant-baseline=\"central\"",
        ));
    }
    let mut tasks: Vec<_> = data.tasks.iter().collect();
    let normal_tasks = tasks.iter().filter(|task| !task.vert).count() as f32;
    tasks.sort_by(|a, b| a.vert.cmp(&b.vert).then(a.start.total_cmp(&b.start)));
    for task in tasks {
        let (x, width) = if task.milestone {
            (
                data.x(task.start) + (data.x(task.end) - data.x(task.start)) / 2.0
                    - data.bar_height / 2.0,
                data.bar_height,
            )
        } else if task.vert {
            (data.x(task.start), 0.08 * data.bar_height)
        } else {
            (
                data.x(task.start),
                data.x(task.render_end) - data.x(task.start),
            )
        };
        let y = if task.vert {
            grid_top
        } else {
            data.top + task.row as f32 * data.gap
        };
        let height = if task.vert {
            normal_tasks * data.gap + 2.0 * data.bar_height
        } else {
            data.bar_height
        };
        let fill = if task.active {
            var("activeTaskBkgColor", "#bfc7ff")
        } else if task.done {
            var("doneTaskBkgColor", "lightgrey")
        } else if task.crit {
            var("critBkgColor", "red")
        } else {
            var("taskBkgColor", "#8a90dd")
        };
        let stroke = if task.vert {
            var("vertLineColor", "navy")
        } else if task.crit {
            var("critBorderColor", "#ff8888")
        } else if task.done {
            var("doneTaskBorderColor", "grey")
        } else if task.active {
            var("activeTaskBorderColor", "#534fbc")
        } else {
            var("taskBorderColor", "#534fbc")
        };
        let transform = if task.milestone {
            format!(
                "transform=\"translate({} {}) rotate(45) scale(0.8) translate({} {})\"",
                x + width / 2.0,
                y + data.bar_height / 2.0,
                -x - width / 2.0,
                -y - data.bar_height / 2.0
            )
        } else {
            String::new()
        };
        svg.push_str(&format!("<rect id=\"gantt-{}\" x=\"{x}\" y=\"{y}\" width=\"{}\" height=\"{height}\" rx=\"3\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"2\" {transform}/>",escape_xml(&task.id),width.max(0.0)));
        let italic = if task.milestone {
            "font-style=\"italic\""
        } else {
            ""
        };
        let label_width = text_metrics::measure_styled_text_width(
            &task.label,
            font,
            family,
            false,
            task.milestone,
        )
        .unwrap_or(task.label.len() as f32 * font * 0.5);
        let outside = label_width > width;
        let (label_x, anchor) = if task.vert {
            (x, "middle")
        } else if outside {
            if x - data.left + width + label_width + 1.5 * data.left > data.width {
                (x - 5.0, "end")
            } else {
                (x + width + 5.0, "start")
            }
        } else {
            (x + width / 2.0, "middle")
        };
        // Mermaid positions using renderEndTime, but chooses the text class
        // against endTime. Excluded dates can make those widths different.
        let outside_color = label_width
            > if task.milestone {
                width
            } else {
                data.x(task.end) - data.x(task.start)
            };
        let color = if task.vert {
            var("vertLineColor", "navy")
        } else if task.active || task.done {
            var("taskTextDarkColor", "black")
        } else if outside_color {
            var("taskTextOutsideColor", "black")
        } else {
            var("taskTextColor", "white")
        };
        svg.push_str(&text(
            label_x,
            if task.vert {
                grid_top + normal_tasks * data.gap + 60.0
            } else {
                y + data.bar_height / 2.0 + font / 2.0 - 2.0
            },
            &task.label,
            if task.vert { 15.0 } else { font },
            anchor,
            &color,
            italic,
        ));
    }
    let today_marker = data
        .options
        .get("gantt")
        .and_then(|v| v.get("todayMarker"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if today_marker != "off" {
        let mut today = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|v| v.as_secs_f64())
            .unwrap_or(0.0);
        if data
            .options
            .get("gantt")
            .and_then(|v| v.get("dateFormat"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|v| v.contains('H'))
        {
            today = today.rem_euclid(86400.0);
        }
        if today >= data.start && today <= data.end {
            svg.push_str(&format!("<line x1=\"{x}\" x2=\"{x}\" y1=\"{top}\" y2=\"{bottom}\" stroke=\"{color}\" stroke-width=\"2\" style=\"{style}\"/>",
                x=data.x(today),top=num("titleTopMargin",25.0),bottom=data.height-num("titleTopMargin",25.0),
                color=escape_xml(&var("todayLineColor","red")),style=escape_xml(&today_marker.replace(',',";"))));
        }
    }
    if let Some(title) = title {
        svg.push_str(&text(
            data.width / 2.0,
            num("titleTopMargin", 25.0),
            &title
                .lines
                .iter()
                .map(|l| l.text())
                .collect::<Vec<_>>()
                .join(" "),
            18.0,
            "middle",
            &var("titleColor", "#333"),
            "",
        ));
    }
    svg
}
