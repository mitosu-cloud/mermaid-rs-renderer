use std::collections::{BTreeMap, HashMap};

use crate::config::LayoutConfig;
use crate::ir::{Graph, VennStyle, VennTextNode};
use crate::theme::{Theme, adjust_color};

use super::{
    DiagramData, Layout, VennCircleLayout, VennIntersectionLayout, VennLayout, VennTextNodeLayout,
};

const VENN_WIDTH: f32 = 800.0;
const VENN_HEIGHT: f32 = 450.0;
const VENN_SCALE: f32 = VENN_WIDTH / 1600.0;
const VENN_PADDING: f32 = 15.0;
const VENN_TEXT_RADIUS_FACTOR: f32 = 1.0335;
const VENN_SET_TEXT_AREA_OVERLAP_FACTOR: f32 = 1.095;

#[derive(Debug, Clone, Copy)]
struct VennTextArea {
    x: f32,
    y: f32,
    inner_radius: f32,
    has_label: bool,
}

pub(super) fn compute_venn_layout(graph: &Graph, theme: &Theme, _config: &LayoutConfig) -> Layout {
    let num_sets = graph.venn.sets.len();
    let title_height = if graph.venn.title.is_some() {
        48.0 * VENN_SCALE
    } else {
        0.0
    };

    if num_sets == 0 {
        return Layout {
            kind: graph.kind,
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
            width: VENN_WIDTH,
            height: VENN_HEIGHT,
            diagram: DiagramData::Venn(VennLayout {
                width: VENN_WIDTH,
                height: VENN_HEIGHT,
                title_height,
                title: graph.venn.title.clone(),
                circles: Vec::new(),
                intersections: Vec::new(),
                text_nodes: Vec::new(),
            }),
            acc_title: None,
            acc_descr: None,
        };
    }

    let content_height = VENN_HEIGHT - title_height;
    let max_radius =
        ((content_height - VENN_PADDING * 2.0) / 2.0).min((VENN_WIDTH - VENN_PADDING * 2.0) / 2.0);

    let max_size = graph
        .venn
        .sets
        .iter()
        .map(|s| s.size)
        .fold(0.0f32, f32::max)
        .max(1.0);

    let radii: Vec<f32> = graph
        .venn
        .sets
        .iter()
        .map(|s| max_radius * (s.size.max(0.0) / max_size).sqrt().max(0.01))
        .collect();

    let mut area_by_key: HashMap<String, VennTextArea> = HashMap::new();
    let mut set_centers: HashMap<String, (f32, f32)> = HashMap::new();
    let mut two_set_distance = None;

    let positions: Vec<(f32, f32)> = match num_sets {
        1 => vec![(VENN_WIDTH / 2.0, content_height / 2.0)],
        2 => {
            let target_area = two_set_target_area(graph, max_radius, max_size);
            let distance = solve_circle_distance(radii[0], radii[1], target_area);
            two_set_distance = Some(distance);
            let total_width = radii[0] + distance + radii[1];
            let x1 = (VENN_WIDTH - total_width) / 2.0 + radii[0];
            let x2 = x1 + distance;
            let cy = content_height / 2.0;
            vec![(x1, cy), (x2, cy)]
        }
        3 => {
            let r_avg = (radii[0] + radii[1] + radii[2]) / 3.0;
            let spread = r_avg * 1.05;
            let cx = VENN_WIDTH / 2.0;
            let cy = content_height / 2.0;
            vec![
                (cx, cy - spread * 0.6),
                (cx - spread * 0.866, cy + spread * 0.4),
                (cx + spread * 0.866, cy + spread * 0.4),
            ]
        }
        _ => {
            let r_avg: f32 = radii.iter().sum::<f32>() / num_sets as f32;
            let ring_radius = r_avg * 1.25;
            let cx = VENN_WIDTH / 2.0;
            let cy = content_height / 2.0;
            (0..num_sets)
                .map(|i| {
                    let angle = -std::f32::consts::FRAC_PI_2
                        + 2.0 * std::f32::consts::PI * i as f32 / num_sets as f32;
                    (
                        cx + ring_radius * angle.cos(),
                        cy + ring_radius * angle.sin(),
                    )
                })
                .collect()
        }
    };

    let mut circles = Vec::with_capacity(num_sets);
    let palette = default_venn_palette(theme);

    for (i, set) in graph.venn.sets.iter().enumerate() {
        let (cx, cy) = positions[i];
        set_centers.insert(set.id.clone(), (cx, cy));

        let default_color = palette[i % palette.len()].clone();
        let (color, fill_opacity, stroke, stroke_width, text_color) =
            venn_set_style(&set.style, default_color);
        let (label_x, label_y, inner_radius) =
            circle_text_area(i, &positions, &radii, two_set_distance);
        let text_area_x =
            circle_text_node_area_x(i, &positions, &radii, two_set_distance).unwrap_or(label_x);
        area_by_key.insert(
            stable_sets_key(std::slice::from_ref(&set.id)),
            VennTextArea {
                x: text_area_x,
                y: label_y,
                inner_radius,
                has_label: !set.label.is_empty(),
            },
        );

        circles.push(VennCircleLayout {
            id: set.id.clone(),
            label: set.label.clone(),
            cx,
            cy,
            radius: radii[i],
            label_x,
            label_y,
            color,
            fill_opacity,
            stroke,
            stroke_width,
            stroke_opacity: 0.95,
            text_color,
        });
    }

    let mut intersections = Vec::new();
    for union in &graph.venn.unions {
        if let Some(intersection) = build_intersection_layout(
            union,
            &graph.venn.sets,
            &set_centers,
            &positions,
            &radii,
            two_set_distance,
            theme,
            &mut area_by_key,
        ) {
            intersections.push(intersection);
        }
    }

    let text_nodes = build_text_node_layouts(&graph.venn.text_nodes, &area_by_key);

    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width: VENN_WIDTH,
        height: VENN_HEIGHT,
        diagram: DiagramData::Venn(VennLayout {
            width: VENN_WIDTH,
            height: VENN_HEIGHT,
            title_height,
            title: graph.venn.title.clone(),
            circles,
            intersections,
            text_nodes,
        }),
        acc_title: None,
        acc_descr: None,
    }
}

