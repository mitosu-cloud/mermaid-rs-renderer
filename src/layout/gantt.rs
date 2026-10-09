use super::*;

fn option<'a>(options: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    options
        .get("gantt")
        .and_then(|v| v.get(key))
        .or_else(|| options.get(key))
}

fn date(value: &str, format: &str) -> Option<f64> {
    if format.contains('H') {
        let parts: Vec<f64> = value
            .trim()
            .split(':')
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()?;
        if !(2..=3).contains(&parts.len()) || parts[0] >= 24.0 || parts[1] >= 60.0 {
            return None;
        }
        return Some(parts[0] * 3600.0 + parts[1] * 60.0 + parts.get(2).copied().unwrap_or(0.0));
    }
    if format == "x" || format == "X" {
        return value.parse::<f64>().ok().map(|v| v / 1000.0);
    }
    if format == "D" {
        return value.parse::<f64>().ok().map(|v| v * 86400.0);
    }
    parse_gantt_date(value).map(|v| v as f64 * 86400.0)
}

fn duration(value: &str) -> Option<f64> {
    let n = value.find(|c: char| !c.is_ascii_digit() && c != '.')?;
    let amount: f64 = value[..n].parse().ok()?;
    let unit = match &value[n..] {
        "ms" => 0.001,
        "s" => 1.0,
        "m" => 60.0,
        "h" => 3600.0,
        "d" => 86400.0,
        "w" => 604800.0,
        "M" => 2592000.0,
        "y" => 31536000.0,
        _ => return None,
    };
    Some(amount * unit)
}

struct WorkingDays {
    excludes: Vec<String>,
    includes: Vec<String>,
    weekend: i32,
    format: String,
}

impl WorkingDays {
    fn new(options: &serde_json::Value, format: &str) -> Self {
        let list = |key| match option(options, key) {
            Some(serde_json::Value::String(value)) => value
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|v| !v.is_empty())
                .map(str::to_string)
                .collect(),
            Some(serde_json::Value::Array(values)) => values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect(),
            _ => Vec::new(),
        };
        Self {
            excludes: list("excludes"),
            includes: list("includes"),
            weekend: if option(options, "weekend").and_then(serde_json::Value::as_str)
                == Some("friday")
            {
                5
            } else {
                6
            },
            format: format.to_string(),
        }
    }

    fn invalid(&self, time: f64) -> bool {
        let day = (time / 86400.0).floor() as i32;
        let (y, m, d) = civil_from_days(day);
        let iso = format!("{y:04}-{m:02}-{d:02}");
        let formatted = self
            .format
            .replace("YYYY", &format!("{y:04}"))
            .replace("MM", &format!("{m:02}"))
            .replace("DD", &format!("{d:02}"));
        if self.includes.iter().any(|v| v == &iso || v == &formatted) {
            return false;
        }
        let weekday = (day + 4).rem_euclid(7);
        let name = [
            "sunday",
            "monday",
            "tuesday",
            "wednesday",
            "thursday",
            "friday",
            "saturday",
        ][weekday as usize];
        self.excludes.iter().any(|v| {
            v == &iso
                || v == &formatted
                || v.eq_ignore_ascii_case(name)
                || (v.eq_ignore_ascii_case("weekends")
                    && (weekday == self.weekend || weekday == (self.weekend + 1) % 7))
        })
    }

    fn end_dates(&self, start: f64, mut end: f64) -> (f64, f64) {
        let mut cursor = start + 86400.0;
        let mut render_end = end;
        let mut invalid = false;
        let limit = end + 10000.0 * 86400.0;
        // Mermaid extends dependencies past excluded days, but may leave the
        // visible bar at an earlier boundary. Do not collapse the two dates.
        while cursor <= end && end < limit {
            if !invalid {
                render_end = end;
            }
            invalid = self.invalid(cursor);
            if invalid {
                end += 86400.0;
            }
            cursor += 86400.0;
        }
        (end, render_end)
    }

    fn ranges(&self, start: f64, end: f64) -> Vec<(f64, f64)> {
        let mut ranges = Vec::new();
        if end - start > 5.0 * 366.0 * 86400.0 || self.excludes.is_empty() {
            return ranges;
        }
        let mut range = None;
        let mut cursor = start;
        while cursor <= end {
            if self.invalid(cursor) {
                let day = (cursor / 86400.0).floor() * 86400.0;
                let first = range.map(|(a, _)| a).unwrap_or(day);
                range = Some((first, day + 86400.0 - 0.001));
            } else if let Some(value) = range.take() {
                ranges.push(value);
            }
            cursor += 86400.0;
        }
        // The JS renderer emits a range only when the next valid day closes it.
        ranges
    }
}

