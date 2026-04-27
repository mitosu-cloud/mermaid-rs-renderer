//! Network simplex rank assignment, as in Gansner et al. 1993,
//! "A Technique for Drawing Directed Graphs", which is the algorithm
//! dagre uses (and which mermaid uses via its dagre fork).
//!
//! The algorithm:
//!   1. **init_rank**: longest-path initial ranking (for each edge u→v with
//!      minlen m, rank[v] >= rank[u] + m).
//!   2. **feasible_tree**: build a tight spanning tree (all tree edges have
//!      slack = 0). If non-spanning, slacken the smallest non-tree edge and
//!      shift ranks to make it tight.
//!   3. **iterate**: while a tree edge has negative cut value, swap it with
//!      a non-tree edge that minimizes the cycle slack. Recompute cut values.
//!   4. **normalize**: shift to make minimum rank = 0.
//!
//! This produces the rank assignment that minimizes total weighted edge
//! length subject to the minlen constraints — exactly what JS dagre does.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::ir::Edge;

/// Compute ranks for `node_ids` using all `edges` whose endpoints are both
/// in the set. Edge minlen defaults to 1. Returns a rank for every node in
/// the set; ranks start at 0 (after normalization).
pub(super) fn compute_ranks_network_simplex(
    node_ids: &[String],
    edges: &[Edge],
    node_order: &HashMap<String, usize>,
) -> HashMap<String, usize> {
    let set: HashSet<&str> = node_ids.iter().map(String::as_str).collect();
    if set.is_empty() {
        return HashMap::new();
    }

    // Build directed adjacency restricted to the subset.
    let mut g_edges: Vec<(String, String, i32, i32)> = Vec::new(); // (from, to, minlen, weight)
    for e in edges {
        if set.contains(e.from.as_str()) && set.contains(e.to.as_str()) {
            g_edges.push((e.from.clone(), e.to.clone(), 1, 1));
        }
    }

    let mut ranks = init_rank(node_ids, &g_edges, node_order);

    // Iterate network simplex until all cut values are non-negative or we
    // hit a safety bound. dagre uses 8 * |V| as the iteration cap.
    let max_iter = (node_ids.len() * 8).max(64);
    for _ in 0..max_iter {
        let tree = feasible_tree(node_ids, &g_edges, &mut ranks);
        let cut_values = compute_cut_values(&tree, &g_edges);
        let leave = find_negative_cut(&tree, &cut_values);
        let Some(leave_edge_idx) = leave else {
            break; // optimal
        };
        let Some((enter_edge_idx, delta)) =
            find_enter_edge(&tree, &g_edges, &ranks, leave_edge_idx)
        else {
            break; // can't improve
        };
        exchange(
            &mut ranks,
            &tree,
            leave_edge_idx,
            enter_edge_idx,
            delta,
            &g_edges,
        );
    }

    normalize(&mut ranks, node_ids)
}

/// Phase 1: Longest-path initial ranking. For each node v: rank[v] = max
/// over all predecessors u of (rank[u] + minlen(u,v)). Nodes with no
/// predecessors get rank 0.
fn init_rank(
    node_ids: &[String],
    edges: &[(String, String, i32, i32)],
    node_order: &HashMap<String, usize>,
) -> HashMap<String, i32> {
    // Build adjacency
    let mut adj: HashMap<String, Vec<(String, i32)>> = HashMap::new();
    let mut rev: HashMap<String, Vec<(String, i32)>> = HashMap::new();
    let mut indeg: HashMap<String, usize> = HashMap::new();
    for id in node_ids {
        indeg.insert(id.clone(), 0);
    }
    for (from, to, minlen, _w) in edges {
        adj.entry(from.clone())
            .or_default()
            .push((to.clone(), *minlen));
        rev.entry(to.clone())
            .or_default()
            .push((from.clone(), *minlen));
        *indeg.entry(to.clone()).or_default() += 1;
    }

    // Kahn's topological sort with deterministic ordering by node_order.
    let order_key = |id: &str| -> usize { node_order.get(id).copied().unwrap_or(usize::MAX) };

    let mut ready: Vec<String> = node_ids
        .iter()
        .filter(|id| *indeg.get(id.as_str()).unwrap_or(&0) == 0)
        .cloned()
        .collect();
    ready.sort_by_key(|id| (order_key(id), id.clone()));
    let mut ready: VecDeque<String> = ready.into();

    let mut topo: Vec<String> = Vec::with_capacity(node_ids.len());
    let mut processed: HashSet<String> = HashSet::new();
    while let Some(id) = ready.pop_front() {
        if !processed.insert(id.clone()) {
            continue;
        }
        topo.push(id.clone());
        if let Some(succs) = adj.get(&id) {
            // Sort successors deterministically.
            let mut sorted_succs: Vec<&(String, i32)> = succs.iter().collect();
            sorted_succs.sort_by_key(|(s, _)| (order_key(s), s.clone()));
            for (s, _) in sorted_succs {
                if processed.contains(s) {
                    continue;
                }
                if let Some(d) = indeg.get_mut(s) {
                    *d = d.saturating_sub(1);
                    if *d == 0 {
                        ready.push_back(s.clone());
                    }
                }
            }
        }
    }
    // Cycle handling: any unprocessed nodes get appended in node_order.
    if topo.len() < node_ids.len() {
        let mut rest: Vec<String> = node_ids
            .iter()
            .filter(|id| !processed.contains(id.as_str()))
            .cloned()
            .collect();
        rest.sort_by_key(|id| (order_key(id), id.clone()));
        topo.extend(rest);
    }

    let mut ranks: HashMap<String, i32> = HashMap::new();
    for id in &topo {
        let mut r: i32 = 0;
        if let Some(preds) = rev.get(id) {
            for (p, ml) in preds {
                if let Some(&pr) = ranks.get(p) {
                    r = r.max(pr + ml);
                }
            }
        }
        ranks.insert(id.clone(), r);
    }
    ranks
}

