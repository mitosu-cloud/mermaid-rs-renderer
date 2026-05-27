use super::*;

const D3_DEFAULT_TICK_COUNT: usize = 10;
const BAR_WIDTH_TO_TICK_WIDTH_RATIO: f32 = 0.7;
const MAX_OUTER_PADDING_PERCENT_FOR_LABEL: f32 = 0.2;
const SVG_TEXT_HEIGHT_FACTOR: f32 = 1.164;

#[derive(Debug, Clone, Copy)]
enum XYTextMetrics {
    Styled,
    SvgDefault,
}

pub(super) fn compute_xychart_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> Layout {
    let data = &graph.xychart;
    let chart_config = &config.xychart;
    let x_axis_config = &chart_config.x_axis;
    let y_axis_config = &chart_config.y_axis;
    let font_family = theme.font_family.as_str();
    let width = chart_config.width;
    let height = chart_config.height;
    let text_metrics = xy_text_metrics(chart_config);

    let all_values: Vec<f32> = data
        .series
        .iter()
        .flat_map(|series| series.values.iter().copied())
        .collect();
    let data_min = all_values.iter().copied().fold(f32::INFINITY, f32::min);
    let data_max = all_values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut y_min = data
        .y_axis_min
        .unwrap_or_else(|| if data_min.is_finite() { data_min } else { 0.0 });
    let mut y_max = data
        .y_axis_max
        .unwrap_or_else(|| if data_max.is_finite() { data_max } else { 1.0 });
    if (y_max - y_min).abs() < f32::EPSILON {
        y_min -= 0.5;
        y_max += 0.5;
    }

    let max_series_len = data
        .series
        .iter()
        .map(|series| series.values.len())
        .max()
        .unwrap_or(0)
        .max(1);
    let has_band_x_axis = !data.x_axis_categories.is_empty();
    let has_bars = data
        .series
        .iter()
        .any(|series| series.kind == crate::ir::XYSeriesKind::Bar);

    let y_tick_values = linear_ticks(y_min, y_max, D3_DEFAULT_TICK_COUNT);
    let y_tick_labels: Vec<String> = y_tick_values
        .iter()
        .rev()
        .map(|value| format_tick(*value))
        .collect();
    let x_tick_labels: Vec<String> = if has_band_x_axis {
        data.x_axis_categories.clone()
    } else {
        linear_ticks(1.0, max_series_len as f32, D3_DEFAULT_TICK_COUNT)
            .into_iter()
            .map(format_tick)
            .collect()
    };

    let title = if chart_config.show_title {
        data.title.as_ref().map(|text| {
            measure_label_with_font_size(
                text,
                chart_config.title_font_size,
                config,
                false,
                font_family,
            )
        })
    } else {
        None
    };
    let x_axis_label = data.x_axis_label.as_ref().map(|text| {
        measure_label_with_font_size(
            text,
            x_axis_config.title_font_size,
            config,
            false,
            font_family,
        )
    });
    let y_axis_label = data.y_axis_label.as_ref().map(|text| {
        measure_label_with_font_size(
            text,
            y_axis_config.title_font_size,
            config,
            false,
            font_family,
        )
    });

    let mut available_width =
        width - ((width * chart_config.plot_reserved_space_percent) / 100.0).floor();
    let mut available_height =
        height - ((height * chart_config.plot_reserved_space_percent) / 100.0).floor();

    let title_height = title
        .as_ref()
        .map(|_| {
            text_height(chart_config.title_font_size, text_metrics)
                + chart_config.title_padding * 2.0
        })
        .unwrap_or(0.0);
    available_height = (available_height - title_height).max(0.0);

    let x_axis_outer_padding = axis_outer_padding_for_horizontal_labels(
        &x_tick_labels,
        x_axis_config.label_font_size,
        available_width,
        config,
        font_family,
        text_metrics,
    );
    let x_axis_height = horizontal_axis_height(x_axis_config, x_axis_label.is_some(), text_metrics);
    available_height = (available_height - x_axis_height).max(0.0);

    let y_axis_outer_padding = axis_outer_padding_for_vertical_labels(
        &y_tick_labels,
        y_axis_config.label_font_size,
        available_height,
        text_metrics,
    );
    let y_axis_width = vertical_axis_width(
        y_axis_config,
        &y_tick_labels,
        y_axis_label.is_some(),
        config,
        font_family,
        text_metrics,
    )
    .min(width * 0.5);
    available_width = (available_width - y_axis_width).max(0.0);

    let mut plot_width =
        ((width * chart_config.plot_reserved_space_percent) / 100.0).floor() + available_width;
    let plot_height =
        ((height * chart_config.plot_reserved_space_percent) / 100.0).floor() + available_height;
    plot_width = plot_width.max(1.0);
    let plot_height = plot_height.max(1.0);
    let plot_x = y_axis_width;
    let plot_y = title_height;

    let mut x_outer_padding = x_axis_outer_padding;
    if has_bars {
        let tick_count = x_tick_labels.len().max(1) as f32;
        let tick_distance = (plot_width - x_outer_padding * 2.0).abs() / tick_count;
        if BAR_WIDTH_TO_TICK_WIDTH_RATIO * tick_distance > x_outer_padding * 2.0 {
            x_outer_padding = ((BAR_WIDTH_TO_TICK_WIDTH_RATIO * tick_distance) / 2.0).floor();
        }
    }

    let x_positions = if has_band_x_axis {
        band_positions(
            plot_x + x_outer_padding,
            plot_x + plot_width - x_outer_padding,
            data.x_axis_categories.len().max(1),
        )
    } else {
        let (x_min, x_max) = linear_x_domain(max_series_len);
        let range_start = plot_x + x_outer_padding;
        let range_end = plot_x + plot_width - x_outer_padding;
        (0..max_series_len)
            .map(|idx| {
                let value = if max_series_len <= 1 {
                    x_min
                } else {
                    x_min + (x_max - x_min) * idx as f32 / (max_series_len - 1) as f32
                };
                linear_scale(value, x_min, x_max, range_start, range_end)
            })
            .collect()
    };

    let y_range_start = plot_y + y_axis_outer_padding;
    let y_range_end = plot_y + plot_height - y_axis_outer_padding;
    let y_scale = |value: f32| linear_scale(value, y_max, y_min, y_range_start, y_range_end);
    let plot_bottom = plot_y + plot_height;
    let tick_distance =
        (plot_width - x_outer_padding * 2.0).abs() / x_tick_labels.len().max(1) as f32;
    let bar_width = (x_outer_padding * 2.0).min(tick_distance).max(1.0) * 0.95;

    let mut bars = Vec::new();
    let mut lines = Vec::new();
    let palette = if theme.xy_chart.plot_colors.is_empty() {
        vec!["#ECECFF".to_string()]
    } else {
        theme.xy_chart.plot_colors.clone()
    };

    for (series_idx, series) in data.series.iter().enumerate() {
        let color = palette[series_idx % palette.len()].clone();
        match series.kind {
            crate::ir::XYSeriesKind::Bar => {
                for (idx, &value) in series.values.iter().enumerate() {
                    let center_x = x_positions
                        .get(idx)
                        .copied()
                        .unwrap_or_else(|| *x_positions.last().unwrap_or(&plot_x));
                    let y = y_scale(value);
                    bars.push(XYChartBarLayout {
                        x: center_x - bar_width / 2.0,
                        y,
                        width: bar_width,
                        height: (plot_bottom - y).max(0.0),
                        value,
                        color: color.clone(),
                    });
                }
            }
            crate::ir::XYSeriesKind::Line => {
                let points = series
                    .values
                    .iter()
                    .enumerate()
                    .map(|(idx, &value)| {
                        let x = x_positions
                            .get(idx)
                            .copied()
                            .unwrap_or_else(|| *x_positions.last().unwrap_or(&plot_x));
                        (x, y_scale(value))
                    })
                    .collect();
                lines.push(XYChartLineLayout { points, color });
            }
        }
    }

    let x_axis_categories = if has_band_x_axis {
        data.x_axis_categories
            .iter()
            .cloned()
            .zip(x_positions.iter().copied())
            .collect()
    } else {
        let (x_min, x_max) = linear_x_domain(max_series_len);
        let range_start = plot_x + x_outer_padding;
        let range_end = plot_x + plot_width - x_outer_padding;
        linear_ticks(x_min, x_max, D3_DEFAULT_TICK_COUNT)
            .into_iter()
            .map(|value| {
                (
                    format_tick(value),
                    linear_scale(value, x_min, x_max, range_start, range_end),
                )
            })
            .collect()
    };

    let y_axis_ticks = y_tick_values
        .iter()
        .rev()
        .map(|value| (format_tick(*value), y_scale(*value)))
        .collect();

    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        acc_title: None,
        acc_descr: None,
        diagram: DiagramData::XYChart(XYChartLayout {
            title,
            title_y: title_height / 2.0,
            x_axis_label,
            x_axis_label_y: plot_y + plot_height + x_axis_height
                - x_axis_config.title_padding
                - text_height(x_axis_config.title_font_size, text_metrics),
            y_axis_label,
            y_axis_label_x: y_axis_config.title_padding,
            x_axis_categories,
            y_axis_ticks,
            bars,
            lines,
            plot_x,
            plot_y,
            plot_width,
            plot_height,
            width,
            height,
        }),
        width,
        height,
    }
}

