use std::collections::BTreeMap;

use crate::config::LayoutConfig;
use crate::ir::{CynefinDomainName, Graph};
use crate::theme::Theme;

use super::{CynefinLayout, DiagramData, Layout};

pub fn compute_cynefin_layout(graph: &Graph, _theme: &Theme, config: &LayoutConfig) -> Layout {
    let cfg = &config.cynefin;
    let diagram_width = cfg.width.max(1.0);
    let diagram_height = cfg.height.max(1.0);
    let padding = cfg.padding.max(0.0);
    let width = diagram_width + padding * 2.0;
    let height = diagram_height + padding * 2.0;
    let mut domains: BTreeMap<CynefinDomainName, Vec<String>> = BTreeMap::new();
    for (name, domain) in &graph.cynefin.domains {
        domains.insert(
            *name,
            domain.items.iter().map(|item| item.label.clone()).collect(),
        );
    }

    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width,
        height,
        diagram: DiagramData::Cynefin(CynefinLayout {
            width,
            height,
            diagram_width,
            diagram_height,
            padding,
            show_domain_descriptions: cfg.show_domain_descriptions,
            boundary_amplitude: cfg.boundary_amplitude.clamp(0.0, 50.0),
            use_max_width: cfg.use_max_width,
            title: graph
                .cynefin
                .title
                .clone()
                .or_else(|| graph.diagram_title.clone()),
            domains,
            transitions: graph.cynefin.transitions.clone(),
        }),
        acc_title: graph.acc_title.clone(),
        acc_descr: graph.acc_descr.clone(),
    }
}
