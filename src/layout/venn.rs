use std::collections::{BTreeMap, HashMap};

use crate::config::LayoutConfig;
use crate::ir::Graph;
use crate::theme::Theme;

use super::{DiagramData, Layout, VennCircleLayout, VennIntersectionLayout, VennLayout};

const VENN_PALETTE: [&str; 8] = [
    "#4e79a7", "#f28e2c", "#e15759", "#76b7b2", "#59a14f", "#edc949", "#af7aa1", "#ff9da7",
];

pub(super) fn compute_venn_layout(graph: &Graph, _theme: &Theme, _config: &LayoutConfig) -> Layout {
    let num_sets = graph.venn.sets.len();

    if num_sets == 0 {
        return Layout {
            kind: graph.kind,
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
            width: 400.0,
            height: 300.0,
            diagram: DiagramData::Venn(VennLayout {
                width: 400.0,
                height: 300.0,
                title: graph.venn.title.clone(),
                circles: Vec::new(),
                intersections: Vec::new(),
            }),
            acc_title: None,
            acc_descr: None,
        };
    }

    // Title height
    let title_h = if graph.venn.title.is_some() {
        40.0
    } else {
        0.0
    };

    // Determine radii from sizes (proportional to sqrt of size)
    let max_size = graph
        .venn
        .sets
        .iter()
        .map(|s| s.size)
        .fold(0.0f32, f32::max)
        .max(1.0);

    let base_radius = match num_sets {
        1 => 120.0,
        2 => 110.0,
        3 => 100.0,
        _ => 90.0,
    };

    let radii: Vec<f32> = graph
        .venn
        .sets
        .iter()
        .map(|s| base_radius * (s.size / max_size).sqrt().max(0.3))
        .collect();

    // Position circles based on count
    let positions: Vec<(f32, f32)> = match num_sets {
        1 => vec![(0.0, 0.0)],
        2 => {
            // Two overlapping circles side by side
            let overlap = radii[0].min(radii[1]) * 0.5;
            let dx = radii[0] + radii[1] - overlap;
            vec![(-dx / 2.0, 0.0), (dx / 2.0, 0.0)]
        }
        3 => {
            // Three circles in a triangle arrangement
            let r_avg = (radii[0] + radii[1] + radii[2]) / 3.0;
            let spread = r_avg * 0.85;
            vec![
                (0.0, -spread * 0.6),            // top
                (-spread * 0.866, spread * 0.4), // bottom-left
                (spread * 0.866, spread * 0.4),  // bottom-right
            ]
        }
        _ => {
            // Arrange in a circle
            let r_avg: f32 = radii.iter().sum::<f32>() / num_sets as f32;
            let ring_radius = r_avg * 1.2;
            (0..num_sets)
                .map(|i| {
                    let angle = -std::f32::consts::FRAC_PI_2
                        + 2.0 * std::f32::consts::PI * i as f32 / num_sets as f32;
                    (ring_radius * angle.cos(), ring_radius * angle.sin())
                })
                .collect()
        }
    };

    // Compute bounding box
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for (i, &(px, py)) in positions.iter().enumerate() {
        let r = radii[i];
        min_x = min_x.min(px - r);
        max_x = max_x.max(px + r);
        min_y = min_y.min(py - r);
        max_y = max_y.max(py + r);
    }

    let pad = 40.0;
    let content_w = max_x - min_x + pad * 2.0;
    let content_h = max_y - min_y + pad * 2.0 + title_h;
    let width = content_w.max(300.0);
    let height = content_h.max(250.0);

    let center_x = width / 2.0 - (min_x + max_x) / 2.0;
    let center_y = title_h + (height - title_h) / 2.0 - (min_y + max_y) / 2.0;

    // Build circles
    let mut circles = Vec::with_capacity(num_sets);
    let mut set_centers: HashMap<String, (f32, f32)> = HashMap::new();

    for (i, set) in graph.venn.sets.iter().enumerate() {
        let cx = positions[i].0 + center_x;
        let cy = positions[i].1 + center_y;
        set_centers.insert(set.id.clone(), (cx, cy));

        let default_color = VENN_PALETTE[i % VENN_PALETTE.len()].to_string();
        let (color, fill_opacity, stroke, stroke_width, text_color) =
            if let Some(ref style) = set.style {
                (
                    style.fill.clone().unwrap_or(default_color),
                    style.fill_opacity.unwrap_or(0.5),
                    style.stroke.clone().unwrap_or_else(|| "#333".to_string()),
                    style.stroke_width.unwrap_or(2.0),
                    style.color.clone().unwrap_or_else(|| "#333".to_string()),
                )
            } else {
                (
                    default_color,
                    0.5,
                    "#333".to_string(),
                    2.0,
                    "#333".to_string(),
                )
            };

        circles.push(VennCircleLayout {
            id: set.id.clone(),
            label: set.label.clone(),
            cx,
            cy,
            radius: radii[i],
            color,
            fill_opacity,
            stroke,
            stroke_width,
            text_color,
        });
    }

    // Build intersections
    let mut intersections = Vec::new();
    for union in &graph.venn.unions {
        // Compute intersection center as average of referenced set centers
        let mut sum_x = 0.0f32;
        let mut sum_y = 0.0f32;
        let mut count = 0;
        for sid in &union.set_ids {
            if let Some(&(cx, cy)) = set_centers.get(sid) {
                sum_x += cx;
                sum_y += cy;
                count += 1;
            }
        }
        if count > 0 {
            let text_color = union
                .style
                .as_ref()
                .and_then(|s| s.color.clone())
                .unwrap_or_else(|| "#333".to_string());
            intersections.push(VennIntersectionLayout {
                set_ids: union.set_ids.clone(),
                label: union.label.clone(),
                cx: sum_x / count as f32,
                cy: sum_y / count as f32,
                text_color,
            });
        }
    }

    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width,
        height,
        diagram: DiagramData::Venn(VennLayout {
            width,
            height,
            title: graph.venn.title.clone(),
            circles,
            intersections,
        }),
        acc_title: None,
        acc_descr: None,
    }
}