fn xy_text_metrics(chart_config: &crate::config::XYChartConfig) -> XYTextMetrics {
    let default = crate::config::XYChartConfig::default();
    if (chart_config.width - default.width).abs() > f32::EPSILON
        || (chart_config.height - default.height).abs() > f32::EPSILON
    {
        XYTextMetrics::SvgDefault
    } else {
        XYTextMetrics::Styled
    }
}

fn horizontal_axis_height(
    axis: &crate::config::XYChartAxisConfig,
    has_title: bool,
    metrics: XYTextMetrics,
) -> f32 {
    let mut height = 0.0;
    if axis.show_axis_line {
        height += axis.axis_line_width;
    }
    if axis.show_label {
        height += text_height(axis.label_font_size, metrics) + axis.label_padding * 2.0;
    }
    if axis.show_tick {
        height += axis.tick_length;
    }
    if axis.show_title && has_title {
        height += text_height(axis.title_font_size, metrics) + axis.title_padding * 2.0;
    }
    height
}

fn vertical_axis_width(
    axis: &crate::config::XYChartAxisConfig,
    labels: &[String],
    has_title: bool,
    config: &LayoutConfig,
    font_family: &str,
    metrics: XYTextMetrics,
) -> f32 {
    let mut width = 0.0;
    if axis.show_axis_line {
        width += axis.axis_line_width;
    }
    if axis.show_label {
        let max_label_width = labels
            .iter()
            .map(|label| {
                text_width_for_font(label, axis.label_font_size, config, font_family, metrics)
            })
            .fold(0.0, f32::max);
        width += max_label_width + axis.label_padding * 2.0;
    }
    if axis.show_tick {
        width += axis.tick_length;
    }
    if axis.show_title && has_title {
        width += text_height(axis.title_font_size, metrics) + axis.title_padding * 2.0;
    }
    width
}