/// Spanning tree representation for network simplex.
#[derive(Debug, Clone)]
struct Tree {
    /// Indices into the original edges array that are tree edges.
    edge_indices: Vec<usize>,
    /// For each node, lim/low DFS numbering used to identify head component.
    lim: HashMap<String, usize>,
    low: HashMap<String, usize>,
    /// Parent in the tree for each node (None for the root).
    parent_edge: HashMap<String, Option<usize>>,
    /// Adjacency of tree edges per node: (other_node, edge_idx).
    tree_adj: HashMap<String, Vec<(String, usize)>>,
}

/// Compute slack = rank[to] - rank[from] - minlen for a directed edge.
fn slack(edges: &[(String, String, i32, i32)], idx: usize, ranks: &HashMap<String, i32>) -> i32 {
    let (from, to, minlen, _) = &edges[idx];
    let rf = *ranks.get(from).unwrap_or(&0);
    let rt = *ranks.get(to).unwrap_or(&0);
    rt - rf - minlen
}

/// Phase 2: Build a tight spanning tree. Tight = slack 0. If we can't span
/// all nodes with slack-0 edges, find the smallest-slack edge crossing the
/// component boundary and shift ranks to make it tight, then continue.
fn feasible_tree(
    node_ids: &[String],
    edges: &[(String, String, i32, i32)],
    ranks: &mut HashMap<String, i32>,
) -> Tree {
    let n = node_ids.len();
    let mut tree_edges: Vec<usize> = Vec::new();
    let mut in_tree: HashSet<String> = HashSet::new();

    // Seed with the first node (lowest order). dagre starts from any node;
    // we pick the first deterministically.
    if let Some(seed) = node_ids.first() {
        in_tree.insert(seed.clone());
    }

    while in_tree.len() < n {
        // Find tight edges that cross the boundary (one endpoint in tree, other not).
        let mut grew = false;
        for (i, (from, to, _, _)) in edges.iter().enumerate() {
            let f_in = in_tree.contains(from);
            let t_in = in_tree.contains(to);
            if f_in == t_in {
                continue;
            }
            if slack(edges, i, ranks) == 0 {
                tree_edges.push(i);
                in_tree.insert(from.clone());
                in_tree.insert(to.clone());
                grew = true;
                break;
            }
        }
        if grew {
            continue;
        }
        // No tight crossing edges. Find min-slack crossing edge and shift.
        let mut min_slack: Option<(usize, i32, bool)> = None; // (idx, slack, head_in_tree)
        for (i, (from, to, _, _)) in edges.iter().enumerate() {
            let f_in = in_tree.contains(from);
            let t_in = in_tree.contains(to);
            if f_in == t_in {
                continue;
            }
            let s = slack(edges, i, ranks);
            if s < 0 {
                continue; // shouldn't happen if init_rank is correct, but skip
            }
            let head_in = t_in;
            if min_slack.map(|(_, m, _)| s < m).unwrap_or(true) {
                min_slack = Some((i, s, head_in));
            }
        }
        let Some((_idx, delta, head_in)) = min_slack else {
            // Disconnected — no edges between in_tree and outside.
            // Pick any out-of-tree node and add it (no edge, just floating).
            for id in node_ids {
                if !in_tree.contains(id) {
                    in_tree.insert(id.clone());
                    break;
                }
            }
            continue;
        };
        // Shift in-tree nodes by ±delta to make this edge tight.
        let shift = if head_in { -delta } else { delta };
        for id in &in_tree {
            if let Some(r) = ranks.get_mut(id) {
                *r += shift;
            }
        }
        // Loop back; the shifted edge now has slack 0.
    }

    // Build tree_adj and lim/low DFS numbering rooted at first node.
    let mut tree_adj: HashMap<String, Vec<(String, usize)>> = HashMap::new();
    for &idx in &tree_edges {
        let (from, to, _, _) = &edges[idx];
        tree_adj
            .entry(from.clone())
            .or_default()
            .push((to.clone(), idx));
        tree_adj
            .entry(to.clone())
            .or_default()
            .push((from.clone(), idx));
    }
    let mut lim: HashMap<String, usize> = HashMap::new();
    let mut low: HashMap<String, usize> = HashMap::new();
    let mut parent_edge: HashMap<String, Option<usize>> = HashMap::new();
    if let Some(root) = node_ids.first() {
        parent_edge.insert(root.clone(), None);
        let mut counter = 0usize;
        dfs_lim_low(
            root,
            None,
            &tree_adj,
            &mut lim,
            &mut low,
            &mut parent_edge,
            &mut counter,
        );
    }

    Tree {
        edge_indices: tree_edges,
        lim,
        low,
        parent_edge,
        tree_adj,
    }
}

