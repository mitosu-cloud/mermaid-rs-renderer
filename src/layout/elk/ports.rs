//! Finish compound port ordering after hierarchical sections are joined.
//!
//! The layered crossing counter sees the split hierarchy edges. Their joined
//! routes can still cross outside containers. Use ELK's greedy port-exchange
//! approach on those routes, keeping the node order and existing port slots.
use super::{Layout, MovedRun, Point, crossing_count, routing, segment_axis};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet, VecDeque};

/// Materialize implicit leaf ports in elkjs import order: ancestor-owned edges
/// before child edges, preserving declaration order within each container.
/// The native hierarchical importer visits internal edges first; otherwise it
/// creates their ports in a different order, changing barycenter ranks and the
/// whole compound layout. Keep ports free so ELK can choose their sides,
/// positions, and final order as usual.
pub(super) fn preserve_hierarchy_port_order(input: &mut Value, parents: &HashMap<String, String>) {
    if parents.is_empty() {
        return;
    }
    fn collect(v: &Value, used: &mut HashSet<String>, leaves: &mut HashSet<String>) {
        if let Some(id) = v.get("id").and_then(Value::as_str) {
            used.insert(id.to_string());
            // Leave user-specified ports intact. An empty container still has
            // a children array and must not be treated as a leaf shape.
            if v.get("children").is_none() && v.get("ports").is_none() {
                leaves.insert(id.to_string());
            }
        }
        for key in ["children", "ports"] {
            if let Some(children) = v.get(key).and_then(Value::as_array) {
                for child in children {
                    collect(child, used, leaves);
                }
            }
        }
    }
    let mut used = HashSet::new();
    let mut leaves = HashSet::new();
    collect(input, &mut used, &mut leaves);
    let root = input
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("root")
        .to_string();
    let mut containers = HashMap::new();
    let mut queue = VecDeque::from([&*input]);
    while let Some(container) = queue.pop_front() {
        if let Some(id) = container.get("id").and_then(Value::as_str) {
            containers.insert(id.to_string(), containers.len());
        }
        if let Some(children) = container.get("children").and_then(Value::as_array) {
            queue.extend(
                children
                    .iter()
                    .filter(|child| child.get("children").is_some()),
            );
        }
    }
    let Some(edges) = input.get_mut("edges").and_then(Value::as_array_mut) else {
        return;
    };
    let mut boundary_leaves = HashSet::new();
    for edge in edges.iter() {
        if let Some(id) = edge.get("id").and_then(Value::as_str) {
            used.insert(id.to_string());
        }
        let source = edge
            .get("sources")
            .and_then(|v| v.get(0))
            .and_then(Value::as_str);
        let target = edge
            .get("targets")
            .and_then(|v| v.get(0))
            .and_then(Value::as_str);
        if let (Some(source), Some(target)) = (source, target)
            && parents.get(source) != parents.get(target)
        {
            for id in [source, target] {
                if leaves.contains(id) {
                    boundary_leaves.insert(id.to_string());
                }
            }
        }
    }
    let mut node_ports = HashMap::<String, Vec<Value>>::new();
    let owner_rank = |edge: &Value| {
        let source = edge
            .get("sources")
            .and_then(|v| v.get(0))
            .and_then(Value::as_str);
        let target = edge
            .get("targets")
            .and_then(|v| v.get(0))
            .and_then(Value::as_str);
        let (Some(source), Some(target)) = (source, target) else {
            return 0;
        };
        let mut ancestors = HashSet::from([root.as_str()]);
        let mut ancestor = parents.get(source);
        while let Some(id) = ancestor {
            ancestors.insert(id.as_str());
            ancestor = parents.get(id);
        }
        let mut owner = parents.get(target).map(String::as_str).unwrap_or(&root);
        while !ancestors.contains(owner) {
            owner = parents.get(owner).map(String::as_str).unwrap_or(&root);
        }
        containers.get(owner).copied().unwrap_or(0)
    };
    let mut edge_order: Vec<_> = (0..edges.len()).collect();
    edge_order.sort_by_key(|&index| (owner_rank(&edges[index]), index));
    for index in edge_order {
        let edge = &mut edges[index];
        for key in ["sources", "targets"] {
            let Some(id) = edge
                .get(key)
                .and_then(|v| v.get(0))
                .and_then(Value::as_str)
                .filter(|id| boundary_leaves.contains(*id))
                .map(str::to_string)
            else {
                continue;
            };
            let mut port_id = format!("__mmdr_edge_{index}_{key}");
            while !used.insert(port_id.clone()) {
                port_id.push('_');
            }
            node_ports.entry(id).or_default().push(json!({
                "id": port_id, "width": 0, "height": 0
            }));
            edge[key] = json!([port_id]);
        }
    }
    fn install(v: &mut Value, ports: &mut HashMap<String, Vec<Value>>) {
        if let Some(p) = v
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| ports.remove(id))
        {
            v["ports"] = json!(p);
        }
        if let Some(children) = v.get_mut("children").and_then(Value::as_array_mut) {
            for child in children {
                install(child, ports);
            }
        }
    }
    install(input, &mut node_ports);
}

