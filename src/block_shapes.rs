//! Mermaid block shapes have their own padding and two-stage grid sizing.
//! Keep outline coordinates shared between rendering and edge intersections.
use crate::ir::{BlockArrowDirection, NodeShape};
use crate::layout::{NodeLayout, TextBlock};

pub(crate) const PADDING: f32 = 8.0;

pub(crate) fn cylinder_radius(width: f32) -> f32 {
    (width / 2.0) / (2.5 + width / 50.0)
}

/// Shape bounds during Mermaid's initial, unpositioned measurement pass.
pub(crate) fn natural_size(shape: NodeShape, label: &TextBlock) -> Option<(f32, f32)> {
    let w = label.width;
    let h = label.height;
    let p = PADDING;
    Some(match shape {
        NodeShape::BlockArrow(_) => (w + h + 3.0 * p, h + 2.0 * p),
        NodeShape::Circle => (w + p, w + p),
        NodeShape::DoubleCircle => (w + 2.0 * p, w + 2.0 * p),
        NodeShape::Cylinder => (w + p, h + p + 3.0 * cylinder_radius(w + p)),
        NodeShape::Hexagon => (w + p + (h + p) / 2.0, h + p),
        NodeShape::Diamond => (w + h + 2.0 * p, w + h + 2.0 * p),
        NodeShape::RoundRect => (w + 2.0 * p, h + 2.0 * p),
        NodeShape::Stadium => (w + (h + p) / 4.0 + p, h + p),
        NodeShape::Subroutine => (w + p + 16.0, h + p),
        NodeShape::Parallelogram | NodeShape::ParallelogramAlt | NodeShape::Trapezoid => {
            (w + p + h + p, h + p)
        }
        NodeShape::TrapezoidAlt => (w + h + 4.0 * p, h + 2.0 * p),
        _ => return None,
    })
}

/// Some shapes retain their natural size; others expand when drawn in a grid cell.
pub(crate) fn positioned_size(
    node: &NodeLayout,
    width: f32,
    height: f32,
    span: usize,
) -> (f32, f32) {
    let natural = natural_size(node.shape, &node.label).unwrap_or((width, height));
    match node.shape {
        NodeShape::BlockArrow(_) => (
            if span > 1 {
                width.max(natural.0)
            } else {
                natural.0
            },
            natural.1,
        ),
        NodeShape::Circle | NodeShape::Diamond | NodeShape::Stadium => natural,
        // doubleCircle.ts adds its label padding again to the positioned width.
        NodeShape::DoubleCircle => (width + 2.0 * PADDING, width + 2.0 * PADDING),
        NodeShape::Cylinder => {
            let w = width.max(natural.0);
            (
                w,
                height.max(node.label.height + PADDING + 3.0 * cylinder_radius(w)),
            )
        }
        NodeShape::Parallelogram
        | NodeShape::ParallelogramAlt
        | NodeShape::Trapezoid
        | NodeShape::TrapezoidAlt => {
            let h = height.max(natural.1);
            let padding = if node.shape == NodeShape::TrapezoidAlt {
                2.0 * PADDING
            } else {
                PADDING
            };
            ((node.label.width + padding).max(width - h) + h, h)
        }
        _ => (width, height),
    }
}

pub(crate) fn polygon_points(node: &NodeLayout) -> Option<Vec<(f32, f32)>> {
    let w = node.width;
    let h = node.height;
    let points = match node.shape {
        NodeShape::BlockArrow(direction) => arrow_points(direction, w, h),
        NodeShape::Hexagon => {
            let m = h / 4.0;
            vec![
                (m, 0.0),
                (w - m, 0.0),
                (w, h / 2.0),
                (w - m, h),
                (m, h),
                (0.0, h / 2.0),
            ]
        }
        NodeShape::Diamond => vec![
            (w / 2.0 + 0.5, 0.0),
            (w + 0.5, h / 2.0),
            (w / 2.0 + 0.5, h),
            (0.5, h / 2.0),
        ],
        NodeShape::Parallelogram => vec![(h / 2.0, 0.0), (w, 0.0), (w - h / 2.0, h), (0.0, h)],
        NodeShape::ParallelogramAlt => vec![(0.0, 0.0), (w - h / 2.0, 0.0), (w, h), (h / 2.0, h)],
        NodeShape::Trapezoid => vec![(h / 2.0, 0.0), (w - h / 2.0, 0.0), (w, h), (0.0, h)],
        NodeShape::TrapezoidAlt => vec![(0.0, 0.0), (w, 0.0), (w - h / 2.0, h), (h / 2.0, h)],
        _ => return None,
    };
    Some(
        points
            .into_iter()
            .map(|(x, y)| (node.x + x, node.y + y))
            .collect(),
    )
}

