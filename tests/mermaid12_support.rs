use mermaid_rs_renderer::ir::{DiagramKind, Direction, NodeShape};
use mermaid_rs_renderer::{LayoutConfig, Theme, compute_layout, parse_mermaid, render};

#[test]
fn mermaid12_documentation_examples_parse_and_render() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/mermaid-js-comparison/reference");
    let mut examples = 0;
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy();
        if !name.starts_with("usecase-") && !name.starts_with("agentflow-") {
            continue;
        }
        let input = std::fs::read_to_string(&path).unwrap();
        let parsed = parse_mermaid(&input).unwrap_or_else(|error| panic!("{name}: {error}"));
        let expected = if name.starts_with("usecase-") {
            DiagramKind::UseCase
        } else {
            DiagramKind::Agentflow
        };
        assert_eq!(parsed.graph.kind, expected, "{name}");
        assert!(!parsed.graph.nodes.is_empty(), "{name}: no nodes");
        let svg = render(&input).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(
            svg.contains("<svg") && svg.ends_with("</svg>"),
            "{name}: invalid SVG"
        );
        assert!(
            !svg.contains("NaN") && !svg.contains("Infinity"),
            "{name}: invalid geometry"
        );
        examples += 1;
    }
    assert!(examples >= 36, "Missing Mermaid 12 documentation examples");
}

#[test]
fn usecase_keeps_literal_text_and_hollow_generalization() {
    let input = r#"---
config:
  usecase:
    actorFontWeight: bold
---
usecase-beta
actor Staff@{business: true} <<Employee>>
Login("First\nSecond #quot;text#quot;")
Base(Base case)
Login --|> Base
Staff --> Login
"#;
    let parsed = parse_mermaid(input).unwrap();
    assert_eq!(parsed.graph.direction, Direction::LeftRight);
    let layout = compute_layout(
        &parsed.graph,
        &Theme::mermaid_default(),
        &LayoutConfig::default(),
    );
    assert_eq!(layout.nodes["Login"].label.lines.len(), 1);
    assert_eq!(
        layout.nodes["Login"].label.lines[0].text(),
        "First\\nSecond \"text\""
    );
    let svg = render(input).unwrap();
    assert!(svg.contains("marker-end=\"url(#arrow-class-open-0)\""));
    assert!(svg.contains("Business Actor «Employee» Staff"));
    assert!(svg.contains("font-weight=\"bold\""));
}

#[test]
fn usecase_rejects_boundary_relationships_and_identifier_collisions() {
    assert!(
        parse_mermaid(
            "usecase-beta\nsystemBoundary App[Application]\nLogin(Sign in)\nend\nApp --> Login"
        )
        .is_err()
    );
    assert!(parse_mermaid("usecase-beta\nA e@--> B\nactor e").is_err());
    assert!(parse_mermaid("usecase-beta\nA e@--> B\nA e@--> C").is_err());
}

#[test]
fn usecase_quoted_class_delimiters_are_label_and_json_content() {
    let parsed = parse_mermaid("usecase-beta\nExample(\"Label ::: text\"):::highlight\njson Data@{\"text\":\"value ::: text\"}:::highlight").unwrap();
    assert_eq!(parsed.graph.nodes["Example"].label, "Label ::: text");
    assert_eq!(parsed.graph.node_classes["Example"], ["highlight"]);
    assert_eq!(
        parsed.graph.usecase.json_tables["Data"][0].1,
        "value ::: text"
    );
}

#[test]
fn usecase_json_scalar_arrays_show_the_key_once() {
    let parsed = parse_mermaid(
        "usecase-beta\njson Data@{\"colors\":[\"Red\",\"Green\"],\"items\":[{\"name\":\"Book\"}]}",
    )
    .unwrap();
    assert_eq!(
        parsed.graph.usecase.json_tables["Data"],
        vec![
            ("colors".to_string(), "Red".to_string()),
            (String::new(), "Green".to_string()),
            ("items[0].name".to_string(), "Book".to_string()),
        ]
    );
}

#[test]
fn agentflow_globals_connectors_and_accessibility_keep_their_roles() {
    let input = "agentflow-beta LR\naccTitle: Team\naccDescr {\nShared inputs for workers.\n}\nglobal\nShared --> Corpus\nend\nconnector api[API]\nflow work[Worker]\nRead@{shape: task, model: test}\nRead -.- Shared\nRead -.- Corpus\nend";
    let parsed = parse_mermaid(input).unwrap();
    let graph = &parsed.graph;
    assert_eq!(graph.nodes["api"].shape, NodeShape::RoundRect);
    assert_eq!(
        graph.acc_descr.as_deref(),
        Some("Shared inputs for workers.")
    );
    assert_eq!(graph.element_metadata["Read"]["model"], "test");
    assert_eq!(graph.subgraphs[0].nodes, ["Read"]);
    let svg = render(input).unwrap();
    assert!(svg.contains("Shared inputs for workers."));
}

#[test]
fn agentflow_reference_documents_and_collapsed_indicators_are_visible() {
    let input = "agentflow-beta\nDoc[Guide]@{shape: refdoc}\nflow worker[Worker]\nA --> B\nend\nworker@{view: collapsed}\nDoc --> A";
    let parsed = parse_mermaid(input).unwrap();
    let layout = compute_layout(
        &parsed.graph,
        &Theme::mermaid_default(),
        &LayoutConfig::default(),
    );
    let document = &layout.nodes["Doc"];
    assert_eq!(document.shape, NodeShape::ReferenceDocument);
    assert!(
        document.height < document.width,
        "Document should have a horizontal body"
    );
    assert_eq!(layout.nodes["worker"].shape, NodeShape::CollapsedGroup);
    assert!(!layout.nodes.contains_key("A") && !layout.nodes.contains_key("B"));
    let svg = render(input).unwrap();
    assert_eq!(svg.matches("r=\"2.5\"").count(), 3);
    assert!(svg.contains("stroke-dasharray=\"3,3\""));
}

#[test]
fn flowchart_images_are_nodes_and_keep_complete_data_urls() {
    for name in [
        "flowchart-image-shape",
        "flowchart-image-with-aspect-ratio-constraint",
    ] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/mermaid-js-comparison/reference/{name}.mmd"));
        let input = std::fs::read_to_string(path).unwrap();
        let parsed = parse_mermaid(&input).unwrap();
        assert_eq!(parsed.graph.nodes.len(), 1);
        let node = &parsed.graph.nodes["A"];
        assert_eq!(node.shape, NodeShape::Image);
        assert!(
            node.img
                .as_ref()
                .unwrap()
                .starts_with("data:image/png;base64,")
        );
        let layout = compute_layout(
            &parsed.graph,
            &Theme::mermaid_default(),
            &LayoutConfig::default(),
        );
        let expected_width = if node.constraint.as_deref() == Some("on") {
            if cfg!(feature = "png") { 200.0 } else { 100.0 }
        } else {
            120.0
        };
        assert_eq!(layout.nodes["A"].img_w, Some(expected_width));
        let svg = render(&input).unwrap();
        assert!(svg.contains("<image"));
        assert!(svg.contains("href=\"data:image/png;base64,"));
        assert!(svg.contains("preserveAspectRatio=\"none\""));
        #[cfg(feature = "png")]
        if node.constraint.as_deref() == Some("on") {
            assert_eq!(layout.nodes["A"].img_h, Some(100.0));
        }
    }
}
