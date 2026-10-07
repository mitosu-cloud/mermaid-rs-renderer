use super::*;
use crate::config::XYChartAxisConfig;

fn label_width(text: &str, size: f32, theme: &Theme) -> f32 {
    text_metrics::measure_text_width_with_kerning(text, size, &theme.font_family)
        .unwrap_or_else(|| text_metrics::get_computed_text_length(text, size, &theme.font_family))
}

fn label_height(size: f32, theme: &Theme) -> f32 {
    text_metrics::svg_text_height(size, &theme.font_family).unwrap_or(size * 1.15)
}

// D3's default linear-axis ticks use a target count of ten and 1/2/5 steps.
// Reciprocal increments avoid rounding artifacts for fractional tick labels.
fn linear_ticks(start: f64, stop: f64) -> Vec<f64> {
    if !start.is_finite() || !stop.is_finite() {
        return Vec::new();
    }
    if start == stop {
        return vec![start];
    }
    if stop < start {
        let mut ticks = linear_ticks(stop, start);
        ticks.reverse();
        return ticks;
    }
    let step = (stop - start) / 10.0;
    let power = step.log10().floor();
    let error = step / 10.0_f64.powf(power);
    let factor = if error >= 50.0_f64.sqrt() {
        10.0
    } else if error >= 10.0_f64.sqrt() {
        5.0
    } else if error >= 2.0_f64.sqrt() {
        2.0
    } else {
        1.0
    };
    let reciprocal = power < 0.0;
    let increment = if reciprocal {
        10.0_f64.powf(-power) / factor
    } else {
        10.0_f64.powf(power) * factor
    };
    let scaled_start = if reciprocal {
        start * increment
    } else {
        start / increment
    };
    let scaled_stop = if reciprocal {
        stop * increment
    } else {
        stop / increment
    };
    let first = scaled_start.ceil() as i64;
    let last = scaled_stop.floor() as i64;
    (first..=last)
        .map(|i| {
            if reciprocal {
                i as f64 / increment
            } else {
                i as f64 * increment
            }
        })
        .collect()
}

fn scale(value: f32, min: f32, max: f32, start: f32, end: f32) -> f32 {
    let fraction = if min == max {
        0.5
    } else {
        (value - min) / (max - min)
    };
    start + fraction * (end - start)
}

struct AxisSpace {
    size: f32,
    outer_padding: f32,
    label_width: f32,
    label_height: f32,
    rotation: f32,
    show_line: bool,
    show_tick: bool,
    show_label: bool,
    title: Option<String>,
    title_height: f32,
}

fn axis_space(
    options: &XYChartAxisConfig,
    labels: &[String],
    title: &Option<String>,
    horizontal: bool,
    available_width: f32,
    available_height: f32,
    theme: &Theme,
) -> AxisSpace {
    let mut available = if horizontal {
        available_height
    } else {
        available_width
    };
    let original = available;
    let show_line = options.show_axis_line && available > options.axis_line_width;
    if show_line {
        available -= options.axis_line_width;
    }
    let label_width = labels
        .iter()
        .map(|text| label_width(text, options.label_font_size, theme))
        .fold(0.0_f32, f32::max);
    let label_height = if labels.is_empty() {
        0.0
    } else {
        label_height(options.label_font_size, theme)
    };
    let rotation = if (-90.0..=90.0).contains(&options.label_rotation) {
        options.label_rotation.to_radians()
    } else {
        0.0
    };
    let outer_padding = if options.show_label {
        if horizontal {
            (label_width / 2.0).min(available_width * 0.2)
        } else {
            (label_height / 2.0).min(available_height * 0.2)
        }
    } else {
        0.0
    };
    let label_size = if horizontal {
        label_height.max(rotation.sin().abs() * label_width + rotation.cos().abs() * label_height)
    } else {
        label_width
    } + options.label_padding * 2.0;
    let show_label = options.show_label && label_size <= available;
    if show_label {
        available -= label_size;
    }
    let show_tick = options.show_tick && options.tick_length <= available;
    if show_tick {
        available -= options.tick_length;
    }
    let title_height = label_height_for_title(title, options.title_font_size, theme);
    let title_size = title_height + options.title_padding * 2.0;
    let title = title
        .as_ref()
        .filter(|text| !text.is_empty() && options.show_title && title_size <= available)
        .cloned();
    if title.is_some() {
        available -= title_size;
    }
    AxisSpace {
        size: original - available,
        outer_padding,
        label_width,
        label_height,
        rotation,
        show_line,
        show_tick,
        show_label,
        title,
        title_height,
    }
}

