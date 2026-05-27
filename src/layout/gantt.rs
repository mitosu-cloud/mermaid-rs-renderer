use super::*;

const GANTT_WIDTH: f32 = 784.0;
const LEFT_PADDING: f32 = 75.0;
const RIGHT_PADDING: f32 = 75.0;
const TOP_PADDING: f32 = 50.0;
const TITLE_TOP_MARGIN: f32 = 25.0;
const BAR_HEIGHT: f32 = 20.0;
const BAR_GAP: f32 = 4.0;
const GRID_LINE_START_PADDING: f32 = 35.0;
const GANTT_FONT_SIZE: f32 = 11.0;
const SECTION_STYLES: usize = 4;
const MINUTES_PER_DAY: f32 = 1_440.0;

#[derive(Debug, Clone)]
struct CompiledGanttTask {
    id: String,
    label: String,
    section: String,
    order: usize,
    start: f32,
    end: f32,
    render_end: Option<f32>,
    active: bool,
    done: bool,
    crit: bool,
    milestone: bool,
    vert: bool,
    status: Option<crate::ir::GanttStatus>,
}

fn gantt_date_format(graph: &Graph) -> String {
    graph
        .gantt_date_format
        .clone()
        .unwrap_or_else(|| "YYYY-MM-DD".to_string())
}

fn gantt_axis_format(graph: &Graph, date_format: &str) -> String {
    graph.gantt_axis_format.clone().unwrap_or_else(|| {
        if date_format.trim() == "D" {
            "%d".to_string()
        } else {
            "%Y-%m-%d".to_string()
        }
    })
}

fn gantt_uses_clock(date_format: &str) -> bool {
    let format = date_format.trim();
    format.contains("HH") || format.contains("%H")
}

fn parse_gantt_duration(value: &str, clock_units: bool) -> Option<f32> {
    let value = value.trim();
    let unit_start = value.find(|ch: char| ch.is_ascii_alphabetic())?;
    let (number, unit) = value.split_at(unit_start);
    if number.is_empty() || unit.is_empty() {
        return None;
    }
    let number: f32 = number.parse().ok()?;
    let multiplier = if clock_units {
        match unit {
            "ms" => 1.0 / 60_000.0,
            "s" => 1.0 / 60.0,
            "m" => 1.0,
            "h" => 60.0,
            "d" => 24.0 * 60.0,
            "w" => 7.0 * 24.0 * 60.0,
            "M" => 30.0 * 24.0 * 60.0,
            "y" => 365.0 * 24.0 * 60.0,
            _ => return None,
        }
    } else {
        match unit {
            "ms" => 1.0 / 86_400_000.0,
            "s" => 1.0 / 86_400.0,
            "m" => 1.0 / 1_440.0,
            "h" => 1.0 / 24.0,
            "d" => 1.0,
            "w" => 7.0,
            "M" => 30.0,
            "y" => 365.0,
            _ => return None,
        }
    };
    Some(number * multiplier)
}

fn parse_gantt_time(value: &str, date_format: &str) -> Option<f32> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if gantt_uses_clock(date_format) {
        let (hours, minutes) = value.split_once(':')?;
        let hours: f32 = hours.trim().parse().ok()?;
        let minutes: f32 = minutes.trim().parse().ok()?;
        return Some(hours * 60.0 + minutes);
    }
    parse_gantt_date(value).map(|days| days as f32)
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

fn format_gantt_time(minutes: f32) -> String {
    let total = minutes.round() as i32;
    let hours = total.div_euclid(60).rem_euclid(24);
    let mins = total.rem_euclid(60);
    format!("{:02}:{:02}", hours, mins)
}

fn format_tick(value: f32, axis_format: &str, clock_units: bool) -> String {
    if clock_units {
        return format_gantt_time(value);
    }
    if axis_format.trim() == "%d" {
        let (_, _, day) = civil_from_days(value.round() as i32);
        return format!("{day:02}");
    }
    format_gantt_date(value.round() as i32)
}

fn first_weekday_of_month(year: i32, month: u32, target_iso: i32) -> i32 {
    let first = days_from_civil(year, month, 1);
    let delta = (target_iso - iso_weekday(first)).rem_euclid(7);
    first + delta
}

fn second_sunday_in_march(year: i32) -> i32 {
    first_weekday_of_month(year, 3, 7) + 7
}

