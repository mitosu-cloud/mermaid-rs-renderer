//! Flat connected-tree subset of COSE-Bilkent, used by Mermaid's mindmaps.
//! Radial initialization, rectangular clipping, forces, and proof cooling
//! follow cose-base / layout-base. Compound and disconnected graphs fall back
//! to the existing mindmap placement rather than entering this solver.
/*
MIT License
Copyright (c) 2019 - present, iVis@Bilkent.
Copyright (c) 2019 iVis@Bilkent

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

use crate::{ir::Graph, layout::NodeLayout};
use std::collections::{BTreeMap, HashMap, VecDeque};

#[derive(Clone, Copy)]
struct Body {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Body {
    fn overlaps(self, other: Self) -> bool {
        (other.x - self.x).abs() <= (self.w + other.w) / 2.0
            && (other.y - self.y).abs() <= (self.h + other.h) / 2.0
    }

    fn clipped_delta(self, other: Self) -> (f64, f64) {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        let first = (self.w / (2.0 * dx.abs())).min(self.h / (2.0 * dy.abs()));
        let second = (other.w / (2.0 * dx.abs())).min(other.h / (2.0 * dy.abs()));
        (
            (other.x - dx * second) - (self.x + dx * first),
            (other.y - dy * second) - (self.y + dy * first),
        )
    }

    fn separation(self, other: Self) -> (f64, f64) {
        let (al, ar, at, ab) = (
            self.x - self.w / 2.0,
            self.x + self.w / 2.0,
            self.y - self.h / 2.0,
            self.y + self.h / 2.0,
        );
        let (bl, br, bt, bb) = (
            other.x - other.w / 2.0,
            other.x + other.w / 2.0,
            other.y - other.h / 2.0,
            other.y + other.h / 2.0,
        );
        let mut ox = ar.min(br) - al.max(bl);
        let mut oy = ab.min(bb) - at.max(bt);
        if al <= bl && ar >= br {
            ox += (bl - al).min(ar - br);
        } else if bl <= al && br >= ar {
            ox += (al - bl).min(br - ar);
        }
        if at <= bt && ab >= bb {
            oy += (bt - at).min(ab - bb);
        } else if bt <= at && bb >= ab {
            oy += (at - bt).min(bb - ab);
        }
        let slope = if self.x == other.x && self.y == other.y {
            1.0
        } else {
            ((other.y - self.y) / (other.x - self.x)).abs()
        };
        let mut mx = oy / slope;
        let mut my = slope * ox;
        if ox < mx {
            mx = ox;
        } else {
            my = oy;
        }
        (
            (mx / 2.0 + 25.0) * if self.x < other.x { 1.0 } else { -1.0 },
            (my / 2.0 + 25.0) * if self.y < other.y { 1.0 } else { -1.0 },
        )
    }
}

fn min_component(value: f64, minimum: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else if value.abs() < minimum {
        value.signum() * minimum
    } else {
        value
    }
}

fn seed_radially(
    index: usize,
    parent: Option<usize>,
    start: f64,
    end: f64,
    distance: f64,
    separation: f64,
    neighbors: &[Vec<usize>],
    bodies: &mut [Body],
) {
    let mut half_interval = (end - start + 1.0) / 2.0;
    if half_interval < 0.0 {
        half_interval += 180.0;
    }
    let angle = ((half_interval + start) % 360.0).to_radians();
    bodies[index].x = distance * angle.cos();
    bodies[index].y = distance * angle.sin();
    let adjacent = &neighbors[index];
    let count = adjacent.len() - usize::from(parent.is_some());
    if count == 0 {
        return;
    }
    let first = parent
        .and_then(|parent| adjacent.iter().position(|neighbor| *neighbor == parent))
        .map(|position| (position + 1) % adjacent.len())
        .unwrap_or(0);
    let step = (end - start).abs() / count as f64;
    let mut branch = 0;
    for offset in 0..adjacent.len() {
        let next = adjacent[(first + offset) % adjacent.len()];
        if Some(next) == parent {
            continue;
        }
        let child_start = (start + branch as f64 * step) % 360.0;
        seed_radially(
            next,
            Some(index),
            child_start,
            (child_start + step) % 360.0,
            distance + separation,
            separation,
            neighbors,
            bodies,
        );
        branch += 1;
    }
}

pub(super) fn place(graph: &Graph, nodes: &mut BTreeMap<String, NodeLayout>) -> bool {
    let ids: Vec<&str> = graph
        .mindmap
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect();
    if ids.len() < 2 || ids.len() != nodes.len() || graph.edges.len() != ids.len() - 1 {
        return false;
    }
    let indices: HashMap<&str, usize> = ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let mut neighbors = vec![Vec::new(); ids.len()];
    let mut edges = Vec::new();
    for edge in &graph.edges {
        let (Some(&a), Some(&b)) = (
            indices.get(edge.from.as_str()),
            indices.get(edge.to.as_str()),
        ) else {
            return false;
        };
        if a == b || neighbors[a].contains(&b) {
            return false;
        }
        neighbors[a].push(b);
        neighbors[b].push(a);
        edges.push((a, b));
    }
    let mut order = Vec::new();
    let mut queue = VecDeque::from([0]);
    let mut visited = vec![false; ids.len()];
    visited[0] = true;
    while let Some(index) = queue.pop_front() {
        order.push(index);
        for &next in &neighbors[index] {
            if !visited[next] {
                visited[next] = true;
                queue.push_back(next);
            }
        }
    }
    if order.len() != ids.len() {
        return false;
    }
    // Match the installed layout-base center selection, which removes entries
    // while walking its BFS list. Stable ordering also fixes branch orientation.
    while order.len() > 2 {
        let mut index = 0;
        while index < order.len() {
            order.remove(index);
            index += 1;
        }
    }
    let mut bodies: Vec<Body> = ids
        .iter()
        .map(|id| {
            let node = &nodes[*id];
            Body {
                x: 0.0,
                y: 0.0,
                w: node.width as f64,
                h: node.height as f64,
            }
        })
        .collect();
    let separation = bodies
        .iter()
        .map(|body| body.w.hypot(body.h))
        .fold(50.0, f64::max);
    seed_radially(
        order[0],
        None,
        0.0,
        359.0,
        0.0,
        separation,
        &neighbors,
        &mut bodies,
    );
    // Keep the upstream world center and pixel-rounded bounds: translation
    // can affect floating-point symmetry when clipping almost vertical edges.
    let left = bodies
        .iter()
        .map(|body| body.x - body.w / 2.0)
        .fold(f64::INFINITY, f64::min)
        .floor();
    let right = bodies
        .iter()
        .map(|body| body.x + body.w / 2.0)
        .fold(f64::NEG_INFINITY, f64::max)
        .ceil();
    let top = bodies
        .iter()
        .map(|body| body.y - body.h / 2.0)
        .fold(f64::INFINITY, f64::min)
        .floor();
    let bottom = bodies
        .iter()
        .map(|body| body.y + body.h / 2.0)
        .fold(f64::NEG_INFINITY, f64::max)
        .ceil();
    for body in &mut bodies {
        body.x += 1200.0 - (left + right) / 2.0;
        body.y += 900.0 - (top + bottom) / 2.0;
    }
    let max_iterations = (ids.len() * 5).max(2500);
    let max_cycles = 25.0_f64;
    let temperature = 0.04_f64;
    let exponent = (100.0 * (1.0 - temperature)).ln() / max_cycles.ln();
    let mut cooling = 1.0;
    let mut cycle = 0.0_f64;
    let mut displacement = 0.0_f64;
    let mut old_displacement = 0.0_f64;
    let mut pairs = Vec::new();
    for iteration in 1..max_iterations {
        if iteration % 100 == 0 {
            if displacement < 1.5 * ids.len() as f64
                || (iteration > max_iterations / 3 && (displacement - old_displacement).abs() < 2.0)
            {
                break;
            }
            old_displacement = displacement;
            cycle += 1.0;
            cooling = (1.0 - cycle.powf(exponent) / 100.0).max(temperature);
        }
        let mut spring = vec![(0.0, 0.0); ids.len()];
        for &(a, b) in &edges {
            if bodies[a].overlaps(bodies[b]) {
                continue;
            }
            let (dx, dy) = bodies[a].clipped_delta(bodies[b]);
            let (dx, dy) = (min_component(dx, 1.0), min_component(dy, 1.0));
            let length = dx.hypot(dy);
            if length == 0.0 {
                continue;
            }
            let factor = 0.45 * (length - 50.0) / length;
            spring[a].0 += factor * dx;
            spring[a].1 += factor * dy;
            spring[b].0 -= factor * dx;
            spring[b].1 -= factor * dy;
        }
        // Cache nearby pairs for ten iterations, like the FR grid variant.
        // The rectangle-distance filter is equivalent for this flat graph.
        if iteration % 10 == 1 {
            pairs.clear();
            for a in 0..bodies.len() {
                for b in a + 1..bodies.len() {
                    if (bodies[b].x - bodies[a].x).abs() - (bodies[a].w + bodies[b].w) / 2.0
                        <= 100.0
                        && (bodies[b].y - bodies[a].y).abs() - (bodies[a].h + bodies[b].h) / 2.0
                            <= 100.0
                    {
                        pairs.push((a, b));
                    }
                }
            }
        }
        let mut repulsion = vec![(0.0, 0.0); ids.len()];
        for &(a, b) in &pairs {
            let (fx, fy) = if bodies[a].overlaps(bodies[b]) {
                bodies[a].separation(bodies[b])
            } else {
                let (dx, dy) = bodies[a].clipped_delta(bodies[b]);
                let (dx, dy) = (min_component(dx, 5.0), min_component(dy, 5.0));
                let squared = dx * dx + dy * dy;
                let factor = 4500.0 / (squared * squared.sqrt());
                (factor * dx, factor * dy)
            };
            repulsion[a].0 -= fx;
            repulsion[a].1 -= fy;
            repulsion[b].0 += fx;
            repulsion[b].1 += fy;
        }
        displacement = 0.0;
        for index in 0..bodies.len() {
            let dx = (cooling * (spring[index].0 + repulsion[index].0))
                .clamp(-cooling * 300.0, cooling * 300.0);
            let dy = (cooling * (spring[index].1 + repulsion[index].1))
                .clamp(-cooling * 300.0, cooling * 300.0);
            bodies[index].x += dx;
            bodies[index].y += dy;
            displacement += dx.abs() + dy.abs();
        }
    }
    if bodies
        .iter()
        .any(|body| !body.x.is_finite() || !body.y.is_finite())
    {
        return false;
    }
    for (id, body) in ids.into_iter().zip(bodies) {
        let node = nodes.get_mut(id).unwrap();
        node.x = body.x as f32 - node.width / 2.0;
        node.y = body.y as f32 - node.height / 2.0;
    }
    true
}