fn label_height_for_title(title: &Option<String>, size: f32, theme: &Theme) -> f32 {
    if title.as_ref().is_some_and(|text| !text.is_empty()) {
        label_height(size, theme)
    } else {
        0.0
    }
}

pub(super) fn compute_xychart_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> Layout {
    let data = &graph.xychart;
    let options = &config.xy_chart;
    let width = options.width.max(1.0);
    let height = options.height.max(1.0);
    let reserved = options.plot_reserved_space_percent.clamp(0.0, 100.0) / 100.0;
    let reserved_width = (width * reserved).floor();
    let reserved_height = (height * reserved).floor();
    let title = data
        .title
        .as_ref()
        .filter(|text| options.show_title && !text.is_empty())
        .cloned();
    let title_height = title
        .as_ref()
        .map(|_| label_height(options.title_font_size, theme) + options.title_padding * 2.0)
        .unwrap_or(0.0);
    let title_x = title
        .as_ref()
        .map(|text| width.max(label_width(text, options.title_font_size, theme)) / 2.0)
        .unwrap_or(width / 2.0);
    let available_width = width - reserved_width;
    let available_height = height - reserved_height - title_height;

    let values: Vec<f32> = data
        .series
        .iter()
        .flat_map(|series| series.values.iter().copied())
        .filter(|v| v.is_finite())
        .collect();
    let min = data
        .y_axis_min
        .unwrap_or_else(|| values.iter().copied().reduce(f32::min).unwrap_or(0.0));
    let max = data
        .y_axis_max
        .unwrap_or_else(|| values.iter().copied().reduce(f32::max).unwrap_or(0.0));
    let y_ticks = linear_ticks(max as f64, min as f64);
    let y_labels: Vec<String> = y_ticks.iter().map(f64::to_string).collect();
    let band_axis = !data.x_axis_categories.is_empty();
    let category_count = if band_axis {
        data.x_axis_categories.len()
    } else {
        data.series
            .iter()
            .map(|series| series.values.len())
            .max()
            .unwrap_or(1)
            .max(1)
    };
    let x_ticks = linear_ticks(1.0, category_count as f64);
    let x_labels = if band_axis {
        data.x_axis_categories.clone()
    } else {
        x_ticks.iter().map(f64::to_string).collect()
    };

    let mut x_space = axis_space(
        &options.x_axis,
        &x_labels,
        &data.x_axis_label,
        true,
        available_width,
        available_height,
        theme,
    );
    let y_space = axis_space(
        &options.y_axis,
        &y_labels,
        &data.y_axis_label,
        false,
        available_width,
        available_height - x_space.size,
        theme,
    );
    let plot_x = y_space.size;
    let plot_y = title_height;
    let plot_width = reserved_width.max(width - plot_x);
    let plot_height = reserved_height.max(height - plot_y - x_space.size);
    let plot_bottom = plot_y + plot_height;
    let has_bars = data
        .series
        .iter()
        .any(|series| series.kind == crate::ir::XYSeriesKind::Bar);
    let tick_count = x_labels.len().max(1) as f32;
    if has_bars {
        let tick_distance = (plot_width - x_space.outer_padding * 2.0).abs() / tick_count;
        if tick_distance * 0.7 > x_space.outer_padding * 2.0 {
            x_space.outer_padding = (tick_distance * 0.7 / 2.0).floor();
        }
    }
    let x_start = plot_x + x_space.outer_padding;
    let x_end = plot_x + plot_width - x_space.outer_padding;
    let value_y = |value| {
        scale(
            value,
            min,
            max,
            plot_bottom - y_space.outer_padding,
            plot_y + y_space.outer_padding,
        )
    };
    let value_x = |index: usize| {
        if category_count == 1 {
            (x_start + x_end) / 2.0
        } else {
            scale(
                index as f32,
                0.0,
                (category_count - 1) as f32,
                x_start,
                x_end,
            )
        }
    };
    let bar_width = (x_space.outer_padding * 2.0).min((x_end - x_start).abs() / tick_count) * 0.95;
    let default_palette =
        "#ECECFF,#8493A6,#FFC3A0,#DCDDE1,#B8E994,#D1A36F,#C3CDE6,#FFB6C1,#496078,#F8F3E3";
    let mut colors: Vec<String> = theme
        .xy_chart
        .plot_color_palette
        .as_deref()
        .unwrap_or(default_palette)
        .split(',')
        .map(str::trim)
        .filter(|color| !color.is_empty())
        .map(str::to_string)
        .collect();
    if colors.is_empty() {
        colors = default_palette.split(',').map(str::to_string).collect();
    }
    let mut bars = Vec::new();
    let mut lines = Vec::new();
    for (series_index, series) in data.series.iter().enumerate() {
        let color = colors[series_index % colors.len()].clone();
        match series.kind {
            crate::ir::XYSeriesKind::Bar => {
                let first = bars.len();
                for (i, &value) in series.values.iter().take(category_count).enumerate() {
                    let y = value_y(value);
                    bars.push(XYChartBarLayout {
                        series_index,
                        x: value_x(i) - bar_width / 2.0,
                        y,
                        width: bar_width,
                        height: plot_bottom - y,
                        value,
                        color: color.clone(),
                        label_font_size: None,
                    });
                }
                if options.show_data_label {
                    let font_size = bars[first..]
                        .iter()
                        .filter(|bar| bar.width > 0.0 && bar.height > 0.0)
                        .map(|bar| {
                            let chars = bar.value.to_string().len() as f32;
                            let mut size = bar.width / (chars * 0.7);
                            while size > 0.0
                                && (size * chars * 0.7 > bar.width || size + 10.0 > bar.height)
                            {
                                size -= 1.0;
                            }
                            size
                        })
                        .reduce(f32::min)
                        .map(|size| size.floor().max(0.0));
                    for bar in &mut bars[first..] {
                        if bar.width > 0.0 && bar.height > 0.0 {
                            bar.label_font_size = font_size;
                        }
                    }
                }
            }
            crate::ir::XYSeriesKind::Line => {
                let points = series
                    .values
                    .iter()
                    .take(category_count)
                    .enumerate()
                    .map(|(i, &value)| (value_x(i), value_y(value)))
                    .collect();
                lines.push(XYChartLineLayout {
                    series_index,
                    points,
                    color,
                });
            }
        }
    }
    let x_tick_positions = if band_axis {
        x_labels
            .into_iter()
            .enumerate()
            .map(|(i, label)| (label, value_x(i)))
            .collect()
    } else {
        x_ticks
            .iter()
            .map(|value| {
                (
                    value.to_string(),
                    scale(*value as f32, 1.0, category_count as f32, x_start, x_end),
                )
            })
            .collect()
    };
    let y_tick_positions = y_ticks
        .iter()
        .map(|value| (value.to_string(), value_y(*value as f32)))
        .collect();
    let x_line_size = if x_space.show_line {
        options.x_axis.axis_line_width
    } else {
        0.0
    };
    let x_tick_size = if x_space.show_tick {
        options.x_axis.tick_length
    } else {
        0.0
    };
    let y_line_size = if y_space.show_line {
        options.y_axis.axis_line_width
    } else {
        0.0
    };
    let y_tick_size = if y_space.show_tick {
        options.y_axis.tick_length
    } else {
        0.0
    };
    let x_axis = XYChartAxisLayout {
        show_line: x_space.show_line,
        show_tick: x_space.show_tick,
        show_label: x_space.show_label,
        ticks: x_tick_positions,
        label_position: plot_bottom
            + options.x_axis.label_padding
            + x_tick_size
            + x_line_size
            + (x_space.rotation.sin() * x_space.label_width / 2.0).abs(),
        label_rotation: x_space.rotation.to_degrees(),
        label_offset: x_space.rotation.sin() * x_space.label_height / 2.0,
        title: x_space.title,
        title_position: height - options.x_axis.title_padding - x_space.title_height,
    };
    let y_axis = XYChartAxisLayout {
        show_line: y_space.show_line,
        show_tick: y_space.show_tick,
        show_label: y_space.show_label,
        ticks: y_tick_positions,
        label_position: plot_x - options.y_axis.label_padding - y_tick_size - y_line_size,
        label_rotation: 0.0,
        label_offset: 0.0,
        title: y_space.title,
        title_position: options.y_axis.title_padding,
    };
    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        acc_title: None,
        acc_descr: None,
        diagram: DiagramData::XYChart(XYChartLayout {
            title,
            title_x,
            title_y: title_height / 2.0,
            x_axis,
            y_axis,
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