fn first_sunday_in_november(year: i32) -> i32 {
    first_weekday_of_month(year, 11, 7)
}

fn local_dst_active_at_midnight_or_time(value: f32) -> bool {
    let day = value.floor() as i32;
    let fraction = value - day as f32;
    let (year, _, _) = civil_from_days(day);
    let start_day = second_sunday_in_march(year);
    let end_day = first_sunday_in_november(year);
    if day > start_day && day < end_day {
        return true;
    }
    if day == start_day {
        return fraction >= 2.0 / 24.0;
    }
    if day == end_day {
        return fraction < 2.0 / 24.0;
    }
    false
}

fn local_date_scale_minutes(value: f32) -> f32 {
    value * MINUTES_PER_DAY
        - if local_dst_active_at_midnight_or_time(value) {
            60.0
        } else {
            0.0
        }
}

fn iso_weekday(days: i32) -> i32 {
    (days + 3).rem_euclid(7) + 1
}

fn weekday_name(iso: i32) -> &'static str {
    match iso {
        1 => "monday",
        2 => "tuesday",
        3 => "wednesday",
        4 => "thursday",
        5 => "friday",
        6 => "saturday",
        _ => "sunday",
    }
}

fn weekday_iso(name: &str) -> i32 {
    match name {
        "monday" => 1,
        "tuesday" => 2,
        "wednesday" => 3,
        "thursday" => 4,
        "friday" => 5,
        "saturday" => 6,
        _ => 7,
    }
}

fn is_invalid_gantt_day(
    days: i32,
    excludes: &[String],
    includes: &[String],
    weekend: &str,
) -> bool {
    let date = format_gantt_date(days);
    let iso = iso_weekday(days);
    let day_name = weekday_name(iso);
    if includes
        .iter()
        .any(|item| item == &date || item == day_name)
    {
        return false;
    }
    let weekend_start = weekday_iso(weekend);
    if excludes.iter().any(|item| item == "weekends")
        && (iso == weekend_start || iso == weekend_start % 7 + 1)
    {
        return true;
    }
    excludes
        .iter()
        .any(|item| item == &date || item == day_name)
}

fn fix_task_dates(
    start: f32,
    end: f32,
    manual_end_time: bool,
    graph: &Graph,
    clock_units: bool,
) -> (f32, Option<f32>) {
    if clock_units || graph.gantt_excludes.is_empty() || manual_end_time {
        return (end, None);
    }
    let weekend = graph.gantt_weekend.as_deref().unwrap_or("saturday").trim();
    let mut check = start.floor() as i32 + 1;
    let mut fixed_end = end;
    let mut render_end = None;
    let mut previous_invalid = false;
    while (check as f32) <= fixed_end {
        if !previous_invalid {
            render_end = Some(fixed_end);
        }
        let invalid =
            is_invalid_gantt_day(check, &graph.gantt_excludes, &graph.gantt_includes, weekend);
        if invalid {
            fixed_end += 1.0;
        }
        previous_invalid = invalid;
        check += 1;
    }
    (fixed_end, render_end)
}

fn resolve_after_start(ids: &[String], timings: &HashMap<String, (f32, f32)>) -> Option<f32> {
    ids.iter()
        .filter_map(|id| timings.get(id).map(|(_, end)| *end))
        .reduce(f32::max)
}

fn resolve_until_end(ids: &[String], timings: &HashMap<String, (f32, f32)>) -> Option<f32> {
    ids.iter()
        .filter_map(|id| timings.get(id).map(|(start, _)| *start))
        .reduce(f32::min)
}

