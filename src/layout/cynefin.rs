use std::collections::BTreeMap;

use crate::ir::Graph;

use super::{DiagramData, Layout};

pub(super) fn compute_cynefin_layout(graph: &Graph) -> Layout {
    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width: 800.0,
        height: 600.0,
        diagram: DiagramData::Cynefin(graph.cynefin.clone()),
        acc_title: None,
        acc_descr: None,
    }
}

#[cfg(test)]
mod tests {
    use crate::{DiagramKind, LayoutConfig, Theme, compute_layout, parse_mermaid, render_svg};

    #[test]
    fn parses_and_renders_domains_items_and_transition() {
        let input = "cynefin-beta\ntitle Delivery decisions\ncomplex\n\"Explore options\"\nclear\n\"Follow checklist\"\ncomplex --> clear : \"Standardized\"";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Cynefin);
        assert_eq!(parsed.graph.cynefin.items.len(), 2);
        assert_eq!(parsed.graph.cynefin.transitions.len(), 1);
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        let svg = render_svg(&layout, &theme, &config);
        for label in [
            "Delivery decisions",
            "Complex",
            "Clear",
            "Explore options",
            "Follow checklist",
            "Standardized",
        ] {
            assert!(svg.contains(label), "missing {label}");
        }
        assert!(svg.contains("cynefin-arrow"));
    }
}