fn default_venn_palette(theme: &Theme) -> [String; 8] {
    [
        adjust_color(&theme.primary_color, 0.0, 0.0, -30.0),
        adjust_color(&theme.secondary_color, 0.0, 0.0, -30.0),
        adjust_color(&theme.tertiary_color, 0.0, 0.0, -40.0),
        adjust_color(&theme.primary_color, 60.0, 0.0, -30.0),
        adjust_color(&theme.primary_color, -60.0, 0.0, -30.0),
        adjust_color(&theme.secondary_color, 60.0, 0.0, -30.0),
        adjust_color(&theme.primary_color, 120.0, 0.0, -30.0),
        adjust_color(&theme.secondary_color, 120.0, 0.0, -30.0),
    ]
}

fn venn_set_style(
    style: &Option<VennStyle>,
    default_color: String,
) -> (String, f32, String, f32, String) {
    let color = style
        .as_ref()
        .and_then(|s| s.fill.clone())
        .unwrap_or(default_color);
    let fill_opacity = style.as_ref().and_then(|s| s.fill_opacity).unwrap_or(0.1);
    let stroke = style
        .as_ref()
        .and_then(|s| s.stroke.clone())
        .unwrap_or_else(|| color.clone());
    let stroke_width = style
        .as_ref()
        .and_then(|s| s.stroke_width)
        .unwrap_or(5.0 * VENN_SCALE);
    let text_color = style
        .as_ref()
        .and_then(|s| s.color.clone())
        .unwrap_or_else(|| adjust_color(&color, 0.0, 0.0, -30.0));

    (color, fill_opacity, stroke, stroke_width, text_color)
}

fn circle_text_area(
    index: usize,
    positions: &[(f32, f32)],
    radii: &[f32],
    two_set_distance: Option<f32>,
) -> (f32, f32, f32) {
    if positions.len() == 2 {
        let d = two_set_distance.unwrap_or_else(|| {
            let dx = positions[1].0 - positions[0].0;
            let dy = positions[1].1 - positions[0].1;
            (dx * dx + dy * dy).sqrt()
        });
        let overlap_margin = ((radii[0] + radii[1] - d) / 2.0).max(0.0);
        let (cx, cy) = positions[index];
        let label_x = if index == 0 {
            cx - overlap_margin
        } else {
            cx + overlap_margin
        };
        let inner_radius = (radii[index] - overlap_margin).max(0.0) * VENN_TEXT_RADIUS_FACTOR;
        return (label_x, cy, inner_radius);
    }

    let (cx, cy) = positions[index];
    (cx, cy, radii[index] * VENN_TEXT_RADIUS_FACTOR)
}