fn compile_gantt_tasks(graph: &Graph, date_format: &str) -> Vec<CompiledGanttTask> {
    let clock_units = gantt_uses_clock(date_format);
    let mut compiled: HashMap<String, CompiledGanttTask> = HashMap::new();
    let mut timings: HashMap<String, (f32, f32)> = HashMap::new();
    let max_depth = 10;

    for _ in 0..=max_depth {
        let mut changed = false;
        for (idx, task) in graph.gantt_tasks.iter().enumerate() {
            if compiled.contains_key(&task.id) {
                continue;
            }

            let start = if let Some(start) = task.start.as_deref() {
                parse_gantt_time(start, date_format)
            } else if !task.after_ids.is_empty() {
                resolve_after_start(&task.after_ids, &timings)
            } else if let Some(prev) = idx
                .checked_sub(1)
                .and_then(|prev_idx| graph.gantt_tasks.get(prev_idx))
            {
                timings.get(&prev.id).map(|(_, end)| *end)
            } else {
                Some(0.0)
            };

            let Some(start) = start else {
                continue;
            };

            let end = if !task.until_ids.is_empty() {
                resolve_until_end(&task.until_ids, &timings)
            } else if let Some(end) = task.end.as_deref() {
                parse_gantt_time(end, date_format).map(|mut parsed| {
                    if graph.gantt_inclusive_end_dates && !clock_units {
                        parsed += 1.0;
                    }
                    parsed
                })
            } else if let Some(duration) = task.duration.as_deref() {
                parse_gantt_duration(duration, clock_units).map(|duration| start + duration)
            } else {
                Some(start + if clock_units { 60.0 } else { 1.0 })
            };

            let Some(end) = end else {
                continue;
            };

            let manual_end_time = task.end.as_deref().and_then(parse_gantt_date).is_some();
            let (end, render_end) = fix_task_dates(start, end, manual_end_time, graph, clock_units);
            timings.insert(task.id.clone(), (start, end));
            compiled.insert(
                task.id.clone(),
                CompiledGanttTask {
                    id: task.id.clone(),
                    label: task.label.clone(),
                    section: task.section.clone().unwrap_or_default(),
                    order: task.order,
                    start,
                    end,
                    render_end,
                    active: task.active,
                    done: task.done,
                    crit: task.crit,
                    milestone: task.milestone,
                    vert: task.vert,
                    status: task.status,
                },
            );
            changed = true;
        }
        if compiled.len() == graph.gantt_tasks.len() || !changed {
            break;
        }
    }

    graph
        .gantt_tasks
        .iter()
        .filter_map(|task| compiled.get(&task.id).cloned())
        .collect()
}

fn unique_categories(tasks: &[CompiledGanttTask]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut categories = Vec::new();
    for task in tasks {
        if seen.insert(task.section.clone()) {
            categories.push(task.section.clone());
        }
    }
    categories
}

fn apply_compact_orders(
    tasks: &mut [CompiledGanttTask],
    categories: &[String],
) -> HashMap<String, usize> {
    let mut category_heights = HashMap::new();
    let mut order_offset = 0usize;
    for category in categories {
        let mut indices: Vec<usize> = tasks
            .iter()
            .enumerate()
            .filter_map(|(idx, task)| (task.section == *category).then_some(idx))
            .collect();
        indices.sort_by(|a, b| {
            tasks[*a]
                .start
                .partial_cmp(&tasks[*b].start)
                .unwrap_or(Ordering::Equal)
                .then_with(|| tasks[*a].order.cmp(&tasks[*b].order))
        });
        let mut timeline = vec![f32::NEG_INFINITY; indices.len().max(1)];
        let mut max_lane = 0usize;
        for idx in indices {
            for (lane, lane_end) in timeline.iter_mut().enumerate() {
                if tasks[idx].start >= *lane_end {
                    *lane_end = tasks[idx].end;
                    tasks[idx].order = lane + order_offset;
                    max_lane = max_lane.max(lane);
                    break;
                }
            }
        }
        let height = max_lane + 1;
        category_heights.insert(category.clone(), height);
        order_offset += height;
    }
    category_heights
}

fn category_heights(tasks: &[CompiledGanttTask], categories: &[String]) -> HashMap<String, usize> {
    let mut heights = HashMap::new();
    for category in categories {
        heights.insert(
            category.clone(),
            tasks
                .iter()
                .filter(|task| task.section == *category)
                .count(),
        );
    }
    heights
}

fn scale_time(value: f32, start: f32, end: f32, clock_units: bool) -> f32 {
    let (value, start, end) = if clock_units {
        (value, start, end)
    } else {
        (
            local_date_scale_minutes(value),
            local_date_scale_minutes(start),
            local_date_scale_minutes(end),
        )
    };
    let span = (end - start).max(f32::EPSILON);
    (((value - start) / span) * (GANTT_WIDTH - LEFT_PADDING - RIGHT_PADDING)).round()
}