fn calendar_ticks(data: &NativeGanttLayout, format: &str) -> Vec<GanttTick> {
    if data.tasks.is_empty() {
        return Vec::new();
    }
    // D3 timeTicks selects the nearest interval by the ratio to its neighbors.
    let intervals: [(i32, i32, f64); 18] = [
        (0, 1, 1.0),
        (0, 5, 5.0),
        (0, 15, 15.0),
        (0, 30, 30.0),
        (1, 1, 60.0),
        (1, 5, 300.0),
        (1, 15, 900.0),
        (1, 30, 1800.0),
        (2, 1, 3600.0),
        (2, 3, 10800.0),
        (2, 6, 21600.0),
        (2, 12, 43200.0),
        (3, 1, 86400.0),
        (3, 2, 172800.0),
        (4, 1, 604800.0),
        (5, 1, 2592000.0),
        (5, 3, 7776000.0),
        (6, 1, 31536000.0),
    ];
    let target = (data.end - data.start) / 10.0;
    let next = intervals
        .iter()
        .position(|v| v.2 > target)
        .unwrap_or(intervals.len() - 1);
    let selected = if next > 0 && target / intervals[next - 1].2 < intervals[next].2 / target {
        next - 1
    } else {
        next
    };
    let (mut kind, mut every, _) = intervals[selected];
    if let Some(custom) = option(&data.options, "tickInterval").and_then(serde_json::Value::as_str)
    {
        let n = custom.find(|c: char| !c.is_ascii_digit()).unwrap_or(0);
        if let Ok(step) = custom[..n].parse::<i32>() {
            let choice = match &custom[n..] {
                "second" => Some(0),
                "minute" => Some(1),
                "hour" => Some(2),
                "day" => Some(3),
                "week" => Some(4),
                "month" => Some(5),
                _ => None,
            };
            if let Some(choice) = choice.filter(|_| step > 0) {
                kind = choice;
                every = step;
            }
        }
    }
    let axis = option(&data.options, "axisFormat")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(if format == "D" { "%d" } else { "%Y-%m-%d" });
    let weekday = match option(&data.options, "weekday")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("sunday")
    {
        "monday" => 1,
        "tuesday" => 2,
        "wednesday" => 3,
        "thursday" => 4,
        "friday" => 5,
        "saturday" => 6,
        _ => 0,
    };
    let mut values = Vec::new();
    if kind <= 2 {
        let seconds = every as f64 * [1.0, 60.0, 3600.0][kind as usize];
        let mut t = (data.start / seconds).ceil() * seconds;
        for _ in 0..10000 {
            if t > data.end {
                break;
            }
            values.push(t);
            t += seconds;
        }
    } else {
        let first = (data.start / 86400.0).floor() as i32;
        let last = (data.end / 86400.0).floor() as i32;
        // Calendar filters deliberately reset days at month boundaries, as
        // timeDay.every does; weeks retain their selected weekday anchor.
        for day in first..=last.min(first.saturating_add(100000)) {
            let (y, m, d) = civil_from_days(day);
            let valid = match kind {
                3 => (d as i32 - 1).rem_euclid(every) == 0,
                4 => {
                    (day + 4).rem_euclid(7) == weekday
                        && ((day - (weekday - 4)).div_euclid(7)).rem_euclid(every) == 0
                }
                5 => d == 1 && (m as i32 - 1).rem_euclid(every) == 0,
                _ => d == 1 && m == 1 && y.rem_euclid(every) == 0,
            };
            let t = day as f64 * 86400.0;
            if valid && t >= data.start {
                values.push(t);
                if values.len() >= 10000 {
                    break;
                }
            }
        }
    }
    values
        .into_iter()
        .map(|t| {
            let (y, m, d) = civil_from_days((t / 86400.0).floor() as i32);
            let clock = t.rem_euclid(86400.0) as i32;
            let mut label = axis.to_string();
            for (token, value) in [
                ("%Y", format!("{y:04}")),
                ("%m", format!("{m:02}")),
                ("%d", format!("{d:02}")),
                ("%H", format!("{:02}", clock / 3600)),
                ("%M", format!("{:02}", clock / 60 % 60)),
                ("%S", format!("{:02}", clock % 60)),
            ] {
                label = label.replace(token, &value);
            }
            GanttTick {
                x: data.x(t) + 0.5,
                label,
            }
        })
        .collect()
}