fn axis_outer_padding_for_horizontal_labels(
    labels: &[String],
    font_size: f32,
    available_width: f32,
    config: &LayoutConfig,
    font_family: &str,
    metrics: XYTextMetrics,
) -> f32 {
    let max_label_width = labels
        .iter()
        .map(|label| text_width_for_font(label, font_size, config, font_family, metrics))
        .fold(0.0, f32::max);
    (max_label_width / 2.0).min(MAX_OUTER_PADDING_PERCENT_FOR_LABEL * available_width)
}

fn axis_outer_padding_for_vertical_labels(
    labels: &[String],
    font_size: f32,
    available_height: f32,
    metrics: XYTextMetrics,
) -> f32 {
    if labels.is_empty() {
        return 0.0;
    }
    (text_height(font_size, metrics) / 2.0)
        .min(MAX_OUTER_PADDING_PERCENT_FOR_LABEL * available_height)
}

fn text_width_for_font(
    text: &str,
    font_size: f32,
    config: &LayoutConfig,
    font_family: &str,
    metrics: XYTextMetrics,
) -> f32 {
    match metrics {
        XYTextMetrics::Styled => styled_svg_text_width(text, font_size, config, font_family),
        XYTextMetrics::SvgDefault => default_svg_text_width(text, font_size),
    }
}

fn text_height(font_size: f32, metrics: XYTextMetrics) -> f32 {
    match metrics {
        XYTextMetrics::Styled => styled_svg_text_height(font_size),
        XYTextMetrics::SvgDefault => font_size,
    }
}

fn styled_svg_text_width(
    text: &str,
    font_size: f32,
    config: &LayoutConfig,
    font_family: &str,
) -> f32 {
    if is_numeric_tick_label(text) {
        return styled_numeric_text_width(text, font_size);
    }
    if (font_size - 14.0).abs() < f32::EPSILON && text.is_ascii() {
        return styled_axis_label_text_width(text, font_size);
    }
    measure_label_with_font_size(text, font_size, config, false, font_family).width * 0.971
}

fn styled_svg_text_height(font_size: f32) -> f32 {
    if (font_size - 14.0).abs() < f32::EPSILON {
        16.0
    } else if (font_size - 16.0).abs() < f32::EPSILON {
        19.0
    } else if (font_size - 20.0).abs() < f32::EPSILON {
        23.05
    } else {
        font_size * SVG_TEXT_HEIGHT_FACTOR
    }
}

fn is_numeric_tick_label(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, '.' | '-' | '+'))
        && text.chars().any(|ch| ch.is_ascii_digit())
}

fn default_svg_text_width(text: &str, font_size: f32) -> f32 {
    text.chars().map(default_svg_char_width_factor).sum::<f32>() * font_size
}

