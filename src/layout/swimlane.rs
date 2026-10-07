use std::collections::HashMap;

use crate::config::LayoutConfig;
use crate::ir::{Direction, Graph, NodeStyle};
use crate::theme::Theme;

use super::{
    DiagramData, Layout, LayoutStageMetrics, SubgraphLayout, TextBlock, compute_flowchart_layout,
};

/// Swimlanes use flowchart nodes and edges, but each top-level subgraph is a
/// fixed row or column. A normal flowchart layout can stagger those subgraphs
/// and lose the relationship between a task and its lane.
pub(super) fn compute_swimlane_layout(
    graph: &Graph,
    theme: &Theme,
    config: &LayoutConfig,
    stage_metrics: Option<&mut LayoutStageMetrics>,
) -> Layout {
    let mut layout = compute_flowchart_layout(graph, theme, config, stage_metrics);
    let horizontal = matches!(graph.direction, Direction::LeftRight | Direction::RightLeft);
    let reversed = matches!(graph.direction, Direction::RightLeft | Direction::BottomTop);

    let mut lane_of = HashMap::<String, usize>::new();
    for (lane, subgraph) in graph.subgraphs.iter().enumerate() {
        for id in &subgraph.nodes {
            lane_of.entry(id.clone()).or_insert(lane);
        }
    }
    let has_unassigned = graph.nodes.keys().any(|id| !lane_of.contains_key(id));
    let lane_count = graph.subgraphs.len() + usize::from(has_unassigned);
    if lane_count == 0 {
        layout.diagram = DiagramData::Swimlane {
            direction: graph.direction,
        };
        return layout;
    }
    if has_unassigned {
        for id in graph.nodes.keys() {
            lane_of.entry(id.clone()).or_insert(graph.subgraphs.len());
        }
        layout.subgraphs.push(SubgraphLayout {
            id: None,
            label: String::new(),
            label_block: TextBlock {
                lines: Vec::new(),
                width: 0.0,
                height: 0.0,
            },
            nodes: graph
                .nodes
                .keys()
                .filter(|id| lane_of.get(*id) == Some(&graph.subgraphs.len()))
                .cloned()
                .collect(),
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            style: NodeStyle::default(),
            icon: None,
        });
    }

    // An edge inside a lane advances to the next step. An edge crossing lanes
    // connects steps at the same rank, as in Mermaid's swimlane layout.
    let mut rank = graph
        .nodes
        .keys()
        .map(|id| (id.clone(), 0usize))
        .collect::<HashMap<_, _>>();
    let max_rank = graph.nodes.len().saturating_sub(1);
    for _ in 0..graph.nodes.len() {
        let mut changed = false;
        for edge in &graph.edges {
            let Some(&from_rank) = rank.get(&edge.from) else {
                continue;
            };
            let Some(&to_rank) = rank.get(&edge.to) else {
                continue;
            };
            let increment = usize::from(lane_of.get(&edge.from) == lane_of.get(&edge.to));
            let desired = from_rank.saturating_add(increment).min(max_rank);
            if desired > to_rank {
                rank.insert(edge.to.clone(), desired);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let rank_count = rank.values().copied().max().unwrap_or(0) + 1;
    let main_size = layout
        .nodes
        .values()
        .map(|node| if horizontal { node.width } else { node.height })
        .fold(0.0_f32, f32::max)
        .max(40.0);
    let cross_size = layout
        .nodes
        .values()
        .map(|node| if horizontal { node.height } else { node.width })
        .fold(0.0_f32, f32::max)
        .max(40.0);
    let label_size = layout
        .subgraphs
        .iter()
        .map(|lane| lane.label_block.width)
        .fold(0.0_f32, f32::max);
    let header = 32.0_f32;
    let pad = 24.0_f32;
    let gap = config.rank_spacing.max(48.0);
    let step = main_size + gap;
    let lane_cross = (cross_size + 2.0 * pad).max(if horizontal {
        label_size + 2.0 * pad
    } else {
        label_size + 16.0
    });
    let main_extent = header + 2.0 * pad + main_size + step * (rank_count - 1) as f32;

    for (lane_index, lane) in layout.subgraphs.iter_mut().enumerate().take(lane_count) {
        if horizontal {
            lane.x = 8.0;
            lane.y = 8.0 + lane_index as f32 * lane_cross;
            lane.width = main_extent;
            lane.height = lane_cross;
        } else {
            lane.x = 8.0 + lane_index as f32 * lane_cross;
            lane.y = 8.0;
            lane.width = lane_cross;
            lane.height = main_extent;
        }
    }

    for (id, node) in &mut layout.nodes {
        let Some(&lane_index) = lane_of.get(id) else {
            continue;
        };
        let Some(&node_rank) = rank.get(id) else {
            continue;
        };
        let display_rank = if reversed {
            rank_count - 1 - node_rank
        } else {
            node_rank
        };
        if horizontal {
            node.x =
                8.0 + header + pad + display_rank as f32 * step + (main_size - node.width) * 0.5;
            node.y = 8.0 + lane_index as f32 * lane_cross + (lane_cross - node.height) * 0.5;
        } else {
            node.x = 8.0 + lane_index as f32 * lane_cross + (lane_cross - node.width) * 0.5;
            node.y =
                8.0 + header + pad + display_rank as f32 * step + (main_size - node.height) * 0.5;
        }
    }

    for edge in &mut layout.edges {
        let (Some(from), Some(to)) = (layout.nodes.get(&edge.from), layout.nodes.get(&edge.to))
        else {
            continue;
        };
        let same_lane = lane_of.get(&edge.from) == lane_of.get(&edge.to);
        let (start, end, points) = if horizontal == same_lane {
            let forward = from.x <= to.x;
            let start = (
                if forward { from.x + from.width } else { from.x },
                from.y + from.height * 0.5,
            );
            let end = (
                if forward { to.x } else { to.x + to.width },
                to.y + to.height * 0.5,
            );
            let mid_x = (start.0 + end.0) * 0.5;
            (
                start,
                end,
                vec![start, (mid_x, start.1), (mid_x, end.1), end],
            )
        } else {
            let forward = from.y <= to.y;
            let start = (
                from.x + from.width * 0.5,
                if forward {
                    from.y + from.height
                } else {
                    from.y
                },
            );
            let end = (
                to.x + to.width * 0.5,
                if forward { to.y } else { to.y + to.height },
            );
            let mid_y = (start.1 + end.1) * 0.5;
            (
                start,
                end,
                vec![start, (start.0, mid_y), (end.0, mid_y), end],
            )
        };
        edge.points = points;
        edge.label_anchor = Some(((start.0 + end.0) * 0.5, (start.1 + end.1) * 0.5));
    }

    layout.width = if horizontal {
        main_extent + 16.0
    } else {
        lane_count as f32 * lane_cross + 16.0
    };
    layout.height = if horizontal {
        lane_count as f32 * lane_cross + 16.0
    } else {
        main_extent + 16.0
    };
    layout.diagram = DiagramData::Swimlane {
        direction: graph.direction,
    };
    layout
}

#[cfg(test)]
mod tests {
    use crate::config::LayoutConfig;
    use crate::ir::{DiagramKind, Direction};
    use crate::layout::compute_layout;
    use crate::parser::parse_mermaid;
    use crate::render::render_svg;
    use crate::theme::Theme;

    const BODY: &str = "subgraph Sales[Sales]\n A[Request]\n B[Approve]\n A --> B\nend\nsubgraph Ops[Operations]\n C[Deploy]\n D[Monitor]\n C --> D\nend\nB --> C";

    #[test]
    fn swimlane_lr_places_lanes_in_rows_and_connects_across_them() {
        let input = format!("swimlane-beta LR\n{BODY}");
        let parsed = parse_mermaid(&input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Swimlane);
        assert_eq!(parsed.graph.direction, Direction::LeftRight);
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        assert_eq!(layout.subgraphs.len(), 2);
        assert_eq!(layout.subgraphs[0].x, layout.subgraphs[1].x);
        assert!(layout.subgraphs[0].y < layout.subgraphs[1].y);
        assert!(layout.nodes["A"].x < layout.nodes["B"].x);
        assert!(layout.nodes["B"].y < layout.nodes["C"].y);
        let crossing = layout
            .edges
            .iter()
            .find(|edge| edge.from == "B" && edge.to == "C")
            .unwrap();
        assert!(crossing.points.len() >= 2);
        let svg = render_svg(&layout, &theme, &config);
        assert!(svg.contains("rotate(-90"));
        assert!(svg.contains("Operations"));
    }

    #[test]
    fn swimlane_tb_places_lanes_in_columns_and_aligns_handoff_rank() {
        let input = format!("swimlane-beta\n{BODY}");
        let parsed = parse_mermaid(&input).unwrap();
        let layout = compute_layout(&parsed.graph, &Theme::modern(), &LayoutConfig::default());
        assert_eq!(layout.subgraphs[0].y, layout.subgraphs[1].y);
        assert!(layout.subgraphs[0].x < layout.subgraphs[1].x);
        assert!(layout.nodes["A"].y < layout.nodes["B"].y);
        assert!((layout.nodes["B"].y - layout.nodes["C"].y).abs() < 1.0);
    }
}