pub(super) fn compute_gantt_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    let options = &graph.appearance_config;
    let num = |key: &str, fallback: f32| {
        option(options, key)
            .and_then(serde_json::Value::as_f64)
            .map(|v| v as f32)
            .unwrap_or(fallback)
    };
    let format = option(options, "dateFormat")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("YYYY-MM-DD");
    let working_days = WorkingDays::new(options, format);
    let today = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| (v.as_secs() / 86400) as f64 * 86400.0)
        .unwrap_or(0.0);
    let mut rows = Vec::new();
    let mut auto_id = 0;
    let mut sections: Vec<(String, usize, usize)> = Vec::new();
    let mut specs = Vec::new();
    let mut normal_row = 0;
    for task in &graph.gantt_tasks {
        let tokens: Vec<_> = task
            .raw_meta
            .split(',')
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .collect();
        let done = tokens.contains(&"done");
        let active = tokens.contains(&"active");
        let crit = tokens.contains(&"crit");
        let milestone = tokens.contains(&"milestone");
        let vert = tokens.contains(&"vert");
        let data: Vec<_> = tokens
            .into_iter()
            .filter(|v| !matches!(*v, "done" | "active" | "crit" | "milestone" | "vert"))
            .collect();
        let (id, start, end) = match data.as_slice() {
            [id, start, end] => (id.to_string(), Some(start.to_string()), end.to_string()),
            [start, end] => {
                auto_id += 1;
                (
                    format!("task{auto_id}"),
                    Some(start.to_string()),
                    end.to_string(),
                )
            }
            [end] => {
                auto_id += 1;
                (format!("task{auto_id}"), None, end.to_string())
            }
            _ => {
                auto_id += 1;
                (format!("task{auto_id}"), None, String::new())
            }
        };
        let section_name = task.section.clone().unwrap_or_default();
        let section = sections
            .iter()
            .position(|(name, _, _)| name == &section_name)
            .unwrap_or_else(|| {
                sections.push((section_name, normal_row, 0));
                sections.len() - 1
            });
        let row = normal_row;
        if !vert {
            normal_row += 1;
            sections[section].2 += 1;
        }
        rows.push(NativeGanttTask {
            id,
            label: task.label.clone(),
            start: today,
            end: today,
            render_end: today,
            row,
            section,
            done,
            active,
            crit,
            milestone,
            vert,
        });
        specs.push((start, end));
    }
    // Resolve forward references as well as implicit previous-task chaining.
    for _ in 0..rows.len().saturating_mul(2).max(1) {
        let mut changed = false;
        for i in 0..rows.len() {
            let (start_spec, end_spec) = &specs[i];
            let previous = if i > 0 { rows[i - 1].end } else { today };
            let start = if let Some(value) = start_spec {
                if let Some(ids) = value.strip_prefix("after ") {
                    ids.split_whitespace()
                        .filter_map(|id| rows.iter().find(|t| t.id == id).map(|t| t.end))
                        .reduce(f64::max)
                        .unwrap_or(today)
                } else {
                    date(value, format).unwrap_or(previous)
                }
            } else {
                previous
            };
            let mut end = if let Some(ids) = end_spec.strip_prefix("until ") {
                ids.split_whitespace()
                    .filter_map(|id| rows.iter().find(|t| t.id == id).map(|t| t.start))
                    .reduce(f64::min)
                    .unwrap_or(start)
            } else {
                date(end_spec, format).unwrap_or_else(|| start + duration(end_spec).unwrap_or(0.0))
            };
            let manual_end = end_spec.len() == 10 && parse_gantt_date(end_spec).is_some();
            if manual_end
                && option(options, "inclusiveEndDates").and_then(serde_json::Value::as_bool)
                    == Some(true)
            {
                end += 86400.0;
            }
            let (end, render_end) = if !manual_end && !working_days.excludes.is_empty() {
                working_days.end_dates(start, end)
            } else {
                (end, end)
            };
            changed |= rows[i].start != start || rows[i].end != end;
            rows[i].start = start;
            rows[i].end = end;
            rows[i].render_end = render_end;
        }
        if !changed {
            break;
        }
    }
    let compact =
        option(options, "displayMode").and_then(serde_json::Value::as_str) == Some("compact");
    if compact {
        let mut offset = 0;
        for (section, (_, first, count)) in sections.iter_mut().enumerate() {
            let mut indices: Vec<_> = (0..rows.len())
                .filter(|i| rows[*i].section == section && !rows[*i].vert)
                .collect();
            indices.sort_by(|a, b| rows[*a].start.total_cmp(&rows[*b].start).then(a.cmp(b)));
            let mut timeline: Vec<f64> = Vec::new();
            for i in indices {
                let slot = timeline
                    .iter()
                    .position(|end| *end <= rows[i].start)
                    .unwrap_or_else(|| {
                        timeline.push(f64::NEG_INFINITY);
                        timeline.len() - 1
                    });
                timeline[slot] = rows[i].end;
                rows[i].row = offset + slot;
            }
            *first = offset;
            *count = timeline.len();
            offset += *count;
        }
        normal_row = sections.iter().map(|s| s.2).sum();
    }
    let start = rows
        .iter()
        .map(|t| t.start)
        .reduce(f64::min)
        .unwrap_or(today);
    let mut end = rows
        .iter()
        .map(|t| t.end)
        .reduce(f64::max)
        .unwrap_or(start + 1.0);
    if end <= start {
        end = start + 1.0;
    }
    let width = num("useWidth", 784.0);
    let top = num("topPadding", 50.0);
    let bar = num("barHeight", 20.0);
    let gap = bar + num("barGap", 4.0);
    let height = 2.0 * top + normal_row as f32 * gap;
    let mut native = NativeGanttLayout {
        options: options.clone(),
        width,
        height,
        left: num("leftPadding", 75.0),
        right: num("rightPadding", 75.0),
        top,
        bar_height: bar,
        gap,
        start,
        end,
        tasks: rows,
        sections,
        ticks: Vec::new(),
        excluded: working_days.ranges(start, end),
    };
    native.ticks = calendar_ticks(&native, format);
    let mut layout = compute_gantt_layout_legacy(graph, theme, config);
    layout.width = width;
    layout.height = height;
    if let DiagramData::Gantt(data) = &mut layout.diagram {
        data.native = Some(native);
    }
    layout
}