fn dfs_lim_low(
    node: &str,
    parent: Option<&str>,
    tree_adj: &HashMap<String, Vec<(String, usize)>>,
    lim: &mut HashMap<String, usize>,
    low: &mut HashMap<String, usize>,
    parent_edge: &mut HashMap<String, Option<usize>>,
    counter: &mut usize,
) {
    *counter += 1;
    let lo = *counter;
    low.insert(node.to_string(), lo);
    if let Some(neighbors) = tree_adj.get(node) {
        for (nbr, edge_idx) in neighbors {
            if Some(nbr.as_str()) == parent {
                continue;
            }
            parent_edge.insert(nbr.clone(), Some(*edge_idx));
            dfs_lim_low(nbr, Some(node), tree_adj, lim, low, parent_edge, counter);
        }
    }
    *counter += 1;
    let li = *counter;
    lim.insert(node.to_string(), li);
}

/// Phase 3a: Compute cut values for each tree edge.
/// Cut value of tree edge e = (sum of weights of non-tree edges going from
/// tail-component to head-component) - (sum of weights of non-tree edges
/// going the opposite direction). Tree edge e itself is included with sign
/// matching its own direction.
fn compute_cut_values(tree: &Tree, edges: &[(String, String, i32, i32)]) -> HashMap<usize, i32> {
    let mut cut: HashMap<usize, i32> = HashMap::new();
    let tree_edges: HashSet<usize> = tree.edge_indices.iter().copied().collect();

    for &t_idx in &tree.edge_indices {
        let (t_from, t_to, _, t_w) = &edges[t_idx];
        // Determine tail/head components by removing edge t and checking lim/low.
        // Convention: head component contains the node with smaller lim
        // (the deeper subtree).
        let (head_node, _tail_node) = head_tail_for_tree_edge(tree, t_from, t_to);
        let head_lim = *tree.lim.get(&head_node).unwrap_or(&0);
        let head_low = *tree.low.get(&head_node).unwrap_or(&0);
        let in_head = |node: &str| -> bool {
            let l = *tree.lim.get(node).unwrap_or(&0);
            head_low <= l && l <= head_lim
        };

        let mut sum: i32 = 0;
        for (i, (from, to, _, w)) in edges.iter().enumerate() {
            if tree_edges.contains(&i) && i != t_idx {
                continue;
            }
            let from_h = in_head(from);
            let to_h = in_head(to);
            if from_h == to_h {
                continue;
            }
            // Tree edge t goes from tail to head if t_to == head_node.
            // Non-tree edge contributes positively if it goes the same way.
            let t_dir_from_tail_to_head = *t_to == head_node;
            let e_dir_from_tail_to_head = !from_h && to_h;
            if t_dir_from_tail_to_head == e_dir_from_tail_to_head {
                sum += w;
            } else {
                sum -= w;
            }
        }
        cut.insert(t_idx, sum);
    }
    cut
}