/// Move a free port and its whole terminal channel. The neighboring bend
/// remains perpendicular; the opposite port and every other channel stay put.
fn move_port(points: &[Point], at_start: bool, target: Point) -> Option<(Vec<Point>, MovedRun)> {
    let mut route = points.to_vec();
    if !at_start {
        route.reverse();
    }
    let first = *route.first()?;
    let horizontal = route.windows(2).find_map(|p| segment_axis(p[0], p[1]))?;
    if (if horizontal {
        target.0 - first.0
    } else {
        target.1 - first.1
    })
    .abs()
        > 0.01
    {
        return None;
    }
    let mut last = 0;
    while last + 1 < route.len() {
        let a = route[last];
        let b = route[last + 1];
        if segment_axis(a, b) != Some(horizontal) && (a.0 - b.0).hypot(a.1 - b.1) > 0.01 {
            break;
        }
        last += 1;
    }
    // A straight connection has no independent terminal channel to move.
    if last == 0
        || last + 1 == route.len()
        || segment_axis(route[last], route[last + 1]) != Some(!horizontal)
    {
        return None;
    }
    let original = (first, route[last]);
    for point in &mut route[..=last] {
        if horizontal {
            point.1 = target.1;
        } else {
            point.0 = target.0;
        }
    }
    let moved = (route[0], route[last]);
    if !at_start {
        route.reverse();
    }
    Some((route, (original, moved)))
}

fn overlap_length(a: &[Point], b: &[Point]) -> f32 {
    a.windows(2)
        .flat_map(|a| b.windows(2).map(move |b| (a, b)))
        .filter_map(|(a, b)| {
            let horizontal = segment_axis(a[0], a[1])?;
            if segment_axis(b[0], b[1]) != Some(horizontal) {
                return None;
            }
            let (a0, a1, b0, b1, distance) = if horizontal {
                (a[0].0, a[1].0, b[0].0, b[1].0, (a[0].1 - b[0].1).abs())
            } else {
                (a[0].1, a[1].1, b[0].1, b[1].1, (a[0].0 - b[0].0).abs())
            };
            (distance <= 0.01)
                .then(|| (a0.max(a1).min(b0.max(b1)) - a0.min(a1).max(b0.min(b1))).max(0.0))
        })
        .sum()
}

fn self_conflicts(points: &[Point]) -> (usize, f32) {
    let mut result = (0, 0.0);
    for (index, segment) in points.windows(2).enumerate() {
        for other in points.windows(2).skip(index + 2) {
            result.0 += crossing_count(segment, other);
            result.1 += overlap_length(segment, other);
        }
    }
    result
}

/// Score only pairs affected by the exchange. Common node endpoints do not
/// count as crossings, while coincident channel lengths are checked separately.
fn score(layout: &Layout, a: usize, b: usize, p: &[Point], q: &[Point]) -> (usize, f32) {
    let mut result = (crossing_count(p, q), overlap_length(p, q));
    for (index, edge) in layout.edges.iter().enumerate() {
        if index != a && index != b {
            result.0 += crossing_count(p, &edge.points) + crossing_count(q, &edge.points);
            result.1 += overlap_length(p, &edge.points) + overlap_length(q, &edge.points);
        }
    }
    result
}

fn obstacle_is_new(
    layout: &Layout,
    edge: usize,
    points: &[Point],
    obstacles: &[routing::Obstacle],
) -> bool {
    let edge = &layout.edges[edge];
    obstacles.iter().any(|obstacle| {
        if obstacle.id == edge.from
            || obstacle.id == edge.to
            || obstacle
                .members
                .as_ref()
                .is_some_and(|members| members.contains(&edge.from) || members.contains(&edge.to))
        {
            return false;
        }
        let intersects = |route: &[Point]| {
            route
                .windows(2)
                .any(|p| routing::segment_intersects_rect(p[0], p[1], obstacle))
        };
        intersects(points) && !intersects(&edge.points)
    })
}