fn circle_text_node_area_x(
    index: usize,
    positions: &[(f32, f32)],
    radii: &[f32],
    two_set_distance: Option<f32>,
) -> Option<f32> {
    if positions.len() != 2 {
        return None;
    }
    let d = two_set_distance.unwrap_or_else(|| {
        let dx = positions[1].0 - positions[0].0;
        let dy = positions[1].1 - positions[0].1;
        (dx * dx + dy * dy).sqrt()
    });
    let overlap_margin = ((radii[0] + radii[1] - d) / 2.0).max(0.0);
    let direction = if index == 0 { -1.0 } else { 1.0 };
    Some(positions[index].0 + direction * overlap_margin * VENN_SET_TEXT_AREA_OVERLAP_FACTOR)
}

fn two_set_target_area(graph: &Graph, max_radius: f32, max_size: f32) -> f32 {
    if graph.venn.sets.len() < 2 {
        return 0.0;
    }
    let mut target_ids = vec![graph.venn.sets[0].id.clone(), graph.venn.sets[1].id.clone()];
    target_ids.sort();
    let Some(union) = graph.venn.unions.iter().find(|union| {
        let mut ids = union.set_ids.clone();
        ids.sort();
        ids == target_ids
    }) else {
        return 0.0;
    };
    let area_per_unit = std::f32::consts::PI * max_radius * max_radius / max_size;
    union.size.max(0.0) * area_per_unit
}