fn head_tail_for_tree_edge(tree: &Tree, from: &str, to: &str) -> (String, String) {
    // Head = the node whose subtree is "below" (deeper). It's the one whose
    // parent_edge points to this tree edge.
    let from_lim = *tree.lim.get(from).unwrap_or(&0);
    let to_lim = *tree.lim.get(to).unwrap_or(&0);
    if from_lim < to_lim {
        (from.to_string(), to.to_string())
    } else {
        (to.to_string(), from.to_string())
    }
}

/// Phase 3b: Find a tree edge with negative cut value.
fn find_negative_cut(tree: &Tree, cut_values: &HashMap<usize, i32>) -> Option<usize> {
    let mut best: Option<(usize, i32)> = None;
    for &idx in &tree.edge_indices {
        let cv = cut_values.get(&idx).copied().unwrap_or(0);
        if cv < 0 {
            if best.map(|(_, b)| cv < b).unwrap_or(true) {
                best = Some((idx, cv));
            }
        }
    }
    best.map(|(i, _)| i)
}

/// Phase 3c: Find non-tree edge crossing the cut (in opposite direction)
/// with minimum slack. This is the entering edge.
fn find_enter_edge(
    tree: &Tree,
    edges: &[(String, String, i32, i32)],
    ranks: &HashMap<String, i32>,
    leave_idx: usize,
) -> Option<(usize, i32)> {
    let tree_edges: HashSet<usize> = tree.edge_indices.iter().copied().collect();
    let (from, to, _, _) = &edges[leave_idx];
    let (head_node, _) = head_tail_for_tree_edge(tree, from, to);
    let head_lim = *tree.lim.get(&head_node).unwrap_or(&0);
    let head_low = *tree.low.get(&head_node).unwrap_or(&0);
    let in_head = |node: &str| -> bool {
        let l = *tree.lim.get(node).unwrap_or(&0);
        head_low <= l && l <= head_lim
    };

    // Leave edge direction: from tail to head if to == head_node.
    let leave_dir_tail_to_head = *to == head_node;

    let mut best: Option<(usize, i32)> = None;
    for (i, (e_from, e_to, _, _)) in edges.iter().enumerate() {
        if i == leave_idx || tree_edges.contains(&i) {
            continue;
        }
        let f_h = in_head(e_from);
        let t_h = in_head(e_to);
        if f_h == t_h {
            continue;
        }
        // Want non-tree edges going opposite direction to leave edge:
        // i.e., from head to tail.
        let e_dir_tail_to_head = !f_h && t_h;
        if e_dir_tail_to_head == leave_dir_tail_to_head {
            continue;
        }
        let s = slack(edges, i, ranks);
        if best.map(|(_, b)| s < b).unwrap_or(true) {
            best = Some((i, s));
        }
    }
    best
}

/// Phase 3d: Exchange leave/enter tree edges and update ranks.
fn exchange(
    ranks: &mut HashMap<String, i32>,
    tree: &Tree,
    _leave_idx: usize,
    _enter_idx: usize,
    delta: i32,
    _edges: &[(String, String, i32, i32)],
) {
    // Shift the head component (subtree below the leave edge) by delta to
    // tighten the entering edge. dagre rebuilds the tree afterwards via
    // feasible_tree on the next iteration, so we don't need to manually
    // update tree structure here.
    if delta == 0 {
        return;
    }
    // Find head component nodes via lim/low and shift their ranks.
    let (head_node, _) = {
        let leave_idx = _leave_idx;
        let (f, t, _, _) = &_edges[leave_idx];
        head_tail_for_tree_edge(tree, f, t)
    };
    let head_lim = *tree.lim.get(&head_node).unwrap_or(&0);
    let head_low = *tree.low.get(&head_node).unwrap_or(&0);
    let mut to_shift: Vec<String> = Vec::new();
    for (id, &lim) in &tree.lim {
        if head_low <= lim && lim <= head_lim {
            to_shift.push(id.clone());
        }
    }
    for id in to_shift {
        if let Some(r) = ranks.get_mut(&id) {
            *r += delta;
        }
    }
}

/// Phase 4: Normalize so minimum rank = 0.
fn normalize(ranks: &mut HashMap<String, i32>, node_ids: &[String]) -> HashMap<String, usize> {
    let min_rank = ranks.values().copied().min().unwrap_or(0);
    let mut out: HashMap<String, usize> = HashMap::new();
    for id in node_ids {
        let r = ranks.get(id).copied().unwrap_or(0);
        out.insert(id.clone(), (r - min_rank).max(0) as usize);
    }
    out
}