fn move_label(edge: &mut super::super::EdgeLayout, ((a, b), (c, d)): MovedRun) {
    let Some(label) = edge.label_anchor else {
        return;
    };
    let ab = (b.0 - a.0, b.1 - a.1);
    let length_squared = ab.0 * ab.0 + ab.1 * ab.1;
    if length_squared <= 0.0 {
        return;
    }
    let t = (((label.0 - a.0) * ab.0 + (label.1 - a.1) * ab.1) / length_squared).clamp(0.0, 1.0);
    if (a.0 + t * ab.0 - label.0).hypot(a.1 + t * ab.1 - label.1) <= 0.5 {
        edge.label_anchor = Some((c.0 + t * (d.0 - c.0), c.1 + t * (d.1 - c.1)));
    }
}

pub(super) fn reduce_container_crossings(layout: &mut Layout, parents: &HashMap<String, String>) {
    if layout.subgraphs.is_empty() {
        return;
    }
    let mut obstacles: Vec<_> = layout
        .nodes
        .values()
        .filter(|n| !n.hidden && n.anchor_subgraph.is_none())
        .map(|n| routing::Obstacle {
            id: n.id.clone(),
            x: n.x,
            y: n.y,
            width: n.width,
            height: n.height,
            members: None,
        })
        .collect();
    for group in &layout.subgraphs {
        let Some(id) = &group.id else { continue };
        let mut members = HashSet::new();
        for child in parents.keys() {
            let mut parent = parents.get(child);
            while let Some(ancestor) = parent {
                if ancestor == id {
                    members.insert(child.clone());
                    break;
                }
                parent = parents.get(ancestor);
            }
        }
        obstacles.push(routing::Obstacle {
            id: id.clone(),
            x: group.x,
            y: group.y,
            width: group.width,
            height: group.height,
            members: Some(members),
        });
    }
    loop {
        let mut best = None;
        let mut best_reduction = 0;
        for group in &layout.subgraphs {
            let Some(id) = &group.id else { continue };
            // Only container ports are free. Leaf ports belong to measured
            // shapes and must retain ELK's anchors.
            let mut ports = Vec::new();
            for (index, edge) in layout.edges.iter().enumerate() {
                for (endpoint, at_start) in [(&edge.from, true), (&edge.to, false)] {
                    if endpoint != id {
                        continue;
                    }
                    let point = if at_start {
                        edge.points.first()
                    } else {
                        edge.points.last()
                    };
                    let Some(&p) = point else { continue };
                    let side = [
                        (routing::EdgeSide::Left, (p.0 - group.x).abs()),
                        (
                            routing::EdgeSide::Right,
                            (p.0 - group.x - group.width).abs(),
                        ),
                        (routing::EdgeSide::Top, (p.1 - group.y).abs()),
                        (
                            routing::EdgeSide::Bottom,
                            (p.1 - group.y - group.height).abs(),
                        ),
                    ]
                    .into_iter()
                    .find(|(_, distance)| *distance <= 0.01);
                    if let Some((side, _)) = side {
                        ports.push((index, at_start, p, side));
                    }
                }
            }
            for (index, &(a, a_start, a_port, side)) in ports.iter().enumerate() {
                for &(b, b_start, b_port, other_side) in &ports[index + 1..] {
                    if a == b || side != other_side {
                        continue;
                    }
                    let Some((p, p_run)) = move_port(&layout.edges[a].points, a_start, b_port)
                    else {
                        continue;
                    };
                    let Some((q, q_run)) = move_port(&layout.edges[b].points, b_start, a_port)
                    else {
                        continue;
                    };
                    let before = score(
                        layout,
                        a,
                        b,
                        &layout.edges[a].points,
                        &layout.edges[b].points,
                    );
                    let after = score(layout, a, b, &p, &q);
                    if after.0 >= before.0
                        || after.1 > before.1 + 0.01
                        || before.0 - after.0 <= best_reduction
                    {
                        continue;
                    }
                    let valid = [(a, &p), (b, &q)].into_iter().all(|(edge, points)| {
                        let before = self_conflicts(&layout.edges[edge].points);
                        let after = self_conflicts(points);
                        after.0 <= before.0
                            && after.1 <= before.1 + 0.01
                            && !obstacle_is_new(layout, edge, points, &obstacles)
                    });
                    if valid {
                        best_reduction = before.0 - after.0;
                        best = Some((a, b, p, q, p_run, q_run));
                    }
                }
            }
        }
        let Some((a, b, p, q, p_run, q_run)) = best else {
            break;
        };
        layout.edges[a].points = p;
        layout.edges[b].points = q;
        move_label(&mut layout.edges[a], p_run);
        move_label(&mut layout.edges[b], q_run);
        // Every accepted exchange strictly reduces the integer crossing count,
        // so the search terminates and retains the original order on ties.
    }
}