fn solve_circle_distance(r1: f32, r2: f32, target_area: f32) -> f32 {
    let max_overlap = std::f32::consts::PI * r1.min(r2).powi(2);
    if target_area <= 0.0 {
        return r1 + r2;
    }
    if target_area >= max_overlap {
        return (r1 - r2).abs();
    }

    let mut lo = (r1 - r2).abs();
    let mut hi = r1 + r2;
    for _ in 0..80 {
        let mid = (lo + hi) / 2.0;
        let area = circle_intersection_area(r1, r2, mid);
        if area > target_area {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

fn circle_intersection_area(r1: f32, r2: f32, d: f32) -> f32 {
    if d >= r1 + r2 {
        return 0.0;
    }
    if d <= (r1 - r2).abs() {
        return std::f32::consts::PI * r1.min(r2).powi(2);
    }
    let a1 = ((d * d + r1 * r1 - r2 * r2) / (2.0 * d * r1)).clamp(-1.0, 1.0);
    let a2 = ((d * d + r2 * r2 - r1 * r1) / (2.0 * d * r2)).clamp(-1.0, 1.0);
    let radical = (-d + r1 + r2) * (d + r1 - r2) * (d - r1 + r2) * (d + r1 + r2);
    r1 * r1 * a1.acos() + r2 * r2 * a2.acos() - 0.5 * radical.max(0.0).sqrt()
}

fn build_intersection_layout(
    union: &crate::ir::VennUnion,
    sets: &[crate::ir::VennSet],
    set_centers: &HashMap<String, (f32, f32)>,
    positions: &[(f32, f32)],
    radii: &[f32],
    two_set_distance: Option<f32>,
    theme: &Theme,
    area_by_key: &mut HashMap<String, VennTextArea>,
) -> Option<VennIntersectionLayout> {
    let text_color = union
        .style
        .as_ref()
        .and_then(|s| s.color.clone())
        .unwrap_or_else(|| theme.text_color.clone());
    let custom_fill = union.style.as_ref().and_then(|s| s.fill.clone());
    let fill = custom_fill.unwrap_or_else(|| "transparent".to_string());
    let fill_opacity = if fill == "transparent" { 0.0 } else { 1.0 };

    if union.set_ids.len() == 2 && sets.len() >= 2 {
        let mut first_two = vec![sets[0].id.clone(), sets[1].id.clone()];
        first_two.sort();
        let mut union_ids = union.set_ids.clone();
        union_ids.sort();
        if union_ids == first_two {
            let d = two_set_distance.unwrap_or_else(|| {
                let dx = positions[1].0 - positions[0].0;
                let dy = positions[1].1 - positions[0].1;
                (dx * dx + dy * dy).sqrt()
            });
            let x = (positions[0].0 + positions[1].0 + radii[0] - radii[1]) / 2.0;
            let y = positions[0].1;
            let path = intersection_path(positions[0], radii[0], positions[1], radii[1], d);
            let inner_radius = (radii[0] - (x - positions[0].0).abs())
                .min(radii[1] - (positions[1].0 - x).abs())
                .max(0.0)
                * VENN_TEXT_RADIUS_FACTOR;
            area_by_key.insert(
                stable_sets_key(&union.set_ids),
                VennTextArea {
                    x,
                    y,
                    inner_radius,
                    has_label: union.label.as_ref().is_some_and(|label| !label.is_empty()),
                },
            );
            return Some(VennIntersectionLayout {
                set_ids: union.set_ids.clone(),
                label: union.label.clone(),
                cx: x,
                cy: y,
                path,
                fill,
                fill_opacity,
                text_color,
            });
        }
    }

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
    if count == 0 {
        return None;
    }
    let x = sum_x / count as f32;
    let y = sum_y / count as f32;
    area_by_key.insert(
        stable_sets_key(&union.set_ids),
        VennTextArea {
            x,
            y,
            inner_radius: 60.0,
            has_label: union.label.as_ref().is_some_and(|label| !label.is_empty()),
        },
    );
    Some(VennIntersectionLayout {
        set_ids: union.set_ids.clone(),
        label: union.label.clone(),
        cx: x,
        cy: y,
        path: None,
        fill,
        fill_opacity,
        text_color,
    })
}

fn intersection_path(
    p1: (f32, f32),
    r1: f32,
    _p2: (f32, f32),
    r2: f32,
    distance: f32,
) -> Option<String> {
    if distance <= 0.0 || distance >= r1 + r2 {
        return None;
    }
    let a = (r1 * r1 - r2 * r2 + distance * distance) / (2.0 * distance);
    let h = (r1 * r1 - a * a).max(0.0).sqrt();
    let x = p1.0 + a;
    let top_y = p1.1 - h;
    let bottom_y = p1.1 + h;
    Some(format!(
        "M {x} {bottom_y} A {r2} {r2} 0 0 1 {x} {top_y} A {r1} {r1} 0 0 1 {x} {bottom_y}"
    ))
}

fn build_text_node_layouts(
    text_nodes: &[VennTextNode],
    area_by_key: &HashMap<String, VennTextArea>,
) -> Vec<VennTextNodeLayout> {
    let mut grouped: Vec<(String, Vec<&VennTextNode>)> = Vec::new();
    for node in text_nodes {
        let key = stable_sets_key(&node.set_ids);
        if let Some((_, nodes)) = grouped.iter_mut().find(|(existing, _)| existing == &key) {
            nodes.push(node);
        } else {
            grouped.push((key, vec![node]));
        }
    }

    let mut layouts = Vec::new();
    for (key, nodes) in grouped {
        let Some(area) = area_by_key.get(&key) else {
            continue;
        };
        let inner_radius = area.inner_radius.max(0.0);
        let inner_width = (80.0 * VENN_SCALE).max(inner_radius * 2.0 * 0.95);
        let inner_height = (60.0 * VENN_SCALE).max(inner_radius * 2.0 * 0.95);
        let label_offset_base = if area.has_label {
            (32.0 * VENN_SCALE).min(inner_radius * 0.25)
        } else {
            0.0
        };
        let label_offset = label_offset_base
            + if nodes.len() <= 2 {
                30.0 * VENN_SCALE
            } else {
                0.0
            };
        let start_x = area.x - inner_width / 2.0;
        let start_y = area.y - inner_height / 2.0 + label_offset;
        let cols = (nodes.len() as f32).sqrt().ceil().max(1.0) as usize;
        let rows = ((nodes.len() as f32) / cols as f32).ceil().max(1.0) as usize;
        let cell_width = inner_width / cols as f32;
        let cell_height = inner_height / rows as f32;

        for (i, node) in nodes.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let cx = start_x + cell_width * (col as f32 + 0.5);
            let cy = start_y + cell_height * (row as f32 + 0.5);
            let box_width = cell_width * 0.9;
            let box_height = cell_height * 0.9;
            layouts.push(VennTextNodeLayout {
                id: node.id.clone(),
                label: node.label.clone().unwrap_or_else(|| node.id.clone()),
                x: cx - box_width / 2.0,
                y: cy - box_height / 2.0,
                width: box_width,
                height: box_height,
                color: node.style.as_ref().and_then(|style| style.color.clone()),
            });
        }
    }
    layouts
}

fn stable_sets_key(set_ids: &[String]) -> String {
    let mut ids = set_ids.to_vec();
    ids.sort();
    ids.join("|")
}
