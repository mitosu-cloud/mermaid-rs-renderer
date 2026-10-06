use std::collections::BTreeMap;

use crate::config::LayoutConfig;
use crate::ir::Graph;
use crate::theme::Theme;

use super::{DiagramData, Layout};

pub(super) fn compute_radar_layout(
    graph: &Graph,
    _theme: &Theme,
    _config: &LayoutConfig,
) -> Layout {
    // Mermaid's default plot is 600 square with 50 px margins on each side.
    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width: 700.0,
        height: 700.0,
        acc_title: None,
        acc_descr: None,
        diagram: DiagramData::Radar(graph.radar.clone()),
    }
}
