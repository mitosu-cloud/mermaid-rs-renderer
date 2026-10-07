//! Decorative mindmap outlines from Mermaid's cloud and bang shape builders.
//! Bounds are measured from the arcs, which extend beyond the padded label box.

use crate::{ir::NodeShape, layout::NodeLayout};

pub(crate) struct Outline {
    pub path: String,
    pub bounds: (f32, f32, f32, f32),
}

struct Arc {
    rx: f32,
    ry: f32,
    dx: f32,
    dy: f32,
    rotation: f32,
    large: bool,
    sweep: bool,
}

fn arc(rx: f32, ry: f32, dx: f32, dy: f32, large: bool, sweep: bool) -> Arc {
    Arc {
        rx,
        ry,
        dx,
        dy,
        rotation: 1.0,
        large,
        sweep,
    }
}

impl Outline {
    pub fn width(&self) -> f32 {
        self.bounds.2 - self.bounds.0
    }

    pub fn height(&self) -> f32 {
        self.bounds.3 - self.bounds.1
    }
}

pub(crate) fn outline(
    shape: NodeShape,
    label_width: f32,
    label_height: f32,
    padding: f32,
) -> Option<Outline> {
    let padding = padding.max(0.0);
    let (w, h, mut arcs) = match shape {
        NodeShape::MindmapCloud => {
            let w = (label_width + padding).max(1.0);
            let h = (label_height + padding).max(1.0);
            let (r1, r2, r3, r4) = (0.15 * w, 0.25 * w, 0.35 * w, 0.2 * w);
            (
                w,
                h,
                vec![
                    arc(r1, r1, w * 0.25, -w * 0.1, false, true),
                    arc(r3, r3, w * 0.4, -w * 0.1, false, true),
                    arc(r2, r2, w * 0.35, w * 0.2, false, true),
                    arc(r1, r1, w * 0.15, h * 0.35, false, true),
                    arc(r4, r4, -w * 0.15, h * 0.65, false, true),
                    arc(r2, r1, -w * 0.25, w * 0.15, false, true),
                    arc(r3, r3, -w * 0.5, 0.0, false, true),
                    arc(r1, r1, -w * 0.25, -w * 0.15, false, true),
                    arc(r1, r1, -w * 0.1, -h * 0.35, false, true),
                    arc(r4, r4, w * 0.1, -h * 0.65, false, true),
                ],
            )
        }
        NodeShape::MindmapBang => {
            let w = (label_width + 5.0 * padding).max(label_width + 20.0);
            let h = (label_height + 4.0 * padding).max(label_height + 20.0);
            let r = 0.15 * (label_width + 5.0 * padding).max(1.0);
            (
                w,
                h,
                vec![
                    arc(r, r, w * 0.25, -h * 0.1, false, false),
                    arc(r, r, w * 0.25, 0.0, false, false),
                    arc(r, r, w * 0.25, 0.0, false, false),
                    arc(r, r, w * 0.25, h * 0.1, false, false),
                    arc(r, r, w * 0.15, h * 0.33, false, false),
                    arc(r * 0.8, r * 0.8, 0.0, h * 0.34, false, false),
                    arc(r, r, -w * 0.15, h * 0.33, false, false),
                    arc(r, r, -w * 0.25, h * 0.15, false, false),
                    arc(r, r, -w * 0.25, 0.0, false, false),
                    arc(r, r, -w * 0.25, 0.0, false, false),
                    arc(r, r, -w * 0.25, -h * 0.15, false, false),
                    arc(r, r, -w * 0.1, -h * 0.33, false, false),
                    arc(r * 0.8, r * 0.8, 0.0, -h * 0.34, false, false),
                    arc(r, r, w * 0.1, -h * 0.33, false, false),
                ],
            )
        }
        _ => return None,
    };
    // The first number after the radii is rotation, not the large-arc flag.
    if shape == NodeShape::MindmapCloud {
        arcs[0].rotation = 0.0;
    }
    let mut x = -w / 2.0;
    let mut y = -h / 2.0;
    let mut bounds = (x, y, x, y);
    let mut path = format!("M{x:.6} {y:.6}");
    for arc in arcs {
        arc_bounds((x, y), &arc, &mut bounds);
        path.push_str(&format!(
            " a{:.6},{:.6} {} {},{} {:.6},{:.6}",
            arc.rx,
            arc.ry,
            arc.rotation,
            u8::from(arc.large),
            u8::from(arc.sweep),
            arc.dx,
            arc.dy
        ));
        x += arc.dx;
        y += arc.dy;
    }
    path.push_str(&format!(" H{:.6} V{:.6} Z", -w / 2.0, -h / 2.0));
    Some(Outline { path, bounds })
}