/// Actual polygon bounds can differ from the logical cell (notably one-sided arrows).
pub(crate) fn visible_bounds(node: &NodeLayout) -> (f32, f32, f32, f32) {
    if let Some(points) = polygon_points(node) {
        let mut bounds = (
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        );
        for (x, y) in points {
            bounds.0 = bounds.0.min(x);
            bounds.1 = bounds.1.min(y);
            bounds.2 = bounds.2.max(x);
            bounds.3 = bounds.3.max(y);
        }
        let cx = node.x + node.width / 2.0;
        let cy = node.y + node.height / 2.0;
        bounds.0 = bounds.0.min(cx - node.label.width / 2.0);
        bounds.1 = bounds.1.min(cy - node.label.height / 2.0);
        bounds.2 = bounds.2.max(cx + node.label.width / 2.0);
        bounds.3 = bounds.3.max(cy + node.label.height / 2.0);
        bounds
    } else {
        (node.x, node.y, node.x + node.width, node.y + node.height)
    }
}

pub(crate) fn arrow_points(shape: BlockArrowDirection, width: f32, height: f32) -> Vec<(f32, f32)> {
    let midpoint = height / 2.0;
    let padding = 4.0;
    let raw = match shape {
        BlockArrowDirection::All => vec![
            (0.0, 0.0),
            (midpoint, 0.0),
            (width / 2.0, 2.0 * padding),
            (width - midpoint, 0.0),
            (width, 0.0),
            (width, -height / 3.0),
            (width + 2.0 * padding, -height / 2.0),
            (width, -2.0 * height / 3.0),
            (width, -height),
            (width - midpoint, -height),
            (width / 2.0, -height - 2.0 * padding),
            (midpoint, -height),
            (0.0, -height),
            (0.0, -2.0 * height / 3.0),
            (-2.0 * padding, -height / 2.0),
            (0.0, -height / 3.0),
        ],
        BlockArrowDirection::XUp => vec![
            (midpoint, 0.0),
            (width - midpoint, 0.0),
            (width, -height / 2.0),
            (width - midpoint, -height),
            (midpoint, -height),
            (0.0, -height / 2.0),
        ],
        BlockArrowDirection::XDown => vec![
            (0.0, 0.0),
            (midpoint, -height),
            (width - midpoint, -height),
            (width, 0.0),
        ],
        BlockArrowDirection::YRight => vec![
            (0.0, 0.0),
            (width, -midpoint),
            (width, -height + midpoint),
            (0.0, -height),
        ],
        BlockArrowDirection::YLeft => vec![
            (width, 0.0),
            (0.0, -midpoint),
            (0.0, -height + midpoint),
            (width, -height),
        ],
        BlockArrowDirection::X => vec![
            (midpoint, 0.0),
            (midpoint, -padding),
            (width - midpoint, -padding),
            (width - midpoint, 0.0),
            (width, -height / 2.0),
            (width - midpoint, -height),
            (width - midpoint, -height + padding),
            (midpoint, -height + padding),
            (midpoint, -height),
            (0.0, -height / 2.0),
        ],
        BlockArrowDirection::Y => vec![
            (width / 2.0, 0.0),
            (0.0, -padding),
            (midpoint, -padding),
            (midpoint, -height + padding),
            (0.0, -height + padding),
            (width / 2.0, -height),
            (width, -height + padding),
            (width - midpoint, -height + padding),
            (width - midpoint, -padding),
            (width, -padding),
        ],
        BlockArrowDirection::RightUp => {
            vec![(0.0, 0.0), (width, -midpoint), (0.0, -height)]
        }
        BlockArrowDirection::RightDown => {
            vec![(0.0, 0.0), (width, 0.0), (0.0, -height)]
        }
        BlockArrowDirection::LeftUp => {
            vec![(width, 0.0), (0.0, -midpoint), (width, -height)]
        }
        BlockArrowDirection::LeftDown => {
            vec![(width, 0.0), (0.0, 0.0), (width, -height)]
        }
        BlockArrowDirection::Right => vec![
            (midpoint, -padding),
            (midpoint, -padding),
            (width - midpoint, -padding),
            (width - midpoint, 0.0),
            (width, -height / 2.0),
            (width - midpoint, -height),
            (width - midpoint, -height + padding),
            (midpoint, -height + padding),
            (midpoint, -height + padding),
        ],
        BlockArrowDirection::Left => vec![
            (midpoint, 0.0),
            (midpoint, -padding),
            (width - midpoint, -padding),
            (width - midpoint, -height + padding),
            (midpoint, -height + padding),
            (midpoint, -height),
            (0.0, -height / 2.0),
        ],
        BlockArrowDirection::Up => vec![
            (midpoint, -padding),
            (midpoint, -height + padding),
            (0.0, -height + padding),
            (width / 2.0, -height),
            (width, -height + padding),
            (width - midpoint, -height + padding),
            (width - midpoint, -padding),
        ],
        BlockArrowDirection::Down => vec![
            (width / 2.0, 0.0),
            (0.0, -padding),
            (midpoint, -padding),
            (midpoint, -height + padding),
            (width - midpoint, -height + padding),
            (width - midpoint, -padding),
            (width, -padding),
        ],
    };

    raw.into_iter().map(|(px, py)| (px, py + height)).collect()
}
