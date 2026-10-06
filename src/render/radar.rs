use std::f64::consts::{FRAC_PI_2, TAU};
use std::fmt::Write;

use crate::ir::RadarData;
use crate::theme::Theme;

use super::{escape_xml, normalize_font_family};

const RADIUS: f64 = 300.0;
const TENSION: f64 = 0.17;
const HUES: [u16; 12] = [240, 60, 80, 270, 300, 330, 0, 30, 90, 150, 180, 210];

fn curve_color(index: usize, theme: &Theme) -> String {
    if let Some(color) = theme.cscale_colors.get(index) {
        return color.clone();
    }
    let index = index % HUES.len();
    let lightness = if index == 1 {
        "73.5294117647"
    } else {
        "76.2745098039"
    };
    format!("hsl({}, 100%, {}%)", HUES[index], lightness)
}

fn point(radius: f64, index: usize, count: usize) -> (f64, f64) {
    let angle = TAU * index as f64 / count as f64 - FRAC_PI_2;
    (radius * angle.cos(), radius * angle.sin())
}

fn polygon_points(points: &[(f64, f64)]) -> String {
    points
        .iter()
        .map(|(x, y)| format!("{x:.3},{y:.3}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn closed_round_curve(points: &[(f64, f64)]) -> String {
    let count = points.len();
    let mut path = format!("M{:.3},{:.3}", points[0].0, points[0].1);
    // Match Mermaid's closed Catmull-Rom spline, including the wraparound
    // tangent at the first/last data point.
    for i in 0..count {
        let p0 = points[(i + count - 1) % count];
        let p1 = points[i];
        let p2 = points[(i + 1) % count];
        let p3 = points[(i + 2) % count];
        let c1 = (
            p1.0 + (p2.0 - p0.0) * TENSION,
            p1.1 + (p2.1 - p0.1) * TENSION,
        );
        let c2 = (
            p2.0 - (p3.0 - p1.0) * TENSION,
            p2.1 - (p3.1 - p1.1) * TENSION,
        );
        let _ = write!(
            path,
            " C{:.3},{:.3} {:.3},{:.3} {:.3},{:.3}",
            c1.0, c1.1, c2.0, c2.1, p2.0, p2.1
        );
    }
    path.push_str(" Z");
    path
}

pub(super) fn render_radar(radar: &RadarData, theme: &Theme) -> String {
    let mut svg = String::from("<g transform=\"translate(350, 350)\">");
    let count = radar.axes.len();
    let font = escape_xml(&normalize_font_family(&theme.font_family));
    let text_color = escape_xml(&theme.text_color);
    let max = radar.max.unwrap_or_else(|| {
        radar
            .curves
            .iter()
            .flat_map(|curve| curve.values.iter().copied())
            .fold(f64::NEG_INFINITY, f64::max)
    });
    let range = max - radar.min;

    for tick in 1..=radar.ticks {
        let radius = RADIUS * tick as f64 / radar.ticks as f64;
        let geometry = if radar.polygon {
            let points: Vec<_> = (0..count).map(|i| point(radius, i, count)).collect();
            format!("polygon points=\"{}\"", polygon_points(&points))
        } else {
            format!("circle r=\"{radius:.3}\"")
        };
        let _ = write!(
            svg,
            "<{geometry} fill=\"#DEDEDE\" fill-opacity=\"0.3\" stroke=\"#DEDEDE\" stroke-width=\"1\"/>"
        );
    }

    for (index, axis) in radar.axes.iter().enumerate() {
        let (cos, sin) = point(1.0, index, count);
        let (x, y) = (RADIUS * cos, RADIUS * sin);
        let _ = write!(
            svg,
            "<line x1=\"0\" y1=\"0\" x2=\"{x:.3}\" y2=\"{y:.3}\" stroke=\"#333333\" stroke-width=\"2\"/>"
        );
        let anchor = if cos > 0.01 {
            "start"
        } else if cos < -0.01 {
            "end"
        } else {
            "middle"
        };
        let baseline = if sin > 0.01 {
            "hanging"
        } else if sin < -0.01 {
            "auto"
        } else {
            "central"
        };
        let (x, y) = ((RADIUS * 1.05 + 4.0) * cos, (RADIUS * 1.05 + 4.0) * sin);
        let label = escape_xml(&axis.label);
        let _ = write!(
            svg,
            "<text x=\"{x:.3}\" y=\"{y:.3}\" text-anchor=\"{anchor}\" dominant-baseline=\"{baseline}\" font-family=\"{font}\" font-size=\"12\" fill=\"{text_color}\">{label}</text>"
        );
    }

    for (index, curve) in radar.curves.iter().enumerate() {
        if count == 0 || curve.values.len() != count {
            continue;
        }
        let points: Vec<_> = curve
            .values
            .iter()
            .enumerate()
            .map(|(i, value)| {
                let radius = if range > 0.0 && range.is_finite() {
                    RADIUS * (value.max(radar.min).min(max) - radar.min) / range
                } else {
                    0.0
                };
                point(radius, i, count)
            })
            .collect();
        let color = escape_xml(&curve_color(index, theme));
        let geometry = if radar.polygon {
            format!("polygon points=\"{}\"", polygon_points(&points))
        } else {
            format!("path d=\"{}\"", closed_round_curve(&points))
        };
        let _ = write!(
            svg,
            "<{geometry} fill=\"{color}\" fill-opacity=\"0.5\" stroke=\"{color}\" stroke-width=\"2\"/>"
        );
    }

    if radar.show_legend {
        for (index, curve) in radar.curves.iter().enumerate() {
            let color = escape_xml(&curve_color(index, theme));
            let y = -262.5 + index as f64 * 20.0;
            let label = escape_xml(&curve.label);
            let _ = write!(
                svg,
                "<g transform=\"translate(262.5, {y:.3})\"><rect width=\"12\" height=\"12\" fill=\"{color}\" fill-opacity=\"0.5\" stroke=\"{color}\"/><text x=\"16\" y=\"0\" text-anchor=\"start\" dominant-baseline=\"hanging\" font-family=\"{font}\" font-size=\"12\" fill=\"{text_color}\">{label}</text></g>"
            );
        }
    }
    let title = escape_xml(radar.title.as_deref().unwrap_or(""));
    let size = theme.font_size;
    let _ = write!(
        svg,
        "<text x=\"0\" y=\"-350\" text-anchor=\"middle\" dominant-baseline=\"hanging\" font-family=\"{font}\" font-size=\"{size}\" fill=\"{text_color}\">{title}</text></g>"
    );
    svg
}