fn include(bounds: &mut (f32, f32, f32, f32), x: f32, y: f32) {
    bounds.0 = bounds.0.min(x);
    bounds.1 = bounds.1.min(y);
    bounds.2 = bounds.2.max(x);
    bounds.3 = bounds.3.max(y);
}

fn arc_bounds(start: (f32, f32), arc: &Arc, bounds: &mut (f32, f32, f32, f32)) {
    use std::f32::consts::{PI, TAU};
    let end = (start.0 + arc.dx, start.1 + arc.dy);
    include(bounds, end.0, end.1);
    let (mut rx, mut ry) = (arc.rx.abs(), arc.ry.abs());
    if rx <= 0.0 || ry <= 0.0 || (arc.dx == 0.0 && arc.dy == 0.0) {
        return;
    }
    let (sin, cos) = arc.rotation.to_radians().sin_cos();
    let (xp, yp) = (
        (-arc.dx * cos - arc.dy * sin) / 2.0,
        (arc.dx * sin - arc.dy * cos) / 2.0,
    );
    let scale = (xp * xp / (rx * rx) + yp * yp / (ry * ry)).sqrt().max(1.0);
    rx *= scale;
    ry *= scale;
    let denominator = rx * rx * yp * yp + ry * ry * xp * xp;
    let sign = if arc.large == arc.sweep { -1.0 } else { 1.0 };
    let factor = sign
        * ((rx * rx * ry * ry - denominator) / denominator)
            .max(0.0)
            .sqrt();
    let (cxp, cyp) = (factor * rx * yp / ry, -factor * ry * xp / rx);
    let cx = (start.0 + end.0) / 2.0 + cxp * cos - cyp * sin;
    let cy = (start.1 + end.1) / 2.0 + cxp * sin + cyp * cos;
    let first = ((yp - cyp) / ry).atan2((xp - cxp) / rx);
    let last = ((-yp - cyp) / ry).atan2((-xp - cxp) / rx);
    let extent = if arc.sweep {
        last - first
    } else {
        first - last
    }
    .rem_euclid(TAU);
    let x_extreme = (-ry * sin).atan2(rx * cos);
    let y_extreme = (ry * cos).atan2(rx * sin);
    for angle in [x_extreme, x_extreme + PI, y_extreme, y_extreme + PI] {
        let distance = if arc.sweep {
            angle - first
        } else {
            first - angle
        }
        .rem_euclid(TAU);
        if distance <= extent + 1e-5 {
            include(
                bounds,
                cx + rx * angle.cos() * cos - ry * angle.sin() * sin,
                cy + rx * angle.cos() * sin + ry * angle.sin() * cos,
            );
        }
    }
}

pub(crate) fn node_bounds(node: &NodeLayout, padding: f32) -> (f32, f32, f32, f32) {
    if let Some(outline) = outline(node.shape, node.label.width, node.label.height, padding) {
        let (cx, cy) = (node.x + node.width / 2.0, node.y + node.height / 2.0);
        let (left, top, right, bottom) = outline.bounds;
        (cx + left, cy + top, cx + right, cy + bottom)
    } else {
        (node.x, node.y, node.x + node.width, node.y + node.height)
    }
}
