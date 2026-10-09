use crate::{ir::NodeShape, layout::TextBlock};

pub(super) fn supported(shape: NodeShape) -> bool {
    matches!(
        shape,
        NodeShape::Document
            | NodeShape::LinedDocument
            | NodeShape::StackedDocument
            | NodeShape::TagDocument
            | NodeShape::Flag
            | NodeShape::WavyRect
    )
}

pub(super) struct Geometry {
    pub bounds: [f32; 4],
    pub label: (f32, f32),
    pub outline: Vec<(f32, f32)>,
    paths: Vec<(Vec<(f32, f32)>, bool)>,
}

impl Geometry {
    pub fn svg(&self, x: f32, y: f32, fill: &str, stroke: &str, sw: f32, dash: &str) -> String {
        let mut svg = String::new();
        for (points, closed) in &self.paths {
            let mut path = String::new();
            for (index, &(px, py)) in points.iter().enumerate() {
                path.push_str(&format!(
                    "{} {:.5},{:.5} ",
                    if index == 0 { "M" } else { "L" },
                    x - self.bounds[0] + px,
                    y - self.bounds[1] + py
                ));
            }
            if *closed {
                path.push('Z');
            }
            let color = if *closed { fill } else { "none" };
            svg.push_str(&format!("<path d=\"{path}\" fill=\"{color}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"{dash}/>"));
        }
        svg
    }
}

// Mermaid samples 50 sine segments; the vertical component is centered between
// the endpoints, including the half-wave on a tagged document's folded corner.
fn wave(from: (f32, f32), to: (f32, f32), amplitude: f32, cycles: f32) -> Vec<(f32, f32)> {
    (0..=50)
        .map(|step| {
            let t = step as f32 / 50.0;
            (
                from.0 + (to.0 - from.0) * t,
                (from.1 + to.1) / 2.0 + amplitude * (std::f32::consts::TAU * cycles * t).sin(),
            )
        })
        .collect()
}

/// Port of Mermaid's waveEdgedRectangle, linedWaveEdgedRect,
/// multiWaveEdgedRectangle, taggedWaveEdgedRectangle, and waveRectangle.
pub(super) fn geometry(shape: NodeShape, label: &TextBlock, neo: bool) -> Option<Geometry> {
    if !supported(shape) {
        return None;
    }
    let px = if neo { 16.0 } else { 15.0 };
    let py = if neo { 12.0 } else { 15.0 };
    let (mut w, mut h) = (label.width + 2.0 * px, label.height + 2.0 * py);
    let mut paths = Vec::new();
    let mut label_center = (0.0, 0.0);
    let mut shift = 0.0;
    let outline = match shape {
        NodeShape::Flag | NodeShape::WavyRect => {
            h = label.height + if neo { 20.0 } else { 15.0 };
            let amplitude = h / 8.0;
            let half = (h + 2.0 * amplitude) / 2.0;
            let mut points = wave((-w / 2.0, half), (w / 2.0, half), amplitude, 1.0);
            points.extend(wave((w / 2.0, -half), (-w / 2.0, -half), amplitude, -1.0));
            points
        }
        NodeShape::StackedDocument => {
            h = label.height + 3.0 * py;
            let amplitude = h / if neo { 4.0 } else { 8.0 };
            let half = (h + amplitude / 2.0) / 2.0;
            let (x, y, offset) = (-w / 2.0, -half, 10.0);
            let wave = wave(
                (x - offset, half + offset),
                (x + w - offset, half + offset),
                amplitude,
                0.8,
            );
            let last_y = wave.last()?.1;
            let mut points = vec![(x - offset, y + offset), (x - offset, half + offset)];
            points.extend(wave);
            points.extend([
                (x + w - offset, last_y - offset),
                (x + w, last_y - offset),
                (x + w, last_y - 2.0 * offset),
                (x + w + offset, last_y - 2.0 * offset),
                (x + w + offset, y - offset),
                (x + offset, y - offset),
                (x + offset, y),
                (x, y),
                (x, y + offset),
            ]);
            let inner = vec![
                (x, y + offset),
                (x + w - offset, y + offset),
                (x + w - offset, last_y - offset),
                (x + w, last_y - offset),
                (x + w, y),
                (x, y),
            ];
            paths.push((inner, true));
            shift = -amplitude / 2.0;
            label_center = (-offset, offset - amplitude);
            points
        }
        NodeShape::TagDocument => {
            w = label.width + 30.0;
            h = label.height + 30.0;
            let amplitude = h / 8.0;
            let half = (h + amplitude) / 2.0;
            let mut points = wave((-w * 0.55, half), (w * 0.55, half), amplitude, 0.8);
            points.extend([(w * 0.55, -half), (-w * 0.55, -half)]);
            let (tag_w, tag_h) = (0.2 * w, 0.2 * h);
            let (x, y) = (-w * 0.45, -half - tag_h * 0.4);
            let mut tag = vec![
                (x + w - tag_w, (y + h) * 1.3),
                (x + w, y + h - tag_h),
                (x + w, (y + h) * 0.9),
            ];
            tag.extend(wave(
                (x + w, (y + h) * 1.25),
                (x + w - tag_w, (y + h) * 1.3),
                -h * 0.02,
                0.5,
            ));
            paths.push((tag, true));
            shift = -amplitude / 2.0;
            label_center = (0.0, -amplitude / 2.0);
            points
        }
        NodeShape::Document | NodeShape::LinedDocument => {
            let amplitude = h / if neo { 4.0 } else { 8.0 };
            let half = (h + amplitude) / 2.0;
            let lined = shape == NodeShape::LinedDocument;
            let half_w = if lined { w * 0.55 } else { (w / 2.0).max(7.0) };
            let mut points = vec![(-half_w, -half), (-half_w, half)];
            points.extend(wave((-half_w, half), (half_w, half), amplitude, 0.8));
            points.extend([(half_w, -half), (-half_w, -half)]);
            if lined {
                paths.push((vec![(-w / 2.0, -half), (-w / 2.0, half * 1.1)], false));
            }
            shift = -amplitude / 2.0;
            // labelHelper uses the flowchart's 15px padding here, even in neo.
            label_center = (
                -w / 2.0 + 15.0 + if lined { w / 40.0 } else { 0.0 } + label.width / 2.0,
                -h / 2.0 + 15.0 - amplitude + label.height / 2.0,
            );
            points
        }
        _ => return None,
    };
    paths.insert(0, (outline.clone(), true));
    for (points, _) in &mut paths {
        for point in points {
            point.1 += shift;
        }
    }
    let bounds = paths.iter().flat_map(|(points, _)| points).fold(
        [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ],
        |mut bounds, &(x, y)| {
            bounds[0] = bounds[0].min(x);
            bounds[1] = bounds[1].min(y);
            bounds[2] = bounds[2].max(x);
            bounds[3] = bounds[3].max(y);
            bounds
        },
    );
    Some(Geometry {
        bounds,
        label: label_center,
        outline: outline.into_iter().map(|(x, y)| (x, y + shift)).collect(),
        paths,
    })
}
