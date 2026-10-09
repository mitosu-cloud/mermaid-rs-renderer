//! Native Dagre adapter, including Mermaid's extraction of isolated clusters.
use super::{Layout, routing};
use crate::ir::{Direction, Graph};
use dagre_native::{
    EdgeLabel, GraphLabel, GraphOptions, LabelPos, LayoutGraph, NodeLabel, RankDir,
};
use std::collections::{HashMap, HashSet};

fn direction(dir: Direction) -> RankDir {
    match dir {
        Direction::TopDown => RankDir::TB,
        Direction::BottomTop => RankDir::BT,
        Direction::LeftRight => RankDir::LR,
        Direction::RightLeft => RankDir::RL,
    }
}

pub(super) fn apply(graph: &Graph, layout: &mut Layout) -> Result<(), String> {
    let mut parents = HashMap::new();
    for sub in &graph.subgraphs {
        let Some(id) = &sub.id else {
            continue;
        };
        if let Some(parent) = graph
            .element_metadata
            .get(id)
            .and_then(|m| m.get("_parent"))
            .and_then(|p| p.as_str())
        {
            parents.insert(id.clone(), parent.to_string());
        }
    }
    for (id, node) in &layout.nodes {
        if node.anchor_subgraph.is_some() {
            continue;
        }
        if let Some(parent) = graph
            .subgraphs
            .iter()
            .filter(|s| s.nodes.contains(id))
            .min_by_key(|s| s.nodes.len())
            .and_then(|s| s.id.clone())
        {
            parents.insert(id.clone(), parent);
        }
    }
    let mut ids: Vec<_> = layout
        .nodes
        .iter()
        .filter(|(_, n)| !n.hidden && n.anchor_subgraph.is_none())
        .map(|(id, _)| id.clone())
        .collect();
    ids.extend(graph.subgraphs.iter().filter_map(|s| s.id.clone()));
    ids.sort_by_key(|id| graph.node_order.get(id).copied().unwrap_or(usize::MAX));
    let descendants = |id: &str| -> HashSet<String> {
        ids.iter()
            .filter(|node| {
                let mut parent = parents.get(*node);
                while let Some(p) = parent {
                    if p == id {
                        return true;
                    }
                    parent = parents.get(p);
                }
                false
            })
            .cloned()
            .collect()
    };
    let descendants: HashMap<_, _> = graph
        .subgraphs
        .iter()
        .filter_map(|s| s.id.as_ref())
        .map(|id| (id.clone(), descendants(id)))
        .collect();
    let external: HashSet<_> = descendants
        .iter()
        .filter(|(_, nodes)| {
            graph
                .edges
                .iter()
                .any(|e| nodes.contains(&e.from) != nodes.contains(&e.to))
        })
        .map(|(id, _)| id.clone())
        .collect();
    let options = graph
        .agentflow_config
        .as_ref()
        .map(crate::agentflow::options);
    let spacing = |key: &str| {
        options
            .and_then(|o| o.get(key))
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0)
    };
    run(
        graph,
        layout,
        &ids,
        &parents,
        &descendants,
        &external,
        None,
        graph.direction,
        spacing("nodeSpacing"),
        spacing("rankSpacing"),
        0,
    )?;
    elk_normalize(graph, layout);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run(
    graph: &Graph,
    layout: &mut Layout,
    ids: &[String],
    parents: &HashMap<String, String>,
    descendants: &HashMap<String, HashSet<String>>,
    external: &HashSet<String>,
    current: Option<&str>,
    dir: Direction,
    nodesep: f64,
    ranksep: f64,
    depth: usize,
) -> Result<(), String> {
    if depth > 20 {
        return Err("Dagre cluster nesting exceeds 20 levels".into());
    }
    let mut extracted = HashMap::new();
    let mut hidden = HashSet::new();
    for id in ids {
        if current == Some(id.as_str()) || hidden.contains(id) || external.contains(id) {
            continue;
        }
        let Some(children) = descendants.get(id).filter(|children| !children.is_empty()) else {
            continue;
        };
        let inner: Vec<_> = ids
            .iter()
            .filter(|n| *n == id || children.contains(*n))
            .cloned()
            .collect();
        let child_dir = graph
            .subgraphs
            .iter()
            .find(|s| s.id.as_ref() == Some(id))
            .and_then(|s| s.direction)
            .unwrap_or(if dir == Direction::TopDown {
                Direction::LeftRight
            } else {
                Direction::TopDown
            });
        run(
            graph,
            layout,
            &inner,
            parents,
            descendants,
            external,
            Some(id),
            child_dir,
            nodesep,
            ranksep + 25.0,
            depth + 1,
        )?;
        let sub = layout
            .subgraphs
            .iter()
            .find(|s| s.id.as_ref() == Some(id))
            .ok_or_else(|| format!("Missing Dagre cluster {id}"))?;
        extracted.insert(id.clone(), (sub.x, sub.y, sub.width, sub.height));
        hidden.extend(children.iter().cloned());
    }
    let visible: Vec<_> = ids
        .iter()
        .filter(|id| !hidden.contains(*id))
        .cloned()
        .collect();
    let mut dagre = LayoutGraph::with_options(&GraphOptions {
        directed: true,
        multigraph: true,
        compound: true,
    });
    dagre.set_graph(GraphLabel {
        rankdir: direction(dir),
        rankdir_explicit: true,
        nodesep,
        ranksep,
        marginx: 8.0,
        marginy: 8.0,
        ..Default::default()
    });
    for id in &visible {
        let (width, height) = if let Some((_, _, w, h)) = extracted.get(id) {
            (*w, *h)
        } else if descendants.contains_key(id) {
            (0.0, 0.0)
        } else {
            let n = &layout.nodes[id];
            (n.width, n.height)
        };
        dagre.set_node(
            id,
            Some(NodeLabel {
                width: width as f64,
                height: height as f64,
                ..Default::default()
            }),
        );
        if let Some(parent) = parents.get(id).filter(|p| visible.contains(p)) {
            dagre.set_parent(id, Some(parent));
        }
    }
    let anchor = |id: &str| -> String {
        if !external.contains(id) {
            return id.to_string();
        }
        visible
            .iter()
            .find(|n| {
                descendants
                    .get(id)
                    .is_some_and(|children| children.contains(*n))
                    && (!descendants.contains_key(*n) || extracted.contains_key(*n))
            })
            .cloned()
            .unwrap_or_else(|| id.to_string())
    };
    let mut edges = Vec::new();
    for (index, edge) in graph.edges.iter().enumerate() {
        if !visible.contains(&edge.from) || !visible.contains(&edge.to) {
            continue;
        }
        let (from, to) = (anchor(&edge.from), anchor(&edge.to));
        let mut label = EdgeLabel {
            labelpos: LabelPos::Center,
            ..Default::default()
        };
        if let Some(block) = &layout.edges[index].label {
            label.width = block.width as f64;
            label.height = block.height as f64;
        }
        dagre.set_edge(&from, &to, Some(label), Some(&index.to_string()));
        edges.push((index, from, to));
    }
    dagre_native::layout(&mut dagre);
    // Mermaid reserves title space after Dagre, equally around regular nodes.
    let title_margin = 0.0_f32;
    for id in &visible {
        let output = dagre
            .node(id)
            .ok_or_else(|| format!("Dagre lost node {id}"))?;
        let (cx, cy) = (
            output.x.unwrap_or(0.0) as f32,
            output.y.unwrap_or(0.0) as f32,
        );
        if let Some(&(old_x, old_y, width, height)) = extracted.get(id) {
            let (x, y) = (cx - width / 2.0, cy - height / 2.0 + title_margin);
            translate_members(layout, &descendants[id], (x - old_x, y - old_y));
            if let Some(sub) = layout
                .subgraphs
                .iter_mut()
                .find(|s| s.id.as_ref() == Some(id))
            {
                sub.x = x;
                sub.y = y;
            }
            if let Some(node) = layout.nodes.get_mut(id) {
                node.x = x;
                node.y = y;
                node.width = width;
                node.height = height;
            }
        } else if let Some(sub) = layout
            .subgraphs
            .iter_mut()
            .find(|s| s.id.as_ref() == Some(id))
        {
            sub.width = output.width as f32;
            sub.height = output.height as f32 + title_margin;
            sub.x = cx - sub.width / 2.0;
            sub.y = cy - sub.height / 2.0;
            if let Some(node) = layout.nodes.get_mut(id) {
                node.x = sub.x;
                node.y = sub.y;
                node.width = sub.width;
                node.height = sub.height;
            }
        } else if let Some(node) = layout.nodes.get_mut(id) {
            node.x = cx - node.width / 2.0;
            node.y = cy - node.height / 2.0 + title_margin / 2.0;
        }
    }
    for (index, from, to) in edges {
        let edge = dagre
            .edge(&from, &to, Some(&index.to_string()))
            .ok_or_else(|| format!("Dagre lost edge {index}"))?;
        let native = &mut layout.edges[index];
        native.points = edge
            .points
            .iter()
            .map(|p| (p.x as f32, p.y as f32 + title_margin / 2.0))
            .collect();
        native.label_anchor = edge
            .x
            .zip(edge.y)
            .map(|(x, y)| (x as f32, y as f32 + title_margin / 2.0));
        for (id, start) in [(&native.from, true), (&native.to, false)] {
            if native.points.len() < 2 {
                continue;
            }
            if let Some(node) = layout.nodes.get(id) {
                let center = (node.x + node.width / 2.0, node.y + node.height / 2.0);
                let neighbor = if start {
                    native.points[1]
                } else {
                    native.points[native.points.len() - 2]
                };
                let ray = (neighbor.0 - center.0, neighbor.1 - center.1);
                let polygon = routing::shape_polygon_points(node).unwrap_or_else(|| {
                    vec![
                        (node.x, node.y),
                        (node.x + node.width, node.y),
                        (node.x + node.width, node.y + node.height),
                        (node.x, node.y + node.height),
                    ]
                });
                if let Some(point) = routing::ray_polygon_intersection(center, ray, &polygon) {
                    let end = if start { 0 } else { native.points.len() - 1 };
                    native.points[end] = point;
                }
            }
        }
    }
    Ok(())
}

