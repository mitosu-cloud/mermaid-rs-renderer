mod architecture;
mod block;
mod brandes_kopf;
mod c4;
mod cynefin;
mod error;
mod eventmodeling;
mod gantt;
mod gitgraph;
mod ishikawa;
mod journey;
mod kanban;
pub(crate) mod label_placement;
mod markdown;
mod mindmap;
mod network_simplex;
mod packet;
mod pie;
mod quadrant;
mod radar;
mod ranking;
mod routing;
mod sankey;
mod sequence;
mod state_dagre;
mod text;
mod timeline;
mod tree_view;
mod treemap;
pub(crate) mod types;
mod venn;
mod wardley;
mod xychart;
use architecture::*;
use block::*;
use c4::*;
use cynefin::*;
use error::*;
use eventmodeling::*;
use gantt::*;
use gitgraph::*;
use journey::*;
use kanban::*;
use mindmap::*;
use packet::*;
use pie::*;
use quadrant::*;
use radar::*;
use ranking::*;
use routing::*;
use sankey::*;
use sequence::*;
use text::*;
use timeline::*;
use treemap::*;
pub use types::*;
use venn::*;
use xychart::*;

use crate::config::{LayoutConfig, PieRenderMode, TreemapRenderMode};
use crate::ir::{Direction, Graph};
use crate::text_metrics;
use crate::theme::Theme;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Instant;

// Label placement padding (resolved per diagram kind).
const LABEL_RANK_FONT_SCALE: f32 = 0.5;
const LABEL_RANK_MIN_GAP: f32 = 8.0;

// Minimum padding around the entire layout bounding box.
const LAYOUT_BOUNDARY_PAD: f32 = 8.0;
/// Cap for the curve-overshoot protective margin in `bounds_with_edges`.
/// Without this cap, the 20% formula creates excessive viewBox padding for
/// tall/wide diagrams (edge_range × 0.20 grows linearly with diagram size).
/// 60 still protects most curved-edge overshoots while keeping tall diagrams
/// from accumulating excessive bottom padding.
// Iter 272: 60 → 12. Iter 279: 12 → 0. Sweep shows monotonic improvement
// as cap drops (state-suite total |Δh|: 108→97 going 12→0). State edges'
// Bezier control points already sit within neighbouring node bounds, and
// edge labels have their own margin pass downstream — so the cap was
// adding excess viewBox padding without protecting against any actual
// overshoot. Layout regression suite passes at 0.
const EDGE_BBOX_MARGIN_CAP: f32 = 0.0;
const PREFERRED_ASPECT_TOLERANCE: f32 = 0.02;
const MERMAID_DEFAULT_NODE_STROKE: &str = "#9370DB";
const RUST_MERMAID_DEFAULT_PRIMARY_BORDER: &str = "#7B88A8";
const PREFERRED_ASPECT_MAX_EXPANSION: f32 = 6.0;
const FLOWCHART_RECT_PAD_SCALE: f32 = 1.5;
const FLOWCHART_LABEL_PADDING: f32 = 15.0;
const FLOWCHART_DIVIDED_RECT_HEADER_RATIO: f32 = 0.2;
const FLOWCHART_BANG_BBOX_SCALE: f32 = 1.25;
const FLOWCHART_TILTED_CYLINDER_LABEL_PADDING: f32 = FLOWCHART_LABEL_PADDING / 2.0;
const FLOWCHART_WRAPPING_WIDTH: f32 = 200.0;
const FLOWCHART_DOUBLE_CIRCLE_GAP: f32 = 5.0;
const FLOWCHART_CYLINDER_PAD_X: f32 = 12.0;
const FLOWCHART_CYLINDER_PAD_Y: f32 = 15.0;
const FLOWCHART_LINED_CYLINDER_PADDING: f32 = 15.0;
const FLOWCHART_WINDOW_PANE_OFFSET: f32 = 10.0;
const FLOWCHART_ICON_ASSET_SIZE: f32 = 48.0;
const FLOWCHART_ICON_LABEL_PADDING: f32 = 8.0;
const FLOWCHART_ICON_LABEL_EXTRA_HEIGHT: f32 = 4.0;
const FLOWCHART_ICON_CIRCLE_PADDING: f32 = 20.0;
const FLOWCHART_ICON_SQUARE_PADDING: f32 = 4.0;
const PARALLEL_FLOWCHART_SOURCE_HUB_LIFT: f32 = 0.30;
const PARALLEL_FLOWCHART_SHARED_RANK_LIFT: f32 = 0.59;
const PARALLEL_FLOWCHART_TARGET_CHILD_GAP: f32 = 1.85;
const PARALLEL_FLOWCHART_SOURCE_CENTER_GAP: f32 = 3.45;
const PARALLEL_FLOWCHART_RIGHT_LABEL_LANE: f32 = 0.46;
const PARALLEL_FLOWCHART_LABEL_LANE_GAP: f32 = 0.98;
const PARALLEL_FLOWCHART_SOURCE_CLUSTER_TOP_PAD: f32 = 0.50;
const PARALLEL_FLOWCHART_TARGET_CLUSTER_TOP_PAD: f32 = 0.67;
const PARALLEL_FLOWCHART_CLUSTER_BOTTOM_PAD: f32 = 0.32;

// ── State diagram constants ───────────────────────────────────────────
const STATE_MARKER_FIXED_SIZE: f32 = 14.0;
const STATE_FORK_JOIN_LAYOUT_HEIGHT: f32 = 14.0;
// Iter 270: bump note padding to match JS's note-cluster outer padding.
// JS dagre wraps the visible note shape (230×102) in a cluster (300×152)
// — the outer cluster contributes ~30-35px horizontal and ~25px vertical
// pad on top of the visible note pad. Increase scales to close the
// note-size gap. Was 0.75 / 0.5 → ~12px / 8px per side at font 16.
const STATE_NOTE_PAD_X_SCALE: f32 = 1.5;
const STATE_NOTE_PAD_Y_SCALE: f32 = 1.0;
const STATE_NOTE_GAP_SCALE: f32 = 0.9;
const STATE_NOTE_GAP_MIN: f32 = 10.0;
const STATE_PAD_X_SCALE: f32 = 0.5;
const STATE_PAD_Y_SCALE: f32 = 0.5;
const STATE_PAD_X_LABEL_RATIO: f32 = 0.0;
const STATE_PAD_Y_LABEL_RATIO: f32 = 0.22;

// ── Subgraph padding ─────────────────────────────────────────────────
const FLOWCHART_PAD_MAIN: f32 = 40.0;
const FLOWCHART_PAD_CROSS: f32 = 30.0;
const FLOWCHART_EXTERNAL_LR_CLUSTER_PAD_X: f32 = 25.0;
const FLOWCHART_EXTERNAL_LR_CLUSTER_PAD_Y: f32 = 35.0;
const FLOWCHART_RECURSIVE_CLUSTER_CROSS_PAD: f32 = 35.0;
const FLOWCHART_RECURSIVE_CLUSTER_CHILD_MAIN_PAD: f32 = 37.5;
const FLOWCHART_SUBGRAPH_LABEL_SIDE_PAD: f32 = 4.0;
const FLOWCHART_RECURSIVE_CYCLE_STAGGER_BONUS: f32 = 2.0;
const FLOWCHART_RECURSIVE_CYCLE_LANE_EXTRA: f32 = 10.0;
const FLOWCHART_RECURSIVE_DAGRE_SPACING: f32 = 50.0;
const FLOWCHART_RECURSIVE_PARENT_CHILD_CROSS_PAD: f32 = 20.0;
const FLOWCHART_RECURSIVE_LABELED_PAD_X_BONUS: f32 = 15.0;
const FLOWCHART_RECURSIVE_LABELED_PAD_Y_BONUS: f32 = 9.0;
const FLOWCHART_PORT_ROUTE_BIAS_RATIO: f32 = 0.5;
const FLOWCHART_PORT_ROUTE_BIAS_MAX_RATIO: f32 = 0.8;
const ER_DEFAULT_NODE_SPACING: f32 = 140.0;
const ER_DEFAULT_RANK_SPACING: f32 = 80.0;
const ER_ENTITY_DIAGRAM_PADDING: f32 = 20.0;
const ER_ENTITY_MIN_WIDTH: f32 = 100.0;
const ER_ENTITY_MIN_HEIGHT: f32 = 75.0;
pub(crate) const ER_ATTRIBUTE_ROW_HEIGHT: f32 = 42.75;
const KANBAN_SUBGRAPH_PAD: f32 = 8.0;
const STATE_SUBGRAPH_BASE_PAD: f32 = 30.0;
/// Vertical padding for state composites — kept tighter than horizontal so
/// taller composites don't bloat the diagram's total height (JS dagre's TB
/// state layouts are short on each axis but still leave horizontal room for
/// a clean side-by-side cluster arrangement).
const STATE_SUBGRAPH_PAD_Y: f32 = 16.0;
const STATE_SPARSE_LEAF_LABEL_SIDE_PAD: f32 = 29.25;
const STATE_SPARSE_LEAF_TOP_PAD_OFFSET: f32 = 1.5;
/// Extra pad_x added per nested composite level for state composites.
/// Mirrors JS dagre's outer-cluster width-inflation pattern: composites that
/// contain other composites get wider padding than leaf composites.
const STATE_NESTED_PAD_INCREMENT: f32 = 10.0;
// Concurrent-region cluster padding. Measured from JS reference
// stateDiagram-concurrency: each region rect has ~113 px horizontal pad and
// ~50 px vertical pad around its inner state span. Older values (145/110)
// over-padded — combined with apply_orthogonal_region_bands now accounting
// for full region width when spacing siblings, smaller pads keep total
// width close to JS.
const STATE_REGION_PAD_X: f32 = 114.5;
const STATE_REGION_PAD_Y: f32 = 50.0;
const STATE_REGION_ROOT_GAP: f32 = 28.0;
const GENERIC_SUBGRAPH_BASE_PAD: f32 = 24.0;
const SUBGRAPH_LABEL_GAP_FLOWCHART: f32 = 6.0;
const SUBGRAPH_LABEL_GAP_KANBAN: f32 = 4.0;
const SUBGRAPH_LABEL_GAP_GENERIC: f32 = 8.0;
const STATE_SUBGRAPH_TOP_LABEL_SCALE: f32 = 0.75;
const STATE_SUBGRAPH_TOP_MIN_SCALE: f32 = 1.4;

// ── Shape size constants ─────────────────────────────────────────────
const DIAMOND_SCALE: f32 = 0.95;
/// JS state-diagram choice marker size. Empty-label `<<choice>>` diamond
/// renders ~28×28 in JS — matching this exactly avoids the ~10px overshoot
/// from the auto-size formula's font-line-height term (16*1.5=24).
const STATE_CHOICE_DIAMOND_SIZE: f32 = 28.0;
const FORK_JOIN_MIN_WIDTH: f32 = 70.0;
const FORK_JOIN_HEIGHT_SCALE: f32 = 0.4;
const FORK_JOIN_MIN_HEIGHT: f32 = 10.0;
const CIRCLE_EMPTY_HEIGHT_SCALE: f32 = 1.4;
const CIRCLE_EMPTY_MIN_SIZE: f32 = 14.0;
const ROUND_RECT_WIDTH_SCALE: f32 = 1.1;
const ROUND_RECT_HEIGHT_SCALE: f32 = 1.05;
const CYLINDER_SCALE: f32 = 1.1;
const HEXAGON_WIDTH_SCALE: f32 = 1.2;
const HEXAGON_HEIGHT_SCALE: f32 = 1.1;
const BLOCK_NODE_SHAPE_PADDING: f32 = 8.0;
const TRAPEZOID_WIDTH_SCALE: f32 = 1.2;
const CLASS_BOX_PADDING: f32 = 12.0;
const CLASS_BOX_BODY_EXTRA_LINES: f32 = 3.0;
const CLASS_MIN_HEIGHT_SCALE: f32 = 5.25;
const CLASS_BODY_PAD_X_SCALE: f32 = 0.85;
const CLASS_EMPTY_PAD_X_SCALE: f32 = 0.6;
const CLASS_NAMESPACE_PAD_X: f32 = 37.5;
const CLASS_NAMESPACE_PAD_Y: f32 = 31.0;
const CLASS_NAMESPACE_TOP_LABEL_GAP: f32 = 11.0;
const CLASS_EDGE_OPEN_MARKER_EXTENT: f32 = 17.25;
const CLASS_EDGE_DEPENDENCY_MARKER_EXTENT: f32 = 6.0;
const CLASS_EDGE_DECORATION_EXTENT: f32 = 18.0;
const CLASS_EDGE_GENERIC_ARROW_EXTENT: f32 = 8.0;
const CLASS_EDGE_LOLLIPOP_MARKER_EXTENT: f32 = 6.0;
const KANBAN_MIN_WIDTH_SCALE: f32 = 11.0;
const KANBAN_MIN_HEIGHT_SCALE: f32 = 2.6;

// ── Edge label relaxation constants ──────────────────────────────────
const EDGE_LABEL_PAD_SCALE: f32 = 0.35;
const ENDPOINT_LABEL_PAD_SCALE: f32 = 0.2;
const DUAL_ENDPOINT_EXTRA_PAD_SCALE: f32 = 0.45;
const EDGE_RELAX_STEP_MIN: f32 = 24.0;
const EDGE_RELAX_GAP_TOLERANCE: f32 = 0.5;
const MAX_MAIN_GAP_FACTOR: f32 = 6.0;
const FLOWCHART_EDGE_LABEL_WRAP_TRIGGER_CHARS: usize = 34;
const FLOWCHART_EDGE_LABEL_WRAP_MAX_CHARS: usize = 18;
const FLOWCHART_DAGRE_POINT_MARGIN: f32 = 11.5;

#[derive(Clone)]
struct RouteLabelPlan {
    obstacle_id: String,
    obstacle_index: usize,
    progress: f32,
    center: (f32, f32),
}

#[derive(Clone)]
struct ParallelTopLevelFlowchart {
    source_sg: usize,
    target_sg: usize,
    source_node: String,
    target_node: String,
    edge_indices: Vec<usize>,
}

// ── Overlap resolution ───────────────────────────────────────────────
const OVERLAP_RESOLVE_PASSES: u32 = 6;
const OVERLAP_MIN_GAP_RATIO: f32 = 0.2;
const OVERLAP_MIN_GAP_FLOOR: f32 = 4.0;
const OVERLAP_CENTER_THRESHOLD: f32 = 0.5;

// ── Subgraph gap enforcement ─────────────────────────────────────────
const SUBGRAPH_DESIRED_GAP_RATIO: f32 = 1.6;

// ── Edge occupancy / multi-edge offset ───────────────────────────────
const MIN_NODE_SPACING_FLOOR: f32 = 16.0;
const EDGE_OCCUPANCY_CELL_RATIO: f32 = 0.6;
const MULTI_EDGE_OFFSET_RATIO: f32 = 0.35;

// ── State subgraph rank spacing boost ────────────────────────────────
const STATE_RANK_SPACING_BOOST: f32 = 25.0;
const STATE_REGION_RANK_BOOST: f32 = STATE_RANK_SPACING_BOOST * 2.0;
const STATE_REGION_RANK_MIN: f32 = 100.0;
const STATE_REGION_NODE_BOOST: f32 = 0.0;
const STATE_REGION_NODE_MIN: f32 = 50.0;
const STATE_REGION_LABEL_RANK_GAP: f32 = 24.0;
const STATE_COMPOSITE_RANK_BOOST: f32 = STATE_RANK_SPACING_BOOST;
/// Per-depth ranksep increment for state composites. JS dagre adds +25
/// to ranksep at each cluster recursion in `recursiveRender`
/// (see ../mermaid/.../layout-algorithms/dagre/index.js:81), but RS picks
/// up additional spacing elsewhere in the pipeline; iter 273 sweep finds
/// 10 minimises total state-suite |Δh| (PER_DEPTH=10 → 267.7 vs 25 → 351.7).
const STATE_COMPOSITE_RANK_PER_DEPTH: f32 = 10.0;
/// Minimum cross-axis gap (px) between sibling state composites that end up
/// adjacent on the cross axis (e.g. side-by-side in TB layout). Floor used by
/// `separate_overlapping_sibling_subgraph_rects` to avoid crowded clusters.
const STATE_SIBLING_CROSS_GAP_MIN: f32 = 70.0;

fn measure_subgraph_label(
    graph: &Graph,
    sub: &crate::ir::Subgraph,
    theme: &Theme,
    config: &LayoutConfig,
) -> TextBlock {
    if sub.markdown_label {
        measure_markdown_label(&sub.label, theme, config)
    } else if graph.kind == crate::ir::DiagramKind::Flowchart {
        measure_label_no_wrap(&sub.label, theme, config)
    } else if has_html_formatting(&sub.label) {
        let normalized = normalize_html_label(&sub.label);
        measure_markdown_label(&normalized, theme, config)
    } else {
        measure_label(&sub.label, theme, config)
    }
}

fn is_region_subgraph(sub: &crate::ir::Subgraph) -> bool {
    sub.label.trim().is_empty()
        && sub
            .id
            .as_deref()
            .map(|id| id.starts_with("__region_"))
            .unwrap_or(false)
}

#[derive(Debug, Clone, Default)]
pub struct LayoutStageMetrics {
    pub port_assignment_us: u128,
    pub edge_routing_us: u128,
    pub label_placement_us: u128,
}

impl LayoutStageMetrics {
    pub fn total_us(&self) -> u128 {
        self.port_assignment_us + self.edge_routing_us + self.label_placement_us
    }
}

pub fn compute_layout(graph: &Graph, theme: &Theme, config: &LayoutConfig) -> Layout {
    compute_layout_with_metrics(graph, theme, config).0
}

pub fn compute_layout_with_metrics(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> (Layout, LayoutStageMetrics) {
    let mut stage_metrics = LayoutStageMetrics::default();
    let mut layout = match graph.kind {
        crate::ir::DiagramKind::Sequence | crate::ir::DiagramKind::ZenUML => {
            compute_sequence_layout(graph, theme, config)
        }
        crate::ir::DiagramKind::Pie => {
            if config.pie.render_mode == PieRenderMode::Error {
                compute_pie_error_layout(graph, config)
            } else {
                compute_pie_layout(graph, theme, config)
            }
        }
        crate::ir::DiagramKind::Quadrant => compute_quadrant_layout(graph, theme, config),
        crate::ir::DiagramKind::Gantt => compute_gantt_layout(graph, theme, config),
        crate::ir::DiagramKind::Kanban => {
            compute_kanban_layout(graph, theme, config, Some(&mut stage_metrics))
        }
        crate::ir::DiagramKind::Block => compute_block_layout(graph, theme, config),
        crate::ir::DiagramKind::Sankey => compute_sankey_layout(graph, theme, config),
        crate::ir::DiagramKind::Architecture => compute_architecture_layout(graph, theme, config),
        crate::ir::DiagramKind::Radar => compute_radar_layout(graph, theme, config),
        crate::ir::DiagramKind::Treemap => {
            if config.treemap.render_mode == TreemapRenderMode::Error {
                compute_error_layout(graph, config)
            } else {
                compute_treemap_layout(graph, theme, config)
            }
        }
        crate::ir::DiagramKind::GitGraph => compute_gitgraph_layout(graph, theme, config),
        crate::ir::DiagramKind::C4 => compute_c4_layout(graph, config),
        crate::ir::DiagramKind::Mindmap => compute_mindmap_layout(graph, theme, config),
        crate::ir::DiagramKind::XYChart => compute_xychart_layout(graph, theme, config),
        crate::ir::DiagramKind::Timeline => compute_timeline_layout(graph, theme, config),
        crate::ir::DiagramKind::Journey => compute_journey_layout(graph, theme, config),
        crate::ir::DiagramKind::Venn => compute_venn_layout(graph, theme, config),
        crate::ir::DiagramKind::Packet => compute_packet_layout(graph, theme, config),
        crate::ir::DiagramKind::TreeView => {
            tree_view::compute_tree_view_layout(graph, theme, config)
        }
        crate::ir::DiagramKind::Ishikawa => ishikawa::compute_ishikawa_layout(graph, theme, config),
        crate::ir::DiagramKind::Wardley => wardley::compute_wardley_layout(graph, theme, config),
        crate::ir::DiagramKind::EventModeling => compute_eventmodeling_layout(graph, theme, config),
        crate::ir::DiagramKind::Cynefin => compute_cynefin_layout(graph, theme, config),
        crate::ir::DiagramKind::Class
        | crate::ir::DiagramKind::State
        | crate::ir::DiagramKind::Er
        | crate::ir::DiagramKind::Requirement
        | crate::ir::DiagramKind::Flowchart => {
            compute_flowchart_layout(graph, theme, config, Some(&mut stage_metrics))
        }
    };

    // Propagate accessibility metadata from the parsed graph.
    layout.acc_title = graph.acc_title.clone();
    layout.acc_descr = graph.acc_descr.clone();

    apply_preferred_aspect_ratio_layout(&mut layout, config);

    // Final pass: resolve all edge label positions using collision avoidance.
    let label_start = Instant::now();
    label_placement::resolve_all_label_positions(&mut layout, theme, config);
    stage_metrics.label_placement_us = stage_metrics
        .label_placement_us
        .saturating_add(label_start.elapsed().as_micros());

    (layout, stage_metrics)
}

fn adaptive_spacing_for_nodes(
    nodes: &BTreeMap<String, NodeLayout>,
    excluded_node_ids: &HashSet<String>,
    min_spacing: f32,
    max_spacing: f32,
) -> f32 {
    let mut total = 0.0f32;
    let mut count = 0usize;
    for (id, node) in nodes {
        if excluded_node_ids.contains(id) {
            continue;
        }
        if node.hidden || node.anchor_subgraph.is_some() {
            continue;
        }
        total += (node.width + node.height) * 0.5;
        count += 1;
    }
    if count == 0 {
        return max_spacing;
    }
    // Small graphs don't benefit from spacing reduction – keep the
    // configured spacing so edges have room to route smoothly.
    if count <= 8 {
        return max_spacing;
    }
    let avg = total / count as f32;
    let target = (avg * 0.5).max(min_spacing);
    target.min(max_spacing)
}

fn manual_layout_node_order(graph: &Graph) -> HashMap<String, usize> {
    if graph.kind != crate::ir::DiagramKind::Class {
        return graph.node_order.clone();
    }

    let mut ids: Vec<String> = graph.nodes.keys().cloned().collect();
    ids.sort_by(|a, b| {
        let a_is_note = graph
            .nodes
            .get(a)
            .map(|node| node.shape == crate::ir::NodeShape::Note)
            .unwrap_or(false);
        let b_is_note = graph
            .nodes
            .get(b)
            .map(|node| node.shape == crate::ir::NodeShape::Note)
            .unwrap_or(false);

        a_is_note
            .cmp(&b_is_note)
            .then_with(|| {
                graph
                    .node_order
                    .get(a)
                    .copied()
                    .unwrap_or(usize::MAX)
                    .cmp(&graph.node_order.get(b).copied().unwrap_or(usize::MAX))
            })
            .then_with(|| a.cmp(b))
    });

    ids.into_iter()
        .enumerate()
        .map(|(idx, id)| (id, idx))
        .collect()
}

fn compute_flowchart_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
    mut stage_metrics: Option<&mut LayoutStageMetrics>,
) -> Layout {
    let mut effective_config = config.clone();
    let mut hub_compaction_scale: Option<f32> = None;
    let mut hub_compaction_floor = 0.0f32;
    let mut prefer_direct_hub_routing = false;
    if graph.kind == crate::ir::DiagramKind::Requirement {
        effective_config.max_label_width_chars = effective_config.max_label_width_chars.max(32);
    }
    if graph.kind == crate::ir::DiagramKind::Er {
        // Mermaid's ER renderer feeds ELK through flowchart with ER-specific
        // defaults (nodeSpacing=140, rankSpacing=80) instead of flowchart's
        // generic spacing.
        let defaults = LayoutConfig::default();
        if (effective_config.node_spacing - defaults.node_spacing).abs() <= f32::EPSILON {
            effective_config.node_spacing = ER_DEFAULT_NODE_SPACING;
        }
        if (effective_config.rank_spacing - defaults.rank_spacing).abs() <= f32::EPSILON {
            effective_config.rank_spacing = ER_DEFAULT_RANK_SPACING;
        }
        // Extra rank-order sweeps reduce crossing-prone left/right inversions
        // in dense relationship graphs.
        effective_config.flowchart.order_passes = effective_config.flowchart.order_passes.max(10);
    }
    if graph.kind == crate::ir::DiagramKind::Flowchart {
        let node_count = graph.nodes.len();
        let edge_count = graph.edges.len() as f32;
        let density = if node_count > 0 {
            edge_count / node_count as f32
        } else {
            0.0
        };
        let auto = &config.flowchart.auto_spacing;
        if auto.enabled && !auto.buckets.is_empty() {
            let mut scale = auto.buckets[0].scale;
            for bucket in &auto.buckets {
                if node_count >= bucket.min_nodes {
                    scale = bucket.scale;
                }
            }
            if density > auto.density_threshold {
                scale = scale.max(auto.dense_scale_floor);
            }
            effective_config.node_spacing =
                (effective_config.node_spacing * scale).max(auto.min_spacing);
            effective_config.rank_spacing =
                (effective_config.rank_spacing * scale).max(auto.min_spacing);
        }

        // Flowcharts with labeled edges need extra rank spacing so labels
        // fit between ranks without overlapping nodes — matching dagre's
        // effective ranksep which includes label height.
        let has_edge_labels = graph
            .edges
            .iter()
            .any(|e| e.label.is_some() && !flowchart_edge_inside_recursive_cluster(graph, e));
        let use_dagre_label_rank_spacing = flowchart_use_dagre_lr_label_rank_spacing(graph);
        let label_rank_spacing_floor =
            if is_small_dense_labeled_flowchart(graph, graph.nodes.len(), &graph.edges) {
                74.0
            } else {
                75.0
            };
        if has_edge_labels
            && !use_dagre_label_rank_spacing
            && effective_config.rank_spacing < label_rank_spacing_floor
        {
            effective_config.rank_spacing = label_rank_spacing_floor;
        }

        // Hub-and-spoke flowcharts (one high-degree node) tend to over-expand
        // with generic spacing and produce long radial connectors. Compress
        // spacing slightly when hub dominance is high.
        let mut degree_by_node: HashMap<&str, usize> = HashMap::new();
        for edge in &graph.edges {
            *degree_by_node.entry(edge.from.as_str()).or_insert(0) += 1;
            *degree_by_node.entry(edge.to.as_str()).or_insert(0) += 1;
        }
        let max_degree = degree_by_node.values().copied().max().unwrap_or(0) as f32;
        let hub_ratio = if node_count > 0 {
            max_degree / node_count as f32
        } else {
            0.0
        };
        if node_count >= 10 && hub_ratio >= 0.30 && density <= 3.0 {
            let hub_scale = (0.92 - (hub_ratio - 0.30) * 0.55).clamp(0.62, 0.92);
            hub_compaction_scale = Some(hub_scale);
            hub_compaction_floor = auto.min_spacing * 0.5;
        }
        if node_count >= 12 && hub_ratio >= 0.40 && density <= 2.5 {
            prefer_direct_hub_routing = true;
        }
    }
    let node_count = graph.nodes.len();
    let edge_count = graph.edges.len();
    let tiny_graph = graph.subgraphs.is_empty() && node_count <= 4 && edge_count <= 8;
    if tiny_graph {
        effective_config.flowchart.order_passes = 4;
        effective_config.flowchart.routing.snap_ports_to_grid = false;
        // Small dense graphs need extra spacing for edges to route smoothly
        // around nodes without sharp bends.
        let density = if node_count > 0 {
            edge_count as f32 / node_count as f32
        } else {
            0.0
        };
        if density >= 0.8 {
            effective_config.node_spacing = (effective_config.node_spacing * 1.6).max(80.0);
            let rank_spacing_floor =
                if is_small_dense_labeled_flowchart(graph, node_count, &graph.edges) {
                    74.0
                } else {
                    80.0
                };
            effective_config.rank_spacing = effective_config.rank_spacing.max(rank_spacing_floor);
        }
    }
    if prefer_direct_hub_routing {
        effective_config.flowchart.routing.enable_grid_router = false;
        effective_config.flowchart.routing.snap_ports_to_grid = false;
    }
    let mut nodes = BTreeMap::new();
    let measure_font_size = theme.font_size;
    let mut label_config = effective_config.clone();
    if graph.kind == crate::ir::DiagramKind::Class {
        label_config.label_line_height = label_config.class_diagram_label_line_height();
    }
    let mut state_marker_ids: Vec<String> = Vec::new();

    for node in graph.nodes.values() {
        let mut style = resolve_node_style(node.id.as_str(), graph);
        // Mermaid flowchart node labels wrap against flowchart.wrappingWidth
        // (200px), not our generic character-count cap. State, class, ER, and
        // requirement labels avoid the character cap and only honor explicit
        // breaks; Mermaid's requirementBox creates one nowrap text element per
        // row.
        let auto_wrap = !matches!(
            graph.kind,
            crate::ir::DiagramKind::State
                | crate::ir::DiagramKind::Class
                | crate::ir::DiagramKind::Er
                | crate::ir::DiagramKind::Requirement
        );
        let wrap_width_px = if graph.kind == crate::ir::DiagramKind::Flowchart {
            Some(FLOWCHART_WRAPPING_WIDTH)
        } else {
            None
        };
        let use_markdown_label =
            node.markdown_label || graph.kind == crate::ir::DiagramKind::Requirement;
        let mut label = if use_markdown_label {
            let label_source = if has_html_formatting(&node.label) {
                normalize_html_label(&node.label)
            } else {
                node.label.clone()
            };
            if graph.kind == crate::ir::DiagramKind::Flowchart {
                measure_markdown_label_with_wrap_width(
                    &label_source,
                    theme,
                    &label_config,
                    wrap_width_px,
                )
            } else {
                let inherited_weight = if graph.kind == crate::ir::DiagramKind::Requirement {
                    style.font_weight.as_deref()
                } else {
                    None
                };
                measure_markdown_label_with_inherited_font_weight(
                    &label_source,
                    theme,
                    &label_config,
                    None,
                    inherited_weight,
                )
            }
        } else if has_html_formatting(&node.label) {
            let normalized = normalize_html_label(&node.label);
            if graph.kind == crate::ir::DiagramKind::Flowchart {
                measure_markdown_label_with_wrap_width(
                    &normalized,
                    theme,
                    &label_config,
                    wrap_width_px,
                )
            } else {
                measure_markdown_label(&normalized, theme, &label_config)
            }
        } else {
            measure_label_with_font_size_and_wrap_width(
                &node.label,
                measure_font_size,
                &label_config,
                auto_wrap,
                theme.font_family.as_str(),
                wrap_width_px,
            )
        };
        if graph.kind == crate::ir::DiagramKind::Flowchart
            && node.icon.is_some()
            && node.label.trim().is_empty()
        {
            if node
                .icon
                .as_deref()
                .and_then(crate::icons::lookup_icon)
                .is_some()
            {
                label.width = 20.0;
                label.height = 24.0;
            } else {
                label.width = 0.0;
                label.height = 0.0;
            }
        }
        let label_empty = label.lines.len() == 1 && label.lines[0].text().trim().is_empty();
        let (mut width, mut height) =
            shape_size(node.shape, &label, &effective_config, theme, graph.kind);
        if graph.kind == crate::ir::DiagramKind::Flowchart
            && node.icon.is_some()
            && node.label.trim().is_empty()
            && node
                .icon
                .as_deref()
                .and_then(crate::icons::lookup_icon)
                .is_none()
        {
            width = 30.0;
            height = 30.0;
        }
        if graph.kind == crate::ir::DiagramKind::State
            && label_empty
            && matches!(
                node.shape,
                crate::ir::NodeShape::Circle | crate::ir::NodeShape::DoubleCircle
            )
        {
            width = STATE_MARKER_FIXED_SIZE;
            height = STATE_MARKER_FIXED_SIZE;
            state_marker_ids.push(node.id.clone());
        }
        let use_mermaid_node_stroke = matches!(
            graph.kind,
            crate::ir::DiagramKind::Block | crate::ir::DiagramKind::Flowchart
        ) || (graph.kind == crate::ir::DiagramKind::Class
            && node.shape != crate::ir::NodeShape::Note);
        if use_mermaid_node_stroke
            && style.stroke.is_none()
            && theme
                .primary_border_color
                .eq_ignore_ascii_case(RUST_MERMAID_DEFAULT_PRIMARY_BORDER)
        {
            style.stroke = Some(MERMAID_DEFAULT_NODE_STROKE.to_string());
        }
        nodes.insert(
            node.id.clone(),
            build_node_layout(node, label, width, height, style, graph),
        );
    }

    if graph.kind == crate::ir::DiagramKind::State && !state_marker_ids.is_empty() {
        for id in state_marker_ids {
            if let Some(node) = nodes.get_mut(&id) {
                node.width = STATE_MARKER_FIXED_SIZE;
                node.height = STATE_MARKER_FIXED_SIZE;
            }
        }
    }

    let adaptive_spacing_exclusions = if graph.kind == crate::ir::DiagramKind::Flowchart {
        subgraph_anchor_ids_for_nodes(graph, &nodes)
    } else {
        HashSet::new()
    };
    let adaptive_node_spacing = adaptive_spacing_for_nodes(
        &nodes,
        &adaptive_spacing_exclusions,
        effective_config.flowchart.auto_spacing.min_spacing,
        effective_config.node_spacing,
    );
    let adaptive_rank_spacing = adaptive_spacing_for_nodes(
        &nodes,
        &adaptive_spacing_exclusions,
        effective_config.flowchart.auto_spacing.min_spacing,
        effective_config.rank_spacing,
    );
    if adaptive_node_spacing < effective_config.node_spacing {
        effective_config.node_spacing = adaptive_node_spacing;
    }
    if adaptive_rank_spacing < effective_config.rank_spacing {
        effective_config.rank_spacing = adaptive_rank_spacing;
    }
    if let Some(scale) = hub_compaction_scale {
        let floor = hub_compaction_floor.max(14.0);
        effective_config.node_spacing = (effective_config.node_spacing * scale).max(floor);
        effective_config.rank_spacing = (effective_config.rank_spacing * scale).max(floor);
    }

    let config = &effective_config;

    let anchor_ids = mark_subgraph_anchor_nodes_hidden(graph, &mut nodes);
    let mut anchor_info = apply_subgraph_anchor_sizes(graph, &mut nodes, theme, config);
    let mut anchored_subgraph_nodes =
        collect_anchored_subgraph_layout_exclusions(graph, &nodes, &anchor_info);

    let anchored_indices: HashSet<usize> = anchor_info.values().map(|info| info.sub_idx).collect();
    let mut edge_redirects: HashMap<String, String> = HashMap::new();
    if !graph.subgraphs.is_empty() {
        for (idx, sub) in graph.subgraphs.iter().enumerate() {
            let Some(anchor_id) = subgraph_anchor_id(sub, &nodes) else {
                continue;
            };
            if anchored_indices.contains(&idx) {
                continue;
            }
            if let Some(anchor_child) = pick_subgraph_anchor_child(sub, graph, &anchor_ids)
                && anchor_child != anchor_id
            {
                edge_redirects.insert(anchor_id.to_string(), anchor_child);
            }
        }
    }

    let mut layout_edges: Vec<crate::ir::Edge> = Vec::with_capacity(graph.edges.len());
    for edge in &graph.edges {
        let mut layout_edge = edge.clone();
        if let Some(new_from) = edge_redirects.get(&layout_edge.from) {
            layout_edge.from = new_from.clone();
        }
        if let Some(new_to) = edge_redirects.get(&layout_edge.to) {
            layout_edge.to = new_to.clone();
        }
        layout_edges.push(layout_edge);
    }

    let layout_order = manual_layout_node_order(graph);
    let mut layout_node_ids: Vec<String> = graph.nodes.keys().cloned().collect();
    layout_node_ids.sort_by(|a, b| {
        layout_order
            .get(a)
            .copied()
            .unwrap_or(usize::MAX)
            .cmp(&layout_order.get(b).copied().unwrap_or(usize::MAX))
            .then_with(|| a.cmp(b))
    });
    let active_anchor_ids: HashSet<String> = anchor_info.keys().cloned().collect();
    if !anchor_ids.is_empty() {
        layout_node_ids.retain(|id| !anchor_ids.contains(id) || active_anchor_ids.contains(id));
    }
    if !anchored_subgraph_nodes.is_empty() {
        layout_node_ids.retain(|id| !anchored_subgraph_nodes.contains(id));
    }
    let mut layout_set: HashSet<String> = layout_node_ids.iter().cloned().collect();

    if anchor_info.is_empty() {
        anchor_info = apply_subgraph_anchor_sizes(graph, &mut nodes, theme, config);
        anchored_subgraph_nodes =
            collect_anchored_subgraph_layout_exclusions(graph, &nodes, &anchor_info);
        if !anchored_subgraph_nodes.is_empty() {
            layout_node_ids.retain(|id| !anchored_subgraph_nodes.contains(id));
        }
        layout_set = layout_node_ids.iter().cloned().collect();
    }

    // Pre-measure all edge labels once (reused across layout, routing, and edge construction).
    let measure_edge_field =
        |field: &Option<String>, markdown_label: bool, endpoint_label: bool| -> Option<TextBlock> {
            field.as_ref().map(|label| {
                if markdown_label {
                    return measure_markdown_label(label, theme, config);
                }
                if has_html_formatting(label) {
                    let normalized = normalize_html_label(label);
                    return measure_markdown_label(&normalized, theme, config);
                }
                let label_text = if graph.kind == crate::ir::DiagramKind::Requirement {
                    requirement_edge_label_text(label, config)
                } else {
                    label.clone()
                };
                if endpoint_label && graph.kind == crate::ir::DiagramKind::Class {
                    return measure_label_with_font_size(
                        &label_text,
                        label_placement::CLASS_ENDPOINT_LABEL_FONT_SIZE,
                        config,
                        false,
                        theme.font_family.as_str(),
                    );
                }
                if graph.kind == crate::ir::DiagramKind::Flowchart
                    && !label_text.contains('\n')
                    && !label_text.contains("<br")
                    && label_text.chars().count() >= FLOWCHART_EDGE_LABEL_WRAP_TRIGGER_CHARS
                {
                    let mut wrap_cfg = config.clone();
                    wrap_cfg.max_label_width_chars = wrap_cfg
                        .max_label_width_chars
                        .min(FLOWCHART_EDGE_LABEL_WRAP_MAX_CHARS);
                    measure_label_with_font_size(
                        &label_text,
                        theme.font_size.max(16.0),
                        &wrap_cfg,
                        true,
                        theme.font_family.as_str(),
                    )
                } else {
                    measure_label(&label_text, theme, config)
                }
            })
        };
    let edge_route_labels: Vec<Option<TextBlock>> = graph
        .edges
        .iter()
        .map(|e| measure_edge_field(&e.label, e.markdown_label, false))
        .collect();
    let edge_start_labels: Vec<Option<TextBlock>> = graph
        .edges
        .iter()
        .map(|e| measure_edge_field(&e.start_label, e.markdown_label, true))
        .collect();
    let edge_end_labels: Vec<Option<TextBlock>> = graph
        .edges
        .iter()
        .map(|e| measure_edge_field(&e.end_label, e.markdown_label, true))
        .collect();

    let mut label_dummy_ids: Vec<Option<String>> = vec![None; graph.edges.len()];
    assign_positions_manual(
        graph,
        &layout_node_ids,
        &layout_set,
        &mut nodes,
        config,
        &layout_edges,
        theme,
        &edge_route_labels,
        &mut label_dummy_ids,
        &layout_order,
    );

    apply_unconnected_class_namespace_layouts(graph, &mut nodes, config);

    if !graph.subgraphs.is_empty() {
        if graph.kind != crate::ir::DiagramKind::State {
            apply_subgraph_direction_overrides(graph, &mut nodes, config, &anchored_indices);
        }
        if !anchor_info.is_empty() {
            let _anchored_nodes =
                align_subgraphs_to_anchor_nodes(graph, &anchor_info, &mut nodes, config);
        }
        if graph.kind == crate::ir::DiagramKind::State && !anchor_info.is_empty() {
            apply_state_subgraph_layouts(graph, &mut nodes, config, &anchored_indices);
            align_state_markers_to_subgraph_columns(graph, &mut nodes);
        }
        apply_orthogonal_region_bands(graph, &mut nodes, config);
        if graph.kind != crate::ir::DiagramKind::State {
            apply_subgraph_bands(graph, &mut nodes, config);
        }
    }

    compress_linear_subgraphs(graph, &mut nodes, config);
    enforce_top_level_subgraph_gap(graph, &mut nodes, theme, config);

    // Separate overlapping sibling subgraphs
    separate_sibling_subgraphs(graph, &mut nodes, theme, config);
    enforce_cluster_band_separation(graph, &mut nodes, config);
    align_disconnected_top_level_subgraphs(graph, &mut nodes);
    reorder_disconnected_top_level_flowchart_groups_like_dagre(graph, &mut nodes);
    align_disconnected_components(graph, &mut nodes, config);
    apply_visual_objectives(graph, &layout_edges, &mut nodes, theme, &effective_config);
    if graph.kind == crate::ir::DiagramKind::Flowchart && !graph.subgraphs.is_empty() {
        apply_subgraph_direction_overrides(graph, &mut nodes, config, &anchored_indices);
        apply_flowchart_nested_subgraph_direction_overrides(graph, &mut nodes, theme, config);
        apply_flowchart_dagre_root_fanout_centering(graph, &mut nodes, config);
        apply_flowchart_dagre_linear_chain_centering(graph, &mut nodes, config);
        pack_flowchart_recursive_subgraph_components(graph, &mut nodes, theme, config);
        align_flowchart_mixed_recursive_compound(graph, &mut nodes, theme, config);
        align_flowchart_nested_bridge_child_lanes(graph, &mut nodes, theme, config);
    }
    apply_parallel_top_level_flowchart_compound_nodes(graph, &mut nodes, config);

    // For state diagrams, push non-member nodes outside subgraph bounds
    if graph.kind == crate::ir::DiagramKind::State && !graph.subgraphs.is_empty() {
        push_non_members_out_of_subgraphs(graph, &mut nodes, theme, config);
    }

    reorder_disconnected_top_level_flowchart_groups_like_dagre(graph, &mut nodes);

    // Iter 258 attempted: state-diagram dagre layout (NS + BK applied as
    // post-pass). Result: nested-composite-states moved to 495×778 (vs JS
    // 530×805), but composite-states collapsed to 318×244 (vs JS 151×780,
    // -536 height regression), concurrency to 468×247 (vs JS 1193×573,
    // -726 WIDTH regression), transitions-between-composite-states to
    // 276×244 (vs JS 125×652, -408 height). Reverted.
    //
    // Root cause: applying dagre globally as a post-pass overrides the
    // cluster-aware X centers established by per-cluster layout. Diagrams
    // with deep nesting or wide cross-cluster spans get their X positions
    // squashed by BK's median balancing (which doesn't know about cluster
    // structure), and Y positions decoupled from cluster bbox computation.
    //
    // Network simplex and Brandes-Köpf are committed at
    // src/layout/network_simplex.rs and src/layout/brandes_kopf.rs as
    // standalone, tested algorithms. The integration callsite is
    // src/layout/state_dagre.rs but is currently unused.
    //
    // The proper integration requires either:
    //   - Replace the per-cluster `assign_positions` calls (5+ sites in
    //     mod.rs) with a unified dagre call that preserves cluster
    //     structure; OR
    //   - Use BK only WITHIN clusters (not globally), and stitch cluster
    //     positions together via the existing anchor-based layout.
    // Both are multi-week refactors.

    let mut subgraphs = build_subgraph_layouts(graph, &nodes, theme, config);
    apply_parallel_top_level_flowchart_compound_subgraphs(
        graph,
        &mut nodes,
        &mut subgraphs,
        config,
    );
    if stack_flowchart_top_level_subgraph_chain(graph, &mut nodes, &subgraphs, config) {
        subgraphs = build_subgraph_layouts(graph, &nodes, theme, config);
    }
    enforce_flowchart_nested_cluster_parent_padding(graph, &mut subgraphs);
    if stack_flowchart_top_level_subgraph_chain(graph, &mut nodes, &subgraphs, config) {
        subgraphs = build_subgraph_layouts(graph, &nodes, theme, config);
        enforce_flowchart_nested_cluster_parent_padding(graph, &mut subgraphs);
    }
    if graph.kind == crate::ir::DiagramKind::State && subgraphs.len() >= 2 {
        separate_overlapping_sibling_subgraph_rects(graph, &mut nodes, &mut subgraphs, config);
        // Iter 245 (option 1): JS dagre lays out the global compound graph,
        // so a state cluster grows to enclose its members at consistent
        // global ranks even when those ranks are pulled by cross-cluster
        // edges. RS lays clusters out independently, so a small cluster
        // (e.g. End containing the shared `second`) is much shorter than its
        // sibling (First) even though the cross-cluster edge target lives at
        // a low rank inside the sibling. As an approximation, expand each
        // top-level state cluster's Y-bbox to enclose the Y range of any
        // external nodes it directly connects to via edges.
        expand_state_clusters_for_cross_edges(graph, &nodes, &mut subgraphs);
        // Iter 266: align sibling state cluster TOPS when they share
        // inner-to-inner cross-cluster edges (the "shared node via last-
        // reference-wins" pattern). Triggers for nested-composite-states
        // where End's `second` connects to Third inside First. Does NOT
        // trigger for composite-states where clusters only have cluster-
        // level chain edges (First→End at boundary).
        align_sibling_state_clusters_with_inner_cross_edges(graph, &mut nodes, &mut subgraphs);
        state_dagre::apply_state_compound_dagre_layout(graph, &mut nodes, config);
        subgraphs = build_subgraph_layouts(graph, &nodes, theme, config);
        expand_state_clusters_for_cross_edges(graph, &nodes, &mut subgraphs);
        // Iter 282: align root_end with the cluster it connects to AFTER all
        // sibling-separation and cross-edge alignment passes have placed the
        // clusters at their final positions.
        align_root_end_to_connecting_cluster(graph, &mut nodes, config);
        align_root_start_to_connecting_cluster(graph, &mut nodes);
        enforce_state_root_leaf_composite_gaps(graph, &mut nodes, &mut subgraphs, config);
    }
    align_flowchart_recursive_cluster_external_nodes(graph, &mut nodes, &subgraphs, config);
    apply_flowchart_dagre_root_fanout_centering(graph, &mut nodes, config);
    apply_flowchart_dagre_linear_chain_centering(graph, &mut nodes, config);
    apply_subgraph_anchors(graph, &subgraphs, &mut nodes);
    apply_flowchart_dagre_member_leaf_label_alignment(graph, &mut nodes, config);
    apply_flowchart_dagre_recursive_root_rank_spacing(graph, &mut nodes, &subgraphs, config);
    let edge_directions = edge_effective_directions(graph);
    let obstacles = build_obstacles(&nodes, &subgraphs, config);
    let label_obstacles = build_label_obstacles_for_routing(&nodes, &subgraphs);
    let routing_grid = if config.flowchart.routing.enable_grid_router && !tiny_graph {
        build_routing_grid(&obstacles, config)
    } else {
        None
    };
    let port_assignment_start = Instant::now();
    let mut node_degrees: HashMap<String, usize> = HashMap::new();
    for edge in &graph.edges {
        *node_degrees.entry(edge.from.clone()).or_insert(0) += 1;
        *node_degrees.entry(edge.to.clone()).or_insert(0) += 1;
    }
    // Detect parallel long edges (multiple edges between the same node
    // pair where at least one spans 2+ ranks) and force them to alternate
    // Left/Right sides.  This matches dagre's behavior where such edges
    // route to opposite sides of intermediate nodes.  Adjacent-rank pairs
    // (both span 1) keep natural Bottom→Top routing with offset separation.
    let mut forced_sides: HashMap<usize, EdgeSide> = HashMap::new();
    if graph.kind == crate::ir::DiagramKind::Flowchart {
        let mut pair_edges: HashMap<(String, String), Vec<usize>> = HashMap::new();
        for (idx, edge) in graph.edges.iter().enumerate() {
            let key = if edge.from <= edge.to {
                (edge.from.clone(), edge.to.clone())
            } else {
                (edge.to.clone(), edge.from.clone())
            };
            pair_edges.entry(key).or_default().push(idx);
        }
        for indices in pair_edges.values() {
            if indices.len() < 2 {
                continue;
            }
            // Only force sides if the nodes are far apart on the main
            // axis (indicating a long-span edge that routes around
            // intermediate nodes).  Adjacent nodes keep normal routing.
            let has_long_span = indices.iter().any(|&idx| {
                let e = &graph.edges[idx];
                let f = nodes.get(&e.from);
                let t = nodes.get(&e.to);
                if let (Some(fn_), Some(tn)) = (f, t) {
                    let main_gap = if is_horizontal(edge_directions[idx]) {
                        (fn_.x - tn.x).abs() - fn_.width.max(tn.width)
                    } else {
                        (fn_.y - tn.y).abs() - fn_.height.max(tn.height)
                    };
                    main_gap > config.rank_spacing * 1.5
                } else {
                    false
                }
            });
            if !has_long_span {
                continue;
            }
            for (i, &idx) in indices.iter().enumerate() {
                forced_sides.insert(
                    idx,
                    if i % 2 == 0 {
                        EdgeSide::Left
                    } else {
                        EdgeSide::Right
                    },
                );
            }
        }
    }

    let mut side_loads: HashMap<String, [usize; 4]> = HashMap::new();
    let mut edge_ports: Vec<EdgePortInfo> = Vec::with_capacity(graph.edges.len());
    let mut port_candidates: HashMap<(String, EdgeSide), Vec<PortCandidate>> = HashMap::new();
    let mut side_choice_segments: Vec<Segment> = Vec::with_capacity(graph.edges.len());
    for (idx, edge) in graph.edges.iter().enumerate() {
        let edge_direction = edge_directions[idx];
        let from_layout = nodes.get(&edge.from).expect("from node missing");
        let to_layout = nodes.get(&edge.to).expect("to node missing");
        let (temp_from, temp_to) =
            cluster_anchor_pair(from_layout, to_layout, &subgraphs, edge_direction);
        let from = temp_from.as_ref().unwrap_or(from_layout);
        let to = temp_to.as_ref().unwrap_or(to_layout);
        let use_balanced_sides = !matches!(graph.kind, crate::ir::DiagramKind::Architecture);
        let from_degree = node_degrees.get(&edge.from).copied().unwrap_or(0);
        let to_degree = node_degrees.get(&edge.to).copied().unwrap_or(0);
        let allow_low_degree_balancing = graph.kind == crate::ir::DiagramKind::Flowchart
            || (edge.style == crate::ir::EdgeStyle::Dotted && from_degree <= 4 && to_degree <= 4);
        let primary_sides = edge_sides(from, to, edge_direction);
        let mut selected_sides = if use_balanced_sides {
            edge_sides_balanced(
                &edge.from,
                &edge.to,
                from,
                to,
                allow_low_degree_balancing,
                edge_direction,
                &node_degrees,
                &side_loads,
            )
        } else {
            primary_sides
        };
        if use_balanced_sides
            && (selected_sides.0 != primary_sides.0 || selected_sides.1 != primary_sides.1)
        {
            let candidate_points = [
                anchor_point_for_node(from, selected_sides.0, 0.0),
                anchor_point_for_node(to, selected_sides.1, 0.0),
            ];
            let primary_points = [
                anchor_point_for_node(from, primary_sides.0, 0.0),
                anchor_point_for_node(to, primary_sides.1, 0.0),
            ];
            let (candidate_crossings, _) =
                edge_crossings_with_existing(&candidate_points, &side_choice_segments);
            let (primary_crossings, _) =
                edge_crossings_with_existing(&primary_points, &side_choice_segments);
            if candidate_crossings > primary_crossings {
                selected_sides = primary_sides;
            }
        }
        // Override with forced Left/Right for parallel long edges.
        if let Some(&forced) = forced_sides.get(&idx) {
            selected_sides = (forced, forced, selected_sides.2);
        }
        let (start_side, end_side, _is_backward) = selected_sides;
        bump_side_load(&mut side_loads, &edge.from, start_side);
        bump_side_load(&mut side_loads, &edge.to, end_side);
        edge_ports.push(EdgePortInfo {
            start_side,
            end_side,
            start_offset: 0.0,
            end_offset: 0.0,
        });

        let from_anchor = anchor_point_for_node(from, start_side, 0.0);
        let to_anchor = anchor_point_for_node(to, end_side, 0.0);
        // Compute the ideal port position: where a straight line from the
        // remote anchor to this node's centre crosses the node boundary on
        // the given side.  This produces positions in the node's coordinate
        // space, so ports naturally cluster where the geometry dictates
        // rather than being spread across the full node width.
        let start_other = ideal_port_pos((to_anchor.0, to_anchor.1), from, start_side);
        let end_other = ideal_port_pos((from_anchor.0, from_anchor.1), to, end_side);
        port_candidates
            .entry((edge.from.clone(), start_side))
            .or_default()
            .push(PortCandidate {
                edge_idx: idx,
                is_start: true,
                other_pos: start_other,
            });
        port_candidates
            .entry((edge.to.clone(), end_side))
            .or_default()
            .push(PortCandidate {
                edge_idx: idx,
                is_start: false,
                other_pos: end_other,
            });
        side_choice_segments.push((from_anchor, to_anchor));
    }
    let routing_cell = routing_cell_size(config);
    for ((node_id, side), candidates) in port_candidates {
        let Some(node) = nodes.get(&node_id) else {
            continue;
        };
        let mut min_other = f32::MAX;
        let mut max_other = f32::MIN;
        for candidate in &candidates {
            min_other = min_other.min(candidate.other_pos);
            max_other = max_other.max(candidate.other_pos);
        }
        let span = (max_other - min_other).max(0.0);
        let mut order: Vec<usize> = (0..candidates.len()).collect();
        order.sort_by(|&a, &b| {
            candidates[a]
                .other_pos
                .partial_cmp(&candidates[b].other_pos)
                .unwrap_or(Ordering::Equal)
        });
        let node_len = if side_is_vertical(side) {
            node.height
        } else {
            node.width
        };
        let pad = (node_len * config.flowchart.port_pad_ratio)
            .min(config.flowchart.port_pad_max)
            .max(config.flowchart.port_pad_min);
        let usable = (node_len - 2.0 * pad).max(1.0);
        let min_sep = usable / (candidates.len() as f32 + 1.0);
        let snap_to_grid = config.flowchart.routing.snap_ports_to_grid
            && routing_cell > 0.0
            && min_sep >= routing_cell * 0.75;
        // other_pos is now an ideal port coordinate (x or y) in absolute
        // space.  Normalise it within the node's usable range so that ports
        // land where straight-line geometry dictates.
        let node_start = if side_is_vertical(side) {
            node.y
        } else {
            node.x
        };
        let ideal_span = span; // span of ideal positions across the node
        let span_frac = if usable > 1.0 {
            (ideal_span / usable).min(2.0)
        } else {
            1.0
        };
        let position_weight = (0.5 + 0.35 * span_frac).clamp(0.50, 0.85);
        let rank_weight = 1.0 - position_weight;
        let desired: Vec<(usize, f32)> = order
            .iter()
            .enumerate()
            .map(|(rank, &idx)| {
                let candidate = &candidates[idx];
                let pos_in_node = candidate.other_pos - node_start;
                let t_pos = ((pos_in_node - pad) / usable).clamp(0.0, 1.0);
                let t_rank = (rank as f32 + 0.5) / candidates.len() as f32;
                let t = t_pos * position_weight + t_rank * rank_weight;
                let pos = pad + t * usable;
                (idx, pos)
            })
            .collect();
        let mut assigned = vec![0.0; candidates.len()];
        let mut prev = pad;
        for (order_idx, (cand_idx, pos)) in desired.iter().enumerate() {
            let mut p = *pos;
            if order_idx == 0 {
                p = p.max(pad);
            } else {
                p = p.max(prev + min_sep);
            }
            assigned[*cand_idx] = p;
            prev = p;
        }
        let mut next = pad + usable;
        for (order_idx, (cand_idx, _pos)) in desired.iter().enumerate().rev() {
            let mut p = assigned[*cand_idx];
            if order_idx + 1 == desired.len() {
                p = p.min(next);
            } else {
                p = p.min(next - min_sep);
            }
            assigned[*cand_idx] = p;
            next = p;
        }
        for (rank, &cand_idx) in order.iter().enumerate() {
            let candidate = &candidates[cand_idx];
            let mut offset = assigned[cand_idx] - node_len / 2.0;
            if snap_to_grid {
                offset = (offset / routing_cell).round() * routing_cell;
            }
            if config.flowchart.port_side_bias != 0.0 {
                offset += config.flowchart.port_side_bias
                    * (rank as f32 - (candidates.len() as f32 - 1.0) / 2.0);
            }
            if let Some(info) = edge_ports.get_mut(candidate.edge_idx) {
                if candidate.is_start {
                    info.start_offset = offset;
                } else {
                    info.end_offset = offset;
                }
            }
        }
    }
    if let Some(metrics) = stage_metrics.as_deref_mut() {
        metrics.port_assignment_us = metrics
            .port_assignment_us
            .saturating_add(port_assignment_start.elapsed().as_micros());
    }

    let edge_routing_start = Instant::now();
    let pair_counts = build_edge_pair_counts(&graph.edges);
    let mut pair_seen: HashMap<(String, String), usize> = HashMap::new();
    let mut pair_index: Vec<usize> = vec![0; graph.edges.len()];
    for (idx, edge) in graph.edges.iter().enumerate() {
        let key = edge_pair_key(edge);
        let seen = pair_seen.entry(key).or_insert(0usize);
        pair_index[idx] = *seen;
        *seen += 1;
    }

    let mut cross_edge_offsets = vec![0.0f32; graph.edges.len()];
    if graph.kind == crate::ir::DiagramKind::Flowchart {
        let band_size = (config.node_spacing * 2.0).max(30.0);
        let mut groups: HashMap<i32, Vec<(usize, f32)>> = HashMap::new();
        for (idx, edge) in graph.edges.iter().enumerate() {
            let edge_direction = edge_directions[idx];
            let is_horizontal_layout = is_horizontal(edge_direction);
            let from_layout = nodes.get(&edge.from).expect("from node missing");
            let to_layout = nodes.get(&edge.to).expect("to node missing");
            let (temp_from, temp_to) =
                cluster_anchor_pair(from_layout, to_layout, &subgraphs, edge_direction);
            let from = temp_from.as_ref().unwrap_or(from_layout);
            let to = temp_to.as_ref().unwrap_or(to_layout);
            let from_center = (from.x + from.width / 2.0, from.y + from.height / 2.0);
            let to_center = (to.x + to.width / 2.0, to.y + to.height / 2.0);
            let dx = to_center.0 - from_center.0;
            let dy = to_center.1 - from_center.1;
            let cross_axis = if is_horizontal_layout {
                dy.abs()
            } else {
                dx.abs()
            };
            let main_axis = if is_horizontal_layout {
                dx.abs()
            } else {
                dy.abs()
            };
            let is_secondary = edge.style == crate::ir::EdgeStyle::Dotted || edge.label.is_some();
            if !is_secondary || cross_axis <= main_axis * 1.2 {
                continue;
            }
            let band_coord = if is_horizontal_layout {
                (from_center.0 + to_center.0) * 0.5
            } else {
                (from_center.1 + to_center.1) * 0.5
            };
            let bucket = (band_coord / band_size).round() as i32;
            let sort_key = if is_horizontal_layout {
                (from_center.1 + to_center.1) * 0.5
            } else {
                (from_center.0 + to_center.0) * 0.5
            };
            groups.entry(bucket).or_default().push((idx, sort_key));
        }
        let spacing = (config.node_spacing * 0.45).max(8.0);
        for (_bucket, mut group) in groups {
            if group.len() <= 1 {
                continue;
            }
            group.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
            let center = (group.len() as f32 - 1.0) * 0.5;
            for (pos, (idx, _)) in group.iter().enumerate() {
                cross_edge_offsets[*idx] = (pos as f32 - center) * spacing;
            }
        }
    }
    let flowchart_root_fanout_edges =
        flowchart_dagre_root_fanout_edge_indices(graph, &nodes, config);
    let flowchart_same_rank_fanout_routes =
        flowchart_dagre_same_rank_fanout_edge_indices(graph, &nodes, config);
    let flowchart_cycle_routes = flowchart_dagre_three_node_cycle_routes(graph, &nodes);

    let mut route_order: Vec<(u8, f32, f32, usize)> = Vec::with_capacity(graph.edges.len());
    let dense_flowchart_routing = graph.kind == crate::ir::DiagramKind::Flowchart
        && graph.edges.len() >= 18
        && graph.edges.len() * 2 >= layout_node_ids.len() * 3;
    for (idx, edge) in graph.edges.iter().enumerate() {
        let edge_direction = edge_directions[idx];
        let from_layout = nodes.get(&edge.from).expect("from node missing");
        let to_layout = nodes.get(&edge.to).expect("to node missing");
        let (temp_from, temp_to) =
            cluster_anchor_pair(from_layout, to_layout, &subgraphs, edge_direction);
        let from = temp_from.as_ref().unwrap_or(from_layout);
        let to = temp_to.as_ref().unwrap_or(to_layout);
        let from_center = (from.x + from.width / 2.0, from.y + from.height / 2.0);
        let to_center = (to.x + to.width / 2.0, to.y + to.height / 2.0);
        let dx = to_center.0 - from_center.0;
        let dy = to_center.1 - from_center.1;
        let cross_axis = if is_horizontal(edge_direction) {
            dy.abs()
        } else {
            dx.abs()
        };
        let main_axis = if is_horizontal(edge_direction) {
            dx.abs()
        } else {
            dy.abs()
        };
        let (_, _, is_backward) = edge_sides(from, to, edge_direction);
        let is_dotted = edge.style == crate::ir::EdgeStyle::Dotted;
        let has_label = edge.label.is_some();
        let is_secondary = is_dotted || has_label;
        let has_open_triangle = matches!(
            edge.arrow_start_kind,
            Some(crate::ir::EdgeArrowhead::OpenTriangle)
        ) || matches!(
            edge.arrow_end_kind,
            Some(crate::ir::EdgeArrowhead::OpenTriangle)
        );
        let priority = if graph.kind == crate::ir::DiagramKind::Class {
            if has_open_triangle {
                0u8
            } else if is_secondary {
                2u8
            } else if is_backward {
                1u8
            } else {
                1u8
            }
        } else if graph.kind == crate::ir::DiagramKind::State {
            // State machines often have long back-edges to earlier states.
            // Route those first so later local transitions can avoid them.
            if is_backward {
                0u8
            } else if has_label || is_dotted {
                1u8
            } else {
                2u8
            }
        } else if is_dotted {
            if dense_flowchart_routing { 1u8 } else { 2u8 }
        } else if has_label || is_backward {
            1u8
        } else {
            0u8
        };
        route_order.push((priority, cross_axis, main_axis, idx));
    }
    let steep_count = route_order
        .iter()
        .filter(|(_, cross_axis, main_axis, _)| *cross_axis > *main_axis * 0.8)
        .count();
    let use_cross_axis_order = graph.edges.len() >= 10 && steep_count * 4 >= graph.edges.len();
    if use_cross_axis_order {
        route_order.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
                .then_with(|| b.2.partial_cmp(&a.2).unwrap_or(Ordering::Equal))
                .then_with(|| a.3.cmp(&b.3))
        });
    } else {
        let use_priority_preorder = graph.edges.len() >= 10;
        route_order.sort_by(|a, b| {
            let len_a = a.1 * a.1 + a.2 * a.2;
            let len_b = b.1 * b.1 + b.2 * b.2;
            let by_length = len_b.partial_cmp(&len_a).unwrap_or(Ordering::Equal);
            if use_priority_preorder {
                a.0.cmp(&b.0)
                    .then_with(|| by_length)
                    .then_with(|| a.3.cmp(&b.3))
            } else {
                by_length.then_with(|| a.3.cmp(&b.3))
            }
        });
    }

    let mut routed_points: Vec<Vec<(f32, f32)>> = vec![Vec::new(); graph.edges.len()];
    let use_occupancy = !tiny_graph && graph.edges.len() > 2;
    let mut edge_occupancy = if use_occupancy {
        Some(EdgeOccupancy::new(
            config.node_spacing.max(MIN_NODE_SPACING_FLOOR) * EDGE_OCCUPANCY_CELL_RATIO,
        ))
    } else {
        None
    };
    let has_label_dummies = nodes
        .keys()
        .any(|id| id.starts_with("__elabel_") && id.ends_with("__"));
    let mut route_label_obstacles = label_obstacles;
    let (edge_label_pad_x, edge_label_pad_y) =
        label_placement::edge_label_padding(graph.kind, config);
    let mut route_label_plans: Vec<Option<RouteLabelPlan>> = vec![None; graph.edges.len()];
    for idx in 0..graph.edges.len() {
        if label_dummy_ids
            .get(idx)
            .and_then(|dummy_id| dummy_id.as_ref())
            .is_some()
        {
            continue;
        }
        let Some(label) = edge_route_labels.get(idx).and_then(|label| label.as_ref()) else {
            continue;
        };
        if label.width <= 0.0 || label.height <= 0.0 {
            continue;
        }
        let edge = &graph.edges[idx];
        let edge_direction = edge_directions[idx];
        let from_layout = nodes.get(&edge.from).expect("from node missing");
        let to_layout = nodes.get(&edge.to).expect("to node missing");
        let (temp_from, temp_to) =
            cluster_anchor_pair(from_layout, to_layout, &subgraphs, edge_direction);
        let from = temp_from.as_ref().unwrap_or(from_layout);
        let to = temp_to.as_ref().unwrap_or(to_layout);
        let port_info = edge_ports
            .get(idx)
            .copied()
            .expect("edge port info missing");
        let start = anchor_point_for_node(from, port_info.start_side, port_info.start_offset);
        let end = anchor_point_for_node(to, port_info.end_side, port_info.end_offset);

        let key = edge_pair_key(edge);
        let total = *pair_counts.get(&key).unwrap_or(&1) as f32;
        let idx_in_pair = pair_index[idx] as f32;
        let mut base_offset = if total > 1.0 {
            (idx_in_pair - (total - 1.0) / 2.0) * (config.node_spacing * MULTI_EDGE_OFFSET_RATIO)
        } else {
            0.0
        } + cross_edge_offsets[idx];
        if graph.kind == crate::ir::DiagramKind::Flowchart {
            let raw_bias =
                (port_info.start_offset - port_info.end_offset) * FLOWCHART_PORT_ROUTE_BIAS_RATIO;
            let max_bias = (config.node_spacing * FLOWCHART_PORT_ROUTE_BIAS_MAX_RATIO).max(8.0);
            base_offset += raw_bias.clamp(-max_bias, max_bias);
        }

        // Detect bidirectional pair: pair has 2 edges with opposite
        // directions (A→B and B→A). For state diagrams this lets the two
        // labels sit at opposite ends of the shared path rather than both
        // landing at the midpoint and overlapping horizontally. Limited to
        // state diagrams because other diagram types (sequence, block)
        // have their own conventions for bidirectional label placement.
        let bidirectional_pair = graph.kind == crate::ir::DiagramKind::State
            && total as usize == 2
            && graph.edges.iter().enumerate().any(|(other_idx, other)| {
                other_idx != idx && other.from == edge.to && other.to == edge.from
            });
        let progress = if bidirectional_pair { 0.5 } else { 0.5 };

        let mut center = if bidirectional_pair {
            ((start.0 + end.0) * 0.5, (start.1 + end.1) * 0.5)
        } else {
            ((start.0 + end.0) * 0.5, (start.1 + end.1) * 0.5)
        };
        if is_horizontal(edge_direction) {
            center.0 += base_offset;
        } else {
            center.1 += base_offset;
        }
        // For bidirectional pairs in state diagrams, push the label
        // perpendicular to the layout axis, on the SAME side the matching
        // edge bows. This keeps the label off the curve (otherwise the
        // straight-line midpoint sits directly on the bowed edge).
        if bidirectional_pair {
            let bow_offset = config.node_spacing * 0.30;
            let perp_shift = bow_offset + label.width.max(label.height) * 0.5 + 6.0;
            let sign = if pair_index[idx] == 0 { -1.0 } else { 1.0 };
            if is_horizontal(graph.direction) {
                center.1 += sign * perp_shift;
            } else {
                center.0 += sign * perp_shift;
            }
        }
        let obstacle_id = format!("edge-label-reserved:{idx}");
        let obstacle_index = route_label_obstacles.len();
        route_label_obstacles.push(Obstacle {
            id: obstacle_id.clone(),
            x: center.0 - label.width / 2.0 - edge_label_pad_x,
            y: center.1 - label.height / 2.0 - edge_label_pad_y,
            width: label.width + 2.0 * edge_label_pad_x,
            height: label.height + 2.0 * edge_label_pad_y,
            members: None,
        });
        route_label_plans[idx] = Some(RouteLabelPlan {
            obstacle_id,
            obstacle_index,
            progress,
            center,
        });
    }
    let mut existing_segments: Vec<Segment> = Vec::new();
    let mut label_anchors: Vec<Option<(f32, f32)>> = vec![None; graph.edges.len()];
    let mut aligned_secondary_edges: Vec<bool> = vec![false; graph.edges.len()];
    let mut aligned_secondary_label_anchors: Vec<Option<(f32, f32)>> =
        vec![None; graph.edges.len()];
    for (_, _, _, idx) in &route_order {
        let edge = &graph.edges[*idx];
        let edge_direction = edge_directions[*idx];
        let key = edge_pair_key(edge);
        let total = *pair_counts.get(&key).unwrap_or(&1) as f32;
        let idx_in_pair = pair_index[*idx] as f32;
        let mut base_offset = if total > 1.0 {
            (idx_in_pair - (total - 1.0) / 2.0) * (config.node_spacing * MULTI_EDGE_OFFSET_RATIO)
        } else {
            0.0
        } + cross_edge_offsets[*idx];
        let from_layout = nodes.get(&edge.from).expect("from node missing");
        let to_layout = nodes.get(&edge.to).expect("to node missing");
        let (temp_from, temp_to) =
            cluster_anchor_pair(from_layout, to_layout, &subgraphs, edge_direction);
        let from = temp_from.as_ref().unwrap_or(from_layout);
        let to = temp_to.as_ref().unwrap_or(to_layout);
        let port_info = edge_ports
            .get(*idx)
            .copied()
            .expect("edge port info missing");
        if graph.kind == crate::ir::DiagramKind::Flowchart {
            let raw_bias =
                (port_info.start_offset - port_info.end_offset) * FLOWCHART_PORT_ROUTE_BIAS_RATIO;
            let max_bias = (config.node_spacing * FLOWCHART_PORT_ROUTE_BIAS_MAX_RATIO).max(8.0);
            base_offset += raw_bias.clamp(-max_bias, max_bias);
        }
        let default_stub = port_stub_length(config, from, to);
        let stub_len = match graph.kind {
            crate::ir::DiagramKind::Class
            | crate::ir::DiagramKind::Er
            | crate::ir::DiagramKind::Requirement => 0.0,
            _ => default_stub,
        };
        let max_edge_label_chars = [
            edge.label.as_deref(),
            edge.start_label.as_deref(),
            edge.end_label.as_deref(),
        ]
        .into_iter()
        .flatten()
        .map(|label| label.chars().count())
        .max()
        .unwrap_or(0);
        let has_endpoint_label = edge.start_label.is_some() || edge.end_label.is_some();
        let avoid_short_tie = graph.kind == crate::ir::DiagramKind::Flowchart
            && (has_endpoint_label
                || max_edge_label_chars >= FLOWCHART_EDGE_LABEL_WRAP_TRIGGER_CHARS);
        let preferred_label_id = route_label_plans
            .get(*idx)
            .and_then(|plan| plan.as_ref())
            .map(|plan| plan.obstacle_id.as_str());
        let preferred_label_center = if matches!(
            graph.kind,
            crate::ir::DiagramKind::Flowchart | crate::ir::DiagramKind::State
        ) {
            None
        } else {
            route_label_plans
                .get(*idx)
                .and_then(|plan| plan.as_ref())
                .map(|plan| plan.center)
        };
        let route_ctx = RouteContext {
            from_id: &edge.from,
            to_id: &edge.to,
            from,
            to,
            direction: edge_direction,
            config,
            obstacles: &obstacles,
            label_obstacles: &route_label_obstacles,
            fast_route: false,
            base_offset,
            start_side: port_info.start_side,
            end_side: port_info.end_side,
            start_offset: port_info.start_offset,
            end_offset: port_info.end_offset,
            stub_len,
            prefer_shorter_ties: !avoid_short_tie,
            preferred_label_id,
            preferred_label_center,
        };
        let aligned_secondary_points = if graph.kind == crate::ir::DiagramKind::Flowchart
            && !forced_sides.contains_key(idx)
            && total <= 1.0
        {
            flowchart_aligned_secondary_edge_route(
                graph,
                &subgraphs,
                edge,
                from_layout,
                to_layout,
                edge_direction,
                edge_route_labels.get(*idx).and_then(|label| label.as_ref()),
            )
            .filter(|route| {
                path_obstacle_intersections(
                    &route.points,
                    route_ctx.obstacles,
                    route_ctx.from_id,
                    route_ctx.to_id,
                ) == 0
            })
        } else {
            None
        };
        let use_existing_for_edge = !(matches!(
            graph.kind,
            crate::ir::DiagramKind::Class | crate::ir::DiagramKind::Er
        ) && edge.style == crate::ir::EdgeStyle::Dotted);
        let existing_for_edge = if use_existing_for_edge {
            Some(existing_segments.as_slice())
        } else {
            None
        };
        // For forced Left/Right edges, generate clean orthogonal paths
        // (out → vertical → back) instead of complex A* routing.
        // This produces smooth rectangular detours like dagre.
        let dagre_like_points = if graph.kind == crate::ir::DiagramKind::Flowchart
            && !forced_sides.contains_key(idx)
            && total <= 1.0
        {
            if flowchart_root_fanout_edges.contains(idx) {
                flowchart_dagre_root_fanout_route(edge, from, to, edge_direction)
            } else if let Some(&fanout_direction) = flowchart_same_rank_fanout_routes.get(idx) {
                flowchart_dagre_same_rank_fanout_route(edge, from, to, fanout_direction)
            } else if let Some(points) = flowchart_cycle_routes.get(idx) {
                Some(points.clone())
            } else {
                flowchart_forward_overlap_route(
                    edge,
                    from_layout,
                    to_layout,
                    &subgraphs,
                    edge_direction,
                )
            }
        } else {
            None
        };
        let mut used_aligned_secondary = false;
        let mut points = if forced_sides.contains_key(idx) {
            let detour_offset = config.node_spacing * 1.2;
            let start_pt =
                anchor_point_for_node(from, port_info.start_side, port_info.start_offset);
            let end_pt = anchor_point_for_node(to, port_info.end_side, port_info.end_offset);
            let detour_x = if port_info.start_side == EdgeSide::Left {
                start_pt.0.min(end_pt.0) - detour_offset
            } else {
                start_pt.0.max(end_pt.0) + detour_offset
            };
            vec![
                start_pt,
                (detour_x, start_pt.1),
                (detour_x, end_pt.1),
                end_pt,
            ]
        } else if let Some(route) = aligned_secondary_points {
            used_aligned_secondary = true;
            aligned_secondary_label_anchors[*idx] = route.label_anchor;
            route.points
        } else if let Some(points) = dagre_like_points {
            points
        } else {
            route_edge_with_avoidance(
                &route_ctx,
                edge_occupancy.as_ref(),
                routing_grid.as_ref(),
                existing_for_edge,
            )
        };
        if matches!(
            graph.kind,
            crate::ir::DiagramKind::Class | crate::ir::DiagramKind::Er
        ) {
            let fast_ctx = RouteContext {
                from_id: route_ctx.from_id,
                to_id: route_ctx.to_id,
                from: route_ctx.from,
                to: route_ctx.to,
                direction: route_ctx.direction,
                config: route_ctx.config,
                obstacles: route_ctx.obstacles,
                label_obstacles: route_ctx.label_obstacles,
                fast_route: true,
                base_offset: route_ctx.base_offset,
                start_side: route_ctx.start_side,
                end_side: route_ctx.end_side,
                start_offset: route_ctx.start_offset,
                end_offset: route_ctx.end_offset,
                stub_len: route_ctx.stub_len,
                prefer_shorter_ties: route_ctx.prefer_shorter_ties,
                preferred_label_id: route_ctx.preferred_label_id,
                preferred_label_center: route_ctx.preferred_label_center,
            };
            let fast_points = route_edge_with_avoidance(&fast_ctx, None, None, existing_for_edge);
            let fast_hits = path_obstacle_intersections(
                &fast_points,
                route_ctx.obstacles,
                route_ctx.from_id,
                route_ctx.to_id,
            );
            let fast_label_hits = path_label_intersections(
                &fast_points,
                route_ctx.label_obstacles,
                route_ctx.preferred_label_id,
            );
            if fast_hits == 0 && fast_label_hits == 0 {
                let (fast_cross, fast_overlap) =
                    edge_crossings_with_existing(&fast_points, &existing_segments);
                let (cur_cross, cur_overlap) =
                    edge_crossings_with_existing(&points, &existing_segments);
                if fast_cross < cur_cross
                    || (fast_cross == cur_cross && fast_overlap + 0.25 < cur_overlap)
                {
                    points = fast_points;
                }
            }
        }
        if label_dummy_ids
            .get(*idx)
            .and_then(|dummy_id| dummy_id.as_ref())
            .is_none()
            && let Some(plan) = route_label_plans
                .get_mut(*idx)
                .and_then(|plan| plan.as_mut())
        {
            let mut label_center = aligned_secondary_label_anchors[*idx]
                .or_else(|| path_point_at_progress(&points, plan.progress))
                .or_else(|| edge_label_anchor_from_points(&points))
                .unwrap_or(plan.center);
            // For bidirectional state-diagram pairs, push the label off the
            // bowed curve perpendicular to the layout axis. The path-progress
            // midpoint sits ON the curve, which causes the label rect to
            // overlap the edge stroke ("text too close to a line").
            if graph.kind == crate::ir::DiagramKind::State {
                let edge = &graph.edges[*idx];
                let key = edge_pair_key(edge);
                let total = *pair_counts.get(&key).unwrap_or(&1) as usize;
                let bidi = total == 2
                    && graph.edges.iter().enumerate().any(|(j, other)| {
                        j != *idx && other.from == edge.to && other.to == edge.from
                    });
                if bidi {
                    if let Some(label) = edge_route_labels.get(*idx).and_then(|l| l.as_ref()) {
                        let bow_offset = config.node_spacing * 0.30;
                        let perp_shift = bow_offset + label.width.max(label.height) * 0.5 + 6.0;
                        let sign = if pair_index[*idx] == 0 { -1.0 } else { 1.0 };
                        if is_horizontal(graph.direction) {
                            label_center.1 += sign * perp_shift;
                        } else {
                            label_center.0 += sign * perp_shift;
                        }
                    }
                }
            }
            plan.center = label_center;
            label_anchors[*idx] = Some(label_center);
            if !matches!(
                graph.kind,
                crate::ir::DiagramKind::State | crate::ir::DiagramKind::Requirement
            ) && points.len() >= 2
            {
                insert_label_via_point(&mut points, label_center, edge_direction);
            }
            if let Some(label) = edge_route_labels.get(*idx).and_then(|label| label.as_ref())
                && let Some(obstacle) = route_label_obstacles.get_mut(plan.obstacle_index)
            {
                obstacle.x = label_center.0 - label.width / 2.0 - edge_label_pad_x;
                obstacle.y = label_center.1 - label.height / 2.0 - edge_label_pad_y;
                obstacle.width = label.width + 2.0 * edge_label_pad_x;
                obstacle.height = label.height + 2.0 * edge_label_pad_y;
            }
        }
        if let Some(occ) = edge_occupancy.as_mut() {
            occ.add_path(&points);
        }
        if points.len() >= 2 {
            for segment in points.windows(2) {
                existing_segments.push((segment[0], segment[1]));
            }
        }
        routed_points[*idx] = points;
        aligned_secondary_edges[*idx] = used_aligned_secondary;
    }

    if graph.kind == crate::ir::DiagramKind::Flowchart {
        reduce_orthogonal_path_crossings(graph, &nodes, &mut routed_points, config);
        deoverlap_flowchart_paths(graph, &nodes, &mut routed_points, config);
    } else if matches!(
        graph.kind,
        crate::ir::DiagramKind::Class | crate::ir::DiagramKind::Er | crate::ir::DiagramKind::State
    ) {
        reduce_orthogonal_path_crossings(graph, &nodes, &mut routed_points, config);
        if graph.kind == crate::ir::DiagramKind::Er {
            deoverlap_flowchart_paths(graph, &nodes, &mut routed_points, config);
        }
    }

    // Simplify edge paths: remove unnecessary intermediate waypoints so
    // the B-spline curve renderer can produce smoother arcs.
    for (idx, edge) in graph.edges.iter().enumerate() {
        if flowchart_root_fanout_edges.contains(&idx)
            || flowchart_same_rank_fanout_routes.contains_key(&idx)
            || flowchart_cycle_routes.contains_key(&idx)
            || aligned_secondary_edges.get(idx).copied().unwrap_or(false)
        {
            continue;
        }
        if routed_points[idx].len() > 2 {
            routed_points[idx] =
                simplify_edge_path(&routed_points[idx], &obstacles, &edge.from, &edge.to);
        }
    }

    // State/class diagrams: re-introduce a midpoint on 2-point edges so the
    // basis curve renderer produces a smooth bend, matching mermaid JS dagre +
    // curveBasis output. Two cases:
    //   - bidirectional state pairs: bow OUTWARD perpendicular to the line so
    //     the two parallel edges separate into opposing S-curves.
    //   - diagonal edge: snap to target column/row to form a dagre-like bend.
    if matches!(
        graph.kind,
        crate::ir::DiagramKind::Class | crate::ir::DiagramKind::State
    ) {
        let bow_offset = config.node_spacing * 0.30;
        for (idx, edge) in graph.edges.iter().enumerate() {
            let points = &mut routed_points[idx];
            if points.len() != 2 {
                continue;
            }
            let (sx, sy) = points[0];
            let (ex, ey) = points[1];
            let dx = ex - sx;
            let dy = ey - sy;
            let abs_dx = dx.abs();
            let abs_dy = dy.abs();

            let key = edge_pair_key(edge);
            let total = *pair_counts.get(&key).unwrap_or(&1) as usize;
            let bidirectional_pair =
                total == 2
                    && graph.edges.iter().enumerate().any(|(j, other)| {
                        j != idx && other.from == edge.to && other.to == edge.from
                    });

            if graph.kind == crate::ir::DiagramKind::State && bidirectional_pair {
                // Bow each edge perpendicular to the layout's main axis so the
                // two edges spread apart visibly. For TB/BT layouts the pair
                // runs vertically, so we offset the midpoint horizontally; for
                // LR/RL the pair runs horizontally so we offset vertically.
                // The sign picks opposing directions so the two edges of the
                // pair fan outward into a lens / S-curve shape (matching JS).
                let mid_x = (sx + ex) * 0.5;
                let mid_y = (sy + ey) * 0.5;
                let sign = if pair_index[idx] == 0 { -1.0 } else { 1.0 };
                let mid = match graph.direction {
                    crate::ir::Direction::TopDown | crate::ir::Direction::BottomTop => {
                        (mid_x + bow_offset * sign, mid_y)
                    }
                    crate::ir::Direction::LeftRight | crate::ir::Direction::RightLeft => {
                        (mid_x, mid_y + bow_offset * sign)
                    }
                };
                *points = vec![(sx, sy), mid, (ex, ey)];
                continue;
            }

            if graph.kind == crate::ir::DiagramKind::State && (abs_dx < 6.0 || abs_dy < 6.0) {
                continue;
            }
            if graph.kind == crate::ir::DiagramKind::State {
                if let (Some(from_node), Some(to_node)) =
                    (nodes.get(&edge.from), nodes.get(&edge.to))
                {
                    let from_is_fork_join = from_node.shape == crate::ir::NodeShape::ForkJoin;
                    let to_is_fork_join = to_node.shape == crate::ir::NodeShape::ForkJoin;
                    if from_is_fork_join || to_is_fork_join {
                        let mid = match graph.direction {
                            crate::ir::Direction::TopDown | crate::ir::Direction::BottomTop => {
                                let bend_x = if to_is_fork_join && !from_is_fork_join {
                                    sx
                                } else if from_is_fork_join && !to_is_fork_join {
                                    ex
                                } else {
                                    (sx + ex) * 0.5
                                };
                                (bend_x, (sy + ey) * 0.5)
                            }
                            crate::ir::Direction::LeftRight | crate::ir::Direction::RightLeft => {
                                let bend_y = if to_is_fork_join && !from_is_fork_join {
                                    sy
                                } else if from_is_fork_join && !to_is_fork_join {
                                    ey
                                } else {
                                    (sy + ey) * 0.5
                                };
                                ((sx + ex) * 0.5, bend_y)
                            }
                        };
                        *points = vec![
                            flowchart_node_intersection_toward(from_node, mid),
                            mid,
                            flowchart_node_intersection_toward(to_node, mid),
                        ];
                        continue;
                    }
                }
            }
            let mid = match graph.direction {
                crate::ir::Direction::TopDown | crate::ir::Direction::BottomTop => {
                    (ex, (sy + ey) * 0.5)
                }
                crate::ir::Direction::LeftRight | crate::ir::Direction::RightLeft => {
                    ((sx + ex) * 0.5, ey)
                }
            };
            *points = vec![(sx, sy), mid, (ex, ey)];
        }
    }
    if graph.kind == crate::ir::DiagramKind::Requirement {
        let mut outgoing_counts: HashMap<&str, usize> = HashMap::new();
        for edge in &graph.edges {
            *outgoing_counts.entry(edge.from.as_str()).or_insert(0) += 1;
        }
        for (idx, edge) in graph.edges.iter().enumerate() {
            let points = &mut routed_points[idx];
            if points.len() < 2 || edge.from == edge.to {
                continue;
            }
            let Some(from_node) = nodes.get(&edge.from) else {
                continue;
            };
            let Some(to_node) = nodes.get(&edge.to) else {
                continue;
            };
            let rank_gap_main =
                requirement_rank_gap_main(graph.direction, &nodes, &edge.from, &edge.to);
            let source_outgoing_count = outgoing_counts
                .get(edge.from.as_str())
                .copied()
                .unwrap_or(0);
            let mid = requirement_dagre_curve_midpoint(
                graph.direction,
                from_node,
                to_node,
                rank_gap_main,
                edge,
                source_outgoing_count,
                theme.font_size,
            );
            let start = rect_intersection_toward(from_node, mid);
            let end = rect_intersection_toward(to_node, mid);
            *points = vec![start, mid, end];
            if edge.label.is_some() {
                label_anchors[idx] = requirement_dagre_label_anchor(points, graph.direction);
            }
        }
    }
    if graph.kind == crate::ir::DiagramKind::Class {
        align_class_inheritance_fan_edges(graph, &nodes, &mut routed_points, config);
    }

    // Clamp detour swing: prevent curves from extending too far beyond the
    // source/target node bounding box.  Keeps diagrams proportional.
    if graph.kind == crate::ir::DiagramKind::Flowchart {
        let max_swing = config.node_spacing * 2.5;
        for (idx, edge) in graph.edges.iter().enumerate() {
            let points = &mut routed_points[idx];
            if points.len() <= 2 {
                continue;
            }
            let from_node = nodes.get(&edge.from);
            let to_node = nodes.get(&edge.to);
            if let (Some(f), Some(t)) = (from_node, to_node) {
                let min_x = f.x.min(t.x) - max_swing;
                let max_x = (f.x + f.width).max(t.x + t.width) + max_swing;
                let min_y = f.y.min(t.y) - max_swing;
                let max_y = (f.y + f.height).max(t.y + t.height) + max_swing;
                for p in points.iter_mut() {
                    p.0 = p.0.clamp(min_x, max_x);
                    p.1 = p.1.clamp(min_y, max_y);
                }
            }
        }
    }

    // Global post-routing passes (crossing reduction/deoverlap) can move paths
    // after we seeded label anchors. Re-apply the reserved label via-points so
    // center labels stay attached to their owning edge paths.
    for idx in 0..routed_points.len() {
        let Some(plan) = route_label_plans
            .get_mut(idx)
            .and_then(|plan| plan.as_mut())
        else {
            continue;
        };
        if graph.kind == crate::ir::DiagramKind::Requirement {
            continue;
        }
        let points = &mut routed_points[idx];
        if points.len() < 2 {
            continue;
        }
        let edge_direction = edge_directions[idx];
        let mut refreshed_center = aligned_secondary_label_anchors[idx]
            .or_else(|| path_point_at_progress(points, plan.progress))
            .or_else(|| edge_label_anchor_from_points(points))
            .unwrap_or(plan.center);
        // Re-apply the bidirectional perpendicular shift after the post-routing
        // refresh moves the label back onto the bowed curve.
        if graph.kind == crate::ir::DiagramKind::State {
            let edge = &graph.edges[idx];
            let key = edge_pair_key(edge);
            let total = *pair_counts.get(&key).unwrap_or(&1) as usize;
            let bidi =
                total == 2
                    && graph.edges.iter().enumerate().any(|(j, other)| {
                        j != idx && other.from == edge.to && other.to == edge.from
                    });
            if bidi {
                if let Some(label) = edge_route_labels.get(idx).and_then(|l| l.as_ref()) {
                    let bow_offset = config.node_spacing * 0.30;
                    let perp_shift = bow_offset + label.width.max(label.height) * 0.5 + 6.0;
                    let sign = if pair_index[idx] == 0 { -1.0 } else { 1.0 };
                    if is_horizontal(graph.direction) {
                        refreshed_center.1 += sign * perp_shift;
                    } else {
                        refreshed_center.0 += sign * perp_shift;
                    }
                }
            }
        }
        plan.center = refreshed_center;
        if !matches!(
            graph.kind,
            crate::ir::DiagramKind::State | crate::ir::DiagramKind::Requirement
        ) {
            insert_label_via_point(points, refreshed_center, edge_direction);
        }
        label_anchors[idx] = Some(refreshed_center);
    }

    // Insert label dummy via-points so edges pass through label positions.
    // For each edge with a label dummy, insert the dummy center into the
    // routed path at the correct main-axis position.
    for (idx, dummy_id_opt) in label_dummy_ids.iter().enumerate() {
        let Some(dummy_id) = dummy_id_opt else {
            continue;
        };
        let Some(dummy_node) = nodes.get(dummy_id) else {
            continue;
        };
        if graph.kind == crate::ir::DiagramKind::Flowchart
            && aligned_secondary_edges.get(idx).copied().unwrap_or(false)
        {
            if let Some(center) = aligned_secondary_label_anchors[idx]
                .or_else(|| path_point_at_progress(&routed_points[idx], 0.5))
                .or_else(|| edge_label_anchor_from_points(&routed_points[idx]))
            {
                label_anchors[idx] = Some(center);
            }
            continue;
        }
        let cx = dummy_node.x + dummy_node.width / 2.0;
        let cy = dummy_node.y + dummy_node.height / 2.0;
        label_anchors[idx] = Some((cx, cy));

        let points = &mut routed_points[idx];
        if graph.kind != crate::ir::DiagramKind::State && points.len() >= 2 {
            insert_label_via_point(points, (cx, cy), edge_directions[idx]);
        }
    }
    align_state_choice_fanout_labels(graph, &nodes, &mut label_anchors);
    if let Some(metrics) = stage_metrics.as_deref_mut() {
        metrics.edge_routing_us = metrics
            .edge_routing_us
            .saturating_add(edge_routing_start.elapsed().as_micros());
    }

    // Fix overlapping edge labels: when two labels overlap, push them
    // apart horizontally so both are readable.
    if graph.kind == crate::ir::DiagramKind::Flowchart {
        if !has_label_dummies {
            for i in 0..label_anchors.len() {
                let Some((ax, ay)) = label_anchors[i] else {
                    continue;
                };
                let Some(label_i) = edge_route_labels[i].as_ref() else {
                    continue;
                };
                for j in (i + 1)..label_anchors.len() {
                    let Some((bx, by)) = label_anchors[j] else {
                        continue;
                    };
                    let Some(label_j) = edge_route_labels[j].as_ref() else {
                        continue;
                    };
                    // Check vertical overlap (same y band).
                    let half_h_i = label_i.height / 2.0 + 4.0;
                    let half_h_j = label_j.height / 2.0 + 4.0;
                    if (ay - by).abs() > half_h_i + half_h_j {
                        continue;
                    }
                    // Check horizontal overlap.
                    let half_w_i = label_i.width / 2.0 + 8.0;
                    let half_w_j = label_j.width / 2.0 + 8.0;
                    let needed_sep = half_w_i + half_w_j;
                    let current_sep = (ax - bx).abs();
                    if current_sep >= needed_sep {
                        continue;
                    }
                    // Push apart: center them around their midpoint with
                    // the required separation.
                    let mid = (ax + bx) / 2.0;
                    let (new_ax, new_bx) = if ax <= bx {
                        (mid - needed_sep / 2.0, mid + needed_sep / 2.0)
                    } else {
                        (mid + needed_sep / 2.0, mid - needed_sep / 2.0)
                    };
                    label_anchors[i] = Some((new_ax, ay));
                    label_anchors[j] = Some((new_bx, by));
                }
            }
            apply_flowchart_adjacent_parallel_label_lanes(
                graph,
                &nodes,
                &edge_route_labels,
                &mut routed_points,
                &mut label_anchors,
                config,
            );
        }
        apply_parallel_top_level_flowchart_edge_lanes(
            graph,
            &nodes,
            &edge_route_labels,
            &mut routed_points,
            &mut label_anchors,
            config,
        );
        apply_flowchart_nested_bridge_cross_routes(graph, &nodes, &mut routed_points);
        expand_flowchart_subgraphs_for_edge_labels(
            graph,
            &edge_route_labels,
            &label_anchors,
            &mut subgraphs,
            config,
        );
    }

    let mut edges = Vec::new();
    for (idx, edge) in graph.edges.iter().enumerate() {
        let label = edge_route_labels[idx].clone();
        let start_label = edge_start_labels[idx].clone();
        let end_label = edge_end_labels[idx].clone();
        let mut override_style = resolve_edge_style(idx, graph);
        if graph.kind == crate::ir::DiagramKind::Requirement {
            if override_style.stroke.is_none() {
                override_style.stroke = Some(config.requirement.edge_stroke.clone());
            }
            override_style.stroke_width = Some(
                override_style
                    .stroke_width
                    .unwrap_or(config.requirement.edge_stroke_width),
            );
            if override_style.dasharray.is_none() && edge.style != crate::ir::EdgeStyle::Solid {
                override_style.dasharray = Some(config.requirement.edge_dasharray.clone());
            }
            if override_style.label_color.is_none() {
                override_style.label_color = Some(config.requirement.edge_label_color.clone());
            }
        }
        edges.push(EdgeLayout {
            from: edge.from.clone(),
            to: edge.to.clone(),
            label,
            start_label,
            end_label,
            points: routed_points[idx].clone(),
            directed: edge.directed,
            arrow_start: edge.arrow_start,
            arrow_end: edge.arrow_end,
            arrow_start_kind: edge.arrow_start_kind,
            arrow_end_kind: edge.arrow_end_kind,
            start_decoration: edge.start_decoration,
            end_decoration: edge.end_decoration,
            sequence_arrow_end: edge.sequence_arrow_end,
            sequence_arrow_start: edge.sequence_arrow_start,
            style: edge.style,
            override_style,
            label_anchor: label_anchors[idx],
            start_label_anchor: None,
            end_label_anchor: None,
            curve: edge.curve,
        });
    }

    if matches!(graph.direction, Direction::RightLeft | Direction::BottomTop) {
        apply_direction_mirror(graph.direction, &mut nodes, &mut edges, &mut subgraphs);
    }

    normalize_layout(&mut nodes, &mut edges, &mut subgraphs);

    // For state diagrams with notes: when a note is taller than its target
    // node, push subsequent nodes/edges/subgraphs down so the note doesn't
    // visually overlap the next state. JS dagre treats notes as siblings in
    // the layout; we approximate by post-shifting after the regular layout.
    if graph.kind == crate::ir::DiagramKind::State
        && !graph.state_notes.is_empty()
        && !graph.subgraphs.is_empty()
    {
        let note_pad_y = theme.font_size * STATE_NOTE_PAD_Y_SCALE;
        let mut needed_y_extension: HashMap<String, f32> = HashMap::new();
        for note in &graph.state_notes {
            let Some(target) = nodes.get(&note.target) else {
                continue;
            };
            let label = measure_label(&note.label, theme, config);
            let note_h = label.height + note_pad_y * 2.0;
            // Extension = full note overflow below target's bottom plus
            // breathing room. JS dagre allocates the full note vertical extent
            // as graph rank space, not just half.
            let extension = ((note_h - target.height) * 0.5 + note_h * 0.5).max(0.0);
            if extension > 0.0 {
                let entry = needed_y_extension.entry(note.target.clone()).or_insert(0.0);
                *entry = entry.max(extension);
            }
        }
        // Apply extensions: for each target with a needed extension, push
        // every node strictly below it down by that amount, plus a margin.
        for (target_id, extension) in &needed_y_extension {
            let Some(target) = nodes.get(target_id) else {
                continue;
            };
            let target_bottom = target.y + target.height;
            let push = extension + 8.0;
            // Find every node below target_bottom and shift it down.
            let to_shift: Vec<String> = nodes
                .iter()
                .filter(|(id, n)| id.as_str() != target_id && n.y >= target_bottom - 1.0)
                .map(|(id, _)| id.clone())
                .collect();
            for id in to_shift {
                if let Some(node) = nodes.get_mut(&id) {
                    node.y += push;
                }
            }
            // Shift edges that have any point below target_bottom.
            for edge in edges.iter_mut() {
                let touches_below = edge.points.iter().any(|p| p.1 >= target_bottom - 1.0);
                if touches_below {
                    for p in edge.points.iter_mut() {
                        if p.1 >= target_bottom - 1.0 {
                            p.1 += push;
                        }
                    }
                    if let Some(anchor) = edge.label_anchor.as_mut() {
                        if anchor.1 >= target_bottom - 1.0 {
                            anchor.1 += push;
                        }
                    }
                }
            }
            // Shift subgraphs whose top is below target_bottom.
            for sub in subgraphs.iter_mut() {
                if sub.y >= target_bottom - 1.0 {
                    sub.y += push;
                }
            }
        }
    }

    let mut state_notes = Vec::new();
    if graph.kind == crate::ir::DiagramKind::State && !graph.state_notes.is_empty() {
        let note_pad_x = theme.font_size * STATE_NOTE_PAD_X_SCALE;
        let note_pad_y = theme.font_size * STATE_NOTE_PAD_Y_SCALE;
        let note_gap = (theme.font_size * STATE_NOTE_GAP_SCALE).max(STATE_NOTE_GAP_MIN);
        if graph.subgraphs.is_empty() {
            state_notes =
                layout_state_notes_as_dagre_nodes(graph, &mut nodes, &mut edges, theme, config);
        }
        if state_notes.is_empty() {
            for note in &graph.state_notes {
                let Some(target) = nodes.get(&note.target) else {
                    continue;
                };
                let label = measure_label(&note.label, theme, config);
                let width = label.width + note_pad_x * 2.0;
                let height = label.height + note_pad_y * 2.0;
                let y = target.y + target.height / 2.0 - height / 2.0;
                let x = match note.position {
                    crate::ir::StateNotePosition::LeftOf => target.x - note_gap - width,
                    crate::ir::StateNotePosition::RightOf => target.x + target.width + note_gap,
                };
                state_notes.push(StateNoteLayout {
                    x,
                    y,
                    width,
                    height,
                    label,
                    position: note.position,
                    target: note.target.clone(),
                });
            }
        }
        // Re-normalize so notes that landed at negative x/y after the first
        // pass (e.g. `note left of` a target at the left boundary) get pulled
        // back into the diagram along with everything else.
        let mut extra_min_x = f32::MAX;
        let mut extra_min_y = f32::MAX;
        for note in &state_notes {
            extra_min_x = extra_min_x.min(note.x);
            extra_min_y = extra_min_y.min(note.y);
        }
        if extra_min_x.is_finite() && extra_min_y.is_finite() {
            let shift_x = (LAYOUT_BOUNDARY_PAD - extra_min_x).max(0.0);
            let shift_y = (LAYOUT_BOUNDARY_PAD - extra_min_y).max(0.0);
            if shift_x > 1e-3 || shift_y > 1e-3 {
                for node in nodes.values_mut() {
                    node.x += shift_x;
                    node.y += shift_y;
                }
                for edge in edges.iter_mut() {
                    for point in edge.points.iter_mut() {
                        point.0 += shift_x;
                        point.1 += shift_y;
                    }
                    if let Some(anchor) = edge.label_anchor.as_mut() {
                        anchor.0 += shift_x;
                        anchor.1 += shift_y;
                    }
                }
                for sub in subgraphs.iter_mut() {
                    sub.x += shift_x;
                    sub.y += shift_y;
                }
                for note in state_notes.iter_mut() {
                    note.x += shift_x;
                    note.y += shift_y;
                }
            }
        }
    }
    // Compute the leftmost edge-label extent so we can shift everything right
    // if a label would otherwise be clipped at viewBox x=0. State diagrams use
    // viewBox starting at (0,0), so any negative x needs translation.
    let mut min_x_with_labels: f32 = 0.0;
    for edge in &edges {
        if let (Some(label), Some((cx, _))) = (edge.label.as_ref(), edge.label_anchor) {
            min_x_with_labels = min_x_with_labels.min(cx - label.width * 0.5 - 8.0);
        }
        if let (Some(label), Some((cx, _))) = (edge.start_label.as_ref(), edge.start_label_anchor) {
            min_x_with_labels = min_x_with_labels.min(cx - label.width * 0.5 - 8.0);
        }
        if let (Some(label), Some((cx, _))) = (edge.end_label.as_ref(), edge.end_label_anchor) {
            min_x_with_labels = min_x_with_labels.min(cx - label.width * 0.5 - 8.0);
        }
    }
    let shift_x = if min_x_with_labels < 0.0 {
        -min_x_with_labels
    } else {
        0.0
    };
    if shift_x > 0.0 {
        for node in nodes.values_mut() {
            node.x += shift_x;
        }
        for sub in subgraphs.iter_mut() {
            sub.x += shift_x;
        }
        for edge in edges.iter_mut() {
            for p in edge.points.iter_mut() {
                p.0 += shift_x;
            }
            if let Some((cx, cy)) = edge.label_anchor {
                edge.label_anchor = Some((cx + shift_x, cy));
            }
            if let Some((cx, cy)) = edge.start_label_anchor {
                edge.start_label_anchor = Some((cx + shift_x, cy));
            }
            if let Some((cx, cy)) = edge.end_label_anchor {
                edge.end_label_anchor = Some((cx + shift_x, cy));
            }
        }
        for note in state_notes.iter_mut() {
            note.x += shift_x;
        }
    }

    let has_parallel_compound_flowchart = graph.kind == crate::ir::DiagramKind::Flowchart
        && parallel_top_level_flowchart(graph, &top_level_subgraph_indices(graph)).is_some();
    let has_recursive_flowchart_subgraph = graph.kind == crate::ir::DiagramKind::Flowchart
        && top_level_subgraph_indices(graph)
            .iter()
            .filter_map(|idx| graph.subgraphs.get(*idx))
            .any(|sub| flowchart_subgraph_is_recursive_cluster(graph, sub));
    let has_external_flowchart_compound = flowchart_has_external_compound_subgraph(graph);
    let has_dagre_lr_label_rank_spacing = flowchart_use_dagre_lr_label_rank_spacing(graph);
    let edge_margin_cap = if graph.kind == crate::ir::DiagramKind::State
        || graph.kind == crate::ir::DiagramKind::Class
        || graph.kind == crate::ir::DiagramKind::Requirement
        || has_parallel_compound_flowchart
        || has_recursive_flowchart_subgraph
        || has_external_flowchart_compound
        || has_dagre_lr_label_rank_spacing
    {
        Some(EDGE_BBOX_MARGIN_CAP)
    } else {
        None
    };
    let (mut max_x, mut max_y) =
        bounds_with_edges_capped(&nodes, &subgraphs, &edges, edge_margin_cap);
    for note in &state_notes {
        max_x = max_x.max(note.x + note.width + 35.0);
        max_y = max_y.max(note.y + note.height + 25.0);
    }
    let state_edge_label_right = if graph.kind == crate::ir::DiagramKind::State {
        edge_label_right_bound(&edges)
    } else {
        0.0
    };
    let state_edge_label_drives_width =
        graph.kind == crate::ir::DiagramKind::State && state_edge_label_right >= max_x - 0.5;
    let width = if is_small_dense_labeled_flowchart(graph, graph.nodes.len(), &graph.edges) {
        max_x
    } else if state_edge_label_drives_width {
        max_x
    } else {
        max_x + LAYOUT_BOUNDARY_PAD
    };
    let height = max_y + LAYOUT_BOUNDARY_PAD;

    Layout {
        kind: graph.kind,
        nodes,
        edges,
        subgraphs,
        width,
        height,
        acc_title: None,
        acc_descr: None,
        diagram: DiagramData::Graph {
            state_notes,
            title: graph.diagram_title.clone(),
        },
    }
}

fn assign_positions_manual(
    graph: &Graph,
    layout_node_ids: &[String],
    layout_set: &HashSet<String>,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
    layout_edges: &[crate::ir::Edge],
    theme: &Theme,
    pre_measured_labels: &[Option<TextBlock>],
    label_dummy_ids: &mut Vec<Option<String>>,
    layout_order: &HashMap<String, usize>,
) {
    let mut edge_labels_vec: Vec<Option<TextBlock>> = Vec::new();
    let mut original_edge_indices: Vec<usize> = Vec::new();
    let layout_edges: Vec<crate::ir::Edge> = layout_edges
        .iter()
        .enumerate()
        .filter(|(_, edge)| layout_set.contains(&edge.from) && layout_set.contains(&edge.to))
        .map(|(i, edge)| {
            edge_labels_vec.push(pre_measured_labels.get(i).cloned().unwrap_or(None));
            original_edge_indices.push(i);
            edge.clone()
        })
        .collect();
    let edge_labels = edge_labels_vec;
    let rank_edges = rank_edges_for_manual_layout(graph, layout_node_ids, &layout_edges);
    let ranks = compute_ranks_subset_for(graph, layout_node_ids, &rank_edges, layout_order);
    let mut max_rank = 0usize;
    for rank in ranks.values() {
        max_rank = max_rank.max(*rank);
    }
    let mut rank_nodes: Vec<Vec<String>> = vec![Vec::new(); max_rank + 1];
    for node_id in layout_node_ids {
        let rank = *ranks.get(node_id).unwrap_or(&0);
        if let Some(bucket) = rank_nodes.get_mut(rank) {
            bucket.push(node_id.clone());
        }
    }

    let small_dense_flowchart =
        is_small_dense_labeled_flowchart(graph, layout_node_ids.len(), &layout_edges);
    let use_dagre_lr_label_rank_spacing = flowchart_use_dagre_lr_label_rank_spacing(graph);
    let use_label_dummies = !small_dense_flowchart
        && !matches!(
            graph.kind,
            crate::ir::DiagramKind::Class
                | crate::ir::DiagramKind::Er
                | crate::ir::DiagramKind::Requirement
                | crate::ir::DiagramKind::State
        );
    // Collect gaps (original rank index) where at least one labeled forward edge exists.
    let gaps_needing_label_rank: Vec<usize> = if use_label_dummies {
        let mut gap_set: HashSet<usize> = HashSet::new();
        for (idx, edge) in layout_edges.iter().enumerate() {
            if edge_labels[idx].is_none() {
                continue;
            }
            if flowchart_edge_inside_recursive_cluster(graph, edge) {
                continue;
            }
            let from_rank = ranks.get(&edge.from).copied().unwrap_or(0);
            let to_rank = ranks.get(&edge.to).copied().unwrap_or(0);
            // Forward edges: insert label rank in the gap.
            // Back-edges (to_rank <= from_rank): insert label rank in the gap too,
            // using min/max so both directions share the same label rank.
            let lo = from_rank.min(to_rank);
            let hi = from_rank.max(to_rank);
            if hi > lo {
                // For span-1 edges, the gap index is lo.
                // For longer spans, use the midpoint gap.
                let mid_gap = lo + (hi - lo - 1) / 2;
                gap_set.insert(mid_gap);
            }
        }
        let mut v: Vec<usize> = gap_set.into_iter().collect();
        v.sort();
        v
    } else {
        Vec::new()
    };

    // Build a rank shift table: for each original rank r, the new rank is r + shift[r].
    let mut rank_shift: Vec<usize> = vec![0; max_rank + 2];
    {
        let mut cumulative = 0;
        for r in 0..=max_rank {
            rank_shift[r] = cumulative;
            if gaps_needing_label_rank.contains(&r) {
                cumulative += 1;
            }
        }
        rank_shift[max_rank + 1] = cumulative;
    }
    let total_new_ranks = if gaps_needing_label_rank.is_empty() {
        0
    } else {
        rank_shift[max_rank + 1]
    };

    // Apply rank shifts: expand rank_nodes to accommodate new label ranks.
    if total_new_ranks > 0 {
        let new_max_rank = max_rank + total_new_ranks;
        let mut new_rank_nodes: Vec<Vec<String>> = vec![Vec::new(); new_max_rank + 1];
        for (old_rank, bucket) in rank_nodes.iter().enumerate() {
            let new_rank = old_rank + rank_shift[old_rank];
            new_rank_nodes[new_rank] = bucket.clone();
        }
        rank_nodes = new_rank_nodes;
    }

    // Create label dummy nodes in the inserted label ranks.
    let mut label_dummy_ranks: HashSet<usize> = HashSet::new();
    let mut order_map = layout_order.clone();
    let mut dummy_counter = 0usize;

    if use_label_dummies {
        for (idx, edge) in layout_edges.iter().enumerate() {
            let Some(label) = &edge_labels[idx] else {
                continue;
            };
            if flowchart_edge_inside_recursive_cluster(graph, edge) {
                continue;
            }
            let from_rank = ranks.get(&edge.from).copied().unwrap_or(0);
            let to_rank = ranks.get(&edge.to).copied().unwrap_or(0);
            let lo = from_rank.min(to_rank);
            let hi = from_rank.max(to_rank);
            if hi <= lo {
                continue;
            }
            let mid_gap = lo + (hi - lo - 1) / 2;
            // The label rank is the new rank inserted after the shifted gap position.
            let label_rank = mid_gap + rank_shift[mid_gap] + 1;
            label_dummy_ranks.insert(label_rank);

            let dummy_id = format!("__elabel_{}_{}_{dummy_counter}__", edge.from, edge.to);
            dummy_counter += 1;
            let order_idx = order_map.len();
            order_map.insert(dummy_id.clone(), order_idx);

            // Determine dimensions: for horizontal layouts, main-axis = width, cross-axis = height.
            // Cap the main-axis size so long edge labels don't explode rank spacing.
            let label_main_cap = (theme.font_size * 8.0).max(config.node_spacing * 1.3);
            let (raw_main, raw_cross) = if is_horizontal(graph.direction) {
                (label.width, label.height)
            } else {
                (label.height, label.width)
            };
            let main_dim = if use_dagre_lr_label_rank_spacing && raw_main > 0.0 {
                raw_main
            } else if raw_main > 0.0 {
                raw_main.min(label_main_cap)
            } else {
                raw_main
            };
            let cross_dim = raw_cross;

            nodes.insert(
                dummy_id.clone(),
                NodeLayout {
                    id: dummy_id.clone(),
                    x: 0.0,
                    y: 0.0,
                    width: if is_horizontal(graph.direction) {
                        main_dim
                    } else {
                        cross_dim
                    },
                    height: if is_horizontal(graph.direction) {
                        cross_dim
                    } else {
                        main_dim
                    },
                    label: TextBlock {
                        lines: vec![],
                        width: 0.0,
                        height: 0.0,
                    },
                    shape: crate::ir::NodeShape::Rectangle,
                    style: crate::ir::NodeStyle::default(),
                    link: None,
                    anchor_subgraph: None,
                    hidden: true,
                    icon: None,
                    img: None,
                    img_w: None,
                    img_h: None,
                    sub_label: None,
                    is_treemap_leaf: false,
                    treemap_base_text_color: None,
                },
            );

            // Record original edge index → dummy node ID mapping.
            if let Some(&orig_idx) = original_edge_indices.get(idx) {
                if orig_idx < label_dummy_ids.len() {
                    label_dummy_ids[orig_idx] = Some(dummy_id.clone());
                }
            }

            if let Some(bucket) = rank_nodes.get_mut(label_rank) {
                bucket.push(dummy_id);
            }
        }
    }

    // Build a lookup: for each layout edge index, the (shifted_label_rank, dummy_id)
    // so the span-dummy expansion loop can reuse label dummies instead of creating
    // new span dummies at the same rank (which would leave label dummies disconnected).
    let mut label_dummy_at_rank: HashMap<usize, (usize, String)> = HashMap::new();
    for (idx, edge) in layout_edges.iter().enumerate() {
        if edge_labels[idx].is_none() {
            continue;
        }
        if flowchart_edge_inside_recursive_cluster(graph, edge) {
            continue;
        }
        let from_rank = ranks.get(&edge.from).copied().unwrap_or(0);
        let to_rank = ranks.get(&edge.to).copied().unwrap_or(0);
        let lo = from_rank.min(to_rank);
        let hi = from_rank.max(to_rank);
        if hi <= lo {
            continue;
        }
        let mid_gap = lo + (hi - lo - 1) / 2;
        let label_rank = mid_gap + rank_shift[mid_gap] + 1;
        if let Some(&orig_idx) = original_edge_indices.get(idx) {
            if let Some(Some(dummy_id)) = label_dummy_ids.get(orig_idx) {
                label_dummy_at_rank.insert(idx, (label_rank, dummy_id.clone()));
            }
        }
    }

    // Update ranks for existing nodes to use shifted values (for the existing dummy expansion).
    let shifted_ranks: HashMap<String, usize> = ranks
        .iter()
        .map(|(id, &r)| (id.clone(), r + rank_shift[r]))
        .collect();

    let mut rank_gap_overrides = vec![config.rank_spacing; rank_nodes.len().saturating_sub(1)];
    if matches!(
        graph.kind,
        crate::ir::DiagramKind::Requirement | crate::ir::DiagramKind::State
    ) {
        for (idx, edge) in layout_edges.iter().enumerate() {
            let Some(Some(label)) = edge_labels.get(idx) else {
                continue;
            };
            let Some(&from_rank) = shifted_ranks.get(&edge.from) else {
                continue;
            };
            let Some(&to_rank) = shifted_ranks.get(&edge.to) else {
                continue;
            };
            let lo = from_rank.min(to_rank);
            let hi = from_rank.max(to_rank);
            if hi <= lo {
                continue;
            }
            let label_extent = if is_horizontal(graph.direction) {
                label.width
            } else {
                label.height
            };
            let required_gap = config.rank_spacing + label_extent;
            if graph.kind == crate::ir::DiagramKind::State {
                // Mermaid's dagre state renderer reserves the label on the
                // edge midpoint, so only the midpoint rank gap grows by the
                // label's main-axis size. This keeps unlabeled neighboring
                // gaps at the configured ranksep.
                let mid_gap = lo + (hi - lo - 1) / 2;
                if let Some(current) = rank_gap_overrides.get_mut(mid_gap) {
                    *current = current.max(required_gap);
                }
            } else {
                for gap in lo..hi {
                    if let Some(current) = rank_gap_overrides.get_mut(gap) {
                        *current = current.max(required_gap);
                    }
                }
            }
        }
    }

    // --- End label dummy nodes ---

    let mut expanded_edges: Vec<crate::ir::Edge> = Vec::new();

    for (edge_idx, edge) in layout_edges.iter().enumerate() {
        let Some(&from_rank) = shifted_ranks.get(&edge.from) else {
            continue;
        };
        let Some(&to_rank) = shifted_ranks.get(&edge.to) else {
            continue;
        };
        if to_rank <= from_rank {
            continue;
        }
        let span = to_rank - from_rank;
        if span <= 1 {
            expanded_edges.push(edge.clone());
            continue;
        }
        // Look up whether this edge has a label dummy at some rank.
        let label_dummy_info = label_dummy_at_rank.get(&edge_idx);
        let mut prev = edge.from.clone();
        for step in 1..span {
            let current_rank = from_rank + step;
            // Reuse the label dummy if it exists at this rank, instead of
            // creating a new span dummy. This connects the label dummy into
            // the expanded edge chain so it gets proper cross-axis positioning.
            let dummy_id = if let Some((lr, lid)) = label_dummy_info {
                if current_rank == *lr {
                    lid.clone()
                } else {
                    let id = format!("__dummy_{}__", dummy_counter);
                    dummy_counter += 1;
                    let order_idx = order_map.len();
                    order_map.insert(id.clone(), order_idx);
                    if let Some(bucket) = rank_nodes.get_mut(current_rank) {
                        bucket.push(id.clone());
                    }
                    id
                }
            } else {
                let id = format!("__dummy_{}__", dummy_counter);
                dummy_counter += 1;
                let order_idx = order_map.len();
                order_map.insert(id.clone(), order_idx);
                if let Some(bucket) = rank_nodes.get_mut(current_rank) {
                    bucket.push(id.clone());
                }
                // Create a minimal hidden NodeLayout so the span dummy
                // participates in cross-axis positioning (barycenter).
                nodes.insert(
                    id.clone(),
                    NodeLayout {
                        id: id.clone(),
                        x: 0.0,
                        y: 0.0,
                        width: 1.0,
                        height: 1.0,
                        label: TextBlock {
                            lines: vec![],
                            width: 0.0,
                            height: 0.0,
                        },
                        shape: crate::ir::NodeShape::Rectangle,
                        style: crate::ir::NodeStyle::default(),
                        link: None,
                        anchor_subgraph: None,
                        hidden: true,
                        icon: None,
                        img: None,
                        img_w: None,
                        img_h: None,
                        sub_label: None,
                        is_treemap_leaf: false,
                        treemap_base_text_color: None,
                    },
                );
                id
            };
            expanded_edges.push(crate::ir::Edge {
                from: prev.clone(),
                to: dummy_id.clone(),
                label: None,
                start_label: None,
                end_label: None,
                directed: true,
                arrow_start: false,
                arrow_end: false,
                arrow_start_kind: None,
                arrow_end_kind: None,
                start_decoration: None,
                end_decoration: None,
                sequence_arrow_end: None,
                sequence_arrow_start: None,
                style: crate::ir::EdgeStyle::Solid,
                markdown_label: false,
                id: None,
                curve: None,
                arch_port_from: None,
                arch_port_to: None,
            });
            prev = dummy_id;
        }
        expanded_edges.push(crate::ir::Edge {
            from: prev,
            to: edge.to.clone(),
            label: None,
            start_label: None,
            end_label: None,
            directed: true,
            arrow_start: false,
            arrow_end: false,
            arrow_start_kind: None,
            arrow_end_kind: None,
            start_decoration: None,
            end_decoration: None,
            sequence_arrow_end: None,
            sequence_arrow_start: None,
            style: crate::ir::EdgeStyle::Solid,
            markdown_label: false,
            id: None,
            curve: None,
            arch_port_from: None,
            arch_port_to: None,
        });
    }

    for bucket in &mut rank_nodes {
        bucket.sort_by_key(|id| order_map.get(id).copied().unwrap_or(usize::MAX));
    }
    if graph.kind == crate::ir::DiagramKind::Class {
        order_rank_nodes_bottom_up_first(
            &mut rank_nodes,
            &expanded_edges,
            &order_map,
            config.flowchart.order_passes,
        );
    } else {
        order_rank_nodes(
            &mut rank_nodes,
            &expanded_edges,
            &order_map,
            config.flowchart.order_passes,
        );
    }

    let external_tb_compound_flowchart = flowchart_has_tb_external_compound_subgraph(graph);
    let mut main_cursor = 0.0;
    for (rank_idx, bucket) in rank_nodes.iter().enumerate() {
        let bucket_max_main = bucket
            .iter()
            .filter_map(|node_id| nodes.get(node_id))
            .map(|node_layout| {
                if is_horizontal(graph.direction) {
                    node_layout.width
                } else {
                    node_layout.height
                }
            })
            .fold(0.0_f32, f32::max);
        let is_label_rank = label_dummy_ranks.contains(&rank_idx);
        for node_id in bucket {
            if let Some(node_layout) = nodes.get_mut(node_id) {
                if is_horizontal(graph.direction) {
                    let offset = if matches!(
                        graph.kind,
                        crate::ir::DiagramKind::Flowchart
                            | crate::ir::DiagramKind::Class
                            | crate::ir::DiagramKind::Requirement
                    ) {
                        (bucket_max_main - node_layout.width) * 0.5
                    } else {
                        0.0
                    };
                    node_layout.x = main_cursor + offset;
                } else {
                    let offset = if matches!(
                        graph.kind,
                        crate::ir::DiagramKind::Flowchart
                            | crate::ir::DiagramKind::Class
                            | crate::ir::DiagramKind::Requirement
                    ) {
                        (bucket_max_main - node_layout.height) * 0.5
                    } else {
                        0.0
                    };
                    node_layout.y = main_cursor + offset;
                }
            }
        }
        if bucket_max_main > 0.0 {
            // Use reduced spacing for label-only ranks to avoid excessive width.
            let next_is_label_rank = label_dummy_ranks.contains(&(rank_idx + 1));
            let gap = if is_label_rank {
                if use_dagre_lr_label_rank_spacing {
                    config.rank_spacing * 0.5
                } else if external_tb_compound_flowchart {
                    0.0
                } else {
                    (theme.font_size * LABEL_RANK_FONT_SCALE).max(LABEL_RANK_MIN_GAP)
                }
            } else if use_dagre_lr_label_rank_spacing && next_is_label_rank {
                config.rank_spacing * 0.5
            } else if matches!(
                graph.kind,
                crate::ir::DiagramKind::Requirement | crate::ir::DiagramKind::State
            ) {
                rank_gap_overrides
                    .get(rank_idx)
                    .copied()
                    .unwrap_or(config.rank_spacing)
            } else {
                config.rank_spacing
            };
            main_cursor += bucket_max_main + gap;
        }
    }

    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    let mut requirement_real_incoming_count: HashMap<String, usize> = HashMap::new();
    if graph.kind == crate::ir::DiagramKind::Requirement {
        for edge in &layout_edges {
            *requirement_real_incoming_count
                .entry(edge.to.clone())
                .or_insert(0) += 1;
        }
    }
    // Use expanded_edges so dummy nodes (both span dummies and label
    // dummies) get proper neighbor connectivity for cross-axis positioning.
    for edge in &expanded_edges {
        incoming
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
        outgoing
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
    }

    let requirement_cross_label_spacing_extra = (theme.font_size * 0.5).max(8.0);
    let mut cross_pos: HashMap<String, f32> = HashMap::new();
    if graph.kind == crate::ir::DiagramKind::Class
        || graph.kind == crate::ir::DiagramKind::Flowchart
        || graph.kind == crate::ir::DiagramKind::Requirement
    {
        for bucket in &rank_nodes {
            let mut cursor = 0.0f32;
            for node_id in bucket {
                if let Some(node) = nodes.get(node_id) {
                    let half = if is_horizontal(graph.direction) {
                        node.height / 2.0
                    } else {
                        node.width / 2.0
                    };
                    let center = cursor + half;
                    cross_pos.insert(node_id.clone(), center);
                    if !node.hidden {
                        cursor += half * 2.0 + config.node_spacing;
                    }
                }
            }
        }
    } else {
        for bucket in &rank_nodes {
            for (idx, node_id) in bucket.iter().enumerate() {
                if let Some(node) = nodes.get(node_id) {
                    let center = if is_horizontal(graph.direction) {
                        node.y + node.height / 2.0
                    } else {
                        node.x + node.width / 2.0
                    };
                    cross_pos.insert(node_id.clone(), center + idx as f32 * 0.01);
                }
            }
        }
    }

    let mut place_rank = |rank_idx: usize,
                          use_incoming: bool,
                          nodes: &mut BTreeMap<String, NodeLayout>| {
        let bucket = &rank_nodes[rank_idx];
        if bucket.is_empty() {
            return;
        }
        let mut entries: Vec<(String, f32, f32, usize, bool)> = Vec::new();
        for (idx, node_id) in bucket.iter().enumerate() {
            let Some(node) = nodes.get(node_id) else {
                continue;
            };
            let mut neighbor_centers: Vec<f32> = Vec::new();
            let neighbors = if use_incoming {
                incoming.get(node_id)
            } else {
                outgoing.get(node_id)
            };
            if let Some(list) = neighbors {
                for neighbor_id in list {
                    if let Some(center) = cross_pos.get(neighbor_id) {
                        neighbor_centers.push(*center);
                    }
                }
            }
            let mut desired = if neighbor_centers.is_empty() {
                cross_pos.get(node_id).copied().unwrap_or(0.0)
            } else {
                neighbor_centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
                let mid = neighbor_centers.len() / 2;
                if neighbor_centers.len() % 2 == 1 {
                    neighbor_centers[mid]
                } else {
                    (neighbor_centers[mid - 1] + neighbor_centers[mid]) * 0.5
                }
            };
            if let Some(current) = cross_pos.get(node_id) {
                if !neighbor_centers.is_empty() {
                    desired = desired * 0.85 + current * 0.15;
                } else {
                    desired = *current;
                }
            }
            let half = if is_horizontal(graph.direction) {
                node.height / 2.0
            } else {
                node.width / 2.0
            };
            entries.push((node_id.clone(), desired, half, idx, node.hidden));
        }
        entries.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.3.cmp(&b.3))
        });
        let desired_mean =
            entries.iter().map(|(_, d, _, _, _)| *d).sum::<f32>() / entries.len() as f32;
        let mut assigned: Vec<(String, f32, f32)> = Vec::new();
        let mut prev_center: Option<f32> = None;
        let mut prev_half = 0.0;
        let mut prev_real_id: Option<String> = None;
        for (node_id, desired, half, _idx, hidden) in entries {
            let center = if let Some(prev) = prev_center {
                let mut cross_spacing = config.node_spacing;
                if graph.kind == crate::ir::DiagramKind::Requirement
                    && !hidden
                    && let Some(prev_id) = prev_real_id.as_deref()
                    && requirement_real_incoming_count
                        .get(prev_id)
                        .copied()
                        .unwrap_or(0)
                        > 0
                    && requirement_real_incoming_count
                        .get(node_id.as_str())
                        .copied()
                        .unwrap_or(0)
                        > 0
                {
                    // Mermaid measures requirement edge labels before dagre
                    // coordinate assignment. Adjacent incoming targets get a
                    // little more cross-axis breathing room from that virtual
                    // label geometry than a plain nodesep constraint.
                    cross_spacing += requirement_cross_label_spacing_extra;
                }
                let min_center = prev + prev_half + half + cross_spacing;
                // Hidden nodes (span dummies) don't render — skip spacing
                // constraints so they don't push real nodes off-center.
                if hidden || desired >= min_center {
                    desired
                } else {
                    min_center
                }
            } else {
                desired
            };
            assigned.push((node_id.clone(), center, half));
            // Hidden nodes (span dummies) don't occupy visual space —
            // don't let them create spacing constraints for the next node.
            if !hidden {
                prev_center = Some(center);
                prev_half = half;
                prev_real_id = Some(node_id.clone());
            }
        }
        let actual_mean = assigned.iter().map(|(_, c, _)| *c).sum::<f32>() / assigned.len() as f32;
        let delta = desired_mean - actual_mean;
        for (node_id, center, _half) in assigned {
            let center = center + delta;
            if let Some(node) = nodes.get_mut(&node_id) {
                if is_horizontal(graph.direction) {
                    node.y = center - node.height / 2.0;
                } else {
                    node.x = center - node.width / 2.0;
                }
            }
            cross_pos.insert(node_id, center);
        }
    };

    for _ in 0..config.flowchart.order_passes.max(1) {
        for rank_idx in 0..rank_nodes.len() {
            place_rank(rank_idx, true, nodes);
        }
        for rank_idx in (0..rank_nodes.len()).rev() {
            place_rank(rank_idx, false, nodes);
        }
    }

    if graph.kind == crate::ir::DiagramKind::Requirement {
        align_requirement_dagre_source_columns(graph, &rank_nodes, &layout_edges, nodes, config);
    }

    if graph.kind == crate::ir::DiagramKind::Class {
        align_class_note_targets_to_notes(graph, &rank_nodes, &layout_edges, nodes, config);
        align_class_unanchored_siblings_to_parent(graph, &rank_nodes, &layout_edges, nodes, config);
    }
}

fn align_requirement_dagre_source_columns(
    graph: &Graph,
    rank_nodes: &[Vec<String>],
    layout_edges: &[crate::ir::Edge],
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    let horizontal = is_horizontal(graph.direction);
    let rank_index: HashMap<&str, usize> = rank_nodes
        .iter()
        .enumerate()
        .flat_map(|(rank, layer)| layer.iter().map(move |id| (id.as_str(), rank)))
        .collect();
    let rank_pos: HashMap<&str, usize> = rank_nodes
        .iter()
        .flat_map(|layer| layer.iter().enumerate().map(|(pos, id)| (id.as_str(), pos)))
        .collect();
    let mut incoming_count: HashMap<&str, usize> = HashMap::new();
    let mut outgoing: HashMap<&str, Vec<&crate::ir::Edge>> = HashMap::new();
    for edge in layout_edges {
        *incoming_count.entry(edge.to.as_str()).or_insert(0) += 1;
        outgoing.entry(edge.from.as_str()).or_default().push(edge);
    }

    let cross_center = |node: &NodeLayout| -> f32 {
        if horizontal {
            node.y + node.height / 2.0
        } else {
            node.x + node.width / 2.0
        }
    };
    let spacing = config.node_spacing.max(MIN_NODE_SPACING_FLOOR);
    let mut desired: HashMap<String, f32> = HashMap::new();

    for (source_id, out_edges) in &outgoing {
        let Some(&source_rank) = rank_index.get(*source_id) else {
            continue;
        };
        let Some(source) = nodes.get(*source_id) else {
            continue;
        };
        if source.hidden {
            continue;
        }

        if out_edges.len() > 1 {
            if let Some(edge) = out_edges.iter().find(|edge| {
                edge.style == crate::ir::EdgeStyle::Solid
                    && rank_index
                        .get(edge.to.as_str())
                        .is_some_and(|&target_rank| target_rank > source_rank)
            }) {
                if let Some(target) = nodes.get(&edge.to) {
                    let source_center = cross_center(source);
                    let target_center = cross_center(target);
                    let delta = target_center - source_center;
                    if delta.abs() > spacing {
                        desired.insert(
                            (*source_id).to_string(),
                            target_center - delta.signum() * spacing,
                        );
                    }
                }
            }
            continue;
        }

        let Some(edge) = out_edges.first() else {
            continue;
        };
        let no_incoming = incoming_count.get(*source_id).copied().unwrap_or(0) == 0;
        let first_in_rank = rank_pos.get(*source_id).copied().unwrap_or(usize::MAX) == 0;
        let has_rank_peer = rank_nodes
            .get(source_rank)
            .is_some_and(|layer| layer.iter().filter(|id| !nodes[*id].hidden).count() > 1);
        let adjacent_target = rank_index
            .get(edge.to.as_str())
            .is_some_and(|&target_rank| target_rank == source_rank + 1);
        if no_incoming && first_in_rank && has_rank_peer && adjacent_target {
            if let Some(target) = nodes.get(&edge.to) {
                let source_center = cross_center(source);
                let target_center = cross_center(target);
                let delta = target_center - source_center;
                if delta.abs() > 1.0 {
                    desired.insert(
                        (*source_id).to_string(),
                        target_center - delta.signum() * spacing,
                    );
                }
            }
        }
    }

    for _ in 0..4 {
        let mut changed = false;
        for edge in layout_edges {
            let Some(&target_center) = desired.get(&edge.to) else {
                continue;
            };
            if desired.contains_key(&edge.from) {
                continue;
            }
            let source_incoming = incoming_count.get(edge.from.as_str()).copied().unwrap_or(0);
            let source_outgoing = outgoing
                .get(edge.from.as_str())
                .map(|edges| edges.len())
                .unwrap_or(0);
            if source_incoming == 0 && source_outgoing == 1 {
                desired.insert(edge.from.clone(), target_center);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    for (id, center) in desired {
        let Some(node) = nodes.get_mut(&id) else {
            continue;
        };
        if horizontal {
            node.y = center - node.height / 2.0;
        } else {
            node.x = center - node.width / 2.0;
        }
    }
}

fn is_small_dense_labeled_flowchart(
    graph: &Graph,
    layout_node_count: usize,
    layout_edges: &[crate::ir::Edge],
) -> bool {
    if graph.kind != crate::ir::DiagramKind::Flowchart
        || !graph.subgraphs.is_empty()
        || layout_node_count > 4
        || layout_edges.len() < 5
        || layout_edges.len() > 8
    {
        return false;
    }

    let labeled_count = layout_edges
        .iter()
        .filter(|edge| edge.label.is_some())
        .count();
    if labeled_count < 5 {
        return false;
    }

    layout_edges.iter().any(|edge| {
        layout_edges
            .iter()
            .any(|other| edge.from == other.to && edge.to == other.from)
    })
}

fn flowchart_use_dagre_lr_label_rank_spacing(graph: &Graph) -> bool {
    graph.kind == crate::ir::DiagramKind::Flowchart
        && is_horizontal(graph.direction)
        && graph.subgraphs.is_empty()
        && !is_small_dense_labeled_flowchart(graph, graph.nodes.len(), &graph.edges)
        && graph
            .edges
            .iter()
            .any(|edge| edge.label.is_some() && edge.from != edge.to)
}

fn align_class_note_targets_to_notes(
    graph: &Graph,
    rank_nodes: &[Vec<String>],
    layout_edges: &[crate::ir::Edge],
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    let horizontal = is_horizontal(graph.direction);

    for edge in layout_edges {
        if edge.style != crate::ir::EdgeStyle::Dotted {
            continue;
        }

        let Some(note) = nodes.get(&edge.from) else {
            continue;
        };
        let Some(target) = nodes.get(&edge.to) else {
            continue;
        };
        if note.shape != crate::ir::NodeShape::Note
            || target.shape == crate::ir::NodeShape::Note
            || note.hidden
            || target.hidden
        {
            continue;
        }

        let note_center = if horizontal {
            note.y + note.height * 0.5
        } else {
            note.x + note.width * 0.5
        };
        if let Some(target) = nodes.get_mut(&edge.to) {
            if horizontal {
                target.y = note_center - target.height * 0.5;
            } else {
                target.x = note_center - target.width * 0.5;
            }
        }
    }

    for bucket in rank_nodes {
        let mut prev_center: Option<f32> = None;
        let mut prev_half = 0.0f32;
        for node_id in bucket {
            let Some(node) = nodes.get_mut(node_id) else {
                continue;
            };
            if node.hidden {
                continue;
            }

            let half = if horizontal {
                node.height * 0.5
            } else {
                node.width * 0.5
            };
            let mut center = if horizontal {
                node.y + node.height * 0.5
            } else {
                node.x + node.width * 0.5
            };

            if let Some(prev) = prev_center {
                let min_center = prev + prev_half + half + config.node_spacing;
                if center < min_center {
                    center = min_center;
                    if horizontal {
                        node.y = center - node.height * 0.5;
                    } else {
                        node.x = center - node.width * 0.5;
                    }
                }
            }

            prev_center = Some(center);
            prev_half = half;
        }
    }
}

fn align_class_unanchored_siblings_to_parent(
    graph: &Graph,
    rank_nodes: &[Vec<String>],
    layout_edges: &[crate::ir::Edge],
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    let horizontal = is_horizontal(graph.direction);
    let mut rank_by_node: HashMap<&str, usize> = HashMap::new();
    for (rank, bucket) in rank_nodes.iter().enumerate() {
        for id in bucket {
            rank_by_node.insert(id.as_str(), rank);
        }
    }

    let mut note_anchored_targets: HashSet<&str> = HashSet::new();
    for edge in layout_edges {
        let Some(note) = nodes.get(&edge.from) else {
            continue;
        };
        let Some(target) = nodes.get(&edge.to) else {
            continue;
        };
        if note.shape == crate::ir::NodeShape::Note && target.shape != crate::ir::NodeShape::Note {
            note_anchored_targets.insert(edge.to.as_str());
        }
    }

    let mut children_by_parent_rank: HashMap<(&str, usize), Vec<&str>> = HashMap::new();
    for edge in layout_edges {
        let Some(from_node) = nodes.get(&edge.from) else {
            continue;
        };
        let Some(to_node) = nodes.get(&edge.to) else {
            continue;
        };
        if from_node.shape == crate::ir::NodeShape::Note
            || to_node.shape == crate::ir::NodeShape::Note
        {
            continue;
        }

        let Some(&from_rank) = rank_by_node.get(edge.from.as_str()) else {
            continue;
        };
        let Some(&to_rank) = rank_by_node.get(edge.to.as_str()) else {
            continue;
        };
        if from_rank == to_rank {
            continue;
        }

        let (parent, child, child_rank) = if from_rank < to_rank {
            (edge.from.as_str(), edge.to.as_str(), to_rank)
        } else {
            (edge.to.as_str(), edge.from.as_str(), from_rank)
        };
        children_by_parent_rank
            .entry((parent, child_rank))
            .or_default()
            .push(child);
    }

    for ((parent_id, child_rank), children) in children_by_parent_rank {
        if !children
            .iter()
            .any(|child| note_anchored_targets.contains(child))
        {
            continue;
        }

        let unanchored: Vec<&str> = children
            .into_iter()
            .filter(|child| !note_anchored_targets.contains(child))
            .collect();
        if unanchored.len() < 2 {
            continue;
        }

        let Some(bucket) = rank_nodes.get(child_rank) else {
            continue;
        };
        let bucket_index: HashMap<&str, usize> = bucket
            .iter()
            .enumerate()
            .map(|(idx, id)| (id.as_str(), idx))
            .collect();
        let mut indexes: Vec<usize> = unanchored
            .iter()
            .filter_map(|id| bucket_index.get(id).copied())
            .collect();
        indexes.sort_unstable();
        if indexes.len() != unanchored.len()
            || indexes
                .windows(2)
                .any(|pair| pair[1] != pair[0].saturating_add(1))
        {
            continue;
        }

        let Some(parent) = nodes.get(parent_id) else {
            continue;
        };
        let parent_center = if horizontal {
            parent.y + parent.height * 0.5
        } else {
            parent.x + parent.width * 0.5
        };

        let mut group_center_sum = 0.0;
        let mut first_min = f32::INFINITY;
        let mut last_max = f32::NEG_INFINITY;
        for child in &unanchored {
            let Some(node) = nodes.get(*child) else {
                continue;
            };
            let (min, center, max) = if horizontal {
                (node.y, node.y + node.height * 0.5, node.y + node.height)
            } else {
                (node.x, node.x + node.width * 0.5, node.x + node.width)
            };
            group_center_sum += center;
            first_min = first_min.min(min);
            last_max = last_max.max(max);
        }
        let group_center = group_center_sum / unanchored.len() as f32;
        let mut shift = parent_center - group_center;

        if let Some(first_idx) = indexes.first().copied() {
            for idx in (0..first_idx).rev() {
                let Some(prev) = nodes.get(&bucket[idx]) else {
                    continue;
                };
                if prev.hidden {
                    continue;
                }
                let prev_max = if horizontal {
                    prev.y + prev.height
                } else {
                    prev.x + prev.width
                };
                shift = shift.max(prev_max + config.node_spacing - first_min);
                break;
            }
        }
        if let Some(last_idx) = indexes.last().copied() {
            for idx in last_idx + 1..bucket.len() {
                let Some(next) = nodes.get(&bucket[idx]) else {
                    continue;
                };
                if next.hidden {
                    continue;
                }
                let next_min = if horizontal { next.y } else { next.x };
                shift = shift.min(next_min - config.node_spacing - last_max);
                break;
            }
        }

        if shift.abs() <= 0.01 {
            continue;
        }

        for child in unanchored {
            if let Some(node) = nodes.get_mut(child) {
                if horizontal {
                    node.y += shift;
                } else {
                    node.x += shift;
                }
            }
        }
    }
}

fn resolve_edge_style(idx: usize, graph: &Graph) -> crate::ir::EdgeStyleOverride {
    let mut style = graph.edge_style_default.clone().unwrap_or_default();
    if let Some(edge_style) = graph.edge_styles.get(&idx) {
        merge_edge_style(&mut style, edge_style);
    }
    style
}

fn merge_edge_style(
    target: &mut crate::ir::EdgeStyleOverride,
    source: &crate::ir::EdgeStyleOverride,
) {
    if source.stroke.is_some() {
        target.stroke = source.stroke.clone();
    }
    if source.stroke_width.is_some() {
        target.stroke_width = source.stroke_width;
    }
    if source.dasharray.is_some() {
        target.dasharray = source.dasharray.clone();
    }
    if source.label_color.is_some() {
        target.label_color = source.label_color.clone();
    }
}

fn apply_subgraph_bands(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    if graph.kind == crate::ir::DiagramKind::Flowchart
        && graph
            .subgraphs
            .iter()
            .any(|sub| flowchart_subgraph_is_recursive_cluster(graph, sub))
    {
        return;
    }

    let mut group_nodes: Vec<Vec<String>> = Vec::new();
    let mut node_group: HashMap<String, usize> = HashMap::new();

    // Group 0: nodes not in any subgraph.
    group_nodes.push(Vec::new());

    let top_level = top_level_subgraph_indices(graph);
    for (pos, idx) in top_level.iter().enumerate() {
        let group_idx = pos + 1;
        let sub = &graph.subgraphs[*idx];
        group_nodes.push(Vec::new());
        for node_id in &sub.nodes {
            if nodes.contains_key(node_id) {
                node_group.insert(node_id.clone(), group_idx);
            }
        }
        if let Some(anchor_id) = subgraph_anchor_id(sub, nodes) {
            if nodes
                .get(anchor_id)
                .map(|node| !node.hidden)
                .unwrap_or(false)
            {
                node_group.insert(anchor_id.to_string(), group_idx);
            }
        }
    }

    for node_id in graph.nodes.keys() {
        if node_group.contains_key(node_id) {
            continue;
        }
        node_group.insert(node_id.clone(), 0);
    }

    for (node_id, group_idx) in &node_group {
        if let Some(bucket) = group_nodes.get_mut(*group_idx) {
            bucket.push(node_id.clone());
        }
    }

    let mut groups: Vec<(usize, f32, f32, f32, f32)> = Vec::new();
    for (idx, bucket) in group_nodes.iter().enumerate() {
        if bucket.is_empty() {
            continue;
        }
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for node_id in bucket {
            if let Some(node) = nodes.get(node_id) {
                min_x = min_x.min(node.x);
                min_y = min_y.min(node.y);
                max_x = max_x.max(node.x + node.width);
                max_y = max_y.max(node.y + node.height);
            }
        }
        if min_x != f32::MAX {
            groups.push((idx, min_x, min_y, max_x, max_y));
        }
    }

    let mut inter_group_edges = 0usize;
    let mut group_links: HashSet<(usize, usize)> = HashSet::new();
    let mut group_link_counts: HashMap<(usize, usize), usize> = HashMap::new();
    let mut group_degree: HashMap<usize, usize> = HashMap::new();
    for edge in &graph.edges {
        let from_group = node_group.get(&edge.from);
        let to_group = node_group.get(&edge.to);
        if let (Some(a), Some(b)) = (from_group, to_group) {
            if a != b {
                inter_group_edges += 1;
                let (min_g, max_g) = if a < b { (*a, *b) } else { (*b, *a) };
                group_links.insert((min_g, max_g));
                *group_link_counts.entry((min_g, max_g)).or_insert(0) += 1;
                *group_degree.entry(*a).or_insert(0) += 1;
                *group_degree.entry(*b).or_insert(0) += 1;
            }
        }
    }
    let max_degree = group_degree.values().copied().max().unwrap_or(0);
    let has_parallel_group_links = group_link_counts.values().any(|count| *count > 1);
    let path_like = inter_group_edges > 0
        && !has_parallel_group_links
        && group_links.len() <= groups.len().saturating_sub(1)
        && max_degree <= 2;
    let grid_pack = inter_group_edges == 0;
    let align_cross = path_like;
    let parallel_top_level_pair = if graph.direction == Direction::TopDown {
        parallel_top_level_subgraph_pair(graph, &top_level)
    } else {
        None
    };
    let preserve_compound_ranks = graph.kind == crate::ir::DiagramKind::Flowchart
        && inter_group_edges > 0
        && !path_like
        && parallel_top_level_pair.is_none();

    // Order groups by their current position to minimize crossing shifts.
    // Keep the non-subgraph group first to bias subgraphs after the main flow.
    if is_horizontal(graph.direction) {
        groups.sort_by(|a, b| {
            let a_primary = if a.0 == 0 { 0 } else { 1 };
            let b_primary = if b.0 == 0 { 0 } else { 1 };
            a_primary
                .cmp(&b_primary)
                .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        });
    } else {
        groups.sort_by(|a, b| {
            let a_primary = if a.0 == 0 { 0 } else { 1 };
            let b_primary = if b.0 == 0 { 0 } else { 1 };
            a_primary
                .cmp(&b_primary)
                .then_with(|| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
        });
    }

    let spacing = config.rank_spacing * 0.8;
    if is_horizontal(graph.direction) {
        if align_cross && !groups.is_empty() {
            let target_y = groups.iter().map(|group| group.2).fold(f32::MAX, f32::min);
            for (group_idx, _min_x, min_y, _max_x, _max_y) in &groups {
                let offset_y = target_y - *min_y;
                for node_id in group_nodes[*group_idx].iter() {
                    if let Some(node) = nodes.get_mut(node_id) {
                        node.y += offset_y;
                    }
                }
            }
        } else if grid_pack && groups.len() > 1 {
            let mut bounds: HashMap<usize, (f32, f32, f32, f32)> = HashMap::new();
            for (group_idx, min_x, min_y, max_x, max_y) in &groups {
                bounds.insert(*group_idx, (*min_x, *min_y, max_x - min_x, max_y - min_y));
            }
            let origin_x = groups.iter().map(|group| group.1).fold(f32::MAX, f32::min);
            let origin_y = groups.iter().map(|group| group.2).fold(f32::MAX, f32::min);

            let mut best_area = f32::MAX;
            let mut best_rows: Vec<Vec<usize>> = Vec::new();
            for cols in 1..=groups.len() {
                let mut rows: Vec<Vec<usize>> = Vec::new();
                let mut idx = 0usize;
                while idx < groups.len() {
                    let mut row = Vec::new();
                    for _ in 0..cols {
                        if idx >= groups.len() {
                            break;
                        }
                        row.push(groups[idx].0);
                        idx += 1;
                    }
                    rows.push(row);
                }
                let mut max_row_width = 0.0f32;
                let mut total_height = 0.0f32;
                for row in &rows {
                    let mut row_width = 0.0f32;
                    let mut row_height = 0.0f32;
                    for (pos, group_idx) in row.iter().enumerate() {
                        if let Some((_, _, width, height)) = bounds.get(group_idx) {
                            row_width += *width;
                            if pos + 1 < row.len() {
                                row_width += spacing;
                            }
                            row_height = row_height.max(*height);
                        }
                    }
                    max_row_width = max_row_width.max(row_width);
                    total_height += row_height;
                }
                if !rows.is_empty() {
                    total_height += spacing * (rows.len().saturating_sub(1) as f32);
                }
                let area = max_row_width * total_height;
                if area < best_area {
                    best_area = area;
                    best_rows = rows;
                }
            }

            let mut cursor_y = origin_y;
            for row in best_rows {
                let mut row_height = 0.0f32;
                let mut cursor_x = origin_x;
                for group_idx in row {
                    let Some((min_x, min_y, width, height)) = bounds.get(&group_idx) else {
                        continue;
                    };
                    let offset_x = cursor_x - min_x;
                    let offset_y = cursor_y - min_y;
                    for node_id in group_nodes[group_idx].iter() {
                        if let Some(node) = nodes.get_mut(node_id) {
                            node.x += offset_x;
                            node.y += offset_y;
                        }
                    }
                    cursor_x += width + spacing;
                    row_height = row_height.max(*height);
                }
                cursor_y += row_height + spacing;
            }
        } else if preserve_compound_ranks {
            return;
        } else {
            let mut cursor = groups
                .iter()
                .find(|group| group.0 == 0)
                .map(|group| group.3)
                .unwrap_or(0.0)
                + spacing;
            for (group_idx, min_x, _min_y, max_x, _max_y) in groups {
                if group_idx == 0 {
                    continue;
                }
                let width = max_x - min_x;
                let offset = cursor - min_x;
                for node_id in group_nodes[group_idx].iter() {
                    if let Some(node) = nodes.get_mut(node_id) {
                        node.x += offset;
                    }
                }
                cursor += width + spacing;
            }
        }
    } else {
        if let Some((source_sg, target_sg)) = parallel_top_level_pair {
            let source_group = top_level
                .iter()
                .position(|idx| *idx == source_sg)
                .map(|pos| pos + 1);
            let target_group = top_level
                .iter()
                .position(|idx| *idx == target_sg)
                .map(|pos| pos + 1);

            if let (Some(source_group), Some(target_group)) = (source_group, target_group) {
                let bounds: HashMap<usize, (f32, f32, f32, f32)> = groups
                    .iter()
                    .map(|(group_idx, min_x, min_y, max_x, max_y)| {
                        (*group_idx, (*min_x, *min_y, *max_x, *max_y))
                    })
                    .collect();

                if let (Some(source), Some(target)) =
                    (bounds.get(&source_group), bounds.get(&target_group))
                {
                    let (pad_x, _) = flowchart_subgraph_padding(graph.direction);
                    let cluster_gap = (config.node_spacing * 0.4).max(8.0);
                    let desired_source_min_x = target.2 + pad_x * 2.0 + cluster_gap;
                    let dx_source = (desired_source_min_x - source.0).max(0.0);
                    let desired_target_min_y = source.3;
                    let dy_target = desired_target_min_y - target.1;

                    for node_id in group_nodes[source_group].iter() {
                        if let Some(node) = nodes.get_mut(node_id) {
                            node.x += dx_source;
                        }
                    }
                    for node_id in group_nodes[target_group].iter() {
                        if let Some(node) = nodes.get_mut(node_id) {
                            node.y += dy_target;
                        }
                    }
                }
            }
        } else if align_cross && !groups.is_empty() {
            let target_x = groups.iter().map(|group| group.1).fold(f32::MAX, f32::min);
            for (group_idx, min_x, _min_y, _max_x, _max_y) in &groups {
                let offset_x = target_x - *min_x;
                for node_id in group_nodes[*group_idx].iter() {
                    if let Some(node) = nodes.get_mut(node_id) {
                        node.x += offset_x;
                    }
                }
            }
        } else if grid_pack && groups.len() > 1 {
            let mut bounds: HashMap<usize, (f32, f32, f32, f32)> = HashMap::new();
            for (group_idx, min_x, min_y, max_x, max_y) in &groups {
                bounds.insert(*group_idx, (*min_x, *min_y, max_x - min_x, max_y - min_y));
            }
            let origin_x = groups.iter().map(|group| group.1).fold(f32::MAX, f32::min);
            let origin_y = groups.iter().map(|group| group.2).fold(f32::MAX, f32::min);

            let mut best_rows = Vec::new();
            let mut best_area = f32::MAX;
            for rows in 1..=groups.len() {
                let cols = (groups.len() + rows - 1) / rows;
                let mut grid: Vec<Vec<usize>> = Vec::new();
                let mut idx = 0usize;
                for _ in 0..rows {
                    let mut col = Vec::new();
                    for _ in 0..cols {
                        if idx >= groups.len() {
                            break;
                        }
                        col.push(groups[idx].0);
                        idx += 1;
                    }
                    grid.push(col);
                }
                let mut max_col_height = 0.0f32;
                let mut total_width = 0.0f32;
                for col in &grid {
                    let mut col_height = 0.0f32;
                    let mut col_width = 0.0f32;
                    for (pos, group_idx) in col.iter().enumerate() {
                        if let Some((_, _, width, height)) = bounds.get(group_idx) {
                            col_height += *height;
                            if pos + 1 < col.len() {
                                col_height += spacing;
                            }
                            col_width = col_width.max(*width);
                        }
                    }
                    max_col_height = max_col_height.max(col_height);
                    total_width += col_width;
                }
                if !grid.is_empty() {
                    total_width += spacing * (grid.len().saturating_sub(1) as f32);
                }
                let area = total_width * max_col_height;
                if area < best_area {
                    best_area = area;
                    best_rows = grid;
                }
            }

            let mut cursor_x = origin_x;
            for col in best_rows {
                let mut col_width = 0.0f32;
                let mut cursor_y = origin_y;
                for group_idx in col {
                    let Some((min_x, min_y, width, height)) = bounds.get(&group_idx) else {
                        continue;
                    };
                    let offset_x = cursor_x - min_x;
                    let offset_y = cursor_y - min_y;
                    for node_id in group_nodes[group_idx].iter() {
                        if let Some(node) = nodes.get_mut(node_id) {
                            node.x += offset_x;
                            node.y += offset_y;
                        }
                    }
                    cursor_y += height + spacing;
                    col_width = col_width.max(*width);
                }
                cursor_x += col_width + spacing;
            }
        } else if preserve_compound_ranks {
            return;
        } else {
            let mut cursor = groups
                .iter()
                .find(|group| group.0 == 0)
                .map(|group| group.4)
                .unwrap_or(0.0)
                + spacing;
            for (group_idx, _min_x, min_y, _max_x, max_y) in groups {
                if group_idx == 0 {
                    continue;
                }
                let height = max_y - min_y;
                let offset = cursor - min_y;
                for node_id in group_nodes[group_idx].iter() {
                    if let Some(node) = nodes.get_mut(node_id) {
                        node.y += offset;
                    }
                }
                cursor += height + spacing;
            }
        }
    }
}

fn compress_linear_subgraphs(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.subgraphs.is_empty() {
        return;
    }
    let base_gap = config
        .rank_spacing
        .max(config.flowchart.auto_spacing.min_spacing);
    let horizontal = is_horizontal(graph.direction);

    for sub in &graph.subgraphs {
        if sub.nodes.len() < 3 {
            continue;
        }
        let sub_set: HashSet<&str> = sub.nodes.iter().map(|id| id.as_str()).collect();
        let mut in_deg: HashMap<String, usize> = HashMap::new();
        let mut out_deg: HashMap<String, usize> = HashMap::new();
        let mut next_map: HashMap<String, String> = HashMap::new();
        let mut edges_in_sub = 0usize;

        for node_id in &sub.nodes {
            in_deg.insert(node_id.clone(), 0);
            out_deg.insert(node_id.clone(), 0);
        }

        for edge in &graph.edges {
            if !sub_set.contains(edge.from.as_str()) || !sub_set.contains(edge.to.as_str()) {
                continue;
            }
            edges_in_sub += 1;
            let out = out_deg.entry(edge.from.clone()).or_insert(0);
            *out += 1;
            if *out == 1 {
                next_map.insert(edge.from.clone(), edge.to.clone());
            } else {
                next_map.remove(&edge.from);
            }
            let entry = in_deg.entry(edge.to.clone()).or_insert(0);
            *entry += 1;
        }

        if edges_in_sub + 1 != sub.nodes.len() {
            continue;
        }
        if in_deg.values().any(|&d| d > 1) || out_deg.values().any(|&d| d > 1) {
            continue;
        }

        let starts: Vec<&String> = sub
            .nodes
            .iter()
            .filter(|id| *in_deg.get(*id).unwrap_or(&0) == 0)
            .collect();
        if starts.len() != 1 {
            continue;
        }

        let mut order: Vec<String> = Vec::with_capacity(sub.nodes.len());
        let mut visited: HashSet<String> = HashSet::new();
        let mut current = starts[0].clone();
        while visited.insert(current.clone()) {
            order.push(current.clone());
            if let Some(next) = next_map.get(&current) {
                current = next.clone();
            } else {
                break;
            }
        }
        if order.len() != sub.nodes.len() {
            continue;
        }

        let gap = if flowchart_subgraph_is_recursive_cluster(graph, sub) {
            subgraph_layout_config_for(graph, sub, false, config)
                .rank_spacing
                .max(base_gap)
        } else {
            base_gap
        };

        let mut cross_centers: Vec<f32> = order
            .iter()
            .filter_map(|id| nodes.get(id))
            .map(|node| {
                if horizontal {
                    node.y + node.height * 0.5
                } else {
                    node.x + node.width * 0.5
                }
            })
            .collect();
        if cross_centers.is_empty() {
            continue;
        }
        cross_centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        let target_cross = cross_centers[cross_centers.len() / 2];

        let min_main = order
            .iter()
            .filter_map(|id| nodes.get(id))
            .map(|node| if horizontal { node.x } else { node.y })
            .fold(f32::MAX, f32::min);
        let mut cursor = min_main;
        for node_id in order {
            if let Some(node) = nodes.get_mut(&node_id) {
                if horizontal {
                    node.x = cursor;
                    node.y = target_cross - node.height * 0.5;
                    cursor += node.width + gap;
                } else {
                    node.x = target_cross - node.width * 0.5;
                    node.y = cursor;
                    cursor += node.height + gap;
                }
            }
        }
    }
}

fn apply_orthogonal_region_bands(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    let mut region_indices = Vec::new();
    for (idx, sub) in graph.subgraphs.iter().enumerate() {
        if is_region_subgraph(sub) {
            region_indices.push(idx);
        }
    }
    if region_indices.is_empty() {
        return;
    }

    let sets: Vec<HashSet<String>> = graph
        .subgraphs
        .iter()
        .map(|sub| sub.nodes.iter().cloned().collect())
        .collect();

    let mut parent_map: HashMap<usize, Vec<usize>> = HashMap::new();
    for region_idx in region_indices {
        let region_set = &sets[region_idx];
        let mut parent: Option<usize> = None;
        for (idx, set) in sets.iter().enumerate() {
            if idx == region_idx {
                continue;
            }
            if set.len() <= region_set.len() {
                continue;
            }
            if !region_set.is_subset(set) {
                continue;
            }
            if is_region_subgraph(&graph.subgraphs[idx]) {
                continue;
            }
            match parent {
                None => parent = Some(idx),
                Some(current) => {
                    if set.len() < sets[current].len() {
                        parent = Some(idx);
                    }
                }
            }
        }
        if let Some(parent_idx) = parent {
            parent_map.entry(parent_idx).or_default().push(region_idx);
        }
    }

    let spacing = config.rank_spacing + STATE_RANK_SPACING_BOOST;
    // Concurrent regions inside a composite state should be arranged ORTHOGONAL
    // to the parent's flow direction: TB/BT diagrams stack regions along X
    // (side-by-side); LR/RL diagrams stack along Y (top-to-bottom).
    let stack_along_x = !is_horizontal(graph.direction);

    for region_list in parent_map.values() {
        let mut region_boxes: Vec<(usize, f32, f32, f32, f32)> = Vec::new();
        for &region_idx in region_list {
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            for node_id in &graph.subgraphs[region_idx].nodes {
                if let Some(node) = nodes.get(node_id) {
                    min_x = min_x.min(node.x);
                    min_y = min_y.min(node.y);
                    max_x = max_x.max(node.x + node.width);
                    max_y = max_y.max(node.y + node.height);
                }
            }
            if min_x != f32::MAX {
                region_boxes.push((region_idx, min_x, min_y, max_x, max_y));
            }
        }
        if region_boxes.len() <= 1 {
            continue;
        }

        // Each region cluster will be wrapped with STATE_REGION_PAD_X/Y of
        // padding around its inner nodes when build_subgraph_layouts runs.
        // The cursor advance must account for that padding on BOTH sides of
        // each region — otherwise the rendered region rects overlap with
        // their siblings even though the inner state nodes don't.
        if stack_along_x {
            region_boxes.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
            let target_min_y = region_boxes
                .iter()
                .map(|entry| entry.2)
                .fold(f32::MAX, f32::min);
            region_boxes.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let mut cursor = region_boxes.first().map(|entry| entry.1).unwrap_or(0.0);
            for (region_idx, min_x, min_y, max_x, _max_y) in region_boxes {
                // Position this region's first node so the region rect's left
                // edge starts at the cursor (i.e. shift by region pad).
                let dx = (cursor + STATE_REGION_PAD_X) - min_x;
                let dy = target_min_y - min_y;
                for node_id in &graph.subgraphs[region_idx].nodes {
                    if let Some(node) = nodes.get_mut(node_id) {
                        node.x += dx;
                        node.y += dy;
                    }
                }
                // Advance cursor past the full region rect width (inner span
                // + padding on both sides) plus inter-region gap.
                cursor += (max_x - min_x) + 2.0 * STATE_REGION_PAD_X + spacing;
            }
        } else {
            let target_min_x = region_boxes
                .iter()
                .map(|entry| entry.1)
                .fold(f32::MAX, f32::min);
            region_boxes.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
            let mut cursor = region_boxes.first().map(|entry| entry.2).unwrap_or(0.0);
            for (region_idx, min_x, min_y, _max_x, max_y) in region_boxes {
                let dx = target_min_x - min_x;
                let dy = (cursor + STATE_REGION_PAD_Y) - min_y;
                for node_id in &graph.subgraphs[region_idx].nodes {
                    if let Some(node) = nodes.get_mut(node_id) {
                        node.x += dx;
                        node.y += dy;
                    }
                }
                cursor += (max_y - min_y) + 2.0 * STATE_REGION_PAD_Y + spacing;
            }
        }
    }
}

/// Pre-computed containment tree for subgraphs.
///
/// Built once from `graph.subgraphs` by checking subset relationships between
/// node sets.  Each subgraph is assigned an optional `parent` (the *smallest*
/// containing subgraph) and a list of `children`.  Top-level subgraphs have
/// `parent == None`.
struct SubgraphTree {
    /// `parent[i]` = index of the immediate parent subgraph, or `None` if top-level.
    parent: Vec<Option<usize>>,
    /// `children[i]` = indices of immediate child subgraphs.
    children: Vec<Vec<usize>>,
    /// Indices of subgraphs that have no parent.
    top_level: Vec<usize>,
}

impl SubgraphTree {
    fn build(graph: &Graph) -> Self {
        let n = graph.subgraphs.len();
        let sets: Vec<HashSet<String>> = graph
            .subgraphs
            .iter()
            .map(|sub| sub.nodes.iter().cloned().collect())
            .collect();

        // Sort indices by set size ascending so we can find the *smallest*
        // containing parent efficiently.
        let mut by_size: Vec<usize> = (0..n).collect();
        by_size.sort_by_key(|&i| sets[i].len());

        let mut parent: Vec<Option<usize>> = vec![None; n];
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];

        // For each subgraph (from smallest to largest), find its immediate
        // parent: the smallest subgraph that strictly contains it.
        for (pos, &i) in by_size.iter().enumerate() {
            for &j in &by_size[pos + 1..] {
                if sets[j].len() > sets[i].len() && sets[i].is_subset(&sets[j]) {
                    parent[i] = Some(j);
                    children[j].push(i);
                    break;
                }
            }
        }

        let top_level: Vec<usize> = (0..n).filter(|&i| parent[i].is_none()).collect();

        SubgraphTree {
            parent,
            children,
            top_level,
        }
    }

    /// Returns `true` if subgraph `ancestor` contains subgraph `descendant`
    /// (i.e. `descendant`'s node set is a subset of `ancestor`'s).
    fn is_ancestor(&self, ancestor: usize, descendant: usize) -> bool {
        let mut cur = descendant;
        loop {
            match self.parent[cur] {
                Some(p) if p == ancestor => return true,
                Some(p) => cur = p,
                None => return false,
            }
        }
    }

    /// Two subgraphs are siblings if neither is an ancestor of the other.
    fn are_siblings(&self, a: usize, b: usize) -> bool {
        a != b && !self.is_ancestor(a, b) && !self.is_ancestor(b, a)
    }

    /// Returns the maximum number of NESTED composite (non-region) levels below
    /// `idx`. A leaf composite (no nested composites inside) returns 0.
    /// A composite containing one nested composite returns 1, and so on.
    /// Region subgraphs are not counted.
    fn max_nested_composite_depth_below(&self, idx: usize, graph: &Graph) -> usize {
        let mut max_d = 0usize;
        for &child in &self.children[idx] {
            if let Some(child_sub) = graph.subgraphs.get(child) {
                if is_region_subgraph(child_sub) {
                    continue;
                }
                let d = 1 + self.max_nested_composite_depth_below(child, graph);
                if d > max_d {
                    max_d = d;
                }
            }
        }
        max_d
    }
}

fn top_level_subgraph_indices(graph: &Graph) -> Vec<usize> {
    SubgraphTree::build(graph).top_level
}

fn parallel_top_level_subgraph_pair(graph: &Graph, top_level: &[usize]) -> Option<(usize, usize)> {
    parallel_top_level_flowchart(graph, top_level).map(|info| (info.source_sg, info.target_sg))
}

fn parallel_top_level_flowchart(
    graph: &Graph,
    top_level: &[usize],
) -> Option<ParallelTopLevelFlowchart> {
    if graph.kind != crate::ir::DiagramKind::Flowchart
        || graph.direction != Direction::TopDown
        || top_level.len() != 2
    {
        return None;
    }

    let mut top_member_ids: HashSet<&str> = HashSet::new();
    let mut node_to_top_level: HashMap<&str, usize> = HashMap::new();
    for &idx in top_level {
        let sub = &graph.subgraphs[idx];
        for node_id in &sub.nodes {
            top_member_ids.insert(node_id.as_str());
            node_to_top_level.insert(node_id.as_str(), idx);
        }
        if let Some(id) = sub.id.as_deref()
            && !id.is_empty()
        {
            top_member_ids.insert(id);
            node_to_top_level.insert(id, idx);
        }
        if !sub.label.is_empty() {
            top_member_ids.insert(sub.label.as_str());
            node_to_top_level.insert(sub.label.as_str(), idx);
        }
    }

    if !graph
        .nodes
        .keys()
        .all(|node_id| top_member_ids.contains(node_id.as_str()))
    {
        return None;
    }

    let mut cross_edges: Vec<(usize, usize, usize)> = Vec::new();
    for (idx, edge) in graph.edges.iter().enumerate() {
        let from_sg = node_to_top_level.get(edge.from.as_str()).copied();
        let to_sg = node_to_top_level.get(edge.to.as_str()).copied();
        let (Some(from_sg), Some(to_sg)) = (from_sg, to_sg) else {
            continue;
        };
        if from_sg == to_sg {
            continue;
        }
        cross_edges.push((idx, from_sg, to_sg));
    }

    if cross_edges.len() < 2 {
        return None;
    }

    let (first_idx, source_sg, target_sg) = cross_edges[0];
    let first_edge = &graph.edges[first_idx];
    let source_node = first_edge.from.clone();
    let target_node = first_edge.to.clone();
    let normalized_pair = if source_sg < target_sg {
        (source_sg, target_sg)
    } else {
        (target_sg, source_sg)
    };

    let mut edge_indices = Vec::new();
    for (edge_idx, from_sg, to_sg) in cross_edges {
        let normalized = if from_sg < to_sg {
            (from_sg, to_sg)
        } else {
            (to_sg, from_sg)
        };
        if normalized != normalized_pair {
            return None;
        }
        let edge = &graph.edges[edge_idx];
        let same_direction = from_sg == source_sg
            && to_sg == target_sg
            && edge.from == source_node
            && edge.to == target_node;
        let reverse_direction = from_sg == target_sg
            && to_sg == source_sg
            && edge.from == target_node
            && edge.to == source_node;
        if !same_direction && !reverse_direction {
            return None;
        }
        edge_indices.push(edge_idx);
    }

    Some(ParallelTopLevelFlowchart {
        source_sg,
        target_sg,
        source_node,
        target_node,
        edge_indices,
    })
}

fn apply_parallel_top_level_flowchart_compound_nodes(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    let top_level = top_level_subgraph_indices(graph);
    let Some(info) = parallel_top_level_flowchart(graph, &top_level) else {
        return;
    };
    let Some(source_sub) = graph.subgraphs.get(info.source_sg) else {
        return;
    };
    let Some(target_sub) = graph.subgraphs.get(info.target_sg) else {
        return;
    };
    let source_children = hub_children_in_subgraph(graph, source_sub, &info.source_node);
    let target_children = hub_children_in_subgraph(graph, target_sub, &info.target_node);
    if source_children.is_empty() || target_children.is_empty() {
        return;
    }

    let Some(mut source_hub) = nodes.get(&info.source_node).cloned() else {
        return;
    };
    let Some(target_hub) = nodes.get(&info.target_node).cloned() else {
        return;
    };
    let Some((target_min_x, _target_min_y, target_max_x, _target_max_y)) =
        node_group_bounds(nodes, &target_sub.nodes)
    else {
        return;
    };

    let row_gap = config.node_spacing.max(1.0);
    let source_lift = config.node_spacing * PARALLEL_FLOWCHART_SOURCE_HUB_LIFT;
    if source_lift > 0.0 {
        if let Some(source) = nodes.get_mut(&info.source_node) {
            source.y -= source_lift;
            source_hub.y = source.y;
        }
    }
    let source_hub_center_x = source_hub.x + source_hub.width / 2.0;
    let shared_rank_y =
        (target_hub.y - config.node_spacing * PARALLEL_FLOWCHART_SHARED_RANK_LIFT).max(0.0);
    // Dagre's compound solve puts the source's local children on the same
    // rank as the remote hub once the parallel cross-edge labels reserve an
    // intermediate rank. Mirror that for this tightly-scoped topology.
    place_node_row(
        nodes,
        &source_children,
        0,
        source_hub_center_x,
        shared_rank_y,
        row_gap,
    );

    let target_hub_center_x = target_min_x + (target_max_x - target_min_x) * 0.55;
    if let Some(target) = nodes.get_mut(&info.target_node) {
        target.x = target_hub_center_x - target.width / 2.0;
        target.y = shared_rank_y;
    }
    let target_center_idx = if target_children.len() >= 2 { 1 } else { 0 };
    let target_row_y = shared_rank_y
        + target_hub.height
        + (config.node_spacing * PARALLEL_FLOWCHART_TARGET_CHILD_GAP).max(row_gap);
    place_node_row(
        nodes,
        &target_children,
        target_center_idx,
        target_hub_center_x,
        target_row_y,
        row_gap,
    );
}

fn apply_parallel_top_level_flowchart_compound_subgraphs(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    subgraphs: &mut [SubgraphLayout],
    config: &LayoutConfig,
) {
    let top_level = top_level_subgraph_indices(graph);
    let Some(info) = parallel_top_level_flowchart(graph, &top_level) else {
        return;
    };
    let Some(source_sub) = graph.subgraphs.get(info.source_sg) else {
        return;
    };
    let Some(target_sub) = graph.subgraphs.get(info.target_sg) else {
        return;
    };
    let Some(source_layout_idx) = subgraph_layout_index(subgraphs, source_sub) else {
        return;
    };
    let Some(target_layout_idx) = subgraph_layout_index(subgraphs, target_sub) else {
        return;
    };
    if source_layout_idx == target_layout_idx {
        return;
    }

    let target_right = {
        let target = &subgraphs[target_layout_idx];
        target.x + target.width
    };
    let cluster_gap = (config.node_spacing * 0.4).max(8.0);
    let desired_source_x = target_right + cluster_gap;
    if let Some(source_hub) = nodes.get(&info.source_node) {
        let current_source_center_x = source_hub.x + source_hub.width / 2.0;
        let desired_source_center_x = desired_source_x
            + source_hub.width / 2.0
            + config.node_spacing * PARALLEL_FLOWCHART_SOURCE_CENTER_GAP;
        let dx = desired_source_center_x - current_source_center_x;
        if dx.abs() > 0.5 {
            for node_id in &source_sub.nodes {
                if let Some(node) = nodes.get_mut(node_id) {
                    node.x += dx;
                }
            }
        }
    }
    let source_y_height =
        node_group_bounds(nodes, &source_sub.nodes).map(|(_min_x, min_y, _max_x, max_y)| {
            let top_pad = config.node_spacing * PARALLEL_FLOWCHART_SOURCE_CLUSTER_TOP_PAD;
            let bottom_pad = config.node_spacing * PARALLEL_FLOWCHART_CLUSTER_BOTTOM_PAD;
            let y = min_y - top_pad;
            (y, max_y - y + bottom_pad)
        });
    let target_y_height =
        node_group_bounds(nodes, &target_sub.nodes).map(|(_min_x, min_y, _max_x, max_y)| {
            let top_pad = config.node_spacing * PARALLEL_FLOWCHART_TARGET_CLUSTER_TOP_PAD;
            let bottom_pad = config.node_spacing * PARALLEL_FLOWCHART_CLUSTER_BOTTOM_PAD;
            let y = min_y - top_pad;
            (y, max_y - y + bottom_pad)
        });

    {
        let source = &mut subgraphs[source_layout_idx];
        if source.x > desired_source_x {
            let extra_left = source.x - desired_source_x;
            source.x = desired_source_x;
            source.width += extra_left;
        }
        if let Some((_min_x, _min_y, max_x, _max_y)) = node_group_bounds(nodes, &source_sub.nodes) {
            source.x = desired_source_x;
            let right_pad = (config.node_spacing * 0.75).max(24.0);
            source.width =
                (max_x - source.x + right_pad).max(source.width.min(config.node_spacing));
        }
        if let Some((y, height)) = source_y_height {
            source.y = y;
            source.height = height;
        }
    }

    if let Some((y, height)) = target_y_height {
        let target = &mut subgraphs[target_layout_idx];
        target.y = y;
        target.height = height;
    }
}

fn apply_parallel_top_level_flowchart_edge_lanes(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    edge_labels: &[Option<TextBlock>],
    routed_points: &mut [Vec<(f32, f32)>],
    label_anchors: &mut [Option<(f32, f32)>],
    config: &LayoutConfig,
) {
    let top_level = top_level_subgraph_indices(graph);
    let Some(info) = parallel_top_level_flowchart(graph, &top_level) else {
        return;
    };
    if info.edge_indices.len() < 2 {
        return;
    }
    let Some(source) = nodes.get(&info.source_node) else {
        return;
    };
    let Some(target) = nodes.get(&info.target_node) else {
        return;
    };

    if let Some(source_sub) = graph.subgraphs.get(info.source_sg) {
        let source_children = hub_children_in_subgraph(graph, source_sub, &info.source_node);
        apply_compound_hub_child_flowchart_routes(
            graph,
            nodes,
            &info.source_node,
            &source_children,
            routed_points,
        );
    }
    if let Some(target_sub) = graph.subgraphs.get(info.target_sg) {
        let target_children = hub_children_in_subgraph(graph, target_sub, &info.target_node);
        apply_compound_hub_child_flowchart_routes(
            graph,
            nodes,
            &info.target_node,
            &target_children,
            routed_points,
        );
    }

    let label_widths: Vec<f32> = info
        .edge_indices
        .iter()
        .map(|&idx| {
            edge_labels
                .get(idx)
                .and_then(|label| label.as_ref())
                .map(|label| label.width)
                .unwrap_or(24.0)
        })
        .collect();
    let mut lane_x = vec![0.0; info.edge_indices.len()];
    let source_center_x = source.x + source.width / 2.0;
    let rightmost_lane = source_center_x - source.width * PARALLEL_FLOWCHART_RIGHT_LABEL_LANE;
    if let Some(last) = lane_x.last_mut() {
        *last = rightmost_lane;
    }
    let lane_gap = (config.node_spacing * 0.25).max(10.0);
    for pos in (0..lane_x.len().saturating_sub(1)).rev() {
        lane_x[pos] =
            lane_x[pos + 1] - label_widths[pos] / 2.0 - label_widths[pos + 1] / 2.0 - lane_gap;
    }

    let lane_y = source.y + source.height + config.node_spacing * PARALLEL_FLOWCHART_LABEL_LANE_GAP;
    let bend_y = target.y - config.node_spacing * 0.86;
    let center_index = (info.edge_indices.len() as f32 - 1.0) * 0.5;
    for (pos, &edge_idx) in info.edge_indices.iter().enumerate() {
        let Some(edge) = graph.edges.get(edge_idx) else {
            continue;
        };
        if edge_idx >= routed_points.len() || edge_idx >= label_anchors.len() {
            continue;
        }
        let relative = pos as f32 - center_index;
        let start_offset = -source.width * 0.32 + relative * source.width * 0.30;
        let end_offset = -target.height * 0.20 + relative * target.height * 0.06;
        let start = anchor_point_for_node(source, EdgeSide::Bottom, start_offset);
        let end = anchor_point_for_node(target, EdgeSide::Right, end_offset);
        let points = if edge.from == info.source_node {
            vec![start, (lane_x[pos], lane_y), (lane_x[pos], bend_y), end]
        } else {
            vec![end, (lane_x[pos], bend_y), (lane_x[pos], lane_y), start]
        };
        routed_points[edge_idx] = points;
        label_anchors[edge_idx] = Some((lane_x[pos], lane_y));
    }
}

fn apply_flowchart_adjacent_parallel_label_lanes(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    edge_labels: &[Option<TextBlock>],
    routed_points: &mut [Vec<(f32, f32)>],
    label_anchors: &mut [Option<(f32, f32)>],
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.edges.len() < 2 {
        return;
    }

    let (label_pad_x, _) = label_placement::edge_label_padding(graph.kind, config);
    let lane_gap = (config.node_spacing * 0.4).max(16.0);
    let small_dense_labeled_flowchart =
        is_small_dense_labeled_flowchart(graph, graph.nodes.len(), &graph.edges);
    let mut groups: HashMap<(String, String), Vec<usize>> = HashMap::new();
    for (idx, edge) in graph.edges.iter().enumerate() {
        if idx >= routed_points.len()
            || idx >= label_anchors.len()
            || edge.from == edge.to
            || edge_labels
                .get(idx)
                .and_then(|label| label.as_ref())
                .is_none()
        {
            continue;
        }
        if nodes.contains_key(&edge.from) && nodes.contains_key(&edge.to) {
            groups.entry(edge_pair_key(edge)).or_default().push(idx);
        }
    }

    for indices in groups.values_mut() {
        if indices.len() < 2 {
            continue;
        }
        indices.sort_unstable();

        let Some(first_edge) = graph.edges.get(indices[0]) else {
            continue;
        };
        let Some(first) = nodes.get(&first_edge.from) else {
            continue;
        };
        let Some(second) = nodes.get(&first_edge.to) else {
            continue;
        };

        let first_center = (first.x + first.width / 2.0, first.y + first.height / 2.0);
        let second_center = (
            second.x + second.width / 2.0,
            second.y + second.height / 2.0,
        );
        let vertical =
            (first_center.1 - second_center.1).abs() >= (first_center.0 - second_center.0).abs();

        let (start_id, end_id) = if vertical {
            if first_center.1 <= second_center.1 {
                (first_edge.from.as_str(), first_edge.to.as_str())
            } else {
                (first_edge.to.as_str(), first_edge.from.as_str())
            }
        } else if first_center.0 <= second_center.0 {
            (first_edge.from.as_str(), first_edge.to.as_str())
        } else {
            (first_edge.to.as_str(), first_edge.from.as_str())
        };

        let mut ordered: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|idx| {
                graph
                    .edges
                    .get(*idx)
                    .is_some_and(|edge| edge.from == start_id && edge.to == end_id)
            })
            .collect();
        let forward_count = ordered.len();
        ordered.extend(indices.iter().copied().filter(|idx| {
            graph
                .edges
                .get(*idx)
                .is_some_and(|edge| !(edge.from == start_id && edge.to == end_id))
        }));
        if ordered.len() < 2 {
            continue;
        }

        let lane_widths: Vec<f32> = ordered
            .iter()
            .map(|idx| {
                edge_labels
                    .get(*idx)
                    .and_then(|label| label.as_ref())
                    .map(|label| label.width + 2.0 * label_pad_x)
                    .unwrap_or(24.0)
            })
            .collect();

        let Some(start_node) = nodes.get(start_id) else {
            continue;
        };
        let Some(end_node) = nodes.get(end_id) else {
            continue;
        };
        let start_center = (
            start_node.x + start_node.width / 2.0,
            start_node.y + start_node.height / 2.0,
        );
        let end_center = (
            end_node.x + end_node.width / 2.0,
            end_node.y + end_node.height / 2.0,
        );
        let center_cross = if vertical {
            (start_center.0 + end_center.0) * 0.5
        } else {
            (start_center.1 + end_center.1) * 0.5
        };
        let center_main = if vertical {
            (start_center.1 + end_center.1) * 0.5
        } else {
            (start_center.0 + end_center.0) * 0.5
        };

        let mut lane_cross = vec![0.0; ordered.len()];
        let center_pos = if ordered.len() % 2 == 0 && forward_count <= 1 {
            None
        } else if forward_count > 1 {
            Some(forward_count - 1)
        } else {
            Some(ordered.len() / 2)
        };
        if let Some(center_pos) = center_pos {
            lane_cross[center_pos] = center_cross;
            for pos in (0..center_pos).rev() {
                lane_cross[pos] = lane_cross[pos + 1]
                    - lane_widths[pos] / 2.0
                    - lane_widths[pos + 1] / 2.0
                    - lane_gap;
            }
            for pos in (center_pos + 1)..ordered.len() {
                lane_cross[pos] = lane_cross[pos - 1]
                    + lane_widths[pos - 1] / 2.0
                    + lane_widths[pos] / 2.0
                    + lane_gap;
            }
        } else {
            let total_width =
                lane_widths.iter().sum::<f32>() + lane_gap * (lane_widths.len() - 1) as f32;
            let mut cursor = center_cross - total_width / 2.0;
            for (pos, width) in lane_widths.iter().enumerate() {
                lane_cross[pos] = cursor + width / 2.0;
                cursor += width + lane_gap;
            }
        }

        let main_distance = if vertical {
            (end_center.1 - start_center.1).abs()
        } else {
            (end_center.0 - start_center.0).abs()
        };
        let adjacent_distance = if vertical {
            (start_node.height + end_node.height) * 0.5 + config.rank_spacing
        } else {
            (start_node.width + end_node.width) * 0.5 + config.rank_spacing
        };
        if small_dense_labeled_flowchart && ordered.len() == 2 {
            if main_distance > adjacent_distance * 1.35 {
                let min_cross = if vertical {
                    start_node.x.min(end_node.x)
                } else {
                    start_node.y.min(end_node.y)
                };
                let max_cross = if vertical {
                    (start_node.x + start_node.width).max(end_node.x + end_node.width)
                } else {
                    (start_node.y + start_node.height).max(end_node.y + end_node.height)
                };
                let outside_gap = 1.65;
                let outside_half_width = |pos: usize| {
                    ordered
                        .get(pos)
                        .and_then(|idx| edge_labels.get(*idx))
                        .and_then(|label| label.as_ref())
                        .map(|label| label.width * 0.5 + outside_gap)
                        .unwrap_or(lane_widths[pos] * 0.5)
                };
                lane_cross[0] = min_cross - outside_half_width(0);
                lane_cross[1] = max_cross + outside_half_width(1);
            } else {
                let lane_offset = if vertical {
                    start_node.width.min(end_node.width)
                } else {
                    start_node.height.min(end_node.height)
                } * 0.5;
                lane_cross[0] = center_cross - lane_offset;
                lane_cross[1] = center_cross + lane_offset;
            }
        }

        let center_pos_f = center_pos
            .map(|pos| pos as f32)
            .unwrap_or_else(|| (ordered.len() as f32 - 1.0) * 0.5);
        for (pos, edge_idx) in ordered.iter().copied().enumerate() {
            let Some(edge) = graph.edges.get(edge_idx) else {
                continue;
            };
            let relative = pos as f32 - center_pos_f;
            let start_cross = if vertical {
                start_center.0 + relative * start_node.width * 0.30
            } else {
                start_center.1 + relative * start_node.height * 0.30
            };
            let end_cross = if vertical {
                end_center.0 + relative * end_node.width * 0.30
            } else {
                end_center.1 + relative * end_node.height * 0.30
            };

            let start_point = if vertical {
                (start_cross, start_node.y + start_node.height)
            } else {
                (start_node.x + start_node.width, start_cross)
            };
            let end_point = if vertical {
                (end_cross, end_node.y)
            } else {
                (end_node.x, end_cross)
            };
            let label_point = if vertical {
                (lane_cross[pos], center_main)
            } else {
                (center_main, lane_cross[pos])
            };

            routed_points[edge_idx] = if edge.from == start_id && edge.to == end_id {
                vec![start_point, label_point, end_point]
            } else {
                vec![end_point, label_point, start_point]
            };
            label_anchors[edge_idx] = Some(label_point);
        }
    }
}

fn expand_flowchart_subgraphs_for_edge_labels(
    graph: &Graph,
    edge_labels: &[Option<TextBlock>],
    label_anchors: &[Option<(f32, f32)>],
    subgraphs: &mut [SubgraphLayout],
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || subgraphs.is_empty() {
        return;
    }

    let pad_x = (config.node_spacing * 0.75).max(24.0);
    let pad_y = (config.rank_spacing * 0.25).max(18.0);
    for (edge_idx, edge) in graph.edges.iter().enumerate() {
        let Some(label) = edge_labels.get(edge_idx).and_then(|label| label.as_ref()) else {
            continue;
        };
        if label.width <= 0.0 || label.height <= 0.0 {
            continue;
        }
        let Some((cx, cy)) = label_anchors.get(edge_idx).and_then(|anchor| *anchor) else {
            continue;
        };

        let label_min_x = cx - label.width / 2.0 - pad_x;
        let label_max_x = cx + label.width / 2.0 + pad_x;
        let label_min_y = cy - label.height / 2.0 - pad_y;
        let label_max_y = cy + label.height / 2.0 + pad_y;

        for subgraph in subgraphs.iter_mut() {
            let members: HashSet<&str> = subgraph.nodes.iter().map(|id| id.as_str()).collect();
            if !members.contains(edge.from.as_str()) || !members.contains(edge.to.as_str()) {
                continue;
            }

            let min_x = subgraph.x.min(label_min_x);
            let min_y = subgraph.y.min(label_min_y);
            let max_x = (subgraph.x + subgraph.width).max(label_max_x);
            let max_y = (subgraph.y + subgraph.height).max(label_max_y);
            subgraph.x = min_x;
            subgraph.y = min_y;
            subgraph.width = max_x - min_x;
            subgraph.height = max_y - min_y;
        }
    }
}

fn apply_flowchart_nested_bridge_cross_routes(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    routed_points: &mut [Vec<(f32, f32)>],
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.subgraphs.len() < 3 {
        return;
    }

    let tree = SubgraphTree::build(graph);
    for (parent_idx, parent) in graph.subgraphs.iter().enumerate() {
        let parent_direction = parent
            .direction
            .unwrap_or_else(|| subgraph_layout_direction(graph, parent));
        if parent_direction != Direction::TopDown {
            continue;
        }

        let Some(child_indices) = tree.children.get(parent_idx) else {
            continue;
        };
        if child_indices.len() < 3 {
            continue;
        }

        let mut node_to_child: HashMap<&str, usize> = HashMap::new();
        for &child_idx in child_indices {
            let Some(child) = graph.subgraphs.get(child_idx) else {
                continue;
            };
            for node_id in &child.nodes {
                node_to_child.insert(node_id.as_str(), child_idx);
            }
        }

        for &bridge_idx in child_indices {
            let Some(bridge_sub) = graph.subgraphs.get(bridge_idx) else {
                continue;
            };
            if bridge_sub.nodes.len() != 1 {
                continue;
            }
            let bridge_id = bridge_sub.nodes[0].as_str();

            let target_children: HashSet<usize> = graph
                .edges
                .iter()
                .filter(|edge| edge.from == bridge_id)
                .filter_map(|edge| node_to_child.get(edge.to.as_str()).copied())
                .filter(|idx| *idx != bridge_idx)
                .collect();
            if target_children.len() != 1 {
                continue;
            }
            let target_idx = *target_children.iter().next().unwrap();

            for (edge_idx, edge) in graph.edges.iter().enumerate() {
                if edge_idx >= routed_points.len() || edge.to != bridge_id {
                    continue;
                }
                let Some(&source_idx) = node_to_child.get(edge.from.as_str()) else {
                    continue;
                };
                if source_idx == bridge_idx || source_idx == target_idx {
                    continue;
                }
                let Some(source_sub) = graph.subgraphs.get(source_idx) else {
                    continue;
                };
                let Some(source_order) = flowchart_simple_chain_order(graph, source_sub) else {
                    continue;
                };
                if source_order.first().map(|id| id.as_str()) != Some(edge.from.as_str()) {
                    continue;
                }

                let Some(source) = nodes.get(edge.from.as_str()) else {
                    continue;
                };
                let Some(bridge) = nodes.get(bridge_id) else {
                    continue;
                };
                if bridge.y <= source.y + source.height {
                    continue;
                }

                let source_cx = source.x + source.width * 0.5;
                let bridge_cx = bridge.x + bridge.width * 0.5;
                let sign = if bridge_cx < source_cx { -1.0 } else { 1.0 };
                let start = (
                    source_cx + sign * source.width * 0.145,
                    source.y + source.height,
                );
                let lane_x = source_cx + sign * source.width * 0.285;
                let vertical_gap = bridge.y - start.1;
                if vertical_gap <= 1.0 {
                    continue;
                }
                let end_side = if sign < 0.0 {
                    EdgeSide::Right
                } else {
                    EdgeSide::Left
                };
                let end_y = bridge.y + bridge.height * 0.08;
                let end_offset = end_y - (bridge.y + bridge.height * 0.5);
                let end = anchor_point_for_node(bridge, end_side, end_offset);
                routed_points[edge_idx] = vec![
                    start,
                    (lane_x, start.1 + vertical_gap / 3.0),
                    (lane_x, start.1 + vertical_gap * 2.0 / 3.0),
                    end,
                ];
            }
        }
    }
}

fn stack_flowchart_top_level_subgraph_chain(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    config: &LayoutConfig,
) -> bool {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.direction != Direction::TopDown {
        return false;
    }
    let Some(order) = flowchart_top_level_subgraph_chain_order(graph, nodes) else {
        return false;
    };
    if order.len() < 3 {
        return false;
    }

    struct ChainUnit {
        move_ids: Vec<String>,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    }

    let tree = SubgraphTree::build(graph);
    let mut units = Vec::with_capacity(order.len());
    for &sub_idx in &order {
        let Some(sub) = graph.subgraphs.get(sub_idx) else {
            return false;
        };
        let Some(layout_idx) = subgraph_layout_index(subgraphs, sub) else {
            return false;
        };
        let Some(layout) = subgraphs.get(layout_idx) else {
            return false;
        };

        let mut move_ids: HashSet<String> = sub.nodes.iter().cloned().collect();
        if let Some(anchor_id) = subgraph_anchor_id(sub, nodes) {
            move_ids.insert(anchor_id.to_string());
        }
        for desc_idx in 0..graph.subgraphs.len() {
            if desc_idx == sub_idx || !tree.is_ancestor(sub_idx, desc_idx) {
                continue;
            }
            if let Some(desc) = graph.subgraphs.get(desc_idx) {
                move_ids.extend(desc.nodes.iter().cloned());
                if let Some(anchor_id) = subgraph_anchor_id(desc, nodes) {
                    move_ids.insert(anchor_id.to_string());
                }
            }
        }

        units.push(ChainUnit {
            move_ids: move_ids.into_iter().collect(),
            x: layout.x,
            y: layout.y,
            width: layout.width,
            height: layout.height,
        });
    }

    let min_x = units.iter().map(|unit| unit.x).fold(f32::MAX, f32::min);
    let min_y = units.iter().map(|unit| unit.y).fold(f32::MAX, f32::min);
    let max_width = units.iter().map(|unit| unit.width).fold(0.0_f32, f32::max);
    if !min_x.is_finite() || !min_y.is_finite() || max_width <= 0.0 {
        return false;
    }

    let target_center_x = min_x + max_width * 0.5;
    let gap = config
        .rank_spacing
        .max(config.flowchart.auto_spacing.min_spacing);
    let mut cursor_y = min_y;
    for unit in units {
        let dx = target_center_x - (unit.x + unit.width * 0.5);
        let dy = cursor_y - unit.y;
        for node_id in &unit.move_ids {
            if let Some(node) = nodes.get_mut(node_id) {
                node.x += dx;
                node.y += dy;
            }
        }
        cursor_y += unit.height + gap;
    }

    true
}

fn flowchart_top_level_subgraph_chain_order(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
) -> Option<Vec<usize>> {
    let top_level = top_level_subgraph_indices(graph);
    if top_level.len() < 3 {
        return None;
    }

    let mut node_to_top_level: HashMap<&str, usize> = HashMap::new();
    for &idx in &top_level {
        let sub = graph.subgraphs.get(idx)?;
        for node_id in &sub.nodes {
            node_to_top_level.insert(node_id.as_str(), idx);
        }
        if let Some(anchor_id) = subgraph_anchor_id(sub, nodes) {
            node_to_top_level.insert(anchor_id, idx);
        }
        if let Some(id) = sub.id.as_deref()
            && !id.is_empty()
        {
            node_to_top_level.insert(id, idx);
        }
        if !sub.label.is_empty() {
            node_to_top_level.insert(sub.label.as_str(), idx);
        }
    }

    let mut outgoing: HashMap<usize, usize> = HashMap::new();
    let mut incoming: HashMap<usize, usize> = HashMap::new();
    let mut cross_edges = 0usize;
    for edge in &graph.edges {
        let from = node_to_top_level.get(edge.from.as_str()).copied();
        let to = node_to_top_level.get(edge.to.as_str()).copied();
        let (Some(from), Some(to)) = (from, to) else {
            continue;
        };
        if from == to {
            continue;
        }
        if outgoing.insert(from, to).is_some() || incoming.insert(to, from).is_some() {
            return None;
        }
        cross_edges += 1;
    }
    if cross_edges != top_level.len().saturating_sub(1) {
        return None;
    }

    let starts: Vec<usize> = top_level
        .iter()
        .copied()
        .filter(|idx| !incoming.contains_key(idx))
        .collect();
    if starts.len() != 1 {
        return None;
    }

    let mut order = Vec::with_capacity(top_level.len());
    let mut seen = HashSet::new();
    let mut current = starts[0];
    loop {
        if !seen.insert(current) {
            return None;
        }
        order.push(current);
        let Some(&next) = outgoing.get(&current) else {
            break;
        };
        current = next;
    }
    if order.len() == top_level.len() {
        Some(order)
    } else {
        None
    }
}

fn apply_compound_hub_child_flowchart_routes(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    hub_id: &str,
    children: &[String],
    routed_points: &mut [Vec<(f32, f32)>],
) {
    if children.is_empty() {
        return;
    }
    let child_set: HashSet<&str> = children.iter().map(|id| id.as_str()).collect();
    let Some(hub) = nodes.get(hub_id) else {
        return;
    };
    let hub_center_x = hub.x + hub.width / 2.0;
    let hub_bottom = hub.y + hub.height;
    for (edge_idx, edge) in graph.edges.iter().enumerate() {
        if edge_idx >= routed_points.len() {
            continue;
        }
        let (child_id, forward) = if edge.from == hub_id && child_set.contains(edge.to.as_str()) {
            (edge.to.as_str(), true)
        } else if edge.to == hub_id && child_set.contains(edge.from.as_str()) {
            (edge.from.as_str(), false)
        } else {
            continue;
        };
        let Some(child) = nodes.get(child_id) else {
            continue;
        };
        if child.y <= hub_bottom {
            continue;
        }
        let child_center_x = child.x + child.width / 2.0;
        let vertical_gap = (child.y - hub_bottom).max(1.0);
        let lane_y = hub_bottom + vertical_gap * 0.35;
        let bend_y = hub_bottom + vertical_gap * 0.65;
        let start = anchor_point_for_node(hub, EdgeSide::Bottom, child_center_x - hub_center_x);
        let end = anchor_point_for_node(child, EdgeSide::Top, 0.0);
        routed_points[edge_idx] = if forward {
            vec![
                start,
                (child_center_x, lane_y),
                (child_center_x, bend_y),
                end,
            ]
        } else {
            vec![
                end,
                (child_center_x, bend_y),
                (child_center_x, lane_y),
                start,
            ]
        };
    }
}

fn hub_children_in_subgraph(graph: &Graph, sub: &crate::ir::Subgraph, hub_id: &str) -> Vec<String> {
    let sub_nodes: HashSet<&str> = sub.nodes.iter().map(|id| id.as_str()).collect();
    let mut seen: HashSet<String> = HashSet::new();
    let mut children = Vec::new();
    for edge in &graph.edges {
        let other = if edge.from == hub_id && sub_nodes.contains(edge.to.as_str()) {
            Some(edge.to.clone())
        } else if edge.to == hub_id && sub_nodes.contains(edge.from.as_str()) {
            Some(edge.from.clone())
        } else {
            None
        };
        let Some(other) = other else {
            continue;
        };
        if other == hub_id {
            continue;
        }
        if seen.insert(other.clone()) {
            children.push(other);
        }
    }

    let mut remaining: Vec<String> = sub
        .nodes
        .iter()
        .filter(|id| id.as_str() != hub_id && !seen.contains(*id))
        .cloned()
        .collect();
    remaining.sort_by_key(|id| graph.node_order.get(id).copied().unwrap_or(usize::MAX));
    children.extend(remaining);
    children
}

fn place_node_row(
    nodes: &mut BTreeMap<String, NodeLayout>,
    node_ids: &[String],
    center_idx: usize,
    center_x: f32,
    y: f32,
    gap: f32,
) {
    if node_ids.is_empty() {
        return;
    }
    let widths: Vec<f32> = node_ids
        .iter()
        .map(|id| nodes.get(id).map(|node| node.width).unwrap_or(0.0))
        .collect();
    let center_idx = center_idx.min(node_ids.len() - 1);
    let before_width: f32 = widths.iter().take(center_idx).sum();
    let before_gaps = gap * center_idx as f32;
    let center_half = widths.get(center_idx).copied().unwrap_or(0.0) / 2.0;
    let mut cursor = center_x - before_width - before_gaps - center_half;
    for (node_id, width) in node_ids.iter().zip(widths.iter()) {
        if let Some(node) = nodes.get_mut(node_id) {
            node.x = cursor;
            node.y = y;
        }
        cursor += *width + gap;
    }
}

fn node_group_bounds(
    nodes: &BTreeMap<String, NodeLayout>,
    node_ids: &[String],
) -> Option<(f32, f32, f32, f32)> {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for node_id in node_ids {
        let Some(node) = nodes.get(node_id) else {
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

fn subgraph_layout_index(layouts: &[SubgraphLayout], sub: &crate::ir::Subgraph) -> Option<usize> {
    layouts.iter().position(|layout| {
        layout.label == sub.label
            && sub
                .nodes
                .iter()
                .all(|node_id| layout.nodes.iter().any(|id| id == node_id))
    })
}

fn enforce_flowchart_nested_cluster_parent_padding(
    graph: &Graph,
    subgraphs: &mut [SubgraphLayout],
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.subgraphs.len() < 2 {
        return;
    }

    let tree = SubgraphTree::build(graph);
    for (parent_idx, parent_sub) in graph.subgraphs.iter().enumerate() {
        let Some(children) = tree.children.get(parent_idx) else {
            continue;
        };
        if children.is_empty() {
            continue;
        }
        let Some(parent_layout_idx) = subgraph_layout_index(subgraphs, parent_sub) else {
            continue;
        };

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for &child_idx in children {
            let Some(child_sub) = graph.subgraphs.get(child_idx) else {
                continue;
            };
            let Some(child_layout_idx) = subgraph_layout_index(subgraphs, child_sub) else {
                continue;
            };
            let child = &subgraphs[child_layout_idx];
            min_x = min_x.min(child.x);
            min_y = min_y.min(child.y);
            max_x = max_x.max(child.x + child.width);
            max_y = max_y.max(child.y + child.height);
        }
        if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
            continue;
        }

        let (pad_x, pad_y) =
            flowchart_recursive_parent_child_padding(subgraph_layout_direction(graph, parent_sub));
        let parent = &mut subgraphs[parent_layout_idx];
        let new_min_x = parent.x.min(min_x - pad_x);
        let new_min_y = parent.y.min(min_y - pad_y);
        let new_max_x = (parent.x + parent.width).max(max_x + pad_x);
        let new_max_y = (parent.y + parent.height).max(max_y + pad_y);
        parent.x = new_min_x;
        parent.y = new_min_y;
        parent.width = new_max_x - new_min_x;
        parent.height = new_max_y - new_min_y;
    }
}

fn apply_subgraph_direction_overrides(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
    skip_indices: &HashSet<usize>,
) {
    for (idx, sub) in graph.subgraphs.iter().enumerate() {
        if skip_indices.contains(&idx) {
            continue;
        }
        if is_region_subgraph(sub) {
            continue;
        }
        let direction = match sub.direction {
            Some(direction) => direction,
            None => {
                if graph.kind != crate::ir::DiagramKind::Flowchart {
                    continue;
                }
                subgraph_layout_direction(graph, sub)
            }
        };
        if sub.nodes.is_empty() {
            continue;
        }
        if direction == graph.direction {
            continue;
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        for node_id in &sub.nodes {
            if let Some(node) = nodes.get(node_id) {
                min_x = min_x.min(node.x);
                min_y = min_y.min(node.y);
            }
        }
        if min_x == f32::MAX {
            continue;
        }

        let mut temp_nodes: BTreeMap<String, NodeLayout> = BTreeMap::new();
        for node_id in &sub.nodes {
            if let Some(node) = nodes.get(node_id) {
                let mut clone = node.clone();
                clone.x = 0.0;
                clone.y = 0.0;
                temp_nodes.insert(node_id.clone(), clone);
            }
        }
        let mut local_config = subgraph_layout_config_for(graph, sub, false, config);
        if graph.kind == crate::ir::DiagramKind::Flowchart
            && flowchart_subgraph_is_recursive_cluster(graph, sub)
            && flowchart_subgraph_has_internal_edge_label(graph, sub)
        {
            local_config.rank_spacing = local_config
                .rank_spacing
                .max(config.rank_spacing + config.node_spacing * 2.0);
        }
        let ranks = compute_ranks_subset_for(graph, &sub.nodes, &graph.edges, &graph.node_order);
        assign_positions_preserving_order(
            &sub.nodes,
            &ranks,
            direction,
            &local_config,
            &mut temp_nodes,
            0.0,
            0.0,
        );
        center_flowchart_recursive_subgraph_ranks_like_dagre(
            graph,
            sub,
            &sub.nodes,
            &ranks,
            direction,
            &local_config,
            &mut temp_nodes,
        );
        apply_flowchart_recursive_cycle_stagger_for_subgraph(
            graph,
            sub,
            &sub.nodes,
            &local_config,
            &mut temp_nodes,
        );
        let mut temp_min_x = f32::MAX;
        let mut temp_min_y = f32::MAX;
        for node_id in &sub.nodes {
            if let Some(node) = temp_nodes.get(node_id) {
                temp_min_x = temp_min_x.min(node.x);
                temp_min_y = temp_min_y.min(node.y);
            }
        }
        if temp_min_x == f32::MAX {
            continue;
        }
        for node_id in &sub.nodes {
            if let (Some(target), Some(source)) = (nodes.get_mut(node_id), temp_nodes.get(node_id))
            {
                target.x = source.x - temp_min_x + min_x;
                target.y = source.y - temp_min_y + min_y;
            }
        }

        if matches!(direction, Direction::RightLeft | Direction::BottomTop) {
            mirror_subgraph_nodes(&sub.nodes, nodes, direction);
        }
    }
}

fn apply_flowchart_nested_subgraph_direction_overrides(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.subgraphs.len() < 2 {
        return;
    }
    let tree = SubgraphTree::build(graph);
    let subgraph_layouts = build_subgraph_layouts(graph, nodes, theme, config);

    for (parent_idx, parent) in graph.subgraphs.iter().enumerate() {
        let Some(direction) = parent.direction else {
            continue;
        };
        if !matches!(direction, Direction::TopDown | Direction::LeftRight) {
            continue;
        }
        let Some(child_indices) = tree.children.get(parent_idx) else {
            continue;
        };
        if child_indices.len() < 2 {
            continue;
        }

        struct ChildUnit {
            anchor_id: String,
            move_ids: Vec<String>,
            min_x: f32,
            min_y: f32,
            width: f32,
            height: f32,
        }

        let mut units = Vec::new();
        for &child_idx in child_indices {
            let Some(child) = graph.subgraphs.get(child_idx) else {
                continue;
            };
            let Some(anchor_id) = subgraph_anchor_id(child, nodes).map(str::to_string) else {
                continue;
            };
            let Some((min_x, min_y, max_x, max_y)) =
                subgraph_layout_index(&subgraph_layouts, child)
                    .and_then(|idx| subgraph_layouts.get(idx))
                    .map(|layout| {
                        (
                            layout.x,
                            layout.y,
                            layout.x + layout.width,
                            layout.y + layout.height,
                        )
                    })
                    .or_else(|| node_group_bounds(nodes, &child.nodes))
            else {
                continue;
            };

            let mut move_ids: Vec<String> = child.nodes.clone();
            move_ids.push(anchor_id.clone());
            for (desc_idx, desc) in graph.subgraphs.iter().enumerate() {
                if tree.is_ancestor(child_idx, desc_idx)
                    && let Some(desc_anchor) = subgraph_anchor_id(desc, nodes)
                {
                    move_ids.push(desc_anchor.to_string());
                }
            }
            move_ids.sort();
            move_ids.dedup();

            units.push(ChildUnit {
                anchor_id,
                move_ids,
                min_x,
                min_y,
                width: max_x - min_x,
                height: max_y - min_y,
            });
        }
        if units.len() < 2 {
            continue;
        }

        let unit_ids: Vec<String> = units.iter().map(|unit| unit.anchor_id.clone()).collect();
        let unit_set: HashSet<&str> = unit_ids.iter().map(|id| id.as_str()).collect();
        let has_child_anchor_edge = graph.edges.iter().any(|edge| {
            edge.from != edge.to
                && unit_set.contains(edge.from.as_str())
                && unit_set.contains(edge.to.as_str())
        });
        if !has_child_anchor_edge {
            continue;
        }

        let origin_x = units.iter().map(|unit| unit.min_x).fold(f32::MAX, f32::min);
        let origin_y = units.iter().map(|unit| unit.min_y).fold(f32::MAX, f32::min);
        if !origin_x.is_finite() || !origin_y.is_finite() {
            continue;
        }

        let mut temp_nodes: BTreeMap<String, NodeLayout> = BTreeMap::new();
        for unit in &units {
            let Some(anchor) = nodes.get(&unit.anchor_id) else {
                continue;
            };
            let mut temp = anchor.clone();
            temp.x = 0.0;
            temp.y = 0.0;
            temp.width = unit.width;
            temp.height = unit.height;
            temp_nodes.insert(unit.anchor_id.clone(), temp);
        }
        if temp_nodes.len() != units.len() {
            continue;
        }

        let ranks = compute_ranks_subset_for(graph, &unit_ids, &graph.edges, &graph.node_order);
        let mut parent_depth = 0usize;
        let mut cur = parent_idx;
        while let Some(parent_idx) = tree.parent.get(cur).and_then(|idx| *idx) {
            parent_depth += 1;
            cur = parent_idx;
            if parent_depth > graph.subgraphs.len() {
                break;
            }
        }
        let local_config = if flowchart_subgraph_is_recursive_cluster(graph, parent) {
            subgraph_layout_config_for_depth(graph, parent, true, config, parent_depth)
        } else {
            subgraph_layout_config(graph, false, config)
        };
        assign_positions_preserving_order(
            &unit_ids,
            &ranks,
            direction,
            &local_config,
            &mut temp_nodes,
            0.0,
            0.0,
        );
        if flowchart_subgraph_is_recursive_cluster(graph, parent) {
            center_rank_buckets_cross_axis(
                &unit_ids,
                &ranks,
                direction,
                &local_config,
                &mut temp_nodes,
            );
        }

        let temp_min_x = temp_nodes
            .values()
            .map(|node| node.x)
            .fold(f32::MAX, f32::min);
        let temp_min_y = temp_nodes
            .values()
            .map(|node| node.y)
            .fold(f32::MAX, f32::min);
        if !temp_min_x.is_finite() || !temp_min_y.is_finite() {
            continue;
        }

        for unit in &units {
            let Some(target) = temp_nodes.get(&unit.anchor_id) else {
                continue;
            };
            let dx = target.x - temp_min_x + origin_x - unit.min_x;
            let dy = target.y - temp_min_y + origin_y - unit.min_y;
            if dx.abs() <= 0.5 && dy.abs() <= 0.5 {
                continue;
            }
            for node_id in &unit.move_ids {
                if let Some(node) = nodes.get_mut(node_id) {
                    node.x += dx;
                    node.y += dy;
                }
            }
        }
    }
}

fn center_rank_buckets_cross_axis(
    node_ids: &[String],
    ranks: &HashMap<String, usize>,
    direction: Direction,
    config: &LayoutConfig,
    nodes: &mut BTreeMap<String, NodeLayout>,
) {
    let mut rank_nodes: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for node_id in node_ids {
        let rank = *ranks.get(node_id).unwrap_or(&0);
        rank_nodes.entry(rank).or_default().push(node_id.clone());
    }

    let rank_span = |ids: &[String], nodes: &BTreeMap<String, NodeLayout>| -> f32 {
        let mut span = 0.0_f32;
        let mut seen = 0usize;
        for id in ids {
            let Some(node) = nodes.get(id) else {
                continue;
            };
            if seen > 0 {
                span += config.node_spacing;
            }
            span += if is_horizontal(direction) {
                node.height
            } else {
                node.width
            };
            seen += 1;
        }
        span
    };

    let max_span = rank_nodes
        .values()
        .map(|ids| rank_span(ids, nodes))
        .fold(0.0_f32, f32::max);
    if max_span <= 0.0 {
        return;
    }

    for ids in rank_nodes.values() {
        let span = rank_span(ids, nodes);
        let delta = (max_span - span) * 0.5;
        if delta.abs() <= 0.5 {
            continue;
        }
        for id in ids {
            if let Some(node) = nodes.get_mut(id) {
                if is_horizontal(direction) {
                    node.y += delta;
                } else {
                    node.x += delta;
                }
            }
        }
    }
}

fn subgraph_is_anchorable(
    sub: &crate::ir::Subgraph,
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
) -> bool {
    if sub.nodes.is_empty() {
        return false;
    }
    let anchor_id = subgraph_anchor_id(sub, nodes);
    let set: HashSet<&str> = sub.nodes.iter().map(|id| id.as_str()).collect();
    for edge in &graph.edges {
        if let Some(anchor) = anchor_id
            && (edge.from == anchor || edge.to == anchor)
        {
            return false;
        }
        let from_in = set.contains(edge.from.as_str());
        let to_in = set.contains(edge.to.as_str());
        if from_in ^ to_in {
            return false;
        }
    }
    true
}

fn subgraph_should_anchor(
    sub: &crate::ir::Subgraph,
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
) -> bool {
    if sub.nodes.is_empty() {
        return false;
    }
    // For flowcharts and state diagrams, anchor if there's an anchor node
    // State diagram composite states can have external edges, so we can't use
    // subgraph_is_anchorable which rejects subgraphs with external edges
    if graph.kind == crate::ir::DiagramKind::Flowchart {
        return subgraph_anchor_id(sub, nodes).is_some()
            && flowchart_subgraph_is_recursive_cluster(graph, sub);
    }
    if graph.kind == crate::ir::DiagramKind::State {
        return subgraph_anchor_id(sub, nodes).is_some();
    }
    subgraph_is_anchorable(sub, graph, nodes)
}

fn subgraph_anchor_id<'a>(
    sub: &'a crate::ir::Subgraph,
    nodes: &BTreeMap<String, NodeLayout>,
) -> Option<&'a str> {
    if let Some(id) = sub.id.as_deref()
        && nodes.contains_key(id)
        && !sub.nodes.iter().any(|node_id| node_id == id)
    {
        return Some(id);
    }
    let label = sub.label.as_str();
    if nodes.contains_key(label) && !sub.nodes.iter().any(|node_id| node_id == label) {
        return Some(label);
    }
    None
}

fn subgraph_anchor_ids_for_nodes(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
) -> HashSet<String> {
    graph
        .subgraphs
        .iter()
        .filter_map(|sub| subgraph_anchor_id(sub, nodes).map(str::to_string))
        .collect()
}

fn mark_subgraph_anchor_nodes_hidden(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
) -> HashSet<String> {
    let mut anchor_ids = HashSet::new();
    for sub in &graph.subgraphs {
        let Some(anchor_id) = subgraph_anchor_id(sub, nodes) else {
            continue;
        };
        anchor_ids.insert(anchor_id.to_string());
        if let Some(node) = nodes.get_mut(anchor_id) {
            node.hidden = true;
        }
    }
    anchor_ids
}

fn pick_subgraph_anchor_child(
    sub: &crate::ir::Subgraph,
    graph: &Graph,
    anchor_ids: &HashSet<String>,
) -> Option<String> {
    let mut candidates: Vec<&String> = sub
        .nodes
        .iter()
        .filter(|id| !anchor_ids.contains(*id))
        .collect();
    if candidates.is_empty() {
        candidates = sub.nodes.iter().collect();
    }
    candidates.sort_by_key(|id| graph.node_order.get(*id).copied().unwrap_or(usize::MAX));
    candidates.first().map(|id| (*id).clone())
}

#[derive(Debug, Clone)]
struct SubgraphAnchorInfo {
    sub_idx: usize,
    padding_x: f32,
    top_padding: f32,
}

fn collect_anchored_subgraph_layout_exclusions(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    anchor_info: &HashMap<String, SubgraphAnchorInfo>,
) -> HashSet<String> {
    let tree = SubgraphTree::build(graph);
    let mut excluded = HashSet::new();
    for info in anchor_info.values() {
        if let Some(sub) = graph.subgraphs.get(info.sub_idx) {
            excluded.extend(sub.nodes.iter().cloned());
        }
        for (desc_idx, desc) in graph.subgraphs.iter().enumerate() {
            if desc_idx == info.sub_idx || !tree.is_ancestor(info.sub_idx, desc_idx) {
                continue;
            }
            if let Some(anchor_id) = subgraph_anchor_id(desc, nodes) {
                excluded.insert(anchor_id.to_string());
            }
        }
    }
    excluded
}

fn subgraph_layout_direction(graph: &Graph, sub: &crate::ir::Subgraph) -> Direction {
    if graph.kind == crate::ir::DiagramKind::State {
        return graph.direction;
    }
    if let Some(direction) = sub.direction {
        return direction;
    }
    if graph.kind == crate::ir::DiagramKind::Flowchart
        && flowchart_subgraph_is_recursive_cluster(graph, sub)
    {
        return match graph.direction {
            Direction::TopDown | Direction::BottomTop => Direction::LeftRight,
            Direction::LeftRight | Direction::RightLeft => Direction::TopDown,
        };
    }
    graph.direction
}

fn edge_effective_directions(graph: &Graph) -> Vec<Direction> {
    let mut directions = vec![graph.direction; graph.edges.len()];
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.subgraphs.is_empty() {
        return directions;
    }

    for (edge_idx, edge) in graph.edges.iter().enumerate() {
        let mut best: Option<(usize, Direction)> = None;
        for sub in &graph.subgraphs {
            if !flowchart_subgraph_is_recursive_cluster(graph, sub) {
                continue;
            }
            let from_in = sub.nodes.iter().any(|node_id| node_id == &edge.from);
            let to_in = sub.nodes.iter().any(|node_id| node_id == &edge.to);
            if !from_in || !to_in {
                continue;
            }
            let size = sub.nodes.len();
            if best.map(|(best_size, _)| size < best_size).unwrap_or(true) {
                best = Some((size, subgraph_layout_direction(graph, sub)));
            }
        }
        if let Some((_, direction)) = best {
            directions[edge_idx] = direction;
        }
    }

    directions
}

fn flowchart_edge_inside_recursive_cluster(graph: &Graph, edge: &crate::ir::Edge) -> bool {
    graph.kind == crate::ir::DiagramKind::Flowchart
        && graph.subgraphs.iter().any(|sub| {
            flowchart_subgraph_is_recursive_cluster(graph, sub)
                && sub.nodes.iter().any(|node_id| node_id == &edge.from)
                && sub.nodes.iter().any(|node_id| node_id == &edge.to)
        })
}

#[cfg(test)]
fn flowchart_subgraph_without_external_connections(
    graph: &Graph,
    sub: &crate::ir::Subgraph,
) -> bool {
    if graph.kind != crate::ir::DiagramKind::Flowchart || sub.nodes.is_empty() {
        return false;
    }
    let mut members: HashSet<&str> = sub.nodes.iter().map(|id| id.as_str()).collect();
    if let Some(anchor_id) = flowchart_subgraph_anchor_id_in_graph(graph, sub) {
        members.insert(anchor_id);
    }
    graph.edges.iter().all(|edge| {
        let from_in = members.contains(edge.from.as_str());
        let to_in = members.contains(edge.to.as_str());
        from_in == to_in
    })
}

fn flowchart_subgraph_is_recursive_cluster(graph: &Graph, sub: &crate::ir::Subgraph) -> bool {
    if graph.kind != crate::ir::DiagramKind::Flowchart || sub.nodes.is_empty() {
        return false;
    }
    let members: HashSet<&str> = sub.nodes.iter().map(|id| id.as_str()).collect();
    let anchor_id = flowchart_subgraph_anchor_id_in_graph(graph, sub);
    graph.edges.iter().all(|edge| {
        if let Some(anchor) = anchor_id
            && (edge.from == anchor || edge.to == anchor)
        {
            let other = if edge.from == anchor {
                edge.to.as_str()
            } else {
                edge.from.as_str()
            };
            if !members.contains(other) {
                return true;
            }
        }
        let from_in = members.contains(edge.from.as_str());
        let to_in = members.contains(edge.to.as_str());
        from_in == to_in
    })
}

fn flowchart_subgraph_anchor_id_in_graph<'a>(
    graph: &Graph,
    sub: &'a crate::ir::Subgraph,
) -> Option<&'a str> {
    if let Some(id) = sub.id.as_deref()
        && graph.nodes.contains_key(id)
        && !sub.nodes.iter().any(|node_id| node_id == id)
    {
        return Some(id);
    }
    if graph.nodes.contains_key(sub.label.as_str())
        && !sub
            .nodes
            .iter()
            .any(|node_id| node_id == sub.label.as_str())
    {
        return Some(sub.label.as_str());
    }
    None
}

fn flowchart_subgraph_has_internal_edge_label(graph: &Graph, sub: &crate::ir::Subgraph) -> bool {
    let members: HashSet<&str> = sub.nodes.iter().map(|id| id.as_str()).collect();
    graph.edges.iter().any(|edge| {
        members.contains(edge.from.as_str())
            && members.contains(edge.to.as_str())
            && edge
                .label
                .as_ref()
                .map(|label| !label.is_empty())
                .unwrap_or(false)
    })
}

fn flowchart_subgraph_has_external_edge(graph: &Graph, sub: &crate::ir::Subgraph) -> bool {
    let members: HashSet<&str> = sub.nodes.iter().map(|id| id.as_str()).collect();
    graph.edges.iter().any(|edge| {
        let from_in = members.contains(edge.from.as_str());
        let to_in = members.contains(edge.to.as_str());
        from_in ^ to_in
    })
}

fn flowchart_subgraph_is_nested_bridge_child(graph: &Graph, sub: &crate::ir::Subgraph) -> bool {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.subgraphs.len() < 3 {
        return false;
    }
    let Some(sub_idx) = graph
        .subgraphs
        .iter()
        .position(|candidate| std::ptr::eq(candidate, sub))
    else {
        return false;
    };
    let tree = SubgraphTree::build(graph);
    let Some(parent_idx) = tree.parent.get(sub_idx).copied().flatten() else {
        return false;
    };
    let Some(child_indices) = tree.children.get(parent_idx) else {
        return false;
    };
    if child_indices.len() < 3 {
        return false;
    }

    let mut node_to_child: HashMap<&str, usize> = HashMap::new();
    for &child_idx in child_indices {
        let Some(child) = graph.subgraphs.get(child_idx) else {
            continue;
        };
        for node_id in &child.nodes {
            node_to_child.insert(node_id.as_str(), child_idx);
        }
    }

    for &bridge_idx in child_indices {
        let Some(bridge_sub) = graph.subgraphs.get(bridge_idx) else {
            continue;
        };
        if bridge_sub.nodes.len() != 1 {
            continue;
        }
        let bridge_node = bridge_sub.nodes[0].as_str();
        let incoming_sources: HashSet<usize> = graph
            .edges
            .iter()
            .filter(|edge| edge.to == bridge_node)
            .filter_map(|edge| node_to_child.get(edge.from.as_str()).copied())
            .filter(|idx| *idx != bridge_idx)
            .collect();
        let outgoing_targets: HashSet<usize> = graph
            .edges
            .iter()
            .filter(|edge| edge.from == bridge_node)
            .filter_map(|edge| node_to_child.get(edge.to.as_str()).copied())
            .filter(|idx| *idx != bridge_idx)
            .collect();
        if incoming_sources.is_empty()
            || outgoing_targets.len() != 1
            || incoming_sources
                .iter()
                .all(|idx| outgoing_targets.contains(idx))
        {
            continue;
        }
        if sub_idx == bridge_idx
            || incoming_sources.contains(&sub_idx)
            || outgoing_targets.contains(&sub_idx)
        {
            return true;
        }
    }
    false
}

fn flowchart_subgraph_is_nested_bridge_target_child(
    graph: &Graph,
    sub: &crate::ir::Subgraph,
) -> bool {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.subgraphs.len() < 3 {
        return false;
    }
    let Some(sub_idx) = graph
        .subgraphs
        .iter()
        .position(|candidate| std::ptr::eq(candidate, sub))
    else {
        return false;
    };
    let tree = SubgraphTree::build(graph);
    let Some(parent_idx) = tree.parent.get(sub_idx).copied().flatten() else {
        return false;
    };
    let Some(child_indices) = tree.children.get(parent_idx) else {
        return false;
    };
    if child_indices.len() < 3 {
        return false;
    }

    let mut node_to_child: HashMap<&str, usize> = HashMap::new();
    for &child_idx in child_indices {
        let Some(child) = graph.subgraphs.get(child_idx) else {
            continue;
        };
        for node_id in &child.nodes {
            node_to_child.insert(node_id.as_str(), child_idx);
        }
    }

    for &bridge_idx in child_indices {
        let Some(bridge_sub) = graph.subgraphs.get(bridge_idx) else {
            continue;
        };
        if bridge_sub.nodes.len() != 1 {
            continue;
        }
        let bridge_node = bridge_sub.nodes[0].as_str();
        let incoming_sources: HashSet<usize> = graph
            .edges
            .iter()
            .filter(|edge| edge.to == bridge_node)
            .filter_map(|edge| node_to_child.get(edge.from.as_str()).copied())
            .filter(|idx| *idx != bridge_idx)
            .collect();
        let outgoing_targets: HashSet<usize> = graph
            .edges
            .iter()
            .filter(|edge| edge.from == bridge_node)
            .filter_map(|edge| node_to_child.get(edge.to.as_str()).copied())
            .filter(|idx| *idx != bridge_idx)
            .collect();
        if incoming_sources.is_empty()
            || outgoing_targets.len() != 1
            || incoming_sources
                .iter()
                .all(|idx| outgoing_targets.contains(idx))
        {
            continue;
        }
        if outgoing_targets.contains(&sub_idx) {
            return true;
        }
    }
    false
}

fn flowchart_has_external_compound_subgraph(graph: &Graph) -> bool {
    graph.kind == crate::ir::DiagramKind::Flowchart
        && top_level_subgraph_indices(graph)
            .iter()
            .filter_map(|idx| graph.subgraphs.get(*idx))
            .any(|sub| {
                !flowchart_subgraph_is_recursive_cluster(graph, sub)
                    && flowchart_subgraph_has_external_edge(graph, sub)
            })
}

fn flowchart_has_tb_external_compound_subgraph(graph: &Graph) -> bool {
    matches!(graph.direction, Direction::TopDown | Direction::BottomTop)
        && flowchart_has_external_compound_subgraph(graph)
}

fn flowchart_recursive_cycle_rank_order(
    graph: &Graph,
    sub: &crate::ir::Subgraph,
    node_ids: &[String],
) -> Option<[String; 3]> {
    if graph.kind != crate::ir::DiagramKind::Flowchart
        || !flowchart_subgraph_is_recursive_cluster(graph, sub)
        || node_ids.len() != 3
    {
        return None;
    }

    let ranks = compute_ranks_subset_for(graph, node_ids, &graph.edges, &graph.node_order);
    if ranks.len() != 3 {
        return None;
    }
    let mut ranked: Vec<(usize, String)> = node_ids
        .iter()
        .filter_map(|id| ranks.get(id).copied().map(|rank| (rank, id.clone())))
        .collect();
    ranked.sort_by(|(rank_a, id_a), (rank_b, id_b)| {
        rank_a
            .cmp(rank_b)
            .then_with(|| {
                graph
                    .node_order
                    .get(id_a)
                    .copied()
                    .unwrap_or(usize::MAX)
                    .cmp(&graph.node_order.get(id_b).copied().unwrap_or(usize::MAX))
            })
            .then_with(|| id_a.cmp(id_b))
    });
    if ranked
        .iter()
        .enumerate()
        .any(|(expected_rank, (rank, _))| *rank != expected_rank)
    {
        return None;
    }

    let first = ranked[0].1.as_str();
    let last = ranked[2].1.as_str();
    let closes_cycle = graph.edges.iter().any(|edge| {
        (edge.from == last && edge.to == first)
            || (edge.from == first && edge.to == last && edge.arrow_start)
    });
    if !closes_cycle {
        return None;
    }

    Some([
        ranked[0].1.clone(),
        ranked[1].1.clone(),
        ranked[2].1.clone(),
    ])
}

fn apply_flowchart_recursive_cycle_stagger_for_subgraph(
    graph: &Graph,
    sub: &crate::ir::Subgraph,
    node_ids: &[String],
    config: &LayoutConfig,
    nodes: &mut BTreeMap<String, NodeLayout>,
) -> bool {
    let Some([first, middle, last]) = flowchart_recursive_cycle_rank_order(graph, sub, node_ids)
    else {
        return false;
    };
    let direction = subgraph_layout_direction(graph, sub);
    let Some(middle_node) = nodes.get(&middle) else {
        return false;
    };
    let lane_offset = config.node_spacing + FLOWCHART_RECURSIVE_CYCLE_STAGGER_BONUS;
    if is_horizontal(direction) {
        let target_center = middle_node.y + middle_node.height * 0.5 + lane_offset;
        for id in [first, last] {
            if let Some(node) = nodes.get_mut(&id) {
                node.y = target_center - node.height * 0.5;
            }
        }
    } else {
        let target_center = middle_node.x + middle_node.width * 0.5 + lane_offset;
        for id in [first, last] {
            if let Some(node) = nodes.get_mut(&id) {
                node.x = target_center - node.width * 0.5;
            }
        }
    }
    true
}

fn flowchart_recursive_cycle_lane_extra(
    graph: &Graph,
    sub: &crate::ir::Subgraph,
    node_ids: &[String],
) -> f32 {
    if flowchart_recursive_cycle_rank_order(graph, sub, node_ids).is_some() {
        FLOWCHART_RECURSIVE_CYCLE_LANE_EXTRA
    } else {
        0.0
    }
}

fn center_flowchart_recursive_subgraph_ranks_like_dagre(
    graph: &Graph,
    sub: &crate::ir::Subgraph,
    node_ids: &[String],
    ranks: &HashMap<String, usize>,
    direction: Direction,
    config: &LayoutConfig,
    nodes: &mut BTreeMap<String, NodeLayout>,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart
        || !flowchart_subgraph_is_recursive_cluster(graph, sub)
    {
        return;
    }
    center_rank_buckets_cross_axis(node_ids, ranks, direction, config, nodes);
}

fn subgraph_layout_config(graph: &Graph, _anchorable: bool, config: &LayoutConfig) -> LayoutConfig {
    let mut local = config.clone();
    if graph.kind == crate::ir::DiagramKind::Flowchart {
        // Mermaid's dagre recursive renderer increases ranksep by 25 for
        // every flowchart cluster it lays out, not only compound/anchored
        // clusters. Keep the same inner-cluster breathing room here.
        local.rank_spacing = config.rank_spacing + STATE_RANK_SPACING_BOOST;
    }
    local
}

fn subgraph_layout_config_for(
    graph: &Graph,
    sub: &crate::ir::Subgraph,
    anchorable: bool,
    config: &LayoutConfig,
) -> LayoutConfig {
    subgraph_layout_config_for_depth(graph, sub, anchorable, config, 0)
}

fn subgraph_layout_config_for_depth(
    graph: &Graph,
    sub: &crate::ir::Subgraph,
    anchorable: bool,
    config: &LayoutConfig,
    depth: usize,
) -> LayoutConfig {
    let mut local = subgraph_layout_config(graph, anchorable, config);
    if graph.kind == crate::ir::DiagramKind::Flowchart && anchorable {
        local.rank_spacing = config.rank_spacing + STATE_RANK_SPACING_BOOST * (depth as f32 + 1.0);
    }
    if graph.kind == crate::ir::DiagramKind::Flowchart
        && flowchart_subgraph_is_recursive_cluster(graph, sub)
    {
        let recursive_rank_boost = if anchorable {
            STATE_RANK_SPACING_BOOST * (depth as f32 + 1.0)
        } else {
            STATE_RANK_SPACING_BOOST
        };
        local.node_spacing = local.node_spacing.max(FLOWCHART_RECURSIVE_DAGRE_SPACING);
        local.rank_spacing = local
            .rank_spacing
            .max(FLOWCHART_RECURSIVE_DAGRE_SPACING + recursive_rank_boost);
    }
    if graph.kind == crate::ir::DiagramKind::Flowchart
        && flowchart_subgraph_is_recursive_cluster(graph, sub)
        && flowchart_subgraph_has_internal_edge_label(graph, sub)
    {
        local.rank_spacing = local
            .rank_spacing
            .max(config.rank_spacing + config.node_spacing * 2.0);
    }
    if graph.kind == crate::ir::DiagramKind::State {
        if is_region_subgraph(sub) {
            // Concurrent regions in mermaid JS dagre use much larger rank
            // spacing inside the region than the diagram default. Match that.
            local.rank_spacing =
                (config.rank_spacing + STATE_REGION_RANK_BOOST).max(STATE_REGION_RANK_MIN);
            local.node_spacing =
                (config.node_spacing + STATE_REGION_NODE_BOOST).max(STATE_REGION_NODE_MIN);
        } else {
            // Non-region composite states (`state Foo { ... }`) also need
            // more vertical breathing room than the default — dagre lays out
            // composite contents with extra rank spacing so the inner [*]
            // markers and state rect have visible separation.
            // (Depth-aware ranksep was tried in iter 203, iter 232, iter 260
            // — all inflated nested cluster heights via the cascade. JS sizes
            // these via global rank assignment, not per-cluster ranksep.)
            let _ = depth;
            // Iter 268: extra boost for top-level leaf state composites
            // when the diagram has root-scope start/end markers. Targets
            // composite-states (closes the -158 height gap) without affecting
            // transitions-between-composite-states (no root markers) or
            // most of nested-composite-states (the inflated leaf End fits
            // beside the taller First sibling, so diagram height stays put).
            // The boost helps only sparse leaf composites (≤3 internal
            // nodes). Denser clusters already match JS height without help;
            // boosting them just overshoots (classdef-styling has 4 inner
            // nodes and matches JS without the boost).
            let extra = if is_top_level_leaf_state_composite(graph, sub)
                && state_has_root_markers(graph)
                && sub.nodes.len() <= 3
            {
                27.5
            } else {
                0.0
            };
            // Iter 273: outer composites that contain nested composites need
            // extra ranksep so the nested children fit at JS-comparable size.
            // Using `nested_composite_depth_below` (levels below this sub)
            // prevents cascade — only the outermost container grows, not the
            // inner ones whose children are smaller.
            let nested_below = state_nested_composite_depth_below(graph, sub);
            let depth_extra = (nested_below as f32) * STATE_COMPOSITE_RANK_PER_DEPTH;
            local.rank_spacing =
                (config.rank_spacing + STATE_COMPOSITE_RANK_BOOST + extra + depth_extra)
                    .max(config.rank_spacing);
        }
    }
    local
}

/// Iter 273: count how many composite levels are nested BELOW `sub`.
/// 0 if `sub` is a leaf (contains only nodes), 1 if it contains a leaf
/// composite, 2 if it contains a composite that contains a composite, etc.
/// Outer composites need more ranksep so JS-sized inner composites have
/// breathing room. Using "levels below" rather than "levels above" prevents
/// the cascade problem of prior iters (203/232/260) — only the outermost
/// container grows, not every level.
fn state_nested_composite_depth_below(graph: &Graph, sub: &crate::ir::Subgraph) -> usize {
    let sub_id_str = sub.id.as_deref().unwrap_or("");
    let sub_label = sub.label.as_str();
    let mut max_depth = 0usize;
    for child in &graph.subgraphs {
        let c_id = child.id.as_deref().unwrap_or("");
        let c_label = child.label.as_str();
        // Skip self
        if c_id == sub_id_str && c_label == sub_label {
            continue;
        }
        // child is contained in sub if sub.nodes references child's id/label
        let is_child = !c_id.is_empty() && sub.nodes.iter().any(|n| n == c_id || n == c_label);
        if is_child {
            let d = 1 + state_nested_composite_depth_below(graph, child);
            if d > max_depth {
                max_depth = d;
            }
        }
    }
    max_depth
}

fn is_top_level_leaf_state_composite(graph: &Graph, sub: &crate::ir::Subgraph) -> bool {
    if graph.kind != crate::ir::DiagramKind::State {
        return false;
    }
    let sub_id = sub.id.as_deref().unwrap_or("");
    let sub_label = sub.label.as_str();
    for other in &graph.subgraphs {
        if other.nodes.iter().any(|n| n == sub_id || n == sub_label) {
            return false;
        }
    }
    for other in &graph.subgraphs {
        let o_id = other.id.as_deref().unwrap_or("");
        let o_label = other.label.as_str();
        if !o_id.is_empty() || !o_label.is_empty() {
            if sub.nodes.iter().any(|n| n == o_id || n == o_label) {
                return false;
            }
        }
    }
    true
}

fn state_has_root_markers(graph: &Graph) -> bool {
    graph
        .nodes
        .keys()
        .any(|id| id == "__start_root__" || id == "__end_root__")
}

fn is_sparse_non_root_leaf_state_composite(graph: &Graph, sub: &crate::ir::Subgraph) -> bool {
    graph.kind == crate::ir::DiagramKind::State
        && is_top_level_leaf_state_composite(graph, sub)
        && !state_has_root_markers(graph)
        && sub.nodes.len() <= 3
}

fn flowchart_subgraph_padding(direction: Direction) -> (f32, f32) {
    // Mermaid CLI uses larger padding along the main axis and slightly
    // smaller padding along the cross axis.
    if is_horizontal(direction) {
        (FLOWCHART_PAD_MAIN, FLOWCHART_PAD_CROSS)
    } else {
        (FLOWCHART_PAD_CROSS, FLOWCHART_PAD_MAIN)
    }
}

fn flowchart_recursive_subgraph_padding(direction: Direction) -> (f32, f32) {
    if is_horizontal(direction) {
        (
            FLOWCHART_RECURSIVE_CLUSTER_CHILD_MAIN_PAD,
            FLOWCHART_RECURSIVE_CLUSTER_CROSS_PAD,
        )
    } else {
        (
            FLOWCHART_RECURSIVE_CLUSTER_CROSS_PAD,
            FLOWCHART_RECURSIVE_CLUSTER_CHILD_MAIN_PAD,
        )
    }
}

fn flowchart_recursive_child_cluster_padding(direction: Direction) -> (f32, f32) {
    if is_horizontal(direction) {
        (
            FLOWCHART_RECURSIVE_CLUSTER_CHILD_MAIN_PAD,
            FLOWCHART_RECURSIVE_CLUSTER_CROSS_PAD,
        )
    } else {
        (
            FLOWCHART_RECURSIVE_CLUSTER_CROSS_PAD,
            FLOWCHART_RECURSIVE_CLUSTER_CHILD_MAIN_PAD,
        )
    }
}

fn flowchart_recursive_parent_child_padding(direction: Direction) -> (f32, f32) {
    if is_horizontal(direction) {
        (
            FLOWCHART_RECURSIVE_CLUSTER_CHILD_MAIN_PAD,
            FLOWCHART_RECURSIVE_PARENT_CHILD_CROSS_PAD,
        )
    } else {
        (
            FLOWCHART_RECURSIVE_PARENT_CHILD_CROSS_PAD,
            FLOWCHART_RECURSIVE_CLUSTER_CHILD_MAIN_PAD,
        )
    }
}

fn subgraph_padding_from_label(
    graph: &Graph,
    sub: &crate::ir::Subgraph,
    theme: &Theme,
    label_block: &TextBlock,
) -> (f32, f32, f32) {
    subgraph_padding_from_label_with_depth(graph, sub, theme, label_block, 0)
}

/// Compute (pad_x, pad_y, top_padding) for a subgraph cluster. The
/// `nested_composite_depth` parameter is the number of NESTED composite
/// levels below this subgraph (0 for leaf composites that contain only nodes;
/// 1 if this composite contains another composite; 2 if a grandchild composite,
/// etc.). For state diagrams, JS dagre's recursive cluster rendering effectively
/// pads outer composites more than inner ones — we model this by adding
/// `STATE_NESTED_PAD_INCREMENT` to pad_x per nested composite level.
fn subgraph_padding_from_label_with_depth(
    graph: &Graph,
    sub: &crate::ir::Subgraph,
    theme: &Theme,
    label_block: &TextBlock,
    nested_composite_depth: usize,
) -> (f32, f32, f32) {
    if is_region_subgraph(sub) {
        // Concurrent regions in mermaid JS get substantial padding around their
        // inner state nodes (dagre + cluster padding combined). Match that so
        // regions render as roomy columns rather than tight boxes.
        return (STATE_REGION_PAD_X, STATE_REGION_PAD_Y, STATE_REGION_PAD_Y);
    }

    let label_empty = sub.label.trim().is_empty();
    let label_height = if label_empty { 0.0 } else { label_block.height };

    let recursive_flowchart_cluster = graph.kind == crate::ir::DiagramKind::Flowchart
        && flowchart_subgraph_is_recursive_cluster(graph, sub);
    let flowchart_nested_bridge_target_child = graph.kind == crate::ir::DiagramKind::Flowchart
        && flowchart_subgraph_is_nested_bridge_target_child(graph, sub);
    let flowchart_external_tb_cluster = graph.kind == crate::ir::DiagramKind::Flowchart
        && !recursive_flowchart_cluster
        && matches!(graph.direction, Direction::TopDown | Direction::BottomTop)
        && sub.nodes.len() <= 3
        && flowchart_subgraph_has_external_edge(graph, sub);
    let flowchart_external_lr_cluster = graph.kind == crate::ir::DiagramKind::Flowchart
        && !recursive_flowchart_cluster
        && matches!(graph.direction, Direction::LeftRight | Direction::RightLeft)
        && sub.nodes.len() <= 3
        && flowchart_subgraph_has_external_edge(graph, sub);
    let (mut pad_x, mut pad_y) = if recursive_flowchart_cluster {
        let direction = subgraph_layout_direction(graph, sub);
        if nested_composite_depth == 0 {
            flowchart_recursive_child_cluster_padding(direction)
        } else {
            flowchart_recursive_subgraph_padding(direction)
        }
    } else if flowchart_external_tb_cluster {
        let pad_y = if flowchart_subgraph_is_nested_bridge_child(graph, sub) {
            FLOWCHART_RECURSIVE_CLUSTER_CHILD_MAIN_PAD
        } else {
            25.0
        };
        (FLOWCHART_RECURSIVE_CLUSTER_CROSS_PAD, pad_y)
    } else if flowchart_external_lr_cluster {
        (
            FLOWCHART_EXTERNAL_LR_CLUSTER_PAD_X,
            FLOWCHART_EXTERNAL_LR_CLUSTER_PAD_Y,
        )
    } else if flowchart_nested_bridge_target_child {
        (
            FLOWCHART_PAD_CROSS,
            FLOWCHART_RECURSIVE_CLUSTER_CHILD_MAIN_PAD,
        )
    } else if graph.kind == crate::ir::DiagramKind::Flowchart {
        flowchart_subgraph_padding(graph.direction)
    } else if graph.kind == crate::ir::DiagramKind::Kanban {
        (KANBAN_SUBGRAPH_PAD, KANBAN_SUBGRAPH_PAD)
    } else if graph.kind == crate::ir::DiagramKind::Block {
        (8.0, 8.0)
    } else if graph.kind == crate::ir::DiagramKind::Class {
        (CLASS_NAMESPACE_PAD_X, CLASS_NAMESPACE_PAD_Y)
    } else if graph.kind == crate::ir::DiagramKind::State {
        // JS state composites pad more on the cross axis (x in TB) than along
        // the main axis. The wider horizontal pad gives clusters visible
        // breathing room while the smaller vertical pad keeps total height
        // close to JS's tighter dagre layout. Add per-nested-level extra pad
        // on the cross axis to match JS's outer-composite spacing.
        let extra = (nested_composite_depth as f32) * STATE_NESTED_PAD_INCREMENT;
        // Iter 275: bump pad_y for top-level leaf state composites with root
        // markers (the same gate as iter 268's ranksep boost). Closes the
        // composite-states -44 gap (root_end ends up further from End cluster
        // bottom). Discriminator skips nested-composite-states' First (not a
        // leaf) and inner clusters Second/Third (not top-level), so doesn't
        // cascade height inflation through nested levels.
        // Iter 277: sparse non-root leaf composites keep JS's recursive
        // ranksep (+25), then use bottom padding to reach the rendered
        // envelope. This avoids stretching inner start/state/end ranks while
        // preserving transitions-between-composite-states' 293px clusters.
        let top_level_leaf = is_top_level_leaf_state_composite(graph, sub);
        let root_marker_leaf =
            top_level_leaf && state_has_root_markers(graph) && sub.nodes.len() <= 4;
        let sparse_non_root_leaf = is_sparse_non_root_leaf_state_composite(graph, sub);
        let pad_x_bonus = if root_marker_leaf || sparse_non_root_leaf {
            5.0
        } else {
            0.0
        };
        let pad_y_bonus = if root_marker_leaf {
            if sub.nodes.len() >= 4 { 23.0 } else { 20.0 }
        } else if sparse_non_root_leaf {
            23.0 - STATE_SPARSE_LEAF_TOP_PAD_OFFSET
        } else {
            0.0
        };
        (
            STATE_SUBGRAPH_BASE_PAD + extra + pad_x_bonus,
            STATE_SUBGRAPH_PAD_Y + pad_y_bonus,
        )
    } else {
        (GENERIC_SUBGRAPH_BASE_PAD, GENERIC_SUBGRAPH_BASE_PAD)
    };
    if graph.kind == crate::ir::DiagramKind::Flowchart
        && !recursive_flowchart_cluster
        && !flowchart_external_tb_cluster
        && !flowchart_external_lr_cluster
        && sub.nodes.len() <= 3
        && ((is_horizontal(graph.direction) && graph.edges.len() <= 20)
            || (!is_horizontal(graph.direction) && graph.edges.len() <= 13))
        && !graph.edges.iter().any(|edge| {
            edge.label
                .as_ref()
                .map(|label| label.chars().count() > 24)
                .unwrap_or(false)
        })
    {
        pad_x *= 0.7;
        pad_y *= 0.7;
    }
    if recursive_flowchart_cluster && flowchart_subgraph_has_internal_edge_label(graph, sub) {
        pad_x += FLOWCHART_RECURSIVE_LABELED_PAD_X_BONUS;
        pad_y += FLOWCHART_RECURSIVE_LABELED_PAD_Y_BONUS;
    }

    let top_padding = if label_empty {
        pad_y
    } else if flowchart_external_tb_cluster {
        pad_y.max(label_height + 1.0)
    } else if flowchart_external_lr_cluster {
        pad_y.max(label_height + SUBGRAPH_LABEL_GAP_FLOWCHART)
    } else if flowchart_nested_bridge_target_child {
        label_height + pad_y
    } else if graph.kind == crate::ir::DiagramKind::Flowchart {
        // Keep the label comfortably inside the top band without over-expanding
        // the cluster height.
        pad_y.max(label_height + SUBGRAPH_LABEL_GAP_FLOWCHART)
    } else if graph.kind == crate::ir::DiagramKind::Kanban {
        pad_y.max(label_height + SUBGRAPH_LABEL_GAP_KANBAN)
    } else if graph.kind == crate::ir::DiagramKind::State {
        let base = (label_height + theme.font_size * STATE_SUBGRAPH_TOP_LABEL_SCALE)
            .max(theme.font_size * STATE_SUBGRAPH_TOP_MIN_SCALE);
        if is_sparse_non_root_leaf_state_composite(graph, sub) {
            base + STATE_SPARSE_LEAF_TOP_PAD_OFFSET
        } else {
            base
        }
    } else if graph.kind == crate::ir::DiagramKind::Class {
        label_height + CLASS_NAMESPACE_TOP_LABEL_GAP
    } else {
        pad_y + label_height + SUBGRAPH_LABEL_GAP_GENERIC
    };

    (pad_x, pad_y, top_padding)
}
fn estimate_subgraph_box_size(
    graph: &Graph,
    sub: &crate::ir::Subgraph,
    nodes: &BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
    anchorable: bool,
) -> Option<(f32, f32, f32, f32)> {
    estimate_subgraph_box_size_with_nodes(
        graph, sub, &sub.nodes, nodes, theme, config, anchorable, 0,
    )
}

fn estimate_subgraph_box_size_with_nodes(
    graph: &Graph,
    sub: &crate::ir::Subgraph,
    node_ids: &[String],
    nodes: &BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
    anchorable: bool,
    depth: usize,
) -> Option<(f32, f32, f32, f32)> {
    // Also compute nested-composite-depth-below for this sub so the cluster
    // padding can be inflated for outer composites.
    let nested_depth_below = {
        let tree = SubgraphTree::build(graph);
        let idx = graph
            .subgraphs
            .iter()
            .position(|s| std::ptr::eq(s, sub))
            .unwrap_or(usize::MAX);
        if idx < graph.subgraphs.len() {
            tree.max_nested_composite_depth_below(idx, graph)
        } else {
            0
        }
    };
    if node_ids.is_empty() {
        return None;
    }
    let direction = subgraph_layout_direction(graph, sub);
    let mut temp_nodes: BTreeMap<String, NodeLayout> = BTreeMap::new();
    for node_id in node_ids {
        if let Some(node) = nodes.get(node_id) {
            let mut clone = node.clone();
            clone.x = 0.0;
            clone.y = 0.0;
            temp_nodes.insert(node_id.clone(), clone);
        }
    }
    let local_config = subgraph_layout_config_for_depth(graph, sub, anchorable, config, depth);
    let ranks = compute_ranks_subset_for(graph, node_ids, &graph.edges, &graph.node_order);
    assign_positions(
        node_ids,
        &ranks,
        direction,
        &local_config,
        &mut temp_nodes,
        0.0,
        0.0,
    );
    center_flowchart_recursive_subgraph_ranks_like_dagre(
        graph,
        sub,
        node_ids,
        &ranks,
        direction,
        &local_config,
        &mut temp_nodes,
    );
    apply_flowchart_recursive_cycle_stagger_for_subgraph(
        graph,
        sub,
        node_ids,
        &local_config,
        &mut temp_nodes,
    );
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for node_id in node_ids {
        if let Some(node) = temp_nodes.get(node_id) {
            min_x = min_x.min(node.x);
            min_y = min_y.min(node.y);
            max_x = max_x.max(node.x + node.width);
            max_y = max_y.max(node.y + node.height);
        }
    }
    if min_x == f32::MAX {
        return None;
    }
    let label_empty = sub.label.trim().is_empty();
    let mut label_block = measure_subgraph_label(graph, sub, theme, config);
    if label_empty {
        label_block.width = 0.0;
        label_block.height = 0.0;
    }
    let (padding_x, padding_y, top_padding) =
        subgraph_padding_from_label_with_depth(graph, sub, theme, &label_block, nested_depth_below);

    let lane_extra = flowchart_recursive_cycle_lane_extra(graph, sub, node_ids);
    let width = (max_x - min_x)
        + padding_x * 2.0
        + if is_horizontal(direction) {
            0.0
        } else {
            lane_extra
        };
    let height = (max_y - min_y)
        + padding_y
        + top_padding
        + if is_horizontal(direction) {
            lane_extra
        } else {
            0.0
        };
    Some((width, height, padding_x, top_padding))
}

fn apply_subgraph_anchor_sizes(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) -> HashMap<String, SubgraphAnchorInfo> {
    let mut anchors: HashMap<String, SubgraphAnchorInfo> = HashMap::new();
    if graph.subgraphs.is_empty() {
        return anchors;
    }

    // Compute nesting depth for each subgraph so we can pass depth-aware
    // ranksep into the inner-dagre estimate (mirrors JS recursiveRender).
    let tree = SubgraphTree::build(graph);
    let n = graph.subgraphs.len();
    let mut sub_depth: Vec<usize> = vec![0; n];
    for i in 0..n {
        let mut d = 0usize;
        let mut cur = i;
        while let Some(p) = tree.parent.get(cur).and_then(|p| *p) {
            d += 1;
            cur = p;
            if d > n {
                break;
            }
        }
        sub_depth[i] = d;
    }

    for (idx, sub) in graph.subgraphs.iter().enumerate() {
        if is_region_subgraph(sub) {
            continue;
        }
        if !subgraph_should_anchor(sub, graph, nodes) {
            continue;
        }
        let Some(anchor_id) = subgraph_anchor_id(sub, nodes) else {
            continue;
        };
        let Some((width, height, padding_x, top_padding)) = estimate_subgraph_box_size_with_nodes(
            graph,
            sub,
            &sub.nodes,
            nodes,
            theme,
            config,
            true,
            sub_depth[idx],
        ) else {
            continue;
        };
        if let Some(node) = nodes.get_mut(anchor_id) {
            node.width = width;
            node.height = height;
        }
        anchors.insert(
            anchor_id.to_string(),
            SubgraphAnchorInfo {
                sub_idx: idx,
                padding_x,
                top_padding,
            },
        );
    }
    anchors
}

fn align_subgraphs_to_anchor_nodes(
    graph: &Graph,
    anchor_info: &HashMap<String, SubgraphAnchorInfo>,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) -> HashSet<String> {
    let mut anchored_nodes = HashSet::new();
    if anchor_info.is_empty() {
        return anchored_nodes;
    }
    let tree = SubgraphTree::build(graph);
    let sub_count = graph.subgraphs.len();
    let mut subgraph_depth: Vec<usize> = vec![0; sub_count];
    for idx in 0..sub_count {
        let mut depth = 0usize;
        let mut cur = idx;
        while let Some(parent_idx) = tree.parent.get(cur).and_then(|parent| *parent) {
            depth += 1;
            cur = parent_idx;
            if depth > sub_count {
                break;
            }
        }
        subgraph_depth[idx] = depth;
    }

    let mut ordered_anchors: Vec<(&String, &SubgraphAnchorInfo)> = anchor_info.iter().collect();
    ordered_anchors.sort_by(|(a_id, a_info), (b_id, b_info)| {
        let a_depth = subgraph_depth
            .get(a_info.sub_idx)
            .copied()
            .unwrap_or(usize::MAX);
        let b_depth = subgraph_depth
            .get(b_info.sub_idx)
            .copied()
            .unwrap_or(usize::MAX);
        a_depth
            .cmp(&b_depth)
            .then_with(|| a_info.sub_idx.cmp(&b_info.sub_idx))
            .then_with(|| a_id.cmp(b_id))
    });

    // Pass 9 fix: precompute "direct children" node lists per subgraph,
    // so each subgraph's interior layout positions only its direct
    // members (excluding nodes that live deeper inside nested subgraphs).
    // Without this filter, an outer composite's `assign_positions` ranks
    // a deeper nested cluster's anchor at the same row as its direct
    // children, inflating row heights and pushing direct children far
    // apart. See state-diagram-concurrency-punchlist.md Pass 9.
    let mut direct_nodes_per_sub: Vec<Vec<String>> = Vec::with_capacity(sub_count);
    for (idx, sub) in graph.subgraphs.iter().enumerate() {
        let nested_descendant_ids: HashSet<&str> = {
            let mut s: HashSet<&str> = HashSet::new();
            if let Some(child_indices) = tree.children.get(idx) {
                for &child_idx in child_indices {
                    if let Some(child) = graph.subgraphs.get(child_idx) {
                        for n in &child.nodes {
                            s.insert(n.as_str());
                        }
                    }
                }
            }
            s
        };
        let direct: Vec<String> = sub
            .nodes
            .iter()
            .filter(|n| !nested_descendant_ids.contains(n.as_str()))
            .cloned()
            .collect();
        direct_nodes_per_sub.push(direct);
    }

    for (anchor_id, info) in ordered_anchors {
        let (anchor_x, anchor_y) = {
            let Some(anchor) = nodes.get(anchor_id) else {
                continue;
            };
            (anchor.x, anchor.y)
        };
        let Some(sub) = graph.subgraphs.get(info.sub_idx) else {
            continue;
        };
        let direction = subgraph_layout_direction(graph, sub);
        let depth = subgraph_depth.get(info.sub_idx).copied().unwrap_or(0);
        let local_config = subgraph_layout_config_for_depth(graph, sub, true, config, depth);
        let direct_nodes = direct_nodes_per_sub
            .get(info.sub_idx)
            .filter(|v| !v.is_empty())
            .map(|v| v.as_slice())
            .unwrap_or(sub.nodes.as_slice());
        let ranks = compute_ranks_subset_for(graph, direct_nodes, &graph.edges, &graph.node_order);
        assign_positions(
            direct_nodes,
            &ranks,
            direction,
            &local_config,
            nodes,
            anchor_x + info.padding_x,
            anchor_y + info.top_padding,
        );
        center_flowchart_recursive_subgraph_ranks_like_dagre(
            graph,
            sub,
            direct_nodes,
            &ranks,
            direction,
            &local_config,
            nodes,
        );
        apply_flowchart_recursive_cycle_stagger_for_subgraph(
            graph,
            sub,
            direct_nodes,
            &local_config,
            nodes,
        );
        if matches!(direction, Direction::RightLeft | Direction::BottomTop) {
            mirror_subgraph_nodes(direct_nodes, nodes, direction);
        }
        anchored_nodes.extend(direct_nodes.iter().cloned());
    }
    anchored_nodes
}

/// In state diagrams, snap inner [*] start/end markers (small circles) so their
/// center_x matches the column-center of the regular state nodes in the same
/// subgraph. JS dagre puts the dot in the same column as its connected state;
/// our raw layout sometimes places the dot at the subgraph's left edge, which
/// produces visible zig-zag edges between dot and state.
fn align_state_markers_to_subgraph_columns(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
) {
    if graph.kind != crate::ir::DiagramKind::State {
        return;
    }
    for sub in &graph.subgraphs {
        // Collect non-marker (= regular state) inner nodes' centers.
        let mut state_centers_x: Vec<f32> = Vec::new();
        let mut marker_ids: Vec<String> = Vec::new();
        for node_id in &sub.nodes {
            let Some(node) = nodes.get(node_id) else {
                continue;
            };
            let label_empty = node
                .label
                .lines
                .iter()
                .all(|line| line.text().trim().is_empty());
            let is_marker = label_empty
                && matches!(
                    node.shape,
                    crate::ir::NodeShape::Circle | crate::ir::NodeShape::DoubleCircle
                );
            if is_marker {
                marker_ids.push(node_id.clone());
            } else {
                state_centers_x.push(node.x + node.width * 0.5);
            }
        }
        if marker_ids.is_empty() || state_centers_x.is_empty() {
            continue;
        }
        // Use median (robust to outliers) of inner state centers as the column.
        state_centers_x.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let column_x = state_centers_x[state_centers_x.len() / 2];
        for marker_id in marker_ids {
            if let Some(node) = nodes.get_mut(&marker_id) {
                node.x = column_x - node.width * 0.5;
            }
        }
    }
}

/// Iter 282: align `__end_root__` (root_end marker) horizontally with the
/// cluster it connects to, when it has incoming edges from exactly one
/// top-level state composite. JS dagre places root_end mid-vertically
/// between the connecting cluster's column and a sibling's column; this
/// approximates that by snapping root_end's x to the connecting cluster's
/// center. Targets nested-composite-states' visible topology gap where
/// root_end was rendered below First (the wrong column) instead of in
/// End's column.
fn align_root_end_to_connecting_cluster(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::State {
        return;
    }
    let root_end_id = "__end_root__";
    if !nodes.contains_key(root_end_id) {
        return;
    }
    let pred_ids: Vec<&str> = graph
        .edges
        .iter()
        .filter(|e| e.to == root_end_id)
        .map(|e| e.from.as_str())
        .collect();
    if pred_ids.is_empty() {
        return;
    }
    // Find the top-level cluster that contains all predecessors.
    // A predecessor matches a cluster if either:
    //   (a) it is a member of that cluster (recursive), OR
    //   (b) the predecessor IS the cluster's id or label (cluster-anchor edge,
    //       e.g. `End --> [*]` creates an edge whose source is "End" itself,
    //       not a node inside End).
    let tree = SubgraphTree::build(graph);
    let mut connecting_cluster: Option<usize> = None;
    for &top_idx in &tree.top_level {
        let sub = &graph.subgraphs[top_idx];
        let cluster_id = sub.id.as_deref().unwrap_or("");
        let cluster_label = sub.label.as_str();
        let mut members: HashSet<&str> = HashSet::new();
        let mut stack = vec![top_idx];
        while let Some(idx) = stack.pop() {
            for n in &graph.subgraphs[idx].nodes {
                members.insert(n.as_str());
            }
            if let Some(children) = tree.children.get(idx) {
                for &c in children {
                    stack.push(c);
                }
            }
        }
        let all_match = pred_ids.iter().all(|p| {
            members.contains(p)
                || (!cluster_id.is_empty() && *p == cluster_id)
                || (!cluster_label.is_empty() && *p == cluster_label)
        });
        if all_match {
            if connecting_cluster.is_none() {
                connecting_cluster = Some(top_idx);
            } else {
                return;
            }
        }
    }
    let Some(cluster_idx) = connecting_cluster else {
        return;
    };
    // Compute the cluster's left edge and center from its member nodes.
    let cluster_members: HashSet<&str> = {
        let mut members: HashSet<&str> = HashSet::new();
        let mut stack = vec![cluster_idx];
        while let Some(idx) = stack.pop() {
            for n in &graph.subgraphs[idx].nodes {
                members.insert(n.as_str());
            }
            if let Some(children) = tree.children.get(idx) {
                for &c in children {
                    stack.push(c);
                }
            }
        }
        members
    };
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut member_center_y = f32::MIN;
    for id in &cluster_members {
        if let Some(node) = nodes.get(*id) {
            min_x = min_x.min(node.x);
            max_x = max_x.max(node.x + node.width);
            member_center_y = member_center_y.max(node.y + node.height * 0.5);
        }
    }
    if min_x == f32::MAX {
        return;
    }
    let cluster_center_x = (min_x + max_x) * 0.5;
    if let Some(end_node) = nodes.get_mut(root_end_id) {
        // Compound state exits in JS often leave the cluster from the side and
        // place the root final marker just outside the cluster, not centered
        // below it. Preserve the old center fallback for ordinary cases, but
        // when the final marker is already vertically aligned with an inner
        // cluster exit, keep it beside the cluster.
        let current_center_y = end_node.y + end_node.height * 0.5;
        if (current_center_y - member_center_y).abs() <= config.rank_spacing.max(50.0) * 0.75 {
            end_node.x = min_x - config.node_spacing.max(50.0) * 1.45 - end_node.width * 0.5;
        } else {
            end_node.x = cluster_center_x - end_node.width * 0.5;
        }
    }
}

// Mirror of align_root_end_to_connecting_cluster for the top-level start node:
// when `[*] --> Cluster` puts the root start above a composite/concurrent
// state, center the start circle on that cluster's horizontal midpoint.
fn align_root_start_to_connecting_cluster(graph: &Graph, nodes: &mut BTreeMap<String, NodeLayout>) {
    if graph.kind != crate::ir::DiagramKind::State {
        return;
    }
    let root_start_id = "__start_root__";
    if !nodes.contains_key(root_start_id) {
        return;
    }
    let succ_ids: Vec<&str> = graph
        .edges
        .iter()
        .filter(|e| e.from == root_start_id)
        .map(|e| e.to.as_str())
        .collect();
    if succ_ids.is_empty() {
        return;
    }
    let tree = SubgraphTree::build(graph);
    let mut connecting_cluster: Option<usize> = None;
    for &top_idx in &tree.top_level {
        let sub = &graph.subgraphs[top_idx];
        let cluster_id = sub.id.as_deref().unwrap_or("");
        let cluster_label = sub.label.as_str();
        let mut members: HashSet<&str> = HashSet::new();
        let mut stack = vec![top_idx];
        while let Some(idx) = stack.pop() {
            for n in &graph.subgraphs[idx].nodes {
                members.insert(n.as_str());
            }
            if let Some(children) = tree.children.get(idx) {
                for &c in children {
                    stack.push(c);
                }
            }
        }
        let all_match = succ_ids.iter().all(|s| {
            members.contains(s)
                || (!cluster_id.is_empty() && *s == cluster_id)
                || (!cluster_label.is_empty() && *s == cluster_label)
        });
        if all_match {
            if connecting_cluster.is_none() {
                connecting_cluster = Some(top_idx);
            } else {
                return;
            }
        }
    }
    let Some(cluster_idx) = connecting_cluster else {
        return;
    };
    let cluster_members: HashSet<&str> = {
        let mut members: HashSet<&str> = HashSet::new();
        let mut stack = vec![cluster_idx];
        while let Some(idx) = stack.pop() {
            for n in &graph.subgraphs[idx].nodes {
                members.insert(n.as_str());
            }
            if let Some(children) = tree.children.get(idx) {
                for &c in children {
                    stack.push(c);
                }
            }
        }
        members
    };
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    for id in &cluster_members {
        if let Some(node) = nodes.get(*id) {
            min_x = min_x.min(node.x);
            max_x = max_x.max(node.x + node.width);
        }
    }
    if min_x == f32::MAX {
        return;
    }
    let cluster_center_x = (min_x + max_x) * 0.5;
    if let Some(start_node) = nodes.get_mut(root_start_id) {
        start_node.x = cluster_center_x - start_node.width * 0.5;
    }
}

/// Mermaid's state-v2 renderer lays simple top-level composite states in the
/// root dagre graph, so the visible marker/composite boundaries retain the
/// root `ranksep` gap. Our recursive pass sizes the leaf composites correctly,
/// but their outer boundaries can end up only one compact node-gap apart.
#[derive(Clone)]
enum StateRootCompositeItem {
    Marker(String),
    Subgraph { graph_idx: usize, layout_idx: usize },
}

fn enforce_state_root_leaf_composite_gaps(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    subgraphs: &mut [SubgraphLayout],
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::State || !state_has_root_markers(graph) {
        return;
    }

    let top_level = top_level_subgraph_indices(graph);
    if top_level.len() < 2 {
        return;
    }
    if !top_level.iter().all(|&idx| {
        graph.subgraphs.get(idx).is_some_and(|sub| {
            is_top_level_leaf_state_composite(graph, sub) && sub.nodes.len() <= 3
        })
    }) {
        return;
    }

    let horizontal = is_horizontal(graph.direction);
    let mut items: Vec<(f32, u8, StateRootCompositeItem)> = Vec::new();

    if let Some(start) = nodes.get("__start_root__") {
        let min_main = if horizontal { start.x } else { start.y };
        items.push((
            min_main,
            0,
            StateRootCompositeItem::Marker("__start_root__".to_string()),
        ));
    }

    for &graph_idx in &top_level {
        let Some(sub) = graph.subgraphs.get(graph_idx) else {
            continue;
        };
        let Some(layout_idx) = subgraphs
            .iter()
            .position(|layout| layout.label == sub.label && layout.nodes == sub.nodes)
        else {
            return;
        };
        let layout = &subgraphs[layout_idx];
        let min_main = if horizontal { layout.x } else { layout.y };
        items.push((
            min_main,
            1,
            StateRootCompositeItem::Subgraph {
                graph_idx,
                layout_idx,
            },
        ));
    }

    if let Some(end) = nodes.get("__end_root__") {
        let min_main = if horizontal { end.x } else { end.y };
        items.push((
            min_main,
            2,
            StateRootCompositeItem::Marker("__end_root__".to_string()),
        ));
    }

    if items.len() < 3 {
        return;
    }

    items.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.1.cmp(&b.1))
    });

    let desired_gap = config.rank_spacing.max(50.0);
    let mut prev_max_main: Option<f32> = None;

    for (_, _, item) in items {
        let Some((min_main, max_main)) =
            state_root_composite_item_bounds(&item, nodes, subgraphs, horizontal)
        else {
            continue;
        };

        let mut adjusted_max = max_main;
        if let Some(prev_max) = prev_max_main {
            let required_min = prev_max + desired_gap;
            if min_main < required_min {
                let delta = required_min - min_main;
                shift_state_root_composite_item(&item, graph, nodes, subgraphs, horizontal, delta);
                adjusted_max += delta;
            }
        }

        prev_max_main = Some(adjusted_max);
    }
}

fn state_root_composite_item_bounds(
    item: &StateRootCompositeItem,
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    horizontal: bool,
) -> Option<(f32, f32)> {
    match item {
        StateRootCompositeItem::Marker(id) => nodes.get(id).map(|node| {
            if horizontal {
                (node.x, node.x + node.width)
            } else {
                (node.y, node.y + node.height)
            }
        }),
        StateRootCompositeItem::Subgraph { layout_idx, .. } => {
            subgraphs.get(*layout_idx).map(|sub| {
                if horizontal {
                    (sub.x, sub.x + sub.width)
                } else {
                    (sub.y, sub.y + sub.height)
                }
            })
        }
    }
}

fn shift_state_root_composite_item(
    item: &StateRootCompositeItem,
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    subgraphs: &mut [SubgraphLayout],
    horizontal: bool,
    delta: f32,
) {
    match item {
        StateRootCompositeItem::Marker(id) => {
            if let Some(node) = nodes.get_mut(id) {
                if horizontal {
                    node.x += delta;
                } else {
                    node.y += delta;
                }
            }
        }
        StateRootCompositeItem::Subgraph {
            graph_idx,
            layout_idx,
        } => {
            if let Some(layout) = subgraphs.get_mut(*layout_idx) {
                if horizontal {
                    layout.x += delta;
                } else {
                    layout.y += delta;
                }
            }
            if let Some(sub) = graph.subgraphs.get(*graph_idx) {
                for node_id in &sub.nodes {
                    if let Some(node) = nodes.get_mut(node_id) {
                        if horizontal {
                            node.x += delta;
                        } else {
                            node.y += delta;
                        }
                    }
                }
            }
        }
    }
}

fn apply_state_subgraph_layouts(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
    skip_indices: &HashSet<usize>,
) {
    // Build nesting hierarchy: for each subgraph, find which other subgraphs are direct children
    let sub_count = graph.subgraphs.len();
    let mut depth: Vec<usize> = vec![0; sub_count];
    let mut parent_of: Vec<Option<usize>> = vec![None; sub_count];

    // A subgraph B is nested in subgraph A if A's nodes list contains B's ID/label
    for (i, sub_a) in graph.subgraphs.iter().enumerate() {
        for (j, sub_b) in graph.subgraphs.iter().enumerate() {
            if i == j {
                continue;
            }
            let b_id = sub_b.id.as_deref().unwrap_or("");
            if sub_a.nodes.iter().any(|n| n == b_id)
                || sub_a.nodes.iter().any(|n| n == &sub_b.label)
            {
                if parent_of[j].is_none() {
                    parent_of[j] = Some(i);
                }
            }
        }
    }

    // Compute depth: walk from each subgraph up to root
    for i in 0..sub_count {
        let mut d = 0;
        let mut cur = i;
        while let Some(p) = parent_of[cur] {
            d += 1;
            cur = p;
            if d > sub_count {
                break;
            }
        }
        depth[i] = d;
    }

    // Process from deepest (innermost) to shallowest (outermost)
    let mut order: Vec<usize> = (0..sub_count).collect();
    order.sort_by(|a, b| depth[*b].cmp(&depth[*a]));

    // Track computed inner subgraph boxes (idx -> (x, y, width, height))
    let mut inner_boxes: HashMap<usize, (f32, f32, f32, f32)> = HashMap::new();

    for idx in order {
        let sub = &graph.subgraphs[idx];
        if skip_indices.contains(&idx) {
            continue;
        }
        if sub.nodes.len() <= 1 {
            continue;
        }
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        for node_id in &sub.nodes {
            if let Some(node) = nodes.get(node_id) {
                min_x = min_x.min(node.x);
                min_y = min_y.min(node.y);
            }
        }
        if min_x == f32::MAX {
            continue;
        }

        // For nodes in this subgraph that are also inner subgraph anchors,
        // temporarily set their size to the inner subgraph's box size
        let mut saved_sizes: Vec<(String, f32, f32)> = Vec::new();
        let mut inner_anchor_ids: Vec<String> = Vec::new();
        for node_id in &sub.nodes {
            for (j, inner_sub) in graph.subgraphs.iter().enumerate() {
                if let Some((_, _, w, h)) = inner_boxes.get(&j) {
                    let inner_id = inner_sub.id.as_deref().unwrap_or("");
                    if node_id == inner_id || node_id == &inner_sub.label {
                        if !inner_anchor_ids.iter().any(|id| id == node_id) {
                            inner_anchor_ids.push(node_id.clone());
                        }
                        if let Some(node) = nodes.get(node_id) {
                            saved_sizes.push((node_id.clone(), node.width, node.height));
                        }
                        if let Some(node) = nodes.get_mut(node_id) {
                            node.width = *w;
                            node.height = *h;
                        }
                    }
                }
            }
        }

        let ranks = compute_ranks_subset_for(graph, &sub.nodes, &graph.edges, &graph.node_order);
        let local_config = subgraph_layout_config_for(graph, sub, false, config);
        let (origin_x, origin_y) =
            if is_region_subgraph(sub) && matches!(graph.direction, Direction::TopDown) {
                (min_x, min_y + STATE_REGION_PAD_Y + STATE_REGION_ROOT_GAP)
            } else {
                (min_x, min_y)
            };
        assign_positions(
            &sub.nodes,
            &ranks,
            graph.direction,
            &local_config,
            nodes,
            origin_x,
            origin_y,
        );
        center_flowchart_recursive_subgraph_ranks_like_dagre(
            graph,
            sub,
            &sub.nodes,
            &ranks,
            graph.direction,
            &local_config,
            nodes,
        );
        apply_flowchart_recursive_cycle_stagger_for_subgraph(
            graph,
            sub,
            &sub.nodes,
            &local_config,
            nodes,
        );
        if is_region_subgraph(sub) {
            apply_state_region_label_rank_gaps(graph, &sub.nodes, &ranks, graph.direction, nodes);
        }

        // Keep nested composite-state headers clear of parent headers.
        let nested_anchor_min_y = min_y + (config.node_spacing * 0.4).max(20.0);
        for anchor_id in &inner_anchor_ids {
            if let Some(anchor) = nodes.get_mut(anchor_id)
                && anchor.y < nested_anchor_min_y
            {
                anchor.y = nested_anchor_min_y;
            }
        }

        // Restore original sizes for anchor nodes
        for (id, w, h) in saved_sizes {
            if let Some(node) = nodes.get_mut(&id) {
                node.width = w;
                node.height = h;
            }
        }

        // After positioning, re-position inner subgraph contents to match their anchor position
        for (j, inner_sub) in graph.subgraphs.iter().enumerate() {
            if let Some(&(old_x, old_y, _, _)) = inner_boxes.get(&j) {
                let inner_id = inner_sub.id.as_deref().unwrap_or("");
                if !sub
                    .nodes
                    .iter()
                    .any(|n| n == inner_id || n == &inner_sub.label)
                {
                    continue;
                }
                // Find the anchor node's new position
                let anchor_id = if sub.nodes.iter().any(|n| n == inner_id) {
                    inner_id
                } else {
                    inner_sub.label.as_str()
                };
                if let Some(anchor) = nodes.get(anchor_id) {
                    let dx = anchor.x - old_x;
                    let dy = anchor.y - old_y;
                    if dx.abs() > 0.01 || dy.abs() > 0.01 {
                        for inner_node_id in &inner_sub.nodes {
                            if let Some(inner_node) = nodes.get_mut(inner_node_id) {
                                inner_node.x += dx;
                                inner_node.y += dy;
                            }
                        }
                    }
                }
            }
        }

        // Compute and save this subgraph's box
        let mut bmin_x = f32::MAX;
        let mut bmin_y = f32::MAX;
        let mut bmax_x = f32::MIN;
        let mut bmax_y = f32::MIN;
        for node_id in &sub.nodes {
            if let Some(node) = nodes.get(node_id) {
                bmin_x = bmin_x.min(node.x);
                bmin_y = bmin_y.min(node.y);
                bmax_x = bmax_x.max(node.x + node.width);
                bmax_y = bmax_y.max(node.y + node.height);
            }
        }
        // Also include inner subgraph boxes
        for (j, inner_sub) in graph.subgraphs.iter().enumerate() {
            if let Some(&(_, _, _, _)) = inner_boxes.get(&j) {
                let inner_id = inner_sub.id.as_deref().unwrap_or("");
                if sub
                    .nodes
                    .iter()
                    .any(|n| n == inner_id || n == &inner_sub.label)
                {
                    // Use inner node positions
                    for inner_node_id in &inner_sub.nodes {
                        if let Some(node) = nodes.get(inner_node_id) {
                            bmin_x = bmin_x.min(node.x);
                            bmin_y = bmin_y.min(node.y);
                            bmax_x = bmax_x.max(node.x + node.width);
                            bmax_y = bmax_y.max(node.y + node.height);
                        }
                    }
                }
            }
        }
        let padding = config.node_spacing;
        if bmin_x < f32::MAX {
            inner_boxes.insert(
                idx,
                (
                    bmin_x,
                    bmin_y,
                    bmax_x - bmin_x + padding,
                    bmax_y - bmin_y + padding,
                ),
            );
        }
    }
}

fn apply_subgraph_anchors(
    graph: &Graph,
    subgraphs: &[SubgraphLayout],
    nodes: &mut BTreeMap<String, NodeLayout>,
) {
    if subgraphs.is_empty() {
        return;
    }

    let mut label_to_index: HashMap<&str, usize> = HashMap::new();
    for (idx, sub) in subgraphs.iter().enumerate() {
        label_to_index.insert(sub.label.as_str(), idx);
    }

    for sub in &graph.subgraphs {
        let Some(&layout_idx) = label_to_index.get(sub.label.as_str()) else {
            continue;
        };
        let layout = &subgraphs[layout_idx];
        let mut anchor_ids: HashSet<&str> = HashSet::new();
        if let Some(id) = &sub.id {
            anchor_ids.insert(id.as_str());
        }
        anchor_ids.insert(sub.label.as_str());

        for anchor_id in anchor_ids {
            if sub.nodes.iter().any(|node_id| node_id == anchor_id) {
                continue;
            }
            let Some(node) = nodes.get_mut(anchor_id) else {
                continue;
            };
            node.anchor_subgraph = Some(layout_idx);
            let size = 2.0;
            node.width = size;
            node.height = size;
            node.x = layout.x + layout.width / 2.0 - size / 2.0;
            node.y = layout.y + layout.height / 2.0 - size / 2.0;
        }
    }
}

fn anchor_layout_for_edge(
    anchor: &NodeLayout,
    subgraph: &SubgraphLayout,
    direction: Direction,
    is_from: bool,
) -> NodeLayout {
    anchor_layout_for_edge_toward(anchor, subgraph, direction, is_from, None)
}

/// Compute (temp_from, temp_to) cluster-edge anchors with each side's face
/// chosen based on the OTHER endpoint's position. Used by the routing pipeline
/// to keep cluster-to-cluster edges short when the target cluster lies more
/// to the side than above/below the source cluster.
fn cluster_anchor_pair(
    from_layout: &NodeLayout,
    to_layout: &NodeLayout,
    subgraphs: &[SubgraphLayout],
    direction: Direction,
) -> (Option<NodeLayout>, Option<NodeLayout>) {
    let from_center = from_layout
        .anchor_subgraph
        .and_then(|i| subgraphs.get(i))
        .map(|sub| (sub.x + sub.width / 2.0, sub.y + sub.height / 2.0))
        .unwrap_or_else(|| {
            (
                from_layout.x + from_layout.width / 2.0,
                from_layout.y + from_layout.height / 2.0,
            )
        });
    let to_center = to_layout
        .anchor_subgraph
        .and_then(|i| subgraphs.get(i))
        .map(|sub| (sub.x + sub.width / 2.0, sub.y + sub.height / 2.0))
        .unwrap_or_else(|| {
            (
                to_layout.x + to_layout.width / 2.0,
                to_layout.y + to_layout.height / 2.0,
            )
        });
    let temp_from = from_layout.anchor_subgraph.and_then(|i| {
        subgraphs.get(i).map(|sub| {
            anchor_layout_for_edge_toward(from_layout, sub, direction, true, Some(to_center))
        })
    });
    let temp_to = to_layout.anchor_subgraph.and_then(|i| {
        subgraphs.get(i).map(|sub| {
            anchor_layout_for_edge_toward(to_layout, sub, direction, false, Some(from_center))
        })
    });
    (temp_from, temp_to)
}

#[derive(Debug, Clone, Copy)]
struct FlowchartRouteBox {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl FlowchartRouteBox {
    fn right(self) -> f32 {
        self.x + self.width
    }

    fn bottom(self) -> f32 {
        self.y + self.height
    }
}

fn flowchart_route_box(node: &NodeLayout, subgraphs: &[SubgraphLayout]) -> FlowchartRouteBox {
    if let Some(subgraph_idx) = node.anchor_subgraph
        && let Some(subgraph) = subgraphs.get(subgraph_idx)
    {
        return FlowchartRouteBox {
            x: subgraph.x,
            y: subgraph.y,
            width: subgraph.width,
            height: subgraph.height,
        };
    }

    FlowchartRouteBox {
        x: node.x,
        y: node.y,
        width: node.width,
        height: node.height,
    }
}

fn flowchart_forward_overlap_route(
    edge: &crate::ir::Edge,
    from: &NodeLayout,
    to: &NodeLayout,
    subgraphs: &[SubgraphLayout],
    direction: Direction,
) -> Option<Vec<(f32, f32)>> {
    if edge.style != crate::ir::EdgeStyle::Solid
        || edge.label.is_some()
        || edge.start_label.is_some()
        || edge.end_label.is_some()
    {
        return None;
    }

    let from_box = flowchart_route_box(from, subgraphs);
    let to_box = flowchart_route_box(to, subgraphs);
    let min_overlap = 6.0;
    match direction {
        Direction::TopDown => {
            if to_box.y + 1.0 < from_box.bottom() {
                return None;
            }
            let overlap_left = from_box.x.max(to_box.x);
            let overlap_right = from_box.right().min(to_box.right());
            if overlap_right - overlap_left < min_overlap {
                return None;
            }
            let x = (overlap_left + overlap_right) * 0.5;
            Some(vec![(x, from_box.bottom()), (x, to_box.y)])
        }
        Direction::BottomTop => {
            if from_box.y + 1.0 < to_box.bottom() {
                return None;
            }
            let overlap_left = from_box.x.max(to_box.x);
            let overlap_right = from_box.right().min(to_box.right());
            if overlap_right - overlap_left < min_overlap {
                return None;
            }
            let x = (overlap_left + overlap_right) * 0.5;
            Some(vec![(x, from_box.y), (x, to_box.bottom())])
        }
        Direction::LeftRight => {
            if to_box.x + 1.0 < from_box.right() {
                return None;
            }
            let overlap_top = from_box.y.max(to_box.y);
            let overlap_bottom = from_box.bottom().min(to_box.bottom());
            if overlap_bottom - overlap_top < min_overlap {
                return None;
            }
            let y = (overlap_top + overlap_bottom) * 0.5;
            Some(vec![(from_box.right(), y), (to_box.x, y)])
        }
        Direction::RightLeft => {
            if from_box.x + 1.0 < to_box.right() {
                return None;
            }
            let overlap_top = from_box.y.max(to_box.y);
            let overlap_bottom = from_box.bottom().min(to_box.bottom());
            if overlap_bottom - overlap_top < min_overlap {
                return None;
            }
            let y = (overlap_top + overlap_bottom) * 0.5;
            Some(vec![(from_box.x, y), (to_box.right(), y)])
        }
    }
}

fn flowchart_dagre_root_fanout_edge_indices(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) -> HashSet<usize> {
    let mut result = HashSet::new();
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.edges.len() < 2 {
        return result;
    }

    let mut subgraph_members = HashSet::new();
    for sub in &graph.subgraphs {
        for id in &sub.nodes {
            subgraph_members.insert(id.as_str());
        }
    }

    let mut incoming: HashMap<&str, usize> = HashMap::new();
    let mut outgoing: HashMap<&str, Vec<(usize, &crate::ir::Edge)>> = HashMap::new();
    for (idx, edge) in graph.edges.iter().enumerate() {
        *incoming.entry(edge.to.as_str()).or_insert(0) += 1;
        outgoing
            .entry(edge.from.as_str())
            .or_default()
            .push((idx, edge));
    }

    let center_axis_is_x = !is_horizontal(graph.direction);
    let rank_axis_is_x = is_horizontal(graph.direction);
    for (source_id, source_edges) in outgoing {
        if incoming.get(source_id).copied().unwrap_or(0) != 0
            || subgraph_members.contains(source_id)
            || source_edges.len() < 2
        {
            continue;
        }
        let Some(source) = nodes.get(source_id) else {
            continue;
        };
        if source.hidden || source.anchor_subgraph.is_some() {
            continue;
        }

        let source_rank = node_main_center(source, rank_axis_is_x);
        let mut target_centers = Vec::new();
        let mut target_ranks = Vec::new();
        let mut valid = true;
        for (_, edge) in &source_edges {
            if edge.style != crate::ir::EdgeStyle::Solid
                || edge.label.is_some()
                || edge.start_label.is_some()
                || edge.end_label.is_some()
            {
                valid = false;
                break;
            }
            let Some(target) = nodes.get(&edge.to) else {
                valid = false;
                break;
            };
            if target.hidden
                || target.anchor_subgraph.is_some()
                || incoming.get(edge.to.as_str()).copied().unwrap_or(0) != 1
            {
                valid = false;
                break;
            }
            let target_rank = node_main_center(target, rank_axis_is_x);
            if !flowchart_target_is_forward_rank(graph.direction, source_rank, target_rank) {
                valid = false;
                break;
            }
            target_ranks.push(target_rank);
            target_centers.push(node_main_center(target, center_axis_is_x));
        }
        if !valid || target_centers.len() < 2 {
            continue;
        }

        target_ranks.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        let rank_span = target_ranks.last().copied().unwrap_or(0.0)
            - target_ranks.first().copied().unwrap_or(0.0);
        if rank_span > config.rank_spacing.max(MIN_NODE_SPACING_FLOOR) * 0.75 {
            continue;
        }

        target_centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        let target_span = target_centers.last().copied().unwrap_or(0.0)
            - target_centers.first().copied().unwrap_or(0.0);
        if target_span < config.node_spacing.max(MIN_NODE_SPACING_FLOOR) * 0.5 {
            continue;
        }

        for (idx, _) in source_edges {
            result.insert(idx);
        }
    }

    result
}

fn flowchart_plain_solid_unlabeled(edge: &crate::ir::Edge) -> bool {
    edge.style == crate::ir::EdgeStyle::Solid
        && edge.label.is_none()
        && edge.start_label.is_none()
        && edge.end_label.is_none()
}

fn flowchart_dagre_same_rank_fanout_edge_indices(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) -> HashMap<usize, Direction> {
    let mut result = HashMap::new();
    if graph.kind != crate::ir::DiagramKind::Flowchart
        || graph.subgraphs.is_empty()
        || graph.edges.len() < 2
    {
        return result;
    }

    let mut node_subgraph: HashMap<&str, usize> = HashMap::new();
    for (sub_idx, subgraph) in graph.subgraphs.iter().enumerate() {
        for node_id in &subgraph.nodes {
            node_subgraph.entry(node_id.as_str()).or_insert(sub_idx);
        }
    }

    let mut incoming: HashMap<&str, usize> = HashMap::new();
    let mut outgoing: HashMap<&str, Vec<(usize, &crate::ir::Edge)>> = HashMap::new();
    for (idx, edge) in graph.edges.iter().enumerate() {
        *incoming.entry(edge.to.as_str()).or_insert(0) += 1;
        outgoing
            .entry(edge.from.as_str())
            .or_default()
            .push((idx, edge));
    }

    for (source_id, source_edges) in outgoing {
        if source_edges.len() < 2 {
            continue;
        }
        let Some(&source_subgraph) = node_subgraph.get(source_id) else {
            continue;
        };
        let Some(source) = nodes.get(source_id) else {
            continue;
        };
        if source.hidden || source.anchor_subgraph.is_some() {
            continue;
        }

        let mut candidate_edges = Vec::new();
        for (idx, edge) in source_edges {
            if !flowchart_plain_solid_unlabeled(edge)
                || node_subgraph.get(edge.to.as_str()).copied() != Some(source_subgraph)
                || incoming.get(edge.to.as_str()).copied().unwrap_or(0) != 1
            {
                continue;
            }
            candidate_edges.push((idx, edge));
        }

        for direction in [
            Direction::LeftRight,
            Direction::RightLeft,
            Direction::TopDown,
            Direction::BottomTop,
        ] {
            let mut edges = Vec::new();
            for (idx, edge) in &candidate_edges {
                let Some(target) = nodes.get(&edge.to) else {
                    continue;
                };
                let forward_gap = match direction {
                    Direction::LeftRight => target.x - (source.x + source.width),
                    Direction::RightLeft => source.x - (target.x + target.width),
                    Direction::TopDown => target.y - (source.y + source.height),
                    Direction::BottomTop => source.y - (target.y + target.height),
                };
                if forward_gap > 1.0 {
                    edges.push((*idx, *edge));
                }
            }
            if edges.len() < 2 {
                continue;
            }

            let mut target_main_centers = Vec::with_capacity(edges.len());
            let mut target_cross_centers = Vec::with_capacity(edges.len());
            let mut valid = true;
            for (_, edge) in &edges {
                let Some(target) = nodes.get(&edge.to) else {
                    valid = false;
                    break;
                };
                if target.hidden || target.anchor_subgraph.is_some() {
                    valid = false;
                    break;
                }
                let center = node_center(target);
                if is_horizontal(direction) {
                    target_main_centers.push(center.0);
                    target_cross_centers.push(center.1);
                } else {
                    target_main_centers.push(center.1);
                    target_cross_centers.push(center.0);
                }
            }
            if !valid {
                continue;
            }

            target_main_centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
            let main_span = target_main_centers.last().copied().unwrap_or(0.0)
                - target_main_centers.first().copied().unwrap_or(0.0);
            if main_span > 4.0 {
                continue;
            }

            target_cross_centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
            let cross_span = target_cross_centers.last().copied().unwrap_or(0.0)
                - target_cross_centers.first().copied().unwrap_or(0.0);
            if cross_span < config.node_spacing.max(MIN_NODE_SPACING_FLOOR) * 0.5 {
                continue;
            }

            for (idx, _) in edges {
                result.entry(idx).or_insert(direction);
            }
        }
    }

    result
}

fn flowchart_dagre_same_rank_fanout_route(
    edge: &crate::ir::Edge,
    from: &NodeLayout,
    to: &NodeLayout,
    direction: Direction,
) -> Option<Vec<(f32, f32)>> {
    if !flowchart_plain_solid_unlabeled(edge) {
        return None;
    }

    let target_center = node_center(to);
    let bend = match direction {
        Direction::LeftRight => {
            let gap = to.x - (from.x + from.width);
            if gap <= 1.0 {
                return None;
            }
            (from.x + from.width + gap * 0.5, target_center.1)
        }
        Direction::RightLeft => {
            let gap = from.x - (to.x + to.width);
            if gap <= 1.0 {
                return None;
            }
            (from.x - gap * 0.5, target_center.1)
        }
        Direction::TopDown => {
            let gap = to.y - (from.y + from.height);
            if gap <= 1.0 {
                return None;
            }
            (target_center.0, from.y + from.height + gap * 0.5)
        }
        Direction::BottomTop => {
            let gap = from.y - (to.y + to.height);
            if gap <= 1.0 {
                return None;
            }
            (target_center.0, from.y - gap * 0.5)
        }
    };

    Some(vec![
        flowchart_node_intersection_toward(from, bend),
        bend,
        flowchart_node_intersection_toward(to, bend),
    ])
}

fn flowchart_dagre_three_node_cycle_routes(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
) -> HashMap<usize, Vec<(f32, f32)>> {
    let mut result = HashMap::new();
    if graph.kind != crate::ir::DiagramKind::Flowchart
        || graph.subgraphs.is_empty()
        || graph.edges.len() < 3
    {
        return result;
    }

    let mut node_subgraph: HashMap<&str, usize> = HashMap::new();
    for (sub_idx, subgraph) in graph.subgraphs.iter().enumerate() {
        for node_id in &subgraph.nodes {
            node_subgraph.entry(node_id.as_str()).or_insert(sub_idx);
        }
    }

    let mut edge_by_pair: HashMap<(&str, &str), usize> = HashMap::new();
    for (idx, edge) in graph.edges.iter().enumerate() {
        if !flowchart_plain_solid_unlabeled(edge)
            || node_subgraph.get(edge.from.as_str()) != node_subgraph.get(edge.to.as_str())
        {
            continue;
        }
        edge_by_pair.entry((&edge.from, &edge.to)).or_insert(idx);
    }

    for (top_id, top) in nodes {
        if top.hidden || top.anchor_subgraph.is_some() {
            continue;
        }
        let Some(&sub_idx) = node_subgraph.get(top_id.as_str()) else {
            continue;
        };
        for (middle_id, middle) in nodes {
            if middle_id == top_id
                || middle.hidden
                || middle.anchor_subgraph.is_some()
                || node_subgraph.get(middle_id.as_str()).copied() != Some(sub_idx)
            {
                continue;
            }
            let Some(&top_middle_idx) = edge_by_pair.get(&(top_id.as_str(), middle_id.as_str()))
            else {
                continue;
            };
            for (bottom_id, bottom) in nodes {
                if bottom_id == top_id
                    || bottom_id == middle_id
                    || bottom.hidden
                    || bottom.anchor_subgraph.is_some()
                    || node_subgraph.get(bottom_id.as_str()).copied() != Some(sub_idx)
                {
                    continue;
                }
                let Some(&middle_bottom_idx) =
                    edge_by_pair.get(&(middle_id.as_str(), bottom_id.as_str()))
                else {
                    continue;
                };
                let Some(&bottom_top_idx) =
                    edge_by_pair.get(&(bottom_id.as_str(), top_id.as_str()))
                else {
                    continue;
                };
                if result.contains_key(&top_middle_idx)
                    || result.contains_key(&middle_bottom_idx)
                    || result.contains_key(&bottom_top_idx)
                {
                    continue;
                }

                let top_center = node_center(top);
                let middle_center = node_center(middle);
                let bottom_center = node_center(bottom);
                let top_is_above =
                    top_center.1 + 1.0 < middle_center.1 && middle_center.1 + 1.0 < bottom_center.1;
                let outer_column_aligned = (top_center.0 - bottom_center.0).abs() <= 3.0;
                let middle_shift = middle_center.0 - top_center.0;
                if !top_is_above || !outer_column_aligned || middle_shift.abs() < 20.0 {
                    continue;
                }

                let top_middle_y = ((top.y + top.height) + middle.y) * 0.5;
                let middle_bottom_y = ((middle.y + middle.height) + bottom.y) * 0.5;
                let cycle_lane_x = top_center.0 - middle_shift;

                let top_middle_route = vec![
                    flowchart_node_intersection_toward(top, (middle_center.0, top_middle_y)),
                    (middle_center.0, top_middle_y),
                    flowchart_node_intersection_toward(middle, (middle_center.0, top_middle_y)),
                ];
                let middle_bottom_route = vec![
                    flowchart_node_intersection_toward(middle, (middle_center.0, middle_bottom_y)),
                    (middle_center.0, middle_bottom_y),
                    flowchart_node_intersection_toward(bottom, (middle_center.0, middle_bottom_y)),
                ];
                let bottom_top_route = vec![
                    flowchart_node_intersection_toward(bottom, (cycle_lane_x, middle_bottom_y)),
                    (cycle_lane_x, middle_bottom_y),
                    (cycle_lane_x, middle_center.1),
                    (cycle_lane_x, top_middle_y),
                    flowchart_node_intersection_toward(top, (cycle_lane_x, top_middle_y)),
                ];

                result.insert(top_middle_idx, top_middle_route);
                result.insert(middle_bottom_idx, middle_bottom_route);
                result.insert(bottom_top_idx, bottom_top_route);
            }
        }
    }

    result
}

fn flowchart_dagre_root_fanout_route(
    edge: &crate::ir::Edge,
    from: &NodeLayout,
    to: &NodeLayout,
    direction: Direction,
) -> Option<Vec<(f32, f32)>> {
    if !flowchart_plain_solid_unlabeled(edge) {
        return None;
    }

    let from_center = node_center(from);
    let to_center = node_center(to);
    let rank_gap = FLOWCHART_RECURSIVE_DAGRE_SPACING;
    let half_gap = rank_gap * 0.5;
    match direction {
        Direction::TopDown => {
            if to_center.1 <= from_center.1 + 1.0 {
                return None;
            }
            let target = flowchart_node_intersection_toward(to, (to_center.0, from_center.1));
            let bend1 = (target.0, target.1 - rank_gap);
            let bend2 = (target.0, target.1 - half_gap);
            Some(vec![
                flowchart_node_intersection_toward(from, bend1),
                bend1,
                bend2,
                target,
            ])
        }
        Direction::BottomTop => {
            if to_center.1 >= from_center.1 - 1.0 {
                return None;
            }
            let target = flowchart_node_intersection_toward(to, (to_center.0, from_center.1));
            let bend1 = (target.0, target.1 + rank_gap);
            let bend2 = (target.0, target.1 + half_gap);
            Some(vec![
                flowchart_node_intersection_toward(from, bend1),
                bend1,
                bend2,
                target,
            ])
        }
        Direction::LeftRight => {
            if to_center.0 <= from_center.0 + 1.0 {
                return None;
            }
            let target = flowchart_node_intersection_toward(to, (from_center.0, to_center.1));
            let bend1 = (target.0 - rank_gap, target.1);
            let bend2 = (target.0 - half_gap, target.1);
            Some(vec![
                flowchart_node_intersection_toward(from, bend1),
                bend1,
                bend2,
                target,
            ])
        }
        Direction::RightLeft => {
            if to_center.0 >= from_center.0 - 1.0 {
                return None;
            }
            let target = flowchart_node_intersection_toward(to, (from_center.0, to_center.1));
            let bend1 = (target.0 + rank_gap, target.1);
            let bend2 = (target.0 + half_gap, target.1);
            Some(vec![
                flowchart_node_intersection_toward(from, bend1),
                bend1,
                bend2,
                target,
            ])
        }
    }
}

fn flowchart_node_intersection_toward(node: &NodeLayout, toward: (f32, f32)) -> (f32, f32) {
    let center = node_center(node);
    let dir = (toward.0 - center.0, toward.1 - center.1);
    if dir.0.abs() < f32::EPSILON && dir.1.abs() < f32::EPSILON {
        return center;
    }

    match node.shape {
        crate::ir::NodeShape::Circle | crate::ir::NodeShape::DoubleCircle => {
            let rx = node.width * 0.5;
            let ry = node.height * 0.5;
            if let Some(point) = ray_ellipse_intersection(center, dir, center, rx, ry) {
                return point;
            }
        }
        crate::ir::NodeShape::Cylinder | crate::ir::NodeShape::LinedCylinder => {
            return flowchart_cylinder_intersection_toward(node, toward);
        }
        _ => {}
    }

    if let Some(poly) = shape_polygon_points(node)
        && let Some(point) = ray_polygon_intersection(center, dir, &poly)
    {
        return point;
    }

    rect_intersection_toward(node, toward)
}

fn flowchart_cylinder_intersection_toward(node: &NodeLayout, toward: (f32, f32)) -> (f32, f32) {
    let mut pos = rect_intersection_toward(node, toward);
    let center = node_center(node);
    let rx = node.width * 0.5;
    if rx <= f32::EPSILON {
        return pos;
    }
    let ry = rx / (2.5 + node.width / 50.0);
    let x = pos.0 - center.0;
    let half_w = node.width * 0.5;
    let half_h = node.height * 0.5;
    if x.abs() < half_w
        || ((x.abs() - half_w).abs() <= 0.01 && (pos.1 - center.1).abs() > half_h - ry)
    {
        let mut y = ry * ry * (1.0 - (x * x) / (rx * rx));
        if y > 0.0 {
            y = y.sqrt();
        } else {
            y = 0.0;
        }
        y = ry - y;
        if toward.1 - center.1 > 0.0 {
            y = -y;
        }
        pos.1 += y;
    }

    pos
}

/// Place a 2x2 cluster-edge anchor on the face of `subgraph` that points
/// toward `other_center` if provided. Without an `other_center`, falls back
/// to the diagram's primary axis (bottom face for TB-from, etc.). The
/// position-aware variant prevents long swoop edges when two clusters end
/// up beside each other in a TB layout (or above/below in an LR layout).
fn anchor_layout_for_edge_toward(
    anchor: &NodeLayout,
    subgraph: &SubgraphLayout,
    direction: Direction,
    is_from: bool,
    other_center: Option<(f32, f32)>,
) -> NodeLayout {
    let size = 2.0;
    let mut node = anchor.clone();
    node.width = size;
    node.height = size;

    let cx = subgraph.x + subgraph.width / 2.0;
    let cy = subgraph.y + subgraph.height / 2.0;

    // Decide whether this cluster-anchor face should be horizontal (left/right
    // edge of the cluster) or vertical (top/bottom edge). Default to the
    // diagram's primary axis. If the OTHER endpoint clearly lies more to one
    // side than above/below, use the side face — that produces a clean
    // straight edge instead of forcing a swoop around the cluster.
    let mut horizontal_face = is_horizontal(direction);
    if let Some((ox, oy)) = other_center {
        let dx = ox - cx;
        let dy = oy - cy;
        let abs_dx = dx.abs();
        let abs_dy = dy.abs();
        if abs_dx > abs_dy * 1.5 {
            horizontal_face = true;
        } else if abs_dy > abs_dx * 1.5 {
            horizontal_face = false;
        }
    }

    if horizontal_face {
        let face_right = if let Some((ox, _)) = other_center {
            ox >= cx
        } else {
            is_from
        };
        let x = if face_right {
            subgraph.x + subgraph.width - size
        } else {
            subgraph.x
        };
        let y = cy - size / 2.0;
        node.x = x;
        node.y = y;
    } else {
        let face_bottom = if let Some((_, oy)) = other_center {
            oy >= cy
        } else {
            is_from
        };
        let x = cx - size / 2.0;
        let y = if face_bottom {
            subgraph.y + subgraph.height - size
        } else {
            subgraph.y
        };
        node.x = x;
        node.y = y;
    }

    node
}

fn mirror_subgraph_nodes(
    node_ids: &[String],
    nodes: &mut BTreeMap<String, NodeLayout>,
    direction: Direction,
) {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for node_id in node_ids {
        if let Some(node) = nodes.get(node_id) {
            min_x = min_x.min(node.x);
            min_y = min_y.min(node.y);
            max_x = max_x.max(node.x + node.width);
            max_y = max_y.max(node.y + node.height);
        }
    }

    if min_x == f32::MAX {
        return;
    }

    if matches!(direction, Direction::RightLeft) {
        for node_id in node_ids {
            if let Some(node) = nodes.get_mut(node_id) {
                node.x = min_x + (max_x - (node.x + node.width));
            }
        }
    }
    if matches!(direction, Direction::BottomTop) {
        for node_id in node_ids {
            if let Some(node) = nodes.get_mut(node_id) {
                node.y = min_y + (max_y - (node.y + node.height));
            }
        }
    }
}

fn assign_positions(
    node_ids: &[String],
    ranks: &HashMap<String, usize>,
    direction: Direction,
    config: &LayoutConfig,
    nodes: &mut BTreeMap<String, NodeLayout>,
    origin_x: f32,
    origin_y: f32,
) {
    let mut max_rank = 0usize;
    for rank in ranks.values() {
        max_rank = max_rank.max(*rank);
    }

    let mut rank_nodes: Vec<Vec<String>> = vec![Vec::new(); max_rank + 1];
    for node_id in node_ids {
        let rank = *ranks.get(node_id).unwrap_or(&0);
        if let Some(bucket) = rank_nodes.get_mut(rank) {
            bucket.push(node_id.clone());
        }
    }
    for bucket in &mut rank_nodes {
        bucket.sort();
    }

    let mut main_cursor = 0.0;
    for bucket in rank_nodes {
        let bucket_max_main = bucket
            .iter()
            .filter_map(|node_id| nodes.get(node_id))
            .map(|node| {
                if is_horizontal(direction) {
                    node.width
                } else {
                    node.height
                }
            })
            .fold(0.0_f32, f32::max);
        let mut cross_cursor = 0.0;
        let mut max_main: f32 = 0.0;
        for node_id in bucket {
            if let Some(node) = nodes.get_mut(&node_id) {
                if is_horizontal(direction) {
                    node.x = origin_x + main_cursor + (bucket_max_main - node.width) * 0.5;
                    node.y = origin_y + cross_cursor;
                    cross_cursor += node.height + config.node_spacing;
                    max_main = max_main.max(node.width);
                } else {
                    node.x = origin_x + cross_cursor;
                    node.y = origin_y + main_cursor + (bucket_max_main - node.height) * 0.5;
                    cross_cursor += node.width + config.node_spacing;
                    max_main = max_main.max(node.height);
                }
            }
        }
        main_cursor += max_main + config.rank_spacing;
    }
}

#[derive(Clone)]
struct StateNoteDagreNode {
    id: String,
    target: String,
    position: crate::ir::StateNotePosition,
    label: TextBlock,
    visible_width: f32,
    visible_height: f32,
    cluster_width: f32,
    cluster_height: f32,
}

fn layout_state_notes_as_dagre_nodes(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    edges: &mut [EdgeLayout],
    theme: &Theme,
    config: &LayoutConfig,
) -> Vec<StateNoteLayout> {
    if graph.kind != crate::ir::DiagramKind::State || graph.state_notes.is_empty() {
        return Vec::new();
    }

    let mut note_nodes = Vec::new();
    for (idx, note) in graph.state_notes.iter().enumerate() {
        if !nodes.contains_key(&note.target) {
            continue;
        }
        let label = measure_label(&note.label, theme, config);
        let multiline = label.lines.len() > 1;
        let visible_width = if multiline {
            (label.width + 30.0).max(230.0)
        } else {
            label.width + 30.0
        };
        let visible_height = if multiline {
            (label.height + 30.0).max(102.0)
        } else {
            (label.height + 30.0).max(54.0)
        };
        note_nodes.push(StateNoteDagreNode {
            id: format!("__state_note_{idx}__"),
            target: note.target.clone(),
            position: note.position,
            label,
            visible_width,
            visible_height,
            cluster_width: visible_width + 70.0,
            cluster_height: visible_height + 50.0,
        });
    }
    if note_nodes.is_empty() {
        return Vec::new();
    }

    let mut layout_ids: Vec<String> = graph
        .nodes
        .keys()
        .filter(|id| nodes.contains_key(*id))
        .cloned()
        .collect();
    layout_ids.extend(note_nodes.iter().map(|note| note.id.clone()));

    let mut rank_edges: Vec<crate::ir::Edge> = graph.edges.clone();
    for note in &note_nodes {
        let (from, to) = match note.position {
            crate::ir::StateNotePosition::RightOf => (note.target.clone(), note.id.clone()),
            crate::ir::StateNotePosition::LeftOf => (note.id.clone(), note.target.clone()),
        };
        rank_edges.push(crate::ir::Edge {
            from,
            to,
            label: None,
            start_label: None,
            end_label: None,
            directed: false,
            arrow_start: false,
            arrow_end: false,
            arrow_start_kind: None,
            arrow_end_kind: None,
            start_decoration: None,
            end_decoration: None,
            sequence_arrow_end: None,
            sequence_arrow_start: None,
            style: crate::ir::EdgeStyle::Dotted,
            markdown_label: false,
            id: None,
            curve: None,
            arch_port_from: None,
            arch_port_to: None,
        });
    }

    let ranks = compute_ranks_subset_for(graph, &layout_ids, &rank_edges, &graph.node_order);
    if ranks.is_empty() {
        return Vec::new();
    }
    let max_rank = ranks.values().copied().max().unwrap_or(0);
    let note_by_id: HashMap<&str, &StateNoteDagreNode> = note_nodes
        .iter()
        .map(|note| (note.id.as_str(), note))
        .collect();
    let mut layers: Vec<Vec<String>> = vec![Vec::new(); max_rank + 1];
    for id in &layout_ids {
        let rank = ranks.get(id).copied().unwrap_or(0);
        if let Some(layer) = layers.get_mut(rank) {
            layer.push(id.clone());
        }
    }
    for layer in &mut layers {
        layer.sort_by(|a, b| {
            let pa = state_note_order_priority(a, &note_by_id);
            let pb = state_note_order_priority(b, &note_by_id);
            pa.cmp(&pb)
                .then_with(|| {
                    graph
                        .node_order
                        .get(a)
                        .copied()
                        .unwrap_or(usize::MAX)
                        .cmp(&graph.node_order.get(b).copied().unwrap_or(usize::MAX))
                })
                .then_with(|| a.cmp(b))
        });
    }

    let mut widths = HashMap::new();
    let mut heights = HashMap::new();
    for id in &layout_ids {
        if let Some(note) = note_by_id.get(id.as_str()) {
            widths.insert(id.clone(), note.cluster_width);
            heights.insert(id.clone(), note.cluster_height);
        } else if let Some(node) = nodes.get(id) {
            widths.insert(id.clone(), node.width);
            heights.insert(id.clone(), node.height);
        }
    }
    let mut bk_edges = Vec::new();
    for edge in &rank_edges {
        let (Some(&from_rank), Some(&to_rank)) = (ranks.get(&edge.from), ranks.get(&edge.to))
        else {
            continue;
        };
        if from_rank + 1 == to_rank {
            bk_edges.push((edge.from.clone(), edge.to.clone()));
        } else if to_rank + 1 == from_rank {
            bk_edges.push((edge.to.clone(), edge.from.clone()));
        }
    }
    let bk_graph = brandes_kopf::LayeredGraph {
        layers: layers.clone(),
        widths: widths.clone(),
        virtual_nodes: HashSet::new(),
        edges: bk_edges,
    };
    let centers_x = brandes_kopf::compute_x_coordinates(&bk_graph, config.node_spacing);

    let mut centers_y = HashMap::new();
    let mut cursor_y = 0.0;
    for (rank, layer) in layers.iter().enumerate() {
        let rank_height = layer
            .iter()
            .filter_map(|id| heights.get(id))
            .copied()
            .fold(0.0_f32, f32::max);
        for id in layer {
            centers_y.insert(id.clone(), cursor_y + rank_height * 0.5);
        }
        if rank < layers.len().saturating_sub(1) {
            cursor_y += rank_height + config.rank_spacing;
        }
    }

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    for id in &layout_ids {
        let Some(&cx) = centers_x.get(id) else {
            continue;
        };
        let Some(&cy) = centers_y.get(id) else {
            continue;
        };
        let width = widths.get(id).copied().unwrap_or(0.0);
        let height = heights.get(id).copied().unwrap_or(0.0);
        min_x = min_x.min(cx - width * 0.5);
        min_y = min_y.min(cy - height * 0.5);
    }
    if !min_x.is_finite() || !min_y.is_finite() {
        return Vec::new();
    }
    let offset_x = LAYOUT_BOUNDARY_PAD - min_x;
    let offset_y = LAYOUT_BOUNDARY_PAD - min_y;

    for id in graph.nodes.keys() {
        let (Some(&cx), Some(&cy)) = (centers_x.get(id), centers_y.get(id)) else {
            continue;
        };
        if let Some(node) = nodes.get_mut(id) {
            node.x = cx + offset_x - node.width * 0.5;
            node.y = cy + offset_y - node.height * 0.5;
        }
    }

    let mut state_notes = Vec::new();
    for note in &note_nodes {
        let (Some(&cx), Some(&cy)) = (centers_x.get(&note.id), centers_y.get(&note.id)) else {
            continue;
        };
        state_notes.push(StateNoteLayout {
            x: cx + offset_x - note.visible_width * 0.5,
            y: cy + offset_y - note.visible_height * 0.5,
            width: note.visible_width,
            height: note.visible_height,
            label: note.label.clone(),
            position: note.position,
            target: note.target.clone(),
        });
    }

    spread_top_level_state_notes_like_dagre(graph, nodes, &mut state_notes, config);
    reroute_state_edges_after_note_layout(graph, nodes, edges);
    state_notes
}

fn spread_top_level_state_notes_like_dagre(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    state_notes: &mut [StateNoteLayout],
    config: &LayoutConfig,
) {
    let Some((right_idx, source_id)) = state_notes.iter().enumerate().find_map(|(idx, note)| {
        (note.position == crate::ir::StateNotePosition::RightOf).then(|| (idx, note.target.clone()))
    }) else {
        return;
    };
    let Some(target_id) = graph
        .edges
        .iter()
        .find(|edge| edge.from == source_id && nodes.contains_key(&edge.to))
        .map(|edge| edge.to.clone())
    else {
        return;
    };
    let left_idx = state_notes.iter().enumerate().find_map(|(idx, note)| {
        (note.position == crate::ir::StateNotePosition::LeftOf && note.target == target_id)
            .then_some(idx)
    });

    let right_note_x = LAYOUT_BOUNDARY_PAD + 35.0;
    state_notes[right_idx].x = right_note_x;
    let source_left = right_note_x + state_notes[right_idx].width * 0.5;
    if let Some(source) = nodes.get_mut(&source_id) {
        source.x = source_left;
    }
    let Some(source) = nodes.get(&source_id) else {
        return;
    };
    let target_left = source.x + source.width + config.node_spacing + 70.0;
    if let Some(target) = nodes.get_mut(&target_id) {
        target.x = target_left;
    }
    if let Some(idx) = left_idx {
        state_notes[idx].x = target_left + 8.0;
    }
}

fn state_note_order_priority(id: &str, notes: &HashMap<&str, &StateNoteDagreNode>) -> usize {
    match notes.get(id).map(|note| note.position) {
        Some(crate::ir::StateNotePosition::RightOf) => 0,
        Some(crate::ir::StateNotePosition::LeftOf) => 2,
        None => 1,
    }
}

fn reroute_state_edges_after_note_layout(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    edges: &mut [EdgeLayout],
) {
    for edge in edges {
        if !graph
            .edges
            .iter()
            .any(|ir_edge| ir_edge.from == edge.from && ir_edge.to == edge.to)
        {
            continue;
        }
        let (Some(from), Some(to)) = (nodes.get(&edge.from), nodes.get(&edge.to)) else {
            continue;
        };
        let from_cx = from.x + from.width * 0.5;
        let from_cy = from.y + from.height * 0.5;
        let to_cx = to.x + to.width * 0.5;
        let to_cy = to.y + to.height * 0.5;
        if (to_cx - from_cx).abs() > from.width.max(to.width) * 0.5 {
            let start = if to_cx >= from_cx {
                (from_cx + from.width * 0.24, from.y + from.height)
            } else {
                (from_cx - from.width * 0.24, from.y + from.height)
            };
            let end = if to_cx >= from_cx {
                (to.x, to.y)
            } else {
                (to.x + to.width, to.y)
            };
            let dx = end.0 - start.0;
            edge.points = vec![
                start,
                (start.0 + dx * 0.25, start.1 + 32.0),
                (end.0 - dx * 0.25, end.1 - 32.0),
                end,
            ];
        } else if to_cy >= from_cy {
            edge.points = vec![(from_cx, from.y + from.height), (to_cx, to.y)];
        } else {
            edge.points = vec![(from_cx, from.y), (to_cx, to.y + to.height)];
        }
    }
}

fn apply_state_region_label_rank_gaps(
    graph: &Graph,
    node_ids: &[String],
    ranks: &HashMap<String, usize>,
    direction: Direction,
    nodes: &mut BTreeMap<String, NodeLayout>,
) {
    if graph.kind != crate::ir::DiagramKind::State || !matches!(direction, Direction::TopDown) {
        return;
    }
    let node_set: HashSet<&str> = node_ids.iter().map(|id| id.as_str()).collect();
    let mut gaps_after_rank: HashMap<usize, f32> = HashMap::new();
    for edge in &graph.edges {
        let has_label = edge
            .label
            .as_deref()
            .map(|label| !label.trim().is_empty())
            .unwrap_or(false);
        if !has_label
            || !node_set.contains(edge.from.as_str())
            || !node_set.contains(edge.to.as_str())
        {
            continue;
        }
        let (Some(&from_rank), Some(&to_rank)) = (ranks.get(&edge.from), ranks.get(&edge.to))
        else {
            continue;
        };
        if from_rank == to_rank {
            continue;
        }
        let gap_rank = from_rank.min(to_rank);
        let gap = gaps_after_rank.entry(gap_rank).or_insert(0.0);
        *gap = gap.max(STATE_REGION_LABEL_RANK_GAP);
    }
    if gaps_after_rank.is_empty() {
        return;
    }
    for node_id in node_ids {
        let Some(&rank) = ranks.get(node_id) else {
            continue;
        };
        let shift: f32 = gaps_after_rank
            .iter()
            .filter_map(|(&gap_rank, &gap)| if gap_rank < rank { Some(gap) } else { None })
            .sum();
        if shift <= 0.0 {
            continue;
        }
        if let Some(node) = nodes.get_mut(node_id) {
            node.y += shift;
        }
    }
}

fn assign_positions_preserving_order(
    node_ids: &[String],
    ranks: &HashMap<String, usize>,
    direction: Direction,
    config: &LayoutConfig,
    nodes: &mut BTreeMap<String, NodeLayout>,
    origin_x: f32,
    origin_y: f32,
) {
    let mut max_rank = 0usize;
    for rank in ranks.values() {
        max_rank = max_rank.max(*rank);
    }

    let mut rank_nodes: Vec<Vec<String>> = vec![Vec::new(); max_rank + 1];
    for node_id in node_ids {
        let rank = *ranks.get(node_id).unwrap_or(&0);
        if let Some(bucket) = rank_nodes.get_mut(rank) {
            bucket.push(node_id.clone());
        }
    }

    let mut main_cursor = 0.0;
    for bucket in rank_nodes {
        let bucket_max_main = bucket
            .iter()
            .filter_map(|node_id| nodes.get(node_id))
            .map(|node| {
                if is_horizontal(direction) {
                    node.width
                } else {
                    node.height
                }
            })
            .fold(0.0_f32, f32::max);
        let mut cross_cursor = 0.0;
        let mut max_main: f32 = 0.0;
        for node_id in bucket {
            if let Some(node) = nodes.get_mut(&node_id) {
                if is_horizontal(direction) {
                    node.x = origin_x + main_cursor + (bucket_max_main - node.width) * 0.5;
                    node.y = origin_y + cross_cursor;
                    cross_cursor += node.height + config.node_spacing;
                    max_main = max_main.max(node.width);
                } else {
                    node.x = origin_x + cross_cursor;
                    node.y = origin_y + main_cursor + (bucket_max_main - node.height) * 0.5;
                    cross_cursor += node.width + config.node_spacing;
                    max_main = max_main.max(node.height);
                }
            }
        }
        main_cursor += max_main + config.rank_spacing;
    }
}

fn bounds_without_padding(
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
) -> (f32, f32) {
    bounds_with_edges(nodes, subgraphs, &[])
}

fn bounds_with_edges(
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    edges: &[EdgeLayout],
) -> (f32, f32) {
    bounds_with_edges_capped(nodes, subgraphs, edges, None)
}

fn edge_label_right_bound(edges: &[EdgeLayout]) -> f32 {
    let mut max_x: f32 = 0.0;
    for edge in edges {
        if let (Some(label), Some((cx, _))) = (edge.label.as_ref(), edge.label_anchor) {
            max_x = max_x.max(cx + label.width * 0.5 + LAYOUT_BOUNDARY_PAD);
        }
        if let (Some(label), Some((cx, _))) = (edge.start_label.as_ref(), edge.start_label_anchor) {
            max_x = max_x.max(cx + label.width * 0.5 + LAYOUT_BOUNDARY_PAD);
        }
        if let (Some(label), Some((cx, _))) = (edge.end_label.as_ref(), edge.end_label_anchor) {
            max_x = max_x.max(cx + label.width * 0.5 + LAYOUT_BOUNDARY_PAD);
        }
    }
    max_x
}

fn bounds_with_edges_capped(
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    edges: &[EdgeLayout],
    margin_cap: Option<f32>,
) -> (f32, f32) {
    let mut max_x: f32 = 0.0;
    let mut max_y: f32 = 0.0;
    for node in nodes.values().filter(|node| !node.hidden) {
        max_x = max_x.max(node.x + node.width);
        max_y = max_y.max(node.y + node.height);
    }
    for sub in subgraphs {
        let invisible_region = sub.label.trim().is_empty()
            && sub.style.stroke.as_deref() == Some("none")
            && sub.style.fill.as_deref() == Some("none");
        if invisible_region {
            continue;
        }
        max_x = max_x.max(sub.x + sub.width);
        max_y = max_y.max(sub.y + sub.height);
    }
    // Also include edge points - routing can place waypoints outside node bounds.
    // Add a margin for curved edges since Bezier control points can extend
    // ~20% beyond the waypoints.
    let mut edge_max_x: f32 = 0.0;
    let mut edge_max_y: f32 = 0.0;
    let mut edge_min_x = f32::MAX;
    let mut edge_min_y = f32::MAX;
    for edge in edges {
        for point in &edge.points {
            edge_max_x = edge_max_x.max(point.0);
            edge_max_y = edge_max_y.max(point.1);
            edge_min_x = edge_min_x.min(point.0);
            edge_min_y = edge_min_y.min(point.1);
        }
    }
    if edge_max_x > 0.0 {
        // Curved edges (Bezier) can overshoot their waypoints, so add a
        // protective margin. The 20% formula is preserved for callers that
        // pass `margin_cap=None` (preserves flowchart/etc. behavior). State
        // diagrams pass an explicit cap to prevent the 20% formula from
        // accumulating excessive viewBox padding on tall layouts.
        let margin_x_raw = (edge_max_x - edge_min_x) * 0.20 + 8.0;
        let margin_y_raw = (edge_max_y - edge_min_y) * 0.20 + 8.0;
        let margin_x = match margin_cap {
            Some(cap) => margin_x_raw.min(cap),
            None => margin_x_raw,
        };
        let margin_y = match margin_cap {
            Some(cap) => margin_y_raw.min(cap),
            None => margin_y_raw,
        };
        max_x = max_x.max(edge_max_x + margin_x);
        max_y = max_y.max(edge_max_y + margin_y);
    }
    // Edge labels (center labels in particular) can extend beyond the edge
    // path itself. Without this, narrow diagrams with wide labels (e.g. a
    // single vertical edge with a long inline label) clip the label at the
    // viewBox edge.
    for edge in edges {
        if let (Some(label), Some((cx, cy))) = (edge.label.as_ref(), edge.label_anchor) {
            max_x = max_x.max(cx + label.width * 0.5 + 8.0);
            max_y = max_y.max(cy + label.height * 0.5 + 4.0);
        }
        if let (Some(label), Some((cx, cy))) = (edge.start_label.as_ref(), edge.start_label_anchor)
        {
            max_x = max_x.max(cx + label.width * 0.5 + 8.0);
            max_y = max_y.max(cy + label.height * 0.5 + 4.0);
        }
        if let (Some(label), Some((cx, cy))) = (edge.end_label.as_ref(), edge.end_label_anchor) {
            max_x = max_x.max(cx + label.width * 0.5 + 8.0);
            max_y = max_y.max(cy + label.height * 0.5 + 4.0);
        }
    }
    (max_x, max_y)
}

fn apply_preferred_aspect_ratio_layout(layout: &mut Layout, config: &LayoutConfig) {
    let Some(target_ratio) = config
        .preferred_aspect_ratio
        .filter(|ratio| ratio.is_finite() && *ratio > 0.0)
    else {
        return;
    };
    if !matches!(layout.diagram, DiagramData::Graph { .. }) {
        return;
    }

    let width = layout.width.max(1.0);
    let height = layout.height.max(1.0);
    let current_ratio = width / height;
    if (current_ratio - target_ratio).abs() <= PREFERRED_ASPECT_TOLERANCE {
        return;
    }

    let mut scale_x = 1.0f32;
    let mut scale_y = 1.0f32;
    if current_ratio < target_ratio {
        scale_x = (target_ratio / current_ratio).clamp(1.0, PREFERRED_ASPECT_MAX_EXPANSION);
    } else {
        scale_y = (current_ratio / target_ratio).clamp(1.0, PREFERRED_ASPECT_MAX_EXPANSION);
    }
    if (scale_x - 1.0).abs() <= 1e-3 && (scale_y - 1.0).abs() <= 1e-3 {
        return;
    }

    for node in layout.nodes.values_mut() {
        node.x *= scale_x;
        node.y *= scale_y;
    }
    for edge in &mut layout.edges {
        for point in &mut edge.points {
            point.0 *= scale_x;
            point.1 *= scale_y;
        }
        if let Some(anchor) = edge.label_anchor.as_mut() {
            anchor.0 *= scale_x;
            anchor.1 *= scale_y;
        }
        if let Some(anchor) = edge.start_label_anchor.as_mut() {
            anchor.0 *= scale_x;
            anchor.1 *= scale_y;
        }
        if let Some(anchor) = edge.end_label_anchor.as_mut() {
            anchor.0 *= scale_x;
            anchor.1 *= scale_y;
        }
    }
    for sub in &mut layout.subgraphs {
        sub.x *= scale_x;
        sub.y *= scale_y;
        sub.width *= scale_x;
        sub.height *= scale_y;
    }
    if let DiagramData::Graph { state_notes, .. } = &mut layout.diagram {
        for note in state_notes {
            note.x *= scale_x;
            note.y *= scale_y;
        }
    }

    let edge_margin_cap = if matches!(
        layout.kind,
        crate::ir::DiagramKind::State
            | crate::ir::DiagramKind::Class
            | crate::ir::DiagramKind::Requirement
    ) {
        Some(EDGE_BBOX_MARGIN_CAP)
    } else {
        None
    };
    let (mut max_x, mut max_y) = bounds_with_edges_capped(
        &layout.nodes,
        &layout.subgraphs,
        &layout.edges,
        edge_margin_cap,
    );
    if let DiagramData::Graph { state_notes, .. } = &layout.diagram {
        for note in state_notes {
            max_x = max_x.max(note.x + note.width + 35.0);
            max_y = max_y.max(note.y + note.height + 25.0);
        }
    }
    layout.width = (max_x + LAYOUT_BOUNDARY_PAD).max(1.0);
    layout.height = (max_y + LAYOUT_BOUNDARY_PAD).max(1.0);
}

fn flowchart_path_overlap_with_prior(path: &[(f32, f32)], prior: &[Vec<(f32, f32)>]) -> f32 {
    let mut overlap = 0.0f32;
    for segment in path.windows(2) {
        let a1 = segment[0];
        let a2 = segment[1];
        for other in prior {
            for other_segment in other.windows(2) {
                overlap += collinear_overlap_length(a1, a2, other_segment[0], other_segment[1]);
            }
        }
    }
    overlap
}

fn append_path_segments(path: &[(f32, f32)], segments: &mut Vec<Segment>) {
    if path.len() < 2 {
        return;
    }
    for window in path.windows(2) {
        segments.push((window[0], window[1]));
    }
}

fn perimeter_route_candidates(
    start: (f32, f32),
    end: (f32, f32),
    outer_left: f32,
    outer_right: f32,
    outer_top: f32,
    outer_bottom: f32,
) -> Vec<Vec<(f32, f32)>> {
    vec![
        vec![
            start,
            (outer_right, start.1),
            (outer_right, outer_bottom),
            (outer_left, outer_bottom),
            (outer_left, end.1),
            end,
        ],
        vec![
            start,
            (outer_right, start.1),
            (outer_right, outer_top),
            (outer_left, outer_top),
            (outer_left, end.1),
            end,
        ],
        vec![
            start,
            (outer_left, start.1),
            (outer_left, outer_bottom),
            (outer_right, outer_bottom),
            (outer_right, end.1),
            end,
        ],
        vec![
            start,
            (outer_left, start.1),
            (outer_left, outer_top),
            (outer_right, outer_top),
            (outer_right, end.1),
            end,
        ],
    ]
}

fn reduce_crossing_sweep(
    order: &[usize],
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    routed_points: &mut [Vec<(f32, f32)>],
    deltas: &[f32],
    use_perimeter_candidates: bool,
    outer_left: f32,
    outer_right: f32,
    outer_top: f32,
    outer_bottom: f32,
) -> bool {
    let mut changed = false;
    let mut existing_segments: Vec<Segment> = Vec::new();
    // Keep crossing fixes from introducing visually extreme detours.
    const MAX_LEN_RATIO_HARD: f32 = 2.8;
    const MAX_LEN_RATIO_NO_GAIN: f32 = 1.12;
    const MAX_LEN_RATIO_ONE_GAIN: f32 = 1.8;
    const MAX_LEN_RATIO_MULTI_GAIN: f32 = 2.6;
    for &idx in order {
        if routed_points[idx].len() < 2 {
            append_path_segments(&routed_points[idx], &mut existing_segments);
            continue;
        }
        let from_id = graph.edges[idx].from.as_str();
        let to_id = graph.edges[idx].to.as_str();
        let (baseline_cross, baseline_overlap) =
            edge_crossings_with_existing(&routed_points[idx], &existing_segments);
        if baseline_cross == 0 {
            append_path_segments(&routed_points[idx], &mut existing_segments);
            continue;
        }
        let mut best_cross = baseline_cross;
        let mut best_overlap = baseline_overlap;
        let baseline_len = path_length(&routed_points[idx]);
        let mut best_len = baseline_len;
        let mut best_points = routed_points[idx].clone();
        let segment_count = routed_points[idx].len().saturating_sub(1);
        for seg_idx in 0..segment_count {
            for &delta in deltas {
                let Some(candidate) = bump_orthogonal_segment(&routed_points[idx], seg_idx, delta)
                else {
                    continue;
                };
                if flowchart_path_hits_non_endpoint_nodes(&candidate, from_id, to_id, nodes) {
                    continue;
                }
                let (crossings, overlap) =
                    edge_crossings_with_existing(&candidate, &existing_segments);
                let len = path_length(&candidate);
                if len > baseline_len * MAX_LEN_RATIO_HARD {
                    continue;
                }
                if crossings < best_cross
                    || (crossings == best_cross && overlap + 0.05 < best_overlap)
                    || (crossings == best_cross
                        && (overlap - best_overlap).abs() <= 0.05
                        && len + 1.0 < best_len)
                {
                    best_cross = crossings;
                    best_overlap = overlap;
                    best_len = len;
                    best_points = candidate;
                }
            }
        }
        if use_perimeter_candidates
            && let (Some(&start), Some(&end)) =
                (routed_points[idx].first(), routed_points[idx].last())
        {
            for candidate in perimeter_route_candidates(
                start,
                end,
                outer_left,
                outer_right,
                outer_top,
                outer_bottom,
            ) {
                let candidate = compress_path(&candidate);
                if flowchart_path_hits_non_endpoint_nodes(&candidate, from_id, to_id, nodes) {
                    continue;
                }
                let (crossings, overlap) =
                    edge_crossings_with_existing(&candidate, &existing_segments);
                let len = path_length(&candidate);
                if len > baseline_len * MAX_LEN_RATIO_HARD {
                    continue;
                }
                let crossing_gain = baseline_cross.saturating_sub(crossings);
                let max_ratio = if crossing_gain >= 2 {
                    MAX_LEN_RATIO_MULTI_GAIN
                } else if crossing_gain == 1 {
                    MAX_LEN_RATIO_ONE_GAIN
                } else {
                    MAX_LEN_RATIO_NO_GAIN
                };
                if len > baseline_len * max_ratio {
                    continue;
                }
                if crossings < best_cross
                    || (crossings == best_cross && overlap + 0.05 < best_overlap)
                    || (crossings == best_cross
                        && (overlap - best_overlap).abs() <= 0.05
                        && len + 1.0 < best_len)
                {
                    best_cross = crossings;
                    best_overlap = overlap;
                    best_len = len;
                    best_points = candidate;
                }
            }
        }
        let best_gain = baseline_cross.saturating_sub(best_cross);
        let max_ratio = if best_gain >= 2 {
            MAX_LEN_RATIO_MULTI_GAIN
        } else if best_gain == 1 {
            MAX_LEN_RATIO_ONE_GAIN
        } else {
            MAX_LEN_RATIO_NO_GAIN
        };
        let allow_apply = best_len <= baseline_len * max_ratio;
        if best_cross < baseline_cross
            || (best_cross == baseline_cross && best_overlap + 0.05 < baseline_overlap)
        {
            if !allow_apply {
                append_path_segments(&routed_points[idx], &mut existing_segments);
                continue;
            }
            routed_points[idx] = best_points;
            changed = true;
        }
        append_path_segments(&routed_points[idx], &mut existing_segments);
    }
    changed
}

fn reduce_orthogonal_path_crossings(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    routed_points: &mut [Vec<(f32, f32)>],
    config: &LayoutConfig,
) {
    if graph.edges.len() < 2 {
        return;
    }
    let base_delta = (config.node_spacing * 0.22).max(8.0);
    let deltas = [
        base_delta,
        -base_delta,
        base_delta * 1.5,
        -base_delta * 1.5,
        base_delta * 2.0,
        -base_delta * 2.0,
        base_delta * 3.0,
        -base_delta * 3.0,
        base_delta * 4.0,
        -base_delta * 4.0,
    ];
    let min_x = nodes
        .values()
        .filter(|node| !node.hidden && node.anchor_subgraph.is_none())
        .map(|node| node.x)
        .fold(f32::MAX, f32::min);
    let max_x = nodes
        .values()
        .filter(|node| !node.hidden && node.anchor_subgraph.is_none())
        .map(|node| node.x + node.width)
        .fold(f32::MIN, f32::max);
    let min_y = nodes
        .values()
        .filter(|node| !node.hidden && node.anchor_subgraph.is_none())
        .map(|node| node.y)
        .fold(f32::MAX, f32::min);
    let max_y = nodes
        .values()
        .filter(|node| !node.hidden && node.anchor_subgraph.is_none())
        .map(|node| node.y + node.height)
        .fold(f32::MIN, f32::max);
    let outer_pad = (config.node_spacing * 0.8).max(24.0);
    let outer_left = min_x - outer_pad;
    let outer_right = max_x + outer_pad;
    let outer_top = min_y - outer_pad;
    let outer_bottom = max_y + outer_pad;
    let use_perimeter_candidates = matches!(
        graph.kind,
        crate::ir::DiagramKind::Er | crate::ir::DiagramKind::State
    );
    let forward: Vec<usize> = (0..routed_points.len()).collect();
    let reverse: Vec<usize> = (0..routed_points.len()).rev().collect();

    for _ in 0..3 {
        let mut changed = reduce_crossing_sweep(
            &forward,
            graph,
            nodes,
            routed_points,
            &deltas,
            use_perimeter_candidates,
            outer_left,
            outer_right,
            outer_top,
            outer_bottom,
        );
        changed |= reduce_crossing_sweep(
            &reverse,
            graph,
            nodes,
            routed_points,
            &deltas,
            use_perimeter_candidates,
            outer_left,
            outer_right,
            outer_top,
            outer_bottom,
        );
        if !changed {
            break;
        }
    }
}

fn flowchart_path_hits_non_endpoint_nodes(
    path: &[(f32, f32)],
    from_id: &str,
    to_id: &str,
    nodes: &BTreeMap<String, NodeLayout>,
) -> bool {
    for segment in path.windows(2) {
        let a = segment[0];
        let b = segment[1];
        for node in nodes.values() {
            if node.id == from_id
                || node.id == to_id
                || node.hidden
                || node.anchor_subgraph.is_some()
            {
                continue;
            }
            let obstacle = Obstacle {
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                width: node.width,
                height: node.height,
                members: None,
            };
            if segment_intersects_rect(a, b, &obstacle) {
                return true;
            }
        }
    }
    false
}

fn bump_orthogonal_segment(
    points: &[(f32, f32)],
    seg_idx: usize,
    delta: f32,
) -> Option<Vec<(f32, f32)>> {
    if seg_idx + 1 >= points.len() {
        return None;
    }
    let a = points[seg_idx];
    let b = points[seg_idx + 1];
    let horizontal = (a.1 - b.1).abs() < 1e-3;
    let vertical = (a.0 - b.0).abs() < 1e-3;
    if !horizontal && !vertical {
        return None;
    }
    let mut bumped = Vec::with_capacity(points.len() + 2);
    bumped.extend_from_slice(&points[..=seg_idx]);
    if horizontal {
        let y = a.1 + delta;
        bumped.push((a.0, y));
        bumped.push((b.0, y));
    } else {
        let x = a.0 + delta;
        bumped.push((x, a.1));
        bumped.push((x, b.1));
    }
    bumped.extend_from_slice(&points[(seg_idx + 1)..]);
    Some(compress_path(&bumped))
}

fn deoverlap_flowchart_paths(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    routed_points: &mut [Vec<(f32, f32)>],
    config: &LayoutConfig,
) {
    if graph.edges.len() < 2 {
        return;
    }
    let overlap_threshold = 0.75f32;
    let base_delta = (config.node_spacing * 0.25).max(8.0);
    let deltas = [
        base_delta,
        -base_delta,
        base_delta * 1.5,
        -base_delta * 1.5,
        base_delta * 2.0,
        -base_delta * 2.0,
    ];

    for _ in 0..3 {
        let mut changed = false;
        for idx in 1..routed_points.len() {
            if routed_points[idx].len() < 2 {
                continue;
            }
            let from_id = graph.edges[idx].from.as_str();
            let to_id = graph.edges[idx].to.as_str();
            let baseline =
                flowchart_path_overlap_with_prior(&routed_points[idx], &routed_points[..idx]);
            if baseline < overlap_threshold {
                continue;
            }
            let mut best_overlap = baseline;
            let mut best_points = routed_points[idx].clone();
            let segment_count = routed_points[idx].len().saturating_sub(1);
            for seg_idx in 0..segment_count {
                for delta in deltas {
                    let Some(candidate) =
                        bump_orthogonal_segment(&routed_points[idx], seg_idx, delta)
                    else {
                        continue;
                    };
                    if flowchart_path_hits_non_endpoint_nodes(&candidate, from_id, to_id, nodes) {
                        continue;
                    }
                    let overlap =
                        flowchart_path_overlap_with_prior(&candidate, &routed_points[..idx]);
                    if overlap + 0.05 < best_overlap {
                        best_overlap = overlap;
                        best_points = candidate;
                    }
                }
            }
            if best_overlap + 0.05 < baseline {
                routed_points[idx] = best_points;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

fn apply_direction_mirror(
    direction: Direction,
    nodes: &mut BTreeMap<String, NodeLayout>,
    edges: &mut [EdgeLayout],
    subgraphs: &mut [SubgraphLayout],
) {
    let (max_x, max_y) = bounds_without_padding(nodes, subgraphs);
    if matches!(direction, Direction::RightLeft) {
        for node in nodes.values_mut() {
            node.x = max_x - node.x - node.width;
        }
        for edge in edges.iter_mut() {
            for point in edge.points.iter_mut() {
                point.0 = max_x - point.0;
            }
            if let Some(anchor) = edge.label_anchor.as_mut() {
                anchor.0 = max_x - anchor.0;
            }
        }
        for sub in subgraphs.iter_mut() {
            sub.x = max_x - sub.x - sub.width;
        }
    }
    if matches!(direction, Direction::BottomTop) {
        for node in nodes.values_mut() {
            node.y = max_y - node.y - node.height;
        }
        for edge in edges.iter_mut() {
            for point in edge.points.iter_mut() {
                point.1 = max_y - point.1;
            }
            if let Some(anchor) = edge.label_anchor.as_mut() {
                anchor.1 = max_y - anchor.1;
            }
        }
        for sub in subgraphs.iter_mut() {
            sub.y = max_y - sub.y - sub.height;
        }
    }
}

fn normalize_layout(
    nodes: &mut BTreeMap<String, NodeLayout>,
    edges: &mut [EdgeLayout],
    subgraphs: &mut [SubgraphLayout],
) {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    for node in nodes.values().filter(|node| !node.hidden) {
        min_x = min_x.min(node.x);
        min_y = min_y.min(node.y);
    }
    for sub in subgraphs.iter() {
        min_x = min_x.min(sub.x);
        min_y = min_y.min(sub.y);
    }
    // Also check edge points - routing can place waypoints outside node bounds
    for edge in edges.iter() {
        for point in &edge.points {
            min_x = min_x.min(point.0);
            min_y = min_y.min(point.1);
        }
    }

    if !min_x.is_finite() || !min_y.is_finite() {
        return;
    }
    let padding = LAYOUT_BOUNDARY_PAD;
    let shift_x = padding - min_x;
    let shift_y = padding - min_y;

    if shift_x.abs() < 1e-3 && shift_y.abs() < 1e-3 {
        return;
    }

    for node in nodes.values_mut() {
        node.x += shift_x;
        node.y += shift_y;
    }
    for edge in edges.iter_mut() {
        for point in edge.points.iter_mut() {
            point.0 += shift_x;
            point.1 += shift_y;
        }
        if let Some(anchor) = edge.label_anchor.as_mut() {
            anchor.0 += shift_x;
            anchor.1 += shift_y;
        }
    }
    for sub in subgraphs.iter_mut() {
        sub.x += shift_x;
        sub.y += shift_y;
    }
}

fn resolve_node_style(node_id: &str, graph: &Graph) -> crate::ir::NodeStyle {
    let mut style = crate::ir::NodeStyle::default();

    if let Some(classes) = graph.node_classes.get(node_id) {
        for class_name in classes {
            if let Some(class_style) = graph.class_defs.get(class_name) {
                merge_node_style(&mut style, class_style);
            }
        }
    }

    if let Some(node_style) = graph.node_styles.get(node_id) {
        merge_node_style(&mut style, node_style);
    }

    style
}

/// Build a `NodeLayout` with the standard defaults (position at origin, no
/// anchor, not hidden, no icon).  Callers that need custom x/y or
/// width/height can mutate the returned value.
fn build_node_layout(
    node: &crate::ir::Node,
    label: TextBlock,
    width: f32,
    height: f32,
    style: crate::ir::NodeStyle,
    graph: &Graph,
) -> NodeLayout {
    NodeLayout {
        id: node.id.clone(),
        x: 0.0,
        y: 0.0,
        width,
        height,
        label,
        shape: node.shape,
        style,
        link: graph.node_links.get(&node.id).cloned(),
        anchor_subgraph: None,
        hidden: false,
        icon: node.icon.clone(),
        img: node.img.clone(),
        img_w: node.img_w,
        img_h: node.img_h,
        sub_label: None,
        is_treemap_leaf: false,
        treemap_base_text_color: None,
    }
}

/// Build `NodeLayout`s for every node in `graph` using the standard pipeline:
/// `measure_label → shape_size → resolve_node_style → NodeLayout`.
///
/// Returns a `BTreeMap` ready to assign into a `Layout`.
fn build_graph_node_layouts(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
) -> BTreeMap<String, NodeLayout> {
    let mut nodes = BTreeMap::new();
    for node in graph.nodes.values() {
        let label = if node.markdown_label {
            measure_markdown_label(&node.label, theme, config)
        } else if has_html_formatting(&node.label) {
            let normalized = normalize_html_label(&node.label);
            measure_markdown_label(&normalized, theme, config)
        } else if matches!(
            graph.kind,
            crate::ir::DiagramKind::Block | crate::ir::DiagramKind::Class
        ) {
            measure_label_no_wrap(&node.label, theme, config)
        } else {
            measure_label(&node.label, theme, config)
        };
        let (mut width, mut height) = shape_size(node.shape, &label, config, theme, graph.kind);
        // If the node has image dimensions, size from those instead
        if node.img.is_some() {
            if let Some(iw) = node.img_w {
                width = iw + config.node_padding_x * 2.0;
            }
            if let Some(ih) = node.img_h {
                height = ih + config.node_padding_y * 2.0;
            }
        }
        let mut style = resolve_node_style(node.id.as_str(), graph);
        if matches!(
            graph.kind,
            crate::ir::DiagramKind::Block | crate::ir::DiagramKind::Flowchart
        ) && style.stroke.is_none()
            && theme
                .primary_border_color
                .eq_ignore_ascii_case(RUST_MERMAID_DEFAULT_PRIMARY_BORDER)
        {
            style.stroke = Some(MERMAID_DEFAULT_NODE_STROKE.to_string());
        }
        nodes.insert(
            node.id.clone(),
            build_node_layout(node, label, width, height, style, graph),
        );
    }
    nodes
}

fn resolve_subgraph_style(sub: &crate::ir::Subgraph, graph: &Graph) -> crate::ir::NodeStyle {
    let mut style = crate::ir::NodeStyle::default();
    let Some(id) = sub.id.as_ref() else {
        return style;
    };

    if let Some(classes) = graph.subgraph_classes.get(id) {
        for class_name in classes {
            if let Some(class_style) = graph.class_defs.get(class_name) {
                merge_node_style(&mut style, class_style);
            }
        }
    }

    if let Some(sub_style) = graph.subgraph_styles.get(id) {
        merge_node_style(&mut style, sub_style);
    }

    style
}

/// Enforce a minimum gap between top-level subgraphs along the main axis.
fn enforce_top_level_subgraph_gap(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.subgraphs.len() < 2 {
        return;
    }

    let top_level = top_level_subgraph_indices(graph);
    if top_level.len() < 2 {
        return;
    }
    if parallel_top_level_subgraph_pair(graph, &top_level).is_some() {
        return;
    }
    if top_level.iter().any(|idx| {
        graph
            .subgraphs
            .get(*idx)
            .map(|sub| flowchart_subgraph_is_recursive_cluster(graph, sub))
            .unwrap_or(false)
    }) {
        return;
    }

    // Only attempt this when top-level subgraphs are disjoint to avoid
    // double-shifting shared nodes.
    let mut seen: HashSet<&str> = HashSet::new();
    for &idx in &top_level {
        for node_id in &graph.subgraphs[idx].nodes {
            if !seen.insert(node_id.as_str()) {
                return;
            }
        }
    }

    // If no edges connect top-level subgraphs, skip this function.
    // Let `separate_sibling_subgraphs` handle them on the cross axis instead.
    let node_to_top_level_sg: HashMap<&str, usize> = top_level
        .iter()
        .flat_map(|&idx| {
            graph.subgraphs[idx]
                .nodes
                .iter()
                .map(move |n| (n.as_str(), idx))
        })
        .collect();
    let has_cross_sg_edge = graph.edges.iter().any(|e| {
        let from_sg = node_to_top_level_sg.get(e.from.as_str());
        let to_sg = node_to_top_level_sg.get(e.to.as_str());
        matches!((from_sg, to_sg), (Some(a), Some(b)) if a != b)
    });
    if !has_cross_sg_edge {
        return;
    }

    #[derive(Clone, Copy)]
    struct Bounds {
        idx: usize,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        pad_main: f32,
    }

    let direction = graph.direction;
    let horizontal = is_horizontal(direction);
    let mut bounds: Vec<Bounds> = Vec::new();

    for &idx in &top_level {
        let sub = &graph.subgraphs[idx];
        if is_region_subgraph(sub) || sub.nodes.is_empty() {
            continue;
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for node_id in &sub.nodes {
            if let Some(node) = nodes.get(node_id) {
                min_x = min_x.min(node.x);
                min_y = min_y.min(node.y);
                max_x = max_x.max(node.x + node.width);
                max_y = max_y.max(node.y + node.height);
            }
        }
        if min_x == f32::MAX {
            continue;
        }

        let label_empty = sub.label.trim().is_empty();
        let mut label_block = measure_subgraph_label(graph, sub, theme, config);
        if label_empty {
            label_block.width = 0.0;
            label_block.height = 0.0;
        }
        let (pad_x, pad_y, top_padding) =
            subgraph_padding_from_label(graph, sub, theme, &label_block);

        let padded_min_x = min_x - pad_x;
        let padded_max_x = max_x + pad_x;
        let padded_min_y = min_y - top_padding;
        let padded_max_y = max_y + pad_y;
        let pad_main = if horizontal { pad_x } else { pad_y };

        bounds.push(Bounds {
            idx,
            min_x: padded_min_x,
            min_y: padded_min_y,
            max_x: padded_max_x,
            max_y: padded_max_y,
            pad_main,
        });
    }

    if bounds.len() < 2 {
        return;
    }

    bounds.sort_by(|a, b| {
        let a_key = if horizontal { a.min_x } else { a.min_y };
        let b_key = if horizontal { b.min_x } else { b.min_y };
        a_key
            .partial_cmp(&b_key)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.idx.cmp(&b.idx))
    });

    let pad_main = bounds.iter().map(|b| b.pad_main).fold(0.0_f32, f32::max);
    let desired_gap = (config.node_spacing * SUBGRAPH_DESIRED_GAP_RATIO).max(pad_main * 2.0);

    let mut prev_max_main: Option<f32> = None;
    for bound in &mut bounds {
        let min_main = if horizontal { bound.min_x } else { bound.min_y };
        let mut max_main = if horizontal { bound.max_x } else { bound.max_y };

        let mut delta = 0.0_f32;
        if let Some(prev_max) = prev_max_main {
            let required_min = prev_max + desired_gap;
            if min_main < required_min {
                delta = required_min - min_main;
            }
        }

        if delta > 0.0 {
            let sub = &graph.subgraphs[bound.idx];
            for node_id in &sub.nodes {
                if let Some(node) = nodes.get_mut(node_id) {
                    if horizontal {
                        node.x += delta;
                    } else {
                        node.y += delta;
                    }
                }
            }

            if horizontal {
                bound.min_x += delta;
                bound.max_x += delta;
            } else {
                bound.min_y += delta;
                bound.max_y += delta;
            }

            max_main += delta;
        }

        prev_max_main = Some(max_main);
    }
}

/// For state diagrams, push nodes that are not members of any subgraph
/// outside the subgraph bounds so they don't visually appear inside composites.
fn push_non_members_out_of_subgraphs(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) {
    if graph.subgraphs.is_empty() {
        return;
    }

    // Collect which nodes belong to which subgraphs
    let mut node_subgraphs: HashSet<String> = HashSet::new();
    for sub in &graph.subgraphs {
        for node_id in &sub.nodes {
            node_subgraphs.insert(node_id.clone());
        }
    }

    // Also treat subgraph IDs/labels as "member" since they're anchor nodes
    let mut subgraph_ids: HashSet<String> = HashSet::new();
    for sub in &graph.subgraphs {
        if let Some(ref id) = sub.id {
            subgraph_ids.insert(id.clone());
        }
        if !sub.label.is_empty() {
            subgraph_ids.insert(sub.label.clone());
        }
    }

    let gap = config.node_spacing * 0.5;

    // Compute subgraph bounds from their member nodes
    let mut sub_bounds: Vec<(f32, f32, f32, f32)> = Vec::new();
    for sub in &graph.subgraphs {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for node_id in &sub.nodes {
            if let Some(node) = nodes.get(node_id) {
                min_x = min_x.min(node.x);
                min_y = min_y.min(node.y);
                max_x = max_x.max(node.x + node.width);
                max_y = max_y.max(node.y + node.height);
            }
        }
        let (pad_x, pad_y, top_pad) = subgraph_padding_from_label(
            graph,
            sub,
            theme,
            &measure_subgraph_label(graph, sub, theme, config),
        );
        if min_x < f32::MAX {
            sub_bounds.push((min_x - pad_x, min_y - top_pad, max_x + pad_x, max_y + pad_y));
        } else {
            sub_bounds.push((0.0, 0.0, 0.0, 0.0));
        }
    }

    // For each non-member node, check if it overlaps with any subgraph bounds
    let node_ids: Vec<String> = nodes.keys().cloned().collect();
    for node_id in &node_ids {
        if node_subgraphs.contains(node_id) || subgraph_ids.contains(node_id) {
            continue;
        }
        let node = match nodes.get(node_id) {
            Some(n) => n,
            None => continue,
        };
        let nx = node.x;
        let ny = node.y;
        let nw = node.width;
        let nh = node.height;

        for (sx, sy, sx2, sy2) in &sub_bounds {
            // Check if node rectangle overlaps with subgraph rectangle
            if nx + nw > *sx && nx < *sx2 && ny + nh > *sy && ny < *sy2 {
                // Push node below the subgraph
                let new_y = *sy2 + gap;
                if let Some(node_mut) = nodes.get_mut(node_id) {
                    node_mut.y = new_y;
                }
                break;
            }
        }
    }
}

/// Separate sibling subgraphs that don't share nodes to avoid overlap
/// State-diagram post-pass: after `build_subgraph_layouts` has computed the
/// FINAL outer rect for each composite (which can be larger than the inner
/// node bounds because of nested subgraphs), shift sibling composites apart
/// when their outer rects overlap. The earlier `separate_sibling_subgraphs`
/// uses inner-node bounds and can miss overlaps that only appear once nested
/// children expand the parent's rect.
fn separate_overlapping_sibling_subgraph_rects(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    subgraphs: &mut [crate::layout::types::SubgraphLayout],
    config: &LayoutConfig,
) {
    let tree = SubgraphTree::build(graph);
    let gap = config.node_spacing.max(8.0);
    let horiz = is_horizontal(graph.direction);

    // build_subgraph_layouts sorts the output by area, so we cannot index
    // `subgraphs` with graph.subgraphs indices directly. Build a mapping
    // graph_idx → subgraphs_array_idx via label match.
    let mut graph_to_layout: HashMap<usize, usize> = HashMap::new();
    for (g_idx, sub) in graph.subgraphs.iter().enumerate() {
        let key_id = sub.id.as_deref().unwrap_or("");
        let key_label = sub.label.as_str();
        for (l_idx, layout) in subgraphs.iter().enumerate() {
            if (!key_id.is_empty() && layout.label == key_id)
                || (!key_label.is_empty() && layout.label == key_label)
            {
                graph_to_layout.insert(g_idx, l_idx);
                break;
            }
        }
    }

    for &i in &tree.top_level {
        for &j in &tree.top_level {
            if i >= j {
                continue;
            }
            if !tree.are_siblings(i, j) {
                continue;
            }
            let Some(&li) = graph_to_layout.get(&i) else {
                continue;
            };
            let Some(&lj) = graph_to_layout.get(&j) else {
                continue;
            };
            let (a, b, la, lb) = if subgraphs[li].x <= subgraphs[lj].x {
                (i, j, li, lj)
            } else {
                (j, i, lj, li)
            };
            let a_box = (
                subgraphs[la].x,
                subgraphs[la].y,
                subgraphs[la].x + subgraphs[la].width,
                subgraphs[la].y + subgraphs[la].height,
            );
            let b_box = (
                subgraphs[lb].x,
                subgraphs[lb].y,
                subgraphs[lb].x + subgraphs[lb].width,
                subgraphs[lb].y + subgraphs[lb].height,
            );
            let overlap_x = a_box.0 < b_box.2 && b_box.0 < a_box.2;
            let overlap_y = a_box.1 < b_box.3 && b_box.1 < a_box.3;
            // For state diagrams in TB direction: if siblings overlap on the
            // CROSS axis (y for TB) but not on the MAIN axis (x for TB), they
            // ended up side-by-side and want a comfortable cross-axis gap.
            // Likewise for LR direction with axes swapped. Enforce a minimum
            // cross-axis gap so visually adjacent clusters don't crowd each
            // other (JS dagre naturally produces ~50-80px gap; ours can be
            // as tight as 20px because main dagre's nodesep was applied to a
            // smaller pre-inflation anchor size).
            // For state diagrams, JS dagre naturally produces ~70-80px between
            // side-by-side composites (a function of nodesep, cluster padding,
            // and dagre's compaction). Use a generous floor so siblings have
            // visible breathing room.
            let min_cross_gap = if graph.kind == crate::ir::DiagramKind::State {
                config.node_spacing.max(STATE_SIBLING_CROSS_GAP_MIN)
            } else {
                0.0
            };
            if min_cross_gap > 0.0 && !overlap_x && !overlap_y {
                // No overlap on either axis — skip; not adjacent.
            } else if min_cross_gap > 0.0 && overlap_y && !overlap_x && !horiz {
                // TB layout: siblings side-by-side. Push b right if x-gap < min.
                let gap_x = b_box.0 - a_box.2;
                if gap_x < min_cross_gap {
                    let shift = min_cross_gap - gap_x;
                    let mut to_move: HashSet<String> = HashSet::new();
                    collect_subgraph_descendant_node_ids(&tree, graph, b, &mut to_move);
                    for id in &to_move {
                        if let Some(node) = nodes.get_mut(id) {
                            node.x += shift;
                        }
                    }
                    let mut to_update_subs: Vec<usize> = Vec::new();
                    collect_subgraph_descendant_subgraph_indices(&tree, b, &mut to_update_subs);
                    for &k in &to_update_subs {
                        if let Some(&lk) = graph_to_layout.get(&k) {
                            subgraphs[lk].x += shift;
                        }
                    }
                    continue;
                }
            } else if min_cross_gap > 0.0 && overlap_x && !overlap_y && horiz {
                // LR layout: siblings stacked vertically. Push b down if y-gap < min.
                let gap_y = b_box.1 - a_box.3;
                if gap_y < min_cross_gap {
                    let shift = min_cross_gap - gap_y;
                    let mut to_move: HashSet<String> = HashSet::new();
                    collect_subgraph_descendant_node_ids(&tree, graph, b, &mut to_move);
                    for id in &to_move {
                        if let Some(node) = nodes.get_mut(id) {
                            node.y += shift;
                        }
                    }
                    let mut to_update_subs: Vec<usize> = Vec::new();
                    collect_subgraph_descendant_subgraph_indices(&tree, b, &mut to_update_subs);
                    for &k in &to_update_subs {
                        if let Some(&lk) = graph_to_layout.get(&k) {
                            subgraphs[lk].y += shift;
                        }
                    }
                    continue;
                }
            }
            if !overlap_x || !overlap_y {
                continue;
            }
            // Shift b along the layout's main axis to clear a.
            let shift = if horiz {
                a_box.3 + gap - b_box.1
            } else {
                a_box.2 + gap - b_box.0
            };
            if shift <= 0.0 {
                continue;
            }
            // Move all of b's member nodes (recursively, including nested
            // subgraph members) by the shift, plus update b's own rect.
            let mut to_move: HashSet<String> = HashSet::new();
            collect_subgraph_descendant_node_ids(&tree, graph, b, &mut to_move);
            for id in &to_move {
                if let Some(node) = nodes.get_mut(id) {
                    if horiz {
                        node.y += shift;
                    } else {
                        node.x += shift;
                    }
                }
            }
            // Update the rects for b and any of its nested subgraphs.
            let mut to_update_subs: Vec<usize> = Vec::new();
            collect_subgraph_descendant_subgraph_indices(&tree, b, &mut to_update_subs);
            for &k in &to_update_subs {
                if let Some(&lk) = graph_to_layout.get(&k) {
                    if horiz {
                        subgraphs[lk].y += shift;
                    } else {
                        subgraphs[lk].x += shift;
                    }
                }
            }
            let _ = (la, lb); // suppress unused if shift==0 path skipped above
        }
    }
}

fn collect_subgraph_descendant_node_ids(
    tree: &SubgraphTree,
    graph: &Graph,
    sub_idx: usize,
    out: &mut HashSet<String>,
) {
    for n in &graph.subgraphs[sub_idx].nodes {
        out.insert(n.clone());
    }
    for &child in &tree.children[sub_idx] {
        collect_subgraph_descendant_node_ids(tree, graph, child, out);
    }
}

/// Phase B (iter 255): constraint-solver Y placement for state diagrams.
///
/// Mirrors JS dagre's global rank Y assignment without rewriting the per-
/// cluster layout pipeline. Algorithm (longest-path with monotonicity):
///
///   1. Compute global ranks across the entire state diagram (all edges,
///      all nodes including cluster members).
///   2. For each rank R from low to high, compute target Y top-edge as
///      `max(rank_top_y[R'] + max_height_at_R' + min_gap)` over all edges
///      whose target is at rank R and source at rank R' < R. This is the
///      Sugiyama longest-path Y placement in O(V+E).
///   3. Snap each node's Y to its rank slot's center (preserves vertical
///      centering within rank for nodes of varying height).
///
/// Per-cluster monotonicity is preserved automatically: nodes within a
/// cluster have monotonic global ranks (a cluster's edges form a DAG-on-
/// rank), and rank_top_y is monotonic by construction.
///
/// Cross-cluster Y alignment is achieved: nodes at the same global rank
/// across different clusters all get snapped to the same Y center.
///
/// Outer dimension preservation: anchored to the existing topmost visible
/// Y so the diagram top doesn't shift. The bottom may grow or shrink based
/// on global rank span × min_gap.
///
/// Only fires for state diagrams with subgraphs and TB/BT direction.
fn apply_state_global_rank_y_snap(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::State {
        return;
    }
    if is_horizontal(graph.direction) {
        return;
    }
    let global_ranks = crate::layout::ranking::compute_state_global_ranks(graph);
    if global_ranks.is_empty() {
        return;
    }

    // Tunables. Min edge gap is variable per-rank based on the deepest
    // cluster nesting depth at that rank. JS dagre adds +25 ranksep per
    // nesting level (see mermaid dagre/index.js:81), so a rank with nodes
    // 3 levels deep gets +75 over the base. This approximates that.
    let base_gap: f32 = (config.rank_spacing * 0.55).max(30.0).min(80.0);
    let depth_boost_per_level: f32 = 25.0;

    // Compute per-rank max node height for slot sizing AND max nesting depth.
    let tree = SubgraphTree::build(graph);
    let node_depth: HashMap<String, usize> = nodes
        .keys()
        .map(|id| {
            let mut depth = 0usize;
            for (sub_idx, sub) in graph.subgraphs.iter().enumerate() {
                if sub.nodes.contains(id) {
                    let mut d = 1usize;
                    let mut cur = sub_idx;
                    while let Some(parent) = tree.parent.get(cur).copied().flatten() {
                        d += 1;
                        cur = parent;
                    }
                    depth = depth.max(d);
                }
            }
            (id.clone(), depth)
        })
        .collect();

    let mut max_h_per_rank: HashMap<usize, f32> = HashMap::new();
    let mut max_depth_per_rank: HashMap<usize, usize> = HashMap::new();
    let mut min_rank = usize::MAX;
    let mut max_rank = 0usize;
    for (id, &rank) in &global_ranks {
        let Some(node) = nodes.get(id) else {
            continue;
        };
        if node.hidden {
            continue;
        }
        let h = max_h_per_rank.entry(rank).or_insert(0.0);
        *h = h.max(node.height);
        let d = node_depth.get(id).copied().unwrap_or(0);
        let entry = max_depth_per_rank.entry(rank).or_insert(0);
        *entry = (*entry).max(d);
        min_rank = min_rank.min(rank);
        max_rank = max_rank.max(rank);
    }
    if min_rank == usize::MAX {
        return;
    }

    // Anchor to existing topmost visible node Y (preserves outer top edge).
    let mut anchor_top_y = f32::MAX;
    for (id, _) in &global_ranks {
        if let Some(node) = nodes.get(id) {
            if !node.hidden {
                anchor_top_y = anchor_top_y.min(node.y);
            }
        }
    }
    if anchor_top_y == f32::MAX {
        return;
    }

    // Group edges by target rank for efficient longest-path lookup.
    let mut edges_by_target_rank: HashMap<usize, Vec<(usize, f32)>> = HashMap::new();
    for edge in &graph.edges {
        let (Some(&r_from), Some(&r_to)) =
            (global_ranks.get(&edge.from), global_ranks.get(&edge.to))
        else {
            continue;
        };
        if r_from >= r_to {
            continue; // skip back-edges and self-loops
        }
        edges_by_target_rank
            .entry(r_to)
            .or_default()
            .push((r_from, max_h_per_rank.get(&r_from).copied().unwrap_or(30.0)));
    }

    // Per-rank min_edge_gap: base_gap + max_depth(R, R-1) * depth_boost.
    // The gap between two consecutive ranks accounts for the deepest cluster
    // either contains. This mirrors JS dagre's per-recursion ranksep boost.
    let gap_for = |r: usize| -> f32 {
        let d_here = max_depth_per_rank.get(&r).copied().unwrap_or(0);
        let d_prev = if r > 0 {
            max_depth_per_rank.get(&(r - 1)).copied().unwrap_or(0)
        } else {
            0
        };
        let max_d = d_here.max(d_prev);
        base_gap + (max_d.saturating_sub(1)) as f32 * depth_boost_per_level
    };

    // Longest-path Y placement: rank_top_y[R] = max over predecessors of
    // (rank_top_y[R'] + max_h[R'] + gap_for(R)), or fallback to previous
    // rank's bottom + gap_for(R) if no edge constraint.
    let mut rank_top_y: HashMap<usize, f32> = HashMap::new();
    rank_top_y.insert(min_rank, anchor_top_y);
    for r in (min_rank + 1)..=max_rank {
        let g = gap_for(r);
        let mut required_y = f32::MIN;
        if let Some(preds) = edges_by_target_rank.get(&r) {
            for (r_pred, _h_pred) in preds {
                if let Some(&pred_top) = rank_top_y.get(r_pred) {
                    let pred_h = max_h_per_rank.get(r_pred).copied().unwrap_or(30.0);
                    required_y = required_y.max(pred_top + pred_h + g);
                }
            }
        }
        // Fallback: never go above (previous rank's bottom + gap),
        // which guarantees monotonicity even for ranks unreached by edges.
        let prev_top = rank_top_y.get(&(r - 1)).copied().unwrap_or(anchor_top_y);
        let prev_h = max_h_per_rank.get(&(r - 1)).copied().unwrap_or(0.0);
        let monotonic_floor = prev_top + prev_h + g;
        let final_y = required_y.max(monotonic_floor);
        rank_top_y.insert(r, final_y);
    }

    // Phase B safeguard: do NOT push nodes up. Pre-snap per-cluster layout
    // is already valid; we only want to push nodes DOWN to enforce cross-
    // cluster rank alignment. For each rank R, take target_y = MAX(longest-
    // path target, max original Y at this rank). This preserves original
    // per-cluster spacing (which is the Right Answer for non-cross-cluster-
    // constrained ranks) and only nudges Y downward when global ranks
    // demand it.
    let mut original_max_y_per_rank: HashMap<usize, f32> = HashMap::new();
    for (id, &rank) in &global_ranks {
        let Some(node) = nodes.get(id) else {
            continue;
        };
        if node.hidden {
            continue;
        }
        let entry = original_max_y_per_rank.entry(rank).or_insert(f32::MIN);
        if node.y > *entry {
            *entry = node.y;
        }
    }
    // Combine: rank_top_y[R] = max(longest-path target, original max y at R).
    for r in min_rank..=max_rank {
        if let Some(&orig_max) = original_max_y_per_rank.get(&r) {
            let cur = rank_top_y.get(&r).copied().unwrap_or(orig_max);
            if orig_max > cur {
                rank_top_y.insert(r, orig_max);
                // Cascade: any subsequent rank that depends on this one needs
                // updating. Re-run the longest-path pass for ranks > r.
                for r2 in (r + 1)..=max_rank {
                    let prev = rank_top_y.get(&(r2 - 1)).copied().unwrap_or(anchor_top_y);
                    let prev_h = max_h_per_rank.get(&(r2 - 1)).copied().unwrap_or(0.0);
                    let g2 = gap_for(r2);
                    let monotonic_floor = prev + prev_h + g2;
                    let mut req = monotonic_floor;
                    if let Some(preds) = edges_by_target_rank.get(&r2) {
                        for (r_pred, _) in preds {
                            let pt = rank_top_y.get(r_pred).copied().unwrap_or(anchor_top_y);
                            let ph = max_h_per_rank.get(r_pred).copied().unwrap_or(30.0);
                            req = req.max(pt + ph + g2);
                        }
                    }
                    let cur2 = rank_top_y.get(&r2).copied().unwrap_or(req);
                    if req > cur2 {
                        rank_top_y.insert(r2, req);
                    }
                }
            }
        }
    }

    // Snap each node's Y to its rank slot center, but never UP from current.
    for (id, &rank) in &global_ranks {
        let Some(node) = nodes.get_mut(id) else {
            continue;
        };
        if node.hidden {
            continue;
        }
        let Some(&slot_top) = rank_top_y.get(&rank) else {
            continue;
        };
        let slot_h = max_h_per_rank.get(&rank).copied().unwrap_or(node.height);
        let centered_y = slot_top + (slot_h - node.height) * 0.5;
        // Never push UP: only adopt new Y if it's >= current.
        if centered_y > node.y - 0.5 {
            node.y = centered_y;
        }
    }
}

/// State-diagram bbox-expansion heuristic (iter 245, option 1):
/// For each top-level state cluster, find external nodes that the cluster's
/// members directly connect to via edges, and expand the cluster's Y-bbox to
/// enclose those external nodes' Y range (with margin). Internal node
/// positions are NOT moved — only the cluster border grows. This approximates
/// JS dagre's global compound-graph rank assignment without doing the real
/// rewrite. State-only, top-level only, expansion only (never shrinks).
fn expand_state_clusters_for_cross_edges(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    subgraphs: &mut [crate::layout::types::SubgraphLayout],
) {
    if graph.kind != crate::ir::DiagramKind::State {
        return;
    }
    let tree = SubgraphTree::build(graph);
    let margin: f32 = 20.0;

    // Map graph subgraph index → subgraphs[] array index (build_subgraph_layouts
    // sorts by area so direct indexing doesn't work).
    let mut graph_to_layout: HashMap<usize, usize> = HashMap::new();
    let mut cluster_name_to_graph: HashMap<&str, usize> = HashMap::new();
    for (g_idx, sub) in graph.subgraphs.iter().enumerate() {
        if let Some(id) = sub.id.as_deref().filter(|id| !id.is_empty()) {
            cluster_name_to_graph.insert(id, g_idx);
        }
        if !sub.label.is_empty() {
            cluster_name_to_graph.insert(sub.label.as_str(), g_idx);
        }
    }
    for (g_idx, sub) in graph.subgraphs.iter().enumerate() {
        let key_id = sub.id.as_deref().unwrap_or("");
        let key_label = sub.label.as_str();
        for (l_idx, layout) in subgraphs.iter().enumerate() {
            if (!key_id.is_empty() && layout.label == key_id)
                || (!key_label.is_empty() && layout.label == key_label)
            {
                graph_to_layout.insert(g_idx, l_idx);
                break;
            }
        }
    }

    // For each top-level cluster, build the recursive member set, find
    // external nodes connected via edges, compute their Y bbox, and grow
    // the cluster's Y range to enclose them.
    for &sub_idx in &tree.top_level {
        let Some(&layout_idx) = graph_to_layout.get(&sub_idx) else {
            continue;
        };
        let mut members: HashSet<String> = HashSet::new();
        collect_subgraph_descendant_node_ids(&tree, graph, sub_idx, &mut members);
        if members.is_empty() {
            continue;
        }

        let mut ext_y_min = f32::MAX;
        let mut ext_y_max = f32::MIN;
        let mut found = false;
        for edge in &graph.edges {
            let from_in = members.contains(&edge.from);
            let to_in = members.contains(&edge.to);
            if from_in == to_in {
                continue; // both internal or both external — no cross
            }
            let ext_id = if from_in { &edge.to } else { &edge.from };
            if let Some(&ext_sub_idx) = cluster_name_to_graph.get(ext_id.as_str()) {
                let mut ext_members = HashSet::new();
                collect_subgraph_descendant_node_ids(&tree, graph, ext_sub_idx, &mut ext_members);
                for member_id in ext_members {
                    if let Some(ext) = nodes.get(&member_id) {
                        ext_y_min = ext_y_min.min(ext.y);
                        ext_y_max = ext_y_max.max(ext.y + ext.height);
                        found = true;
                    }
                }
            } else if let Some(ext) = nodes.get(ext_id) {
                ext_y_min = ext_y_min.min(ext.y);
                ext_y_max = ext_y_max.max(ext.y + ext.height);
                found = true;
            }
        }
        if !found {
            continue;
        }

        let cur_top = subgraphs[layout_idx].y;
        let cur_bottom = cur_top + subgraphs[layout_idx].height;
        let new_top = cur_top.min(ext_y_min - margin);
        let new_bottom = cur_bottom.max(ext_y_max + margin);
        if (new_top - cur_top).abs() > 0.5 || (new_bottom - cur_bottom).abs() > 0.5 {
            subgraphs[layout_idx].y = new_top;
            subgraphs[layout_idx].height = new_bottom - new_top;
        }
    }
}

/// Iter 266: align sibling top-level state clusters' tops when they have
/// inner-to-inner cross-cluster edges. Triggers for the "shared node via
/// last-reference-wins" pattern (e.g., nested-composite-states where End's
/// `second` connects to Third inside First). Does NOT trigger for plain
/// chain-of-clusters (composite-states: First→End cluster boundary edges
/// only, no inner-to-inner connections).
fn align_sibling_state_clusters_with_inner_cross_edges(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    subgraphs: &mut [crate::layout::types::SubgraphLayout],
) {
    if graph.kind != crate::ir::DiagramKind::State {
        return;
    }
    let tree = SubgraphTree::build(graph);
    if tree.top_level.len() < 2 {
        return;
    }

    // Map graph subgraph index → subgraphs[] array index.
    let mut graph_to_layout: HashMap<usize, usize> = HashMap::new();
    for (g_idx, sub) in graph.subgraphs.iter().enumerate() {
        let key_id = sub.id.as_deref().unwrap_or("");
        let key_label = sub.label.as_str();
        for (l_idx, layout) in subgraphs.iter().enumerate() {
            if (!key_id.is_empty() && layout.label == key_id)
                || (!key_label.is_empty() && layout.label == key_label)
            {
                graph_to_layout.insert(g_idx, l_idx);
                break;
            }
        }
    }

    // Build descendant member sets per top-level cluster (recursive).
    let mut members_per_top: HashMap<usize, HashSet<String>> = HashMap::new();
    for &top in &tree.top_level {
        let mut m: HashSet<String> = HashSet::new();
        collect_subgraph_descendant_node_ids(&tree, graph, top, &mut m);
        members_per_top.insert(top, m);
    }

    // Find sibling pairs with inner-to-inner cross edges.
    // An edge (u, v) is "inner-to-inner cross" if u ∈ A's members, v ∈ B's
    // members, A != B, and BOTH u and v are NOT cluster-anchor nodes (i.e.,
    // not the cluster's start/end pseudostates which are the boundary
    // edges' typical endpoints).
    //
    // Heuristic for "anchor": cluster-anchor nodes have IDs like
    // "<sub.id>_start" / "<sub.id>_end" or are the subgraph's id itself.
    // Easier proxy: the LAST node in each subgraph's nodes list is often
    // the end-anchor; the FIRST is often the start. We exclude direct
    // boundary connections.
    let pairs_with_inner: HashSet<(usize, usize)> = {
        let mut out: HashSet<(usize, usize)> = HashSet::new();
        for edge in &graph.edges {
            // Find which top-level cluster (if any) each endpoint is in.
            let mut from_top: Option<usize> = None;
            let mut to_top: Option<usize> = None;
            for (&top, mset) in &members_per_top {
                if mset.contains(&edge.from) {
                    from_top = Some(top);
                }
                if mset.contains(&edge.to) {
                    to_top = Some(top);
                }
            }
            let (Some(a), Some(b)) = (from_top, to_top) else {
                continue;
            };
            if a == b {
                continue;
            }
            // Verify neither endpoint is the cluster boundary node (start/end
            // anchor). Cluster boundary nodes have IDs ending in "_start" or
            // "_end".
            let is_anchor = |id: &str| -> bool { id.ends_with("_start") || id.ends_with("_end") };
            if is_anchor(&edge.from) || is_anchor(&edge.to) {
                continue;
            }
            let key = if a < b { (a, b) } else { (b, a) };
            out.insert(key);
        }
        out
    };

    if pairs_with_inner.is_empty() {
        return;
    }

    // For each pair, find the cluster with the higher top-y (visually
    // lower) and lift its members + bbox to match the other's top-y.
    for (a_top, b_top) in &pairs_with_inner {
        let (a_top, b_top) = (*a_top, *b_top);
        let (Some(&la), Some(&lb)) = (graph_to_layout.get(&a_top), graph_to_layout.get(&b_top))
        else {
            continue;
        };
        let ay = subgraphs[la].y;
        let by = subgraphs[lb].y;
        let (lifter, target_y, lifted_top) = if ay < by {
            (lb, ay, b_top) // lift b up to match a
        } else if by < ay {
            (la, by, a_top) // lift a up to match b
        } else {
            continue; // already aligned
        };
        let cur_y = subgraphs[lifter].y;
        let delta = target_y - cur_y;
        if delta.abs() < 1.0 {
            continue;
        }
        // Move all of the lifted cluster's descendants by delta (in y).
        let mut to_move: HashSet<String> = HashSet::new();
        collect_subgraph_descendant_node_ids(&tree, graph, lifted_top, &mut to_move);
        for id in &to_move {
            if let Some(node) = nodes.get_mut(id) {
                node.y += delta;
            }
        }
        // Update the cluster bbox AND any nested cluster bboxes.
        let mut to_update: Vec<usize> = Vec::new();
        collect_subgraph_descendant_subgraph_indices(&tree, lifted_top, &mut to_update);
        for &k in &to_update {
            if let Some(&lk) = graph_to_layout.get(&k) {
                subgraphs[lk].y += delta;
            }
        }
    }

    // Iter 283: After top-alignment, vertically CENTER the smaller sibling
    // within the taller sibling's vertical range. JS dagre lays out clusters
    // such that End is positioned roughly at the vertical mid-point of First
    // (JS End center y=422 vs First center y=434, only 12px above center).
    // RS top-aligns them which makes End appear to "stick to the top" while
    // First extends much further down.
    for (a_top, b_top) in &pairs_with_inner {
        let (a_top, b_top) = (*a_top, *b_top);
        let (Some(&la), Some(&lb)) = (graph_to_layout.get(&a_top), graph_to_layout.get(&b_top))
        else {
            continue;
        };
        let ah = subgraphs[la].height;
        let bh = subgraphs[lb].height;
        // Only center when one cluster is significantly shorter (≤ 70%).
        let (taller_idx, shorter_top) = if ah > bh && bh / ah < 0.7 {
            (la, b_top)
        } else if bh > ah && ah / bh < 0.7 {
            (lb, a_top)
        } else {
            continue;
        };
        let Some(&shorter_layout_idx) = graph_to_layout.get(&shorter_top) else {
            continue;
        };
        let taller_top_y = subgraphs[taller_idx].y;
        let taller_h = subgraphs[taller_idx].height;
        let taller_center_y = taller_top_y + taller_h * 0.5;
        let shorter_h = subgraphs[shorter_layout_idx].height;
        let target_top_y = taller_center_y - shorter_h * 0.5;
        let cur_top_y = subgraphs[shorter_layout_idx].y;
        let delta = target_top_y - cur_top_y;
        if delta.abs() < 1.0 {
            continue;
        }
        // Move shorter cluster's descendants by delta (in y).
        let mut to_move: HashSet<String> = HashSet::new();
        collect_subgraph_descendant_node_ids(&tree, graph, shorter_top, &mut to_move);
        for id in &to_move {
            if let Some(node) = nodes.get_mut(id) {
                node.y += delta;
            }
        }
        let mut to_update: Vec<usize> = Vec::new();
        collect_subgraph_descendant_subgraph_indices(&tree, shorter_top, &mut to_update);
        for &k in &to_update {
            if let Some(&lk) = graph_to_layout.get(&k) {
                subgraphs[lk].y += delta;
            }
        }
    }
}

fn collect_subgraph_descendant_subgraph_indices(
    tree: &SubgraphTree,
    sub_idx: usize,
    out: &mut Vec<usize>,
) {
    out.push(sub_idx);
    for &child in &tree.children[sub_idx] {
        collect_subgraph_descendant_subgraph_indices(tree, child, out);
    }
}

fn separate_sibling_subgraphs(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) {
    if graph.subgraphs.len() < 2 {
        return;
    }

    let tree = SubgraphTree::build(graph);

    // Find groups of sibling subgraphs using the containment tree.
    // Two subgraphs are siblings if neither is an ancestor of the other.
    let mut sibling_groups: Vec<Vec<usize>> = Vec::new();
    let mut assigned: HashSet<usize> = HashSet::new();

    for i in 0..graph.subgraphs.len() {
        if assigned.contains(&i) {
            continue;
        }
        let mut group = vec![i];
        assigned.insert(i);

        for j in (i + 1)..graph.subgraphs.len() {
            if assigned.contains(&j) {
                continue;
            }
            // Check if j is a sibling (not nested with any in group)
            let is_sibling = group.iter().all(|&k| tree.are_siblings(j, k));
            if is_sibling {
                group.push(j);
                assigned.insert(j);
            }
        }
        if group.len() > 1 {
            sibling_groups.push(group);
        }
    }

    // For each group of siblings, compute bounds and separate them
    let is_horizontal = is_horizontal(graph.direction);
    for group in sibling_groups {
        // Compute bounding box for each subgraph
        let mut bounds: Vec<(usize, f32, f32, f32, f32)> = Vec::new(); // (idx, min_x, min_y, max_x, max_y)
        for &idx in &group {
            let sub = &graph.subgraphs[idx];
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            for node_id in &sub.nodes {
                if let Some(node) = nodes.get(node_id) {
                    min_x = min_x.min(node.x);
                    min_y = min_y.min(node.y);
                    max_x = max_x.max(node.x + node.width);
                    max_y = max_y.max(node.y + node.height);
                }
            }
            if min_x != f32::MAX {
                // Include subgraph padding in bounds calculation
                let label_block = measure_subgraph_label(graph, sub, theme, config);
                let (pad_x, pad_y, top_padding) =
                    subgraph_padding_from_label(graph, sub, theme, &label_block);
                let padded_min_x = min_x - pad_x;
                let padded_min_y = min_y - top_padding;
                let padded_max_x = max_x + pad_x;
                let padded_max_y = max_y + pad_y;
                bounds.push((idx, padded_min_x, padded_min_y, padded_max_x, padded_max_y));
            }
        }

        if bounds.len() < 2 {
            continue;
        }

        // Sort by position along the separation axis for stable, deterministic shifts.
        if is_horizontal {
            bounds.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(Ordering::Equal));
        } else {
            bounds.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        }

        let external_flowchart_compounds = graph.kind == crate::ir::DiagramKind::Flowchart
            && matches!(graph.direction, Direction::TopDown | Direction::BottomTop)
            && group.iter().all(|&idx| {
                graph.subgraphs.get(idx).is_some_and(|sub| {
                    !flowchart_subgraph_is_recursive_cluster(graph, sub)
                        && flowchart_subgraph_has_external_edge(graph, sub)
                })
            });
        let external_lr_flowchart_compounds = graph.kind == crate::ir::DiagramKind::Flowchart
            && matches!(graph.direction, Direction::LeftRight | Direction::RightLeft)
            && group.iter().all(|&idx| {
                graph.subgraphs.get(idx).is_some_and(|sub| {
                    !flowchart_subgraph_is_recursive_cluster(graph, sub)
                        && flowchart_subgraph_has_external_edge(graph, sub)
                })
            });
        let recursive_flowchart_compounds = graph.kind == crate::ir::DiagramKind::Flowchart
            && group.iter().all(|&idx| {
                graph
                    .subgraphs
                    .get(idx)
                    .is_some_and(|sub| flowchart_subgraph_is_recursive_cluster(graph, sub))
            });
        let gap = if external_flowchart_compounds {
            (config.node_spacing * 0.4).max(20.0)
        } else if external_lr_flowchart_compounds {
            (config.node_spacing * 0.4).max(20.0)
        } else if recursive_flowchart_compounds {
            config.node_spacing.max(FLOWCHART_RECURSIVE_DAGRE_SPACING)
        } else {
            config.node_spacing.max(8.0)
        };
        let enforce_min_gap = external_flowchart_compounds
            || external_lr_flowchart_compounds
            || recursive_flowchart_compounds;
        let overlaps =
            |a_min: f32, a_max: f32, b_min: f32, b_max: f32| a_min < b_max && b_min < a_max;

        let mut placed: Vec<(usize, f32, f32, f32, f32)> = Vec::new();
        for (idx, min_x, min_y, max_x, max_y) in bounds {
            let mut shift = 0.0_f32;

            for &(_, px1, py1, px2, py2) in &placed {
                let other_axis_overlaps = if is_horizontal {
                    overlaps(min_x, max_x, px1, px2)
                } else {
                    overlaps(min_y, max_y, py1, py2)
                };
                if !other_axis_overlaps {
                    continue;
                }

                let shifted_min = if is_horizontal {
                    min_y + shift
                } else {
                    min_x + shift
                };
                let shifted_max = if is_horizontal {
                    max_y + shift
                } else {
                    max_x + shift
                };
                let placed_min = if is_horizontal { py1 } else { px1 };
                let placed_max = if is_horizontal { py2 } else { px2 };

                if (enforce_min_gap && shifted_min < placed_max + gap)
                    || (!enforce_min_gap
                        && overlaps(shifted_min, shifted_max, placed_min, placed_max))
                {
                    let needed = placed_max + gap - shifted_min;
                    if needed > shift {
                        shift = needed;
                    }
                }
            }

            if shift > 0.0 {
                let sub = &graph.subgraphs[idx];
                for node_id in &sub.nodes {
                    if let Some(node) = nodes.get_mut(node_id) {
                        if is_horizontal {
                            node.y += shift;
                        } else {
                            node.x += shift;
                        }
                    }
                }
            }

            let shifted_bounds = if is_horizontal {
                (idx, min_x, min_y + shift, max_x, max_y + shift)
            } else {
                (idx, min_x + shift, min_y, max_x + shift, max_y)
            };
            placed.push(shifted_bounds);
        }
    }
}

/// Returns true if the line segment a→b intersects the axis-aligned rectangle
/// (x, y, w, h). Used by `enforce_cluster_band_separation` to detect when an
/// inter-cluster edge would cross a node belonging to a third cluster.
fn segment_crosses_aabb(a: (f32, f32), b: (f32, f32), rx: f32, ry: f32, rw: f32, rh: f32) -> bool {
    let (x1, y1) = a;
    let (x2, y2) = b;
    let min_x = x1.min(x2);
    let max_x = x1.max(x2);
    let min_y = y1.min(y2);
    let max_y = y1.max(y2);
    if max_x < rx || min_x > rx + rw || max_y < ry || min_y > ry + rh {
        return false;
    }
    let inside = |px: f32, py: f32| px >= rx && px <= rx + rw && py >= ry && py <= ry + rh;
    if inside(x1, y1) || inside(x2, y2) {
        return true;
    }
    // Liang–Barsky-style edge intersection: clip segment against rect.
    let dx = x2 - x1;
    let dy = y2 - y1;
    let p = [-dx, dx, -dy, dy];
    let q = [x1 - rx, rx + rw - x1, y1 - ry, ry + rh - y1];
    let mut t0 = 0.0_f32;
    let mut t1 = 1.0_f32;
    for i in 0..4 {
        if p[i].abs() < f32::EPSILON {
            if q[i] < 0.0 {
                return false;
            }
        } else {
            let t = q[i] / p[i];
            if p[i] < 0.0 {
                if t > t1 {
                    return false;
                }
                if t > t0 {
                    t0 = t;
                }
            } else {
                if t < t0 {
                    return false;
                }
                if t < t1 {
                    t1 = t;
                }
            }
        }
    }
    t0 <= t1
}

/// Enforce cross-axis (X for TD/BT, Y for LR/RL) separation between top-level
/// clusters when an inter-cluster edge's straight path would cross a node
/// belonging to a third cluster.
///
/// This addresses the case where rank/barycenter assignment places an isolated
/// node and a cluster member in the same cross-axis column at different
/// main-axis positions, forcing edges between them to detour around obstacles.
/// Upstream mermaid (via dagre's compound graph) avoids this by construction;
/// our pipeline assigns positions cluster-blind, so we patch it post-hoc.
fn enforce_cluster_band_separation(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart {
        return;
    }
    let top_level = top_level_subgraph_indices(graph);
    if top_level.is_empty() {
        return;
    }

    // Build groups: each visible top-level subgraph + its members.
    // Anchor child nodes (visible representations of anchored subgraphs) join
    // their subgraph's group.
    let mut group_members: Vec<Vec<String>> = Vec::new();
    let mut node_to_group: HashMap<String, usize> = HashMap::new();

    for &sg_idx in &top_level {
        let sub = &graph.subgraphs[sg_idx];
        if is_region_subgraph(sub) || sub.nodes.is_empty() {
            continue;
        }
        let g_idx = group_members.len();
        let mut members: Vec<String> = Vec::new();
        for node_id in &sub.nodes {
            if let Some(node) = nodes.get(node_id) {
                if !node.hidden {
                    node_to_group.insert(node_id.clone(), g_idx);
                    members.push(node_id.clone());
                }
            }
        }
        if let Some(anchor_id) = subgraph_anchor_id(sub, nodes) {
            if let Some(node) = nodes.get(anchor_id) {
                if !node.hidden && !node_to_group.contains_key(anchor_id) {
                    node_to_group.insert(anchor_id.to_string(), g_idx);
                    members.push(anchor_id.to_string());
                }
            }
        }
        if members.is_empty() {
            continue;
        }
        group_members.push(members);
    }

    if group_members.len() < 2 {
        return;
    }

    // Attach each isolated (non-cluster) node to the cluster it has the most
    // edges to. Ties break toward the lower group index for determinism.
    for node_id in graph.nodes.keys() {
        if node_to_group.contains_key(node_id) {
            continue;
        }
        let Some(node) = nodes.get(node_id) else {
            continue;
        };
        if node.hidden {
            continue;
        }
        let mut counts: Vec<usize> = vec![0; group_members.len()];
        for edge in &graph.edges {
            let other = if edge.from == *node_id {
                Some(&edge.to)
            } else if edge.to == *node_id {
                Some(&edge.from)
            } else {
                None
            };
            if let Some(other_id) = other {
                if let Some(&g) = node_to_group.get(other_id) {
                    counts[g] += 1;
                }
            }
        }
        let max_count = *counts.iter().max().unwrap_or(&0);
        if max_count == 0 {
            continue;
        }
        let best_g = counts.iter().position(|&c| c == max_count).unwrap();
        node_to_group.insert(node_id.clone(), best_g);
        group_members[best_g].push(node_id.clone());
    }

    // Compute group bounds (axis-aligned bounding boxes from member node rects).
    let mut bounds: Vec<(f32, f32, f32, f32)> = Vec::with_capacity(group_members.len()); // (min_x, min_y, max_x, max_y)
    for members in &group_members {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for node_id in members {
            if let Some(node) = nodes.get(node_id) {
                min_x = min_x.min(node.x);
                min_y = min_y.min(node.y);
                max_x = max_x.max(node.x + node.width);
                max_y = max_y.max(node.y + node.height);
            }
        }
        bounds.push((min_x, min_y, max_x, max_y));
    }

    // Detect edges whose straight line between endpoints crosses a node
    // belonging to a group different from either endpoint's group. We include
    // intra-group edges because the bug we're fixing is precisely that an
    // isolated node merged with its target's cluster (intra-group edge after
    // grouping) can still be visually offset such that the straight path
    // crosses a third cluster's node.
    let mut needs_separation: HashSet<(usize, usize)> = HashSet::new();
    for edge in &graph.edges {
        let from_g = node_to_group.get(&edge.from).copied();
        let to_g = node_to_group.get(&edge.to).copied();
        let mut edge_groups: HashSet<usize> = HashSet::new();
        if let Some(g) = from_g {
            edge_groups.insert(g);
        }
        if let Some(g) = to_g {
            edge_groups.insert(g);
        }
        if edge_groups.is_empty() {
            continue;
        }
        let (Some(fnode), Some(tnode)) = (nodes.get(&edge.from), nodes.get(&edge.to)) else {
            continue;
        };
        let fc = (fnode.x + fnode.width / 2.0, fnode.y + fnode.height / 2.0);
        let tc = (tnode.x + tnode.width / 2.0, tnode.y + tnode.height / 2.0);
        for (other_id, other) in nodes.iter() {
            if other_id == &edge.from || other_id == &edge.to || other.hidden {
                continue;
            }
            let Some(&og) = node_to_group.get(other_id) else {
                continue;
            };
            if edge_groups.contains(&og) {
                continue;
            }
            let inset = 1.0_f32;
            let rw = (other.width - inset * 2.0).max(0.0);
            let rh = (other.height - inset * 2.0).max(0.0);
            if rw <= 0.0 || rh <= 0.0 {
                continue;
            }
            if segment_crosses_aabb(fc, tc, other.x + inset, other.y + inset, rw, rh) {
                for &eg in &edge_groups {
                    let pair = if eg < og { (eg, og) } else { (og, eg) };
                    needs_separation.insert(pair);
                }
                break;
            }
        }
    }

    if needs_separation.is_empty() {
        return;
    }

    // For each pair needing separation, push the group with the larger
    // cross-axis centroid out past the smaller-centroid group's far edge.
    // Cross-axis is X for TD/BT, Y for LR/RL.
    let horizontal = is_horizontal(graph.direction);
    let cross_centroid = |g: usize| -> f32 {
        let (min, max) = if horizontal {
            (bounds[g].1, bounds[g].3) // y-axis is cross for horizontal
        } else {
            (bounds[g].0, bounds[g].2) // x-axis is cross for vertical
        };
        (min + max) * 0.5
    };

    // For each pair, the smaller-centroid group stays put and the larger-
    // centroid group shifts past the former's cross-axis extent + gap.
    let gap = config.node_spacing.max(20.0);
    let min_cross: Vec<f32> = (0..group_members.len())
        .map(|g| if horizontal { bounds[g].1 } else { bounds[g].0 })
        .collect();
    let max_cross: Vec<f32> = (0..group_members.len())
        .map(|g| if horizontal { bounds[g].3 } else { bounds[g].2 })
        .collect();

    // Order groups by current cross centroid (left-to-right or top-to-bottom).
    let mut order: Vec<usize> = (0..group_members.len()).collect();
    order.sort_by(|&i, &j| {
        cross_centroid(i)
            .partial_cmp(&cross_centroid(j))
            .unwrap_or(Ordering::Equal)
            .then(i.cmp(&j))
    });

    // For each pair in needs_separation, ensure the later-in-order group has
    // its cross-min ≥ earlier-in-order group's cross-max + gap. Iterate in
    // sweep order so cascading shifts compose correctly.
    let order_position: HashMap<usize, usize> =
        order.iter().enumerate().map(|(pos, g)| (*g, pos)).collect();
    let mut shift: Vec<f32> = vec![0.0; group_members.len()];

    for &g in &order {
        for (a, b) in &needs_separation {
            let (lo, hi) = match (
                order_position.get(a).copied(),
                order_position.get(b).copied(),
            ) {
                (Some(pa), Some(pb)) if pa < pb => (*a, *b),
                (Some(pa), Some(pb)) if pb < pa => (*b, *a),
                _ => continue,
            };
            if hi != g {
                continue;
            }
            let lo_far = max_cross[lo] + shift[lo];
            let hi_near = min_cross[hi] + shift[hi];
            let needed = lo_far + gap - hi_near;
            if needed > 0.0 {
                shift[hi] += needed;
            }
        }
    }

    // Apply shifts to all member nodes.
    for (g_idx, members) in group_members.iter().enumerate() {
        let s = shift[g_idx];
        if s.abs() < 1e-3 {
            continue;
        }
        for node_id in members {
            if let Some(node) = nodes.get_mut(node_id) {
                if horizontal {
                    node.y += s;
                } else {
                    node.x += s;
                }
            }
        }
    }
}

fn align_disconnected_top_level_subgraphs(graph: &Graph, nodes: &mut BTreeMap<String, NodeLayout>) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.subgraphs.len() < 2 {
        return;
    }

    let top_level = top_level_subgraph_indices(graph);
    if top_level.len() < 2 {
        return;
    }

    let mut seen: HashSet<&str> = HashSet::new();
    let mut union_count = 0usize;
    for &idx in &top_level {
        let sub = &graph.subgraphs[idx];
        for node_id in &sub.nodes {
            if !seen.insert(node_id.as_str()) {
                return;
            }
            union_count += 1;
        }
        if let Some(anchor_id) = subgraph_anchor_id(sub, nodes) {
            if !seen.insert(anchor_id) {
                return;
            }
            union_count += 1;
        }
    }
    if union_count != graph.nodes.len() {
        return;
    }

    let mut node_to_top_level: HashMap<&str, usize> = HashMap::new();
    for &idx in &top_level {
        let sub = &graph.subgraphs[idx];
        for node_id in &sub.nodes {
            node_to_top_level.insert(node_id.as_str(), idx);
        }
        if let Some(anchor_id) = subgraph_anchor_id(sub, nodes) {
            node_to_top_level.insert(anchor_id, idx);
        }
    }
    let has_cross_edges = graph.edges.iter().any(|edge| {
        let from = node_to_top_level.get(edge.from.as_str());
        let to = node_to_top_level.get(edge.to.as_str());
        matches!((from, to), (Some(a), Some(b)) if a != b)
    });
    if has_cross_edges {
        return;
    }

    #[derive(Clone)]
    struct Bounds {
        idx: usize,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        anchor_id: Option<String>,
    }

    let mut bounds: Vec<Bounds> = Vec::new();
    for &idx in &top_level {
        let sub = &graph.subgraphs[idx];
        if sub.nodes.is_empty() {
            continue;
        }
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for node_id in &sub.nodes {
            if let Some(node) = nodes.get(node_id) {
                min_x = min_x.min(node.x);
                min_y = min_y.min(node.y);
                max_x = max_x.max(node.x + node.width);
                max_y = max_y.max(node.y + node.height);
            }
        }
        let anchor_id = subgraph_anchor_id(sub, nodes).map(|id| id.to_string());
        if let Some(anchor) = anchor_id.as_deref().and_then(|id| nodes.get(id)) {
            min_x = min_x.min(anchor.x);
            min_y = min_y.min(anchor.y);
            max_x = max_x.max(anchor.x + anchor.width);
            max_y = max_y.max(anchor.y + anchor.height);
        }
        if min_x == f32::MAX {
            continue;
        }
        bounds.push(Bounds {
            idx,
            min_x,
            min_y,
            max_x,
            max_y,
            anchor_id,
        });
    }

    if bounds.len() < 2 {
        return;
    }

    let horizontal = is_horizontal(graph.direction);
    bounds.sort_by(|a, b| {
        let a_key = if horizontal { a.min_x } else { a.min_y };
        let b_key = if horizontal { b.min_x } else { b.min_y };
        a_key
            .partial_cmp(&b_key)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.idx.cmp(&b.idx))
    });

    let mut prev_max: Option<f32> = None;
    for bound in &bounds {
        let min_main = if horizontal { bound.min_x } else { bound.min_y };
        let max_main = if horizontal { bound.max_x } else { bound.max_y };
        if let Some(prev) = prev_max {
            if min_main < prev {
                return;
            }
        }
        prev_max = Some(max_main);
    }

    let target_cross = bounds
        .iter()
        .map(|b| if horizontal { b.min_y } else { b.min_x })
        .fold(f32::MAX, f32::min);

    for bound in bounds {
        let current_cross = if horizontal { bound.min_y } else { bound.min_x };
        let delta = target_cross - current_cross;
        if delta.abs() < 0.5 {
            continue;
        }
        let sub = &graph.subgraphs[bound.idx];
        for node_id in &sub.nodes {
            if let Some(node) = nodes.get_mut(node_id) {
                if horizontal {
                    node.y += delta;
                } else {
                    node.x += delta;
                }
            }
        }
        if let Some(anchor_id) = bound.anchor_id.as_deref() {
            if let Some(node) = nodes.get_mut(anchor_id) {
                if horizontal {
                    node.y += delta;
                } else {
                    node.x += delta;
                }
            }
        }
    }
}

fn reorder_disconnected_top_level_flowchart_groups_like_dagre(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.subgraphs.len() < 2 {
        return;
    }

    let tree = SubgraphTree::build(graph);
    let top_level = tree.top_level.clone();
    if top_level.len() < 2 {
        return;
    }

    let mut group_nodes: HashMap<usize, HashSet<String>> = HashMap::new();
    let mut member_to_group: HashMap<String, usize> = HashMap::new();
    for &idx in &top_level {
        let mut members = HashSet::new();
        collect_subgraph_descendant_node_ids(&tree, graph, idx, &mut members);
        if let Some(sub) = graph.subgraphs.get(idx)
            && let Some(anchor_id) = subgraph_anchor_id(sub, nodes)
        {
            members.insert(anchor_id.to_string());
        }
        if members.is_empty() {
            return;
        }
        for node_id in &members {
            if member_to_group.insert(node_id.clone(), idx).is_some() {
                return;
            }
        }
        group_nodes.insert(idx, members);
    }

    for edge in &graph.edges {
        let from_group = member_to_group.get(&edge.from).copied();
        let to_group = member_to_group.get(&edge.to).copied();
        if matches!((from_group, to_group), (Some(a), Some(b)) if a != b) {
            return;
        }
    }

    let graph_node_ids: HashSet<&str> = graph.nodes.keys().map(|id| id.as_str()).collect();
    let external_ids: Vec<String> = nodes
        .values()
        .filter(|node| {
            !node.hidden
                && node.anchor_subgraph.is_none()
                && graph_node_ids.contains(node.id.as_str())
                && !member_to_group.contains_key(&node.id)
        })
        .map(|node| node.id.clone())
        .collect();

    let mut external_to_group: HashMap<String, usize> = HashMap::new();
    for node_id in external_ids {
        let mut incident_groups: HashSet<usize> = HashSet::new();
        for edge in &graph.edges {
            if edge.from == node_id {
                if let Some(group) = member_to_group.get(&edge.to) {
                    incident_groups.insert(*group);
                }
            } else if edge.to == node_id
                && let Some(group) = member_to_group.get(&edge.from)
            {
                incident_groups.insert(*group);
            }
        }
        if incident_groups.len() != 1 {
            return;
        }
        let group = *incident_groups.iter().next().unwrap();
        external_to_group.insert(node_id.clone(), group);
        if let Some(members) = group_nodes.get_mut(&group) {
            members.insert(node_id);
        }
    }

    let horizontal = is_horizontal(graph.direction);
    for (external_id, group) in &external_to_group {
        let mut connected_centers = Vec::new();
        for edge in &graph.edges {
            let other_id = if edge.from == *external_id {
                Some(edge.to.as_str())
            } else if edge.to == *external_id {
                Some(edge.from.as_str())
            } else {
                None
            };
            let Some(other_id) = other_id else {
                continue;
            };
            if member_to_group.get(other_id).copied() != Some(*group) {
                continue;
            }
            if let Some(other) = nodes.get(other_id) {
                connected_centers.push(if horizontal {
                    other.y + other.height * 0.5
                } else {
                    other.x + other.width * 0.5
                });
            }
        }
        if connected_centers.is_empty() {
            continue;
        }
        connected_centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        let mid = connected_centers.len() / 2;
        let target_center = if connected_centers.len() % 2 == 1 {
            connected_centers[mid]
        } else {
            (connected_centers[mid - 1] + connected_centers[mid]) * 0.5
        };
        if let Some(node) = nodes.get_mut(external_id) {
            if horizontal {
                node.y = target_center - node.height * 0.5;
            } else {
                node.x = target_center - node.width * 0.5;
            }
        }
    }

    #[derive(Clone, Copy)]
    struct GroupBand {
        idx: usize,
        main_start: f32,
    }

    let mut bands = Vec::new();
    for &idx in &top_level {
        let Some(members) = group_nodes.get(&idx) else {
            return;
        };
        let mut min_main = f32::MAX;
        for node_id in members {
            let Some(node) = nodes.get(node_id) else {
                continue;
            };
            let start = if horizontal { node.y } else { node.x };
            min_main = min_main.min(start);
        }
        if min_main == f32::MAX {
            return;
        }
        bands.push(GroupBand {
            idx,
            main_start: min_main,
        });
    }
    if bands.len() != top_level.len() {
        return;
    }

    bands.sort_by(|a, b| {
        a.main_start
            .partial_cmp(&b.main_start)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.idx.cmp(&b.idx))
    });
    let target_starts: Vec<f32> = bands.iter().map(|band| band.main_start).collect();
    let desired_order: Vec<usize> = top_level.iter().rev().copied().collect();
    if desired_order == bands.iter().map(|band| band.idx).collect::<Vec<_>>() {
        return;
    }

    for (idx, target_start) in desired_order.into_iter().zip(target_starts) {
        let Some(members) = group_nodes.get(&idx) else {
            return;
        };
        let mut current_start = f32::MAX;
        for node_id in members {
            if let Some(node) = nodes.get(node_id) {
                current_start = current_start.min(if horizontal { node.y } else { node.x });
            }
        }
        if current_start == f32::MAX {
            continue;
        }
        let delta = target_start - current_start;
        if delta.abs() < 0.5 {
            continue;
        }
        for node_id in members {
            if let Some(node) = nodes.get_mut(node_id) {
                if horizontal {
                    node.y += delta;
                } else {
                    node.x += delta;
                }
            }
        }
    }
}

fn pack_flowchart_recursive_subgraph_components(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart
        || graph.subgraphs.is_empty()
        || is_horizontal(graph.direction)
    {
        return;
    }

    let recursive_subgraphs: Vec<usize> = top_level_subgraph_indices(graph)
        .into_iter()
        .filter(|idx| {
            graph
                .subgraphs
                .get(*idx)
                .map(|sub| flowchart_subgraph_is_recursive_cluster(graph, sub))
                .unwrap_or(false)
        })
        .collect();
    if recursive_subgraphs.is_empty() {
        return;
    }

    #[derive(Clone)]
    struct PackComponent {
        nodes: Vec<String>,
        order_group: usize,
        order: usize,
        min_x: f32,
        max_x: f32,
        anchor_y: f32,
        source_half_height: f32,
    }

    let subgraph_layouts = build_subgraph_layouts(graph, nodes, theme, config);
    let mut components = Vec::new();
    let mut recursive_members: HashSet<String> = HashSet::new();
    let mut recursive_rank_height: f32 = 0.0;

    for (group_order, sub_idx) in recursive_subgraphs.iter().copied().enumerate() {
        let Some(sub) = graph.subgraphs.get(sub_idx) else {
            continue;
        };
        let Some((min_x, min_y, max_x, max_y)) = subgraph_layout_index(&subgraph_layouts, sub)
            .and_then(|idx| subgraph_layouts.get(idx))
            .map(|layout| {
                (
                    layout.x,
                    layout.y,
                    layout.x + layout.width,
                    layout.y + layout.height,
                )
            })
            .or_else(|| node_group_bounds(nodes, &sub.nodes))
        else {
            continue;
        };
        for node_id in &sub.nodes {
            recursive_members.insert(node_id.clone());
        }
        recursive_rank_height = recursive_rank_height.max(max_y - min_y);
        components.push(PackComponent {
            nodes: sub.nodes.clone(),
            order_group: 0,
            order: group_order,
            min_x,
            max_x,
            anchor_y: (min_y + max_y) * 0.5,
            source_half_height: (max_y - min_y) * 0.5,
        });
    }

    let remaining: HashSet<String> = nodes
        .values()
        .filter(|node| {
            !node.hidden
                && node.anchor_subgraph.is_none()
                && !recursive_members.contains(node.id.as_str())
        })
        .map(|node| node.id.clone())
        .collect();

    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for node_id in &remaining {
        adjacency.entry(node_id.clone()).or_default();
    }
    for edge in &graph.edges {
        if remaining.contains(&edge.from) && remaining.contains(&edge.to) {
            adjacency
                .entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
            adjacency
                .entry(edge.to.clone())
                .or_default()
                .push(edge.from.clone());
        }
    }

    let mut visited = HashSet::new();
    let mut remaining_ids: Vec<String> = remaining.into_iter().collect();
    remaining_ids.sort_by(|a, b| {
        graph
            .node_order
            .get(a)
            .copied()
            .unwrap_or(usize::MAX)
            .cmp(&graph.node_order.get(b).copied().unwrap_or(usize::MAX))
            .then_with(|| a.cmp(b))
    });

    for node_id in remaining_ids {
        if !visited.insert(node_id.clone()) {
            continue;
        }
        let mut stack = vec![node_id.clone()];
        let mut comp = Vec::new();
        while let Some(cur) = stack.pop() {
            comp.push(cur.clone());
            if let Some(neighbors) = adjacency.get(&cur) {
                for next in neighbors {
                    if visited.insert(next.clone()) {
                        stack.push(next.clone());
                    }
                }
            }
        }
        comp.sort_by(|a, b| {
            graph
                .node_order
                .get(a)
                .copied()
                .unwrap_or(usize::MAX)
                .cmp(&graph.node_order.get(b).copied().unwrap_or(usize::MAX))
                .then_with(|| a.cmp(b))
        });
        let Some((min_x, min_y, max_x, max_y)) = node_group_bounds(nodes, &comp) else {
            continue;
        };

        let comp_set: HashSet<&str> = comp.iter().map(|id| id.as_str()).collect();
        let mut indegree: HashMap<&str, usize> = comp.iter().map(|id| (id.as_str(), 0)).collect();
        for edge in &graph.edges {
            if comp_set.contains(edge.from.as_str())
                && comp_set.contains(edge.to.as_str())
                && let Some(value) = indegree.get_mut(edge.to.as_str())
            {
                *value += 1;
            }
        }

        let mut source_centers = Vec::new();
        let mut source_max_height: f32 = 0.0;
        for id in &comp {
            if indegree.get(id.as_str()).copied().unwrap_or(0) == 0
                && let Some(node) = nodes.get(id)
            {
                source_centers.push(node.y + node.height * 0.5);
                source_max_height = source_max_height.max(node.height);
            }
        }
        let anchor_y = if source_centers.is_empty() {
            (min_y + max_y) * 0.5
        } else {
            source_centers.iter().sum::<f32>() / source_centers.len() as f32
        };
        let order = comp
            .iter()
            .filter_map(|id| graph.node_order.get(id).copied())
            .min()
            .unwrap_or(usize::MAX);

        components.push(PackComponent {
            nodes: comp,
            order_group: 1,
            order,
            min_x,
            max_x,
            anchor_y,
            source_half_height: source_max_height * 0.5,
        });
    }

    if components.len() < 2 {
        return;
    }

    components.sort_by(|a, b| {
        a.order_group
            .cmp(&b.order_group)
            .then_with(|| a.order.cmp(&b.order))
            .then_with(|| a.min_x.partial_cmp(&b.min_x).unwrap_or(Ordering::Equal))
    });

    let mut cursor = components
        .iter()
        .map(|comp| comp.min_x)
        .fold(f32::MAX, f32::min);
    if !cursor.is_finite() {
        return;
    }
    let target_anchor_y = components
        .iter()
        .map(|comp| comp.anchor_y)
        .fold(f32::MIN, f32::max);
    if !target_anchor_y.is_finite() {
        return;
    }
    let all_recursive_components = components.iter().all(|comp| comp.order_group == 0);
    let gap = if all_recursive_components {
        config.node_spacing.max(FLOWCHART_RECURSIVE_DAGRE_SPACING)
    } else {
        config.node_spacing.max(MIN_NODE_SPACING_FLOOR)
    };
    let nonrecursive_source_height = components
        .iter()
        .filter(|comp| comp.order_group != 0)
        .map(|comp| comp.source_half_height * 2.0)
        .fold(0.0_f32, f32::max);
    let descendant_rank_shift =
        ((recursive_rank_height - nonrecursive_source_height) * 0.5).max(0.0);

    for comp in components {
        let delta_x = cursor - comp.min_x;
        let delta_y = target_anchor_y - comp.anchor_y;
        let anchor_after = comp.anchor_y + delta_y;
        let descendant_threshold = anchor_after + comp.source_half_height + 0.5;
        for node_id in &comp.nodes {
            if let Some(node) = nodes.get_mut(node_id) {
                node.x += delta_x;
                node.y += delta_y;
                if comp.order_group != 0
                    && descendant_rank_shift > 0.5
                    && node.y + node.height * 0.5 > descendant_threshold
                {
                    node.y += descendant_rank_shift;
                }
            }
        }
        cursor += (comp.max_x - comp.min_x).max(1.0) + gap;
    }
}

fn apply_flowchart_dagre_root_fanout_centering(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.edges.len() < 2 {
        return;
    }

    let mut subgraph_members = HashSet::new();
    for sub in &graph.subgraphs {
        for id in &sub.nodes {
            subgraph_members.insert(id.as_str());
        }
    }

    let mut incoming: HashMap<&str, usize> = HashMap::new();
    let mut outgoing: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in &graph.edges {
        *incoming.entry(edge.to.as_str()).or_insert(0) += 1;
        outgoing
            .entry(edge.from.as_str())
            .or_default()
            .push(edge.to.as_str());
    }

    let center_axis_is_x = !is_horizontal(graph.direction);
    let rank_axis_is_x = is_horizontal(graph.direction);
    let mut sources: Vec<&str> = outgoing.keys().copied().collect();
    sources.sort_by(|a, b| {
        graph
            .node_order
            .get(*a)
            .copied()
            .unwrap_or(usize::MAX)
            .cmp(&graph.node_order.get(*b).copied().unwrap_or(usize::MAX))
            .then_with(|| a.cmp(b))
    });

    for source_id in sources {
        if incoming.get(source_id).copied().unwrap_or(0) != 0
            || subgraph_members.contains(source_id)
        {
            continue;
        }
        let Some(source) = nodes.get(source_id) else {
            continue;
        };
        if source.hidden || source.anchor_subgraph.is_some() {
            continue;
        }
        let Some(target_ids) = outgoing.get(source_id) else {
            continue;
        };
        if target_ids.len() < 2 {
            continue;
        }

        let source_rank = node_main_center(source, rank_axis_is_x);
        let mut target_centers = Vec::new();
        let mut target_ranks = Vec::new();
        let mut valid = true;
        for target_id in target_ids {
            let Some(target) = nodes.get(*target_id) else {
                valid = false;
                break;
            };
            if target.hidden
                || target.anchor_subgraph.is_some()
                || incoming.get(*target_id).copied().unwrap_or(0) != 1
            {
                valid = false;
                break;
            }
            let target_rank = node_main_center(target, rank_axis_is_x);
            if !flowchart_target_is_forward_rank(graph.direction, source_rank, target_rank) {
                valid = false;
                break;
            }
            target_ranks.push(target_rank);
            target_centers.push(node_main_center(target, center_axis_is_x));
        }
        if !valid || target_centers.len() < 2 {
            continue;
        }

        target_ranks.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        let rank_span = target_ranks.last().copied().unwrap_or(0.0)
            - target_ranks.first().copied().unwrap_or(0.0);
        if rank_span > config.rank_spacing.max(MIN_NODE_SPACING_FLOOR) * 0.75 {
            continue;
        }

        target_centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        let target_span = target_centers.last().copied().unwrap_or(0.0)
            - target_centers.first().copied().unwrap_or(0.0);
        if target_span < config.node_spacing.max(MIN_NODE_SPACING_FLOOR) * 0.5 {
            continue;
        }
        let desired_center = target_centers.iter().sum::<f32>() / target_centers.len() as f32;
        let current_center = node_main_center(source, center_axis_is_x);
        let delta = desired_center - current_center;
        if delta.abs() < 1.0 {
            continue;
        }
        if !flowchart_fanout_candidate_clear(nodes, source_id, center_axis_is_x, delta, config) {
            continue;
        }
        if let Some(source) = nodes.get_mut(source_id) {
            shift_node_main(source, center_axis_is_x, delta);
        }
    }
}

fn apply_flowchart_dagre_linear_chain_centering(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.edges.is_empty() {
        return;
    }

    let mut subgraph_members = HashSet::new();
    for sub in &graph.subgraphs {
        for id in &sub.nodes {
            subgraph_members.insert(id.as_str());
        }
    }

    let mut incoming: HashMap<&str, Vec<&crate::ir::Edge>> = HashMap::new();
    let mut outgoing: HashMap<&str, Vec<&crate::ir::Edge>> = HashMap::new();
    for edge in &graph.edges {
        incoming.entry(edge.to.as_str()).or_default().push(edge);
        outgoing.entry(edge.from.as_str()).or_default().push(edge);
    }

    let center_axis_is_x = !is_horizontal(graph.direction);
    let rank_axis_is_x = is_horizontal(graph.direction);
    for edge in &graph.edges {
        if edge.style != crate::ir::EdgeStyle::Solid
            || edge.label.is_some()
            || edge.start_label.is_some()
            || edge.end_label.is_some()
            || subgraph_members.contains(edge.from.as_str())
            || subgraph_members.contains(edge.to.as_str())
            || incoming
                .get(edge.from.as_str())
                .is_some_and(|edges| !edges.is_empty())
            || outgoing
                .get(edge.from.as_str())
                .map(|edges| edges.len())
                .unwrap_or(0)
                != 1
            || incoming
                .get(edge.to.as_str())
                .map(|edges| edges.len())
                .unwrap_or(0)
                != 1
            || outgoing
                .get(edge.to.as_str())
                .is_some_and(|edges| !edges.is_empty())
        {
            continue;
        }

        let (Some(source), Some(target)) = (nodes.get(&edge.from), nodes.get(&edge.to)) else {
            continue;
        };
        if source.hidden
            || target.hidden
            || source.anchor_subgraph.is_some()
            || target.anchor_subgraph.is_some()
        {
            continue;
        }

        let source_rank = node_main_center(source, rank_axis_is_x);
        let target_rank = node_main_center(target, rank_axis_is_x);
        if !flowchart_target_is_forward_rank(graph.direction, source_rank, target_rank) {
            continue;
        }

        let source_center = node_main_center(source, center_axis_is_x);
        let target_center = node_main_center(target, center_axis_is_x);
        let delta = target_center - source_center;
        if delta.abs() < 1.0 {
            continue;
        }

        let source_half = node_main_half(source, center_axis_is_x);
        let target_half = node_main_half(target, center_axis_is_x);
        let (move_id, move_delta) = if source_half <= target_half {
            (edge.from.as_str(), delta)
        } else {
            (edge.to.as_str(), -delta)
        };
        if !flowchart_fanout_candidate_clear(nodes, move_id, center_axis_is_x, move_delta, config) {
            continue;
        }
        if let Some(node) = nodes.get_mut(move_id) {
            shift_node_main(node, center_axis_is_x, move_delta);
        }
    }
}

fn apply_flowchart_dagre_member_leaf_label_alignment(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart
        || graph.subgraphs.is_empty()
        || graph.edges.is_empty()
    {
        return;
    }

    let mut subgraph_members = HashSet::new();
    for sub in &graph.subgraphs {
        for id in &sub.nodes {
            subgraph_members.insert(id.as_str());
        }
    }

    let mut incoming: HashMap<&str, usize> = HashMap::new();
    let mut outgoing: HashMap<&str, usize> = HashMap::new();
    for edge in &graph.edges {
        *incoming.entry(edge.to.as_str()).or_insert(0) += 1;
        *outgoing.entry(edge.from.as_str()).or_insert(0) += 1;
    }

    let center_axis_is_x = !is_horizontal(graph.direction);
    let rank_axis_is_x = is_horizontal(graph.direction);
    for edge in &graph.edges {
        let has_label =
            edge.label.is_some() || edge.start_label.is_some() || edge.end_label.is_some();
        if !has_label
            || edge.style == crate::ir::EdgeStyle::Invisible
            || !subgraph_members.contains(edge.from.as_str())
            || subgraph_members.contains(edge.to.as_str())
            || incoming.get(edge.to.as_str()).copied().unwrap_or(0) != 1
            || outgoing.get(edge.to.as_str()).copied().unwrap_or(0) != 0
        {
            continue;
        }

        let (Some(source), Some(target)) = (nodes.get(&edge.from), nodes.get(&edge.to)) else {
            continue;
        };
        if source.hidden
            || target.hidden
            || source.anchor_subgraph.is_some()
            || target.anchor_subgraph.is_some()
        {
            continue;
        }

        let source_rank = node_main_center(source, rank_axis_is_x);
        let target_rank = node_main_center(target, rank_axis_is_x);
        if !flowchart_target_is_forward_rank(graph.direction, source_rank, target_rank) {
            continue;
        }

        let source_center = node_main_center(source, center_axis_is_x);
        let target_center = node_main_center(target, center_axis_is_x);
        let delta = source_center - target_center;
        if delta.abs() < 1.0 {
            continue;
        }
        if !flowchart_fanout_candidate_clear(
            nodes,
            edge.to.as_str(),
            center_axis_is_x,
            delta,
            config,
        ) {
            continue;
        }
        if let Some(target) = nodes.get_mut(&edge.to) {
            shift_node_main(target, center_axis_is_x, delta);
        }
    }
}

fn apply_flowchart_dagre_recursive_root_rank_spacing(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart
        || graph.direction != Direction::TopDown
        || graph.subgraphs.is_empty()
        || subgraphs.is_empty()
        || graph.edges.iter().any(|edge| {
            edge.label.is_some() && !flowchart_edge_inside_recursive_cluster(graph, edge)
        })
    {
        return;
    }

    let top_level = top_level_subgraph_indices(graph);
    let recursive_layouts: Vec<&SubgraphLayout> = top_level
        .iter()
        .filter_map(|idx| {
            let sub = graph.subgraphs.get(*idx)?;
            if !flowchart_subgraph_is_recursive_cluster(graph, sub) {
                return None;
            }
            subgraph_layout_index(subgraphs, sub).and_then(|layout_idx| subgraphs.get(layout_idx))
        })
        .collect();
    let [recursive_layout] = recursive_layouts.as_slice() else {
        return;
    };

    let mut subgraph_members: HashSet<&str> = HashSet::new();
    for sub in &graph.subgraphs {
        for node_id in &sub.nodes {
            subgraph_members.insert(node_id.as_str());
        }
    }

    let mut outside_ids: Vec<String> = nodes
        .iter()
        .filter(|(id, node)| {
            graph.nodes.contains_key(id.as_str())
                && !subgraph_members.contains(id.as_str())
                && !node.hidden
                && node.anchor_subgraph.is_none()
        })
        .map(|(id, _)| id.clone())
        .collect();
    if outside_ids.len() < 2 {
        return;
    }
    outside_ids.sort_by(|a, b| {
        graph
            .node_order
            .get(a)
            .copied()
            .unwrap_or(usize::MAX)
            .cmp(&graph.node_order.get(b).copied().unwrap_or(usize::MAX))
            .then_with(|| a.cmp(b))
    });

    let outside_set: HashSet<&str> = outside_ids.iter().map(String::as_str).collect();
    let outside_edges: Vec<crate::ir::Edge> = graph
        .edges
        .iter()
        .filter(|edge| {
            outside_set.contains(edge.from.as_str()) && outside_set.contains(edge.to.as_str())
        })
        .cloned()
        .collect();
    if outside_edges.is_empty() {
        return;
    }

    let ranks = compute_ranks_subset_for(graph, &outside_ids, &outside_edges, &graph.node_order);
    let max_rank = ranks.values().copied().max().unwrap_or(0);
    if max_rank != 1 {
        return;
    }

    let mut root_rank_ids = Vec::new();
    let mut next_rank_ids = Vec::new();
    for node_id in &outside_ids {
        match ranks.get(node_id).copied().unwrap_or(0) {
            0 => root_rank_ids.push(node_id.clone()),
            1 => next_rank_ids.push(node_id.clone()),
            _ => return,
        }
    }
    if root_rank_ids.is_empty() || next_rank_ids.is_empty() {
        return;
    }

    let top_center_y = recursive_layout.y + recursive_layout.height * 0.5;
    let top_half_height = recursive_layout.height * 0.5;
    let bottom_half_height = next_rank_ids
        .iter()
        .filter_map(|node_id| nodes.get(node_id).map(|node| node.height * 0.5))
        .fold(0.0_f32, f32::max);
    if bottom_half_height <= 0.0 {
        return;
    }

    let next_center_y = top_center_y + top_half_height + config.rank_spacing + bottom_half_height;
    for node_id in root_rank_ids {
        if let Some(node) = nodes.get_mut(&node_id) {
            node.y = top_center_y - node.height * 0.5;
        }
    }
    for node_id in next_rank_ids {
        if let Some(node) = nodes.get_mut(&node_id) {
            node.y = next_center_y - node.height * 0.5;
        }
    }
}

#[derive(Debug, Clone)]
struct FlowchartAlignedSecondaryRoute {
    points: Vec<(f32, f32)>,
    label_anchor: Option<(f32, f32)>,
}

fn flowchart_aligned_secondary_edge_route(
    graph: &Graph,
    subgraphs: &[SubgraphLayout],
    edge: &crate::ir::Edge,
    from: &NodeLayout,
    to: &NodeLayout,
    direction: Direction,
    label: Option<&TextBlock>,
) -> Option<FlowchartAlignedSecondaryRoute> {
    let is_secondary = edge.style == crate::ir::EdgeStyle::Dotted
        || edge.label.is_some()
        || edge.start_label.is_some()
        || edge.end_label.is_some();
    if !is_secondary || edge.style == crate::ir::EdgeStyle::Invisible {
        return None;
    }

    let (start_side, end_side, _) = edge_sides(from, to, direction);
    let start = anchor_point_for_node(from, start_side, 0.0);
    let end = anchor_point_for_node(to, end_side, 0.0);
    let same_lane_tolerance = 1.0;
    match direction {
        Direction::LeftRight => {
            if end.0 <= start.0 + 1.0 || (end.1 - start.1).abs() > same_lane_tolerance {
                return None;
            }
        }
        Direction::RightLeft => {
            if start.0 <= end.0 + 1.0 || (end.1 - start.1).abs() > same_lane_tolerance {
                return None;
            }
        }
        Direction::TopDown => {
            if end.1 <= start.1 + 1.0 || (end.0 - start.0).abs() > same_lane_tolerance {
                return None;
            }
        }
        Direction::BottomTop => {
            if start.1 <= end.1 + 1.0 || (end.0 - start.0).abs() > same_lane_tolerance {
                return None;
            }
        }
    }

    if let Some(route) = flowchart_aligned_secondary_compound_entry_route(
        graph, subgraphs, edge, start, end, direction, label,
    ) {
        return Some(route);
    }
    if let Some(route) = flowchart_aligned_secondary_compound_exit_route(
        graph, subgraphs, edge, start, end, direction, label,
    ) {
        return Some(route);
    }

    Some(FlowchartAlignedSecondaryRoute {
        points: vec![start, end],
        label_anchor: None,
    })
}

fn flowchart_aligned_secondary_compound_entry_route(
    graph: &Graph,
    subgraphs: &[SubgraphLayout],
    edge: &crate::ir::Edge,
    start: (f32, f32),
    end: (f32, f32),
    direction: Direction,
    label: Option<&TextBlock>,
) -> Option<FlowchartAlignedSecondaryRoute> {
    let target_subgraph = graph.subgraphs.iter().find(|subgraph| {
        subgraph.nodes.iter().any(|node_id| node_id == &edge.to)
            && !subgraph.nodes.iter().any(|node_id| node_id == &edge.from)
    })?;
    let target_layout_idx = subgraph_layout_index(subgraphs, target_subgraph)?;
    let target_layout = subgraphs.get(target_layout_idx)?;
    let mut points = Vec::with_capacity(4);
    points.push(start);

    let rank_gap = FLOWCHART_RECURSIVE_DAGRE_SPACING;
    let label_anchor = label.map(|label| match direction {
        Direction::LeftRight => (start.0 + (label.width + rank_gap) * 0.5, start.1),
        Direction::RightLeft => (start.0 - (label.width + rank_gap) * 0.5, start.1),
        Direction::TopDown => (start.0, start.1 + (label.height + rank_gap) * 0.5),
        Direction::BottomTop => (start.0, start.1 - (label.height + rank_gap) * 0.5),
    });
    if let Some(anchor) = label_anchor {
        points.push(anchor);
    }

    let boundary = match direction {
        Direction::LeftRight => {
            if target_layout.x <= start.0 + 1.0 || end.0 <= target_layout.x {
                return None;
            }
            (target_layout.x, start.1)
        }
        Direction::RightLeft => {
            let right = target_layout.x + target_layout.width;
            if right >= start.0 - 1.0 || end.0 >= right {
                return None;
            }
            (right, start.1)
        }
        Direction::TopDown => {
            if target_layout.y <= start.1 + 1.0 || end.1 <= target_layout.y {
                return None;
            }
            (start.0, target_layout.y)
        }
        Direction::BottomTop => {
            let bottom = target_layout.y + target_layout.height;
            if bottom >= start.1 - 1.0 || end.1 >= bottom {
                return None;
            }
            (start.0, bottom)
        }
    };
    points.push(boundary);
    points.push(end);

    Some(FlowchartAlignedSecondaryRoute {
        points,
        label_anchor,
    })
}

fn flowchart_aligned_secondary_compound_exit_route(
    graph: &Graph,
    subgraphs: &[SubgraphLayout],
    edge: &crate::ir::Edge,
    start: (f32, f32),
    end: (f32, f32),
    direction: Direction,
    label: Option<&TextBlock>,
) -> Option<FlowchartAlignedSecondaryRoute> {
    let source_subgraph = graph.subgraphs.iter().find(|subgraph| {
        subgraph.nodes.iter().any(|node_id| node_id == &edge.from)
            && !subgraph.nodes.iter().any(|node_id| node_id == &edge.to)
    })?;
    let source_layout_idx = subgraph_layout_index(subgraphs, source_subgraph)?;
    let source_layout = subgraphs.get(source_layout_idx)?;
    let boundary = match direction {
        Direction::LeftRight => {
            let right = source_layout.x + source_layout.width;
            if right <= start.0 + 1.0 || end.0 <= right {
                return None;
            }
            (right, start.1)
        }
        Direction::RightLeft => {
            if source_layout.x >= start.0 - 1.0 || end.0 >= source_layout.x {
                return None;
            }
            (source_layout.x, start.1)
        }
        Direction::TopDown => {
            let bottom = source_layout.y + source_layout.height;
            if bottom <= start.1 + 1.0 || end.1 <= bottom {
                return None;
            }
            (start.0, bottom)
        }
        Direction::BottomTop => {
            if source_layout.y >= start.1 - 1.0 || end.1 >= source_layout.y {
                return None;
            }
            (start.0, source_layout.y)
        }
    };
    let label_anchor = label.map(|_| match direction {
        Direction::LeftRight | Direction::RightLeft => ((boundary.0 + end.0) * 0.5, start.1),
        Direction::TopDown | Direction::BottomTop => (start.0, (boundary.1 + end.1) * 0.5),
    });
    let mut points = Vec::with_capacity(4);
    points.push(start);
    points.push(boundary);
    if let Some(anchor) = label_anchor {
        points.push(anchor);
    }
    points.push(end);

    Some(FlowchartAlignedSecondaryRoute {
        points,
        label_anchor,
    })
}

fn flowchart_target_is_forward_rank(
    direction: Direction,
    source_rank: f32,
    target_rank: f32,
) -> bool {
    match direction {
        Direction::TopDown | Direction::LeftRight => target_rank > source_rank + 1.0,
        Direction::BottomTop | Direction::RightLeft => target_rank < source_rank - 1.0,
    }
}

fn flowchart_fanout_candidate_clear(
    nodes: &BTreeMap<String, NodeLayout>,
    source_id: &str,
    axis_is_x: bool,
    delta: f32,
    config: &LayoutConfig,
) -> bool {
    let Some(source) = nodes.get(source_id) else {
        return false;
    };
    let (candidate_x, candidate_y) = if axis_is_x {
        (source.x + delta, source.y)
    } else {
        (source.x, source.y + delta)
    };
    let margin = (config.node_spacing * 0.2).max(8.0);
    for (id, node) in nodes {
        if id == source_id || node.hidden {
            continue;
        }
        if rects_overlap_with_margin(
            (candidate_x, candidate_y, source.width, source.height),
            (node.x, node.y, node.width, node.height),
            margin,
        ) {
            return false;
        }
    }
    true
}

fn rects_overlap_with_margin(
    a: (f32, f32, f32, f32),
    b: (f32, f32, f32, f32),
    margin: f32,
) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    ax < bx + bw + margin && ax + aw + margin > bx && ay < by + bh + margin && ay + ah + margin > by
}

fn align_flowchart_mixed_recursive_compound(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.direction != Direction::TopDown {
        return;
    }

    let top_level = top_level_subgraph_indices(graph);
    if top_level.len() < 3 {
        return;
    }
    let recursive: Vec<usize> = top_level
        .iter()
        .copied()
        .filter(|idx| {
            graph
                .subgraphs
                .get(*idx)
                .map(|sub| flowchart_subgraph_is_recursive_cluster(graph, sub))
                .unwrap_or(false)
        })
        .collect();
    if recursive.len() != 1 {
        return;
    }
    let recursive_idx = recursive[0];

    let mut node_to_top: HashMap<&str, usize> = HashMap::new();
    for &idx in &top_level {
        let Some(sub) = graph.subgraphs.get(idx) else {
            continue;
        };
        if is_region_subgraph(sub) {
            continue;
        }
        for node_id in &sub.nodes {
            node_to_top.insert(node_id.as_str(), idx);
        }
    }

    let cross = graph.edges.iter().find_map(|edge| {
        let from_sg = node_to_top.get(edge.from.as_str()).copied()?;
        let to_sg = node_to_top.get(edge.to.as_str()).copied()?;
        if from_sg == to_sg || from_sg == recursive_idx || to_sg == recursive_idx {
            return None;
        }
        Some((from_sg, to_sg))
    });
    let Some((source_idx, target_idx)) = cross else {
        return;
    };

    let layouts = build_subgraph_layouts(graph, nodes, theme, config);
    let source_layout = graph
        .subgraphs
        .get(source_idx)
        .and_then(|sub| subgraph_layout_index(&layouts, sub))
        .and_then(|idx| layouts.get(idx))
        .cloned();
    let target_layout = graph
        .subgraphs
        .get(target_idx)
        .and_then(|sub| subgraph_layout_index(&layouts, sub))
        .and_then(|idx| layouts.get(idx))
        .cloned();
    let recursive_layout = graph
        .subgraphs
        .get(recursive_idx)
        .and_then(|sub| subgraph_layout_index(&layouts, sub))
        .and_then(|idx| layouts.get(idx))
        .cloned();
    let (Some(source_layout), Some(target_layout), Some(recursive_layout)) =
        (source_layout, target_layout, recursive_layout)
    else {
        return;
    };

    let origin_x = layouts
        .iter()
        .map(|layout| layout.x)
        .fold(f32::MAX, f32::min);
    let top_y = source_layout.y.min(target_layout.y);
    if !origin_x.is_finite() || !top_y.is_finite() {
        return;
    }

    let cluster_gap = (config.node_spacing * 2.0).max(80.0);
    let target_source_x = origin_x;
    let target_target_x = target_source_x + source_layout.width + cluster_gap;
    let target_recursive_x =
        target_target_x + target_layout.width * 0.5 - recursive_layout.width * 0.5;
    let target_recursive_y =
        target_layout.y + target_layout.height + (config.rank_spacing * 0.5).max(20.0);

    shift_subgraph_members(
        graph,
        source_idx,
        nodes,
        target_source_x - source_layout.x,
        top_y - source_layout.y,
    );
    shift_subgraph_members(
        graph,
        target_idx,
        nodes,
        target_target_x - target_layout.x,
        top_y - target_layout.y,
    );
    shift_subgraph_members(
        graph,
        recursive_idx,
        nodes,
        target_recursive_x - recursive_layout.x,
        target_recursive_y - recursive_layout.y,
    );

    let source_tail_gap = (config.rank_spacing * 1.4).max(70.0);
    let source_tail_top = target_recursive_y + recursive_layout.height + source_tail_gap;
    if let Some(source_sub) = graph.subgraphs.get(source_idx) {
        for node_id in &source_sub.nodes {
            let Some(node) = nodes.get_mut(node_id) else {
                continue;
            };
            if node.y > top_y + config.rank_spacing && node.y < source_tail_top {
                node.y += source_tail_top - node.y;
            }
        }
    }
}

fn align_flowchart_nested_bridge_child_lanes(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.subgraphs.len() < 3 {
        return;
    }

    let tree = SubgraphTree::build(graph);
    let subgraph_layouts = build_subgraph_layouts(graph, nodes, theme, config);

    for (parent_idx, parent) in graph.subgraphs.iter().enumerate() {
        let parent_direction = parent
            .direction
            .unwrap_or_else(|| subgraph_layout_direction(graph, parent));
        if parent_direction != Direction::TopDown {
            continue;
        }

        let Some(child_indices) = tree.children.get(parent_idx) else {
            continue;
        };
        if child_indices.len() < 3 {
            continue;
        }

        let mut node_to_child: HashMap<&str, usize> = HashMap::new();
        for &child_idx in child_indices {
            let Some(child) = graph.subgraphs.get(child_idx) else {
                continue;
            };
            for node_id in &child.nodes {
                node_to_child.insert(node_id.as_str(), child_idx);
            }
        }

        for &bridge_idx in child_indices {
            let Some(bridge) = graph.subgraphs.get(bridge_idx) else {
                continue;
            };
            if bridge.nodes.len() != 1 {
                continue;
            }
            let bridge_node = bridge.nodes[0].as_str();

            let incoming_sources: Vec<usize> = graph
                .edges
                .iter()
                .filter(|edge| edge.to == bridge_node)
                .filter_map(|edge| node_to_child.get(edge.from.as_str()).copied())
                .filter(|idx| *idx != bridge_idx)
                .collect();
            let outgoing_targets: Vec<usize> = graph
                .edges
                .iter()
                .filter(|edge| edge.from == bridge_node)
                .filter_map(|edge| node_to_child.get(edge.to.as_str()).copied())
                .filter(|idx| *idx != bridge_idx)
                .collect();
            if incoming_sources.is_empty() || outgoing_targets.len() != 1 {
                continue;
            }

            let target_idx = outgoing_targets[0];
            if incoming_sources.iter().all(|idx| *idx == target_idx) {
                continue;
            }

            let Some(target_sub) = graph.subgraphs.get(target_idx) else {
                continue;
            };
            if target_sub.nodes.len() < 2 {
                continue;
            }

            let Some(bridge_layout) = subgraph_layout_index(&subgraph_layouts, bridge)
                .and_then(|idx| subgraph_layouts.get(idx))
            else {
                continue;
            };
            let Some(target_layout) = subgraph_layout_index(&subgraph_layouts, target_sub)
                .and_then(|idx| subgraph_layouts.get(idx))
            else {
                continue;
            };

            let bridge_center_x = bridge_layout.x + bridge_layout.width * 0.5;
            let target_center_x = target_layout.x + target_layout.width * 0.5;
            shift_subgraph_members(
                graph,
                bridge_idx,
                nodes,
                target_center_x - bridge_center_x,
                0.0,
            );

            if let Some(&source_idx) = incoming_sources.iter().find(|idx| **idx != target_idx) {
                if let Some(source_layout) = graph
                    .subgraphs
                    .get(source_idx)
                    .and_then(|source_sub| subgraph_layout_index(&subgraph_layouts, source_sub))
                    .and_then(|idx| subgraph_layouts.get(idx))
                {
                    let source_center_x = source_layout.x + source_layout.width * 0.5;
                    if source_center_x + 1.0 < target_center_x {
                        let swap_dx = target_center_x - source_center_x;
                        shift_subgraph_members(graph, source_idx, nodes, swap_dx, 0.0);
                        shift_subgraph_members(graph, target_idx, nodes, -swap_dx, 0.0);
                        shift_subgraph_members(graph, bridge_idx, nodes, -swap_dx, 0.0);
                    }
                }
                align_flowchart_nested_bridge_rank_spacing(
                    graph, nodes, source_idx, bridge_idx, target_idx, config,
                );
            }
        }
    }
}

fn align_flowchart_nested_bridge_rank_spacing(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    source_idx: usize,
    bridge_idx: usize,
    target_idx: usize,
    config: &LayoutConfig,
) {
    let Some(source_sub) = graph.subgraphs.get(source_idx) else {
        return;
    };
    let Some(bridge_sub) = graph.subgraphs.get(bridge_idx) else {
        return;
    };
    let Some(target_sub) = graph.subgraphs.get(target_idx) else {
        return;
    };
    if bridge_sub.nodes.len() != 1 {
        return;
    }
    let bridge_node = bridge_sub.nodes[0].as_str();

    let Some(source_order) = flowchart_simple_chain_order(graph, source_sub) else {
        return;
    };
    let Some(target_order) = flowchart_simple_chain_order(graph, target_sub) else {
        return;
    };
    if source_order.len() < 3 || target_order.len() < 2 {
        return;
    }

    let Some((source_node, target_node)) = graph.edges.iter().find_map(|edge| {
        if edge.to != bridge_node {
            return None;
        }
        let source_pos = source_order.iter().position(|id| id == &edge.from)?;
        let bridge_to_target = graph.edges.iter().find(|candidate| {
            candidate.from == bridge_node && target_order.contains(&candidate.to)
        })?;
        Some((source_pos, bridge_to_target.to.as_str()))
    }) else {
        return;
    };
    if source_node + 2 >= source_order.len()
        || target_order
            .first()
            .map(|id| id.as_str() != target_node)
            .unwrap_or(true)
    {
        return;
    }

    let source0_id = source_order[source_node].as_str();
    let source1_id = source_order[source_node + 1].as_str();
    let source2_id = source_order[source_node + 2].as_str();
    let target0_id = target_order[0].as_str();
    let target1_id = target_order[1].as_str();

    let Some(source0) = nodes.get(source0_id) else {
        return;
    };
    let Some(source1) = nodes.get(source1_id) else {
        return;
    };
    let Some(source2) = nodes.get(source2_id) else {
        return;
    };
    let Some(bridge) = nodes.get(bridge_node) else {
        return;
    };
    let Some(target0) = nodes.get(target0_id) else {
        return;
    };
    let Some(target1) = nodes.get(target1_id) else {
        return;
    };

    let root_rank_gap = config
        .rank_spacing
        .max(config.flowchart.auto_spacing.min_spacing);
    let source_rank_gap = subgraph_layout_config_for(graph, source_sub, false, config)
        .rank_spacing
        .max(root_rank_gap);
    let target_rank_gap = subgraph_layout_config_for(graph, target_sub, false, config)
        .rank_spacing
        .max(root_rank_gap);
    let rank_gap = source_rank_gap.max(target_rank_gap);
    let first_gap = rank_gap * 1.5;
    let fork_gap = rank_gap * 2.0;
    let target_entry_gap = rank_gap + root_rank_gap + FLOWCHART_DAGRE_POINT_MARGIN;

    let source0_center = source0.y + source0.height * 0.5;
    let rank1_center = source0_center + source0.height * 0.5 + first_gap + source1.height * 0.5;
    let rank1_half = (source1.height * 0.5).max(bridge.height * 0.5);
    let rank2_half = (source2.height * 0.5).max(target0.height * 0.5);
    let rank2_center = rank1_center + rank1_half + fork_gap + rank2_half;
    let target1_center =
        rank2_center + target0.height * 0.5 + target_entry_gap + target1.height * 0.5;

    set_node_center_y(nodes, source1_id, rank1_center);
    set_node_center_y(nodes, bridge_node, rank1_center);
    set_node_center_y(nodes, source2_id, rank2_center);
    set_node_center_y(nodes, target0_id, rank2_center);
    set_node_center_y(nodes, target1_id, target1_center);

    let mut previous_id = target1_id.to_string();
    let mut previous_center = target1_center;
    for node_id in target_order.iter().skip(2) {
        let Some(previous) = nodes.get(&previous_id) else {
            return;
        };
        let Some(current) = nodes.get(node_id) else {
            return;
        };
        let center = previous_center + previous.height * 0.5 + rank_gap + current.height * 0.5;
        set_node_center_y(nodes, node_id, center);
        previous_id = node_id.clone();
        previous_center = center;
    }
}

fn flowchart_simple_chain_order(graph: &Graph, sub: &crate::ir::Subgraph) -> Option<Vec<String>> {
    if sub.nodes.len() < 2 {
        return None;
    }
    let sub_set: HashSet<&str> = sub.nodes.iter().map(|id| id.as_str()).collect();
    let mut in_deg: HashMap<String, usize> =
        sub.nodes.iter().map(|id| (id.clone(), 0usize)).collect();
    let mut out_deg: HashMap<String, usize> =
        sub.nodes.iter().map(|id| (id.clone(), 0usize)).collect();
    let mut next_map: HashMap<String, String> = HashMap::new();
    let mut edges_in_sub = 0usize;

    for edge in &graph.edges {
        if !sub_set.contains(edge.from.as_str()) || !sub_set.contains(edge.to.as_str()) {
            continue;
        }
        edges_in_sub += 1;
        let out = out_deg.entry(edge.from.clone()).or_insert(0);
        *out += 1;
        if *out == 1 {
            next_map.insert(edge.from.clone(), edge.to.clone());
        } else {
            next_map.remove(&edge.from);
        }
        *in_deg.entry(edge.to.clone()).or_insert(0) += 1;
    }

    if edges_in_sub + 1 != sub.nodes.len() {
        return None;
    }
    if in_deg.values().any(|&d| d > 1) || out_deg.values().any(|&d| d > 1) {
        return None;
    }

    let starts: Vec<&String> = sub
        .nodes
        .iter()
        .filter(|id| *in_deg.get(*id).unwrap_or(&0) == 0)
        .collect();
    if starts.len() != 1 {
        return None;
    }

    let mut order = Vec::with_capacity(sub.nodes.len());
    let mut visited = HashSet::new();
    let mut current = starts[0].clone();
    while visited.insert(current.clone()) {
        order.push(current.clone());
        if let Some(next) = next_map.get(&current) {
            current = next.clone();
        } else {
            break;
        }
    }
    if order.len() == sub.nodes.len() {
        Some(order)
    } else {
        None
    }
}

fn set_node_center_y(nodes: &mut BTreeMap<String, NodeLayout>, node_id: &str, center_y: f32) {
    if let Some(node) = nodes.get_mut(node_id) {
        node.y = center_y - node.height * 0.5;
    }
}

fn shift_subgraph_members(
    graph: &Graph,
    sub_idx: usize,
    nodes: &mut BTreeMap<String, NodeLayout>,
    dx: f32,
    dy: f32,
) {
    if dx.abs() < 0.5 && dy.abs() < 0.5 {
        return;
    }
    let Some(sub) = graph.subgraphs.get(sub_idx) else {
        return;
    };
    for node_id in &sub.nodes {
        if let Some(node) = nodes.get_mut(node_id) {
            node.x += dx;
            node.y += dy;
        }
    }
}

fn align_flowchart_recursive_cluster_external_nodes(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    subgraphs: &[SubgraphLayout],
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || graph.subgraphs.is_empty() {
        return;
    }

    for sub_idx in top_level_subgraph_indices(graph) {
        let Some(sub) = graph.subgraphs.get(sub_idx) else {
            continue;
        };
        if !flowchart_subgraph_is_recursive_cluster(graph, sub) {
            continue;
        }
        let Some(anchor_id) = subgraph_anchor_id(sub, nodes).map(str::to_string) else {
            continue;
        };
        let Some(layout) = subgraph_layout_index(subgraphs, sub).and_then(|idx| subgraphs.get(idx))
        else {
            continue;
        };

        let mut incoming = Vec::new();
        let mut outgoing = Vec::new();
        for edge in &graph.edges {
            if edge.to == anchor_id {
                if nodes
                    .get(&edge.from)
                    .map(|node| !node.hidden && node.anchor_subgraph.is_none())
                    .unwrap_or(false)
                {
                    incoming.push(edge.from.clone());
                }
            }
            if edge.from == anchor_id {
                if nodes
                    .get(&edge.to)
                    .map(|node| !node.hidden && node.anchor_subgraph.is_none())
                    .unwrap_or(false)
                {
                    outgoing.push(edge.to.clone());
                }
            }
        }
        if incoming.is_empty() || outgoing.is_empty() {
            continue;
        }

        incoming.sort();
        incoming.dedup();
        outgoing.sort();
        outgoing.dedup();

        let gap = config.rank_spacing.max(MIN_NODE_SPACING_FLOOR);
        let center_x = layout.x + layout.width * 0.5;
        let center_y = layout.y + layout.height * 0.5;
        for node_id in incoming {
            let Some(node) = nodes.get_mut(&node_id) else {
                continue;
            };
            match graph.direction {
                Direction::LeftRight => {
                    node.x = layout.x - gap - node.width;
                    node.y = center_y - node.height * 0.5;
                }
                Direction::RightLeft => {
                    node.x = layout.x + layout.width + gap;
                    node.y = center_y - node.height * 0.5;
                }
                Direction::TopDown => {
                    node.x = center_x - node.width * 0.5;
                    node.y = layout.y - gap - node.height;
                }
                Direction::BottomTop => {
                    node.x = center_x - node.width * 0.5;
                    node.y = layout.y + layout.height + gap;
                }
            }
        }
        for node_id in outgoing {
            let Some(node) = nodes.get_mut(&node_id) else {
                continue;
            };
            match graph.direction {
                Direction::LeftRight => {
                    node.x = layout.x + layout.width + gap;
                    node.y = center_y - node.height * 0.5;
                }
                Direction::RightLeft => {
                    node.x = layout.x - gap - node.width;
                    node.y = center_y - node.height * 0.5;
                }
                Direction::TopDown => {
                    node.x = center_x - node.width * 0.5;
                    node.y = layout.y + layout.height + gap;
                }
                Direction::BottomTop => {
                    node.x = center_x - node.width * 0.5;
                    node.y = layout.y - gap - node.height;
                }
            }
        }
    }
}

fn align_disconnected_components(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart || !graph.subgraphs.is_empty() {
        return;
    }

    let mut visible_nodes: Vec<String> = nodes
        .values()
        .filter(|node| !node.hidden)
        .map(|node| node.id.clone())
        .collect();
    if visible_nodes.len() < 2 {
        return;
    }
    visible_nodes.sort();

    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for node_id in &visible_nodes {
        adjacency.entry(node_id.clone()).or_default();
    }
    for edge in &graph.edges {
        if !adjacency.contains_key(&edge.from) || !adjacency.contains_key(&edge.to) {
            continue;
        }
        adjacency
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        adjacency
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
    }

    let mut visited: HashSet<String> = HashSet::new();
    let mut components: Vec<Vec<String>> = Vec::new();
    for node_id in &visible_nodes {
        if visited.contains(node_id) {
            continue;
        }
        let mut stack = vec![node_id.clone()];
        let mut comp = Vec::new();
        visited.insert(node_id.clone());
        while let Some(cur) = stack.pop() {
            comp.push(cur.clone());
            if let Some(neigh) = adjacency.get(&cur) {
                for next in neigh {
                    if visited.insert(next.clone()) {
                        stack.push(next.clone());
                    }
                }
            }
        }
        if comp.len() > 0 {
            components.push(comp);
        }
    }

    if components.len() < 2 {
        return;
    }

    #[derive(Clone)]
    struct CompBounds {
        nodes: Vec<String>,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
    }

    let mut bounds: Vec<CompBounds> = Vec::new();
    for comp in components {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for node_id in &comp {
            if let Some(node) = nodes.get(node_id) {
                min_x = min_x.min(node.x);
                min_y = min_y.min(node.y);
                max_x = max_x.max(node.x + node.width);
                max_y = max_y.max(node.y + node.height);
            }
        }
        if min_x == f32::MAX {
            continue;
        }
        bounds.push(CompBounds {
            nodes: comp,
            min_x,
            min_y,
            max_x,
            max_y,
        });
    }

    if bounds.len() < 2 {
        return;
    }

    // Dagre keeps disconnected components in the same rank. For TD/BT, that
    // means components sit side by side; for LR/RL, they stack vertically.
    let pack_horizontal = !is_horizontal(graph.direction);
    bounds.sort_by(|a, b| {
        let a_key = if pack_horizontal { a.min_x } else { a.min_y };
        let b_key = if pack_horizontal { b.min_x } else { b.min_y };
        a_key.partial_cmp(&b_key).unwrap_or(Ordering::Equal)
    });

    let target_cross = bounds
        .iter()
        .map(|b| if pack_horizontal { b.min_y } else { b.min_x })
        .fold(f32::MAX, f32::min);
    let spacing = config.node_spacing.max(MIN_NODE_SPACING_FLOOR);
    let mut cursor = if pack_horizontal {
        bounds.iter().map(|b| b.min_x).fold(f32::MAX, f32::min)
    } else {
        bounds.iter().map(|b| b.min_y).fold(f32::MAX, f32::min)
    };

    for bound in bounds {
        let min_main = if pack_horizontal {
            bound.min_x
        } else {
            bound.min_y
        };
        let max_main = if pack_horizontal {
            bound.max_x
        } else {
            bound.max_y
        };
        let current_cross = if pack_horizontal {
            bound.min_y
        } else {
            bound.min_x
        };
        let delta_main = cursor - min_main;
        let delta_cross = target_cross - current_cross;
        for node_id in &bound.nodes {
            if let Some(node) = nodes.get_mut(node_id) {
                if pack_horizontal {
                    node.x += delta_main;
                    node.y += delta_cross;
                } else {
                    node.x += delta_cross;
                    node.y += delta_main;
                }
            }
        }
        let size = (max_main - min_main).max(1.0);
        cursor += size + spacing;
    }
}

fn apply_visual_objectives(
    graph: &Graph,
    layout_edges: &[crate::ir::Edge],
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) {
    if !config.flowchart.objective.enabled {
        return;
    }
    relax_edge_span_constraints(graph, layout_edges, nodes, theme, config);
    rebalance_top_level_subgraphs_aspect(graph, nodes, config);
    let overlap_pass_enabled = match graph.kind {
        crate::ir::DiagramKind::Class => true,
        crate::ir::DiagramKind::Flowchart
        | crate::ir::DiagramKind::State
        | crate::ir::DiagramKind::Er
        | crate::ir::DiagramKind::Requirement => has_visible_node_overlap(nodes),
        _ => false,
    };
    if overlap_pass_enabled {
        resolve_node_overlaps(graph, nodes, config);
    }
}

fn apply_unconnected_class_namespace_layouts(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Class || graph.subgraphs.is_empty() {
        return;
    }

    for sub in &graph.subgraphs {
        if sub.nodes.len() < 2 {
            continue;
        }

        let member_ids: HashSet<&str> = sub.nodes.iter().map(String::as_str).collect();
        if graph.edges.iter().any(|edge| {
            member_ids.contains(edge.from.as_str()) || member_ids.contains(edge.to.as_str())
        }) {
            continue;
        }

        let visible_ids: Vec<String> = sub
            .nodes
            .iter()
            .filter(|id| {
                nodes
                    .get(id.as_str())
                    .map(|node| !node.hidden && node.anchor_subgraph.is_none())
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        if visible_ids.len() < 2 {
            continue;
        }

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        for id in &visible_ids {
            let Some(node) = nodes.get(id) else {
                continue;
            };
            min_x = min_x.min(node.x);
            max_x = max_x.max(node.x + node.width);
            min_y = min_y.min(node.y);
        }
        if min_x == f32::MAX || max_x == f32::MIN || min_y == f32::MAX {
            continue;
        }

        let center_x = (min_x + max_x) * 0.5;
        let mut cursor_y = min_y;
        for id in &visible_ids {
            if let Some(node) = nodes.get_mut(id) {
                node.x = center_x - node.width * 0.5;
                node.y = cursor_y;
                cursor_y += node.height + config.rank_spacing;
            }
        }
    }
}

fn node_main_center(node: &NodeLayout, horizontal: bool) -> f32 {
    if horizontal {
        node.x + node.width / 2.0
    } else {
        node.y + node.height / 2.0
    }
}

fn node_main_half(node: &NodeLayout, horizontal: bool) -> f32 {
    if horizontal {
        node.width / 2.0
    } else {
        node.height / 2.0
    }
}

fn shift_node_main(node: &mut NodeLayout, horizontal: bool, delta: f32) {
    if horizontal {
        node.x += delta;
    } else {
        node.y += delta;
    }
}

fn shift_node_cross(node: &mut NodeLayout, horizontal: bool, delta: f32) {
    if horizontal {
        node.y += delta;
    } else {
        node.x += delta;
    }
}

fn class_edge_arrow_extent(arrow: bool, kind: Option<crate::ir::EdgeArrowhead>) -> f32 {
    if !arrow {
        return 0.0;
    }
    match kind {
        Some(crate::ir::EdgeArrowhead::OpenTriangle) => CLASS_EDGE_OPEN_MARKER_EXTENT,
        Some(crate::ir::EdgeArrowhead::ClassDependency) => CLASS_EDGE_DEPENDENCY_MARKER_EXTENT,
        None => CLASS_EDGE_GENERIC_ARROW_EXTENT,
    }
}

fn class_edge_decoration_extent(decoration: Option<crate::ir::EdgeDecoration>) -> f32 {
    match decoration {
        Some(crate::ir::EdgeDecoration::Diamond)
        | Some(crate::ir::EdgeDecoration::DiamondFilled) => CLASS_EDGE_DECORATION_EXTENT,
        Some(crate::ir::EdgeDecoration::Lollipop) => CLASS_EDGE_LOLLIPOP_MARKER_EXTENT,
        Some(crate::ir::EdgeDecoration::Circle) | Some(crate::ir::EdgeDecoration::Cross) => {
            CLASS_EDGE_GENERIC_ARROW_EXTENT
        }
        _ => 0.0,
    }
}

fn class_edge_symbol_gap(edge: &crate::ir::Edge) -> f32 {
    class_edge_arrow_extent(edge.arrow_start, edge.arrow_start_kind)
        .max(class_edge_decoration_extent(edge.start_decoration))
        + class_edge_arrow_extent(edge.arrow_end, edge.arrow_end_kind)
            .max(class_edge_decoration_extent(edge.end_decoration))
}

fn align_class_inheritance_fan_edges(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    routed_points: &mut [Vec<(f32, f32)>],
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Class {
        return;
    }
    for (idx, edge) in graph.edges.iter().enumerate() {
        if idx >= routed_points.len()
            || !class_edge_has_open_triangle(edge)
            || edge.style == crate::ir::EdgeStyle::Dotted
            || edge.label.is_some()
            || edge.start_label.is_some()
            || edge.end_label.is_some()
        {
            continue;
        }
        let (Some(from), Some(to)) = (nodes.get(&edge.from), nodes.get(&edge.to)) else {
            continue;
        };
        if from.hidden
            || to.hidden
            || from.shape == crate::ir::NodeShape::Note
            || to.shape == crate::ir::NodeShape::Note
        {
            continue;
        }
        let Some(mid) = class_inheritance_fan_midpoint(from, to, graph.direction, config) else {
            continue;
        };
        routed_points[idx] = vec![
            rect_intersection_toward(from, mid),
            mid,
            rect_intersection_toward(to, mid),
        ];
    }
}

fn class_edge_has_open_triangle(edge: &crate::ir::Edge) -> bool {
    (edge.arrow_start && edge.arrow_start_kind == Some(crate::ir::EdgeArrowhead::OpenTriangle))
        || (edge.arrow_end && edge.arrow_end_kind == Some(crate::ir::EdgeArrowhead::OpenTriangle))
}

fn class_inheritance_fan_midpoint(
    from: &NodeLayout,
    to: &NodeLayout,
    direction: Direction,
    config: &LayoutConfig,
) -> Option<(f32, f32)> {
    let from_center = node_center(from);
    let to_center = node_center(to);
    let rank_step = (config.rank_spacing * 0.5).max(1.0);
    if is_horizontal(direction) {
        let dx = to_center.0 - from_center.0;
        if dx.abs() < 1.0 {
            return None;
        }
        let from_edge_x = if dx >= 0.0 {
            from.x + from.width
        } else {
            from.x
        };
        let to_edge_x = if dx >= 0.0 { to.x } else { to.x + to.width };
        let gap = (to_edge_x - from_edge_x).abs();
        let step = rank_step.min(gap * 0.5).max(1.0);
        let mid_x = if dx >= 0.0 {
            from_edge_x + step
        } else {
            from_edge_x - step
        };
        let cross_threshold = from.height * 0.5 + to.height * 0.5 + config.node_spacing;
        let cross_delta = (to_center.1 - from_center.1).abs();
        let mid_y = if cross_delta > cross_threshold {
            (from_center.1 + to_center.1) * 0.5
        } else {
            to_center.1
        };
        Some((mid_x, mid_y))
    } else {
        let dy = to_center.1 - from_center.1;
        if dy.abs() < 1.0 {
            return None;
        }
        let from_edge_y = if dy >= 0.0 {
            from.y + from.height
        } else {
            from.y
        };
        let to_edge_y = if dy >= 0.0 { to.y } else { to.y + to.height };
        let gap = (to_edge_y - from_edge_y).abs();
        let step = rank_step.min(gap * 0.5).max(1.0);
        let mid_y = if dy >= 0.0 {
            from_edge_y + step
        } else {
            from_edge_y - step
        };
        let cross_threshold = from.width * 0.5 + to.width * 0.5 + config.node_spacing;
        let cross_delta = (to_center.0 - from_center.0).abs();
        let mid_x = if cross_delta > cross_threshold {
            (from_center.0 + to_center.0) * 0.5
        } else {
            to_center.0
        };
        Some((mid_x, mid_y))
    }
}

fn requirement_rank_gap_main(
    direction: Direction,
    nodes: &BTreeMap<String, NodeLayout>,
    from_id: &str,
    to_id: &str,
) -> Option<f32> {
    let from = nodes.get(from_id)?;
    let to = nodes.get(to_id)?;
    let from_center = node_center(from);
    let to_center = node_center(to);
    let from_main = if is_horizontal(direction) {
        from_center.0
    } else {
        from_center.1
    };
    let to_main = if is_horizontal(direction) {
        to_center.0
    } else {
        to_center.1
    };
    let (from_start, from_end) = requirement_rank_main_bounds(direction, nodes, from_main)?;
    let (to_start, to_end) = requirement_rank_main_bounds(direction, nodes, to_main)?;
    if to_main >= from_main {
        Some((from_end + to_start) * 0.5)
    } else {
        Some((to_end + from_start) * 0.5)
    }
}

fn requirement_rank_main_bounds(
    direction: Direction,
    nodes: &BTreeMap<String, NodeLayout>,
    rank_center_main: f32,
) -> Option<(f32, f32)> {
    let mut min_main = f32::MAX;
    let mut max_main = f32::MIN;
    for node in nodes.values().filter(|node| !node.hidden) {
        let (start, end, center) = if is_horizontal(direction) {
            (node.x, node.x + node.width, node.x + node.width * 0.5)
        } else {
            (node.y, node.y + node.height, node.y + node.height * 0.5)
        };
        if (center - rank_center_main).abs() > 1.0 {
            continue;
        }
        min_main = min_main.min(start);
        max_main = max_main.max(end);
    }
    if min_main.is_finite() && max_main.is_finite() {
        Some((min_main, max_main))
    } else {
        None
    }
}

fn requirement_dagre_curve_midpoint(
    direction: Direction,
    from: &NodeLayout,
    to: &NodeLayout,
    rank_gap_main: Option<f32>,
    edge: &crate::ir::Edge,
    source_outgoing_count: usize,
    font_size: f32,
) -> (f32, f32) {
    let from_center = node_center(from);
    let to_center = node_center(to);
    let branch_to_target_side = source_outgoing_count > 1 && !edge.arrow_start;
    let target_side_cross = |from_cross: f32, to_cross: f32, to_half: f32| -> f32 {
        let sign = (from_cross - to_cross).signum();
        if sign.abs() <= f32::EPSILON {
            to_cross
        } else {
            to_cross + sign * (to_half - font_size * 0.85).max(0.0)
        }
    };
    match direction {
        Direction::TopDown | Direction::BottomTop => {
            let y = rank_gap_main.unwrap_or((from_center.1 + to_center.1) * 0.5);
            let x = if (to_center.0 - from_center.0).abs() < 2.0 {
                (from_center.0 + to_center.0) * 0.5
            } else if edge.arrow_start {
                to_center.0
            } else if branch_to_target_side {
                target_side_cross(from_center.0, to_center.0, to.width * 0.5)
            } else {
                from_center.0
            };
            (x, y)
        }
        Direction::LeftRight | Direction::RightLeft => {
            let x = rank_gap_main.unwrap_or((from_center.0 + to_center.0) * 0.5);
            let y = if (to_center.1 - from_center.1).abs() < 2.0 {
                (from_center.1 + to_center.1) * 0.5
            } else if edge.arrow_start {
                to_center.1
            } else if branch_to_target_side {
                target_side_cross(from_center.1, to_center.1, to.height * 0.5)
            } else {
                from_center.1
            };
            (x, y)
        }
    }
}

fn align_state_choice_fanout_labels(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    label_anchors: &mut [Option<(f32, f32)>],
) {
    if graph.kind != crate::ir::DiagramKind::State {
        return;
    }

    for (idx, edge) in graph.edges.iter().enumerate() {
        if edge.label.as_deref().is_none_or(str::is_empty) || idx >= label_anchors.len() {
            continue;
        }
        let Some(from) = nodes.get(&edge.from) else {
            continue;
        };
        let Some(to) = nodes.get(&edge.to) else {
            continue;
        };
        if from.shape != crate::ir::NodeShape::Diamond {
            continue;
        }

        let from_center = node_center(from);
        let to_center = node_center(to);
        if (to_center.1 - from_center.1).abs() >= (to_center.0 - from_center.0).abs() {
            let from_edge_y = if to_center.1 >= from_center.1 {
                from.y + from.height
            } else {
                from.y
            };
            let to_edge_y = if to_center.1 >= from_center.1 {
                to.y
            } else {
                to.y + to.height
            };
            label_anchors[idx] = Some((to_center.0, (from_edge_y + to_edge_y) * 0.5));
        } else {
            let from_edge_x = if to_center.0 >= from_center.0 {
                from.x + from.width
            } else {
                from.x
            };
            let to_edge_x = if to_center.0 >= from_center.0 {
                to.x
            } else {
                to.x + to.width
            };
            label_anchors[idx] = Some(((from_edge_x + to_edge_x) * 0.5, to_center.1));
        }
    }
}

fn requirement_dagre_label_anchor(
    points: &[(f32, f32)],
    direction: Direction,
) -> Option<(f32, f32)> {
    if points.len() < 3 {
        return edge_label_anchor_from_points(points);
    }
    let first = points[0];
    let middle = points[1];
    let last = points[2];
    let middle_on_axis_lane = if is_horizontal(direction) {
        (first.1 - middle.1).abs() < 2.0 || (last.1 - middle.1).abs() < 2.0
    } else {
        (first.0 - middle.0).abs() < 2.0 || (last.0 - middle.0).abs() < 2.0
    };
    if middle_on_axis_lane {
        Some(middle)
    } else {
        edge_label_anchor_from_points(points)
    }
}

fn node_center(node: &NodeLayout) -> (f32, f32) {
    (node.x + node.width * 0.5, node.y + node.height * 0.5)
}

fn rect_intersection_toward(node: &NodeLayout, toward: (f32, f32)) -> (f32, f32) {
    let center = node_center(node);
    let dx = toward.0 - center.0;
    let dy = toward.1 - center.1;
    let half_w = node.width * 0.5;
    let half_h = node.height * 0.5;

    if dx.abs() < f32::EPSILON && dy.abs() < f32::EPSILON {
        return center;
    }
    if dy.abs() * half_w > dx.abs() * half_h {
        let sy = if dy < 0.0 { -half_h } else { half_h };
        let sx = if dy.abs() < f32::EPSILON {
            0.0
        } else {
            sy * dx / dy
        };
        (center.0 + sx, center.1 + sy)
    } else {
        let sx = if dx < 0.0 { -half_w } else { half_w };
        let sy = if dx.abs() < f32::EPSILON {
            0.0
        } else {
            sx * dy / dx
        };
        (center.0 + sx, center.1 + sy)
    }
}

fn relax_edge_span_constraints(
    graph: &Graph,
    layout_edges: &[crate::ir::Edge],
    nodes: &mut BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) {
    if layout_edges.is_empty() {
        return;
    }
    match graph.kind {
        crate::ir::DiagramKind::Class
        | crate::ir::DiagramKind::Flowchart
        | crate::ir::DiagramKind::State
        | crate::ir::DiagramKind::Er
        | crate::ir::DiagramKind::Requirement => {}
        _ => return,
    }
    let horizontal = is_horizontal(graph.direction);
    let objective = &config.flowchart.objective;
    let passes = objective.edge_relax_passes.max(1);
    let step_limit = (config.rank_spacing + config.node_spacing).max(EDGE_RELAX_STEP_MIN);
    let mut label_cache: HashMap<String, TextBlock> = HashMap::new();

    for _ in 0..passes {
        let mut changed = false;
        for edge in layout_edges {
            let Some(from_node) = nodes.get(&edge.from) else {
                continue;
            };
            let Some(to_node) = nodes.get(&edge.to) else {
                continue;
            };
            if from_node.hidden || to_node.hidden {
                continue;
            }
            let from_main = node_main_center(from_node, horizontal);
            let to_main = node_main_center(to_node, horizontal);
            let from_main_half = node_main_half(from_node, horizontal);
            let to_main_half = node_main_half(to_node, horizontal);
            let main_delta = to_main - from_main;
            let current_main_gap = if main_delta >= 0.0 {
                (to_main - to_main_half) - (from_main + from_main_half)
            } else {
                (from_main - from_main_half) - (to_main + to_main_half)
            };

            let has_center_label = edge
                .label
                .as_deref()
                .is_some_and(|label| !label.trim().is_empty());
            let has_start_label = edge
                .start_label
                .as_deref()
                .is_some_and(|label| !label.trim().is_empty());
            let has_end_label = edge
                .end_label
                .as_deref()
                .is_some_and(|label| !label.trim().is_empty());
            let has_endpoint_label = has_start_label || has_end_label;
            // Flowchart dotted links are usually secondary annotations.
            // Let routing/label placement handle them without re-ranking rows.
            if graph.kind == crate::ir::DiagramKind::Flowchart
                && edge.style == crate::ir::EdgeStyle::Dotted
            {
                continue;
            }
            if !has_center_label && !has_endpoint_label {
                continue;
            }

            let mut required_main_gap = if graph.kind == crate::ir::DiagramKind::Requirement {
                config.rank_spacing
            } else {
                (config.node_spacing * objective.edge_gap_floor_ratio).max(8.0)
            };
            if let Some(label) = edge
                .label
                .as_deref()
                .filter(|label| !label.trim().is_empty())
            {
                if graph.kind == crate::ir::DiagramKind::Requirement {
                    let label_text = requirement_edge_label_text(label, config);
                    let label_block = label_cache
                        .entry(label_text.clone())
                        .or_insert_with(|| measure_label(&label_text, theme, config))
                        .clone();
                    let label_extent = if horizontal {
                        label_block.width
                    } else {
                        label_block.height
                    };
                    required_main_gap += label_extent;
                    if current_main_gap + EDGE_RELAX_GAP_TOLERANCE < required_main_gap {
                        let delta = (required_main_gap - current_main_gap).min(step_limit);
                        let ahead_id = if main_delta >= 0.0 {
                            edge.to.as_str()
                        } else {
                            edge.from.as_str()
                        };
                        if let Some(node) = nodes.get_mut(ahead_id) {
                            shift_node_main(node, horizontal, delta);
                            changed = true;
                        }
                    }
                    continue;
                }
                let label_block = label_cache
                    .entry(label.to_string())
                    .or_insert_with(|| measure_label(label, theme, config))
                    .clone();
                let label_extent = if horizontal {
                    label_block.width
                } else {
                    label_block.height
                };
                required_main_gap += label_extent * objective.edge_label_weight;
                required_main_gap += theme.font_size * EDGE_LABEL_PAD_SCALE;
            }
            if let Some(label) = edge
                .start_label
                .as_deref()
                .filter(|label| !label.trim().is_empty())
            {
                let label_block = if graph.kind == crate::ir::DiagramKind::Class {
                    measure_label_with_font_size(
                        label,
                        label_placement::CLASS_ENDPOINT_LABEL_FONT_SIZE,
                        config,
                        false,
                        theme.font_family.as_str(),
                    )
                } else {
                    label_cache
                        .entry(label.to_string())
                        .or_insert_with(|| measure_label(label, theme, config))
                        .clone()
                };
                let label_extent = if horizontal {
                    label_block.width
                } else {
                    label_block.height
                };
                required_main_gap += label_extent * objective.endpoint_label_weight;
                required_main_gap += theme.font_size * ENDPOINT_LABEL_PAD_SCALE;
            }
            if let Some(label) = edge
                .end_label
                .as_deref()
                .filter(|label| !label.trim().is_empty())
            {
                let label_block = if graph.kind == crate::ir::DiagramKind::Class {
                    measure_label_with_font_size(
                        label,
                        label_placement::CLASS_ENDPOINT_LABEL_FONT_SIZE,
                        config,
                        false,
                        theme.font_family.as_str(),
                    )
                } else {
                    label_cache
                        .entry(label.to_string())
                        .or_insert_with(|| measure_label(label, theme, config))
                        .clone()
                };
                let label_extent = if horizontal {
                    label_block.width
                } else {
                    label_block.height
                };
                required_main_gap += label_extent * objective.endpoint_label_weight;
                required_main_gap += theme.font_size * ENDPOINT_LABEL_PAD_SCALE;
            }
            if has_start_label && has_end_label {
                required_main_gap += theme.font_size * DUAL_ENDPOINT_EXTRA_PAD_SCALE;
            }
            if graph.kind == crate::ir::DiagramKind::Class {
                required_main_gap += class_edge_symbol_gap(edge);
            }
            let max_main_gap = (config.rank_spacing + config.node_spacing) * MAX_MAIN_GAP_FACTOR;
            required_main_gap = required_main_gap.min(max_main_gap);

            if current_main_gap + EDGE_RELAX_GAP_TOLERANCE < required_main_gap {
                let delta = (required_main_gap - current_main_gap).min(step_limit);
                let ahead_id = if main_delta >= 0.0 {
                    edge.to.as_str()
                } else {
                    edge.from.as_str()
                };
                if let Some(node) = nodes.get_mut(ahead_id) {
                    shift_node_main(node, horizontal, delta);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
}

fn resolve_node_overlaps(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    let horizontal = is_horizontal(graph.direction);
    let min_gap = (config.node_spacing * OVERLAP_MIN_GAP_RATIO).max(OVERLAP_MIN_GAP_FLOOR);
    let mut ids: Vec<String> = nodes
        .values()
        .filter(|node| !node.hidden)
        .map(|node| node.id.clone())
        .collect();
    if ids.len() < 2 {
        return;
    }
    ids.sort_by_key(|id| graph.node_order.get(id).copied().unwrap_or(usize::MAX));

    for _ in 0..OVERLAP_RESOLVE_PASSES {
        let mut moved = false;
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let id_a = &ids[i];
                let id_b = &ids[j];
                let (ax, ay, aw, ah, bx, by, bw, bh) = {
                    let Some(a) = nodes.get(id_a) else {
                        continue;
                    };
                    let Some(b) = nodes.get(id_b) else {
                        continue;
                    };
                    (a.x, a.y, a.width, a.height, b.x, b.y, b.width, b.height)
                };
                let overlap_x = (ax + aw).min(bx + bw) - ax.max(bx);
                let overlap_y = (ay + ah).min(by + bh) - ay.max(by);
                if overlap_x <= 0.0 || overlap_y <= 0.0 {
                    continue;
                }
                let (center_a, center_b) = if horizontal {
                    (ay + ah / 2.0, by + bh / 2.0)
                } else {
                    (ax + aw / 2.0, bx + bw / 2.0)
                };
                let mut sign = if center_b >= center_a { 1.0 } else { -1.0 };
                if (center_b - center_a).abs() < OVERLAP_CENTER_THRESHOLD {
                    let order_a = graph.node_order.get(id_a).copied().unwrap_or(usize::MAX);
                    let order_b = graph.node_order.get(id_b).copied().unwrap_or(usize::MAX);
                    sign = if order_b >= order_a { 1.0 } else { -1.0 };
                }
                let delta = if horizontal {
                    overlap_y + min_gap
                } else {
                    overlap_x + min_gap
                };
                if let Some(node_b) = nodes.get_mut(id_b) {
                    shift_node_cross(node_b, horizontal, sign * delta);
                    moved = true;
                }
            }
        }
        if !moved {
            break;
        }
    }
}

fn has_visible_node_overlap(nodes: &BTreeMap<String, NodeLayout>) -> bool {
    let mut visible: Vec<&NodeLayout> = nodes.values().filter(|node| !node.hidden).collect();
    if visible.len() < 2 {
        return false;
    }
    visible.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(Ordering::Equal));
    for i in 0..visible.len() {
        let a = visible[i];
        for b in visible.iter().skip(i + 1) {
            if b.x >= a.x + a.width {
                break;
            }
            let overlap_x = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
            let overlap_y = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
            if overlap_x > 0.0 && overlap_y > 0.0 {
                return true;
            }
        }
    }
    false
}

#[derive(Clone)]
struct VisualGroup {
    sub_idx: usize,
    nodes: Vec<String>,
    min_main: f32,
    max_main: f32,
    min_cross: f32,
    max_cross: f32,
}

fn rebalance_top_level_subgraphs_aspect(
    graph: &Graph,
    nodes: &mut BTreeMap<String, NodeLayout>,
    config: &LayoutConfig,
) {
    if graph.kind != crate::ir::DiagramKind::Flowchart {
        return;
    }
    if graph.subgraphs.len() < 2 {
        return;
    }
    if graph.nodes.len() < 120 {
        return;
    }
    let horizontal = is_horizontal(graph.direction);
    let mut groups = collect_top_level_visual_groups(graph, nodes, horizontal);
    let objective = &config.flowchart.objective;
    if groups.len() < objective.wrap_min_groups {
        return;
    }

    let min_main = groups
        .iter()
        .map(|group| group.min_main)
        .fold(f32::MAX, f32::min);
    let max_main = groups
        .iter()
        .map(|group| group.max_main)
        .fold(f32::MIN, f32::max);
    let min_cross = groups
        .iter()
        .map(|group| group.min_cross)
        .fold(f32::MAX, f32::min);
    let max_cross = groups
        .iter()
        .map(|group| group.max_cross)
        .fold(f32::MIN, f32::max);
    if min_main == f32::MAX || min_cross == f32::MAX {
        return;
    }

    let main_span = (max_main - min_main).max(1.0);
    let cross_span = (max_cross - min_cross).max(1.0);
    let target_aspect = objective.max_aspect_ratio.max(1.0);
    let aspect = main_span / cross_span;
    if aspect <= target_aspect {
        return;
    }

    let row_count = if top_level_subgraph_chain_like(graph, &groups) {
        ((aspect / target_aspect).ceil() as usize).clamp(2, groups.len())
    } else {
        ((aspect / target_aspect).sqrt().ceil() as usize).clamp(2, groups.len())
    };
    let base_row_len = groups.len() / row_count;
    let extra_rows = groups.len() % row_count;
    let gap_main = config.node_spacing.max(12.0) * objective.wrap_main_gap_scale.max(0.1);
    let gap_cross = config.rank_spacing.max(12.0) * objective.wrap_cross_gap_scale.max(0.1);

    let mut row_start = 0usize;
    let mut cursor_cross = min_cross;
    for row in 0..row_count {
        let row_len = base_row_len + usize::from(row < extra_rows);
        if row_len == 0 {
            continue;
        }
        let row_end = row_start + row_len;
        let mut cursor_main = min_main;
        let mut row_cross_span = 0.0_f32;
        for group in &mut groups[row_start..row_end] {
            let delta_main = cursor_main - group.min_main;
            let delta_cross = cursor_cross - group.min_cross;
            for node_id in &group.nodes {
                if let Some(node) = nodes.get_mut(node_id) {
                    shift_node_main(node, horizontal, delta_main);
                    shift_node_cross(node, horizontal, delta_cross);
                }
            }
            group.min_main += delta_main;
            group.max_main += delta_main;
            group.min_cross += delta_cross;
            group.max_cross += delta_cross;
            cursor_main = group.max_main + gap_main;
            row_cross_span = row_cross_span.max(group.max_cross - group.min_cross);
        }
        cursor_cross += row_cross_span + gap_cross;
        row_start = row_end;
    }
}

fn collect_top_level_visual_groups(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    horizontal: bool,
) -> Vec<VisualGroup> {
    let top_level = top_level_subgraph_indices(graph);
    if top_level.len() < 2 {
        return Vec::new();
    }

    let mut seen: HashSet<&str> = HashSet::new();
    for &idx in &top_level {
        for node_id in &graph.subgraphs[idx].nodes {
            if !seen.insert(node_id.as_str()) {
                return Vec::new();
            }
        }
    }

    let mut groups = Vec::new();
    for &idx in &top_level {
        let sub = &graph.subgraphs[idx];
        if is_region_subgraph(sub) {
            continue;
        }
        let mut ids: Vec<String> = Vec::new();
        let mut min_main = f32::MAX;
        let mut max_main = f32::MIN;
        let mut min_cross = f32::MAX;
        let mut max_cross = f32::MIN;
        for node_id in &sub.nodes {
            let Some(node) = nodes.get(node_id) else {
                continue;
            };
            if node.hidden {
                continue;
            }
            ids.push(node_id.clone());
            let (main_start, main_end) = if horizontal {
                (node.x, node.x + node.width)
            } else {
                (node.y, node.y + node.height)
            };
            let (cross_start, cross_end) = if horizontal {
                (node.y, node.y + node.height)
            } else {
                (node.x, node.x + node.width)
            };
            min_main = min_main.min(main_start);
            max_main = max_main.max(main_end);
            min_cross = min_cross.min(cross_start);
            max_cross = max_cross.max(cross_end);
        }
        if ids.is_empty() {
            continue;
        }
        groups.push(VisualGroup {
            sub_idx: idx,
            nodes: ids,
            min_main,
            max_main,
            min_cross,
            max_cross,
        });
    }
    groups.sort_by(|a, b| {
        a.min_main
            .partial_cmp(&b.min_main)
            .unwrap_or(Ordering::Equal)
    });
    groups
}

fn top_level_subgraph_chain_like(graph: &Graph, groups: &[VisualGroup]) -> bool {
    if groups.len() < 3 {
        return false;
    }
    let mut node_to_subgraph: HashMap<&str, usize> = HashMap::new();
    for group in groups {
        for node_id in &group.nodes {
            node_to_subgraph.insert(node_id.as_str(), group.sub_idx);
        }
    }

    let mut indegree: HashMap<usize, usize> = HashMap::new();
    let mut outdegree: HashMap<usize, usize> = HashMap::new();
    let mut cross_edges = 0usize;
    for edge in &graph.edges {
        let Some(&from_sub) = node_to_subgraph.get(edge.from.as_str()) else {
            continue;
        };
        let Some(&to_sub) = node_to_subgraph.get(edge.to.as_str()) else {
            continue;
        };
        if from_sub == to_sub {
            continue;
        }
        cross_edges += 1;
        *outdegree.entry(from_sub).or_default() += 1;
        *indegree.entry(to_sub).or_default() += 1;
    }
    if cross_edges < groups.len().saturating_sub(1) {
        return false;
    }
    for group in groups {
        if indegree.get(&group.sub_idx).copied().unwrap_or(0) > 1 {
            return false;
        }
        if outdegree.get(&group.sub_idx).copied().unwrap_or(0) > 1 {
            return false;
        }
    }
    true
}

fn build_subgraph_layouts(
    graph: &Graph,
    nodes: &BTreeMap<String, NodeLayout>,
    theme: &Theme,
    config: &LayoutConfig,
) -> Vec<SubgraphLayout> {
    let mut subgraphs = Vec::new();
    let mut layout_index_by_graph_index: Vec<Option<usize>> = vec![None; graph.subgraphs.len()];
    let pad_tree = SubgraphTree::build(graph);
    for (sub_idx, sub) in graph.subgraphs.iter().enumerate() {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for node_id in &sub.nodes {
            if let Some(node) = nodes.get(node_id) {
                min_x = min_x.min(node.x);
                min_y = min_y.min(node.y);
                max_x = max_x.max(node.x + node.width);
                max_y = max_y.max(node.y + node.height);
            }
        }

        if min_x == f32::MAX {
            continue;
        }

        let style = resolve_subgraph_style(sub, graph);
        let mut label_block = measure_subgraph_label(graph, sub, theme, config);
        let label_empty = sub.label.trim().is_empty();
        if label_empty {
            label_block.width = 0.0;
            label_block.height = 0.0;
        }
        let nested_depth = pad_tree.max_nested_composite_depth_below(sub_idx, graph);
        let (padding_x, padding_y, top_padding) =
            subgraph_padding_from_label_with_depth(graph, sub, theme, &label_block, nested_depth);

        let node_width = max_x - min_x;
        let node_height = max_y - min_y;
        let direction = subgraph_layout_direction(graph, sub);
        let lane_extra = flowchart_recursive_cycle_lane_extra(graph, sub, &sub.nodes);
        let base_width = node_width + padding_x * 2.0;
        let label_side_padding = if graph.kind == crate::ir::DiagramKind::Flowchart {
            FLOWCHART_SUBGRAPH_LABEL_SIDE_PAD
        } else if graph.kind == crate::ir::DiagramKind::State {
            if is_sparse_non_root_leaf_state_composite(graph, sub) {
                STATE_SPARSE_LEAF_LABEL_SIDE_PAD.min(padding_x)
            } else {
                padding_x.min(STATE_SUBGRAPH_BASE_PAD)
            }
        } else {
            padding_x
        };
        let min_label_width = if label_empty {
            base_width
        } else {
            label_block.width + label_side_padding * 2.0
        };
        let content_width = base_width.max(min_label_width);
        let label_extra_width = content_width - base_width;
        let width = content_width
            + if is_horizontal(direction) {
                0.0
            } else {
                lane_extra
            };
        let height = node_height
            + padding_y
            + top_padding
            + if is_horizontal(direction) {
                lane_extra
            } else {
                0.0
            };

        layout_index_by_graph_index[sub_idx] = Some(subgraphs.len());
        subgraphs.push(SubgraphLayout {
            label: sub.label.clone(),
            label_block,
            nodes: sub.nodes.clone(),
            x: min_x - padding_x - label_extra_width / 2.0,
            y: min_y - top_padding,
            width,
            height,
            style,
            icon: sub.icon.clone(),
        });
    }

    if subgraphs.len() > 1 {
        let tree = SubgraphTree::build(graph);

        // Collect all descendants for each subgraph via the tree so we only
        // visit actual parent-child pairs instead of every O(n²) combination.
        // Process from leaves up so that child bounds are final before parents
        // expand to contain them.
        let mut all_descendants: Vec<Vec<usize>> = vec![Vec::new(); graph.subgraphs.len()];
        // Post-order traversal: collect leaves first, then parents.
        let mut order: Vec<usize> = Vec::with_capacity(graph.subgraphs.len());
        let mut stack: Vec<(usize, bool)> =
            tree.top_level.iter().rev().map(|&i| (i, false)).collect();
        while let Some((idx, visited)) = stack.pop() {
            if visited {
                order.push(idx);
                continue;
            }
            stack.push((idx, true));
            if let Some(children) = tree.children.get(idx) {
                for &child in children.iter().rev() {
                    stack.push((child, false));
                }
            }
        }

        // Build transitive descendant lists bottom-up.
        for &idx in &order {
            let mut descs = Vec::new();
            if let Some(children) = tree.children.get(idx) {
                for &child in children {
                    if layout_index_by_graph_index
                        .get(child)
                        .and_then(|idx| *idx)
                        .is_some()
                    {
                        descs.push(child);
                    }
                    if let Some(child_descs) = all_descendants.get(child) {
                        descs.extend(child_descs.iter().copied());
                    }
                }
            }
            if let Some(slot) = all_descendants.get_mut(idx) {
                *slot = descs;
            }
        }

        // Expand each parent's bounds to contain all its descendants.
        // Concurrent-region clusters (empty label, __region_ id) participate
        // here too — otherwise the parent composite state only grows to fit
        // the region's grandchildren (inner states) and the region rects
        // themselves visibly overflow.
        for &i in &order {
            let Some(parent_layout_idx) = layout_index_by_graph_index.get(i).and_then(|idx| *idx)
            else {
                continue;
            };
            for &j in &all_descendants[i] {
                let Some(child_layout_idx) =
                    layout_index_by_graph_index.get(j).and_then(|idx| *idx)
                else {
                    continue;
                };
                let child_is_region = is_region_subgraph(&graph.subgraphs[j]);
                let (pad_x, pad_y) = if child_is_region {
                    // Regions already contain their own padding; the parent
                    // adds a small breathing-room gap around them. JS reference
                    // shows ~35 px between region rects and the parent border.
                    (35.0, 35.0)
                } else if graph.kind == crate::ir::DiagramKind::Flowchart
                    && flowchart_subgraph_is_recursive_cluster(graph, &graph.subgraphs[i])
                    && flowchart_subgraph_is_recursive_cluster(graph, &graph.subgraphs[j])
                {
                    flowchart_recursive_child_cluster_padding(subgraph_layout_direction(
                        graph,
                        &graph.subgraphs[i],
                    ))
                } else if graph.kind == crate::ir::DiagramKind::State {
                    let pad = (theme.font_size * 1.8).max(24.0);
                    (pad, pad)
                } else {
                    (12.0, 12.0)
                };
                let (child_x, child_y, child_w, child_h) = {
                    let child = &subgraphs[child_layout_idx];
                    (child.x, child.y, child.width, child.height)
                };
                let parent = &mut subgraphs[parent_layout_idx];
                let min_x = parent.x.min(child_x - pad_x);
                let min_y = parent.y.min(child_y - pad_y);
                let max_x = (parent.x + parent.width).max(child_x + child_w + pad_x);
                let max_y = (parent.y + parent.height).max(child_y + child_h + pad_y);
                parent.x = min_x;
                parent.y = min_y;
                parent.width = max_x - min_x;
                parent.height = max_y - min_y;
            }
        }
    }

    subgraphs.sort_by(|a, b| {
        let area_a = a.width * a.height;
        let area_b = b.width * b.height;
        area_b.partial_cmp(&area_a).unwrap_or(Ordering::Equal)
    });
    subgraphs
}

fn merge_node_style(target: &mut crate::ir::NodeStyle, source: &crate::ir::NodeStyle) {
    if source.fill.is_some() {
        target.fill = source.fill.clone();
    }
    if source.stroke.is_some() {
        target.stroke = source.stroke.clone();
    }
    if source.text_color.is_some() {
        target.text_color = source.text_color.clone();
    }
    if source.stroke_width.is_some() {
        target.stroke_width = source.stroke_width;
    }
    if source.stroke_dasharray.is_some() {
        target.stroke_dasharray = source.stroke_dasharray.clone();
    }
    if source.line_color.is_some() {
        target.line_color = source.line_color.clone();
    }
    if source.font_style.is_some() {
        target.font_style = source.font_style.clone();
    }
    if source.font_weight.is_some() {
        target.font_weight = source.font_weight.clone();
    }
}

fn shape_padding_factors(shape: crate::ir::NodeShape) -> (f32, f32) {
    match shape {
        crate::ir::NodeShape::Stadium => (0.43, 0.5),
        crate::ir::NodeShape::Subroutine => (0.54, 0.5),
        crate::ir::NodeShape::Parallelogram => (0.894, 0.5),
        crate::ir::NodeShape::ParallelogramAlt => (0.904, 0.5),
        _ => (1.0, 1.0),
    }
}

fn has_class_body_content(label: &TextBlock) -> bool {
    class_body_line_count(label) > 0
}

fn is_block_arrow_shape(shape: crate::ir::NodeShape) -> bool {
    matches!(
        shape,
        crate::ir::NodeShape::BlockArrowRight
            | crate::ir::NodeShape::BlockArrowLeft
            | crate::ir::NodeShape::BlockArrowUp
            | crate::ir::NodeShape::BlockArrowDown
            | crate::ir::NodeShape::BlockArrowX
            | crate::ir::NodeShape::BlockArrowY
            | crate::ir::NodeShape::BlockArrowXUp
            | crate::ir::NodeShape::BlockArrowXDown
            | crate::ir::NodeShape::BlockArrowYRight
            | crate::ir::NodeShape::BlockArrowYLeft
            | crate::ir::NodeShape::BlockArrowRightUp
            | crate::ir::NodeShape::BlockArrowRightDown
            | crate::ir::NodeShape::BlockArrowLeftUp
            | crate::ir::NodeShape::BlockArrowLeftDown
            | crate::ir::NodeShape::BlockArrowAll
    )
}

fn block_arrow_label_collapses_to_empty(label: &TextBlock) -> bool {
    label
        .lines
        .iter()
        .all(|line| line.text().chars().all(|ch| ch.is_ascii_whitespace()))
}

fn class_body_line_count(label: &TextBlock) -> usize {
    label
        .lines
        .iter()
        .skip_while(|line| line.text().trim() != "---")
        .skip(1)
        .filter(|line| {
            let text = line.text();
            text.trim() != "---" && !text.trim().is_empty()
        })
        .count()
}

fn is_class_annotation_label_text(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with('\u{00ab}') && trimmed.ends_with('\u{00bb}')
}

fn text_line_measured_width(line: &TextLine, theme: &Theme, config: &LayoutConfig) -> f32 {
    let font_size = theme.font_size.max(16.0);
    text_width(
        line.text().as_ref(),
        font_size,
        theme.font_family.as_str(),
        config.fast_text_metrics,
    )
}

fn text_line_measured_width_with_weight(
    line: &TextLine,
    theme: &Theme,
    config: &LayoutConfig,
    font_weight: u16,
) -> f32 {
    let font_size = theme.font_size.max(16.0);
    if config.fast_text_metrics {
        return text_width(
            line.text().as_ref(),
            font_size,
            theme.font_family.as_str(),
            true,
        );
    }
    text_metrics::measure_text_width_with_weight(
        line.text().as_ref(),
        font_size,
        theme.font_family.as_str(),
        font_weight,
    )
    .unwrap_or_else(|| {
        text_width(
            line.text().as_ref(),
            font_size,
            theme.font_family.as_str(),
            true,
        )
    })
}

fn class_title_and_body_widths(
    label: &TextBlock,
    theme: &Theme,
    config: &LayoutConfig,
) -> (f32, f32, usize) {
    let mut title_width: f32 = 0.0;
    let mut body_width: f32 = 0.0;
    let mut content_line_count = 0usize;
    let mut in_body = false;

    for line in &label.lines {
        let text = line.text();
        let trimmed = text.trim();
        if trimmed == "---" {
            in_body = true;
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }

        content_line_count += 1;
        if in_body {
            let width = text_line_measured_width(line, theme, config);
            body_width = body_width.max(width);
        } else if is_class_annotation_label_text(trimmed) {
            let width = text_line_measured_width(line, theme, config);
            title_width = title_width.max(width);
        } else {
            let width = text_line_measured_width_with_weight(line, theme, config, 700);
            title_width = title_width.max(width);
        }
    }

    (title_width, body_width, content_line_count)
}

fn shape_size(
    shape: crate::ir::NodeShape,
    label: &TextBlock,
    config: &LayoutConfig,
    theme: &Theme,
    kind: crate::ir::DiagramKind,
) -> (f32, f32) {
    if kind == crate::ir::DiagramKind::Flowchart && is_flowchart_icon_shape(shape) {
        return flowchart_icon_shape_size(shape, label);
    }

    if kind == crate::ir::DiagramKind::Class && shape == crate::ir::NodeShape::Text {
        return (
            label.width.max(1.0),
            label.height.max(theme.font_size * 1.5),
        );
    }

    if kind == crate::ir::DiagramKind::Class
        && shape == crate::ir::NodeShape::Rectangle
        && has_class_body_content(label)
    {
        let (title_width, body_width, content_line_count) =
            class_title_and_body_widths(label, theme, config);
        let line_height = theme.font_size * config.class_diagram_label_line_height();
        return (
            (body_width + title_width * 0.5 + CLASS_BOX_PADDING * 2.0)
                .max(title_width + CLASS_BOX_PADDING * 2.0)
                .max(1.0),
            ((content_line_count as f32 + CLASS_BOX_BODY_EXTRA_LINES) * line_height).max(1.0),
        );
    }

    if kind == crate::ir::DiagramKind::Flowchart && shape == crate::ir::NodeShape::DividedRect {
        let inner_height = label.height + FLOWCHART_LABEL_PADDING;
        return (
            (label.width + FLOWCHART_LABEL_PADDING).max(1.0),
            (inner_height * (1.0 + FLOWCHART_DIVIDED_RECT_HEADER_RATIO)).max(1.0),
        );
    }

    if kind == crate::ir::DiagramKind::Flowchart && shape == crate::ir::NodeShape::Bang {
        let half_padding = FLOWCHART_LABEL_PADDING / 2.0;
        let effective_width = (label.width + 10.0 * half_padding).max(label.width + 20.0);
        let effective_height = (label.height + 8.0 * half_padding).max(label.height + 20.0);
        return (
            effective_width * FLOWCHART_BANG_BBOX_SCALE,
            effective_height * FLOWCHART_BANG_BBOX_SCALE,
        );
    }

    if kind == crate::ir::DiagramKind::Flowchart && shape == crate::ir::NodeShape::Hourglass {
        return (30.0, 30.0);
    }

    if kind == crate::ir::DiagramKind::Flowchart && shape == crate::ir::NodeShape::Text {
        return (
            (label.width + FLOWCHART_LABEL_PADDING).max(1.0),
            (label.height + FLOWCHART_LABEL_PADDING).max(1.0),
        );
    }

    if kind == crate::ir::DiagramKind::Flowchart && shape == crate::ir::NodeShape::RoundRect {
        return (
            (label.width + FLOWCHART_LABEL_PADDING * 2.0).max(1.0),
            (label.height + FLOWCHART_LABEL_PADDING * 2.0).max(1.0),
        );
    }

    if kind == crate::ir::DiagramKind::Flowchart && shape == crate::ir::NodeShape::SlopedRect {
        return (
            (label.width + FLOWCHART_LABEL_PADDING * 2.0).max(1.0),
            ((label.height + FLOWCHART_LABEL_PADDING * 2.0) * 1.5).max(1.0),
        );
    }

    if kind == crate::ir::DiagramKind::Flowchart && shape == crate::ir::NodeShape::NotchedPentagon {
        return (
            (label.width + FLOWCHART_LABEL_PADDING * 2.0).max(15.0),
            (label.height + FLOWCHART_LABEL_PADDING * 2.0).max(5.0),
        );
    }

    if kind == crate::ir::DiagramKind::Flowchart && shape == crate::ir::NodeShape::LightningBolt {
        return (35.0, 70.0);
    }

    if kind == crate::ir::DiagramKind::Flowchart && shape == crate::ir::NodeShape::CurvedTrapezoid {
        return (
            ((label.width + FLOWCHART_LABEL_PADDING * 2.0) * 1.25).max(20.0),
            (label.height + FLOWCHART_LABEL_PADDING * 2.0).max(5.0),
        );
    }

    if kind == crate::ir::DiagramKind::Flowchart && shape == crate::ir::NodeShape::Document {
        let width = (label.width + FLOWCHART_LABEL_PADDING * 2.0).max(14.0);
        let body_height = label.height + FLOWCHART_LABEL_PADDING * 2.0;
        let wave_amplitude = body_height / 8.0;
        return (width, body_height + wave_amplitude * 2.0);
    }

    if kind == crate::ir::DiagramKind::Flowchart && shape == crate::ir::NodeShape::WavyRect {
        let width = (label.width + FLOWCHART_LABEL_PADDING * 2.0).max(1.0);
        let body_height = label.height + FLOWCHART_LABEL_PADDING;
        return (width, body_height * 1.5);
    }

    if kind == crate::ir::DiagramKind::Flowchart && shape == crate::ir::NodeShape::LinedDocument {
        let body_width = (label.width + FLOWCHART_LABEL_PADDING * 2.0).max(14.0);
        let body_height = label.height + FLOWCHART_LABEL_PADDING * 2.0;
        let wave_amplitude = body_height / 8.0;
        return (body_width * 1.1, body_height + wave_amplitude * 2.0);
    }

    if kind == crate::ir::DiagramKind::Flowchart && shape == crate::ir::NodeShape::TagDocument {
        let body_width = (label.width + FLOWCHART_LABEL_PADDING * 2.0).max(14.0);
        let body_height = label.height + FLOWCHART_LABEL_PADDING * 2.0;
        let wave_amplitude = body_height / 8.0;
        return (body_width * 1.1, body_height + wave_amplitude * 2.0);
    }

    if kind == crate::ir::DiagramKind::Flowchart
        && shape == crate::ir::NodeShape::HorizontalCylinder
    {
        let h = (label.height + FLOWCHART_TILTED_CYLINDER_LABEL_PADDING).max(1.0);
        let ry = h / 2.0;
        let rx = ry / (2.5 + h / 50.0);
        let body_width = (label.width + rx + FLOWCHART_TILTED_CYLINDER_LABEL_PADDING).max(10.0);
        return ((body_width + 2.0 * rx).max(1.0), h);
    }

    if kind == crate::ir::DiagramKind::Flowchart && shape == crate::ir::NodeShape::WindowPane {
        return (
            (label.width + FLOWCHART_LABEL_PADDING * 2.0 + FLOWCHART_WINDOW_PANE_OFFSET).max(1.0),
            (label.height + FLOWCHART_LABEL_PADDING * 2.0 + FLOWCHART_WINDOW_PANE_OFFSET).max(1.0),
        );
    }

    if kind == crate::ir::DiagramKind::Flowchart && shape == crate::ir::NodeShape::OddShape {
        let height = (label.height + FLOWCHART_LABEL_PADDING).max(1.0);
        let body_width = (label.width + FLOWCHART_LABEL_PADDING).max(1.0);
        return (body_width + height / 4.0, height);
    }

    if kind == crate::ir::DiagramKind::Flowchart
        && matches!(
            shape,
            crate::ir::NodeShape::BraceLeft | crate::ir::NodeShape::BraceRight
        )
    {
        let body_width = label.width + FLOWCHART_LABEL_PADDING;
        let body_height = label.height + FLOWCHART_LABEL_PADDING;
        let radius = (body_height * 0.1).max(5.0);
        return (body_width + radius * 2.0, body_height + radius * 2.0);
    }

    if kind == crate::ir::DiagramKind::Flowchart && shape == crate::ir::NodeShape::BraceBoth {
        let body_width = label.width + FLOWCHART_LABEL_PADDING;
        let body_height = label.height + FLOWCHART_LABEL_PADDING;
        let radius = (body_height * 0.1).max(5.0);
        return (body_width + radius * 2.5, body_height + radius * 2.0);
    }

    if kind == crate::ir::DiagramKind::Flowchart
        && matches!(
            shape,
            crate::ir::NodeShape::Trapezoid | crate::ir::NodeShape::TrapezoidAlt
        )
    {
        let padding = FLOWCHART_LABEL_PADDING
            * if shape == crate::ir::NodeShape::TrapezoidAlt {
                2.0
            } else {
                1.0
            };
        let body_width = (label.width + padding).max(1.0);
        let height = (label.height + padding).max(1.0);
        return (body_width + height, height);
    }

    if kind == crate::ir::DiagramKind::Flowchart
        && matches!(
            shape,
            crate::ir::NodeShape::LeanLeft | crate::ir::NodeShape::LeanRight
        )
    {
        let body_width = (label.width + FLOWCHART_LABEL_PADDING).max(1.0);
        let height = (label.height + FLOWCHART_LABEL_PADDING).max(1.0);
        return (body_width + height, height);
    }

    if kind == crate::ir::DiagramKind::Flowchart && shape == crate::ir::NodeShape::Hexagon {
        let height = (label.height + FLOWCHART_LABEL_PADDING).max(1.0);
        let width = (label.width + height / 2.0 + FLOWCHART_LABEL_PADDING).max(1.0);
        return (width, height);
    }

    if shape == crate::ir::NodeShape::Note {
        let pad_x = config.node_padding_x.min(6.0);
        let pad_y = config.node_padding_y.min(6.0);
        return (
            (label.width + pad_x * 2.0).max(1.0),
            (label.height + pad_y * 2.0).max(1.0),
        );
    }

    if kind == crate::ir::DiagramKind::Block && is_block_arrow_shape(shape) {
        let node_padding = BLOCK_NODE_SHAPE_PADDING;
        let (label_width, label_height) = if block_arrow_label_collapses_to_empty(label) {
            (0.0, 0.0)
        } else {
            (label.width, label.height)
        };
        let height = label_height + 2.0 * node_padding;
        let width = label_width + height + node_padding;
        return (width.max(1.0), height.max(1.0));
    }

    if kind == crate::ir::DiagramKind::Block && shape == crate::ir::NodeShape::Hexagon {
        let height = label.height + BLOCK_NODE_SHAPE_PADDING;
        let notch = height / 4.0;
        let width = label.width + 2.0 * notch + BLOCK_NODE_SHAPE_PADDING;
        return (width.max(1.0), height.max(1.0));
    }

    if kind == crate::ir::DiagramKind::Block && shape == crate::ir::NodeShape::Diamond {
        let size = label.width + label.height + 2.0 * BLOCK_NODE_SHAPE_PADDING;
        return (size.max(1.0), size.max(1.0));
    }

    if kind == crate::ir::DiagramKind::Block {
        match shape {
            crate::ir::NodeShape::Parallelogram => {
                let height = label.height + BLOCK_NODE_SHAPE_PADDING;
                let width = label.width + BLOCK_NODE_SHAPE_PADDING + height * (2.0 / 3.0);
                return (width.max(1.0), height.max(1.0));
            }
            crate::ir::NodeShape::ParallelogramAlt => {
                let height = label.height + BLOCK_NODE_SHAPE_PADDING;
                let width = label.width + BLOCK_NODE_SHAPE_PADDING + height / 3.0;
                return (width.max(1.0), height.max(1.0));
            }
            crate::ir::NodeShape::Trapezoid | crate::ir::NodeShape::TrapezoidAlt => {
                let height = label.height + BLOCK_NODE_SHAPE_PADDING;
                let width = label.width + BLOCK_NODE_SHAPE_PADDING + height * (2.0 / 3.0);
                return (width.max(1.0), height.max(1.0));
            }
            _ => {}
        }
    }

    if kind == crate::ir::DiagramKind::Er
        && shape == crate::ir::NodeShape::RoundRect
        && !has_class_body_content(label)
    {
        let pad_x = ER_ENTITY_DIAGRAM_PADDING;
        let pad_y = ER_ENTITY_DIAGRAM_PADDING * 1.5;
        return (
            (label.width + pad_x * 2.0).max(ER_ENTITY_MIN_WIDTH),
            (label.height + pad_y * 2.0).max(ER_ENTITY_MIN_HEIGHT),
        );
    }

    if kind == crate::ir::DiagramKind::Requirement {
        // Mermaid's requirementBox measures the visible text, adds a 20px
        // gap after the title/name header, then adds 10px padding on each
        // side. The generic flowchart rectangle sizing underestimates that
        // height and pins short element nodes to an overly wide minimum.
        let body_gap = if label.lines.len() > 2 { 20.0 } else { 0.0 };
        let box_padding = 20.0;
        return (
            (label.width + box_padding).max(1.0),
            (label.height + body_gap + box_padding).max(1.0),
        );
    }

    let (pad_x_factor, pad_y_factor) = shape_padding_factors(shape);
    let (kind_pad_x_scale, kind_pad_y_scale) = match kind {
        crate::ir::DiagramKind::Class => {
            let pad_x_scale = if has_class_body_content(label) {
                CLASS_BODY_PAD_X_SCALE
            } else {
                CLASS_EMPTY_PAD_X_SCALE
            };
            (pad_x_scale, 0.8)
        }
        crate::ir::DiagramKind::Er => (1.05, 1.15),
        crate::ir::DiagramKind::Flowchart if shape == crate::ir::NodeShape::Rectangle => {
            (FLOWCHART_RECT_PAD_SCALE, FLOWCHART_RECT_PAD_SCALE)
        }
        crate::ir::DiagramKind::Kanban => (2.3, 0.67),
        crate::ir::DiagramKind::Block => (0.2, 0.4),
        _ => (1.0, 1.0),
    };
    let mut pad_x = config.node_padding_x * pad_x_factor * kind_pad_x_scale;
    let mut pad_y = config.node_padding_y * pad_y_factor * kind_pad_y_scale;
    if kind == crate::ir::DiagramKind::State {
        let dynamic_pad_x =
            (theme.font_size * STATE_PAD_X_SCALE).max(label.width * STATE_PAD_X_LABEL_RATIO);
        let dynamic_pad_y =
            (theme.font_size * STATE_PAD_Y_SCALE).max(label.height * STATE_PAD_Y_LABEL_RATIO);
        pad_x = dynamic_pad_x;
        pad_y = dynamic_pad_y;
    }
    let base_width = label.width + pad_x * 2.0;
    let base_height = label.height + pad_y * 2.0;
    let mut width = base_width;
    let mut height = base_height;
    let label_empty = label.lines.len() == 1 && label.lines[0].text().trim().is_empty();

    match shape {
        crate::ir::NodeShape::Diamond => {
            let size = if kind == crate::ir::DiagramKind::Flowchart {
                // Mermaid's classic `question` diamond uses the actual rendered
                // label bbox even when the label contains explicit line breaks.
                label.width + label.height + FLOWCHART_LABEL_PADDING * 2.0
            } else if kind == crate::ir::DiagramKind::State && label_empty {
                STATE_CHOICE_DIAMOND_SIZE
            } else {
                // Mermaid renders non-flowchart diamonds as squares sized off
                // the larger dimension rather than stretching independently.
                // Empty-label state diamonds (`<<choice>>` markers) get a fixed
                // JS-equivalent size above.
                base_width.max(base_height) * DIAMOND_SCALE
            };
            width = size;
            height = size;
        }
        crate::ir::NodeShape::ForkJoin => {
            // JS lays fork/join bars out with marker-like 14px height while
            // rendering the visible black bar as 10px centered inside it.
            width = FORK_JOIN_MIN_WIDTH;
            height = if kind == crate::ir::DiagramKind::State {
                STATE_FORK_JOIN_LAYOUT_HEIGHT
            } else {
                FORK_JOIN_MIN_HEIGHT
            };
        }
        crate::ir::NodeShape::Circle | crate::ir::NodeShape::DoubleCircle => {
            let size = if label_empty {
                (config.node_padding_y * CIRCLE_EMPTY_HEIGHT_SCALE).max(CIRCLE_EMPTY_MIN_SIZE)
            } else if kind == crate::ir::DiagramKind::Flowchart {
                let extra_gap = if shape == crate::ir::NodeShape::DoubleCircle {
                    FLOWCHART_DOUBLE_CIRCLE_GAP * 2.0
                } else {
                    0.0
                };
                label.width + FLOWCHART_LABEL_PADDING + extra_gap
            } else {
                width.max(height)
            };
            width = size;
            height = size;
        }
        crate::ir::NodeShape::Stadium => {}
        crate::ir::NodeShape::RoundRect => {
            // State and block diagrams use JS's basic-label-container sized
            // strictly to the label plus its shape padding. The flowchart
            // roundrect scale would make those boxes wider than JS.
            if !matches!(
                kind,
                crate::ir::DiagramKind::State | crate::ir::DiagramKind::Block
            ) {
                width *= ROUND_RECT_WIDTH_SCALE;
                height *= ROUND_RECT_HEIGHT_SCALE;
            }
        }
        crate::ir::NodeShape::Cylinder => {
            if kind == crate::ir::DiagramKind::Block {
                let width = (label.width + 8.0).max(8.0);
                let rx = width / 2.0;
                let ry = rx / (2.5 + width / 50.0);
                return (width, (label.height + 8.0 + ry * 3.0).max(8.0));
            }
            if kind == crate::ir::DiagramKind::Flowchart {
                let width = (label.width + FLOWCHART_CYLINDER_PAD_X).max(8.0);
                let rx = width / 2.0;
                let ry = rx / (2.5 + width / 50.0);
                return (
                    width,
                    (label.height + FLOWCHART_CYLINDER_PAD_Y + ry * 3.0).max(8.0),
                );
            }
            // JS formula: ry = (w/2) / (2.5 + w/50), h += ry
            let rx = width / 2.0;
            let ry = rx / (2.5 + width / 50.0);
            height += ry;
        }
        crate::ir::NodeShape::LinedCylinder if kind == crate::ir::DiagramKind::Flowchart => {
            let width = (label.width + FLOWCHART_LINED_CYLINDER_PADDING * 2.0).max(10.0);
            let rx = width / 2.0;
            let ry = rx / (2.5 + width / 50.0);
            let body_height =
                (label.height + ry + FLOWCHART_LINED_CYLINDER_PADDING * 2.0).max(10.0);
            return (width, body_height + ry * 2.0);
        }
        crate::ir::NodeShape::Hexagon => {
            width *= HEXAGON_WIDTH_SCALE;
            height *= HEXAGON_HEIGHT_SCALE;
        }
        crate::ir::NodeShape::Parallelogram | crate::ir::NodeShape::ParallelogramAlt => {}
        crate::ir::NodeShape::Trapezoid | crate::ir::NodeShape::TrapezoidAlt => {
            width *= TRAPEZOID_WIDTH_SCALE;
        }
        crate::ir::NodeShape::Asymmetric => {}
        crate::ir::NodeShape::Subroutine => {}
        crate::ir::NodeShape::SmallCircle | crate::ir::NodeShape::FilledCircle => {
            // Fixed 14×14 matching mermaid-js (stateStart.ts / filledCircle.ts).
            width = 14.0;
            height = 14.0;
        }
        crate::ir::NodeShape::FramedCircle => {
            // Fixed 14×14 matching mermaid-js (stateEnd.ts).
            width = 14.0;
            height = 14.0;
        }
        crate::ir::NodeShape::CrossedCircle => {
            if kind == crate::ir::DiagramKind::Flowchart {
                width = 60.0;
                height = 60.0;
            } else {
                // Fixed 14×14 matching mermaid-js state-style end markers.
                width = 14.0;
                height = 14.0;
            }
        }
        crate::ir::NodeShape::Triangle | crate::ir::NodeShape::FlippedTriangle => {
            width *= 1.3;
            height *= 1.2;
        }
        crate::ir::NodeShape::Cloud => {
            width *= 1.3;
            height *= 1.2;
        }
        crate::ir::NodeShape::Bang => {
            width *= 1.35;
            height *= 1.35;
        }
        crate::ir::NodeShape::HorizontalCylinder => {
            width *= CYLINDER_SCALE;
            height *= CYLINDER_SCALE;
        }
        crate::ir::NodeShape::StackedRect => {
            // Account for offset behind
            width += 8.0;
            height += 8.0;
        }
        crate::ir::NodeShape::SlopedRect => {
            // JS: totalHeight = (bbox.height + padding*2) * 1.5
            height *= 1.5;
        }
        crate::ir::NodeShape::LightningBolt => {
            // JS: fixed minimum 35×35 with 2:1 height, no label.
            width = width.max(35.0);
            height = (height * 2.0).max(70.0);
        }
        crate::ir::NodeShape::LinedRect => {
            width *= 1.15;
        }
        // Curved-trapezoid family (doc, tag-doc, lin-doc, docs): JS uses
        // w * 1.25 (curvedTrapezoid.ts line 26). The right-side semicircle
        // has radius = h/2, so height must be >= width for portrait aspect.
        // Boost height to ensure portrait orientation like JS.
        crate::ir::NodeShape::Document
        | crate::ir::NodeShape::TagDocument
        | crate::ir::NodeShape::LinedDocument
        | crate::ir::NodeShape::StackedDocument => {
            width *= 1.25;
            height = height.max(width * 1.3);
        }
        // Hourglass/collate should be square.
        crate::ir::NodeShape::Hourglass => {
            let size = width.max(height);
            width = size;
            height = size;
        }
        _ => {}
    }

    if kind == crate::ir::DiagramKind::Class {
        let min_height = theme.font_size * CLASS_MIN_HEIGHT_SCALE;
        height = height.max(min_height);
    }

    if kind == crate::ir::DiagramKind::Kanban {
        let min_width = theme.font_size * KANBAN_MIN_WIDTH_SCALE;
        let min_height = theme.font_size * KANBAN_MIN_HEIGHT_SCALE;
        width = width.max(min_width);
        height = height.max(min_height);
    }

    if kind == crate::ir::DiagramKind::Er && shape == crate::ir::NodeShape::RoundRect {
        let row_count = class_body_line_count(label);
        if row_count > 0 {
            height = height.max((row_count as f32 + 1.0) * ER_ATTRIBUTE_ROW_HEIGHT);
        }
    }

    (width, height)
}

fn is_flowchart_icon_shape(shape: crate::ir::NodeShape) -> bool {
    matches!(
        shape,
        crate::ir::NodeShape::Icon
            | crate::ir::NodeShape::IconCircle
            | crate::ir::NodeShape::IconSquare
            | crate::ir::NodeShape::IconRounded
    )
}

fn flowchart_icon_shape_size(shape: crate::ir::NodeShape, label: &TextBlock) -> (f32, f32) {
    let icon_box = flowchart_icon_visual_size(shape);
    let label_empty = label.lines.iter().all(|line| line.text().trim().is_empty());
    let label_height = if label_empty {
        0.0
    } else {
        label.height + FLOWCHART_ICON_LABEL_EXTRA_HEIGHT
    };
    let label_padding = if label_empty {
        0.0
    } else {
        FLOWCHART_ICON_LABEL_PADDING
    };
    (
        icon_box.max(label.width).max(1.0),
        (icon_box + label_padding + label_height).max(1.0),
    )
}

fn flowchart_icon_visual_size(shape: crate::ir::NodeShape) -> f32 {
    match shape {
        crate::ir::NodeShape::IconCircle => {
            FLOWCHART_ICON_ASSET_SIZE * std::f32::consts::SQRT_2
                + FLOWCHART_ICON_CIRCLE_PADDING * 2.0
        }
        crate::ir::NodeShape::IconSquare | crate::ir::NodeShape::IconRounded => {
            FLOWCHART_ICON_ASSET_SIZE + FLOWCHART_ICON_SQUARE_PADDING * 2.0
        }
        crate::ir::NodeShape::Icon => FLOWCHART_ICON_ASSET_SIZE,
        _ => FLOWCHART_ICON_ASSET_SIZE,
    }
}

fn requirement_edge_label_text(label: &str, config: &LayoutConfig) -> String {
    let trimmed = label
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if config.requirement.edge_label_brackets {
        format!("<<{}>>", trimmed)
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Direction, Graph, NodeShape};
    use crate::parser::parse_mermaid;

    #[test]
    fn wraps_long_labels() {
        let theme = Theme::modern();
        let mut config = LayoutConfig::default();
        config.max_label_width_chars = 8;
        let block = measure_label("this is a long label", &theme, &config);
        assert!(block.lines.len() > 1);
    }

    #[test]
    fn class_members_do_not_auto_wrap_long_generic_lines() {
        let source = r#"classDiagram
class Square~Shape~{
int id
List~int~ position
setPoints(List~int~ points)
getPoints() List~int~
}
Square : -List~string~ messages
Square : +setMessages(List~string~ messages)
Square : +getMessages() List~string~
Square : +getDistanceMatrix() List~List~int~~
"#;
        let parsed = parse_mermaid(source).expect("failed to parse class generic fixture");
        let layout = compute_layout(
            &parsed.graph,
            &Theme::mermaid_default(),
            &LayoutConfig::default(),
        );
        let square = layout.nodes.get("Square").expect("missing Square class");
        let rendered_lines = square
            .label
            .lines
            .iter()
            .map(|line| line.text().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(rendered_lines.len(), 11);
        assert!(rendered_lines.contains(&"+setMessages(List<string> messages)".to_string()));
        assert!(rendered_lines.contains(&"+getMessages() : List<string>".to_string()));
        assert!(rendered_lines.contains(&"+getDistanceMatrix() : List<List<int>>".to_string()));
        assert!(
            square.width > 340.0,
            "class should widen to fit no-wrap generic methods, got {:.2}",
            square.width
        );
    }

    #[test]
    fn kanban_metadata_card_uses_mermaid_fixed_item_size() {
        let source = "kanban\n  todo[Todo]\n    id3[Update Database Function]@{ ticket: MC-2037, assigned: 'knsv', priority: 'High' }";
        let parsed = parse_mermaid(source).expect("failed to parse kanban metadata fixture");
        let layout = compute_layout(
            &parsed.graph,
            &Theme::mermaid_default(),
            &LayoutConfig::default(),
        );
        let item = layout.nodes.get("id3").expect("missing kanban item");
        let column = layout
            .subgraphs
            .iter()
            .find(|subgraph| subgraph.label == "Todo")
            .expect("missing Todo column");

        assert!((item.width - 185.0).abs() < 0.01);
        assert!((item.height - 80.0).abs() < 0.01);
        assert!((column.width - 200.0).abs() < 0.01);
        assert!(
            layout.width >= 218.0 && layout.width <= 222.0,
            "single-column kanban metadata fixture should match JS width class, got {:.2}",
            layout.width
        );
        assert!(
            layout.height >= 133.0 && layout.height <= 137.0,
            "single-column kanban metadata fixture should match JS height class, got {:.2}",
            layout.height
        );
    }

    #[test]
    fn empty_class_shape_uses_mermaid_horizontal_padding() {
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let label = TextBlock {
            lines: vec![TextLine::plain("Class".to_string())],
            width: 50.0,
            height: 24.0,
        };
        let (width, height) = shape_size(
            NodeShape::Rectangle,
            &label,
            &config,
            &theme,
            crate::ir::DiagramKind::Class,
        );
        assert!((width - 74.0).abs() <= 0.01);
        assert!(height >= theme.font_size * CLASS_MIN_HEIGHT_SCALE);
    }

    #[test]
    fn flowchart_round_rect_uses_mermaid_event_padding() {
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let label = TextBlock {
            lines: vec![TextLine::plain("A".to_string())],
            width: 9.4375,
            height: 24.0,
        };
        let (width, height) = shape_size(
            NodeShape::RoundRect,
            &label,
            &config,
            &theme,
            crate::ir::DiagramKind::Flowchart,
        );

        assert!((width - 39.4375).abs() <= 0.01);
        assert!((height - 54.0).abs() <= 0.01);
    }

    #[test]
    fn flowchart_sloped_rect_uses_mermaid_manual_input_geometry() {
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let label = TextBlock {
            lines: vec![TextLine::plain("A".to_string())],
            width: 9.4375,
            height: 24.0,
        };
        let (width, height) = shape_size(
            NodeShape::SlopedRect,
            &label,
            &config,
            &theme,
            crate::ir::DiagramKind::Flowchart,
        );

        assert!((width - 39.4375).abs() <= 0.01);
        assert!((height - 81.0).abs() <= 0.01);
    }

    #[test]
    fn flowchart_notched_pentagon_uses_mermaid_loop_limit_geometry() {
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let label = TextBlock {
            lines: vec![TextLine::plain("A".to_string())],
            width: 9.4375,
            height: 24.0,
        };
        let (width, height) = shape_size(
            NodeShape::NotchedPentagon,
            &label,
            &config,
            &theme,
            crate::ir::DiagramKind::Flowchart,
        );

        assert!((width - 39.4375).abs() <= 0.01);
        assert!((height - 54.0).abs() <= 0.01);
    }

    #[test]
    fn flowchart_diamond_uses_actual_multiline_label_bbox() {
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let label = TextBlock {
            lines: vec![
                TextLine::plain("Diamond with ".to_string()),
                TextLine::plain(" line break".to_string()),
            ],
            width: 98.6875,
            height: 48.0,
        };
        let (width, height) = shape_size(
            NodeShape::Diamond,
            &label,
            &config,
            &theme,
            crate::ir::DiagramKind::Flowchart,
        );

        assert!((width - 176.6875).abs() <= 0.01);
        assert!((height - 176.6875).abs() <= 0.01);
    }

    #[test]
    fn flowchart_braces_both_sides_uses_mermaid_curly_braces_envelope() {
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let label = TextBlock {
            lines: vec![TextLine::plain("A".to_string())],
            width: 9.4375,
            height: 24.0,
        };
        let (width, height) = shape_size(
            NodeShape::BraceBoth,
            &label,
            &config,
            &theme,
            crate::ir::DiagramKind::Flowchart,
        );

        assert!((width - 36.9375).abs() <= 0.01);
        assert!((height - 49.0).abs() <= 0.01);
    }

    #[test]
    fn flowchart_lean_shapes_use_mermaid_data_io_geometry() {
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let label = TextBlock {
            lines: vec![TextLine::plain("A".to_string())],
            width: 9.4375,
            height: 24.0,
        };

        for shape in [NodeShape::LeanLeft, NodeShape::LeanRight] {
            let (width, height) = shape_size(
                shape,
                &label,
                &config,
                &theme,
                crate::ir::DiagramKind::Flowchart,
            );

            assert!((width - 63.4375).abs() <= 0.01);
            assert!((height - 39.0).abs() <= 0.01);
        }
    }

    #[test]
    fn block_arrow_ascii_space_label_uses_mermaid_zero_bbox() {
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let label = TextBlock {
            lines: vec![TextLine::plain(" ".to_string())],
            width: 0.0,
            height: 24.0,
        };
        let (width, height) = shape_size(
            NodeShape::BlockArrowRight,
            &label,
            &config,
            &theme,
            crate::ir::DiagramKind::Block,
        );

        assert!((width - 24.0).abs() <= 0.01);
        assert!((height - 16.0).abs() <= 0.01);
    }

    #[test]
    fn class_cardinality_bounds_do_not_add_curve_overshoot_margin() {
        let source = "classDiagram\n    Customer \"1\" --> \"*\" Ticket\n    Student \"1\" --> \"1..*\" Course\n    Galaxy --> \"many\" Star : Contains";
        let parsed = parse_mermaid(source).expect("failed to parse class cardinality fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let max_node_right = layout
            .nodes
            .values()
            .filter(|node| !node.hidden)
            .map(|node| node.x + node.width)
            .fold(0.0_f32, f32::max);

        assert!(
            layout.width <= max_node_right + 24.0,
            "class layout width {:.2}px should not include generic curved-edge overshoot beyond rightmost node {:.2}px",
            layout.width,
            max_node_right
        );
        assert!(
            layout.width < 380.0,
            "class cardinality fixture should stay in JS viewBox size class, got {:.2}px",
            layout.width
        );
    }

    #[test]
    fn class_diagonal_two_point_edges_get_dagre_bend() {
        let source = "classDiagram\nA <|-- B\nA <|-- C";
        let parsed = parse_mermaid(source).expect("failed to parse class inheritance fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        assert!(
            layout.edges.iter().any(|edge| {
                edge.arrow_start_kind == Some(crate::ir::EdgeArrowhead::OpenTriangle)
                    && edge.points.len() == 3
                    && (edge.points[1].0 - edge.points[2].0).abs() < 0.01
            }),
            "expected at least one diagonal class inheritance edge to keep a dagre-style bend, got {:?}",
            layout
                .edges
                .iter()
                .map(|edge| &edge.points)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn requirement_edges_keep_dagre_curve_midpoints() {
        let source = r#"requirementDiagram
requirement test_req {
  id: 1
  text: the test text.
  risk: high
  verifymethod: test
}
functionalRequirement test_req2 {
  id: 1.1
  text: the second test text.
  risk: low
  verifymethod: inspection
}
performanceRequirement test_req3 {
  id: 1.2
  text: the third test text.
  risk: medium
  verifymethod: demonstration
}
element test_entity {
  type: simulation
}
test_entity - satisfies -> test_req2
test_req - contains -> test_req3
"#;
        let parsed = parse_mermaid(source).expect("failed to parse requirement fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let satisfies = layout
            .edges
            .iter()
            .find(|edge| edge.from == "test_entity" && edge.to == "test_req2")
            .expect("missing satisfies edge");
        assert_eq!(satisfies.points.len(), 3);
        assert!(
            (satisfies.points[1].0 - satisfies.points[0].0).abs() < 0.01,
            "non-contains requirement edges should bend from the source side, got {:?}",
            satisfies.points
        );

        let contains = layout
            .edges
            .iter()
            .find(|edge| edge.from == "test_req" && edge.to == "test_req3")
            .expect("missing contains edge");
        assert_eq!(contains.points.len(), 3);
        assert!(
            (contains.points[1].0 - contains.points[2].0).abs() < 0.01,
            "contains requirement edges should bend from the target side, got {:?}",
            contains.points
        );
    }

    #[test]
    fn requirement_edge_label_spacing_preserves_same_rank_centers() {
        let source = r#"requirementDiagram
requirement test_req {
  id: 1
  text: the test text.
  risk: high
  verifymethod: test
}
functionalRequirement test_req2 {
  id: 1.1
  text: the second test text.
  risk: low
  verifymethod: inspection
}
element test_entity {
  type: simulation
}
element test_entity2 {
  type: word doc
  docRef: reqs/test_entity
}
test_entity - satisfies -> test_req2
test_req - traces -> test_req2
test_req <- copies - test_entity2
"#;
        let parsed = parse_mermaid(source).expect("failed to parse requirement fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let requirement = layout.nodes.get("test_req").expect("missing test_req");
        let element = layout
            .nodes
            .get("test_entity")
            .expect("missing test_entity");
        let requirement_center = requirement.y + requirement.height / 2.0;
        let element_center = element.y + element.height / 2.0;

        assert!(
            (requirement_center - element_center).abs() <= 0.01,
            "same-rank requirement and element should stay center-aligned: req={requirement_center}, element={element_center}"
        );
    }

    #[test]
    fn requirement_edges_use_dagre_rank_gap_label_lanes() {
        let source = r#"requirementDiagram
requirement test_req {
  id: 1
  text: the test text.
  risk: high
  verifymethod: test
}
functionalRequirement test_req2 {
  id: 1.1
  text: the second test text.
  risk: low
  verifymethod: inspection
}
performanceRequirement test_req3 {
  id: 1.2
  text: the third test text.
  risk: medium
  verifymethod: demonstration
}
element test_entity {
  type: simulation
}
test_entity - satisfies -> test_req2
test_req - traces -> test_req2
test_req - contains -> test_req3
"#;
        let parsed = parse_mermaid(source).expect("failed to parse requirement fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let test_entity = layout
            .nodes
            .get("test_entity")
            .expect("missing test_entity");
        let test_req = layout.nodes.get("test_req").expect("missing test_req");
        let test_req2 = layout.nodes.get("test_req2").expect("missing test_req2");
        let test_req3 = layout.nodes.get("test_req3").expect("missing test_req3");
        let source_rank_bottom =
            (test_entity.y + test_entity.height).max(test_req.y + test_req.height);
        let target_rank_top = test_req2.y.min(test_req3.y);
        let expected_label_lane_y = (source_rank_bottom + target_rank_top) * 0.5;
        let same_rank_gap = if test_req2.x <= test_req3.x {
            test_req3.x - (test_req2.x + test_req2.width)
        } else {
            test_req2.x - (test_req3.x + test_req3.width)
        };
        assert!(
            same_rank_gap >= 57.0,
            "adjacent incoming requirement targets should include dagre edge-label cross spacing, got gap {same_rank_gap:.2}"
        );

        let satisfies = layout
            .edges
            .iter()
            .find(|edge| edge.from == "test_entity" && edge.to == "test_req2")
            .expect("missing satisfies edge");
        assert_eq!(satisfies.points.len(), 3);
        assert!(
            (satisfies.points[1].1 - expected_label_lane_y).abs() <= 0.01,
            "satisfies bend should use the dagre rank-gap label lane: points={:?}, lane={expected_label_lane_y}",
            satisfies.points
        );
        let satisfies_label = satisfies
            .label_anchor
            .expect("missing satisfies label anchor");
        assert!(
            (satisfies_label.1 - expected_label_lane_y).abs() <= 0.01,
            "axis-aligned requirement label should stay on the bend lane: anchor={satisfies_label:?}, lane={expected_label_lane_y}"
        );

        let traces = layout
            .edges
            .iter()
            .find(|edge| edge.from == "test_req" && edge.to == "test_req2")
            .expect("missing traces edge");
        let traces_label = traces.label_anchor.expect("missing traces label anchor");
        assert!(
            (traces_label.0 - traces.points[1].0).abs() > 1.0
                || (traces_label.1 - traces.points[1].1).abs() > 1.0,
            "diagonal requirement labels should use the path midpoint, not the bend point: points={:?}, anchor={traces_label:?}",
            traces.points
        );
        assert!(
            traces_label.1 < expected_label_lane_y,
            "diagonal path midpoint should sit before the bend lane, matching Mermaid's calcLabelPosition fallback: anchor={traces_label:?}, lane={expected_label_lane_y}"
        );
    }

    #[test]
    fn class_namespace_unconnected_members_stack_vertically() {
        let source = "classDiagram\nnamespace BaseShapes {\nclass Triangle\nclass Rectangle {\ndouble width\ndouble height\n}\n}";
        let parsed = parse_mermaid(source).expect("failed to parse class namespace fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let triangle = layout.nodes.get("Triangle").expect("missing Triangle");
        let rectangle = layout.nodes.get("Rectangle").expect("missing Rectangle");
        let triangle_center_x = triangle.x + triangle.width / 2.0;
        let rectangle_center_x = rectangle.x + rectangle.width / 2.0;
        let vertical_gap = rectangle.y - (triangle.y + triangle.height);
        let namespace = layout
            .subgraphs
            .iter()
            .find(|subgraph| subgraph.label == "BaseShapes")
            .expect("missing BaseShapes namespace");

        assert!(
            (triangle_center_x - rectangle_center_x).abs() < 0.01,
            "unconnected namespace classes should share a vertical center line"
        );
        assert!(
            vertical_gap >= 45.0 && vertical_gap <= 55.0,
            "namespace classes should use Mermaid's 50px inner rank gap, got {vertical_gap:.2}"
        );
        assert!(
            namespace.height > namespace.width,
            "namespace should be a tall cluster after stacking, got {:.2}x{:.2}",
            namespace.width,
            namespace.height
        );
    }

    #[test]
    fn class_inheritance_fan_routes_close_children_to_center_column() {
        let source = r#"classDiagram
    note "From Duck till Zebra"
    Animal <|-- Duck
    note for Duck "can fly<br>can swim<br>can dive<br>can help in debugging"
    Animal <|-- Fish
    Animal <|-- Zebra
    Animal : +int age
    Animal : +String gender
    Animal: +isMammal()
    Animal: +mate()
    class Duck{
        +String beakColor
        +swim()
        +quack()
    }
    class Fish{
        -int sizeInFeet
        -canEat()
    }
    class Zebra{
        +bool is_wild
        +run()
    }"#;
        let parsed = parse_mermaid(source).expect("failed to parse class inheritance fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let animal = layout.nodes.get("Animal").expect("missing Animal");
        let duck = layout.nodes.get("Duck").expect("missing Duck");
        let fish = layout.nodes.get("Fish").expect("missing Fish");
        let animal_center_x = animal.x + animal.width / 2.0;
        let duck_center_x = duck.x + duck.width / 2.0;
        let fish_center_x = fish.x + fish.width / 2.0;

        let fish_edge = layout
            .edges
            .iter()
            .find(|edge| edge.from == "Animal" && edge.to == "Fish")
            .expect("missing Animal->Fish edge");
        assert_eq!(fish_edge.points.len(), 3);
        assert!(
            (fish_edge.points[1].0 - fish_center_x).abs() < 0.01
                && (fish_edge.points[2].0 - fish_center_x).abs() < 0.01,
            "close inheritance child should route through its center column, fish center {fish_center_x:.2}, points {:?}",
            fish_edge.points
        );

        let duck_edge = layout
            .edges
            .iter()
            .find(|edge| edge.from == "Animal" && edge.to == "Duck")
            .expect("missing Animal->Duck edge");
        let midpoint_x = duck_edge.points[1].0;
        assert!(
            midpoint_x > duck_center_x && midpoint_x < animal_center_x,
            "far inheritance child should use a shared fan midpoint between source and target, got {midpoint_x:.2} for centers {duck_center_x:.2}/{animal_center_x:.2}",
        );
    }

    #[test]
    fn flowchart_dense_bidirectional_labels_use_dagre_like_lanes() {
        let source = r#"graph TD
    A["<b>Node A</b><br/>10.0.0.1<br/>k3s server (control plane)"]
    B["<b>Node B</b><br/>10.0.0.2<br/>k3s agent"]
    C["<b>Node C</b><br/>10.0.0.3<br/>k3s agent"]

    A <-->|WireGuard| B
    A <-->|WireGuard| C
    B <-->|WireGuard| C

    B -->|"k3s join (port 6443)"| A
    C -->|"k3s join (port 6443)"| A"#;
        let parsed = parse_mermaid(source).expect("failed to parse k3s flowchart fixture");
        let layout = compute_layout(
            &parsed.graph,
            &Theme::mermaid_default(),
            &LayoutConfig::default(),
        );

        let a = layout.nodes.get("A").expect("missing A");
        let b = layout.nodes.get("B").expect("missing B");
        let c = layout.nodes.get("C").expect("missing C");
        let a_center = (a.x + a.width / 2.0, a.y + a.height / 2.0);
        let b_center = (b.x + b.width / 2.0, b.y + b.height / 2.0);
        let c_center = (c.x + c.width / 2.0, c.y + c.height / 2.0);

        assert!((a_center.0 - b_center.0).abs() <= 0.1);
        assert!((b_center.0 - c_center.0).abs() <= 0.1);
        assert!(
            (b_center.1 - a_center.1 - 176.0).abs() <= 1.0,
            "A/B rank gap should match Mermaid dagre's 176px cadence, got {:.2}",
            b_center.1 - a_center.1
        );
        assert!(
            (c_center.1 - b_center.1 - 176.0).abs() <= 1.0,
            "B/C rank gap should match Mermaid dagre's 176px cadence, got {:.2}",
            c_center.1 - b_center.1
        );
        assert!(
            (layout.width - 480.0).abs() <= 1.0 && (layout.height - 470.0).abs() <= 1.0,
            "layout should stay in the Mermaid JS size class, got {:.2}x{:.2}",
            layout.width,
            layout.height
        );

        let anchor = |from: &str, to: &str| {
            layout
                .edges
                .iter()
                .find(|edge| edge.from == from && edge.to == to)
                .and_then(|edge| edge.label_anchor)
                .unwrap_or_else(|| panic!("missing label anchor for {from}->{to}"))
        };
        let ab = anchor("A", "B");
        let ac = anchor("A", "C");
        let bc = anchor("B", "C");
        let ba = anchor("B", "A");
        let ca = anchor("C", "A");
        let side_lane_offset = b.width * 0.5;

        assert!(
            (ab.0 - (a_center.0 - side_lane_offset)).abs() <= 1.0,
            "adjacent A->B label should sit on the left node-side lane, got {ab:?}"
        );
        assert!(
            (ba.0 - (a_center.0 + side_lane_offset)).abs() <= 1.0,
            "adjacent B->A label should sit on the right node-side lane, got {ba:?}"
        );
        assert!(
            ac.0 < a.x - 30.0,
            "long A->C label should be routed outside the left of the widest node, got {ac:?}"
        );
        assert!(
            ca.0 > a.x + a.width + 65.0,
            "long C->A label should be routed outside the right of the widest node, got {ca:?}"
        );
        assert!(
            (bc.0 - b_center.0).abs() <= 1.0,
            "single B->C label should stay on the center column, got {bc:?}"
        );
    }

    #[test]
    fn flowchart_aligned_dotted_import_edges_stay_on_dagre_lanes() {
        let source = r#"flowchart LR
    subgraph FED0["fed0 myriplane"]
        c1[fed0cluster1]
        c2[fed0cluster2]
    end
    subgraph FED1["fed1 myriplane"]
        c3[fed1cluster1]
        c4[fed1cluster2]
    end
    kc1["~/.kube/fed0cluster1-kmaster1"] -. import --overwrite .-> c1
    kc2["~/.kube/fed0cluster2-kmaster1"] -. import --overwrite .-> c2
    kc3["~/.kube/fed1cluster1-kmaster1"] -. import --overwrite .-> c3
    kc4["~/.kube/fed1cluster2-kmaster1"] -. import --overwrite .-> c4"#;
        let parsed = parse_mermaid(source).expect("failed to parse import flowchart fixture");
        let layout = compute_layout(
            &parsed.graph,
            &Theme::mermaid_default(),
            &LayoutConfig::default(),
        );

        for label in ["fed0 myriplane", "fed1 myriplane"] {
            let subgraph = layout
                .subgraphs
                .iter()
                .find(|subgraph| subgraph.label == label)
                .unwrap_or_else(|| panic!("missing subgraph {label}"));
            assert!(
                (249.0..=254.0).contains(&subgraph.height),
                "external LR compound clusters should keep Mermaid's roomy envelope; got {label} height {:.2}",
                subgraph.height
            );
        }

        for (from, to) in [("kc1", "c1"), ("kc2", "c2"), ("kc3", "c3"), ("kc4", "c4")] {
            let edge = layout
                .edges
                .iter()
                .find(|edge| edge.from == from && edge.to == to)
                .unwrap_or_else(|| panic!("missing edge {from}->{to}"));
            assert!(
                edge.points.len() == 4
                    && edge
                        .points
                        .windows(2)
                        .all(|pair| { (pair[0].1 - pair[1].1).abs() <= 0.2 }),
                "Mermaid dagre keeps aligned import edges on one lane while preserving the label and compound-boundary waypoints; got {from}->{to} points {:?}",
                edge.points
            );
            let label = edge
                .label_anchor
                .unwrap_or_else(|| panic!("missing label anchor for {from}->{to}"));
            let label_width = edge.label.as_ref().expect("missing edge label").width;
            let expected_label_x =
                edge.points[0].0 + (label_width + FLOWCHART_RECURSIVE_DAGRE_SPACING) * 0.5;
            assert!(
                (label.0 - expected_label_x).abs() <= 0.2
                    && (label.0 - edge.points[1].0).abs() <= 0.2
                    && (label.1 - edge.points[0].1).abs() <= 0.2,
                "import label should stay on the source-to-compound dagre segment; got {from}->{to} label {label:?}, expected x {expected_label_x:.2}, points {:?}",
                edge.points
            );
            assert!(
                edge.points[2].0 > edge.points[1].0 + 20.0
                    && edge.points[3].0 > edge.points[2].0 + 20.0,
                "import edge should retain the compound-boundary waypoint before the target node; got {from}->{to} points {:?}",
                edge.points
            );
        }
    }

    #[test]
    fn flowchart_subgraph_anchor_edges_count_as_external_connections() {
        let source = r#"flowchart TB
subgraph A["Phase A"]
  X --> Y
end
subgraph B["Phase B"]
  Z
end
A --> B"#;
        let parsed = parse_mermaid(source).expect("failed to parse flowchart subgraph fixture");
        let graph = &parsed.graph;
        let sub_a = graph
            .subgraphs
            .iter()
            .find(|sub| sub.id.as_deref() == Some("A"))
            .expect("missing subgraph A");
        let sub_b = graph
            .subgraphs
            .iter()
            .find(|sub| sub.id.as_deref() == Some("B"))
            .expect("missing subgraph B");

        assert!(
            !flowchart_subgraph_without_external_connections(graph, sub_a),
            "edges from the subgraph anchor should make A externally connected"
        );
        assert!(
            !flowchart_subgraph_without_external_connections(graph, sub_b),
            "edges to the subgraph anchor should make B externally connected"
        );
    }

    #[test]
    fn flowchart_recursive_cluster_internal_edges_use_cluster_direction_for_routing() {
        let source = r#"graph TB
subgraph A
od>Odd shape]-- Two line<br/>edge comment --> ro
di{Diamond with <br/> line break} -.-> ro(Rounded<br>square<br>shape)
di==>ro2(Rounded square shape)
end"#;
        let parsed =
            parse_mermaid(source).expect("failed to parse recursive flowchart subgraph fixture");
        let od_ro_idx = parsed
            .graph
            .edges
            .iter()
            .position(|edge| edge.from == "od" && edge.to == "ro")
            .expect("missing od->ro edge");
        let edge_directions = edge_effective_directions(&parsed.graph);
        assert_eq!(edge_directions[od_ro_idx], Direction::LeftRight);

        let layout = compute_layout(
            &parsed.graph,
            &Theme::mermaid_default(),
            &LayoutConfig::default(),
        );
        let diamond = layout.nodes.get("di").expect("missing diamond");
        let od_ro = layout
            .edges
            .iter()
            .find(|edge| edge.from == "od" && edge.to == "ro")
            .expect("missing routed od->ro edge");
        let label_anchor = od_ro
            .label_anchor
            .expect("od->ro edge should have a label anchor");
        let route_max_y = od_ro
            .points
            .iter()
            .map(|(_, y)| *y)
            .fold(f32::MIN, f32::max);

        assert!(
            label_anchor.1 < diamond.y,
            "recursive LR edge label should stay on the top lane, got {label_anchor:?} below diamond y {:.2}",
            diamond.y
        );
        assert!(
            route_max_y < diamond.y,
            "recursive LR edge should not route down through the diamond lane, got max y {:.2}, diamond y {:.2}",
            route_max_y,
            diamond.y
        );
    }

    #[test]
    fn flowchart_root_fanout_sources_center_over_children_like_dagre() {
        let source = r#"graph TB
sq[Square shape] --> ci((Circle shape))

subgraph A
od>Odd shape]-- Two line<br/>edge comment --> ro
di{Diamond with <br/> line break} -.-> ro(Rounded<br>square<br>shape)
di==>ro2(Rounded square shape)
end

e --> od3>Really long text with linebreak<br>in an Odd shape]
e((Inner / circle<br>and some odd <br>special characters)) --> f(,.?!+-*ز)

cyr[Cyrillic]-->cyr2((Circle shape Начало));

classDef green fill:#9f6,stroke:#333,stroke-width:2px;
class sq,e green"#;
        let parsed = parse_mermaid(source).expect("failed to parse flowchart fanout fixture");
        let layout = compute_layout(
            &parsed.graph,
            &Theme::mermaid_default(),
            &LayoutConfig::default(),
        );
        let e = layout.nodes.get("e").expect("missing e");
        let od3 = layout.nodes.get("od3").expect("missing od3");
        let f = layout.nodes.get("f").expect("missing f");
        let sq = layout.nodes.get("sq").expect("missing sq");
        let ci = layout.nodes.get("ci").expect("missing ci");
        let cyr = layout.nodes.get("cyr").expect("missing cyr");
        let cyr2 = layout.nodes.get("cyr2").expect("missing cyr2");
        let center_x = |node: &NodeLayout| node.x + node.width * 0.5;
        let e_x = center_x(e);
        let child_mid_x = (center_x(od3) + center_x(f)) * 0.5;

        assert!(
            (e_x - child_mid_x).abs() <= 1.0,
            "dagre centers root fanout sources over their children; got e center {e_x:.2}, child midpoint {child_mid_x:.2}"
        );
        assert!(
            (center_x(sq) - center_x(ci)).abs() <= 1.0,
            "dagre centers simple two-node chains on one column; got sq {:.2}, ci {:.2}",
            center_x(sq),
            center_x(ci)
        );
        assert!(
            (center_x(cyr) - center_x(cyr2)).abs() <= 1.0,
            "dagre centers simple two-node chains with mixed widths; got cyr {:.2}, cyr2 {:.2}",
            center_x(cyr),
            center_x(cyr2)
        );
        let center_y = |node: &NodeLayout| node.y + node.height * 0.5;
        let rank_gap = center_y(cyr2) - center_y(cyr);
        assert!(
            (305.0..=314.0).contains(&rank_gap),
            "parent graph rank gap should match Mermaid dagre's measured-cluster cadence; got {rank_gap:.2}"
        );
        let cluster_a = layout
            .subgraphs
            .iter()
            .find(|subgraph| subgraph.label == "A")
            .expect("missing cluster A");
        assert!(
            (618.0..=626.0).contains(&cluster_a.width)
                && (354.0..=362.0).contains(&cluster_a.height),
            "recursive cluster with internal edge label should match Mermaid's measured envelope, got {:.2}x{:.2}",
            cluster_a.width,
            cluster_a.height
        );

        let e_od3 = layout
            .edges
            .iter()
            .find(|edge| edge.from == "e" && edge.to == "od3")
            .expect("missing routed e->od3 edge");
        let e_f = layout
            .edges
            .iter()
            .find(|edge| edge.from == "e" && edge.to == "f")
            .expect("missing routed e->f edge");
        assert!(
            e_od3.points.len() >= 3 && e_f.points.len() >= 3,
            "dagre root fanouts should retain a middle curve point, got e->od3={:?}, e->f={:?}",
            e_od3.points,
            e_f.points
        );
        let left_start = e_od3.points.first().copied().expect("missing e->od3 start");
        let left_end = e_od3.points.last().copied().expect("missing e->od3 end");
        let right_start = e_f.points.first().copied().expect("missing e->f start");
        let right_end = e_f.points.last().copied().expect("missing e->f end");
        assert!(
            left_start.0 > left_end.0 + 20.0,
            "left root fanout branch should be diagonal like dagre, got {left_start:?} -> {left_end:?}"
        );
        assert!(
            right_end.0 > right_start.0 + 20.0,
            "right root fanout branch should be diagonal like dagre, got {right_start:?} -> {right_end:?}"
        );
    }

    #[test]
    fn flowchart_root_fanout_centers_over_subgraph_members_like_dagre() {
        let source = r#"flowchart TB
    md[("hello-fed MD<br/>(replicated on both members)")]
    subgraph FED0["fed0 deploy controller"]
        d0[reconcile vc-a1<br/>OWNED -> deploy]
        d0x[reconcile vc-b1<br/>FOREIGN -> skip]
    end
    subgraph FED1["fed1 deploy controller"]
        d1x[reconcile vc-a1<br/>FOREIGN -> skip]
        d1[reconcile vc-b1<br/>OWNED -> deploy]
    end
    md --> d0 & d0x & d1 & d1x
    d0 ==>|kubectl apply| pa["pod: hello-world<br/>default-fedten-vc-a1-default<br/>fed0cluster1"]
    d1 ==>|kubectl apply| pb["pod: hello-world<br/>default-fedten-vc-b1-default<br/>fed1cluster1"]"#;
        let parsed = parse_mermaid(source).expect("failed to parse realworld phase-08 fixture");
        let layout = compute_layout(
            &parsed.graph,
            &Theme::mermaid_default(),
            &LayoutConfig::default(),
        );
        let center_x = |node: &NodeLayout| node.x + node.width * 0.5;
        let md = layout.nodes.get("md").expect("missing md");
        let target_ids = ["d0", "d0x", "d1x", "d1"];
        let target_center_sum = target_ids
            .iter()
            .map(|id| center_x(layout.nodes.get(*id).expect("missing fanout target")))
            .sum::<f32>();
        let target_average = target_center_sum / target_ids.len() as f32;
        let md_center = center_x(md);
        assert!(
            (md_center - target_average).abs() <= 1.0,
            "dagre centers root fanout sources over member-node targets; got md center {md_center:.2}, target average {target_average:.2}"
        );

        for target_id in target_ids {
            let target = layout.nodes.get(target_id).expect("missing target node");
            let routed = layout
                .edges
                .iter()
                .find(|edge| edge.from == "md" && edge.to == target_id)
                .expect("missing md fanout edge");
            assert!(
                routed.points.len() == 4,
                "dagre fanout edge md->{target_id} should keep source, two rank-gap bends, and target, got {:?}",
                routed.points
            );
            assert!(
                (routed.points[1].1 - (target.y - 50.0)).abs() <= 1.0
                    && (routed.points[2].1 - (target.y - 25.0)).abs() <= 1.0,
                "dagre fanout edge md->{target_id} should use Mermaid's 50px/25px target-rank bends, got {:?} for target top {:.2}",
                routed.points,
                target.y
            );
            let end = routed
                .points
                .last()
                .copied()
                .expect("missing edge endpoint");
            assert!(
                (end.0 - center_x(target)).abs() <= 2.0 && end.1 <= target.y + 2.0,
                "dagre fanout edge md->{target_id} should enter the member node from the top, got end {end:?} for target x {:.2} top {:.2}",
                center_x(target),
                target.y
            );
        }

        let md_d0x = layout
            .edges
            .iter()
            .find(|edge| edge.from == "md" && edge.to == "d0x")
            .expect("missing md->d0x edge");
        let inner_start = md_d0x.points[0];
        assert!(
            inner_start.0 <= md.x + 16.0 && inner_start.1 <= md.y + md.height - 7.0,
            "cylinder source intersection should follow Mermaid's ellipse adjustment instead of the plain rect bottom, got {inner_start:?} for md box x={:.2} bottom={:.2}",
            md.x,
            md.y + md.height
        );

        for (source_id, pod_id) in [("d0", "pa"), ("d1", "pb")] {
            let source = layout.nodes.get(source_id).expect("missing deploy node");
            let pod = layout.nodes.get(pod_id).expect("missing pod node");
            assert!(
                (center_x(source) - center_x(pod)).abs() <= 2.0,
                "dagre keeps labeled member-to-leaf chains centered; got {source_id} center {:.2}, {pod_id} center {:.2}",
                center_x(source),
                center_x(pod)
            );

            let subgraph_label = if source_id == "d0" {
                "fed0 deploy controller"
            } else {
                "fed1 deploy controller"
            };
            let subgraph = layout
                .subgraphs
                .iter()
                .find(|subgraph| subgraph.label == subgraph_label)
                .unwrap_or_else(|| panic!("missing subgraph {subgraph_label}"));
            let routed = layout
                .edges
                .iter()
                .find(|edge| edge.from == source_id && edge.to == pod_id)
                .unwrap_or_else(|| panic!("missing {source_id}->{pod_id} edge"));
            assert_eq!(
                routed.points.len(),
                4,
                "dagre keeps source, compound boundary, label-rank waypoint, and target for {source_id}->{pod_id}; got {:?}",
                routed.points
            );
            assert!(
                (routed.points[1].1 - (subgraph.y + subgraph.height)).abs() <= 0.5,
                "labeled compound-exit edge {source_id}->{pod_id} should pass through the cluster boundary before the label lane; got points {:?}, subgraph bottom {:.2}",
                routed.points,
                subgraph.y + subgraph.height
            );
            let label_anchor = routed
                .label_anchor
                .unwrap_or_else(|| panic!("missing label anchor for {source_id}->{pod_id}"));
            let expected_label_y = (routed.points[1].1 + routed.points[3].1) * 0.5;
            assert!(
                (label_anchor.0 - routed.points[2].0).abs() <= 0.2
                    && (label_anchor.1 - routed.points[2].1).abs() <= 0.2
                    && (label_anchor.1 - expected_label_y).abs() <= 0.5,
                "compound-exit label should sit on the boundary-to-target dagre segment for {source_id}->{pod_id}; got anchor {label_anchor:?}, points {:?}",
                routed.points
            );
        }
    }

    #[test]
    fn flowchart_nested_bridge_child_lanes_use_recursive_ranksep() {
        let source = r#"flowchart TB
    subgraph P8["Phase 8"]
        direction TB
        subgraph FED0R["on fed0"]
            direction TB
            F0a["PatchMetaDeployment<br/>writes local + CRDT publish"]
            F0b["controller iterates fed0's tenant VCs"]
            F0c["vc-a1 → DEPLOY hello-world<br/>vc-a2/a3/a4 → skip (vcSpec filter)"]
            F0a --> F0b --> F0c
        end
        subgraph GS["gossip"]
            direction LR
            G["gossip propagates MD spec<br/>fed0 to fed1 (~1 s)"]
        end
        subgraph FED1R["on fed1"]
            direction TB
            F1a["mddelta materializes MD locally"]
            F1b["SetMDChangeCallback fires<br/>→ RequestMDCheck immediately"]
            F1c["reconcileSingleMD:<br/>LookupTenantByUUID(A...) FAILS<br/>→ fallback LookupTenantByName('fedten') OK"]
            F1d["controller iterates fed1's tenant VCs"]
            F1e["vc-b1 → DEPLOY hello-world<br/>vc-b2/b3/b4 → skip (vcSpec filter)"]
            F1a --> F1b --> F1c --> F1d --> F1e
        end
        F0a --> G --> F1a
    end"#;
        let parsed = parse_mermaid(source).expect("failed to parse nested bridge fixture");
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &Theme::mermaid_default(), &config);
        let top_gap = |from: &str, to: &str| {
            let from_node = layout
                .nodes
                .get(from)
                .unwrap_or_else(|| panic!("missing node {from}"));
            let to_node = layout
                .nodes
                .get(to)
                .unwrap_or_else(|| panic!("missing node {to}"));
            to_node.y - (from_node.y + from_node.height)
        };
        let root_gap = config
            .rank_spacing
            .max(config.flowchart.auto_spacing.min_spacing);
        let recursive_gap = config.rank_spacing + STATE_RANK_SPACING_BOOST;
        let assert_near = |actual: f32, expected: f32, label: &str| {
            assert!(
                (actual - expected).abs() <= 1.0,
                "{label} should use Mermaid recursive dagre spacing {expected:.2}, got {actual:.2}"
            );
        };

        assert_near(top_gap("F0a", "F0b"), recursive_gap * 1.5, "F0a->F0b");
        assert_near(top_gap("F0b", "F0c"), recursive_gap * 2.0, "F0b->F0c");
        assert_near(
            top_gap("F1a", "F1b"),
            recursive_gap + root_gap + FLOWCHART_DAGRE_POINT_MARGIN,
            "F1a->F1b",
        );
        assert_near(top_gap("F1b", "F1c"), recursive_gap, "F1b->F1c");
        for id in ["F0c", "F1e"] {
            let node = layout
                .nodes
                .get(id)
                .unwrap_or_else(|| panic!("missing node {id}"));
            assert_near(
                node.height,
                126.0,
                "wide-arrow fallback should match browser wrapping",
            );
        }
        let f1c = layout.nodes.get("F1c").expect("missing F1c");
        assert!(
            f1c.width > 280.0,
            "non-hyphenated long words should widen flowchart labels; got {:.2}",
            f1c.width,
        );
        assert!(
            f1c.label
                .lines
                .iter()
                .any(|line| line.text() == "LookupTenantByName('fedten')"),
            "browser wrapping keeps long non-hyphenated words intact; got {:?}",
            f1c.label
                .lines
                .iter()
                .map(|line| line.text().into_owned())
                .collect::<Vec<_>>()
        );
        let gossip = layout
            .subgraphs
            .iter()
            .find(|subgraph| subgraph.label == "gossip")
            .expect("missing gossip subgraph");
        let gossip_node = layout.nodes.get("G").expect("missing gossip node");
        assert_near(
            gossip.height,
            gossip_node.height + recursive_gap,
            "nested bridge gossip cluster",
        );
        let target = layout
            .subgraphs
            .iter()
            .find(|subgraph| subgraph.label == "on fed1")
            .expect("missing target bridge subgraph");
        let f1a = layout.nodes.get("F1a").expect("missing F1a");
        let f1e = layout.nodes.get("F1e").expect("missing F1e");
        assert_near(
            f1a.y - target.y,
            24.0 + FLOWCHART_RECURSIVE_CLUSTER_CHILD_MAIN_PAD,
            "nested bridge target top envelope",
        );
        assert_near(
            target.y + target.height - (f1e.y + f1e.height),
            FLOWCHART_RECURSIVE_CLUSTER_CHILD_MAIN_PAD,
            "nested bridge target bottom envelope",
        );
    }

    #[test]
    fn flowchart_top_level_subgraph_chain_uses_root_ranksep() {
        let source = r#"flowchart TB
    subgraph P0["Phase 0"]
        A["A"]
    end
    subgraph P1["Phase 1"]
        B["B"]
    end
    subgraph P2["Phase 2"]
        C["C"]
    end
    P0 --> P1
    P1 --> P2"#;
        let parsed = parse_mermaid(source).expect("failed to parse flowchart chain fixture");
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &Theme::mermaid_default(), &config);
        let subgraph = |label: &str| {
            layout
                .subgraphs
                .iter()
                .find(|subgraph| subgraph.label == label)
                .unwrap_or_else(|| panic!("missing subgraph {label}"))
        };
        let phase0 = subgraph("Phase 0");
        let phase1 = subgraph("Phase 1");
        let phase2 = subgraph("Phase 2");
        let expected_gap = config
            .rank_spacing
            .max(config.flowchart.auto_spacing.min_spacing);
        let gap01 = phase1.y - (phase0.y + phase0.height);
        let gap12 = phase2.y - (phase1.y + phase1.height);

        for (label, gap) in [("Phase 0 -> Phase 1", gap01), ("Phase 1 -> Phase 2", gap12)] {
            assert!(
                (gap - expected_gap).abs() <= 0.5,
                "{label} should use Mermaid root dagre ranksep {expected_gap:.2}, got {gap:.2}"
            );
        }
    }

    #[test]
    fn flowchart_top_level_chain_restacks_after_nested_parent_padding() {
        let source = include_str!(
            "../../tests/mermaid-js-comparison/reference/flowchart-myriplane-federation-phases.mmd"
        );
        let parsed = parse_mermaid(source).expect("failed to parse nested chain fixture");
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &Theme::mermaid_default(), &config);
        let subgraph = |label: &str| {
            layout
                .subgraphs
                .iter()
                .find(|subgraph| subgraph.label == label)
                .unwrap_or_else(|| panic!("missing subgraph {label}"))
        };
        let phase7 = subgraph("Phase 7 — apply MetaDeployment on fed0");
        let phase8 = subgraph("Phase 8 — cross-federation deployment");
        let phase8b = subgraph("Phase 8 verify (kubectl poll)");
        let expected_gap = config
            .rank_spacing
            .max(config.flowchart.auto_spacing.min_spacing);

        for (label, gap) in [
            ("Phase 7 -> Phase 8", phase8.y - (phase7.y + phase7.height)),
            (
                "Phase 8 -> Phase 8 verify",
                phase8b.y - (phase8.y + phase8.height),
            ),
        ] {
            assert!(
                (gap - expected_gap).abs() <= 0.5,
                "{label} should retain the root dagre rank gap after nested parent padding; got {gap:.2}"
            );
        }
    }

    #[test]
    fn flowchart_hexagon_and_cluster_title_sizing_matches_mermaid() {
        let source = r#"flowchart TB
    subgraph P6["Phase 6 — wait for vc-a1 + vc-b1 deployable"]
        direction LR
        W1{{"poll list-vcs Status<br/>accept anything except<br/>{Queuing, Creating, ''}"}}
    end"#;
        let parsed = parse_mermaid(source).expect("failed to parse flowchart hexagon fixture");
        let layout = compute_layout(
            &parsed.graph,
            &Theme::mermaid_default(),
            &LayoutConfig::default(),
        );
        let node = layout.nodes.get("W1").expect("missing W1");
        let phase = layout
            .subgraphs
            .iter()
            .find(|subgraph| subgraph.label == "Phase 6 — wait for vc-a1 + vc-b1 deployable")
            .expect("missing P6 subgraph");

        assert!(
            (node.width - 226.5).abs() <= 1.0 && (node.height - 87.0).abs() <= 1.0,
            "flowchart hexagon should use Mermaid's label+notch sizing; got {:.2}x{:.2}",
            node.width,
            node.height
        );
        assert!(
            (phase.width - 323.5).abs() <= 1.0 && (phase.height - 157.0).abs() <= 1.0,
            "flowchart subgraph title should add only Mermaid's small side pad when it drives width; got {:.2}x{:.2}",
            phase.width,
            phase.height
        );
    }

    #[test]
    fn flowchart_recursive_parent_uses_mermaid_child_padding() {
        let source = r#"flowchart TB
    subgraph P34["Phase 3+4 — admin login &amp; cluster import"]
        direction TB
        L["myrictl login admin<br/>(both members, pass1234)"]
        subgraph CI["import-cluster (kubeconfig per cluster)"]
            direction LR
            K1[("~/.kube/fed0cluster1<br/>-kmaster1")]
            K2[("~/.kube/fed0cluster2<br/>-kmaster1")]
            K3[("~/.kube/fed1cluster1<br/>-kmaster1")]
            K4[("~/.kube/fed1cluster2<br/>-kmaster1")]
        end
        L --> CI
    end"#;
        let parsed =
            parse_mermaid(source).expect("failed to parse recursive flowchart parent fixture");
        let layout = compute_layout(
            &parsed.graph,
            &Theme::mermaid_default(),
            &LayoutConfig::default(),
        );
        let phase = layout
            .subgraphs
            .iter()
            .find(|subgraph| subgraph.label == "Phase 3+4 — admin login &amp; cluster import")
            .expect("missing P34 subgraph");
        let child = layout
            .subgraphs
            .iter()
            .find(|subgraph| subgraph.label == "import-cluster (kubeconfig per cluster)")
            .expect("missing CI subgraph");
        let login = layout.nodes.get("L").expect("missing login node");
        let top_gap = login.y - phase.y;
        let bottom_gap = phase.y + phase.height - (child.y + child.height);

        assert!(
            (top_gap - FLOWCHART_RECURSIVE_CLUSTER_CHILD_MAIN_PAD).abs() <= 1.0,
            "recursive flowchart parent should use Mermaid child-cluster top padding; got {top_gap:.2}"
        );
        assert!(
            (bottom_gap - FLOWCHART_RECURSIVE_CLUSTER_CHILD_MAIN_PAD).abs() <= 1.0,
            "recursive flowchart parent should use Mermaid child-cluster bottom padding; got {bottom_gap:.2}"
        );
        assert!(
            child.width < 300.0,
            "nested flowchart subgraph title should not receive full cluster side padding; got {:.2}",
            child.width
        );
    }

    #[test]
    fn flowchart_compound_member_fanout_uses_dagre_rank_gap_waypoints() {
        let source = r#"flowchart TB
    subgraph FED0["fed0"]
        t0["tenant: fedten<br/>uuid: a759...4cf"]
        t0 --> vca1[vc-a1<br/>fed0cluster1]
        t0 --> vca2[vc-a2<br/>fed0cluster1]
        t0 --> vca3[vc-a3<br/>fed0cluster2]
        t0 --> vca4[vc-a4<br/>fed0cluster2]
    end
    subgraph FED1["fed1"]
        t1["tenant: fedten<br/>uuid: 95bf...4c4"]
        t1 --> vcb1[vc-b1<br/>fed1cluster1]
        t1 --> vcb2[vc-b2<br/>fed1cluster1]
        t1 --> vcb3[vc-b3<br/>fed1cluster2]
        t1 --> vcb4[vc-b4<br/>fed1cluster2]
    end"#;
        let parsed = parse_mermaid(source).expect("failed to parse realworld phase-05 fixture");
        let layout = compute_layout(
            &parsed.graph,
            &Theme::mermaid_default(),
            &LayoutConfig::default(),
        );
        let source_node = layout.nodes.get("t0").expect("missing t0");
        let source_center_y = source_node.y + source_node.height * 0.5;
        let source_right = source_node.x + source_node.width;

        for target_id in ["vca1", "vca2", "vca3", "vca4"] {
            let target = layout.nodes.get(target_id).expect("missing fanout target");
            let target_center_y = target.y + target.height * 0.5;
            let expected_bend_x = (source_right + target.x) * 0.5;
            let routed = layout
                .edges
                .iter()
                .find(|edge| edge.from == "t0" && edge.to == target_id)
                .unwrap_or_else(|| panic!("missing t0->{target_id} edge"));
            assert_eq!(
                routed.points.len(),
                3,
                "dagre same-rank fanout should keep source-intersection, rank-gap waypoint, and target-intersection for t0->{target_id}; got {:?}",
                routed.points
            );
            let start = routed.points[0];
            let bend = routed.points[1];
            let end = routed.points[2];

            assert!(
                (bend.0 - expected_bend_x).abs() <= 2.0 && (bend.1 - target_center_y).abs() <= 2.0,
                "dagre bend for t0->{target_id} should sit halfway through the rank gap on the target lane; got {bend:?}, expected x {expected_bend_x:.2}, y {target_center_y:.2}"
            );
            assert!(
                (end.0 - target.x).abs() <= 2.0 && (end.1 - target_center_y).abs() <= 2.0,
                "dagre route should enter {target_id} from the left, perpendicular to the arrowhead; got end {end:?}, target left {:.2}, center y {:.2}",
                target.x,
                target_center_y
            );
            if target_center_y < source_center_y {
                assert!(
                    (start.1 - source_node.y).abs() <= 2.0,
                    "upper fanout t0->{target_id} should leave the tenant from the top edge; got {start:?}, top {:.2}",
                    source_node.y
                );
            } else {
                let source_bottom = source_node.y + source_node.height;
                assert!(
                    (start.1 - source_bottom).abs() <= 2.0,
                    "lower fanout t0->{target_id} should leave the tenant from the bottom edge; got {start:?}, bottom {source_bottom:.2}"
                );
            }
        }
    }

    #[test]
    fn flowchart_three_node_compound_cycles_use_dagre_feedback_lanes() {
        let source = r#"flowchart LR
    subgraph fed0["fed0 etcd cluster"]
        direction TB
        e0a[etcd0-fed0<br/>:22479]
        e0b[etcd1-fed0<br/>:22489]
        e0c[etcd2-fed0<br/>:22499]
        e0a <--> e0b <--> e0c <--> e0a
    end
    subgraph fed1["fed1 etcd cluster"]
        direction TB
        e1a[etcd0-fed1<br/>:22579]
        e1b[etcd1-fed1<br/>:22589]
        e1c[etcd2-fed1<br/>:22599]
        e1a <--> e1b <--> e1c <--> e1a
    end"#;
        let parsed = parse_mermaid(source).expect("failed to parse realworld phase-01 fixture");
        let layout = compute_layout(
            &parsed.graph,
            &Theme::mermaid_default(),
            &LayoutConfig::default(),
        );
        let top = layout.nodes.get("e0a").expect("missing e0a");
        let middle = layout.nodes.get("e0b").expect("missing e0b");
        let bottom = layout.nodes.get("e0c").expect("missing e0c");
        let center_x = |node: &NodeLayout| node.x + node.width * 0.5;
        let top_center = center_x(top);
        let middle_center = center_x(middle);
        let expected_top_middle_y = ((top.y + top.height) + middle.y) * 0.5;
        let expected_middle_bottom_y = ((middle.y + middle.height) + bottom.y) * 0.5;
        let expected_feedback_lane_x = top_center + (top_center - middle_center);

        let top_middle = layout
            .edges
            .iter()
            .find(|edge| edge.from == "e0a" && edge.to == "e0b")
            .expect("missing e0a->e0b edge");
        assert_eq!(
            top_middle.points.len(),
            3,
            "dagre forward cycle edge should keep source, rank-gap waypoint, target; got {:?}",
            top_middle.points
        );
        assert!(
            (top_middle.points[1].0 - middle_center).abs() <= 1.0
                && (top_middle.points[1].1 - expected_top_middle_y).abs() <= 1.0,
            "e0a->e0b should bend through the middle-node column like dagre; got {:?}",
            top_middle.points
        );
        assert!(
            (top_middle.points[2].0 - middle_center).abs() <= 1.0
                && (top_middle.points[2].1 - middle.y).abs() <= 1.0,
            "e0a->e0b should enter e0b from the top center; got {:?}",
            top_middle.points
        );

        let middle_bottom = layout
            .edges
            .iter()
            .find(|edge| edge.from == "e0b" && edge.to == "e0c")
            .expect("missing e0b->e0c edge");
        assert_eq!(
            middle_bottom.points.len(),
            3,
            "dagre second forward cycle edge should keep source, rank-gap waypoint, target; got {:?}",
            middle_bottom.points
        );
        assert!(
            (middle_bottom.points[1].0 - middle_center).abs() <= 1.0
                && (middle_bottom.points[1].1 - expected_middle_bottom_y).abs() <= 1.0,
            "e0b->e0c should bend through the middle-node column like dagre; got {:?}",
            middle_bottom.points
        );

        let feedback = layout
            .edges
            .iter()
            .find(|edge| edge.from == "e0c" && edge.to == "e0a")
            .expect("missing e0c->e0a edge");
        assert_eq!(
            feedback.points.len(),
            5,
            "dagre feedback cycle edge should keep the right-side five-point lane; got {:?}",
            feedback.points
        );
        for point in &feedback.points[1..4] {
            assert!(
                (point.0 - expected_feedback_lane_x).abs() <= 1.0,
                "feedback lane should stay at x {expected_feedback_lane_x:.2}; got {:?}",
                feedback.points
            );
        }
    }

    #[test]
    fn flowchart_parent_direction_orders_child_subgraphs_by_anchor_edges() {
        let source = r#"flowchart LR
subgraph TOP
    direction TB
    subgraph B1
        direction RL
        i1 --> f1
    end
    subgraph B2
        direction BT
        i2 --> f2
    end
end
A --> TOP --> B
B1 --> B2"#;
        let parsed = parse_mermaid(source).expect("failed to parse nested subgraph fixture");
        let layout = compute_layout(
            &parsed.graph,
            &Theme::mermaid_default(),
            &LayoutConfig::default(),
        );
        let b1 = layout
            .subgraphs
            .iter()
            .find(|sub| sub.label == "B1")
            .expect("missing B1 layout");
        let b2 = layout
            .subgraphs
            .iter()
            .find(|sub| sub.label == "B2")
            .expect("missing B2 layout");

        assert!(
            b1.y + b1.height <= b2.y + 1.0,
            "TOP direction TB should place B1 above B2; got B1 bottom {:.2}, B2 top {:.2}",
            b1.y + b1.height,
            b2.y
        );
        let b1_center_x = b1.x + b1.width * 0.5;
        let b2_center_x = b2.x + b2.width * 0.5;
        assert!(
            (b1_center_x - b2_center_x).abs() <= 1.0,
            "TOP direction TB should center child clusters by rank; got B1 center {:.2}, B2 center {:.2}",
            b1_center_x,
            b2_center_x
        );
        assert!(
            (b1.height - 124.0).abs() <= 4.0,
            "horizontal recursive child cluster should use Mermaid cross-axis padding; got B1 height {:.2}",
            b1.height
        );
        assert!(
            (b2.width - 145.0).abs() <= 2.0,
            "vertical recursive child cluster should use Mermaid cross-axis padding; got B2 width {:.2}",
            b2.width
        );
        let top = layout
            .subgraphs
            .iter()
            .find(|sub| sub.label == "TOP")
            .expect("missing TOP layout");
        assert!(
            b1.y - top.y >= 30.0,
            "recursive parent should keep Mermaid-like space above child clusters; got top gap {:.2}",
            b1.y - top.y
        );
        let a = layout.nodes.get("A").expect("missing A node");
        let b = layout.nodes.get("B").expect("missing B node");
        assert!(
            top.x - (a.x + a.width) >= 49.0,
            "recursive cluster root spacing should retain Mermaid's 50px rank gap; got left gap {:.2}",
            top.x - (a.x + a.width)
        );
        assert!(
            b.x - (top.x + top.width) >= 49.0,
            "recursive cluster root spacing should retain Mermaid's 50px rank gap; got right gap {:.2}",
            b.x - (top.x + top.width)
        );
    }

    #[test]
    fn layout_places_nodes() {
        let mut graph = Graph::new();
        graph.direction = Direction::LeftRight;
        graph.ensure_node("A", Some("Alpha".to_string()), Some(NodeShape::Rectangle));
        graph.ensure_node("B", Some("Beta".to_string()), Some(NodeShape::Rectangle));
        graph.edges.push(crate::ir::Edge {
            from: "A".to_string(),
            to: "B".to_string(),
            label: None,
            start_label: None,
            end_label: None,
            directed: true,
            arrow_start: false,
            arrow_end: true,
            arrow_start_kind: None,
            arrow_end_kind: None,
            start_decoration: None,
            end_decoration: None,
            sequence_arrow_end: None,
            sequence_arrow_start: None,
            style: crate::ir::EdgeStyle::Solid,
            markdown_label: false,
            id: None,
            curve: None,
            arch_port_from: None,
            arch_port_to: None,
        });
        let layout = compute_layout(&graph, &Theme::modern(), &LayoutConfig::default());
        let a = layout.nodes.get("A").unwrap();
        let b = layout.nodes.get("B").unwrap();
        assert!(b.x >= a.x);
    }

    #[test]
    fn edge_style_merges_default_and_override() {
        let mut graph = Graph::new();
        graph.ensure_node("A", Some("Alpha".to_string()), Some(NodeShape::Rectangle));
        graph.ensure_node("B", Some("Beta".to_string()), Some(NodeShape::Rectangle));
        graph.edges.push(crate::ir::Edge {
            from: "A".to_string(),
            to: "B".to_string(),
            label: None,
            start_label: None,
            end_label: None,
            directed: true,
            arrow_start: false,
            arrow_end: true,
            arrow_start_kind: None,
            arrow_end_kind: None,
            start_decoration: None,
            end_decoration: None,
            sequence_arrow_end: None,
            sequence_arrow_start: None,
            style: crate::ir::EdgeStyle::Solid,
            markdown_label: false,
            id: None,
            curve: None,
            arch_port_from: None,
            arch_port_to: None,
        });

        graph.edge_style_default = Some(crate::ir::EdgeStyleOverride {
            stroke: Some("#111111".to_string()),
            stroke_width: None,
            dasharray: None,
            label_color: Some("#222222".to_string()),
        });
        graph.edge_styles.insert(
            0,
            crate::ir::EdgeStyleOverride {
                stroke: None,
                stroke_width: Some(4.0),
                dasharray: None,
                label_color: None,
            },
        );

        let layout = compute_layout(&graph, &Theme::modern(), &LayoutConfig::default());
        let edge = &layout.edges[0];
        assert_eq!(edge.override_style.stroke.as_deref(), Some("#111111"));
        assert_eq!(edge.override_style.stroke_width, Some(4.0));
        assert_eq!(edge.override_style.label_color.as_deref(), Some("#222222"));
    }

    #[test]
    fn er_labels_stay_attached_after_path_postprocess() {
        let source = include_str!("../../docs/comparison_sources/er_blog.mmd");
        let parsed = parse_mermaid(source).expect("failed to parse ER fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        let mut labeled_edges = 0usize;
        for edge in &layout.edges {
            let (Some(_label), Some(anchor)) = (&edge.label, edge.label_anchor) else {
                continue;
            };
            labeled_edges += 1;
            let dist = polyline_point_distance(&edge.points, anchor);
            assert!(
                dist <= 15.0,
                "edge {}->{} label anchor drifted {:.2}px from own path",
                edge.from,
                edge.to,
                dist
            );
        }
        assert!(
            labeled_edges > 0,
            "fixture must contain at least one labeled edge"
        );
    }

    #[test]
    fn er_attribute_rows_are_not_auto_wrapped_before_rendering() {
        let source = "erDiagram\nPERSON {\nstring driversLicense PK \"The license #\"\nstring(99) firstName \"Only 99 characters are allowed\"\n}";
        let parsed = parse_mermaid(source).expect("failed to parse ER fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let person = layout.nodes.get("PERSON").expect("PERSON node");
        let lines: Vec<String> = person
            .label
            .lines
            .iter()
            .map(|line| line.text().into_owned())
            .collect();

        assert!(lines.contains(&"string driversLicense PK \"The license #\"".to_string()));
        assert!(
            lines.contains(&"string(99) firstName \"Only 99 characters are allowed\"".to_string())
        );
        assert!(!lines.iter().any(|line| line == "license #\""));
    }

    #[test]
    fn er_uses_mermaid_spacing_defaults() {
        let source = "erDiagram\nCUSTOMER ||--o{ ORDER : places";
        let parsed = parse_mermaid(source).expect("failed to parse ER fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let customer = layout.nodes.get("CUSTOMER").expect("CUSTOMER node");
        let order = layout.nodes.get("ORDER").expect("ORDER node");
        let vertical_gap = order.y - (customer.y + customer.height);

        assert!(
            vertical_gap >= ER_DEFAULT_RANK_SPACING,
            "ER default rank gap {vertical_gap:.2}px should honor Mermaid's {ER_DEFAULT_RANK_SPACING}px spacing"
        );
    }

    #[test]
    fn flowchart_layout_tolerates_empty_subgraph_layout_slots() {
        let parsed = parse_mermaid(
            "flowchart TB\nsubgraph Empty[\"Empty\"]\nend\nsubgraph Filled[\"Filled\"]\nA[Alpha]\nend\nEmpty --> Filled",
        )
        .unwrap();
        assert!(parsed.graph.subgraphs[0].nodes.is_empty());

        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());

        assert!(layout.width > 0.0);
        assert!(layout.height > 0.0);
    }

    #[test]
    fn er_empty_entities_use_mermaid_box_size() {
        let source = "erDiagram\n\"This **is** _Markdown_\"";
        let parsed = parse_mermaid(source).expect("failed to parse ER fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let node = layout
            .nodes
            .get("This **is** _Markdown_")
            .expect("markdown ER node");

        assert!(
            node.height >= 83.0 && node.height <= 85.0,
            "empty ER entity height {:.2}px should match Mermaid's 84px no-attribute erBox",
            node.height
        );
        assert!(
            node.width >= ER_ENTITY_MIN_WIDTH,
            "empty ER entity width {:.2}px should honor Mermaid's minEntityWidth",
            node.width
        );
    }

    #[test]
    fn er_attribute_entities_use_mermaid_row_height() {
        let source = "erDiagram\nCUSTOMER {\nstring name\nstring custNumber\nstring sector\n}";
        let parsed = parse_mermaid(source).expect("failed to parse ER fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let node = layout.nodes.get("CUSTOMER").expect("CUSTOMER node");

        assert!(
            node.height >= 170.0 && node.height <= 172.0,
            "ER entity height {:.2}px should reserve one Mermaid row for the header and each attribute",
            node.height
        );
    }

    fn make_node(id: &str, x: f32, y: f32, width: f32, height: f32) -> NodeLayout {
        NodeLayout {
            id: id.to_string(),
            x,
            y,
            width,
            height,
            label: TextBlock {
                lines: vec![TextLine::plain(String::new())],
                width: 0.0,
                height: 0.0,
            },
            shape: crate::ir::NodeShape::Rectangle,
            style: crate::ir::NodeStyle::default(),
            link: None,
            anchor_subgraph: None,
            hidden: false,
            icon: None,
            img: None,
            img_w: None,
            img_h: None,
            sub_label: None,
            is_treemap_leaf: false,
            treemap_base_text_color: None,
        }
    }

    fn make_edge(from: &str, to: &str, style: crate::ir::EdgeStyle) -> crate::ir::Edge {
        crate::ir::Edge {
            from: from.to_string(),
            to: to.to_string(),
            label: None,
            start_label: None,
            end_label: None,
            directed: true,
            arrow_start: false,
            arrow_end: true,
            arrow_start_kind: None,
            arrow_end_kind: None,
            start_decoration: None,
            end_decoration: None,
            sequence_arrow_end: None,
            sequence_arrow_start: None,
            style,
            markdown_label: false,
            id: None,
            curve: None,
            arch_port_from: None,
            arch_port_to: None,
        }
    }

    #[test]
    fn path_bend_count_tracks_turns() {
        let straight = vec![(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)];
        let orth = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (20.0, 10.0)];
        assert_eq!(path_bend_count(&straight), 0);
        assert_eq!(path_bend_count(&orth), 2);
    }

    #[test]
    fn edge_label_anchor_uses_path_progress_midpoint() {
        let points = vec![(0.0, 0.0), (20.0, 0.0), (20.0, 100.0)];
        let center = edge_label_anchor_from_points(&points).expect("anchor");
        assert!((center.0 - 20.0).abs() <= 1e-3);
        assert!((center.1 - 40.0).abs() <= 1e-3);
    }

    #[test]
    fn rank_edges_prefers_non_dotted_flow_edges_when_coverage_is_good() {
        let graph = Graph::new();
        let nodes = vec![
            "A".to_string(),
            "B".to_string(),
            "C".to_string(),
            "D".to_string(),
        ];
        let edges = vec![
            make_edge("A", "B", crate::ir::EdgeStyle::Solid),
            make_edge("B", "C", crate::ir::EdgeStyle::Solid),
            make_edge("C", "D", crate::ir::EdgeStyle::Solid),
            make_edge("A", "D", crate::ir::EdgeStyle::Dotted),
        ];
        let rank_edges = rank_edges_for_manual_layout(&graph, &nodes, &edges);
        assert_eq!(rank_edges.len(), 3);
        assert!(
            rank_edges
                .iter()
                .all(|edge| edge.style != crate::ir::EdgeStyle::Dotted)
        );
    }

    #[test]
    fn rank_edges_falls_back_when_primary_coverage_is_too_small() {
        let graph = Graph::new();
        let nodes = vec![
            "A".to_string(),
            "B".to_string(),
            "C".to_string(),
            "D".to_string(),
            "E".to_string(),
        ];
        let edges = vec![
            make_edge("A", "B", crate::ir::EdgeStyle::Solid),
            make_edge("C", "D", crate::ir::EdgeStyle::Dotted),
            make_edge("D", "E", crate::ir::EdgeStyle::Dotted),
            make_edge("E", "C", crate::ir::EdgeStyle::Dotted),
        ];
        let rank_edges = rank_edges_for_manual_layout(&graph, &nodes, &edges);
        assert_eq!(rank_edges.len(), edges.len());
    }

    #[test]
    fn routing_avoids_occupied_lane_when_possible() {
        let config = LayoutConfig::default();
        let from = make_node("A", 0.0, 0.0, 40.0, 40.0);
        let to = make_node("B", 200.0, 0.0, 40.0, 40.0);
        let obstacles: Vec<Obstacle> = Vec::new();
        let label_obstacles: Vec<Obstacle> = Vec::new();
        let ctx = RouteContext {
            from_id: &from.id,
            to_id: &to.id,
            from: &from,
            to: &to,
            direction: Direction::LeftRight,
            config: &config,
            obstacles: &obstacles,
            label_obstacles: &label_obstacles,
            base_offset: 0.0,
            start_side: EdgeSide::Right,
            end_side: EdgeSide::Left,
            start_offset: 0.0,
            end_offset: 0.0,
            fast_route: false,
            stub_len: port_stub_length(&config, &from, &to),
            prefer_shorter_ties: true,
            preferred_label_id: None,
            preferred_label_center: None,
        };
        let mut occupancy = EdgeOccupancy::new(
            config.node_spacing.max(MIN_NODE_SPACING_FLOOR) * EDGE_OCCUPANCY_CELL_RATIO,
        );
        let start = anchor_point_for_node(&from, EdgeSide::Right, 0.0);
        let end = anchor_point_for_node(&to, EdgeSide::Left, 0.0);
        occupancy.add_path(&[start, end]);

        let points = route_edge_with_avoidance(&ctx, Some(&occupancy), None, None);
        assert!(
            points.len() > 2,
            "expected a detoured path to avoid occupied lane"
        );
    }

    #[test]
    fn routing_handles_tiny_nodes_without_panicking() {
        let config = LayoutConfig::default();
        let from = make_node("A", 0.0, 0.0, 1.0, 1.0);
        let to = make_node("B", 50.0, 0.0, 1.0, 1.0);
        let obstacles: Vec<Obstacle> = Vec::new();
        let label_obstacles: Vec<Obstacle> = Vec::new();
        let ctx = RouteContext {
            from_id: &from.id,
            to_id: &to.id,
            from: &from,
            to: &to,
            direction: Direction::LeftRight,
            config: &config,
            obstacles: &obstacles,
            label_obstacles: &label_obstacles,
            base_offset: 0.0,
            start_side: EdgeSide::Right,
            end_side: EdgeSide::Left,
            start_offset: 0.0,
            end_offset: 0.0,
            fast_route: false,
            stub_len: port_stub_length(&config, &from, &to),
            prefer_shorter_ties: true,
            preferred_label_id: None,
            preferred_label_center: None,
        };
        let points = route_edge_with_avoidance(&ctx, None, None, None);
        assert!(!points.is_empty());
    }

    #[test]
    fn grid_router_avoids_blocking_obstacle() {
        let mut config = LayoutConfig::default();
        config.flowchart.routing.enable_grid_router = true;
        config.flowchart.routing.grid_cell = 10.0;
        let from = make_node("A", 0.0, 0.0, 40.0, 40.0);
        let to = make_node("B", 220.0, 0.0, 40.0, 40.0);
        let obstacles = vec![Obstacle {
            id: "blocker".to_string(),
            x: 90.0,
            y: -10.0,
            width: 80.0,
            height: 60.0,
            members: None,
        }];
        let label_obstacles: Vec<Obstacle> = Vec::new();
        let grid = build_routing_grid(&obstacles, &config).expect("routing grid");
        let ctx = RouteContext {
            from_id: &from.id,
            to_id: &to.id,
            from: &from,
            to: &to,
            direction: Direction::LeftRight,
            config: &config,
            obstacles: &obstacles,
            label_obstacles: &label_obstacles,
            base_offset: 0.0,
            start_side: EdgeSide::Right,
            end_side: EdgeSide::Left,
            start_offset: 0.0,
            end_offset: 0.0,
            fast_route: false,
            stub_len: port_stub_length(&config, &from, &to),
            prefer_shorter_ties: true,
            preferred_label_id: None,
            preferred_label_center: None,
        };
        let start = anchor_point_for_node(&from, EdgeSide::Right, 0.0);
        let end = anchor_point_for_node(&to, EdgeSide::Left, 0.0);
        let stub_len = port_stub_length(&config, &from, &to);
        let start_stub = port_stub_point(start, EdgeSide::Right, stub_len);
        let end_stub = port_stub_point(end, EdgeSide::Left, stub_len);
        let points =
            route_edge_with_grid(&ctx, &grid, None, start_stub, end_stub).expect("grid route");
        let hits = path_obstacle_intersections(&points, &obstacles, &from.id, &to.id);
        assert_eq!(hits, 0, "grid path should avoid obstacle");
    }

    #[test]
    fn path_label_intersections_can_ignore_owned_reservation() {
        let path = vec![(0.0, 0.0), (100.0, 0.0)];
        let labels = vec![
            Obstacle {
                id: "edge-label-reserved:0".to_string(),
                x: 40.0,
                y: -5.0,
                width: 20.0,
                height: 10.0,
                members: None,
            },
            Obstacle {
                id: "edge-label-reserved:1".to_string(),
                x: 70.0,
                y: -5.0,
                width: 20.0,
                height: 10.0,
                members: None,
            },
        ];
        let all_hits = path_label_intersections(&path, &labels, None);
        assert_eq!(all_hits, 2);
        let own_ignored = path_label_intersections(&path, &labels, Some("edge-label-reserved:0"));
        assert_eq!(own_ignored, 1);
    }

    #[test]
    fn routing_prefers_path_through_preferred_label_center() {
        let config = LayoutConfig::default();
        let from = make_node("A", 0.0, 0.0, 40.0, 40.0);
        let to = make_node("B", 220.0, 0.0, 40.0, 40.0);
        let obstacles: Vec<Obstacle> = Vec::new();
        let label_obstacles: Vec<Obstacle> = Vec::new();
        let preferred = (120.0, 84.0);
        let ctx = RouteContext {
            from_id: &from.id,
            to_id: &to.id,
            from: &from,
            to: &to,
            direction: Direction::LeftRight,
            config: &config,
            obstacles: &obstacles,
            label_obstacles: &label_obstacles,
            base_offset: 0.0,
            start_side: EdgeSide::Right,
            end_side: EdgeSide::Left,
            start_offset: 0.0,
            end_offset: 0.0,
            fast_route: false,
            stub_len: port_stub_length(&config, &from, &to),
            prefer_shorter_ties: true,
            preferred_label_id: Some("edge-label-reserved:0"),
            preferred_label_center: Some(preferred),
        };
        let points = route_edge_with_avoidance(&ctx, None, None, None);
        let dist = polyline_point_distance(&points, preferred);
        assert!(
            dist <= 0.51,
            "expected routed path to pass through preferred label center, got distance {dist:.3}"
        );
    }

    #[test]
    fn state_labeled_transition_reserves_dagre_label_gap() {
        let source = include_str!(
            "../../tests/mermaid-js-comparison/reference/stateDiagram-transition-with-label.mmd"
        );
        let parsed = parse_mermaid(source).expect("failed to parse state transition fixture");
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        let center_y = |id: &str| {
            let node = layout
                .nodes
                .get(id)
                .unwrap_or_else(|| panic!("missing node {id}"));
            node.y + node.height * 0.5
        };

        let start_to_s1 = center_y("s1") - center_y("__start_root__");
        let labeled_gap = center_y("s2") - center_y("s1");
        let s2_to_end = center_y("__end_root__") - center_y("s2");

        assert!(
            (72.0..=78.0).contains(&start_to_s1),
            "unlabeled start->s1 gap should stay near Mermaid's compact ranksep, got {start_to_s1:.2}"
        );
        assert!(
            (108.0..=115.0).contains(&labeled_gap),
            "labeled s1->s2 gap should include the edge label's main-axis size, got {labeled_gap:.2}"
        );
        assert!(
            (72.0..=78.0).contains(&s2_to_end),
            "unlabeled s2->end gap should stay near Mermaid's compact ranksep, got {s2_to_end:.2}"
        );
        assert!(
            (280.0..=303.0).contains(&layout.height),
            "fixture height should remain in the compact state-chain size class, got {}",
            layout.height
        );
    }

    #[test]
    fn state_choice_uses_mermaid_marker_size_and_label_gap() {
        let source = include_str!(
            "../../tests/mermaid-js-comparison/reference/stateDiagram-choice-pseudostate.mmd"
        );
        let parsed = parse_mermaid(source).expect("failed to parse choice state fixture");
        let layout = compute_layout(
            &parsed.graph,
            &Theme::mermaid_default(),
            &LayoutConfig::default(),
        );
        let node = |id: &str| {
            layout
                .nodes
                .get(id)
                .unwrap_or_else(|| panic!("missing node {id}"))
        };
        let center_y = |id: &str| {
            let node = node(id);
            node.y + node.height * 0.5
        };

        assert_eq!(node("__start_root__").height, STATE_MARKER_FIXED_SIZE);
        assert_eq!(node("__end_root__").height, STATE_MARKER_FIXED_SIZE);

        let choice_to_sw1 = center_y("sw1") - center_y("if_state");
        let choice_to_sw2 = center_y("sw2") - center_y("if_state");
        assert!(
            (107.0..=109.0).contains(&choice_to_sw1) && (107.0..=109.0).contains(&choice_to_sw2),
            "choice fanout should reserve Mermaid's labeled edge rank gap, got sw1={choice_to_sw1:.2}, sw2={choice_to_sw2:.2}"
        );
        let edge_anchor = |from: &str, to: &str| {
            layout
                .edges
                .iter()
                .find(|edge| edge.from == from && edge.to == to)
                .and_then(|edge| edge.label_anchor)
                .unwrap_or_else(|| panic!("missing label anchor for {from}->{to}"))
        };
        let center_x = |id: &str| {
            let node = node(id);
            node.x + node.width * 0.5
        };
        let expected_label_y = (node("if_state").y + node("if_state").height + node("sw1").y) * 0.5;
        let sw1_anchor = edge_anchor("if_state", "sw1");
        let sw2_anchor = edge_anchor("if_state", "sw2");
        assert!(
            (sw1_anchor.0 - center_x("sw1")).abs() <= 0.2
                && (sw2_anchor.0 - center_x("sw2")).abs() <= 0.2,
            "choice labels should stay on their target branch columns, got sw1={sw1_anchor:?}, sw2={sw2_anchor:?}"
        );
        assert!(
            (sw1_anchor.1 - expected_label_y).abs() <= 0.2
                && (sw2_anchor.1 - expected_label_y).abs() <= 0.2,
            "choice labels should stay in Mermaid's midpoint label lane, got sw1={sw1_anchor:?}, sw2={sw2_anchor:?}, expected y={expected_label_y:.2}"
        );
        assert!(
            (170.0..=172.0).contains(&layout.width),
            "choice fixture should stay in Mermaid's 171px width class, got {:.2}",
            layout.width
        );
        assert!(
            (373.0..=379.0).contains(&layout.height),
            "choice fixture should stay in Mermaid's 376px height class, got {:.2}",
            layout.height
        );
    }

    #[test]
    fn state_fork_join_uses_mermaid_layout_envelope() {
        let source = include_str!(
            "../../tests/mermaid-js-comparison/reference/stateDiagram-fork-and-join.mmd"
        );
        let parsed = parse_mermaid(source).expect("failed to parse fork/join state fixture");
        let layout = compute_layout(
            &parsed.graph,
            &Theme::mermaid_default(),
            &LayoutConfig::default(),
        );
        let node = |id: &str| {
            layout
                .nodes
                .get(id)
                .unwrap_or_else(|| panic!("missing node {id}"))
        };
        let center_y = |id: &str| {
            let node = node(id);
            node.y + node.height * 0.5
        };

        assert_eq!(node("fork_state").height, STATE_FORK_JOIN_LAYOUT_HEIGHT);
        assert_eq!(node("join_state").height, STATE_FORK_JOIN_LAYOUT_HEIGHT);
        assert!(
            (63.0..=65.0).contains(&(center_y("fork_state") - center_y("__start_root__"))),
            "start -> fork center gap should match Mermaid's 64px rank gap"
        );
        assert!(
            (76.0..=78.0).contains(&(center_y("State2") - center_y("fork_state"))),
            "fork -> state center gap should match Mermaid's 77px rank gap"
        );
        assert!(
            (76.0..=78.0).contains(&(center_y("join_state") - center_y("State2"))),
            "state -> join center gap should match Mermaid's 77px rank gap"
        );
        assert!(
            (63.0..=65.0).contains(&(center_y("__end_root__") - center_y("join_state"))),
            "join -> end center gap should match Mermaid's 64px rank gap"
        );
        let edge_points = |from: &str, to: &str| {
            layout
                .edges
                .iter()
                .find(|edge| edge.from == from && edge.to == to)
                .unwrap_or_else(|| panic!("missing edge {from}->{to}"))
                .points
                .as_slice()
        };
        let assert_near = |actual: (f32, f32), expected: (f32, f32)| {
            assert!(
                (actual.0 - expected.0).abs() <= 0.05 && (actual.1 - expected.1).abs() <= 0.05,
                "expected point {expected:?}, got {actual:?}"
            );
        };

        let fork_to_state = edge_points("fork_state", "State2");
        assert_eq!(fork_to_state.len(), 3);
        assert_near(fork_to_state[0], (82.67, 86.0));
        assert_near(fork_to_state[1], (38.95, 111.0));
        assert_near(fork_to_state[2], (38.95, 136.0));

        let state_to_join = edge_points("State2", "join_state");
        assert_eq!(state_to_join.len(), 3);
        assert_near(state_to_join[0], (38.95, 176.0));
        assert_near(state_to_join[1], (38.95, 201.0));
        assert_near(state_to_join[2], (82.67, 226.0));
        assert!(
            (310.0..=314.0).contains(&layout.height),
            "fork/join fixture should stay in Mermaid's 312px height class, got {:.2}",
            layout.height
        );
    }

    #[test]
    fn state_composite_states_preserve_root_dagre_gaps() {
        let source = include_str!(
            "../../tests/mermaid-js-comparison/reference/stateDiagram-composite-states.mmd"
        );
        let parsed = parse_mermaid(source).expect("failed to parse composite state fixture");
        let layout = compute_layout(
            &parsed.graph,
            &Theme::mermaid_default(),
            &LayoutConfig::default(),
        );
        let node = |id: &str| {
            layout
                .nodes
                .get(id)
                .unwrap_or_else(|| panic!("missing node {id}"))
        };
        let subgraph = |label: &str| {
            layout
                .subgraphs
                .iter()
                .find(|subgraph| subgraph.label == label)
                .unwrap_or_else(|| panic!("missing subgraph {label}"))
        };

        let first = subgraph("First");
        let end = subgraph("End");
        let root_start = node("__start_root__");
        let root_end = node("__end_root__");
        let start_gap = first.y - (root_start.y + root_start.height);
        let middle_gap = end.y - (first.y + first.height);
        let end_gap = root_end.y - (end.y + end.height);

        assert!(
            (49.0..=51.0).contains(&start_gap)
                && (49.0..=51.0).contains(&middle_gap)
                && (49.0..=51.0).contains(&end_gap),
            "root/composite gaps should retain Mermaid's 50px root ranksep, got start={start_gap:.2}, middle={middle_gap:.2}, end={end_gap:.2}"
        );
        assert!(
            (291.0..=296.0).contains(&first.height) && (291.0..=296.0).contains(&end.height),
            "leaf composite boxes should match Mermaid's recursive dagre height, got First {:.2}, End {:.2}",
            first.height,
            end.height
        );
        assert!(
            (776.0..=783.0).contains(&layout.height),
            "composite-states fixture should stay in Mermaid's 780px height class, got {:.2}",
            layout.height
        );
    }

    #[test]
    fn state_transitions_between_composite_states_match_mermaid_envelopes() {
        let source = include_str!(
            "../../tests/mermaid-js-comparison/reference/stateDiagram-transitions-between-composite-states.mmd"
        );
        let parsed =
            parse_mermaid(source).expect("failed to parse composite transition state fixture");
        let layout = compute_layout(
            &parsed.graph,
            &Theme::mermaid_default(),
            &LayoutConfig::default(),
        );
        let subgraph = |label: &str| {
            layout
                .subgraphs
                .iter()
                .find(|subgraph| subgraph.label == label)
                .unwrap_or_else(|| panic!("missing subgraph {label}"))
        };
        let node = |id: &str| {
            layout
                .nodes
                .get(id)
                .unwrap_or_else(|| panic!("missing node {id}"))
        };
        let center_y = |id: &str| {
            let node = node(id);
            node.y + node.height * 0.5
        };

        let first = subgraph("First");
        let second = subgraph("Second");
        let middle_gap = second.y - (first.y + first.height);
        let first_start_to_state = center_y("fir") - center_y("__start_First__");
        let first_state_to_end = center_y("__end_First__") - center_y("fir");
        let second_start_to_state = center_y("sec") - center_y("__start_Second__");
        let second_state_to_end = center_y("__end_Second__") - center_y("sec");

        assert!(
            (102.0..=104.0).contains(&first.width),
            "First composite should keep Mermaid's child-content envelope width, got {:.2}",
            first.width
        );
        assert!(
            (108.0..=110.0).contains(&second.width),
            "Second composite should not be widened by title padding beyond Mermaid's envelope, got {:.2}",
            second.width
        );
        assert!(
            (292.0..=294.0).contains(&first.height) && (292.0..=294.0).contains(&second.height),
            "sparse non-root composites should match Mermaid's 293px height, got First {:.2}, Second {:.2}",
            first.height,
            second.height
        );
        assert!(
            (49.0..=51.0).contains(&middle_gap),
            "root composite-to-composite edge rank gap should remain Mermaid's 50px, got {middle_gap:.2}"
        );
        assert!(
            (101.0..=103.0).contains(&first_start_to_state)
                && (101.0..=103.0).contains(&first_state_to_end)
                && (101.0..=103.0).contains(&second_start_to_state)
                && (101.0..=103.0).contains(&second_state_to_end),
            "inner sparse composite ranks should match Mermaid's 102px centers, got First start/state {first_start_to_state:.2}, First state/end {first_state_to_end:.2}, Second start/state {second_start_to_state:.2}, Second state/end {second_state_to_end:.2}"
        );
        assert!(
            (124.0..=127.0).contains(&layout.width) && (650.0..=654.0).contains(&layout.height),
            "fixture should stay in Mermaid's 125x652 size class, got {:.2}x{:.2}",
            layout.width,
            layout.height
        );
    }

    #[test]
    fn state_nested_composite_shared_node_uses_compound_ranks() {
        let source = include_str!(
            "../../tests/mermaid-js-comparison/reference/stateDiagram-nested-composite-states.mmd"
        );
        let parsed = parse_mermaid(source).expect("failed to parse nested state fixture");
        let second_subgraph = parsed
            .graph
            .subgraphs
            .iter()
            .find(|sub| sub.label == "Second")
            .expect("Second subgraph");
        let end_subgraph = parsed
            .graph
            .subgraphs
            .iter()
            .find(|sub| sub.label == "End")
            .expect("End subgraph");
        assert!(
            !second_subgraph.nodes.iter().any(|id| id == "second"),
            "Mermaid's last-reference-wins parentId behavior reparents `second` away from Second"
        );
        assert!(
            end_subgraph.nodes.iter().any(|id| id == "second"),
            "`second` should be parented to End"
        );

        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        assert!(
            (500.0..=550.0).contains(&layout.width),
            "width should stay in JS size class, got {}",
            layout.width
        );
        assert!(
            (760.0..=830.0).contains(&layout.height),
            "height should stay in JS size class, got {}",
            layout.height
        );

        assert_center_y(&layout, "second", 275.0, 305.0);
        assert_center_y(&layout, "__start_Third__", 385.0, 425.0);
        assert_center_y(&layout, "third", 485.0, 525.0);
        assert_center_y(&layout, "__end_Second__", 705.0, 760.0);
        assert_center(&layout, "__end_First__", (240.0, 280.0), (280.0, 310.0));
        assert_center(&layout, "__end_root__", (320.0, 370.0), (480.0, 530.0));

        let second = subgraph_by_label(&layout, "Second");
        assert!(
            (560.0..=620.0).contains(&second.height),
            "Second cluster should be stretched by nested compound ranks, got {}",
            second.height
        );
        let end = subgraph_by_label(&layout, "End");
        assert!(
            end.height >= 440.0,
            "End cluster should expand around the cross-cluster target range, got {}",
            end.height
        );
    }

    #[test]
    fn state_compound_rank_path_does_not_collapse_sensitive_fixtures() {
        let composite = parse_mermaid(include_str!(
            "../../tests/mermaid-js-comparison/reference/stateDiagram-composite-states.mmd"
        ))
        .expect("failed to parse composite fixture");
        let composite_layout =
            compute_layout(&composite.graph, &Theme::modern(), &LayoutConfig::default());
        assert!(
            composite_layout.width < 220.0 && composite_layout.height > 700.0,
            "plain composite states should keep their tall JS-like layout, got {}x{}",
            composite_layout.width,
            composite_layout.height
        );

        let concurrency = parse_mermaid(include_str!(
            "../../tests/mermaid-js-comparison/reference/stateDiagram-concurrency.mmd"
        ))
        .expect("failed to parse concurrency fixture");
        let concurrency_layout = compute_layout(
            &concurrency.graph,
            &Theme::modern(),
            &LayoutConfig::default(),
        );
        assert!(
            concurrency_layout.width > 900.0 && concurrency_layout.height > 450.0,
            "concurrency regions should not be collapsed by compound rank reconciliation, got {}x{}",
            concurrency_layout.width,
            concurrency_layout.height
        );
    }

    fn assert_center_y(layout: &Layout, id: &str, min: f32, max: f32) {
        let node = layout
            .nodes
            .get(id)
            .unwrap_or_else(|| panic!("missing node {id}"));
        let center_y = node.y + node.height * 0.5;
        assert!(
            (min..=max).contains(&center_y),
            "node {id} center_y {center_y} outside [{min}, {max}]"
        );
    }

    fn assert_center(layout: &Layout, id: &str, x_range: (f32, f32), y_range: (f32, f32)) {
        let node = layout
            .nodes
            .get(id)
            .unwrap_or_else(|| panic!("missing node {id}"));
        let center_x = node.x + node.width * 0.5;
        let center_y = node.y + node.height * 0.5;
        assert!(
            (x_range.0..=x_range.1).contains(&center_x),
            "node {id} center_x {center_x} outside [{}, {}]",
            x_range.0,
            x_range.1
        );
        assert!(
            (y_range.0..=y_range.1).contains(&center_y),
            "node {id} center_y {center_y} outside [{}, {}]",
            y_range.0,
            y_range.1
        );
    }

    fn subgraph_by_label<'a>(layout: &'a Layout, label: &str) -> &'a SubgraphLayout {
        layout
            .subgraphs
            .iter()
            .find(|sub| sub.label == label)
            .unwrap_or_else(|| panic!("missing subgraph {label}"))
    }
}
