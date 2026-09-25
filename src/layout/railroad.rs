use std::collections::BTreeMap;

use crate::ir::{Graph, RailroadExpr};

use super::{DiagramData, Layout};

#[derive(Clone, Copy)]
pub(crate) struct Size {
    pub width: f32,
    pub height: f32,
    pub baseline: f32,
}

pub(crate) fn measure(expression: &RailroadExpr) -> Size {
    match expression {
        RailroadExpr::Terminal(text) | RailroadExpr::Nonterminal(text) => Size {
            width: (text.chars().count() as f32 * 8.5 + 26.0).max(44.0),
            height: 34.0,
            baseline: 17.0,
        },
        RailroadExpr::Sequence(items) => {
            let sizes: Vec<_> = items.iter().map(measure).collect();
            let baseline = sizes.iter().map(|s| s.baseline).fold(0.0_f32, f32::max);
            Size {
                width: sizes.iter().map(|s| s.width).sum::<f32>()
                    + 24.0 * sizes.len().saturating_sub(1) as f32,
                height: baseline
                    + sizes
                        .iter()
                        .map(|s| s.height - s.baseline)
                        .fold(0.0_f32, f32::max),
                baseline,
            }
        }
        RailroadExpr::Choice(options) => {
            let sizes: Vec<_> = options.iter().map(measure).collect();
            Size {
                width: sizes.iter().map(|s| s.width).fold(0.0_f32, f32::max) + 60.0,
                height: sizes.iter().map(|s| s.height).sum::<f32>()
                    + 22.0 * options.len().saturating_sub(1) as f32
                    + 20.0,
                baseline: sizes[0].baseline + 10.0,
            }
        }
        RailroadExpr::Optional(inner) => {
            let size = measure(inner);
            Size {
                width: size.width + 60.0,
                height: size.height + 54.0,
                baseline: 19.0,
            }
        }
        RailroadExpr::ZeroOrMore(inner) => {
            let size = measure(inner);
            Size {
                width: size.width + 60.0,
                height: size.height + 80.0,
                baseline: size.baseline + 28.0,
            }
        }
        RailroadExpr::OneOrMore(inner) => {
            let size = measure(inner);
            Size {
                width: size.width + 60.0,
                height: size.height + 55.0,
                baseline: size.baseline + 10.0,
            }
        }
    }
}

pub(super) fn compute_railroad_layout(graph: &Graph) -> Layout {
    let mut width = 280.0_f32;
    let mut height = if graph.railroad.title.is_some() {
        66.0
    } else {
        28.0
    };
    for rule in &graph.railroad.rules {
        let size = measure(&rule.expression);
        width = width.max(size.width + 112.0);
        height += size.height + 70.0;
    }
    Layout {
        kind: graph.kind,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        subgraphs: Vec::new(),
        width,
        height,
        diagram: DiagramData::Railroad(graph.railroad.clone()),
        acc_title: None,
        acc_descr: None,
    }
}

#[cfg(test)]
mod tests {
    use crate::{DiagramKind, LayoutConfig, Theme, compute_layout, parse_mermaid, render_svg};

    #[test]
    fn renders_ebnf_choices_and_repetition() {
        let source = "railroad-ebnf-beta\nexpression = term, { (\"+\" | \"-\"), term } ;\nterm = factor, [\"*\"], factor+ ;";
        let parsed = parse_mermaid(source).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Railroad);
        assert_eq!(parsed.graph.railroad.rules.len(), 2);
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let layout = compute_layout(&parsed.graph, &theme, &config);
        let svg = render_svg(&layout, &theme, &config);
        for label in ["expression", "term", "factor", "+", "-"] {
            assert!(svg.contains(label), "missing {label}");
        }
        assert!(svg.contains("railroad-loop"));
        assert!(svg.contains("railroad-choice"));
    }

    #[test]
    fn malformed_ebnf_is_rejected() {
        assert!(parse_mermaid("railroad-ebnf-beta\nrule = ('x' ;").is_err());
        assert!(parse_mermaid("railroad-ebnf-beta\nrule = 'x'").is_err());
    }

    #[test]
    fn accepts_each_railroad_dialect() {
        for source in [
            "railroad-abnf-beta\nvalue = word *( \"+\" word ) ;",
            "railroad-peg-beta\nvalue <- word (\"+\" word)* ;",
            "railroad-beta\nvalue = sequence(nonterminal(\"word\"), zeroOrMore(terminal(\"+\"))) ;",
        ] {
            let parsed = parse_mermaid(source).unwrap();
            assert_eq!(parsed.graph.kind, DiagramKind::Railroad);
            assert_eq!(parsed.graph.railroad.rules.len(), 1);
            let theme = Theme::modern();
            let config = LayoutConfig::default();
            let svg = render_svg(
                &compute_layout(&parsed.graph, &theme, &config),
                &theme,
                &config,
            );
            assert!(svg.contains("railroad-loop"), "{source}");
            assert!(svg.contains("word"), "{source}");
        }
    }

    #[test]
    fn rejects_unhandled_grammar_operators() {
        assert!(parse_mermaid("railroad-peg-beta\nrule <- &term ;").is_err());
        assert!(parse_mermaid("railroad-abnf-beta\nrule = 2*4term ;").is_err());
        assert!(parse_mermaid("railroad-beta\nrule = mystery(\"x\") ;").is_err());
    }
}