fn build_gantt_ticks(
    graph: &Graph,
    time_start: f32,
    time_end: f32,
    date_format: &str,
) -> Vec<GanttTick> {
    if graph.gantt_tasks.is_empty() {
        return Vec::new();
    }
    let clock_units = gantt_uses_clock(date_format);
    let axis_format = gantt_axis_format(graph, date_format);
    let mut tick_values = Vec::new();

    if let Some(interval) = graph.gantt_tick_interval.as_deref() {
        if let Some((every, unit)) = parse_tick_interval(interval) {
            match unit {
                "minute" if clock_units => {
                    let mut value = (time_start / every as f32).ceil() * every as f32;
                    while value <= time_end {
                        tick_values.push(value);
                        value += every as f32;
                    }
                }
                "hour" if clock_units => {
                    let step = every as f32 * 60.0;
                    let mut value = (time_start / step).ceil() * step;
                    while value <= time_end {
                        tick_values.push(value);
                        value += step;
                    }
                }
                "day" if !clock_units => {
                    let mut value = time_start.ceil();
                    while value < time_end {
                        tick_values.push(value);
                        value += every as f32;
                    }
                }
                "week" if !clock_units => {
                    let weekday = graph.gantt_weekday.as_deref().unwrap_or("sunday");
                    let target = weekday_iso(weekday);
                    let mut value = time_start.ceil() as i32;
                    while iso_weekday(value) != target {
                        value += 1;
                    }
                    while (value as f32) <= time_end {
                        tick_values.push(value as f32);
                        value += every as i32 * 7;
                    }
                }
                _ => {}
            }
        }
    }

    if tick_values.is_empty() {
        if clock_units {
            let step = if time_end - time_start <= 90.0 {
                5.0
            } else {
                15.0
            };
            let mut value = (time_start / step).ceil() * step;
            while value <= time_end {
                tick_values.push(value);
                value += step;
            }
        } else {
            let span = time_end - time_start;
            if span <= 10.0 {
                let mut value = time_start.ceil();
                while value < time_end {
                    tick_values.push(value);
                    value += 1.0;
                }
            } else if span <= 35.0 {
                let mut value = time_start.ceil() as i32;
                while civil_from_days(value).2 % 2 == 0 {
                    value += 1;
                }
                while (value as f32) <= time_end {
                    tick_values.push(value as f32);
                    value += 2;
                }
            } else {
                let mut value = time_start.ceil() as i32;
                while iso_weekday(value) != 7 {
                    value += 1;
                }
                while (value as f32) <= time_end {
                    tick_values.push(value as f32);
                    value += 7;
                }
            }
        }
    }

    tick_values
        .into_iter()
        .map(|value| GanttTick {
            x: LEFT_PADDING + scale_time(value, time_start, time_end, clock_units) + 0.5,
            label: format_tick(value, &axis_format, clock_units),
        })
        .collect()
}

fn parse_tick_interval(value: &str) -> Option<(usize, &str)> {
    let value = value.trim();
    let unit_start = value.find(|ch: char| ch.is_ascii_alphabetic())?;
    let (number, unit) = value.split_at(unit_start);
    Some((number.parse().ok()?, unit))
}

fn build_exclude_ranges(
    graph: &Graph,
    time_start: f32,
    time_end: f32,
    height: f32,
    clock_units: bool,
) -> Vec<GanttExcludeRange> {
    if clock_units || (graph.gantt_excludes.is_empty() && graph.gantt_includes.is_empty()) {
        return Vec::new();
    }
    let weekend = graph.gantt_weekend.as_deref().unwrap_or("saturday").trim();
    let mut ranges = Vec::new();
    let mut range_start: Option<i32> = None;
    let mut range_end: Option<i32> = None;
    let mut day = time_start.floor() as i32;
    while (day as f32) <= time_end {
        if is_invalid_gantt_day(day, &graph.gantt_excludes, &graph.gantt_includes, weekend) {
            if range_start.is_none() {
                range_start = Some(day);
            }
            range_end = Some(day);
        } else if let (Some(start), Some(end)) = (range_start.take(), range_end.take()) {
            ranges.push(build_exclude_range(
                start, end, time_start, time_end, height,
            ));
        }
        day += 1;
    }
    if let (Some(start), Some(end)) = (range_start, range_end) {
        ranges.push(build_exclude_range(
            start, end, time_start, time_end, height,
        ));
    }
    ranges
}