fn gantt_palette(theme: &Theme) -> Vec<String> {
    vec![
        theme.primary_border_color.clone(),
        "#0ea5e9".to_string(), // sky-500
        "#10b981".to_string(), // emerald-500
        "#6366f1".to_string(), // indigo-500
        "#f97316".to_string(), // orange-500
    ]
}

fn hsl_color(h: f32, s: f32, l: f32) -> String {
    format!("hsl({:.10}, {:.10}%, {:.10}%)", h, s, l)
}

fn shift_color(color: &str, target_s: f32, target_l: f32, strength: f32) -> String {
    let Some((_h, s, l)) = parse_color_to_hsl(color) else {
        return color.to_string();
    };
    let delta_s = (target_s - s) * strength;
    let delta_l = (target_l - l) * strength;
    adjust_color(color, 0.0, delta_s, delta_l)
}

fn gantt_section_palette(theme: &Theme, sections: &[String]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if sections.is_empty() {
        return map;
    }
    let base = theme.primary_border_color.as_str();
    let step = 360.0 / sections.len().max(1) as f32;
    for (idx, name) in sections.iter().enumerate() {
        let hue_shift = step * idx as f32;
        let mut color = adjust_color(base, hue_shift, 0.0, 0.0);
        color = shift_color(&color, 60.0, 55.0, 0.4);
        map.insert(name.clone(), color);
    }
    map
}

fn gantt_task_color(status: Option<crate::ir::GanttStatus>, base: &str, fallback: &str) -> String {
    let base = if parse_color_to_hsl(base).is_some() {
        base.to_string()
    } else {
        fallback.to_string()
    };
    match status {
        Some(crate::ir::GanttStatus::Done) => shift_color(&base, 30.0, 80.0, 0.7),
        Some(crate::ir::GanttStatus::Active) => shift_color(&base, 70.0, 52.0, 0.6),
        Some(crate::ir::GanttStatus::Crit) => {
            if let Some((_, s, l)) = parse_color_to_hsl(&base) {
                hsl_color(0.0, s.max(65.0), l.clamp(45.0, 60.0))
            } else {
                "#ef4444".to_string()
            }
        }
        Some(crate::ir::GanttStatus::Milestone) => {
            if let Some((_, s, l)) = parse_color_to_hsl(&base) {
                hsl_color(45.0, s.max(65.0), l.clamp(50.0, 65.0))
            } else {
                "#f59e0b".to_string()
            }
        }
        None => base,
    }
}

