//! Iter 258: state-diagram dagre layout pipeline. Wires together
//! Network Simplex (rank assignment) + Brandes-Köpf (X coordinate
//! assignment) into a unified position assigner that mirrors what
//! JS dagre does for state diagrams.
//!
//! Pipeline:
//!   1. Compute global ranks via NS over the full graph (cluster members
//!      included, all edges including cross-cluster).
//!   2. Order nodes within each rank using the median-of-neighbors
//!      heuristic (a small subset of dagre's order phase).
//!   3. Compute X coordinates via BK.
//!   4. Compute Y coordinates via rank * (ranksep + max_h_per_rank/2 +
//!      next_h/2).
//!   5. Return (id → x, y) absolute positions.
//!
//! This is INTEGRATED via `apply_state_dagre_positions` in mod.rs as a
//! state-diagram-only override that runs after the existing per-cluster
//! layout. Cluster bboxes get re-derived from member positions in
//! `build_subgraph_layouts`.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::config::LayoutConfig;
use crate::ir::{Direction, Graph};
use crate::layout::brandes_kopf;
use crate::layout::types::NodeLayout;

/// Apply NS+BK dagre positions to state-diagram nodes. Replaces existing
/// X and Y positions for nodes participating in the global graph.
/// Falls back gracefully (no-op) for diagrams without the structure
/// dagre expects.
pub(super) fn apply_state_dagre_positions(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    rank_spacing: f32,
    node_spacing: f32,
) {
    // Get all visible non-hidden nodes that participate.
    let participating: Vec<String> = nodes
        .iter()
        .filter(|(_, n)| !n.hidden)
        .map(|(id, _)| id.clone())
        .collect();
    if participating.len() < 2 {
        return;
    }

    // Step 1: global ranks via NS.
    let global_ranks = crate::layout::ranking::compute_state_global_ranks(graph);
    if global_ranks.is_empty() {
        return;
    }

    // Step 2: organize into layers, ordering nodes within each rank by
    // their existing X (preserves the per-cluster X order which is mostly
    // correct).
    let max_rank = *global_ranks.values().max().unwrap_or(&0);
    let min_rank = *global_ranks.values().min().unwrap_or(&0);
    let mut layers: Vec<Vec<String>> = vec![Vec::new(); max_rank - min_rank + 1];
    for (id, &r) in &global_ranks {
        if !nodes.contains_key(id) {
            continue;
        }
        let Some(node) = nodes.get(id) else {
            continue;
        };
        if node.hidden {
            continue;
        }
        layers[r - min_rank].push(id.clone());
    }
    // Sort each layer by current X (keeps existing column order).
    for layer in &mut layers {
        layer.sort_by(|a, b| {
            let xa = nodes.get(a).map(|n| n.x).unwrap_or(0.0);
            let xb = nodes.get(b).map(|n| n.x).unwrap_or(0.0);
            xa.partial_cmp(&xb).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    // Step 3: build LayeredGraph for BK.
    let mut widths: HashMap<String, f32> = HashMap::new();
    let mut max_h_per_rank: HashMap<usize, f32> = HashMap::new();
    for (id, &r) in &global_ranks {
        let Some(node) = nodes.get(id) else {
            continue;
        };
        if node.hidden {
            continue;
        }
        widths.insert(id.clone(), node.width);
        let h = max_h_per_rank.entry(r).or_insert(0.0);
        *h = h.max(node.height);
    }
    let mut bk_edges: Vec<(String, String)> = Vec::new();
    for e in &graph.edges {
        if let (Some(&rf), Some(&rt)) = (global_ranks.get(&e.from), global_ranks.get(&e.to)) {
            if rf < rt {
                bk_edges.push((e.from.clone(), e.to.clone()));
            } else if rt < rf {
                bk_edges.push((e.to.clone(), e.from.clone()));
            }
        }
    }
    let virtual_nodes: HashSet<String> = HashSet::new(); // no virtual nodes inserted yet
    let bk_graph = brandes_kopf::LayeredGraph {
        layers: layers.clone(),
        widths: widths.clone(),
        virtual_nodes,
        edges: bk_edges,
    };

    // Step 4: compute X via BK.
    let bk_x = brandes_kopf::compute_x_coordinates(&bk_graph, node_spacing);

    // Step 5: compute Y via cumulative rank height.
    let mut rank_top_y: HashMap<usize, f32> = HashMap::new();
    // Anchor at the topmost existing node Y to preserve outer top edge.
    let mut anchor_y = f32::MAX;
    for n in nodes.values() {
        if !n.hidden {
            anchor_y = anchor_y.min(n.y);
        }
    }
    if anchor_y == f32::MAX {
        anchor_y = 0.0;
    }
    let mut cur_y = anchor_y;
    for r in min_rank..=max_rank {
        rank_top_y.insert(r, cur_y);
        let h = max_h_per_rank.get(&r).copied().unwrap_or(0.0);
        if h > 0.0 {
            cur_y += h + rank_spacing;
        }
    }

    // Step 6: compute the X centering offset. BK's X coords have arbitrary
    // origin (smallest X = 0 typically). Offset so the X range is centered
    // on the existing X centroid (preserves outer left edge).
    let bk_min_x = bk_x.values().cloned().fold(f32::INFINITY, f32::min);
    let mut existing_min_x = f32::MAX;
    for n in nodes.values() {
        if !n.hidden {
            existing_min_x = existing_min_x.min(n.x);
        }
    }
    let x_offset = if bk_min_x.is_finite() && existing_min_x.is_finite() {
        existing_min_x - bk_min_x
    } else {
        0.0
    };

    // Step 7: apply new positions. Use the BK x and rank-derived y, but
    // PRESERVE original positions when the node doesn't participate in the
    // global rank graph (orphans, special nodes).
    for (id, node) in nodes.iter_mut() {
        if node.hidden {
            continue;
        }
        if let (Some(&r), Some(&new_x)) = (global_ranks.get(id), bk_x.get(id)) {
            let slot_top = rank_top_y.get(&r).copied().unwrap_or(node.y);
            let slot_h = max_h_per_rank.get(&r).copied().unwrap_or(node.height);
            let centered_y = slot_top + (slot_h - node.height) / 2.0;
            node.x = new_x + x_offset;
            node.y = centered_y;
        }
    }
}

/// Compound-state rank reconciliation for the Mermaid state renderer's
/// cluster-edge behavior.
///
/// Mermaid's dagre wrapper rewrites edges that touch cluster nodes to a
/// representative child before rank assignment, while the cluster bbox still
/// spans all descendants. That lets a final marker beside a composite sit near
/// the cluster entry rank even when the cluster's own children extend much
/// lower. The generic mmdr state path lays each composite independently and
/// then routes against finished bboxes, so shared-node cross-cluster diagrams
/// can collapse into the wrong vertical topology.
///
/// This pass is intentionally gated to the high-risk compound pattern: nested
/// state diagrams with inner-to-inner cross-top-level edges. Plain composite
/// diagrams, concurrency regions, and simple state diagrams continue to use the
/// existing tuned path.
pub(super) fn apply_state_compound_dagre_layout(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::State
        || !matches!(graph.direction, Direction::TopDown | Direction::BottomTop)
        || graph.subgraphs.len() < 2
    {
        return;
    }

    let tree = CompoundTree::build(graph);
    if tree.top_level.len() < 2 || has_region_subgraph(graph) {
        return;
    }

    let node_top = node_top_membership(graph, &tree);
    if !has_inner_cross_top_edge(graph, &node_top) {
        return;
    }

    let sub_by_id = subgraph_lookup(graph);
    let mut touched = false;

    let mut incoming_by_to: HashMap<&str, Vec<&crate::ir::Edge>> = HashMap::new();
    let mut outgoing_by_from: HashMap<&str, Vec<&crate::ir::Edge>> = HashMap::new();
    for edge in &graph.edges {
        incoming_by_to
            .entry(edge.to.as_str())
            .or_default()
            .push(edge);
        outgoing_by_from
            .entry(edge.from.as_str())
            .or_default()
            .push(edge);
    }

    for (target_id, incoming) in incoming_by_to {
        let start_markers: Vec<&str> = incoming
            .iter()
            .filter_map(|edge| {
                let from = edge.from.as_str();
                if is_start_marker(from) && nodes.contains_key(from) {
                    Some(from)
                } else {
                    None
                }
            })
            .collect();
        if start_markers.len() < 2 || !nodes.contains_key(target_id) {
            continue;
        }

        let mut marker_tops: HashSet<usize> = HashSet::new();
        for marker in &start_markers {
            if let Some(top) = node_top.get(*marker) {
                marker_tops.insert(*top);
            }
        }
        if marker_tops.len() < 2 {
            continue;
        }

        let Some(reference_center) = start_markers
            .iter()
            .filter_map(|id| nodes.get(*id).map(center_y))
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        else {
            continue;
        };

        for marker in &start_markers {
            set_center_y(nodes, marker, reference_center);
        }

        let Some(first_marker) = start_markers.first().copied() else {
            continue;
        };
        let target_center = reference_center + center_gap(nodes, first_marker, target_id, config);
        set_center_y(nodes, target_id, target_center);
        touched = true;

        let mut deep_regular_center: Option<f32> = None;
        if let Some(outgoing) = outgoing_by_from.get(target_id) {
            for edge in outgoing {
                let Some(&target_sub_idx) = sub_by_id.get(edge.to.as_str()) else {
                    continue;
                };
                let Some(entry_id) = start_marker_for_subgraph(graph, target_sub_idx) else {
                    continue;
                };
                if !nodes.contains_key(entry_id.as_str()) {
                    continue;
                }

                let depth = tree.depth(target_sub_idx);
                let entry_gap = center_gap(nodes, target_id, &entry_id, config)
                    + 25.0 * depth.saturating_sub(1) as f32
                    + dagre_rank_spacing(config) * 0.25;
                let entry_center = target_center + entry_gap;
                let delta = entry_center - center_y(nodes.get(&entry_id).unwrap());
                shift_subgraph_descendants(graph, &tree, target_sub_idx, nodes, 0.0, delta);

                if let Some(mid) =
                    stretch_linear_leaf_cluster(graph, &tree, target_sub_idx, nodes, config)
                {
                    deep_regular_center = Some(mid);
                }

                place_parent_end_after_cluster(graph, &tree, target_sub_idx, nodes, config);
            }
        }

        if let Some(&target_top) = node_top.get(target_id) {
            if let Some(top_end_id) = end_marker_for_subgraph(graph, target_top) {
                if nodes.contains_key(top_end_id.as_str()) {
                    let center = deep_regular_center.unwrap_or_else(|| {
                        target_center + config.rank_spacing + node_height(nodes, target_id)
                    });
                    set_center_y(nodes, &top_end_id, center);
                    align_root_end_with_cluster_exit(graph, nodes, target_top, center, config);
                }
            }
        }
    }

    if !touched {
        return;
    }

    // For cluster-to-parent-final transitions, Mermaid's cluster-edge rewrite
    // uses a child entry anchor for the source cluster when the edge is part of
    // the cross-top shared-node pattern. Keep the parent final marker beside
    // that entry rank rather than beneath the whole nested cluster.
    for edge in &graph.edges {
        let Some(&source_sub_idx) = sub_by_id.get(edge.from.as_str()) else {
            continue;
        };
        if !is_end_marker(edge.to.as_str()) || !nodes.contains_key(edge.to.as_str()) {
            continue;
        }
        let Some(entry_id) = start_marker_for_subgraph(graph, source_sub_idx) else {
            continue;
        };
        if !nodes.contains_key(entry_id.as_str())
            || !source_has_cross_top_entry(graph, &node_top, &entry_id)
        {
            continue;
        }
        let target_center = center_y(nodes.get(&entry_id).unwrap())
            + center_gap(nodes, &entry_id, edge.to.as_str(), config)
            + dagre_rank_spacing(config) * 0.4;
        set_center_y(nodes, edge.to.as_str(), target_center);

        let max_x = descendant_bounds(graph, &tree, source_sub_idx, nodes)
            .map(|(_, _, max_x, _)| max_x)
            .unwrap_or_else(|| nodes.get(&entry_id).map(|n| n.x + n.width).unwrap_or(0.0));
        if let Some(end) = nodes.get_mut(edge.to.as_str()) {
            end.x = max_x + dagre_node_spacing(config) * 1.05 - end.width * 0.5;
        }
    }
}

fn has_region_subgraph(graph: &Graph) -> bool {
    graph.subgraphs.iter().any(|sub| {
        sub.label.trim().is_empty()
            && sub
                .id
                .as_deref()
                .map(|id| id.starts_with("__region_"))
                .unwrap_or(false)
    })
}

fn is_start_marker(id: &str) -> bool {
    id.starts_with("__start_") && id.ends_with("__")
}

fn is_end_marker(id: &str) -> bool {
    id.starts_with("__end_") && id.ends_with("__")
}

fn start_marker_for_subgraph(graph: &Graph, sub_idx: usize) -> Option<String> {
    let name = subgraph_name(graph, sub_idx)?;
    let id = format!("__start_{name}__");
    Some(id)
}

fn end_marker_for_subgraph(graph: &Graph, sub_idx: usize) -> Option<String> {
    let name = subgraph_name(graph, sub_idx)?;
    Some(format!("__end_{name}__"))
}

fn subgraph_name(graph: &Graph, sub_idx: usize) -> Option<&str> {
    let sub = graph.subgraphs.get(sub_idx)?;
    sub.id
        .as_deref()
        .filter(|id| !id.is_empty())
        .or_else(|| Some(sub.label.as_str()).filter(|label| !label.is_empty()))
}

fn subgraph_lookup(graph: &Graph) -> HashMap<&str, usize> {
    let mut out = HashMap::new();
    for (idx, sub) in graph.subgraphs.iter().enumerate() {
        if let Some(id) = sub.id.as_deref().filter(|id| !id.is_empty()) {
            out.insert(id, idx);
        }
        if !sub.label.is_empty() {
            out.insert(sub.label.as_str(), idx);
        }
    }
    out
}

fn node_top_membership(graph: &Graph, tree: &CompoundTree) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for &top in &tree.top_level {
        collect_node_top(graph, tree, top, top, &mut out);
    }
    out
}

fn collect_node_top(
    graph: &Graph,
    tree: &CompoundTree,
    sub_idx: usize,
    top_idx: usize,
    out: &mut HashMap<String, usize>,
) {
    if let Some(sub) = graph.subgraphs.get(sub_idx) {
        for node_id in &sub.nodes {
            out.entry(node_id.clone()).or_insert(top_idx);
        }
    }
    for &child in &tree.children[sub_idx] {
        collect_node_top(graph, tree, child, top_idx, out);
    }
}

fn has_inner_cross_top_edge(graph: &Graph, node_top: &HashMap<String, usize>) -> bool {
    for edge in &graph.edges {
        let (Some(a), Some(b)) = (node_top.get(&edge.from), node_top.get(&edge.to)) else {
            continue;
        };
        if a == b {
            continue;
        }
        if is_start_marker(&edge.from) || is_end_marker(&edge.from) {
            // A start marker that targets a real state in another top-level
            // cluster is exactly the shared-node bridge Mermaid handles with
            // compound ranking.
            if !is_start_marker(&edge.to) && !is_end_marker(&edge.to) {
                return true;
            }
        } else if !is_start_marker(&edge.to) && !is_end_marker(&edge.to) {
            return true;
        }
    }
    false
}

fn source_has_cross_top_entry(
    graph: &Graph,
    node_top: &HashMap<String, usize>,
    entry_id: &str,
) -> bool {
    let Some(entry_top) = node_top.get(entry_id) else {
        return false;
    };
    graph.edges.iter().any(|edge| {
        edge.from == entry_id
            && node_top
                .get(&edge.to)
                .map(|target_top| target_top != entry_top)
                .unwrap_or(false)
    })
}

fn stretch_linear_leaf_cluster(
    graph: &Graph,
    tree: &CompoundTree,
    sub_idx: usize,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) -> Option<f32> {
    if !tree.children[sub_idx].is_empty() {
        return None;
    }
    let entry_id = start_marker_for_subgraph(graph, sub_idx)?;
    let end_id = end_marker_for_subgraph(graph, sub_idx)?;
    if !nodes.contains_key(&entry_id) || !nodes.contains_key(&end_id) {
        return None;
    }
    let sub = graph.subgraphs.get(sub_idx)?;
    let middle_id = sub.nodes.iter().find(|id| {
        id.as_str() != entry_id
            && id.as_str() != end_id
            && !is_start_marker(id)
            && !is_end_marker(id)
            && nodes.contains_key(id.as_str())
    })?;

    let entry_center = center_y(nodes.get(&entry_id).unwrap());
    let middle_gap =
        center_gap(nodes, &entry_id, middle_id, config) + dagre_rank_spacing(config) * 0.5;
    let middle_center = entry_center + middle_gap;
    set_center_y(nodes, middle_id, middle_center);
    let end_gap = center_gap(nodes, middle_id, &end_id, config) + dagre_rank_spacing(config) * 0.5;
    set_center_y(nodes, &end_id, middle_center + end_gap);
    Some(middle_center)
}

fn place_parent_end_after_cluster(
    graph: &Graph,
    tree: &CompoundTree,
    sub_idx: usize,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    let Some(cluster_name) = subgraph_name(graph, sub_idx) else {
        return;
    };
    let Some(cluster_end_id) = end_marker_for_subgraph(graph, sub_idx) else {
        return;
    };
    let Some(cluster_end_center) = nodes.get(&cluster_end_id).map(center_y) else {
        return;
    };
    let depth = tree.depth(sub_idx);
    for edge in &graph.edges {
        if edge.from != cluster_name || !is_end_marker(&edge.to) || !nodes.contains_key(&edge.to) {
            continue;
        }
        if source_has_cross_top_entry(
            graph,
            &node_top_membership(graph, tree),
            &start_marker_for_subgraph(graph, sub_idx).unwrap_or_default(),
        ) {
            continue;
        }
        let gap = center_gap(nodes, &cluster_end_id, &edge.to, config)
            + 25.0 * depth as f32
            + dagre_rank_spacing(config) * 0.25;
        set_center_y(nodes, &edge.to, cluster_end_center + gap);
    }
}

fn align_root_end_with_cluster_exit(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    top_idx: usize,
    center: f32,
    config: &LayoutConfig,
) {
    let Some(top_name) = subgraph_name(graph, top_idx) else {
        return;
    };
    let exits_to_root = graph
        .edges
        .iter()
        .any(|edge| edge.from == top_name && edge.to == "__end_root__");
    if !exits_to_root || !nodes.contains_key("__end_root__") {
        return;
    }
    set_center_y(nodes, "__end_root__", center);
    let mut min_x = f32::MAX;
    for node_id in &graph.subgraphs[top_idx].nodes {
        if let Some(node) = nodes.get(node_id) {
            min_x = min_x.min(node.x);
        }
    }
    if min_x == f32::MAX {
        return;
    }
    if let Some(root_end) = nodes.get_mut("__end_root__") {
        root_end.x = min_x - dagre_node_spacing(config) * 1.45 - root_end.width * 0.5;
    }
}

fn shift_subgraph_descendants(
    graph: &Graph,
    tree: &CompoundTree,
    sub_idx: usize,
    nodes: &mut BTreeMap<String, NodeLayout>,
    dx: f32,
    dy: f32,
) {
    if dx.abs() < 0.01 && dy.abs() < 0.01 {
        return;
    }
    let mut ids = HashSet::new();
    collect_descendant_node_ids(graph, tree, sub_idx, &mut ids);
    for id in ids {
        if let Some(node) = nodes.get_mut(&id) {
            node.x += dx;
            node.y += dy;
        }
    }
}

fn collect_descendant_node_ids(
    graph: &Graph,
    tree: &CompoundTree,
    sub_idx: usize,
    out: &mut HashSet<String>,
) {
    if let Some(sub) = graph.subgraphs.get(sub_idx) {
        for id in &sub.nodes {
            out.insert(id.clone());
        }
    }
    for &child in &tree.children[sub_idx] {
        collect_descendant_node_ids(graph, tree, child, out);
    }
}

fn descendant_bounds(
    graph: &Graph,
    tree: &CompoundTree,
    sub_idx: usize,
    nodes: &BTreeMap<String, NodeLayout>,
) -> Option<(f32, f32, f32, f32)> {
    let mut ids = HashSet::new();
    collect_descendant_node_ids(graph, tree, sub_idx, &mut ids);
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for id in ids {
        let Some(node) = nodes.get(&id) else {
            continue;
        };
        min_x = min_x.min(node.x);
        min_y = min_y.min(node.y);
        max_x = max_x.max(node.x + node.width);
        max_y = max_y.max(node.y + node.height);
    }
    if min_x == f32::MAX {
        None
    } else {
        Some((min_x, min_y, max_x, max_y))
    }
}

fn center_y(node: &NodeLayout) -> f32 {
    node.y + node.height * 0.5
}

fn node_height(nodes: &BTreeMap<String, NodeLayout>, id: &str) -> f32 {
    nodes.get(id).map(|n| n.height).unwrap_or(0.0)
}

fn set_center_y(nodes: &mut BTreeMap<String, NodeLayout>, id: &str, center: f32) {
    if let Some(node) = nodes.get_mut(id) {
        node.y = center - node.height * 0.5;
    }
}

fn center_gap(
    nodes: &BTreeMap<String, NodeLayout>,
    from: &str,
    to: &str,
    config: &LayoutConfig,
) -> f32 {
    let from_h = nodes.get(from).map(|n| n.height).unwrap_or(0.0);
    let to_h = nodes.get(to).map(|n| n.height).unwrap_or(0.0);
    dagre_rank_spacing(config) + (from_h + to_h) * 0.5
}

fn dagre_rank_spacing(config: &LayoutConfig) -> f32 {
    config.rank_spacing.max(50.0)
}

fn dagre_node_spacing(config: &LayoutConfig) -> f32 {
    config.node_spacing.max(50.0)
}

#[derive(Debug)]
struct CompoundTree {
    parent: Vec<Option<usize>>,
    children: Vec<Vec<usize>>,
    top_level: Vec<usize>,
}

impl CompoundTree {
    fn build(graph: &Graph) -> Self {
        let n = graph.subgraphs.len();
        let sets: Vec<HashSet<String>> = graph
            .subgraphs
            .iter()
            .map(|sub| sub.nodes.iter().cloned().collect())
            .collect();
        let mut by_size: Vec<usize> = (0..n).collect();
        by_size.sort_by_key(|&i| sets[i].len());

        let mut parent = vec![None; n];
        let mut children = vec![Vec::new(); n];
        for (pos, &i) in by_size.iter().enumerate() {
            for &j in &by_size[pos + 1..] {
                if sets[j].len() > sets[i].len() && sets[i].is_subset(&sets[j]) {
                    parent[i] = Some(j);
                    children[j].push(i);
                    break;
                }
            }
        }
        let top_level = (0..n).filter(|&i| parent[i].is_none()).collect();
        Self {
            parent,
            children,
            top_level,
        }
    }

    fn depth(&self, mut idx: usize) -> usize {
        let mut depth = 0;
        while let Some(parent) = self.parent.get(idx).copied().flatten() {
            depth += 1;
            idx = parent;
        }
        depth
    }
}
