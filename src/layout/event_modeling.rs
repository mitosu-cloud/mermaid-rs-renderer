use std::collections::BTreeMap;

use crate::ir::Graph;

use super::{DiagramData, Layout};

pub(super) fn compute_event_modeling_layout(graph: &Graph) -> Layout {
    let frames = graph.event_modeling.frames.len();
    let width = (370.0 + frames.saturating_sub(1) as f32 * 145.0).max(650.0);
    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width,
        height: 400.0,
        diagram: DiagramData::EventModeling(graph.event_modeling.clone()),
        acc_title: None,
        acc_descr: None,
    }
}

#[cfg(test)]
mod tests {
    use crate::{DiagramKind, LayoutConfig, Theme, compute_layout, parse_mermaid, render_svg};

    #[test]
    fn renders_timeframes_in_three_semantic_lanes() {
        let source = "eventmodeling\ntf 01 ui CartUI\ntimeframe 02 command AddItem\ntf 03 evt ItemAdded\ntf 04 rmo CartContents";
        let parsed = parse_mermaid(source).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::EventModeling);
        assert_eq!(parsed.graph.event_modeling.frames.len(), 4);
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        let svg = render_svg(&layout, &theme, &config);
        for label in [
            "UI/Automation",
            "Command/Read Model",
            "Events",
            "CartUI",
            "AddItem",
            "ItemAdded",
            "CartContents",
        ] {
            assert!(svg.contains(label), "missing {label}");
        }
        assert!(svg.contains("eventmodeling-arrow"));
    }

    #[test]
    fn rejects_unhandled_event_modeling_statements() {
        assert!(parse_mermaid("eventmodeling\ntf 01 ui CartUI\ndata CartUI { a: 1 }").is_err());
    }
}