fn parse_gantt_duration(value: &str) -> Option<f32> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let mut digits = String::new();
    let mut unit = None;
    for ch in value.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            digits.push(ch);
        } else if !ch.is_whitespace() {
            unit = Some(ch.to_ascii_lowercase());
        }
    }
    let number: f32 = digits.parse().ok()?;
    let mult = match unit {
        Some('d') => 1.0,
        Some('w') => 7.0,
        Some('h') => 1.0 / 24.0,
        Some('m') => 30.0,
        Some('y') => 365.0,
        _ => 1.0,
    };
    Some(number * mult)
}

fn parse_gantt_date(value: &str) -> Option<i32> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let parts: Vec<&str> = value
        .split(|ch| ch == '-' || ch == '/' || ch == '.')
        .collect();
    if parts.len() != 3 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;
    if month == 0 || month > 12 || day == 0 || day > 31 {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i32 {
    let y = year - (month <= 2) as i32;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = month as i32;
    let d = day as i32;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(days: i32) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + (m <= 2) as i32;
    (year, m as u32, d as u32)
}

fn format_gantt_date(days: i32) -> String {
    let (year, month, day) = civil_from_days(days);
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn compute_gantt_layout_legacy(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    let padding = theme.font_size * 1.25;
    let row_height = (theme.font_size * 1.5).max(theme.font_size + 8.0);
    let label_gap = theme.font_size * 1.05;
    let default_duration = 3.0_f32;

    let title = graph
        .gantt_title
        .as_ref()
        .map(|t| measure_label(t, theme, config));
    let title_height = title.as_ref().map(|t| t.height + padding).unwrap_or(0.0);

    let mut task_label_width = 0.0_f32;
    let mut section_label_width = 0.0_f32;
    for task in &graph.gantt_tasks {
        let label = measure_label(&task.label, theme, config);
        task_label_width = task_label_width.max(label.width);
        if let Some(section) = task.section.as_ref() {
            let section_label = measure_label(section, theme, config);
            section_label_width = section_label_width.max(section_label.width);
        }
    }
    task_label_width = task_label_width.max(theme.font_size * 6.5);

    let label_x = padding;
    let section_task_gap = if section_label_width > 0.0 {
        theme.font_size * 0.8
    } else {
        0.0
    };
    let label_width = section_label_width + section_task_gap + task_label_width;
    let section_label_x = label_x;
    let task_label_x = label_x + section_label_width + section_task_gap;
    let chart_x = padding + label_width + label_gap;
    let chart_y = title_height + padding;
    let chart_width = theme.font_size * 26.0;

    let mut parsed_starts: HashMap<String, f32> = HashMap::new();
    let mut origin: Option<f32> = None;
    for task in &graph.gantt_tasks {
        if let Some(start) = task.start.as_deref().and_then(parse_gantt_date) {
            let start = start as f32;
            parsed_starts.insert(task.id.clone(), start);
            origin = Some(origin.map_or(start, |v| v.min(start)));
        }
    }
    let has_dates = origin.is_some();

    let mut timing: HashMap<String, (f32, f32)> = HashMap::new();
    let mut cursor = 0.0_f32;
    let mut time_start = f32::MAX;
    let mut time_end = f32::MIN;

    let mut computed: Vec<(
        String,
        f32,
        f32,
        Option<crate::ir::GanttStatus>,
        Option<String>,
    )> = Vec::with_capacity(graph.gantt_tasks.len());
    for task in &graph.gantt_tasks {
        let duration = task
            .duration
            .as_deref()
            .and_then(parse_gantt_duration)
            .unwrap_or(default_duration)
            .max(0.1);
        let mut start = parsed_starts.get(&task.id).copied();
        if start.is_none() {
            if let Some(after_id) = task.after.as_deref() {
                if let Some((_, end)) = timing.get(after_id) {
                    start = Some(*end);
                }
            }
        }
        let fallback_base = origin.unwrap_or(0.0);
        let start = start.unwrap_or(fallback_base + cursor);
        let end = start + duration;
        timing.insert(task.id.clone(), (start, end));
        cursor = cursor.max(end + 0.5);
        time_start = time_start.min(start);
        time_end = time_end.max(end);
        computed.push((
            task.label.clone(),
            start,
            duration,
            task.status,
            task.section.clone(),
        ));
    }
    if !time_start.is_finite() || !time_end.is_finite() {
        time_start = 0.0;
        time_end = 1.0;
    }
    if (time_end - time_start).abs() < 0.01 {
        time_end = time_start + 1.0;
    }
    let time_span = (time_end - time_start).max(1.0);
    let time_scale = chart_width / time_span;

    let mut ticks: Vec<GanttTick> = Vec::new();
    let tick_count = 4;
    for i in 0..=tick_count {
        let t = time_start + time_span * (i as f32) / (tick_count as f32);
        let x = chart_x + (t - time_start) * time_scale;
        let label = if has_dates {
            format_gantt_date(t.round() as i32)
        } else {
            format!("{:.0}", t - time_start)
        };
        ticks.push(GanttTick { x, label });
    }

    let palette = gantt_palette(theme);
    let section_palette = gantt_section_palette(theme, &graph.gantt_sections);
    let mut current_section: Option<String> = None;
    let mut current_section_idx: Option<usize> = None;
    let mut sections: Vec<GanttSectionLayout> = Vec::new();
    let mut tasks: Vec<GanttTaskLayout> = Vec::new();
    let mut y = chart_y;

    for (idx, (label, start, duration, status, section)) in computed.iter().enumerate() {
        if section != &current_section {
            if let Some(sec) = section.as_ref() {
                if let Some(prev_idx) = current_section_idx {
                    let height = (y - sections[prev_idx].y).max(row_height);
                    sections[prev_idx].height = height;
                }
                let base_color = section_palette
                    .get(sec)
                    .cloned()
                    .unwrap_or_else(|| palette[idx % palette.len()].clone());
                let band_color = shift_color(&base_color, 20.0, 92.0, 0.7);
                sections.push(GanttSectionLayout {
                    label: measure_label(sec, theme, config),
                    y,
                    height: 0.0,
                    color: base_color,
                    band_color,
                });
                current_section_idx = Some(sections.len() - 1);
            } else if let Some(prev_idx) = current_section_idx {
                let height = (y - sections[prev_idx].y).max(row_height);
                sections[prev_idx].height = height;
                current_section_idx = None;
            }
            current_section = section.clone();
        }

        let bar_x = chart_x + (start - time_start) * time_scale;
        let mut bar_width = duration * time_scale;
        let min_width = row_height * 0.5;
        if bar_width < min_width {
            bar_width = min_width;
        }
        let base_color = if let Some(sec) = section.as_ref() {
            section_palette
                .get(sec)
                .cloned()
                .unwrap_or_else(|| palette[idx % palette.len()].clone())
        } else {
            palette[idx % palette.len()].clone()
        };
        let color = gantt_task_color(*status, &base_color, &palette[0]);

        tasks.push(GanttTaskLayout {
            label: measure_label(label, theme, config),
            x: bar_x,
            y,
            width: bar_width,
            height: row_height,
            color,
            start: *start,
            duration: *duration,
            status: *status,
        });
        y += row_height;
    }
    if let Some(prev_idx) = current_section_idx {
        let height = (y - sections[prev_idx].y).max(row_height);
        sections[prev_idx].height = height;
    }

    let tick_font = theme.font_size * 0.8;
    let max_tick_half_width = ticks
        .iter()
        .map(|tick| {
            measure_label_with_font_size(
                tick.label.as_str(),
                tick_font,
                config,
                false,
                theme.font_family.as_str(),
            )
            .width
                / 2.0
        })
        .fold(0.0_f32, f32::max);
    let axis_pad = row_height * 0.9 + theme.font_size;
    let height = y + padding + axis_pad;
    let width = (chart_x + chart_width + padding)
        .max(chart_x + chart_width + max_tick_half_width + padding * 0.4);

    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        acc_title: None,
        acc_descr: None,
        diagram: DiagramData::Gantt(GanttLayout {
            native: None,
            title,
            sections,
            tasks,
            time_start,
            time_end,
            chart_x,
            chart_y,
            chart_width,
            chart_height: y - chart_y,
            row_height,
            label_x,
            label_width,
            section_label_x,
            section_label_width,
            task_label_x,
            task_label_width,
            title_y: chart_y - row_height * 0.6,
            ticks,
            today_x: None,
        }),
        width,
        height,
    }
}