fn translate_members(layout: &mut Layout, members: &HashSet<String>, shift: (f32, f32)) {
    for (id, node) in &mut layout.nodes {
        if members.contains(id) {
            node.x += shift.0;
            node.y += shift.1;
        }
    }
    for sub in &mut layout.subgraphs {
        if sub.id.as_ref().is_some_and(|id| members.contains(id)) {
            sub.x += shift.0;
            sub.y += shift.1;
        }
    }
    for edge in &mut layout.edges {
        if members.contains(&edge.from) && members.contains(&edge.to) {
            for p in &mut edge.points {
                p.0 += shift.0;
                p.1 += shift.1;
            }
            if let Some(p) = &mut edge.label_anchor {
                p.0 += shift.0;
                p.1 += shift.1;
            }
        }
    }
}

fn elk_normalize(graph: &Graph, layout: &mut Layout) {
    let pad = graph
        .agentflow_config
        .as_ref()
        .map(crate::agentflow::options)
        .and_then(|v| v.get("diagramPadding"))
        .and_then(|v| v.as_f64())
        .unwrap_or(8.0) as f32;
    let min_x = layout
        .nodes
        .values()
        .filter(|n| !n.hidden)
        .map(|n| n.x)
        .chain(layout.subgraphs.iter().map(|s| s.x))
        .fold(f32::INFINITY, f32::min);
    let min_y = layout
        .nodes
        .values()
        .filter(|n| !n.hidden)
        .map(|n| n.y)
        .chain(layout.subgraphs.iter().map(|s| s.y))
        .fold(f32::INFINITY, f32::min);
    let ids = layout
        .nodes
        .keys()
        .cloned()
        .chain(layout.subgraphs.iter().filter_map(|s| s.id.clone()))
        .collect();
    translate_members(layout, &ids, (pad - min_x, pad - min_y));
    let (w, h) =
        super::bounds_with_edges_capped(&layout.nodes, &layout.subgraphs, &layout.edges, Some(0.0));
    layout.width = w + pad;
    layout.height = h + pad;
}