fn styled_numeric_text_width(text: &str, font_size: f32) -> f32 {
    text.chars()
        .map(styled_numeric_char_width_factor)
        .sum::<f32>()
        * font_size
}

fn styled_numeric_char_width_factor(ch: char) -> f32 {
    match ch {
        '0'..='9' => 0.52445,
        '.' | ',' => 0.3672,
        '-' => 0.3674,
        '+' => 0.5,
        _ => default_svg_char_width_factor(ch),
    }
}

fn styled_axis_label_text_width(text: &str, font_size: f32) -> f32 {
    let has_digit = text.chars().any(|ch| ch.is_ascii_digit());
    let scale = if has_digit { 0.971 } else { 0.995 };
    text.chars()
        .map(|ch| styled_axis_label_char_width_factor(ch, has_digit))
        .sum::<f32>()
        * font_size
        * scale
}

fn styled_axis_label_char_width_factor(ch: char, has_digit: bool) -> f32 {
    if has_digit && ch.is_ascii_digit() {
        char_width_factor('1')
    } else {
        char_width_factor(ch)
    }
}

fn default_svg_char_width_factor(ch: char) -> f32 {
    match ch {
        '0'..='9' => 0.4568,
        ' ' => 0.25,
        '.' | ',' | ':' | ';' | '|' | '!' | '(' | ')' | '[' | ']' | '{' | '}' => 0.285,
        '-' | '_' | '/' | '\\' => 0.32,
        '$' => 0.5,
        'A'..='Z' => 0.62,
        'a'..='z' => 0.47,
        _ => 0.5,
    }
}

fn linear_x_domain(len: usize) -> (f32, f32) {
    if len <= 1 {
        (1.0, 2.0)
    } else {
        (1.0, len as f32)
    }
}

fn linear_scale(
    value: f32,
    domain_start: f32,
    domain_end: f32,
    range_start: f32,
    range_end: f32,
) -> f32 {
    let domain_span = domain_end - domain_start;
    if domain_span.abs() < f32::EPSILON {
        return (range_start + range_end) / 2.0;
    }
    range_start + ((value - domain_start) / domain_span) * (range_end - range_start)
}

fn band_positions(range_start: f32, range_end: f32, count: usize) -> Vec<f32> {
    if count <= 1 {
        return vec![(range_start + range_end) / 2.0];
    }
    let step = (range_end - range_start) / (count - 1) as f32;
    (0..count)
        .map(|idx| range_start + idx as f32 * step)
        .collect()
}

fn linear_ticks(start: f32, stop: f32, count: usize) -> Vec<f32> {
    if !start.is_finite() || !stop.is_finite() {
        return Vec::new();
    }
    if (start - stop).abs() < f32::EPSILON {
        return vec![start];
    }
    let reverse = stop < start;
    let (lo, hi) = if reverse {
        (stop, start)
    } else {
        (start, stop)
    };
    let step = tick_step(lo, hi, count.max(1));
    if step <= 0.0 || !step.is_finite() {
        return vec![start, stop];
    }
    let precision = step_precision(step);
    let mut ticks = Vec::new();
    let mut value = (lo / step).ceil() * step;
    let end = (hi / step).floor() * step;
    while value <= end + step * 0.5 {
        ticks.push(round_to_precision(value, precision));
        value += step;
    }
    if reverse {
        ticks.reverse();
    }
    ticks
}

fn tick_step(start: f32, stop: f32, count: usize) -> f32 {
    let step0 = (stop - start).abs() / count as f32;
    if step0 <= 0.0 || !step0.is_finite() {
        return 0.0;
    }
    let step1 = 10.0_f32.powf(step0.log10().floor());
    let error = step0 / step1;
    let factor = if error >= 50.0_f32.sqrt() {
        10.0
    } else if error >= 10.0_f32.sqrt() {
        5.0
    } else if error >= 2.0_f32.sqrt() {
        2.0
    } else {
        1.0
    };
    step1 * factor
}

fn step_precision(step: f32) -> i32 {
    if step >= 1.0 {
        0
    } else {
        (-step.log10().floor() as i32).max(0) + 1
    }
}

fn round_to_precision(value: f32, precision: i32) -> f32 {
    if precision <= 0 {
        value.round()
    } else {
        let factor = 10.0_f32.powi(precision);
        (value * factor).round() / factor
    }
}

fn format_tick(value: f32) -> String {
    let normalized = if value.abs() < 0.000_001 { 0.0 } else { value };
    if (normalized - normalized.round()).abs() < 0.000_001 {
        return format!("{:.0}", normalized.round());
    }
    let mut text = format!("{:.6}", normalized);
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}