fn build_exclude_range(
    start: i32,
    end: i32,
    time_start: f32,
    time_end: f32,
    height: f32,
) -> GanttExcludeRange {
    let x = LEFT_PADDING + scale_time(start as f32, time_start, time_end, false);
    let width = scale_time(end as f32 + 1.0, time_start, time_end, false)
        - scale_time(start as f32, time_start, time_end, false);
    GanttExcludeRange {
        x,
        y: GRID_LINE_START_PADDING,
        width,
        height: height - TOP_PADDING - GRID_LINE_START_PADDING,
    }
}

fn task_text_width(label: &str, theme: &Theme) -> f32 {
    text_metrics::get_computed_text_length(label.trim(), GANTT_FONT_SIZE, &theme.font_family)
}

fn task_fill(task: &CompiledGanttTask) -> String {
    if task.done {
        "lightgrey".to_string()
    } else if task.active {
        "#bfc7ff".to_string()
    } else if task.crit {
        "red".to_string()
    } else {
        "#8a90dd".to_string()
    }
}

pub(super) fn compute_gantt_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    let date_format = gantt_date_format(graph);
    let clock_units = gantt_uses_clock(&date_format);
    let mut compiled = compile_gantt_tasks(graph, &date_format);
    let categories = unique_categories(&compiled);
    let compact = graph
        .gantt_display_mode
        .as_deref()
        .is_some_and(|mode| mode.eq_ignore_ascii_case("compact"));
    let heights = if compact {
        apply_compact_orders(&mut compiled, &categories)
    } else {
        category_heights(&compiled, &categories)
    };

    let row_count = if compact {
        heights.values().sum::<usize>()
    } else {
        compiled.len()
    };
    let height = 2.0 * TOP_PADDING + row_count as f32 * (BAR_HEIGHT + BAR_GAP);

    let mut time_start = compiled
        .iter()
        .map(|task| task.start)
        .fold(f32::INFINITY, f32::min);
    let mut time_end = compiled
        .iter()
        .map(|task| task.end)
        .fold(f32::NEG_INFINITY, f32::max);
    if !time_start.is_finite() || !time_end.is_finite() || time_start == time_end {
        time_start = 0.0;
        time_end = 1.0;
    }

    compiled.sort_by(|a, b| {
        if a.vert != b.vert {
            return if a.vert {
                Ordering::Greater
            } else {
                Ordering::Less
            };
        }
        a.start
            .partial_cmp(&b.start)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.order.cmp(&b.order))
    });

    let mut sections = Vec::new();
    let mut prev_gap = 0usize;
    for (idx, category) in categories.iter().enumerate() {
        let category_height = heights.get(category).copied().unwrap_or(0);
        let label_y = (category_height as f32 * (BAR_HEIGHT + BAR_GAP)) / 2.0
            + prev_gap as f32 * (BAR_HEIGHT + BAR_GAP)
            + TOP_PADDING;
        if !category.is_empty() {
            sections.push(GanttSectionLayout {
                label: measure_label_with_font_size(
                    category,
                    GANTT_FONT_SIZE,
                    config,
                    false,
                    theme.font_family.as_str(),
                ),
                y: label_y,
                height: category_height as f32 * (BAR_HEIGHT + BAR_GAP),
                color: "#333".to_string(),
                band_color: section_band_color(idx),
                index: idx % SECTION_STYLES,
            });
        }
        prev_gap += category_height;
    }

    let tasks = compiled
        .iter()
        .map(|task| {
            let section_index = categories
                .iter()
                .position(|category| category == &task.section)
                .unwrap_or(0)
                % SECTION_STYLES;
            let row_y = if task.vert {
                GRID_LINE_START_PADDING
            } else {
                task.order as f32 * (BAR_HEIGHT + BAR_GAP) + TOP_PADDING
            };
            let start_scaled = scale_time(task.start, time_start, time_end, clock_units);
            let end_for_width = task.render_end.unwrap_or(task.end);
            let end_scaled = scale_time(end_for_width, time_start, time_end, clock_units);
            let raw_end_scaled = scale_time(task.end, time_start, time_end, clock_units);
            let mut x = LEFT_PADDING + start_scaled;
            let mut width = end_scaled - start_scaled;
            if task.milestone {
                x = LEFT_PADDING + start_scaled + 0.5 * (raw_end_scaled - start_scaled)
                    - 0.5 * BAR_HEIGHT;
                width = BAR_HEIGHT;
            } else if task.vert {
                width = 0.08 * BAR_HEIGHT;
            }
            let height = if task.vert {
                graph.gantt_tasks.len() as f32 * (BAR_HEIGHT + BAR_GAP) + BAR_HEIGHT * 2.0
            } else {
                BAR_HEIGHT
            };
            let label = measure_label_with_font_size(
                &task.label,
                GANTT_FONT_SIZE,
                config,
                false,
                theme.font_family.as_str(),
            );
            let text_width = task_text_width(&task.label, theme);
            let text_start_scaled = if task.milestone {
                start_scaled + 0.5 * (raw_end_scaled - start_scaled) - 0.5 * BAR_HEIGHT
            } else {
                start_scaled
            };
            let text_end_scaled = if task.milestone {
                text_start_scaled + BAR_HEIGHT
            } else {
                end_scaled
            };
            let (label_x, label_anchor, label_inside) = if task.vert {
                (LEFT_PADDING + start_scaled, "middle".to_string(), false)
            } else if text_width > width {
                if text_end_scaled + text_width + 1.5 * LEFT_PADDING > GANTT_WIDTH {
                    (x - 5.0, "end".to_string(), false)
                } else {
                    (
                        LEFT_PADDING + text_end_scaled + 5.0,
                        "start".to_string(),
                        false,
                    )
                }
            } else {
                (x + width / 2.0, "middle".to_string(), true)
            };
            let label_y = if task.vert {
                GRID_LINE_START_PADDING
                    + graph.gantt_tasks.len() as f32 * (BAR_HEIGHT + BAR_GAP)
                    + 60.0
            } else {
                task.order as f32 * (BAR_HEIGHT + BAR_GAP)
                    + BAR_HEIGHT / 2.0
                    + (GANTT_FONT_SIZE / 2.0 - 2.0)
                    + TOP_PADDING
            };
            let origin_x = LEFT_PADDING + start_scaled + 0.5 * (raw_end_scaled - start_scaled);
            let origin_y =
                task.order as f32 * (BAR_HEIGHT + BAR_GAP) + TOP_PADDING + 0.5 * BAR_HEIGHT;
            GanttTaskLayout {
                id: task.id.clone(),
                label,
                x,
                y: row_y,
                width,
                height,
                color: task_fill(task),
                start: task.start,
                duration: task.end - task.start,
                status: task.status,
                order: task.order,
                section_index,
                label_x,
                label_y,
                label_anchor,
                label_inside,
                active: task.active,
                done: task.done,
                crit: task.crit,
                milestone: task.milestone,
                vert: task.vert,
                transform_origin_x: origin_x,
                transform_origin_y: origin_y,
            }
        })
        .collect();

    let ticks = build_gantt_ticks(graph, time_start, time_end, &date_format);
    let exclude_ranges = build_exclude_ranges(graph, time_start, time_end, height, clock_units);
    let today_x = if graph.gantt_today_marker.as_deref() == Some("off") {
        None
    } else if clock_units {
        None
    } else {
        let today_days = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| (duration.as_secs() / 86_400) as f32)
            .unwrap_or(0.0);
        Some(LEFT_PADDING + scale_time(today_days, time_start, time_end, clock_units))
    };

    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        acc_title: None,
        acc_descr: None,
        diagram: DiagramData::Gantt(GanttLayout {
            title: graph.gantt_title.as_ref().map(|title| {
                measure_label_with_font_size(title, 18.0, config, false, theme.font_family.as_str())
            }),
            sections,
            tasks,
            exclude_ranges,
            time_start,
            time_end,
            chart_x: LEFT_PADDING,
            chart_y: TOP_PADDING,
            chart_width: GANTT_WIDTH - LEFT_PADDING - RIGHT_PADDING,
            chart_height: height - 2.0 * TOP_PADDING,
            row_height: BAR_HEIGHT + BAR_GAP,
            label_x: 10.0,
            label_width: LEFT_PADDING,
            section_label_x: 10.0,
            section_label_width: LEFT_PADDING,
            task_label_x: 10.0,
            task_label_width: LEFT_PADDING,
            title_y: TITLE_TOP_MARGIN,
            ticks,
            today_x,
        }),
        width: GANTT_WIDTH,
        height,
    }
}

fn section_band_color(index: usize) -> String {
    match index % SECTION_STYLES {
        0 => "#6666ff".to_string(),
        2 => "#fff400".to_string(),
        _ => "#ffffff".to_string(),
    }
}
