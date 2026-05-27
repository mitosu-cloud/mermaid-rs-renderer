use crate::ir::{DiagramKind, Direction, Graph, NodeStyle, Subgraph};
use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{HashMap, VecDeque};

type NodeTokenParts = (
    String,
    Option<String>,
    Option<crate::ir::NodeShape>,
    Vec<String>,
    bool, // markdown_label
);

static HEADER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(flowchart|graph)\s+(\w+)").unwrap());
static SUBGRAPH_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^subgraph\s+(.*)$").unwrap());
static INIT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^%%\{\s*init\s*:\s*(\{.*\})\s*\}%%").unwrap());
static PIPE_LABEL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(?P<left>.+?)\s*(?P<arrow><[-.=ox]*[-=]+[-.=ox]*>|<[-.=ox]*[-=]+|[-.=ox]*[-=]+>|[-.=ox]*[-=]+)\|(?P<label>.+?)\|\s*(?P<right>.+)$",
    )
    .unwrap()
});
static QUOTED_LABEL_ARROW_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"^(?P<left>.+?)\s*(?P<start><)?(?P<dash1>[-.=ox]*[-=]+[-.=ox]*)\s+"(?P<label>[^"]+)"\s+(?P<dash2>[-.=ox]*[-=]+[-.=ox]*)(?P<end>>)?\s*(?P<right>.+)$"#,
    )
    .unwrap()
});
static LABEL_ARROW_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(?P<left>.+?)\s*(?P<start><)?(?P<dash1>[-.=ox]*[-=]+[-.=ox]*)\s+(?P<label>.+?)\s+(?P<dash2>[-.=ox]*[-=]+[-.=ox]*)(?P<end>>)?\s*(?P<right>.+)$",
    )
    .unwrap()
});
static COMPACT_DOTTED_LABEL_ARROW_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(?P<left>.+?)\s*(?P<start><)?(?P<dash1>[-=ox]*[-=]+[-=ox]*)\.(?P<label>[^<>=|].*?)\.(?P<dash2>[-.=ox]*[-=]+[-.=ox]*)(?P<end>>)?\s*(?P<right>.+)$",
    )
    .unwrap()
});
static ARROW_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(?P<left>.+?)\s*(?P<arrow><[-.=ox]*[-=]+[-.=ox]*>|<[-.=ox]*[-=]+|[-.=ox]*[-=]+>|[-.=ox]*[-=]+|~+)\s*(?P<right>.+)$",
    )
    .unwrap()
});
static ARROW_TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<[-.=ox]*[-=]+[-.=ox]*>|<[-.=ox]*[-=]+|[-.=ox]*[-=]+>|[-.=ox]*[-=]+|~+").unwrap()
});
static CYNEFIN_TRANSITION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?i)^(complex|complicated|clear|chaotic|confusion)\s*-->\s*(complex|complicated|clear|chaotic|confusion)(?:\s*:\s*(.+?))?\s*$"#,
    )
    .unwrap()
});

#[derive(Debug, Default)]
pub struct ParseOutput {
    pub graph: Graph,
    pub init_config: Option<serde_json::Value>,
}

pub fn parse_mermaid(input: &str) -> Result<ParseOutput> {
    match detect_diagram_kind(input) {
        DiagramKind::Class => parse_class_diagram(input),
        DiagramKind::State => parse_state_diagram(input),
        DiagramKind::Sequence => parse_sequence_diagram(input),
        DiagramKind::Er => parse_er_diagram(input),
        DiagramKind::Pie => parse_pie_diagram(input),
        DiagramKind::Mindmap => parse_mindmap_diagram(input),
        DiagramKind::Journey => parse_journey_diagram(input),
        DiagramKind::Timeline => parse_timeline_diagram(input),
        DiagramKind::Gantt => parse_gantt_diagram(input),
        DiagramKind::Requirement => parse_requirement_diagram(input),
        DiagramKind::GitGraph => parse_gitgraph_diagram(input),
        DiagramKind::C4 => parse_c4_diagram(input),
        DiagramKind::Sankey => parse_sankey_diagram(input),
        DiagramKind::Quadrant => parse_quadrant_diagram(input),
        DiagramKind::ZenUML => parse_zenuml_diagram(input),
        DiagramKind::Block => parse_block_diagram(input),
        DiagramKind::Packet => parse_packet_diagram(input),
        DiagramKind::Kanban => parse_kanban_diagram(input),
        DiagramKind::Architecture => parse_architecture_diagram(input),
        DiagramKind::Radar => parse_radar_diagram(input),
        DiagramKind::Treemap => parse_treemap_diagram(input),
        DiagramKind::XYChart => parse_xy_chart_diagram(input),
        DiagramKind::Venn => parse_venn_diagram(input),
        DiagramKind::TreeView => parse_tree_view_diagram(input),
        DiagramKind::Ishikawa => parse_ishikawa_diagram(input),
        DiagramKind::Wardley => parse_wardley_diagram(input),
        DiagramKind::EventModeling => parse_eventmodeling_diagram(input),
        DiagramKind::Cynefin => parse_cynefin_diagram(input),
        DiagramKind::Flowchart => parse_flowchart(input),
    }
}

fn detect_diagram_kind(input: &str) -> DiagramKind {
    // Skip YAML frontmatter if present.
    let input = extract_yaml_frontmatter(input).1;
    for raw_line in input.lines() {
        let trimmed_line = raw_line.trim();
        if trimmed_line.is_empty() {
            continue;
        }
        if trimmed_line.starts_with("%%") {
            continue;
        }
        if trimmed_line.starts_with("%%{") {
            continue;
        }
        let without_comment = strip_trailing_comment(trimmed_line);
        if without_comment.is_empty() {
            continue;
        }
        let lower = without_comment.to_ascii_lowercase();
        if lower.starts_with("sequencediagram") {
            return DiagramKind::Sequence;
        }
        if lower.starts_with("classdiagram") {
            return DiagramKind::Class;
        }
        if lower.starts_with("statediagram") {
            return DiagramKind::State;
        }
        if lower.starts_with("erdiagram") {
            return DiagramKind::Er;
        }
        if lower.starts_with("pie") {
            return DiagramKind::Pie;
        }
        if lower.starts_with("mindmap") {
            return DiagramKind::Mindmap;
        }
        if lower.starts_with("journey") {
            return DiagramKind::Journey;
        }
        if lower.starts_with("timeline") {
            return DiagramKind::Timeline;
        }
        if lower.starts_with("gantt") {
            return DiagramKind::Gantt;
        }
        if lower.starts_with("requirementdiagram") {
            return DiagramKind::Requirement;
        }
        if lower.starts_with("gitgraph") {
            return DiagramKind::GitGraph;
        }
        if lower.starts_with("c4") {
            return DiagramKind::C4;
        }
        if lower.starts_with("sankey") {
            return DiagramKind::Sankey;
        }
        if lower.starts_with("quadrantchart") {
            return DiagramKind::Quadrant;
        }
        if lower.starts_with("zenuml") {
            return DiagramKind::ZenUML;
        }
        if lower.starts_with("block") {
            return DiagramKind::Block;
        }
        if lower.starts_with("packet") {
            return DiagramKind::Packet;
        }
        if lower.starts_with("kanban") {
            return DiagramKind::Kanban;
        }
        if lower.starts_with("architecture") {
            return DiagramKind::Architecture;
        }
        if lower.starts_with("radar") {
            return DiagramKind::Radar;
        }
        if lower.starts_with("treemap") {
            return DiagramKind::Treemap;
        }
        if lower.starts_with("xychart") {
            return DiagramKind::XYChart;
        }
        if lower.starts_with("venn") {
            return DiagramKind::Venn;
        }
        if lower.starts_with("treeview") {
            return DiagramKind::TreeView;
        }
        if lower.starts_with("ishikawa") {
            return DiagramKind::Ishikawa;
        }
        if lower.starts_with("wardley") {
            return DiagramKind::Wardley;
        }
        if lower.starts_with("eventmodeling") {
            return DiagramKind::EventModeling;
        }
        if lower.starts_with("cynefin-beta") {
            return DiagramKind::Cynefin;
        }
        if lower.starts_with("flowchart") || lower.starts_with("graph") {
            return DiagramKind::Flowchart;
        }
    }
    DiagramKind::Flowchart
}

/// Try to extract YAML frontmatter delimited by `---` lines. Returns the
/// parsed `serde_json::Value` (YAML is a superset of JSON, so we convert) and
/// the remaining input without the frontmatter block.
fn extract_yaml_frontmatter(input: &str) -> (Option<serde_json::Value>, &str) {
    let trimmed = input.trim_start();
    if !trimmed.starts_with("---") {
        return (None, input);
    }
    // Find the opening `---` line.
    let after_open = &trimmed[3..];
    let after_open = after_open
        .strip_prefix('\n')
        .unwrap_or(after_open.strip_prefix("\r\n").unwrap_or(after_open));
    // Find the closing `---`.
    if let Some(close_pos) = after_open.find("\n---") {
        let yaml_block = &after_open[..close_pos];
        let rest_start = close_pos + 4; // skip "\n---"
        let rest = if rest_start < after_open.len() {
            &after_open[rest_start..]
        } else {
            ""
        };
        // Parse the YAML block. The frontmatter should be a mapping; the
        // official Mermaid spec nests everything under a `config:` key.
        if let Ok(yaml_val) = serde_yaml::from_str::<serde_json::Value>(yaml_block) {
            // If the YAML has a top-level `config` key, unwrap it so it looks
            // like an %%{init: ...}%% value (which has `theme`, `themeVariables`,
            // diagram-specific keys at the top level).
            let config_val = if let Some(inner) = yaml_val.get("config") {
                inner.clone()
            } else {
                yaml_val
            };
            return (Some(config_val), rest);
        }
        return (None, rest);
    }
    (None, input)
}

fn extract_yaml_frontmatter_title(input: &str) -> Option<String> {
    let trimmed = input.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after_open = &trimmed[3..];
    let after_open = after_open
        .strip_prefix('\n')
        .unwrap_or(after_open.strip_prefix("\r\n").unwrap_or(after_open));
    let close_pos = after_open.find("\n---")?;
    let yaml_block = &after_open[..close_pos];
    let yaml_val = serde_yaml::from_str::<serde_json::Value>(yaml_block).ok()?;
    yaml_val
        .get("title")
        .and_then(|value| value.as_str())
        .map(|title| title.trim().to_string())
        .filter(|title| !title.is_empty())
}

fn preprocess_input(input: &str) -> Result<(Vec<String>, Option<serde_json::Value>)> {
    let (yaml_config, input) = extract_yaml_frontmatter(input);
    let mut init_config: Option<serde_json::Value> = yaml_config;
    let mut lines = Vec::new();

    for raw_line in input.lines() {
        let trimmed_line = raw_line.trim();
        if trimmed_line.is_empty() {
            continue;
        }
        if let Some(caps) = INIT_RE.captures(trimmed_line) {
            if let Some(json_str) = caps.get(1).map(|m| m.as_str()) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    init_config = Some(value);
                } else if let Ok(value) = json5::from_str::<serde_json::Value>(json_str) {
                    init_config = Some(value);
                }
            }
            continue;
        }
        if trimmed_line.starts_with("%%") {
            continue;
        }
        let without_comment = strip_trailing_comment(trimmed_line);
        if without_comment.is_empty() {
            continue;
        }
        lines.push(without_comment.to_string());
    }

    Ok((lines, init_config))
}

fn preprocess_input_keep_indent(input: &str) -> Result<(Vec<String>, Option<serde_json::Value>)> {
    let (yaml_config, input) = extract_yaml_frontmatter(input);
    let mut init_config: Option<serde_json::Value> = yaml_config;
    let mut lines = Vec::new();

    for raw_line in input.lines() {
        let trimmed_line = raw_line.trim();
        if trimmed_line.is_empty() {
            continue;
        }
        if let Some(caps) = INIT_RE.captures(trimmed_line) {
            if let Some(json_str) = caps.get(1).map(|m| m.as_str()) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    init_config = Some(value);
                } else if let Ok(value) = json5::from_str::<serde_json::Value>(json_str) {
                    init_config = Some(value);
                }
            }
            continue;
        }
        if trimmed_line.starts_with("%%") {
            continue;
        }
        let without_comment = strip_trailing_comment_keep_indent(raw_line);
        if without_comment.trim().is_empty() {
            continue;
        }
        lines.push(without_comment);
    }

    Ok((lines, init_config))
}

fn parse_flowchart(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Flowchart;
    let mut subgraph_stack: Vec<usize> = Vec::new();

    let (lines, init_config) = preprocess_input(input)?;
    let lines = join_flowchart_multiline_statements(lines);

    for raw_line in lines {
        for line in split_statements(&raw_line) {
            if line.is_empty() {
                continue;
            }

            if let Some(caps) = HEADER_RE.captures(&line) {
                if let Some(dir) = caps.get(2).and_then(|m| Direction::from_token(m.as_str())) {
                    graph.direction = dir;
                }
                continue;
            }

            if line == "end" {
                subgraph_stack.pop();
                continue;
            }

            if let Some(caps) = SUBGRAPH_RE.captures(&line) {
                let rest = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let (id, label, classes, md) = parse_subgraph_header(rest);
                graph.subgraphs.push(Subgraph {
                    id: id.clone(),
                    label,
                    nodes: Vec::new(),
                    direction: None,
                    icon: None,
                    markdown_label: md,
                });
                subgraph_stack.push(graph.subgraphs.len() - 1);
                if let Some(id) = id {
                    apply_subgraph_classes(&mut graph, &id, &classes);
                }
                continue;
            }

            if let Some(direction) = parse_direction_line(&line) {
                if let Some(idx) = subgraph_stack.last().copied() {
                    if let Some(sub) = graph.subgraphs.get_mut(idx) {
                        sub.direction = Some(direction);
                    }
                } else {
                    graph.direction = direction;
                }
                continue;
            }

            if line.starts_with("classDef") {
                parse_class_def(&line, &mut graph);
                continue;
            }

            if line.starts_with("class ") {
                parse_class_line(&line, &mut graph);
                continue;
            }

            if line.starts_with("style ") {
                parse_style_line(&line, &mut graph);
                continue;
            }

            if line.starts_with("linkStyle") {
                parse_link_style_line(&line, &mut graph);
                continue;
            }

            if let Some((id, link)) = parse_click_line(&line) {
                graph.node_links.insert(id, link);
                continue;
            }

            if let Some(rest) = line.strip_prefix("accTitle") {
                let val = rest.trim_start_matches(':').trim();
                if !val.is_empty() {
                    graph.acc_title = Some(val.to_string());
                }
                continue;
            }
            if let Some(rest) = line.strip_prefix("accDescr") {
                let val = rest.trim_start_matches(':').trim();
                if !val.is_empty() {
                    graph.acc_descr = Some(val.to_string());
                }
                continue;
            }
            if line.starts_with("title ") {
                continue;
            }

            // Edge metadata: e1@{ curve: "linear" }
            if let Some((edge_id, curve)) = parse_edge_metadata_line(&line) {
                for edge in graph.edges.iter_mut().rev() {
                    if edge.id.as_deref() == Some(&edge_id) {
                        if let Some(c) = curve {
                            edge.curve = Some(c);
                        }
                        break;
                    }
                }
                continue;
            }

            if let Some(chain_lines) = split_edge_chain(&line) {
                let mut added = false;
                for edge_line in chain_lines {
                    added |= add_flowchart_edge(&edge_line, &mut graph, &subgraph_stack);
                }
                if added {
                    continue;
                }
            }

            if add_flowchart_edge(&line, &mut graph, &subgraph_stack) {
                continue;
            }

            if let Some((node_id, node_label, node_shape, node_classes, node_md)) =
                parse_node_only(&line)
            {
                graph.ensure_node_md(&node_id, node_label, node_shape, node_md);
                apply_flowchart_legacy_icon_label(&mut graph, &node_id);
                apply_node_classes(&mut graph, &node_id, &node_classes);
                apply_at_node_metadata(&mut graph, &line);
                add_node_to_subgraphs(&mut graph, &subgraph_stack, &node_id);
            }
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn join_flowchart_multiline_statements(lines: Vec<String>) -> Vec<String> {
    let mut joined = Vec::with_capacity(lines.len());
    let mut current: Option<String> = None;

    for line in lines {
        if let Some(acc) = current.as_mut() {
            acc.push('\n');
            acc.push_str(line.trim());
            if !flowchart_statement_needs_continuation(acc) {
                joined.push(current.take().unwrap());
            }
            continue;
        }

        if flowchart_statement_needs_continuation(&line) {
            current = Some(line);
        } else {
            joined.push(line);
        }
    }

    if let Some(acc) = current {
        joined.push(acc);
    }

    joined
}

fn flowchart_statement_needs_continuation(line: &str) -> bool {
    let mut square_depth = 0i32;
    let mut paren_depth = 0i32;
    let mut curly_depth = 0i32;
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in line.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }

        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            continue;
        }

        if ch == '"' {
            quote = Some(ch);
            continue;
        }

        match ch {
            '[' => square_depth += 1,
            ']' if square_depth > 0 => square_depth -= 1,
            '(' => paren_depth += 1,
            ')' if paren_depth > 0 => paren_depth -= 1,
            '{' => curly_depth += 1,
            '}' if curly_depth > 0 => curly_depth -= 1,
            _ => {}
        }
    }

    quote.is_some() || square_depth > 0 || paren_depth > 0 || curly_depth > 0
}

/// Parse a standalone edge metadata line like `e1@{ curve: "linear" }`.
fn parse_edge_metadata_line(line: &str) -> Option<(String, Option<crate::ir::CurveType>)> {
    let trimmed = line.trim();
    let at_pos = trimmed.find("@{")?;
    if !trimmed.ends_with('}') {
        return None;
    }
    let edge_id = trimmed[..at_pos].trim().to_string();
    if edge_id.is_empty() {
        return None;
    }
    let block = &trimmed[at_pos + 2..trimmed.len() - 1].trim();
    let mut curve: Option<crate::ir::CurveType> = None;
    let mut has_shape = false;
    for pair in block.split(',') {
        let pair = pair.trim();
        if let Some(colon) = pair.find(':') {
            let key = pair[..colon].trim().trim_matches('"').trim_matches('\'');
            let val = pair[colon + 1..]
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            if key == "curve" {
                curve = crate::ir::CurveType::from_name(val);
            }
            if key == "shape" {
                has_shape = true;
            }
            // "animate" is skipped (not applicable to static output)
        }
    }
    if has_shape {
        return None; // This is a node @{} declaration, not edge metadata
    }
    Some((edge_id, curve))
}

/// Apply @{...} metadata (img, icon, etc.) to a node in the graph.
fn apply_at_node_metadata(graph: &mut Graph, token: &str) {
    let (base, _classes) = split_inline_classes(token);
    let trimmed = base.trim();
    if let Some(meta) = parse_at_shape_syntax(trimmed) {
        if let Some(node) = graph.nodes.get_mut(&meta.id) {
            if meta.img.is_some() {
                node.img = meta.img;
            }
            if meta.img_w.is_some() {
                node.img_w = meta.img_w;
            }
            if meta.img_h.is_some() {
                node.img_h = meta.img_h;
            }
            if meta.img_pos.is_some() {
                node.img_pos = meta.img_pos;
            }
            if meta.constraint.is_some() {
                node.constraint = meta.constraint;
            }
            if meta.icon.is_some() {
                node.icon = meta.icon;
            }
        }
    }
}

fn apply_flowchart_legacy_icon_label(graph: &mut Graph, node_id: &str) {
    let Some(icon) = graph
        .nodes
        .get(node_id)
        .and_then(|node| flowchart_legacy_icon_label(&node.label))
    else {
        return;
    };
    if let Some(node) = graph.nodes.get_mut(node_id) {
        node.label.clear();
        node.markdown_label = false;
        node.icon = Some(icon);
    }
}

fn flowchart_legacy_icon_label(label: &str) -> Option<String> {
    let trimmed = label.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_whitespace) {
        return None;
    }
    let (prefix, icon) = trimmed.split_once(':')?;
    if !matches!(prefix, "fa" | "fab" | "fas" | "far" | "fal" | "fad" | "fak") {
        return None;
    }
    if !icon.starts_with("fa-") {
        return None;
    }
    Some(trimmed.to_string())
}

/// Extract edge ID prefix from an edge line (e.g., "A e1@--> B" → edge_id="e1").
fn extract_edge_id_prefix(line: &str) -> (Option<String>, String) {
    // Look for pattern: `<id>@` immediately before an arrow token
    // The edge ID appears as a word followed by @ before arrows like -->, -.->
    static EDGE_ID_RE: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"\b(\w+)@(--|-\.|-=|==|~~)").unwrap());
    if let Some(caps) = EDGE_ID_RE.captures(line) {
        let edge_id = caps.get(1).unwrap().as_str().to_string();
        let full_match = caps.get(0).unwrap();
        // Remove the edge ID prefix, keeping the arrow
        let cleaned = format!(
            "{}{}{}",
            &line[..full_match.start()],
            caps.get(2).unwrap().as_str(),
            &line[full_match.end()..]
        );
        (Some(edge_id), cleaned)
    } else {
        (None, line.to_string())
    }
}

fn add_flowchart_edge(line: &str, graph: &mut Graph, subgraph_stack: &[usize]) -> bool {
    let (edge_id, cleaned_line) = extract_edge_id_prefix(line);
    let Some((left, label_raw, right, edge_meta)) = parse_edge_line(&cleaned_line) else {
        return false;
    };
    let (label, edge_md) = match label_raw {
        Some(l) => {
            let (text, md) = strip_quotes_markdown(&l);
            (Some(text), md)
        }
        None => (None, false),
    };

    let sources = split_on_ampersand(&left);
    let targets = split_on_ampersand(&right);

    let mut source_ids = Vec::new();
    for source in sources {
        let (left_id, left_label, left_shape, left_classes, left_md) = parse_node_token(source);
        graph.ensure_node_md(&left_id, left_label, left_shape, left_md);
        apply_flowchart_legacy_icon_label(graph, &left_id);
        apply_node_classes(graph, &left_id, &left_classes);
        apply_at_node_metadata(graph, source);
        add_node_to_subgraphs(graph, subgraph_stack, &left_id);
        source_ids.push(left_id);
    }

    let mut target_ids = Vec::new();
    for target in targets {
        let (right_id, right_label, right_shape, right_classes, right_md) =
            parse_node_token(target);
        graph.ensure_node_md(&right_id, right_label, right_shape, right_md);
        apply_flowchart_legacy_icon_label(graph, &right_id);
        apply_node_classes(graph, &right_id, &right_classes);
        apply_at_node_metadata(graph, target);
        add_node_to_subgraphs(graph, subgraph_stack, &right_id);
        target_ids.push(right_id);
    }

    for left_id in &source_ids {
        for right_id in &target_ids {
            graph.edges.push(crate::ir::Edge {
                from: left_id.clone(),
                to: right_id.clone(),
                label: label.clone(),
                start_label: None,
                end_label: None,
                directed: edge_meta.directed,
                arrow_start: edge_meta.arrow_start,
                arrow_end: edge_meta.arrow_end,
                arrow_start_kind: edge_meta.arrow_start_kind,
                arrow_end_kind: edge_meta.arrow_end_kind,
                start_decoration: edge_meta.start_decoration,
                end_decoration: edge_meta.end_decoration,
                sequence_arrow_end: None,
                sequence_arrow_start: None,
                style: edge_meta.style,
                markdown_label: edge_md,
                id: edge_id.clone(),
                curve: None,
                arch_port_from: None,
                arch_port_to: None,
            });
        }
    }

    true
}

fn split_trailing_quoted(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim_end();
    let quote = trimmed.chars().last()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let mut iter = trimmed.char_indices().rev();
    let _ = iter.next();
    for (idx, ch) in iter {
        if ch == quote {
            let before = &trimmed[..idx];
            let value = &trimmed[idx + 1..trimmed.len() - 1];
            return Some((before, value));
        }
    }
    None
}

fn split_leading_quoted(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim_start();
    let mut iter = trimmed.char_indices();
    let Some((_, quote)) = iter.next() else {
        return None;
    };
    if quote != '"' && quote != '\'' {
        return None;
    }
    for (idx, ch) in iter {
        if ch == quote {
            let value = &trimmed[1..idx];
            let rest = &trimmed[idx + 1..];
            return Some((value, rest));
        }
    }
    None
}

fn split_multiplicity_left(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }
    if let Some((before, value)) = split_trailing_quoted(trimmed) {
        let before = before.trim();
        if !before.is_empty() && !value.is_empty() {
            return (before.to_string(), Some(value.to_string()));
        }
    }
    (trimmed.to_string(), None)
}

fn split_multiplicity_right(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }
    if let Some((value, rest)) = split_leading_quoted(trimmed) {
        let rest = rest.trim();
        if !rest.is_empty() && !value.is_empty() {
            return (rest.to_string(), Some(value.to_string()));
        }
    }
    (trimmed.to_string(), None)
}

fn parse_class_relation_line(
    line: &str,
) -> Option<(
    String,
    String,
    EdgeMeta,
    Option<String>,
    Option<String>,
    Option<String>,
)> {
    for token in class_relation_tokens() {
        if let Some(pos) = line.find(&token) {
            let left = line[..pos].trim();
            let right_part = line[pos + token.len()..].trim();
            if left.is_empty() || right_part.is_empty() {
                continue;
            }
            let (right, label) = split_label(right_part);
            let (left, start_label) = split_multiplicity_left(left);
            let (right, end_label) = split_multiplicity_right(&right);
            let meta = edge_meta_from_class_token(&token);
            return Some((left, right, meta, label, start_label, end_label));
        }
    }
    None
}

enum ClassLollipopSide {
    Start,
    End,
}

struct ClassLollipopRelation {
    class_token: String,
    interface_label: String,
    side: ClassLollipopSide,
    meta: EdgeMeta,
    label: Option<String>,
}

fn parse_class_lollipop_relation_line(line: &str) -> Option<ClassLollipopRelation> {
    const TOKENS: [(&str, crate::ir::EdgeStyle, ClassLollipopSide); 4] = [
        (
            "()--",
            crate::ir::EdgeStyle::Solid,
            ClassLollipopSide::Start,
        ),
        (
            "()..",
            crate::ir::EdgeStyle::Dotted,
            ClassLollipopSide::Start,
        ),
        ("--()", crate::ir::EdgeStyle::Solid, ClassLollipopSide::End),
        ("..()", crate::ir::EdgeStyle::Dotted, ClassLollipopSide::End),
    ];

    for (token, style, side) in TOKENS {
        if let Some(pos) = line.find(token) {
            let left = line[..pos].trim();
            let right_part = line[pos + token.len()..].trim();
            if left.is_empty() || right_part.is_empty() {
                continue;
            }
            let (right, label) = split_label(right_part);
            if right.trim().is_empty() {
                continue;
            }
            let (class_token, interface_label, side) = match side {
                ClassLollipopSide::Start => (
                    right.trim().to_string(),
                    strip_quotes(left),
                    ClassLollipopSide::Start,
                ),
                ClassLollipopSide::End => (
                    left.to_string(),
                    strip_quotes(right.trim()),
                    ClassLollipopSide::End,
                ),
            };
            if class_token.is_empty() || interface_label.is_empty() {
                continue;
            }
            let (start_decoration, end_decoration) = match side {
                ClassLollipopSide::Start => (Some(crate::ir::EdgeDecoration::Lollipop), None),
                ClassLollipopSide::End => (None, Some(crate::ir::EdgeDecoration::Lollipop)),
            };
            return Some(ClassLollipopRelation {
                class_token,
                interface_label,
                side,
                meta: EdgeMeta {
                    directed: false,
                    arrow_start: false,
                    arrow_end: false,
                    arrow_start_kind: None,
                    arrow_end_kind: None,
                    start_decoration,
                    end_decoration,
                    style,
                },
                label,
            });
        }
    }

    None
}

fn class_relation_tokens() -> Vec<String> {
    let relation_starts = ["<|", "<", "*", "o", ""];
    let relation_ends = ["|>", ">", "*", "o", ""];
    let line_types = ["--", ".."];

    let mut tokens = Vec::new();
    for start in relation_starts {
        for line in line_types {
            for end in relation_ends {
                tokens.push(format!("{start}{line}{end}"));
            }
        }
    }
    tokens.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    tokens.dedup();
    tokens
}

fn edge_meta_from_class_token(token: &str) -> EdgeMeta {
    let arrow_start = token.starts_with('<');
    let arrow_end = token.ends_with('>');
    let directed = arrow_start || arrow_end;
    let style = if token.contains("..") {
        crate::ir::EdgeStyle::Dotted
    } else {
        crate::ir::EdgeStyle::Solid
    };

    let mut start_decoration = None;
    let mut end_decoration = None;
    if token.starts_with('*') {
        start_decoration = Some(crate::ir::EdgeDecoration::DiamondFilled);
    }
    if token.ends_with('*') {
        end_decoration = Some(crate::ir::EdgeDecoration::DiamondFilled);
    }
    if token.starts_with('o') {
        start_decoration = Some(crate::ir::EdgeDecoration::Diamond);
    }
    if token.ends_with('o') {
        end_decoration = Some(crate::ir::EdgeDecoration::Diamond);
    }

    let mut arrow_start_kind = None;
    let mut arrow_end_kind = None;
    if token.starts_with("<|") {
        arrow_start_kind = Some(crate::ir::EdgeArrowhead::OpenTriangle);
    } else if token.starts_with('<') {
        arrow_start_kind = Some(crate::ir::EdgeArrowhead::ClassDependency);
    }
    if token.ends_with("|>") {
        arrow_end_kind = Some(crate::ir::EdgeArrowhead::OpenTriangle);
    } else if token.ends_with('>') {
        arrow_end_kind = Some(crate::ir::EdgeArrowhead::ClassDependency);
    }

    EdgeMeta {
        directed,
        arrow_start,
        arrow_end,
        arrow_start_kind,
        arrow_end_kind,
        start_decoration,
        end_decoration,
        style,
    }
}

fn parse_class_declaration(
    input: &str,
) -> Option<(
    String,
    Option<String>,
    Option<String>,
    bool,
    Vec<String>,
    Vec<String>,
)> {
    let mut rest = input.trim();
    if rest.is_empty() {
        return None;
    }

    let mut body: Option<String> = None;
    let mut open_body = false;
    if let Some(open_idx) = find_class_body_start(rest) {
        let header = rest[..open_idx].trim();
        let tail = rest[open_idx + 1..].trim();
        if let Some(close_idx) = tail.find('}') {
            let body_str = tail[..close_idx].trim();
            if !body_str.is_empty() {
                body = Some(body_str.to_string());
            }
        } else {
            open_body = true;
        }
        rest = header;
    }

    let (without_annotations, annotations) = extract_class_inline_annotations(rest);
    rest = without_annotations.trim();

    let (base, classes) = split_inline_classes(rest);
    rest = base.trim();

    let lower = rest.to_ascii_lowercase();
    if let Some(as_idx) = lower.find(" as ") {
        let label_part = rest[..as_idx].trim();
        let id_part = rest[as_idx + 4..].trim();
        if !id_part.is_empty() {
            let (id, generic) = split_class_generic_id(id_part);
            let label = class_display_label(&strip_quotes(label_part), generic.as_deref());
            return Some((id, Some(label), body, open_body, classes, annotations));
        }
    }

    if let Some((id, label)) = split_class_bracket_label(rest) {
        let (id, generic) = split_class_generic_id(&id);
        let label = class_display_label(&label, generic.as_deref());
        return Some((id, Some(label), body, open_body, classes, annotations));
    }

    if rest.starts_with('"') && rest.ends_with('"') {
        let label = strip_quotes(rest);
        return Some((
            label.clone(),
            Some(label),
            body,
            open_body,
            classes,
            annotations,
        ));
    }

    let (id, generic) = split_class_generic_id(rest);
    let label = generic
        .as_deref()
        .map(|generic| class_display_label(&id, Some(generic)));
    Some((id, label, body, open_body, classes, annotations))
}

fn extract_class_inline_annotations(input: &str) -> (String, Vec<String>) {
    let mut rest = input;
    let mut cleaned = String::new();
    let mut annotations = Vec::new();

    while let Some(start) = rest.find("<<") {
        cleaned.push_str(&rest[..start]);
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find(">>") else {
            cleaned.push_str(&rest[start..]);
            return (cleaned, annotations);
        };

        let annotation = after_start[..end].trim();
        if !annotation.is_empty() {
            annotations.push(annotation.to_string());
        }
        rest = &after_start[end + 2..];
    }

    cleaned.push_str(rest);
    (
        cleaned.split_whitespace().collect::<Vec<_>>().join(" "),
        annotations,
    )
}

fn class_annotation_member(annotation: &str) -> String {
    format!("<<{}>>", annotation.trim())
}

fn parse_class_annotation_member(entry: &str) -> Option<String> {
    let trimmed = entry.trim();
    let inner = trimmed.strip_prefix("<<")?.strip_suffix(">>")?.trim();
    if inner.is_empty() {
        None
    } else {
        Some(inner.to_string())
    }
}

fn class_annotation_label(annotation: &str) -> String {
    format!("\u{00ab}{}\u{00bb}", annotation.trim())
}

fn parse_class_annotation_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("<<")?;
    let end = rest.find(">>")?;
    let annotation = rest[..end].trim();
    if annotation.is_empty() {
        return None;
    }
    let target = rest[end + 2..].trim();
    if target.is_empty() {
        return None;
    }
    Some((target.to_string(), annotation.to_string()))
}

fn split_class_generic_id(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    let Some(first) = trimmed.find('~') else {
        return (strip_quotes(trimmed), None);
    };
    let Some(last) = trimmed.rfind('~') else {
        return (strip_quotes(trimmed), None);
    };
    if first == last || first == 0 {
        return (strip_quotes(trimmed), None);
    }

    let base = strip_quotes(trimmed[..first].trim());
    let generic = convert_tilde_generics(trimmed[first + 1..last].trim());
    if base.is_empty() || generic.is_empty() {
        return (strip_quotes(trimmed), None);
    }
    (base, Some(generic))
}

fn class_display_label(base: &str, generic: Option<&str>) -> String {
    match generic {
        Some(generic) if !generic.is_empty() => format!("{base}<{generic}>"),
        _ => base.to_string(),
    }
}

fn find_class_body_start(input: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut bracket_depth = 0usize;

    for (idx, ch) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }
        match ch {
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' if bracket_depth == 0 => return Some(idx),
            _ => {}
        }
    }

    None
}

fn split_class_bracket_label(input: &str) -> Option<(String, String)> {
    let trimmed = input.trim();
    let start = trimmed.find('[')?;
    if !trimmed.ends_with(']') {
        return None;
    }
    let id = trimmed[..start].trim();
    if id.is_empty() {
        return None;
    }
    let (label, _, _) = parse_shape_from_brackets(&trimmed[start..]);
    Some((strip_quotes(id), label))
}

fn parse_class_note_line(line: &str) -> Option<(Option<String>, String, bool)> {
    let lower = line.to_ascii_lowercase();
    if !lower.starts_with("note ") {
        return None;
    }

    let rest = line[4..].trim();
    if rest.is_empty() {
        return None;
    }

    let rest_lower = rest.to_ascii_lowercase();
    if rest_lower.starts_with("for ") {
        let target_and_text = rest[4..].trim();
        if let Some((target, text)) = split_trailing_quoted(target_and_text) {
            let (text, markdown) = strip_quotes_markdown(text);
            return Some((Some(strip_quotes(target.trim())), text, markdown));
        }

        let mut parts = target_and_text.splitn(2, char::is_whitespace);
        let target = parts.next()?.trim();
        let text = parts.next().unwrap_or("").trim();
        if target.is_empty() || text.is_empty() {
            return None;
        }
        let (text, markdown) = strip_quotes_markdown(text);
        return Some((Some(strip_quotes(target)), text, markdown));
    }

    let (text, markdown) = strip_quotes_markdown(rest);
    Some((None, text, markdown))
}

fn split_class_body(body: &str) -> Vec<String> {
    let mut entries = Vec::new();
    for part in body.split(';') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        for line in trimmed.lines() {
            let line_trim = line.trim();
            if !line_trim.is_empty() {
                entries.push(line_trim.to_string());
            }
        }
    }
    entries
}

fn normalize_class_method_signature(entry: &str) -> String {
    let trimmed = entry.trim();
    let Some(close_idx) = trimmed.find(')') else {
        return trimmed.to_string();
    };
    let (sig, rest) = trimmed.split_at(close_idx + 1);
    let rest = rest.trim();
    if rest.is_empty() {
        return trimmed.to_string();
    }
    if rest.starts_with(':') {
        return format!("{} {}", sig, rest);
    }
    if trimmed.contains("):") || trimmed.contains(") :") {
        return trimmed.to_string();
    }
    format!("{} : {}", sig, rest)
}

fn parse_class_member_line(line: &str) -> Option<(String, String)> {
    let (left, right) = line.split_once(':')?;
    let id = left.trim();
    let member = right.trim();
    if id.is_empty() || member.is_empty() {
        return None;
    }
    if id.contains(' ') {
        return None;
    }
    Some((id.to_string(), member.to_string()))
}

fn normalize_class_id(token: &str) -> (String, Option<String>) {
    let trimmed = token.trim();
    let (base, _) = split_inline_classes(trimmed);
    let trimmed = base.trim();
    if let Some((id, label)) = split_class_bracket_label(trimmed) {
        return (id, Some(label));
    }
    if trimmed.starts_with('"') && trimmed.ends_with('"') {
        let label = strip_quotes(trimmed);
        return (label.clone(), Some(label));
    }
    let (id, generic) = split_class_generic_id(trimmed);
    let label = generic
        .as_deref()
        .map(|generic| class_display_label(&id, Some(generic)));
    (id, label)
}

fn parse_state_alias_line(line: &str) -> Option<(String, String, Vec<String>)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("state ") {
        return None;
    }
    if trimmed.contains('{') {
        return None;
    }
    let rest = trimmed.trim_start_matches("state ").trim();
    if !rest.starts_with('"') {
        return None;
    }
    let end_quote = rest[1..].find('"')? + 1;
    let label = rest[1..end_quote].to_string();
    let remaining = rest[end_quote + 1..].trim();
    if !remaining.to_ascii_lowercase().starts_with("as ") {
        return None;
    }
    let id = remaining[3..].trim();
    let (id, classes) = parse_state_id_with_classes(id);
    if id.is_empty() {
        return None;
    }
    Some((id, label, classes))
}

fn parse_state_stereotype(line: &str) -> (String, Option<crate::ir::NodeShape>, Option<String>) {
    let trimmed = line.trim();
    if !trimmed.starts_with("state ") {
        return (trimmed.to_string(), None, None);
    }
    let Some(start) = trimmed.find("<<") else {
        return (trimmed.to_string(), None, None);
    };
    let Some(end) = trimmed[start + 2..].find(">>") else {
        return (trimmed.to_string(), None, None);
    };
    let stereo_raw = &trimmed[start + 2..start + 2 + end];
    let stereo = stereo_raw.trim().to_ascii_lowercase();

    let before = trimmed[..start].trim_end();
    let after = trimmed[start + 2 + end + 2..].trim_start();
    let cleaned = if after.is_empty() {
        before.to_string()
    } else if before.is_empty() {
        after.to_string()
    } else {
        format!("{before} {after}")
    };

    let (shape, label_override) = match stereo.as_str() {
        // Iter 271: choice stereotype is a pure-shape marker in JS — the
        // diamond renders without any text inside. Force empty label so the
        // diamond sizes from padding only (~28×28) instead of from the
        // ID "if_state" (~64×64).
        "choice" => (Some(crate::ir::NodeShape::Diamond), Some(String::new())),
        "fork" | "join" => (Some(crate::ir::NodeShape::ForkJoin), Some(String::new())),
        _ => (None, None),
    };

    (cleaned, shape, label_override)
}

fn parse_state_description_line(line: &str) -> Option<(String, String, Vec<String>)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.to_ascii_lowercase().starts_with("note ") {
        return None;
    }
    let rest = if trimmed.starts_with("state ") {
        trimmed[6..].trim()
    } else {
        trimmed
    };
    if rest.to_ascii_lowercase().contains(" as ") {
        return None;
    }
    let mut sep = None;
    let bytes = rest.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        if bytes[idx] == b':' {
            if idx + 2 < bytes.len() && bytes[idx + 1] == b':' && bytes[idx + 2] == b':' {
                idx += 3;
                continue;
            }
            sep = Some(idx);
            break;
        }
        idx += 1;
    }
    let sep = sep?;
    let (id_part, desc_part) = rest.split_at(sep);
    let desc_part = desc_part.get(1..).unwrap_or("");
    let (id, classes) = parse_state_id_with_classes(id_part.trim());
    let desc = strip_quotes(desc_part.trim());
    if id.is_empty() || desc.is_empty() {
        return None;
    }
    Some((id, desc, classes))
}

fn parse_state_id_with_classes(input: &str) -> (String, Vec<String>) {
    let (base, classes) = split_inline_classes(input);
    (strip_quotes(base.trim()), classes)
}

fn parse_state_note(line: &str) -> Option<(crate::ir::StateNotePosition, String, String)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("note ") {
        return None;
    }
    let rest = trimmed[4..].trim();
    let lower_rest = rest.to_ascii_lowercase();
    let (position, targets_part) = if lower_rest.starts_with("right of ") {
        (crate::ir::StateNotePosition::RightOf, rest[9..].trim())
    } else if lower_rest.starts_with("left of ") {
        (crate::ir::StateNotePosition::LeftOf, rest[8..].trim())
    } else {
        return None;
    };
    let (target, label) = targets_part.split_once(':')?;
    let target = target.trim();
    let label = label.trim();
    if target.is_empty() || label.is_empty() {
        return None;
    }
    Some((position, target.to_string(), label.to_string()))
}

/// Parse the header of a multi-line `note` block. Returns Some when the line
/// is `note right of X` or `note left of X` *without* a trailing `:` (which
/// would indicate the inline form, handled by `parse_state_note`).
///
/// The body lines that follow until `end note` are collected by the caller
/// and joined into a single label.
fn parse_state_note_block_header(line: &str) -> Option<(crate::ir::StateNotePosition, String)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("note ") {
        return None;
    }
    let rest = trimmed[4..].trim();
    let lower_rest = rest.to_ascii_lowercase();
    let (position, target_raw) = if lower_rest.starts_with("right of ") {
        (crate::ir::StateNotePosition::RightOf, rest[9..].trim())
    } else if lower_rest.starts_with("left of ") {
        (crate::ir::StateNotePosition::LeftOf, rest[8..].trim())
    } else {
        return None;
    };
    if target_raw.contains(':') {
        return None;
    }
    if target_raw.is_empty() {
        return None;
    }
    Some((position, target_raw.to_string()))
}

fn parse_state_transition(line: &str) -> Option<(String, EdgeMeta, String, Option<String>)> {
    let tokens = ["<-->", "<--", "-->", "<->", "->", "<-", "..>", "<.."];
    for token in tokens {
        if let Some(pos) = line.find(token) {
            let left = line[..pos].trim();
            let right_part = line[pos + token.len()..].trim();
            if left.is_empty() || right_part.is_empty() {
                continue;
            }
            let (right, label) = split_label(right_part);
            let meta = edge_meta_from_state_token(token);
            return Some((left.to_string(), meta, right.to_string(), label));
        }
    }
    None
}

fn edge_meta_from_state_token(token: &str) -> EdgeMeta {
    let arrow_start = token.contains('<');
    let arrow_end = token.contains('>');
    let directed = arrow_start || arrow_end;
    let style = if token.contains("..") {
        crate::ir::EdgeStyle::Dotted
    } else {
        crate::ir::EdgeStyle::Solid
    };
    EdgeMeta {
        directed,
        arrow_start,
        arrow_end,
        arrow_start_kind: None,
        arrow_end_kind: None,
        start_decoration: None,
        end_decoration: None,
        style,
    }
}

fn normalize_state_token(
    token: &str,
    is_start: bool,
    start_states: &mut HashMap<String, String>,
    end_states: &mut HashMap<String, String>,
    scope: &str,
) -> (String, crate::ir::NodeShape, Option<String>) {
    let trimmed = token.trim();
    if trimmed == "[*]" || trimmed == "*" {
        let (id, shape) = if is_start {
            // Start states are shared per scope. This lets fan-out/fan-in
            // patterns be recognized and rendered as fork/join bars.
            let id = start_states
                .entry(scope.to_string())
                .or_insert_with(|| format!("__start_{}__", scope))
                .clone();
            (id, crate::ir::NodeShape::Circle)
        } else {
            // End states are shared per scope - all X --> [*] in same scope go to same node
            let id = end_states
                .entry(scope.to_string())
                .or_insert_with(|| format!("__end_{}__", scope))
                .clone();
            (id, crate::ir::NodeShape::DoubleCircle)
        };
        return (id, shape, Some(String::new()));
    }
    (strip_quotes(trimmed), crate::ir::NodeShape::RoundRect, None)
}

fn parse_state_simple(line: &str) -> Option<(String, Vec<String>)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("state ") {
        return None;
    }
    if trimmed.contains('{') {
        return None;
    }
    let mut rest = trimmed.trim_start_matches("state ").trim();
    if rest.to_ascii_lowercase().contains(" as ") {
        return None;
    }
    if let Some(idx) = rest.find('{') {
        rest = rest[..idx].trim();
    }
    if rest.is_empty() {
        return None;
    }
    let (id, classes) = parse_state_id_with_classes(rest);
    if id.is_empty() {
        return None;
    }
    Some((id, classes))
}

fn parse_state_container_header(line: &str) -> Option<(Option<String>, String, String)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("state ") {
        return None;
    }
    let brace_idx = trimmed.find('{')?;
    let head = trimmed[..brace_idx].trim();
    let tail = trimmed[brace_idx + 1..].trim().to_string();

    let rest = head.trim_start_matches("state ").trim();
    if rest.is_empty() {
        return None;
    }

    if rest.starts_with('"') {
        let end_quote = rest[1..].find('"')? + 1;
        let label = rest[1..end_quote].to_string();
        let remaining = rest[end_quote + 1..].trim();
        if remaining.to_ascii_lowercase().starts_with("as ") {
            let id = remaining[3..].trim();
            if id.is_empty() {
                return None;
            }
            return Some((Some(id.to_string()), label, tail));
        }
        return Some((None, label, tail));
    }

    let lower = rest.to_ascii_lowercase();
    if let Some(as_idx) = lower.find(" as ") {
        let id_part = rest[..as_idx].trim();
        let label_part = rest[as_idx + 4..].trim();
        if id_part.is_empty() || label_part.is_empty() {
            return None;
        }
        let id = strip_quotes(id_part);
        let label = strip_quotes(label_part);
        return Some((Some(id), label, tail));
    }

    let id = strip_quotes(rest);
    Some((Some(id.clone()), id, tail))
}

/// Extract the value of a `"type" : "..."` key inside a participant
/// `@{ ... }` block. Lenient — accepts single/double quotes and arbitrary
/// whitespace.
fn parse_at_block_type(body: &str) -> Option<String> {
    parse_at_block_string(body, "type").map(|s| s.to_ascii_lowercase())
}

/// Generic extractor for `"<key>": "<value>"` from a participant `@{ ... }`
/// block. Preserves the value's casing.
fn parse_at_block_string(body: &str, key: &str) -> Option<String> {
    let needle_d = format!("\"{key}\"");
    let needle_s = format!("'{key}'");
    let lower = body.to_ascii_lowercase();
    let key_idx = lower.find(&needle_d).or_else(|| lower.find(&needle_s))?;
    let after_key = &body[key_idx + needle_d.len()..];
    let colon_idx = after_key.find(':')?;
    let after_colon = after_key[colon_idx + 1..].trim_start();
    let mut chars = after_colon.chars();
    let quote = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = chars.as_str();
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

fn parse_sequence_participant(
    line: &str,
) -> Option<(String, Option<String>, crate::ir::NodeShape)> {
    let lowered = line.to_ascii_lowercase();
    let keywords = [
        ("participant ", crate::ir::NodeShape::ActorBox),
        ("actor ", crate::ir::NodeShape::StickFigure),
        ("boundary ", crate::ir::NodeShape::Boundary),
        ("control ", crate::ir::NodeShape::Control),
        ("entity ", crate::ir::NodeShape::Entity),
        ("database ", crate::ir::NodeShape::Cylinder),
        ("collections ", crate::ir::NodeShape::Collections),
        ("queue ", crate::ir::NodeShape::Queue),
    ];
    let mut rest = None;
    let mut shape = crate::ir::NodeShape::ActorBox;
    for (keyword, keyword_shape) in keywords {
        if lowered.starts_with(keyword) {
            rest = Some(line[keyword.len()..].trim());
            shape = keyword_shape;
            break;
        }
    }
    let rest = rest?;
    if rest.is_empty() {
        return None;
    }

    // Strip an optional `@{ ... }` extended-type block. Extract `"type"` to
    // override the shape, and `"alias"` to use as the display label.
    let mut at_block_alias: Option<String> = None;
    let rest = match (rest.find("@{"), rest.rfind('}')) {
        (Some(start), Some(end)) if end > start => {
            let body = &rest[start + 2..end];
            if let Some(t) = parse_at_block_type(body) {
                shape = match t.as_str() {
                    "actor" => crate::ir::NodeShape::StickFigure,
                    "boundary" => crate::ir::NodeShape::Boundary,
                    "control" => crate::ir::NodeShape::Control,
                    "entity" => crate::ir::NodeShape::Entity,
                    "database" => crate::ir::NodeShape::Cylinder,
                    "collections" => crate::ir::NodeShape::Collections,
                    "queue" => crate::ir::NodeShape::Queue,
                    _ => shape,
                };
            }
            at_block_alias = parse_at_block_string(body, "alias");
            let mut stripped = String::with_capacity(rest.len());
            stripped.push_str(rest[..start].trim_end());
            let tail = rest[end + 1..].trim_start();
            if !tail.is_empty() {
                stripped.push(' ');
                stripped.push_str(tail);
            }
            stripped
        }
        _ => rest.to_string(),
    };
    let rest = rest.as_str();

    let lower_rest = rest.to_ascii_lowercase();
    if let Some(as_idx) = lower_rest.find(" as ") {
        let label_part = rest[..as_idx].trim();
        let id_part = rest[as_idx + 4..].trim();
        if id_part.is_empty() {
            return None;
        }
        let id = strip_quotes(label_part);
        let display_label = strip_quotes(id_part);
        return Some((id, Some(display_label), shape));
    }

    if rest.starts_with('"') && rest.ends_with('"') {
        let label = strip_quotes(rest);
        return Some((label.clone(), Some(label), shape));
    }

    // If no `as` was given, prefer the @{} block's alias as the display label.
    let id = strip_quotes(rest);
    let label = at_block_alias.or(None);
    Some((id, label, shape))
}

fn is_color_token(token: &str) -> bool {
    let lower = token.trim().to_ascii_lowercase();
    lower == "transparent"
        || lower.starts_with('#')
        || lower.starts_with("rgb(")
        || lower.starts_with("rgba(")
        || lower.starts_with("hsl(")
        || lower.starts_with("hsla(")
        || is_css_named_color(&lower)
}

fn is_css_named_color(name: &str) -> bool {
    matches!(
        name,
        "aliceblue"
            | "antiquewhite"
            | "aqua"
            | "aquamarine"
            | "azure"
            | "beige"
            | "bisque"
            | "black"
            | "blanchedalmond"
            | "blue"
            | "blueviolet"
            | "brown"
            | "burlywood"
            | "cadetblue"
            | "chartreuse"
            | "chocolate"
            | "coral"
            | "cornflowerblue"
            | "cornsilk"
            | "crimson"
            | "cyan"
            | "darkblue"
            | "darkcyan"
            | "darkgoldenrod"
            | "darkgray"
            | "darkgreen"
            | "darkgrey"
            | "darkkhaki"
            | "darkmagenta"
            | "darkolivegreen"
            | "darkorange"
            | "darkorchid"
            | "darkred"
            | "darksalmon"
            | "darkseagreen"
            | "darkslateblue"
            | "darkslategray"
            | "darkslategrey"
            | "darkturquoise"
            | "darkviolet"
            | "deeppink"
            | "deepskyblue"
            | "dimgray"
            | "dimgrey"
            | "dodgerblue"
            | "firebrick"
            | "floralwhite"
            | "forestgreen"
            | "fuchsia"
            | "gainsboro"
            | "ghostwhite"
            | "gold"
            | "goldenrod"
            | "gray"
            | "green"
            | "greenyellow"
            | "grey"
            | "honeydew"
            | "hotpink"
            | "indianred"
            | "indigo"
            | "ivory"
            | "khaki"
            | "lavender"
            | "lavenderblush"
            | "lawngreen"
            | "lemonchiffon"
            | "lightblue"
            | "lightcoral"
            | "lightcyan"
            | "lightgoldenrodyellow"
            | "lightgray"
            | "lightgreen"
            | "lightgrey"
            | "lightpink"
            | "lightsalmon"
            | "lightseagreen"
            | "lightskyblue"
            | "lightslategray"
            | "lightslategrey"
            | "lightsteelblue"
            | "lightyellow"
            | "lime"
            | "limegreen"
            | "linen"
            | "magenta"
            | "maroon"
            | "mediumaquamarine"
            | "mediumblue"
            | "mediumorchid"
            | "mediumpurple"
            | "mediumseagreen"
            | "mediumslateblue"
            | "mediumspringgreen"
            | "mediumturquoise"
            | "mediumvioletred"
            | "midnightblue"
            | "mintcream"
            | "mistyrose"
            | "moccasin"
            | "navajowhite"
            | "navy"
            | "oldlace"
            | "olive"
            | "olivedrab"
            | "orange"
            | "orangered"
            | "orchid"
            | "palegoldenrod"
            | "palegreen"
            | "paleturquoise"
            | "palevioletred"
            | "papayawhip"
            | "peachpuff"
            | "peru"
            | "pink"
            | "plum"
            | "powderblue"
            | "purple"
            | "rebeccapurple"
            | "red"
            | "rosybrown"
            | "royalblue"
            | "saddlebrown"
            | "salmon"
            | "sandybrown"
            | "seagreen"
            | "seashell"
            | "sienna"
            | "silver"
            | "skyblue"
            | "slateblue"
            | "slategray"
            | "slategrey"
            | "snow"
            | "springgreen"
            | "steelblue"
            | "tan"
            | "teal"
            | "thistle"
            | "tomato"
            | "turquoise"
            | "violet"
            | "wheat"
            | "white"
            | "whitesmoke"
            | "yellow"
            | "yellowgreen"
    )
}

fn parse_sequence_box_line(line: &str) -> Option<(Option<String>, Option<String>)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("box") {
        return None;
    }
    let rest = trimmed[3..].trim();
    if rest.is_empty() {
        return Some((None, None));
    }
    let tokens = tokenize_quoted(rest);
    if tokens.is_empty() {
        return Some((None, None));
    }
    let first = tokens[0].clone();
    if first.eq_ignore_ascii_case("transparent") {
        let label = tokens[1..].join(" ");
        let label = if label.trim().is_empty() {
            None
        } else {
            Some(label)
        };
        return Some((None, label));
    }
    // The first token is treated as a color only when it actually is one
    // (CSS named color, hex, rgb()/rgba()/hsl()/hsla(), or `transparent`).
    // Otherwise the entire token list is the label.
    let first_is_color = is_color_token(&first);
    let color = if first_is_color {
        Some(first.clone())
    } else {
        None
    };
    let label = if first_is_color && tokens.len() > 1 {
        Some(tokens[1..].join(" "))
    } else {
        Some(tokens.join(" "))
    };
    let label = label.filter(|value| !value.trim().is_empty());
    let color = color.filter(|value| !value.eq_ignore_ascii_case("transparent"));
    Some((color, label))
}

fn ensure_sequence_node(
    graph: &mut Graph,
    labels: &HashMap<String, String>,
    id: &str,
    shape: Option<crate::ir::NodeShape>,
) {
    let label = labels.get(id).cloned();
    if let Some(shape) = shape {
        graph.ensure_node(id, label, Some(shape));
        return;
    }
    if graph.nodes.contains_key(id) {
        graph.ensure_node(id, label, None);
    } else {
        graph.ensure_node(id, label, Some(crate::ir::NodeShape::ActorBox));
    }
}

fn parse_sequence_message(
    line: &str,
) -> Option<(
    String,
    String,
    Option<String>,
    crate::ir::EdgeStyle,
    Option<crate::ir::SequenceActivationKind>,
    crate::ir::SequenceArrowHead,
    Option<crate::ir::SequenceArrowHead>,
    Option<crate::ir::EdgeDecoration>, // start endpoint decoration (e.g., () = Circle)
    Option<crate::ir::EdgeDecoration>, // end endpoint decoration
)> {
    let tokens = [
        // Bidirectional arrows (longest first)
        "<<-->>", "<<->>", // Cross/open with activation
        "--x+", "-x+", "--)+", "-)+", "--x-", "-x-", "--)-", "-)-",
        // Existing activation variants
        "-->>+", "->>+", "-->+", "->+", "-->>-", "->>-", "-->-", "->-",
        // Reverse cross/open
        "<--x", "<-x", "<--)", "<-)", // Existing reverse
        "<--+", "<-+", "<--", "<-", // Cross and open arrows
        "--x", "-x", "--)", "-)", // Existing arrows
        "-->>", "->>", "-->", "->",
    ];
    for token in tokens {
        if let Some(pos) = line.find(token) {
            let left = line[..pos].trim();
            let right_part = line[pos + token.len()..].trim();
            if left.is_empty() || right_part.is_empty() {
                continue;
            }
            let (right, label) = split_label(right_part);
            // Detect and strip `()` central-connection markers from from/to
            // identifiers. JS treats `Alice->>()John`, `Alice()->>John`,
            // `John()->>()Alice` as central-connection arrows that draw a
            // circle marker at the lifeline center on the marked end. We
            // record which sides are marked so the renderer emits the
            // circle decoration; we also strip the marker so we don't create
            // phantom actors named `()John`, `Alice()`, etc.
            fn strip_cc(s: &str) -> (String, bool) {
                let mut marked = false;
                let mut t = s;
                if let Some(stripped) = t.strip_suffix("()") {
                    marked = true;
                    t = stripped;
                }
                if let Some(stripped) = t.strip_prefix("()") {
                    marked = true;
                    t = stripped;
                }
                (t.trim().to_string(), marked)
            }
            let (mut from, from_cc_marked) = strip_cc(left);
            let (mut to, to_cc_marked) = strip_cc(right.as_str());
            let is_bidirectional = token.starts_with("<<");
            if token.starts_with('<') && !is_bidirectional {
                std::mem::swap(&mut from, &mut to);
            }
            let trimmed = token.trim_start_matches('<').trim_end_matches(['+', '-']);
            let style = if trimmed.starts_with("--") {
                crate::ir::EdgeStyle::Dotted
            } else {
                crate::ir::EdgeStyle::Solid
            };
            let activation = if token.ends_with('+') {
                Some(crate::ir::SequenceActivationKind::Activate)
            } else if token.ends_with('-') {
                Some(crate::ir::SequenceActivationKind::Deactivate)
            } else {
                None
            };
            let arrow_head = if trimmed.ends_with('x') {
                crate::ir::SequenceArrowHead::Cross
            } else if trimmed.ends_with(')') {
                crate::ir::SequenceArrowHead::Open
            } else if trimmed.contains(">>") {
                crate::ir::SequenceArrowHead::Filled
            } else {
                crate::ir::SequenceArrowHead::None
            };
            let start_arrow = if is_bidirectional {
                Some(crate::ir::SequenceArrowHead::Filled)
            } else {
                None
            };
            // Map cc-marked flags to circle decorations. The arrow direction
            // is from→to AFTER the swap above, but the cc flags were captured
            // BEFORE the swap (still tied to original left/right). Remap them
            // to start/end matching the post-swap direction.
            let (start_marked, end_marked) = if token.starts_with('<') && !is_bidirectional {
                // Arrow points from `right` (now from) to `left` (now to).
                (to_cc_marked, from_cc_marked)
            } else {
                (from_cc_marked, to_cc_marked)
            };
            let start_decoration = if start_marked {
                Some(crate::ir::EdgeDecoration::Circle)
            } else {
                None
            };
            let end_decoration = if end_marked {
                Some(crate::ir::EdgeDecoration::Circle)
            } else {
                None
            };
            return Some((
                from,
                to,
                label,
                style,
                activation,
                arrow_head,
                start_arrow,
                start_decoration,
                end_decoration,
            ));
        }
    }
    None
}

fn parse_sequence_note(
    line: &str,
) -> Option<(crate::ir::SequenceNotePosition, Vec<String>, String)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("note ") {
        return None;
    }
    let rest = trimmed[4..].trim();
    let lower_rest = rest.to_ascii_lowercase();
    let (position, targets_part) = if lower_rest.starts_with("left of ") {
        (crate::ir::SequenceNotePosition::LeftOf, rest[8..].trim())
    } else if lower_rest.starts_with("right of ") {
        (crate::ir::SequenceNotePosition::RightOf, rest[9..].trim())
    } else if lower_rest.starts_with("over ") {
        (crate::ir::SequenceNotePosition::Over, rest[5..].trim())
    } else {
        return None;
    };

    let (targets, label) = targets_part.split_once(':')?;
    let label = label.trim();
    if label.is_empty() {
        return None;
    }
    let participants = targets
        .split(',')
        .map(|part| strip_quotes(part.trim()))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if participants.is_empty() {
        return None;
    }

    Some((position, participants, label.to_string()))
}

fn split_label(input: &str) -> (String, Option<String>) {
    // Find the first single ':' that is NOT part of a ':::' inline-class
    // marker (e.g. `s1 :::someclass` should NOT split into target "s1" and
    // label "::someclass" — the triple-colon belongs entirely to the class
    // suffix). A single ':' is a label separator only if neither neighbor
    // is also a ':'.
    let bytes = input.as_bytes();
    let mut split_at = None;
    for (i, &b) in bytes.iter().enumerate() {
        if b != b':' {
            continue;
        }
        let prev_colon = i > 0 && bytes[i - 1] == b':';
        let next_colon = i + 1 < bytes.len() && bytes[i + 1] == b':';
        if !prev_colon && !next_colon {
            split_at = Some(i);
            break;
        }
    }
    if let Some(i) = split_at {
        let target = input[..i].trim();
        let label = input[i + 1..].trim();
        if !label.is_empty() {
            return (target.to_string(), Some(label.to_string()));
        }
        return (target.to_string(), None);
    }
    (input.trim().to_string(), None)
}

fn parse_class_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Class;
    graph.direction = Direction::TopDown;
    graph.diagram_title = extract_yaml_frontmatter_title(input);
    let (lines, init_config) = preprocess_input(input)?;

    let mut members: HashMap<String, Vec<String>> = HashMap::new();
    let mut labels: HashMap<String, String> = HashMap::new();
    let mut current_class: Option<String> = None;
    let mut namespace_stack: Vec<usize> = Vec::new();
    let mut note_index = 0usize;
    let mut class_interface_index = 0usize;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();

        // Namespace support (v10.5+): `namespace Foo {`
        if lower.starts_with("namespace ") {
            let rest = line[10..].trim().trim_end_matches('{').trim();
            if !rest.is_empty() {
                graph.subgraphs.push(Subgraph {
                    id: Some(rest.to_string()),
                    label: rest.to_string(),
                    nodes: Vec::new(),
                    direction: None,
                    icon: None,
                    markdown_label: false,
                });
                namespace_stack.push(graph.subgraphs.len() - 1);
            }
            continue;
        }

        // End of namespace or class body
        if line == "}" && current_class.is_none() {
            namespace_stack.pop();
            continue;
        }

        if lower.starts_with("classdiagram") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1
                && let Some(dir) = Direction::from_token(parts[1])
            {
                graph.direction = dir;
            }
            continue;
        }

        if let Some(direction) = parse_direction_line(line) {
            graph.direction = direction;
            continue;
        }

        if let Some(active) = current_class.clone() {
            if let Some(end_idx) = line.find('}') {
                let fragment = line[..end_idx].trim();
                if !fragment.is_empty() {
                    members
                        .entry(active.clone())
                        .or_default()
                        .push(fragment.to_string());
                }
                current_class = None;
            } else {
                members
                    .entry(active.clone())
                    .or_default()
                    .push(line.to_string());
            }
            continue;
        }

        if let Some((target, note_text, markdown_label)) = parse_class_note_line(line) {
            let note_id = format!("note{note_index}");
            graph.ensure_node_md(
                &note_id,
                Some(note_text),
                Some(crate::ir::NodeShape::Note),
                markdown_label,
            );
            if let Some(&ns_idx) = namespace_stack.last() {
                let sg = &mut graph.subgraphs[ns_idx];
                if !sg.nodes.contains(&note_id) {
                    sg.nodes.push(note_id.clone());
                }
            }

            if let Some(target) = target {
                let (target_id, target_label) = normalize_class_id(&target);
                if let Some(label) = target_label {
                    labels.insert(target_id.clone(), label);
                }
                graph.ensure_node(
                    &target_id,
                    labels.get(&target_id).cloned(),
                    Some(crate::ir::NodeShape::Rectangle),
                );
                if let Some(&ns_idx) = namespace_stack.last() {
                    let sg = &mut graph.subgraphs[ns_idx];
                    if !sg.nodes.contains(&target_id) {
                        sg.nodes.push(target_id.clone());
                    }
                }
                graph.edges.push(crate::ir::Edge {
                    from: note_id.clone(),
                    to: target_id,
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
                    id: Some(format!("edgeNote{note_index}")),
                    curve: None,
                    arch_port_from: None,
                    arch_port_to: None,
                });
            }

            note_index += 1;
            continue;
        }

        if line.starts_with("style ") {
            parse_style_line(line, &mut graph);
            continue;
        }

        if let Some((target, annotation)) = parse_class_annotation_line(line) {
            let (target_id, target_label) = normalize_class_id(&target);
            if let Some(label) = target_label {
                labels.insert(target_id.clone(), label);
            }
            graph.ensure_node(
                &target_id,
                labels.get(&target_id).cloned(),
                Some(crate::ir::NodeShape::Rectangle),
            );
            members
                .entry(target_id)
                .or_default()
                .push(class_annotation_member(&annotation));
            continue;
        }

        if let Some(lollipop) = parse_class_lollipop_relation_line(line) {
            let (class_id, class_label) = normalize_class_id(&lollipop.class_token);
            if let Some(label) = class_label {
                labels.insert(class_id.clone(), label);
            }
            graph.ensure_node(
                &class_id,
                labels.get(&class_id).cloned(),
                Some(crate::ir::NodeShape::Rectangle),
            );
            if let Some(&ns_idx) = namespace_stack.last() {
                let sg = &mut graph.subgraphs[ns_idx];
                if !sg.nodes.contains(&class_id) {
                    sg.nodes.push(class_id.clone());
                }
            }

            let interface_id = format!("interface{class_interface_index}");
            class_interface_index += 1;
            graph.ensure_node(
                &interface_id,
                Some(lollipop.interface_label),
                Some(crate::ir::NodeShape::Text),
            );
            if let Some(&ns_idx) = namespace_stack.last() {
                let sg = &mut graph.subgraphs[ns_idx];
                if !sg.nodes.contains(&interface_id) {
                    sg.nodes.push(interface_id.clone());
                }
            }

            let (from, to) = match lollipop.side {
                ClassLollipopSide::Start => (interface_id, class_id),
                ClassLollipopSide::End => (class_id, interface_id),
            };
            graph.edges.push(crate::ir::Edge {
                from,
                to,
                label: lollipop.label,
                start_label: None,
                end_label: None,
                directed: lollipop.meta.directed,
                arrow_start: lollipop.meta.arrow_start,
                arrow_end: lollipop.meta.arrow_end,
                arrow_start_kind: lollipop.meta.arrow_start_kind,
                arrow_end_kind: lollipop.meta.arrow_end_kind,
                start_decoration: lollipop.meta.start_decoration,
                end_decoration: lollipop.meta.end_decoration,
                sequence_arrow_end: None,
                sequence_arrow_start: None,
                style: lollipop.meta.style,
                markdown_label: false,
                id: None,
                curve: None,
                arch_port_from: None,
                arch_port_to: None,
            });
            continue;
        }

        if let Some((left, right, meta, label, start_label, end_label)) =
            parse_class_relation_line(line)
        {
            let (left_id, left_label) = normalize_class_id(&left);
            let (right_id, right_label) = normalize_class_id(&right);
            if let Some(label) = left_label {
                labels.insert(left_id.clone(), label);
            }
            if let Some(label) = right_label {
                labels.insert(right_id.clone(), label);
            }
            graph.ensure_node(
                &left_id,
                labels.get(&left_id).cloned(),
                Some(crate::ir::NodeShape::Rectangle),
            );
            if let Some(&ns_idx) = namespace_stack.last() {
                let sg = &mut graph.subgraphs[ns_idx];
                if !sg.nodes.contains(&left_id) {
                    sg.nodes.push(left_id.clone());
                }
            }
            graph.ensure_node(
                &right_id,
                labels.get(&right_id).cloned(),
                Some(crate::ir::NodeShape::Rectangle),
            );
            if let Some(&ns_idx) = namespace_stack.last() {
                let sg = &mut graph.subgraphs[ns_idx];
                if !sg.nodes.contains(&right_id) {
                    sg.nodes.push(right_id.clone());
                }
            }
            graph.edges.push(crate::ir::Edge {
                from: left_id,
                to: right_id,
                label,
                start_label,
                end_label,
                directed: meta.directed,
                arrow_start: meta.arrow_start,
                arrow_end: meta.arrow_end,
                arrow_start_kind: meta.arrow_start_kind,
                arrow_end_kind: meta.arrow_end_kind,
                start_decoration: meta.start_decoration,
                end_decoration: meta.end_decoration,
                sequence_arrow_end: None,
                sequence_arrow_start: None,
                style: meta.style,
                markdown_label: false,
                id: None,
                curve: None,
                arch_port_from: None,
                arch_port_to: None,
            });
            continue;
        }

        if line.starts_with("class ") {
            let rest = line.trim_start_matches("class ").trim();
            if let Some((id, label, body, open_body, classes, annotations)) =
                parse_class_declaration(rest)
            {
                if let Some(label) = label.clone() {
                    labels.insert(id.clone(), label);
                }
                graph.ensure_node(
                    &id,
                    labels.get(&id).cloned(),
                    Some(crate::ir::NodeShape::Rectangle),
                );
                apply_node_classes(&mut graph, &id, &classes);
                if let Some(&ns_idx) = namespace_stack.last() {
                    let sg = &mut graph.subgraphs[ns_idx];
                    if !sg.nodes.contains(&id) {
                        sg.nodes.push(id.clone());
                    }
                }
                if !annotations.is_empty() {
                    let class_members = members.entry(id.clone()).or_default();
                    for annotation in annotations {
                        class_members.push(class_annotation_member(&annotation));
                    }
                }
                if let Some(body) = body {
                    for entry in split_class_body(&body) {
                        if !entry.is_empty() {
                            members.entry(id.clone()).or_default().push(entry);
                        }
                    }
                }
                if open_body {
                    current_class = Some(id.clone());
                }
                continue;
            }
        }

        if let Some((id, member)) = parse_class_member_line(line) {
            members.entry(id).or_default().push(member);
            continue;
        }
    }

    for (id, node) in graph.nodes.iter_mut() {
        if matches!(
            node.shape,
            crate::ir::NodeShape::Note | crate::ir::NodeShape::Text
        ) {
            continue;
        }
        let class_name = labels
            .get(id)
            .cloned()
            .unwrap_or_else(|| node.label.clone());
        // Convert generics: `List~int~` → `List<int>`
        let class_name = convert_tilde_generics(&class_name);
        let mut lines = Vec::new();
        // Extract annotations (<<Interface>>, <<Abstract>>, etc.) from members.
        let mut annotation: Option<String> = None;
        if let Some(items) = members.get(id) {
            for entry in items {
                if let Some(parsed_annotation) = parse_class_annotation_member(entry) {
                    annotation = Some(parsed_annotation);
                    break;
                }
            }
        }
        if let Some(ref ann) = annotation {
            lines.push(class_annotation_label(ann));
        }
        lines.push(class_name.clone());
        let mut attrs = Vec::new();
        let mut methods = Vec::new();
        if let Some(items) = members.get(id) {
            for entry in items {
                // Skip annotations (already handled above).
                if parse_class_annotation_member(entry).is_some() {
                    continue;
                }
                let trimmed = entry.trim();
                let converted = convert_tilde_generics(trimmed);
                if converted.contains('(') && converted.contains(')') {
                    methods.push(normalize_class_method_signature(&converted));
                } else {
                    attrs.push(converted);
                }
            }
        }
        lines.push("---".to_string());
        if !attrs.is_empty() {
            lines.extend(attrs);
        }
        lines.push("---".to_string());
        if !methods.is_empty() {
            lines.extend(methods);
        }
        node.label = lines.join("\n");
    }

    Ok(ParseOutput { graph, init_config })
}

/// Convert Mermaid generic syntax `List~int~` to `List<int>`.
fn convert_tilde_generics(input: &str) -> String {
    let tilde_count = input.chars().filter(|ch| *ch == '~').count();
    if tilde_count <= 1 {
        return input.to_string();
    }

    let mut chars = input.chars().collect::<Vec<_>>();
    let mut restore_package_visibility = false;
    if tilde_count % 2 != 0 && chars.first() == Some(&'~') {
        chars.remove(0);
        restore_package_visibility = true;
    }

    loop {
        let Some(first) = chars.iter().position(|ch| *ch == '~') else {
            break;
        };
        let Some(last) = chars.iter().rposition(|ch| *ch == '~') else {
            break;
        };
        if first == last {
            break;
        }
        chars[first] = '<';
        chars[last] = '>';
    }

    if restore_package_visibility {
        chars.insert(0, '~');
    }

    chars.into_iter().collect()
}

fn is_er_card_char(ch: char) -> bool {
    matches!(ch, '|' | 'o' | '{' | '}')
}

fn er_cardinality_token_at_end(input: &str) -> Option<(&str, &str)> {
    const TOKENS: &[&str] = &[
        "zero or more",
        "zero or many",
        "one or more",
        "one or many",
        "zero or one",
        "one or zero",
        "only one",
        "many(0)",
        "many(1)",
        "many",
        "0+",
        "1+",
        "1",
    ];
    let trimmed = input.trim_end();
    let lower = trimmed.to_ascii_lowercase();
    for token in TOKENS {
        if lower == *token {
            return Some(("", &trimmed[trimmed.len() - token.len()..]));
        }
        if lower.ends_with(token) {
            let start = trimmed.len().saturating_sub(token.len());
            let before = trimmed[..start].trim_end();
            if trimmed[..start]
                .chars()
                .last()
                .is_some_and(|ch| ch.is_whitespace())
            {
                return Some((before, &trimmed[start..]));
            }
        }
    }
    None
}

fn er_cardinality_token_at_start(input: &str) -> Option<(&str, &str)> {
    const TOKENS: &[&str] = &[
        "zero or more",
        "zero or many",
        "one or more",
        "one or many",
        "zero or one",
        "one or zero",
        "only one",
        "many(0)",
        "many(1)",
        "many",
        "0+",
        "1+",
        "1",
    ];
    let trimmed = input.trim_start();
    let lower = trimmed.to_ascii_lowercase();
    for token in TOKENS {
        if lower == *token {
            return Some((&trimmed[..token.len()], ""));
        }
        if lower.starts_with(token) {
            let end = token.len();
            let after = trimmed[end..].trim_start();
            if trimmed[end..]
                .chars()
                .next()
                .is_some_and(|ch| ch.is_whitespace())
            {
                return Some((&trimmed[..end], after));
            }
        }
    }
    None
}

fn split_er_cardinality_left(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }
    if let Some((entity, token)) = er_cardinality_token_at_end(trimmed) {
        return (entity.trim().to_string(), Some(token.trim().to_string()));
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let len = chars.len();
    if len >= 2 {
        let last_two = &chars[len - 2..];
        if last_two.iter().all(|ch| is_er_card_char(*ch)) {
            let entity = chars[..len - 2].iter().collect::<String>();
            let token = last_two.iter().collect::<String>();
            return (entity.trim().to_string(), Some(token));
        }
    }
    if let Some(&last) = chars.last()
        && is_er_card_char(last)
    {
        let entity = chars[..len - 1].iter().collect::<String>();
        return (entity.trim().to_string(), Some(last.to_string()));
    }
    (trimmed.to_string(), None)
}

fn split_er_cardinality_right(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }
    if let Some((token, entity)) = er_cardinality_token_at_start(trimmed) {
        return (entity.trim().to_string(), Some(token.trim().to_string()));
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let len = chars.len();
    if len >= 2 {
        let first_two = &chars[..2];
        if first_two.iter().all(|ch| is_er_card_char(*ch)) {
            let entity = chars[2..].iter().collect::<String>();
            let token = first_two.iter().collect::<String>();
            return (entity.trim().to_string(), Some(token));
        }
    }
    if is_er_card_char(chars[0]) {
        let entity = chars[1..].iter().collect::<String>();
        return (entity.trim().to_string(), Some(chars[0].to_string()));
    }
    (trimmed.to_string(), None)
}

fn normalize_er_cardinality(token: &str) -> (String, Option<crate::ir::EdgeDecoration>) {
    let trimmed = token.trim().to_ascii_lowercase();
    match trimmed.as_str() {
        "||" | "|" | "one" | "only one" | "1" => (
            "1".to_string(),
            Some(crate::ir::EdgeDecoration::CrowsFootOne),
        ),
        "o|" | "|o" | "o" | "zero or one" | "one or zero" => (
            "0..1".to_string(),
            Some(crate::ir::EdgeDecoration::CrowsFootZeroOne),
        ),
        "|{" | "}|" | "one or more" | "one or many" | "many(1)" | "1+" => (
            "1..*".to_string(),
            Some(crate::ir::EdgeDecoration::CrowsFootMany),
        ),
        "o{" | "}o" | "}" | "{" | "zero or more" | "zero or many" | "many(0)" | "many" | "0+" => (
            "0..*".to_string(),
            Some(crate::ir::EdgeDecoration::CrowsFootZeroMany),
        ),
        _ => (token.trim().to_string(), None),
    }
}

fn split_er_inline_classes(classes: Vec<String>) -> Vec<String> {
    classes
        .into_iter()
        .flat_map(|class| {
            class
                .split(',')
                .map(|name| name.trim().to_string())
                .collect::<Vec<_>>()
        })
        .filter(|name| !name.is_empty())
        .collect()
}

fn parse_er_entity_ref(token: &str) -> (String, Option<String>, Vec<String>) {
    let (base, classes) = split_inline_classes(token);
    let classes = split_er_inline_classes(classes);
    let base = base.trim();
    if let Some(open_idx) = base.find('[')
        && base.ends_with(']')
    {
        let id = strip_quotes(base[..open_idx].trim());
        let alias = strip_quotes(base[open_idx + 1..base.len() - 1].trim());
        if !id.is_empty() {
            let label = if alias.is_empty() { None } else { Some(alias) };
            return (id, label, classes);
        }
    }
    (strip_quotes(base), None, classes)
}

fn ensure_er_node(graph: &mut Graph, id: &str, label: Option<String>, classes: &[String]) {
    graph.ensure_node_md(id, label, Some(crate::ir::NodeShape::RoundRect), true);
    apply_node_classes(graph, id, classes);
}

fn apply_er_default_classes(graph: &mut Graph) {
    let ids: Vec<String> = graph.nodes.keys().cloned().collect();
    for id in ids {
        let classes = graph.node_classes.entry(id).or_default();
        if !classes.iter().any(|class| class == "default") {
            classes.insert(0, "default".to_string());
        }
    }
}

fn find_er_relation_separator(relation_part: &str) -> Option<(usize, usize, crate::ir::EdgeStyle)> {
    if let Some(idx) = relation_part.find("--") {
        return Some((idx, 2, crate::ir::EdgeStyle::Solid));
    }
    if let Some(idx) = relation_part.find("..") {
        return Some((idx, 2, crate::ir::EdgeStyle::Dotted));
    }
    if let Some(idx) = relation_part.find(".-") {
        return Some((idx, 2, crate::ir::EdgeStyle::Dotted));
    }
    if let Some(idx) = relation_part.find("-.") {
        return Some((idx, 2, crate::ir::EdgeStyle::Dotted));
    }
    let lower = relation_part.to_ascii_lowercase();
    if let Some(idx) = lower.find(" optionally to ") {
        return Some((idx, " optionally to ".len(), crate::ir::EdgeStyle::Dotted));
    }
    if let Some(idx) = lower.find(" to ") {
        return Some((idx, " to ".len(), crate::ir::EdgeStyle::Solid));
    }
    None
}

fn parse_er_relation_line(
    line: &str,
) -> Option<(
    (String, Option<String>, Vec<String>),
    (String, Option<String>, Vec<String>),
    Option<String>,
    Option<String>,
    Option<String>,
    Option<crate::ir::EdgeDecoration>,
    Option<crate::ir::EdgeDecoration>,
    crate::ir::EdgeStyle,
)> {
    let (relation_part, label) = if let Some((before, after)) = line.rsplit_once(':') {
        let label = after.trim();
        let label = if label.is_empty() {
            None
        } else {
            Some(label.to_string())
        };
        (before.trim(), label)
    } else {
        (line.trim(), None)
    };

    let (sep, sep_len, style) = find_er_relation_separator(relation_part)?;
    let left_part = relation_part[..sep].trim();
    let right_part = relation_part[sep + sep_len..].trim();
    if left_part.is_empty() || right_part.is_empty() {
        return None;
    }
    let (left_entity, left_card) = split_er_cardinality_left(left_part);
    let (right_entity, right_card) = split_er_cardinality_right(right_part);
    if left_entity.is_empty() || right_entity.is_empty() {
        return None;
    }
    let left_ref = parse_er_entity_ref(left_entity.trim());
    let right_ref = parse_er_entity_ref(right_entity.trim());
    let left_id = left_ref.0.as_str();
    let right_id = right_ref.0.as_str();
    if left_id.is_empty() || right_id.is_empty() {
        return None;
    }
    let (left_label, left_decoration) = left_card
        .map(|token| normalize_er_cardinality(&token))
        .map(|(label, dec)| (Some(label), dec))
        .unwrap_or((None, None));
    let (right_label, right_decoration) = right_card
        .map(|token| normalize_er_cardinality(&token))
        .map(|(label, dec)| (Some(label), dec))
        .unwrap_or((None, None));
    Some((
        left_ref,
        right_ref,
        label,
        left_label,
        right_label,
        left_decoration,
        right_decoration,
        style,
    ))
}

fn parse_er_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Er;
    graph.direction = Direction::TopDown;
    graph.diagram_title = extract_yaml_frontmatter_title(input);
    let (lines, init_config) = preprocess_input(input)?;

    let mut members: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_entity: Option<String> = None;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("erdiagram") {
            continue;
        }
        if let Some(direction) = parse_direction_line(line) {
            graph.direction = direction;
            continue;
        }
        if lower.starts_with("title ") {
            let title = line.get(5..).unwrap_or("").trim();
            if !title.is_empty() {
                graph.diagram_title = Some(strip_quotes(title));
            }
            continue;
        }
        if line.starts_with("classDef") {
            parse_class_def(line, &mut graph);
            continue;
        }
        if line.starts_with("class ") {
            parse_class_line(line, &mut graph);
            continue;
        }
        if line.starts_with("style ") {
            parse_style_line(line, &mut graph);
            continue;
        }

        if let Some(active) = current_entity.clone() {
            if let Some(end_idx) = line.find('}') {
                let fragment = line[..end_idx].trim();
                if !fragment.is_empty() {
                    members
                        .entry(active.clone())
                        .or_default()
                        .push(fragment.to_string());
                }
                current_entity = None;
            } else {
                members
                    .entry(active.clone())
                    .or_default()
                    .push(line.to_string());
            }
            continue;
        }

        if let Some((
            (left, left_label, left_classes),
            (right, right_label, right_classes),
            label,
            _left_label,
            _right_label,
            left_decoration,
            right_decoration,
            style,
        )) = parse_er_relation_line(line)
        {
            ensure_er_node(&mut graph, &left, left_label, &left_classes);
            ensure_er_node(&mut graph, &right, right_label, &right_classes);
            // Don't use start_label/end_label for ER diagrams - crow's foot symbols convey cardinality
            graph.edges.push(crate::ir::Edge {
                from: left,
                to: right,
                label,
                start_label: None,
                end_label: None,
                directed: false,
                arrow_start: false,
                arrow_end: false,
                arrow_start_kind: None,
                arrow_end_kind: None,
                start_decoration: left_decoration,
                end_decoration: right_decoration,
                sequence_arrow_end: None,
                sequence_arrow_start: None,
                style,
                markdown_label: false,
                id: None,
                curve: None,
                arch_port_from: None,
                arch_port_to: None,
            });
            continue;
        }

        if let Some(open_idx) = line.find('{') {
            let (name, label, classes) = parse_er_entity_ref(line[..open_idx].trim());
            if !name.is_empty() {
                ensure_er_node(&mut graph, &name, label, &classes);
                current_entity = Some(name.clone());
                let tail = line[open_idx + 1..].trim();
                if let Some(close_idx) = tail.find('}') {
                    let fragment = tail[..close_idx].trim();
                    if !fragment.is_empty() {
                        members.entry(name).or_default().push(fragment.to_string());
                    }
                    current_entity = None;
                } else if !tail.is_empty() {
                    members.entry(name).or_default().push(tail.to_string());
                }
            }
            continue;
        }

        let (entity, label, classes) = parse_er_entity_ref(line);
        if !entity.is_empty() {
            ensure_er_node(&mut graph, &entity, label, &classes);
        }
    }

    apply_er_default_classes(&mut graph);

    for (id, node) in graph.nodes.iter_mut() {
        let mut lines = Vec::new();
        lines.push(node.label.clone());
        if let Some(attrs) = members.get(id)
            && !attrs.is_empty()
        {
            lines.push("---".to_string());
            lines.extend(attrs.clone());
        }
        node.label = lines.join("\n");
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_pie_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Pie;
    let (lines, init_config) = preprocess_input(input)?;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("pie") {
            if lower.contains("showdata") {
                graph.pie_show_data = true;
            }
            // Check for title on the same line: "pie title My Title"
            if let Some(title_pos) = lower.find("title") {
                let title_start = title_pos + 5; // len("title")
                if let Some(title) = line.get(title_start..) {
                    let title = title.trim();
                    if !title.is_empty() {
                        graph.pie_title = Some(title.to_string());
                    }
                }
            }
            continue;
        }
        if lower.starts_with("showdata") {
            graph.pie_show_data = true;
            continue;
        }
        if lower.starts_with("title") {
            let title = line.get(5..).unwrap_or("").trim();
            if !title.is_empty() {
                graph.pie_title = Some(title.to_string());
            }
            continue;
        }
        if let Some((label, value)) = parse_pie_slice_line(line) {
            graph.pie_slices.push(crate::ir::PieSlice { label, value });
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_pie_slice_line(line: &str) -> Option<(String, f32)> {
    let (label_part, value_part) = line.split_once(':')?;
    let label = strip_quotes(label_part.trim());
    if label.is_empty() {
        return None;
    }
    let value_str = value_part.trim();
    if value_str.is_empty() {
        return None;
    }
    let value = value_str.parse::<f32>().ok()?;
    Some((label, value))
}

fn parse_venn_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Venn;
    let (lines, init_config) = preprocess_input(input)?;

    let mut current_sets: Option<Vec<String>> = None;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();

        // Skip header line
        if lower.starts_with("venn") {
            continue;
        }

        // Title
        if lower.starts_with("title") {
            let title = line.get(5..).unwrap_or("").trim();
            if !title.is_empty() {
                graph.venn.title = Some(title.to_string());
            }
            continue;
        }

        // accTitle / accDescr
        if lower.starts_with("acctitle") {
            if let Some(rest) = line.get(8..) {
                let rest = rest.trim().trim_start_matches(':').trim();
                if !rest.is_empty() {
                    graph.acc_title = Some(rest.to_string());
                }
            }
            continue;
        }
        if lower.starts_with("accdescr") {
            if let Some(rest) = line.get(8..) {
                let rest = rest.trim().trim_start_matches(':').trim();
                if !rest.is_empty() {
                    graph.acc_descr = Some(rest.to_string());
                }
            }
            continue;
        }

        // Style line: style SetID fill:#fff, stroke:#333, ...
        if lower.starts_with("style ") {
            let rest = line.get(6..).unwrap_or("").trim();
            if let Some((target_ids, props_str)) = rest.split_once(' ') {
                let mut targets: Vec<String> = target_ids
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                targets.sort();
                let style = parse_venn_style(props_str);

                if targets.len() == 1 {
                    let target_id = &targets[0];
                    for set in &mut graph.venn.sets {
                        if &set.id == target_id {
                            set.style = Some(style.clone());
                        }
                    }
                    for node in &mut graph.venn.text_nodes {
                        if &node.id == target_id {
                            node.style = Some(style.clone());
                        }
                    }
                } else if targets.len() > 1 {
                    for union in &mut graph.venn.unions {
                        let mut union_ids = union.set_ids.clone();
                        union_ids.sort();
                        if union_ids == targets {
                            union.style = Some(style.clone());
                        }
                    }
                }
            }
            continue;
        }

        // Set line: set A["Label"] :100
        if lower.starts_with("set ") {
            let rest = line.get(4..).unwrap_or("").trim();
            let (id, label, size) = parse_venn_set_line(rest);
            current_sets = Some(vec![id.clone()]);
            graph.venn.sets.push(crate::ir::VennSet {
                id,
                label,
                size,
                style: None,
            });
            continue;
        }

        // Union line: union A, B
        if lower.starts_with("union ") {
            let rest = line.get(6..).unwrap_or("").trim();
            let (set_ids, label, size) = parse_venn_union_line(rest);
            current_sets = Some(set_ids.clone());
            graph.venn.unions.push(crate::ir::VennUnion {
                set_ids,
                size,
                label,
                style: None,
            });
            continue;
        }

        // Text line: Mermaid attaches indented text nodes to the most recent
        // set or union subset.
        if lower.starts_with("text ") {
            let rest = line.get(5..).unwrap_or("").trim();
            let (id, label) = parse_venn_text_line(rest);
            if !id.is_empty()
                && let Some(set_ids) = current_sets.clone()
            {
                graph.venn.text_nodes.push(crate::ir::VennTextNode {
                    set_ids,
                    id,
                    label,
                    style: None,
                });
            }
            continue;
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_venn_set_line(input: &str) -> (String, String, f32) {
    let (ids, label, size) = parse_venn_subset_line(input, 10.0);
    let id = ids.first().cloned().unwrap_or_default();
    let label = label.unwrap_or_else(|| id.clone());
    (id, label, size)
}

fn parse_venn_union_line(input: &str) -> (Vec<String>, Option<String>, f32) {
    let (ids, label, size) = parse_venn_subset_line(input, -1.0);
    let size = if size < 0.0 {
        let count = ids.len().max(1) as f32;
        10.0 / (count * count)
    } else {
        size
    };
    (ids, label, size)
}

fn parse_venn_subset_line(input: &str, default_size: f32) -> (Vec<String>, Option<String>, f32) {
    let mut rest = input.trim();
    let mut size = default_size;
    if let Some(colon_pos) = rest.rfind(':') {
        let after_colon = rest[colon_pos + 1..].trim();
        if let Ok(val) = after_colon.parse::<f32>() {
            size = val;
            rest = rest[..colon_pos].trim();
        }
    }

    let mut label = None;
    if let Some(bracket_start) = rest.find('[') {
        let ids_part = rest[..bracket_start].trim();
        let label_part = &rest[bracket_start + 1..];
        let parsed_label = if label_part.ends_with(']') {
            strip_quotes(label_part[..label_part.len() - 1].trim())
        } else {
            strip_quotes(label_part.trim())
        };
        if !parsed_label.is_empty() {
            label = Some(parsed_label);
        }
        rest = ids_part;
    }

    let ids: Vec<String> = rest
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    (ids, label, size)
}

fn parse_venn_text_line(input: &str) -> (String, Option<String>) {
    let s = input.trim();
    if let Some(bracket_start) = s.find('[') {
        let id = s[..bracket_start].trim().to_string();
        let label_part = &s[bracket_start + 1..];
        let label = if label_part.ends_with(']') {
            strip_quotes(label_part[..label_part.len() - 1].trim())
        } else {
            strip_quotes(label_part.trim())
        };
        (id, if label.is_empty() { None } else { Some(label) })
    } else {
        (strip_quotes(s), None)
    }
}

fn parse_venn_style(input: &str) -> crate::ir::VennStyle {
    let mut style = crate::ir::VennStyle::default();
    for part in input.split(',') {
        let part = part.trim();
        if let Some((key, value)) = part.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "fill" => style.fill = Some(value.to_string()),
                "stroke" => style.stroke = Some(value.to_string()),
                "stroke-width" => {
                    if let Ok(w) = value.trim_end_matches("px").parse::<f32>() {
                        style.stroke_width = Some(w);
                    }
                }
                "fill-opacity" => {
                    if let Ok(o) = value.parse::<f32>() {
                        style.fill_opacity = Some(o);
                    }
                }
                "color" => style.color = Some(value.to_string()),
                _ => {}
            }
        }
    }
    style
}

fn parse_mindmap_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Mindmap;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input_keep_indent(input)?;
    let mut stack: Vec<String> = Vec::new();
    let mut base_indent: Option<usize> = None;
    let mut node_index: HashMap<String, usize> = HashMap::new();

    let mut line_index = 0;
    while line_index < lines.len() {
        let mut raw_line = lines[line_index].clone();
        while mindmap_shape_fragment_unclosed(raw_line.trim()) && line_index + 1 < lines.len() {
            line_index += 1;
            raw_line.push('\n');
            raw_line.push_str(lines[line_index].trim());
        }
        line_index += 1;

        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("mindmap") {
            continue;
        }
        if let Some(icon) = parse_mindmap_icon_directive(trimmed) {
            if let Some(node) = graph.mindmap.nodes.last_mut() {
                node.icon = Some(icon);
            }
            continue;
        }

        let indent = count_indent(&raw_line);
        let base = *base_indent.get_or_insert(indent);
        let rel_indent = indent.saturating_sub(base);
        let mut level = rel_indent / 2;
        if level > stack.len() {
            level = stack.len();
        }

        let (raw_id, label, node_type, classes, md_label) = parse_mindmap_node_token(trimmed);
        let mut id = raw_id;
        if id.is_empty() {
            id = sanitize_id(&label);
        }
        if id.is_empty() {
            id = format!("mindmap_{}", graph.mindmap.nodes.len());
        }
        if graph.nodes.contains_key(&id) {
            id = format!("{}_{}", id, graph.nodes.len());
        }

        let shape = match node_type {
            crate::ir::MindmapNodeType::Circle => crate::ir::NodeShape::Circle,
            crate::ir::MindmapNodeType::RoundedRect => crate::ir::NodeShape::RoundRect,
            crate::ir::MindmapNodeType::Rect => crate::ir::NodeShape::Rectangle,
            crate::ir::MindmapNodeType::Hexagon => crate::ir::NodeShape::Hexagon,
            crate::ir::MindmapNodeType::Cloud => crate::ir::NodeShape::Cloud,
            crate::ir::MindmapNodeType::Bang => crate::ir::NodeShape::Bang,
            crate::ir::MindmapNodeType::Default => crate::ir::NodeShape::MindmapDefault,
        };

        graph.ensure_node_md(&id, Some(label.clone()), Some(shape), md_label);
        if !classes.is_empty() {
            apply_node_classes(&mut graph, &id, &classes);
        }

        if graph.mindmap.root_id.is_none() {
            graph.mindmap.root_id = Some(id.clone());
        }

        if level > 0 && stack.len() > level {
            stack.truncate(level);
        }

        let parent_id = if level > 0 {
            stack.last().cloned()
        } else {
            None
        };

        let section = if level == 0 {
            None
        } else if let Some(parent_id) = parent_id.as_ref() {
            let parent_idx = node_index.get(parent_id).copied();
            if let Some(parent_idx) = parent_idx {
                let parent = &graph.mindmap.nodes[parent_idx];
                if parent.level == 0 {
                    Some(parent.children.len())
                } else {
                    parent.section
                }
            } else {
                None
            }
        } else {
            None
        };

        let node = crate::ir::MindmapNode {
            id: id.clone(),
            label: label.clone(),
            level,
            section,
            node_type,
            icon: None,
            class: None,
            children: Vec::new(),
            markdown_label: md_label,
        };

        let idx = graph.mindmap.nodes.len();
        graph.mindmap.nodes.push(node);
        node_index.insert(id.clone(), idx);

        if let Some(parent_id) = parent_id {
            if let Some(parent_idx) = node_index.get(&parent_id).copied() {
                graph.mindmap.nodes[parent_idx].children.push(id.clone());
            }
            graph.edges.push(crate::ir::Edge {
                from: parent_id,
                to: id.clone(),
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
                style: crate::ir::EdgeStyle::Solid,
                markdown_label: false,
                id: None,
                curve: None,
                arch_port_from: None,
                arch_port_to: None,
            });
        } else {
            stack.clear();
        }

        stack.push(id);
    }

    Ok(ParseOutput { graph, init_config })
}

fn mindmap_shape_fragment_unclosed(trimmed: &str) -> bool {
    let Some(shape_start) = trimmed.find(['[', '(', '{', ')']) else {
        return false;
    };
    if shape_start > 0 && trimmed[..shape_start].contains(' ') {
        return false;
    }
    let raw = trimmed[shape_start..].trim();
    if raw.starts_with("[") {
        return !raw.ends_with("]");
    }
    if raw.starts_with("{{") {
        return !raw.ends_with("}}");
    }
    if raw.starts_with("((") {
        return !raw.ends_with("))");
    }
    if raw.starts_with("))") {
        return !raw.ends_with("((");
    }
    if raw.starts_with(")") {
        return !raw.ends_with("(");
    }
    if raw.starts_with("(") {
        return !raw.ends_with(")");
    }
    false
}

fn parse_mindmap_node_token(
    token: &str,
) -> (
    String,
    String,
    crate::ir::MindmapNodeType,
    Vec<String>,
    bool,
) {
    let (base, classes) = split_inline_classes(token);
    let trimmed = base.trim();
    if trimmed.is_empty() {
        return (
            String::new(),
            String::new(),
            crate::ir::MindmapNodeType::Default,
            classes,
            false,
        );
    }

    let mut id = String::new();
    let mut label = trimmed.to_string();
    let mut node_type = crate::ir::MindmapNodeType::Default;
    let mut md = false;

    let shape_start = trimmed.find(['[', '(', '{', ')']).unwrap_or(0);
    if shape_start > 0 && !trimmed[..shape_start].contains(' ') {
        id = trimmed[..shape_start].trim().to_string();
        let raw = trimmed[shape_start..].trim();
        if let Some((shape_label, shape_type, shape_md)) = parse_mindmap_shape(raw) {
            label = shape_label;
            node_type = shape_type;
            md = shape_md;
        }
    } else if let Some((shape_label, shape_type, shape_md)) = parse_mindmap_shape(trimmed) {
        label = shape_label;
        node_type = shape_type;
        md = shape_md;
    }

    if id.is_empty() {
        id = sanitize_id(&label);
    }

    (id, label, node_type, classes, md)
}

fn parse_mindmap_icon_directive(trimmed: &str) -> Option<String> {
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("::icon(") {
        return None;
    }
    let start = "::icon(".len();
    let end = trimmed[start..].find(')')? + start;
    let icon = trimmed[start..end].trim();
    if icon.is_empty() {
        None
    } else {
        Some(icon.to_string())
    }
}

fn parse_mindmap_shape(raw: &str) -> Option<(String, crate::ir::MindmapNodeType, bool)> {
    let trimmed = raw.trim();
    if trimmed.starts_with("((") && trimmed.ends_with("))") {
        let (t, md) = strip_quotes_markdown(&trimmed[2..trimmed.len() - 2]);
        return Some((t, crate::ir::MindmapNodeType::Circle, md));
    }
    if trimmed.starts_with("{{") && trimmed.ends_with("}}") {
        let (t, md) = strip_quotes_markdown(&trimmed[2..trimmed.len() - 2]);
        return Some((t, crate::ir::MindmapNodeType::Hexagon, md));
    }
    if trimmed.starts_with("))") && trimmed.ends_with("((") {
        let (t, md) = strip_quotes_markdown(&trimmed[2..trimmed.len() - 2]);
        return Some((t, crate::ir::MindmapNodeType::Bang, md));
    }
    if trimmed.starts_with(')') && trimmed.ends_with('(') {
        let (t, md) = strip_quotes_markdown(&trimmed[1..trimmed.len() - 1]);
        return Some((t, crate::ir::MindmapNodeType::Cloud, md));
    }
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let (t, md) = strip_quotes_markdown(&trimmed[1..trimmed.len() - 1]);
        return Some((t, crate::ir::MindmapNodeType::Rect, md));
    }
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let (t, md) = strip_quotes_markdown(&trimmed[1..trimmed.len() - 1]);
        return Some((t, crate::ir::MindmapNodeType::RoundedRect, md));
    }
    None
}

fn sanitize_id(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
        } else if (ch.is_whitespace() || ch == '-' || ch == '_') && !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

fn parse_journey_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Journey;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;

    let mut current_section: Option<usize> = None;
    let mut last_task: Option<String> = None;
    let mut acc_descr_block: Option<Vec<String>> = None;

    for raw_line in lines {
        let line = raw_line.trim();
        if let Some(lines) = acc_descr_block.as_mut() {
            if line == "}" {
                graph.acc_descr = Some(lines.join("\n"));
                acc_descr_block = None;
            } else {
                lines.push(line.to_string());
            }
            continue;
        }

        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("journey") {
            continue;
        }
        if lower.starts_with("acctitle") {
            if let Some(rest) = line.get(8..) {
                let rest = rest.trim().trim_start_matches(':').trim();
                if !rest.is_empty() {
                    graph.acc_title = Some(rest.to_string());
                }
            }
            continue;
        }
        if lower.starts_with("accdescr") {
            if line.contains('{') {
                acc_descr_block = Some(Vec::new());
            } else if let Some(rest) = line.get(8..) {
                let rest = rest.trim().trim_start_matches(':').trim();
                if !rest.is_empty() {
                    graph.acc_descr = Some(rest.to_string());
                }
            }
            continue;
        }
        if lower.starts_with("title") {
            let title = line.get(5..).unwrap_or("").trim();
            if !title.is_empty() {
                graph.journey_title = Some(title.to_string());
            }
            continue;
        }
        if lower.starts_with("section") {
            let label = line.get(7..).unwrap_or("").trim();
            let id = format!("section_{}", graph.subgraphs.len());
            graph.subgraphs.push(Subgraph {
                id: Some(id),
                label: label.to_string(),
                nodes: Vec::new(),
                direction: None,
                icon: None,
                markdown_label: false,
            });
            current_section = Some(graph.subgraphs.len() - 1);
            last_task = None;
            continue;
        }

        if let Some((label, score, actors)) = parse_journey_task_line(line) {
            let node_id = format!("journey_{}", graph.nodes.len());
            let mut node_label = label;
            if !actors.is_empty() {
                node_label.push_str(&format!("\n{}", actors.join(", ")));
            }
            graph.ensure_node(
                &node_id,
                Some(node_label),
                Some(crate::ir::NodeShape::Rectangle),
            );
            if let Some(score) = score {
                if let Some(node) = graph.nodes.get_mut(&node_id) {
                    node.value = Some(score);
                }
            }
            if let Some(idx) = current_section
                && let Some(subgraph) = graph.subgraphs.get_mut(idx)
            {
                subgraph.nodes.push(node_id.clone());
            }
            if let Some(prev) = last_task.take() {
                graph.edges.push(crate::ir::Edge {
                    from: prev,
                    to: node_id.clone(),
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
                    style: crate::ir::EdgeStyle::Solid,
                    markdown_label: false,
                    id: None,
                    curve: None,
                    arch_port_from: None,
                    arch_port_to: None,
                });
            }
            last_task = Some(node_id);
        }
    }
    if let Some(lines) = acc_descr_block.take() {
        graph.acc_descr = Some(lines.join("\n"));
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_journey_task_line(line: &str) -> Option<(String, Option<f32>, Vec<String>)> {
    let mut parts = line.split(':').map(|part| part.trim()).collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    let label = parts.remove(0).to_string();
    if label.is_empty() {
        return None;
    }
    let score = parts.first().and_then(|value| value.parse::<f32>().ok());
    let actors = if parts.len() >= 2 {
        parts[1]
            .split(',')
            .map(|actor| actor.trim().to_string())
            .filter(|actor| !actor.is_empty())
            .collect()
    } else {
        Vec::new()
    };
    Some((label, score, actors))
}

fn parse_timeline_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Timeline;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;

    let mut current_section: Option<String> = None;
    let mut pending_time: Option<String> = None;
    let mut pending_events: Vec<String> = Vec::new();

    let flush_pending = |graph: &mut Graph,
                         pending_time: &mut Option<String>,
                         pending_events: &mut Vec<String>,
                         current_section: &Option<String>| {
        if let Some(time) = pending_time.take() {
            graph.timeline.events.push(crate::ir::TimelineEvent {
                time,
                events: std::mem::take(pending_events),
                section: current_section.clone(),
            });
        }
        pending_events.clear();
    };

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("timeline") {
            continue;
        }
        if lower.starts_with("title") {
            let rest = line.get(5..).unwrap_or("").trim();
            if !rest.is_empty() {
                graph.timeline.title = Some(strip_quotes(rest));
            }
            continue;
        }
        if lower.starts_with("section") {
            // Flush any pending event before starting new section
            flush_pending(
                &mut graph,
                &mut pending_time,
                &mut pending_events,
                &current_section,
            );

            let label = line.get(7..).unwrap_or("").trim();
            graph.timeline.sections.push(label.to_string());
            current_section = Some(label.to_string());
            continue;
        }

        // Parse timeline event line: "time : event" or "time : event1 : event2"
        if let Some(colon_idx) = line.find(':') {
            let time_part = line[..colon_idx].trim();
            let events_part = line[colon_idx + 1..].trim();

            if !time_part.is_empty() {
                // New time entry - flush any previous
                flush_pending(
                    &mut graph,
                    &mut pending_time,
                    &mut pending_events,
                    &current_section,
                );
                pending_time = Some(time_part.to_string());

                // Parse events (can be multiple separated by :)
                for event in events_part.split(':') {
                    let event = event.trim();
                    if !event.is_empty() {
                        pending_events.push(event.to_string());
                    }
                }
            } else {
                // Continuation line (": event") — add to current time period
                for event in events_part.split(':') {
                    let event = event.trim();
                    if !event.is_empty() {
                        pending_events.push(event.to_string());
                    }
                }
            }
        }
    }

    // Flush any remaining pending event
    flush_pending(
        &mut graph,
        &mut pending_time,
        &mut pending_events,
        &current_section,
    );

    Ok(ParseOutput { graph, init_config })
}

fn parse_gantt_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Gantt;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;
    if let Some(display_mode) = gantt_init_display_mode(init_config.as_ref()) {
        graph.gantt_display_mode = Some(display_mode);
    }

    let mut current_section: Option<usize> = None;
    let mut current_section_name: Option<String> = None;
    let mut last_task: Option<String> = None;
    let mut auto_task_counter = 0usize;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("gantt") {
            continue;
        }
        if lower.starts_with("title") {
            let title = line.get(5..).unwrap_or("").trim();
            if !title.is_empty() {
                graph.gantt_title = Some(title.to_string());
            }
            continue;
        }
        if let Some(value) = gantt_directive_value(line, "dateFormat") {
            graph.gantt_date_format = Some(value.to_string());
            continue;
        }
        if let Some(value) = gantt_directive_value(line, "axisFormat") {
            graph.gantt_axis_format = Some(value.to_string());
            continue;
        }
        if let Some(value) = gantt_directive_value(line, "tickInterval") {
            graph.gantt_tick_interval = Some(value.to_string());
            continue;
        }
        if let Some(value) = gantt_directive_value(line, "todayMarker") {
            graph.gantt_today_marker = Some(value.to_string());
            continue;
        }
        if let Some(value) = gantt_directive_value(line, "excludes") {
            graph.gantt_excludes = split_gantt_directive_list(value);
            continue;
        }
        if let Some(value) = gantt_directive_value(line, "includes") {
            graph.gantt_includes = split_gantt_directive_list(value);
            continue;
        }
        if let Some(value) = gantt_directive_value(line, "weekday") {
            graph.gantt_weekday = Some(value.to_ascii_lowercase());
            continue;
        }
        if let Some(value) = gantt_directive_value(line, "weekend") {
            graph.gantt_weekend = Some(value.to_ascii_lowercase());
            continue;
        }
        if let Some(value) = gantt_directive_value(line, "displayMode") {
            graph.gantt_display_mode = Some(value.to_ascii_lowercase());
            continue;
        }
        if lower.starts_with("inclusiveenddates") {
            graph.gantt_inclusive_end_dates = true;
            continue;
        }
        if lower.starts_with("topaxis") {
            graph.gantt_top_axis = true;
            continue;
        }
        if lower.starts_with("section") {
            let label = line.get(7..).unwrap_or("").trim();
            let id = format!("section_{}", graph.subgraphs.len());
            graph.subgraphs.push(Subgraph {
                id: Some(id),
                label: label.to_string(),
                nodes: Vec::new(),
                direction: None,
                icon: None,
                markdown_label: false,
            });
            current_section = Some(graph.subgraphs.len() - 1);
            current_section_name = Some(label.to_string());
            graph.gantt_sections.push(label.to_string());
            continue;
        }

        if let Some((task_label, meta)) = line.split_once(':') {
            let label = task_label.trim();
            if label.is_empty() {
                continue;
            }
            let parsed = parse_gantt_task_meta(meta);
            let node_id = if let Some(id) = parsed.id.clone() {
                id
            } else {
                auto_task_counter += 1;
                format!("task{auto_task_counter}")
            };
            let mut node_label = label.to_string();
            if !parsed.details.is_empty() {
                node_label.push_str(&format!("\n{}", parsed.details.join(" | ")));
            }

            // Add to gantt_tasks
            let order = graph.gantt_tasks.len();
            graph.gantt_tasks.push(crate::ir::GanttTask {
                id: node_id.clone(),
                label: label.to_string(),
                start: parsed.start.clone(),
                end: parsed.end.clone(),
                duration: parsed.duration.clone(),
                after: parsed.after_ids.first().cloned(),
                after_ids: parsed.after_ids.clone(),
                until_ids: parsed.until_ids.clone(),
                section: current_section_name.clone(),
                status: parsed.status,
                active: parsed.active,
                done: parsed.done,
                crit: parsed.crit,
                milestone: parsed.milestone,
                vert: parsed.vert,
                order,
            });

            graph.ensure_node(
                &node_id,
                Some(node_label),
                Some(crate::ir::NodeShape::Rectangle),
            );
            if let Some(idx) = current_section
                && let Some(subgraph) = graph.subgraphs.get_mut(idx)
            {
                subgraph.nodes.push(node_id.clone());
            }

            if let Some(after_id) = parsed.after_ids.first().cloned() {
                graph.ensure_node(&after_id, None, Some(crate::ir::NodeShape::Rectangle));
                graph.edges.push(crate::ir::Edge {
                    from: after_id,
                    to: node_id.clone(),
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
            } else if let Some(prev) = last_task.take() {
                graph.edges.push(crate::ir::Edge {
                    from: prev,
                    to: node_id.clone(),
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
                    style: crate::ir::EdgeStyle::Solid,
                    markdown_label: false,
                    id: None,
                    curve: None,
                    arch_port_from: None,
                    arch_port_to: None,
                });
            }

            last_task = Some(node_id);
        }
    }

    Ok(ParseOutput { graph, init_config })
}

#[derive(Debug, Default)]
struct ParsedGanttTaskMeta {
    id: Option<String>,
    details: Vec<String>,
    start: Option<String>,
    end: Option<String>,
    duration: Option<String>,
    after_ids: Vec<String>,
    until_ids: Vec<String>,
    status: Option<crate::ir::GanttStatus>,
    active: bool,
    done: bool,
    crit: bool,
    milestone: bool,
    vert: bool,
}

fn parse_gantt_task_meta(meta: &str) -> ParsedGanttTaskMeta {
    let mut parsed = ParsedGanttTaskMeta::default();
    let mut data: Vec<String> = meta
        .trim_start_matches(':')
        .split(',')
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
        .collect();

    while let Some(first) = data.first() {
        let lower = first.to_ascii_lowercase();
        let Some(token_status) = gantt_status_from_token(&lower) else {
            break;
        };
        match token_status {
            crate::ir::GanttStatus::Done => parsed.done = true,
            crate::ir::GanttStatus::Active => parsed.active = true,
            crate::ir::GanttStatus::Crit => parsed.crit = true,
            crate::ir::GanttStatus::Milestone => parsed.milestone = true,
            crate::ir::GanttStatus::Vert => parsed.vert = true,
        }
        parsed.details.push(data.remove(0));
    }

    match data.len() {
        0 => {}
        1 => {
            apply_gantt_end_data(&mut parsed, &data[0]);
        }
        2 => {
            apply_gantt_start_data(&mut parsed, &data[0]);
            apply_gantt_end_data(&mut parsed, &data[1]);
        }
        _ => {
            parsed.id = Some(data[0].clone());
            apply_gantt_start_data(&mut parsed, &data[1]);
            apply_gantt_end_data(&mut parsed, &data[2]);
        }
    }

    parsed.status = if parsed.milestone {
        Some(crate::ir::GanttStatus::Milestone)
    } else if parsed.vert {
        Some(crate::ir::GanttStatus::Vert)
    } else if parsed.active {
        Some(crate::ir::GanttStatus::Active)
    } else if parsed.done {
        Some(crate::ir::GanttStatus::Done)
    } else if parsed.crit {
        Some(crate::ir::GanttStatus::Crit)
    } else {
        None
    };

    parsed
}

fn apply_gantt_start_data(parsed: &mut ParsedGanttTaskMeta, token: &str) {
    let lower = token.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("after ") {
        parsed.after_ids = split_gantt_dependency_ids(rest);
        parsed.details.push(token.to_string());
    } else {
        parsed.start = Some(token.trim().to_string());
        parsed.details.push(token.to_string());
    }
}

fn apply_gantt_end_data(parsed: &mut ParsedGanttTaskMeta, token: &str) {
    let lower = token.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("until ") {
        parsed.until_ids = split_gantt_dependency_ids(rest);
    } else if looks_like_duration(token) {
        parsed.duration = Some(token.trim().to_string());
    } else {
        parsed.end = Some(token.trim().to_string());
    }
    parsed.details.push(token.to_string());
}

fn gantt_status_from_token(token: &str) -> Option<crate::ir::GanttStatus> {
    match token {
        "done" => Some(crate::ir::GanttStatus::Done),
        "active" => Some(crate::ir::GanttStatus::Active),
        "crit" => Some(crate::ir::GanttStatus::Crit),
        "milestone" => Some(crate::ir::GanttStatus::Milestone),
        "vert" => Some(crate::ir::GanttStatus::Vert),
        _ => None,
    }
}

fn looks_like_duration(token: &str) -> bool {
    let token = token.trim();
    if token.len() < 2 {
        return false;
    }
    let Some(unit_start) = token.find(|ch: char| ch.is_ascii_alphabetic()) else {
        return false;
    };
    let (number, unit) = token.split_at(unit_start);
    if number.is_empty() || unit.is_empty() {
        return false;
    }
    number.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
        && matches!(unit, "ms" | "s" | "m" | "h" | "d" | "w" | "M" | "y")
}

fn gantt_directive_value<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    if line.len() >= keyword.len() && line[..keyword.len()].eq_ignore_ascii_case(keyword) {
        Some(line[keyword.len()..].trim())
    } else {
        None
    }
}

fn split_gantt_directive_list(value: &str) -> Vec<String> {
    value
        .split(|ch: char| ch.is_whitespace() || ch == ',')
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| !token.is_empty())
        .collect()
}

fn split_gantt_dependency_ids(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn gantt_init_display_mode(init_config: Option<&serde_json::Value>) -> Option<String> {
    let value = init_config?;
    value
        .get("displayMode")
        .or_else(|| value.pointer("/gantt/displayMode"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_ascii_lowercase())
}

fn requirement_kind_label(kind: &str) -> String {
    let trimmed = kind.trim();
    let lower = trimmed.to_ascii_lowercase();
    match lower.as_str() {
        "requirement" => "Requirement".to_string(),
        "functionalrequirement" => "Functional Requirement".to_string(),
        "interfacerequirement" => "Interface Requirement".to_string(),
        "performancerequirement" => "Performance Requirement".to_string(),
        "physicalrequirement" => "Physical Requirement".to_string(),
        "designconstraint" => "Design Constraint".to_string(),
        "element" => "Element".to_string(),
        "docref" => "Doc Ref".to_string(),
        _ => {
            let mut chars = lower.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            let mut out = String::new();
            out.push(first.to_ascii_uppercase());
            out.extend(chars);
            out
        }
    }
}

fn requirement_title_case(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let lower = trimmed.to_ascii_lowercase();
    let mut chars = lower.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.push(first.to_ascii_uppercase());
    out.extend(chars);
    out
}

fn normalize_requirement_attr(line: &str) -> String {
    let Some((key_raw, value_raw)) = line.split_once(':') else {
        return line.trim().to_string();
    };
    let key = key_raw.trim().to_ascii_lowercase();
    let value = strip_quotes(value_raw.trim());
    let pretty_key = match key.as_str() {
        "id" => "ID".to_string(),
        "text" => "Text".to_string(),
        "risk" => "Risk".to_string(),
        "verifymethod" | "verification" => "Verification".to_string(),
        other => requirement_kind_label(other),
    };
    let pretty_value = match key.as_str() {
        "risk" | "verifymethod" | "verification" => requirement_title_case(&value),
        _ => value,
    };
    if pretty_value.is_empty() {
        pretty_key
    } else {
        format!("{pretty_key}: {pretty_value}")
    }
}

fn parse_requirement_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Requirement;
    graph.direction = Direction::TopDown;
    let (lines, init_config) = preprocess_input(input)?;

    let mut attributes: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_id: Option<String> = None;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("requirementdiagram") {
            continue;
        }

        if let Some(active) = current_id.clone() {
            if let Some(end_idx) = line.find('}') {
                let fragment = line[..end_idx].trim();
                if !fragment.is_empty() {
                    attributes
                        .entry(active.clone())
                        .or_default()
                        .push(fragment.to_string());
                }
                current_id = None;
            } else {
                attributes
                    .entry(active.clone())
                    .or_default()
                    .push(line.to_string());
            }
            continue;
        }

        if let Some(direction) = parse_direction_line(line) {
            graph.direction = direction;
            continue;
        }

        if line.starts_with("classDef") {
            parse_class_def(line, &mut graph);
            continue;
        }

        if line.starts_with("class ") {
            parse_class_line(line, &mut graph);
            continue;
        }

        if line.starts_with("style ") {
            parse_style_line(line, &mut graph);
            continue;
        }

        if let Some((id, classes)) = parse_requirement_inline_class_assignment(line) {
            apply_node_classes(&mut graph, &id, &classes);
            continue;
        }

        if let Some((from, rel, to)) = parse_requirement_relation_line(line) {
            let is_contains = rel.eq_ignore_ascii_case("contains");
            graph.ensure_node(&from, None, Some(crate::ir::NodeShape::Rectangle));
            graph.ensure_node(&to, None, Some(crate::ir::NodeShape::Rectangle));
            graph.edges.push(crate::ir::Edge {
                from,
                to,
                label: Some(rel),
                start_label: None,
                end_label: None,
                directed: true,
                arrow_start: is_contains,
                arrow_end: !is_contains,
                arrow_start_kind: None,
                arrow_end_kind: None,
                start_decoration: None,
                end_decoration: None,
                sequence_arrow_end: None,
                sequence_arrow_start: None,
                style: if is_contains {
                    crate::ir::EdgeStyle::Solid
                } else {
                    crate::ir::EdgeStyle::Dotted
                },
                markdown_label: false,
                id: None,
                curve: None,
                arch_port_from: None,
                arch_port_to: None,
            });
            continue;
        }

        if let Some(open_idx) = line.find('{') {
            let header = line[..open_idx].trim();
            let mut parts = header.split_whitespace();
            let kind = parts.next().unwrap_or("").to_string();
            let (id, classes) = parse_requirement_id_token(parts.next().unwrap_or(""));
            if !id.is_empty() {
                let label = if kind.is_empty() {
                    id.clone()
                } else {
                    let kind_label = requirement_kind_label(&kind);
                    format!("<<{}>>\n{}", kind_label, id)
                };
                graph.ensure_node(&id, Some(label), Some(crate::ir::NodeShape::Rectangle));
                apply_node_classes(&mut graph, &id, &classes);
                current_id = Some(id.clone());
                let tail = line[open_idx + 1..].trim();
                if let Some(close_idx) = tail.find('}') {
                    let fragment = tail[..close_idx].trim();
                    if !fragment.is_empty() {
                        attributes.entry(id).or_default().push(fragment.to_string());
                    }
                    current_id = None;
                } else if !tail.is_empty() {
                    attributes.entry(id).or_default().push(tail.to_string());
                }
            }
            continue;
        }

        let mut parts = line.split_whitespace();
        let kind = parts.next().unwrap_or("");
        let (id, classes) = parse_requirement_id_token(parts.next().unwrap_or(""));
        if !id.is_empty() {
            let label = if kind.is_empty() {
                id.clone()
            } else {
                let kind_label = requirement_kind_label(kind);
                format!("<<{}>>\n{}", kind_label, id)
            };
            graph.ensure_node(&id, Some(label), Some(crate::ir::NodeShape::Rectangle));
            apply_node_classes(&mut graph, &id, &classes);
        }
    }

    for (id, node) in graph.nodes.iter_mut() {
        if let Some(attrs) = attributes.get(id)
            && !attrs.is_empty()
        {
            let mut lines = Vec::new();
            lines.push(node.label.clone());
            lines.extend(attrs.iter().map(|attr| normalize_requirement_attr(attr)));
            node.label = lines.join("\n");
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_requirement_id_token(token: &str) -> (String, Vec<String>) {
    let (base, classes) = split_inline_classes(token);
    (strip_quotes(base.trim()), classes)
}

fn parse_requirement_inline_class_assignment(line: &str) -> Option<(String, Vec<String>)> {
    let (base, classes) = split_inline_classes(line);
    if classes.is_empty() {
        return None;
    }
    let id = strip_quotes(base.trim());
    if id.is_empty() || id.chars().any(char::is_whitespace) {
        return None;
    }
    Some((id, classes))
}

fn parse_requirement_relation_line(line: &str) -> Option<(String, String, String)> {
    if let Some((to_part, right)) = line.split_once("<-") {
        let to = strip_quotes(to_part.trim());
        let right = right.trim();
        let (rel_part, from_part) = right.split_once('-')?;
        let from = strip_quotes(from_part.trim());
        let rel_clean = clean_requirement_relation(rel_part);
        if from.is_empty() || rel_clean.is_empty() || to.is_empty() {
            return None;
        }
        return Some((from, rel_clean, to));
    }

    let (left, right) = line.split_once("->")?;
    let to = strip_quotes(right.trim());
    if to.is_empty() {
        return None;
    }
    let left = left.trim();
    let (from_part, rel_part) = left.rsplit_once('-')?;
    let from = strip_quotes(from_part.trim());
    let rel_clean = clean_requirement_relation(rel_part);
    if from.is_empty() || rel_clean.is_empty() {
        return None;
    }
    Some((from, rel_clean, to))
}

fn clean_requirement_relation(rel: &str) -> String {
    rel.trim()
        .trim_matches('-')
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim()
        .to_string()
}

fn parse_gitgraph_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::GitGraph;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;

    let mut branch_heads: HashMap<String, Option<String>> = HashMap::new();
    let mut branch_insertion: HashMap<String, usize> = HashMap::new();
    let mut commit_index_by_id: HashMap<String, usize> = HashMap::new();

    let main_branch =
        gitgraph_init_main_branch_name(init_config.as_ref()).unwrap_or_else(|| "main".to_string());
    let main_branch_order = gitgraph_init_main_branch_order(init_config.as_ref()).unwrap_or(0.0);
    graph.gitgraph.main_branch = main_branch.clone();
    branch_heads.insert(main_branch.clone(), None);
    branch_insertion.insert(main_branch.clone(), 0);
    graph.gitgraph.branches.push(crate::ir::GitGraphBranch {
        name: main_branch.clone(),
        order: Some(main_branch_order),
        insertion_index: 0,
    });

    let mut current_branch = main_branch;
    let mut commit_seq: usize = 0;
    let mut rng = GitGraphIdRng::new(hash_seed(input));

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("gitgraph") {
            if let Some(direction) = parse_gitgraph_header_direction(line) {
                graph.direction = direction;
            }
            continue;
        }
        if let Some(direction) = parse_gitgraph_direction(line) {
            graph.direction = direction;
            continue;
        }
        if lower.starts_with("branch ") {
            let name = extract_gitgraph_branch_name(line);
            if !name.is_empty() {
                let order = extract_gitgraph_order(line).or(Some(0.0));
                let head = branch_heads.get(&current_branch).cloned().unwrap_or(None);
                branch_heads.insert(name.clone(), head);
                if !branch_insertion.contains_key(name.as_str()) {
                    let idx = graph.gitgraph.branches.len();
                    branch_insertion.insert(name.clone(), idx);
                    graph.gitgraph.branches.push(crate::ir::GitGraphBranch {
                        name: name.clone(),
                        order,
                        insertion_index: idx,
                    });
                }
                current_branch = name;
            }
            continue;
        }
        if lower.starts_with("checkout ") || lower.starts_with("switch ") {
            let raw_name = if lower.starts_with("checkout ") {
                line.get(9..).unwrap_or("").trim()
            } else {
                line.get(7..).unwrap_or("").trim()
            };
            let name = strip_quotes(raw_name);
            if !name.is_empty() {
                current_branch = name.clone();
                branch_heads.entry(current_branch.clone()).or_insert(None);
                if !branch_insertion.contains_key(name.as_str()) {
                    let idx = graph.gitgraph.branches.len();
                    branch_insertion.insert(name.clone(), idx);
                    graph.gitgraph.branches.push(crate::ir::GitGraphBranch {
                        name,
                        order: None,
                        insertion_index: idx,
                    });
                }
            }
            continue;
        }
        if lower.starts_with("merge ") {
            let from_branch = extract_gitgraph_merge_branch_name(line);
            if from_branch.is_empty() {
                continue;
            }
            let from_head = branch_heads
                .get(from_branch.as_str())
                .cloned()
                .unwrap_or(None);
            let current_head = branch_heads.get(&current_branch).cloned().unwrap_or(None);
            if from_head.is_none() && current_head.is_none() {
                continue;
            }
            let mut parents = Vec::new();
            if let Some(parent) = current_head.clone() {
                parents.push(parent);
            }
            if let Some(parent) = from_head.clone() {
                parents.push(parent);
            }

            let (id, custom_id) = extract_gitgraph_id(line)
                .map(|value| (value, true))
                .unwrap_or_else(|| {
                    let hex = rng.next_hex(7);
                    (format!("{commit_seq}-{hex}"), false)
                });
            let tags = extract_gitgraph_tags(line);
            let custom_type = extract_gitgraph_commit_type(line);
            let commit = crate::ir::GitGraphCommit {
                id: id.clone(),
                message: Some(format!(
                    "merged branch {} into {}",
                    from_branch, current_branch
                )),
                seq: commit_seq,
                commit_type: crate::ir::GitGraphCommitType::Merge,
                custom_type,
                tags,
                parents,
                branch: current_branch.clone(),
                custom_id,
            };
            commit_seq += 1;
            upsert_gitgraph_commit(&mut graph.gitgraph.commits, &mut commit_index_by_id, commit);
            branch_heads.insert(current_branch.clone(), Some(id));
            continue;
        }
        if lower.starts_with("commit") {
            let (id, custom_id) = extract_gitgraph_id(line)
                .map(|value| (value, true))
                .unwrap_or_else(|| {
                    let hex = rng.next_hex(7);
                    (format!("{commit_seq}-{hex}"), false)
                });
            let tags = extract_gitgraph_tags(line);
            let commit_type =
                extract_gitgraph_commit_type(line).unwrap_or(crate::ir::GitGraphCommitType::Normal);
            let parents = branch_heads
                .get(&current_branch)
                .cloned()
                .unwrap_or(None)
                .map(|parent| vec![parent])
                .unwrap_or_default();
            let message = extract_gitgraph_message(line);
            let commit = crate::ir::GitGraphCommit {
                id: id.clone(),
                message,
                seq: commit_seq,
                commit_type,
                custom_type: None,
                tags,
                parents,
                branch: current_branch.clone(),
                custom_id,
            };
            commit_seq += 1;
            upsert_gitgraph_commit(&mut graph.gitgraph.commits, &mut commit_index_by_id, commit);
            branch_heads.insert(current_branch.clone(), Some(id));
            continue;
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn upsert_gitgraph_commit(
    commits: &mut Vec<crate::ir::GitGraphCommit>,
    commit_index_by_id: &mut HashMap<String, usize>,
    commit: crate::ir::GitGraphCommit,
) {
    if let Some(index) = commit_index_by_id.get(commit.id.as_str()).copied() {
        commits[index] = commit;
    } else {
        commit_index_by_id.insert(commit.id.clone(), commits.len());
        commits.push(commit);
    }
}

fn parse_gitgraph_header_direction(line: &str) -> Option<Direction> {
    let mut parts = line.trim().split_whitespace();
    let header = parts.next()?;
    if !header
        .trim_end_matches(':')
        .eq_ignore_ascii_case("gitgraph")
    {
        return None;
    }
    let token = parts.next()?.trim_end_matches(':');
    parse_gitgraph_direction(token)
}

fn parse_gitgraph_direction(line: &str) -> Option<Direction> {
    let trimmed = line.trim();
    if trimmed.eq_ignore_ascii_case("LR") {
        return Some(Direction::LeftRight);
    }
    if trimmed.eq_ignore_ascii_case("TB") {
        return Some(Direction::TopDown);
    }
    if trimmed.eq_ignore_ascii_case("BT") {
        return Some(Direction::BottomTop);
    }
    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("direction") {
        let token = rest.trim();
        if token.eq_ignore_ascii_case("lr") {
            return Some(Direction::LeftRight);
        }
        if token.eq_ignore_ascii_case("tb") {
            return Some(Direction::TopDown);
        }
        if token.eq_ignore_ascii_case("bt") {
            return Some(Direction::BottomTop);
        }
    }
    None
}

fn extract_gitgraph_id(line: &str) -> Option<String> {
    extract_gitgraph_attr(line, "id")
}

fn extract_gitgraph_message(line: &str) -> Option<String> {
    extract_gitgraph_attr(line, "msg")
}

fn extract_gitgraph_commit_type(line: &str) -> Option<crate::ir::GitGraphCommitType> {
    let raw = extract_gitgraph_attr(line, "type")?;
    match raw.to_ascii_uppercase().as_str() {
        "NORMAL" => Some(crate::ir::GitGraphCommitType::Normal),
        "REVERSE" => Some(crate::ir::GitGraphCommitType::Reverse),
        "HIGHLIGHT" => Some(crate::ir::GitGraphCommitType::Highlight),
        _ => None,
    }
}

fn extract_gitgraph_order(line: &str) -> Option<f32> {
    let raw = extract_gitgraph_attr(line, "order")?;
    raw.parse::<f32>().ok()
}

fn gitgraph_init_main_branch_name(init_config: Option<&serde_json::Value>) -> Option<String> {
    init_config?
        .pointer("/gitGraph/mainBranchName")
        .or_else(|| init_config?.pointer("/gitgraph/mainBranchName"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn gitgraph_init_main_branch_order(init_config: Option<&serde_json::Value>) -> Option<f32> {
    init_config?
        .pointer("/gitGraph/mainBranchOrder")
        .or_else(|| init_config?.pointer("/gitgraph/mainBranchOrder"))
        .and_then(|value| {
            value
                .as_f64()
                .map(|number| number as f32)
                .or_else(|| value.as_str()?.trim().parse::<f32>().ok())
        })
}

fn extract_gitgraph_branch_name(line: &str) -> String {
    let rest = line.get(7..).unwrap_or("").trim();
    let Some(attr_start) = find_gitgraph_attr_start(rest, "order") else {
        return strip_quotes(rest);
    };
    strip_quotes(rest[..attr_start].trim())
}

fn extract_gitgraph_merge_branch_name(line: &str) -> String {
    let rest = line.get(6..).unwrap_or("").trim();
    let attr_start = ["tag", "id", "type"]
        .iter()
        .filter_map(|key| find_gitgraph_attr_start(rest, key))
        .min();
    let branch = attr_start
        .map(|idx| rest[..idx].trim())
        .unwrap_or(rest)
        .trim();
    strip_quotes(branch)
}

fn find_gitgraph_attr_start(text: &str, key: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut quote: Option<u8> = None;
    let mut idx = 0;
    while idx < bytes.len() {
        let byte = bytes[idx];
        if matches!(byte, b'\'' | b'"') {
            quote = if quote == Some(byte) {
                None
            } else {
                Some(byte)
            };
            idx += 1;
            continue;
        }
        if quote.is_none()
            && (idx == 0 || bytes[idx - 1].is_ascii_whitespace())
            && text[idx..].starts_with(key)
        {
            let after_key = idx + key.len();
            let mut cursor = after_key;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < bytes.len() && bytes[cursor] == b':' {
                return Some(idx);
            }
        }
        idx += 1;
    }
    None
}

fn extract_gitgraph_tags(line: &str) -> Vec<String> {
    extract_gitgraph_attrs(line, "tag")
}

fn extract_gitgraph_attrs(line: &str, key: &str) -> Vec<String> {
    let mut values = Vec::new();
    let lower = line.to_ascii_lowercase();
    let needle = format!("{}:", key);
    let mut start = 0;
    while let Some(idx) = lower[start..].find(&needle) {
        let offset = start + idx;
        if let Some((value, next)) = extract_gitgraph_attr_at(line, offset + needle.len()) {
            values.push(value);
            start = next;
        } else {
            break;
        }
    }
    values
}

fn extract_gitgraph_attr_at(line: &str, start: usize) -> Option<(String, usize)> {
    let bytes = line.as_bytes();
    let mut idx = start;
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    if idx >= bytes.len() {
        return None;
    }
    let first = bytes[idx] as char;
    if first == '"' || first == '\'' {
        idx += 1;
        let begin = idx;
        while idx < bytes.len() && bytes[idx] as char != first {
            idx += 1;
        }
        let value = String::from_utf8_lossy(&bytes[begin..idx]).to_string();
        let next = (idx + 1).min(bytes.len());
        return Some((value, next));
    }
    let begin = idx;
    while idx < bytes.len() && !bytes[idx].is_ascii_whitespace() && bytes[idx] != b',' {
        idx += 1;
    }
    let value = String::from_utf8_lossy(&bytes[begin..idx]).to_string();
    Some((value, idx))
}

fn hash_seed(input: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

struct GitGraphIdRng {
    state: u64,
}

impl GitGraphIdRng {
    fn new(seed: u64) -> Self {
        let seed = if seed == 0 {
            0xA5A5_A5A5_5A5A_5A5A
        } else {
            seed
        };
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        (x >> 32) as u32
    }

    fn next_hex(&mut self, len: usize) -> String {
        let mut out = String::with_capacity(len);
        for _ in 0..len {
            let val = (self.next_u32() & 0xF) as u8;
            out.push(std::char::from_digit(val as u32, 16).unwrap_or('0'));
        }
        out
    }
}

fn extract_gitgraph_attr(line: &str, key: &str) -> Option<String> {
    let needle = format!("{}:", key);
    let idx = line.find(&needle)?;
    let mut rest = line[idx + needle.len()..].trim_start();
    if rest.is_empty() {
        return None;
    }
    let first = rest.chars().next()?;
    if first == '"' || first == '\'' {
        rest = &rest[1..];
        if let Some(end) = rest.find(first) {
            return Some(rest[..end].to_string());
        }
        return Some(rest.to_string());
    }
    let end = rest
        .find(|ch: char| ch.is_whitespace() || ch == ',')
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

fn parse_c4_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::C4;
    graph.direction = Direction::LeftRight;
    graph.c4 = crate::ir::C4Data::default();
    graph.c4.boundaries.push(crate::ir::C4Boundary {
        id: "global".to_string(),
        label: "global".to_string(),
        boundary_type: "global".to_string(),
        descr: None,
        sprite: None,
        tags: None,
        link: None,
        parent_boundary: String::new(),
        bg_color: None,
        border_color: None,
        font_color: None,
    });
    let (lines, init_config) = preprocess_input(input)?;
    let mut boundary_stack: Vec<String> = vec!["global".to_string()];

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("c4") {
            graph.c4.c4_type = Some(line.trim().to_string());
            continue;
        }
        if let Some(title) = line.strip_prefix("title ") {
            graph.diagram_title = Some(strip_quotes(title.trim()));
            continue;
        }
        if line == "}" || lower == "end" {
            if boundary_stack.len() > 1 {
                boundary_stack.pop();
            }
            continue;
        }

        if let Some(brace_idx) = line.find('{') {
            let before = line[..brace_idx].trim();
            let after = line[brace_idx + 1..].trim();
            if !before.is_empty() {
                process_c4_line(before, &mut graph.c4, &mut boundary_stack);
            }
            if !after.is_empty() {
                let closes = after.ends_with('}');
                let after_trimmed = after.trim_end_matches('}').trim();
                if !after_trimmed.is_empty() {
                    process_c4_line(after_trimmed, &mut graph.c4, &mut boundary_stack);
                }
                if closes && boundary_stack.len() > 1 {
                    boundary_stack.pop();
                }
            }
            continue;
        }

        process_c4_line(line, &mut graph.c4, &mut boundary_stack);
    }

    Ok(ParseOutput { graph, init_config })
}

fn process_c4_line(line: &str, c4: &mut crate::ir::C4Data, boundary_stack: &mut Vec<String>) {
    if let Some((func, args)) = parse_function_call(line) {
        let func_lower = func.to_ascii_lowercase();
        let (positional, kv) = parse_c4_args(&args);
        if is_c4_boundary(&func_lower) {
            let id = positional
                .first()
                .cloned()
                .unwrap_or_else(|| format!("boundary_{}", c4.boundaries.len()));
            let label = positional.get(1).cloned().unwrap_or_else(|| id.clone());
            let mut boundary_type = positional.get(2).cloned();
            if let Some(value) = kv.get("type") {
                boundary_type = Some(value.clone());
            }
            let boundary_type =
                boundary_type.unwrap_or_else(|| c4_boundary_default_type(&func_lower));
            let descr = kv.get("descr").or_else(|| kv.get("description")).cloned();
            let sprite = kv.get("sprite").cloned();
            let tags = kv.get("tags").cloned();
            let link = kv.get("link").cloned();
            let parent_boundary = boundary_stack.last().cloned().unwrap_or_default();
            c4.boundaries.push(crate::ir::C4Boundary {
                id: id.clone(),
                label,
                boundary_type,
                descr,
                sprite,
                tags,
                link,
                parent_boundary,
                bg_color: None,
                border_color: None,
                font_color: None,
            });
            boundary_stack.push(id);
            return;
        }
        if let Some(rel_kind) = c4_rel_kind_for(&func_lower) {
            let mut rel_args = positional;
            if func_lower.starts_with("relindex") && rel_args.len() > 1 {
                rel_args.remove(0);
            }
            if rel_args.len() >= 3 {
                let from = rel_args[0].clone();
                let to = rel_args[1].clone();
                let label = rel_args[2].clone();
                let techn = rel_args
                    .get(3)
                    .cloned()
                    .or_else(|| kv.get("techn").cloned());
                let descr = rel_args
                    .get(4)
                    .cloned()
                    .or_else(|| kv.get("descr").cloned());
                let sprite = rel_args
                    .get(5)
                    .cloned()
                    .or_else(|| kv.get("sprite").cloned());
                let tags = rel_args.get(6).cloned().or_else(|| kv.get("tags").cloned());
                let link = rel_args.get(7).cloned().or_else(|| kv.get("link").cloned());
                c4.rels.push(crate::ir::C4Rel {
                    kind: rel_kind,
                    from,
                    to,
                    label,
                    techn,
                    descr,
                    sprite,
                    tags,
                    link,
                    offset_x: 0.0,
                    offset_y: 0.0,
                    line_color: None,
                    text_color: None,
                });
            }
            return;
        }

        if func_lower == "updateelementstyle"
            || func_lower == "update_el_style"
            || func_lower == "updateelstyle"
        {
            let element = positional
                .first()
                .cloned()
                .or_else(|| get_c4_kv(&kv, "element"));
            if let Some(element) = element {
                let bg_color = get_c4_kv(&kv, "bgColor").or_else(|| positional.get(1).cloned());
                let font_color = get_c4_kv(&kv, "fontColor").or_else(|| positional.get(2).cloned());
                let border_color =
                    get_c4_kv(&kv, "borderColor").or_else(|| positional.get(3).cloned());
                let sprite = get_c4_kv(&kv, "sprite").or_else(|| positional.get(6).cloned());
                let techn = get_c4_kv(&kv, "techn").or_else(|| positional.get(7).cloned());

                if let Some(shape) = c4.shapes.iter_mut().find(|s| s.id == element) {
                    if let Some(val) = bg_color {
                        shape.bg_color = Some(val);
                    }
                    if let Some(val) = font_color {
                        shape.font_color = Some(val);
                    }
                    if let Some(val) = border_color {
                        shape.border_color = Some(val);
                    }
                    if let Some(val) = sprite {
                        shape.sprite = Some(val);
                    }
                    if let Some(val) = techn {
                        shape.techn = Some(val);
                    }
                } else if let Some(boundary) = c4.boundaries.iter_mut().find(|b| b.id == element) {
                    if let Some(val) = bg_color {
                        boundary.bg_color = Some(val);
                    }
                    if let Some(val) = font_color {
                        boundary.font_color = Some(val);
                    }
                    if let Some(val) = border_color {
                        boundary.border_color = Some(val);
                    }
                    if let Some(val) = sprite {
                        boundary.sprite = Some(val);
                    }
                }
            }
            return;
        }

        if func_lower == "updaterelstyle" || func_lower == "update_rel_style" {
            let from = positional
                .first()
                .cloned()
                .or_else(|| get_c4_kv(&kv, "from"));
            let to = positional.get(1).cloned().or_else(|| get_c4_kv(&kv, "to"));
            if let (Some(from), Some(to)) = (from, to)
                && let Some(rel) = c4.rels.iter_mut().find(|r| r.from == from && r.to == to)
            {
                let text_color = get_c4_kv(&kv, "textColor").or_else(|| positional.get(2).cloned());
                let line_color = get_c4_kv(&kv, "lineColor").or_else(|| positional.get(3).cloned());
                let offset_x = get_c4_kv(&kv, "offsetX").or_else(|| positional.get(4).cloned());
                let offset_y = get_c4_kv(&kv, "offsetY").or_else(|| positional.get(5).cloned());
                if let Some(val) = text_color {
                    rel.text_color = Some(val);
                }
                if let Some(val) = line_color {
                    rel.line_color = Some(val);
                }
                if let Some(val) = offset_x
                    && let Ok(num) = val.trim().parse::<f32>()
                {
                    rel.offset_x = num;
                }
                if let Some(val) = offset_y
                    && let Ok(num) = val.trim().parse::<f32>()
                {
                    rel.offset_y = num;
                }
            }
            return;
        }

        if func_lower == "updatelayoutconfig" || func_lower == "update_layout_config" {
            let shape_in_row =
                get_c4_kv(&kv, "c4ShapeInRow").or_else(|| positional.first().cloned());
            let boundary_in_row =
                get_c4_kv(&kv, "c4BoundaryInRow").or_else(|| positional.get(1).cloned());
            if let Some(val) = shape_in_row
                && let Ok(num) = val.trim().parse::<usize>()
                && num >= 1
            {
                c4.c4_shape_in_row_override = Some(num);
            }
            if let Some(val) = boundary_in_row
                && let Ok(num) = val.trim().parse::<usize>()
                && num >= 1
            {
                c4.c4_boundary_in_row_override = Some(num);
            }
            return;
        }

        if let Some(kind) = c4_shape_kind_for(&func_lower)
            && let Some(id) = positional.first().cloned()
        {
            let label = positional.get(1).cloned().unwrap_or_else(|| id.clone());
            let mut type_label: Option<String> = None;
            let mut techn: Option<String> = None;
            let mut descr: Option<String> = None;
            let mut sprite: Option<String> = None;
            let mut tags: Option<String> = None;
            let mut link: Option<String> = None;
            if let Some(value) = kv.get("type") {
                type_label = Some(value.clone());
            }
            if let Some(value) = kv.get("techn").or_else(|| kv.get("technology")) {
                techn = Some(value.clone());
            }
            if let Some(value) = kv.get("descr").or_else(|| kv.get("description")) {
                descr = Some(value.clone());
            }
            if let Some(value) = kv.get("sprite") {
                sprite = Some(value.clone());
            }
            if let Some(value) = kv.get("tags") {
                tags = Some(value.clone());
            }
            if let Some(value) = kv.get("link") {
                link = Some(value.clone());
            }
            if kind_uses_techn(kind) {
                if techn.is_none() {
                    techn = positional.get(2).cloned();
                }
                if descr.is_none() {
                    descr = positional.get(3).cloned();
                }
                if sprite.is_none() {
                    sprite = positional.get(4).cloned();
                }
                if tags.is_none() {
                    tags = positional.get(5).cloned();
                }
                if link.is_none() {
                    link = positional.get(6).cloned();
                }
            } else {
                if descr.is_none() {
                    descr = positional.get(2).cloned();
                }
                if sprite.is_none() {
                    sprite = positional.get(3).cloned();
                }
                if tags.is_none() {
                    tags = positional.get(4).cloned();
                }
                if link.is_none() {
                    link = positional.get(5).cloned();
                }
            }
            let parent_boundary = boundary_stack.last().cloned().unwrap_or_default();
            c4.shapes.push(crate::ir::C4Shape {
                id,
                label,
                type_label,
                techn,
                descr,
                sprite,
                tags,
                link,
                parent_boundary,
                kind,
                bg_color: None,
                border_color: None,
                font_color: None,
            });
        }
    }
}

fn parse_function_call(line: &str) -> Option<(String, Vec<String>)> {
    let trimmed = line.trim();
    let open = trimmed.find('(')?;
    let close = trimmed.rfind(')')?;
    if close <= open {
        return None;
    }
    let func = trimmed[..open].trim();
    let args_str = &trimmed[open + 1..close];
    let args = split_args(args_str)
        .into_iter()
        .map(|arg| strip_quotes(arg.trim()))
        .collect();
    if func.is_empty() {
        None
    } else {
        Some((func.to_string(), args))
    }
}

fn split_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for ch in input.chars() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            current.push(ch);
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            current.push(ch);
            continue;
        }
        if ch == ',' {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                args.push(trimmed.to_string());
            }
            current.clear();
            continue;
        }
        current.push(ch);
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        args.push(trimmed.to_string());
    }
    args
}

fn parse_c4_args(args: &[String]) -> (Vec<String>, std::collections::HashMap<String, String>) {
    let mut positional = Vec::new();
    let mut kv = std::collections::HashMap::new();
    for arg in args {
        let trimmed = arg.trim();
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim().trim_start_matches('$');
            let value = clean_c4_arg_value(value);
            if !key.is_empty() {
                kv.insert(key.to_string(), value);
                continue;
            }
        }
        if !trimmed.is_empty() {
            positional.push(clean_c4_arg_value(trimmed));
        }
    }
    (positional, kv)
}

fn clean_c4_arg_value(value: &str) -> String {
    strip_quotes(value.trim())
}

fn normalize_c4_key(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    for ch in key.trim().trim_start_matches('$').chars() {
        if ch == '_' || ch == '-' {
            continue;
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

fn get_c4_kv(kv: &std::collections::HashMap<String, String>, key: &str) -> Option<String> {
    let target = normalize_c4_key(key);
    kv.iter()
        .find(|(k, _)| normalize_c4_key(k) == target)
        .map(|(_, v)| v.clone())
}

fn is_c4_boundary(func_lower: &str) -> bool {
    func_lower.contains("boundary")
        || func_lower.starts_with("deployment_node")
        || func_lower == "node"
        || func_lower == "node_l"
        || func_lower == "node_r"
}

fn c4_boundary_default_type(func_lower: &str) -> String {
    if func_lower.contains("enterprise") {
        "ENTERPRISE".to_string()
    } else if func_lower.contains("container") {
        "CONTAINER".to_string()
    } else if func_lower.contains("system") {
        "SYSTEM".to_string()
    } else if func_lower.contains("node") {
        "node".to_string()
    } else {
        "system".to_string()
    }
}

fn c4_shape_kind_for(func_lower: &str) -> Option<crate::ir::C4ShapeKind> {
    let f = func_lower.replace('-', "_");
    let is_ext = f.contains("ext");
    if f.contains("person") {
        return Some(if is_ext {
            crate::ir::C4ShapeKind::ExternalPerson
        } else {
            crate::ir::C4ShapeKind::Person
        });
    }
    if f.contains("system") {
        let is_db = f.contains("db");
        let is_queue = f.contains("queue");
        return Some(match (is_ext, is_db, is_queue) {
            (true, true, _) => crate::ir::C4ShapeKind::ExternalSystemDb,
            (true, _, true) => crate::ir::C4ShapeKind::ExternalSystemQueue,
            (true, _, _) => crate::ir::C4ShapeKind::ExternalSystem,
            (false, true, _) => crate::ir::C4ShapeKind::SystemDb,
            (false, _, true) => crate::ir::C4ShapeKind::SystemQueue,
            (false, _, _) => crate::ir::C4ShapeKind::System,
        });
    }
    if f.contains("container") {
        let is_db = f.contains("db");
        let is_queue = f.contains("queue");
        return Some(match (is_ext, is_db, is_queue) {
            (true, true, _) => crate::ir::C4ShapeKind::ExternalContainerDb,
            (true, _, true) => crate::ir::C4ShapeKind::ExternalContainerQueue,
            (true, _, _) => crate::ir::C4ShapeKind::ExternalContainer,
            (false, true, _) => crate::ir::C4ShapeKind::ContainerDb,
            (false, _, true) => crate::ir::C4ShapeKind::ContainerQueue,
            (false, _, _) => crate::ir::C4ShapeKind::Container,
        });
    }
    if f.contains("component") {
        let is_db = f.contains("db");
        let is_queue = f.contains("queue");
        return Some(match (is_ext, is_db, is_queue) {
            (true, true, _) => crate::ir::C4ShapeKind::ExternalComponentDb,
            (true, _, true) => crate::ir::C4ShapeKind::ExternalComponentQueue,
            (true, _, _) => crate::ir::C4ShapeKind::ExternalComponent,
            (false, true, _) => crate::ir::C4ShapeKind::ComponentDb,
            (false, _, true) => crate::ir::C4ShapeKind::ComponentQueue,
            (false, _, _) => crate::ir::C4ShapeKind::Component,
        });
    }
    None
}

fn kind_uses_techn(kind: crate::ir::C4ShapeKind) -> bool {
    matches!(
        kind,
        crate::ir::C4ShapeKind::Container
            | crate::ir::C4ShapeKind::ContainerDb
            | crate::ir::C4ShapeKind::ContainerQueue
            | crate::ir::C4ShapeKind::ExternalContainer
            | crate::ir::C4ShapeKind::ExternalContainerDb
            | crate::ir::C4ShapeKind::ExternalContainerQueue
            | crate::ir::C4ShapeKind::Component
            | crate::ir::C4ShapeKind::ComponentDb
            | crate::ir::C4ShapeKind::ComponentQueue
            | crate::ir::C4ShapeKind::ExternalComponent
            | crate::ir::C4ShapeKind::ExternalComponentDb
            | crate::ir::C4ShapeKind::ExternalComponentQueue
    )
}

fn c4_rel_kind_for(func_lower: &str) -> Option<crate::ir::C4RelKind> {
    let f = func_lower.replace('-', "_");
    if f.starts_with("birel") {
        return Some(crate::ir::C4RelKind::BiRel);
    }
    if f.starts_with("rel_u") || f.starts_with("rel_up") {
        return Some(crate::ir::C4RelKind::RelUp);
    }
    if f.starts_with("rel_d") || f.starts_with("rel_down") {
        return Some(crate::ir::C4RelKind::RelDown);
    }
    if f.starts_with("rel_l") || f.starts_with("rel_left") {
        return Some(crate::ir::C4RelKind::RelLeft);
    }
    if f.starts_with("rel_r") || f.starts_with("rel_right") {
        return Some(crate::ir::C4RelKind::RelRight);
    }
    if f.starts_with("rel_b") || f.starts_with("rel_back") {
        return Some(crate::ir::C4RelKind::RelBack);
    }
    if f.starts_with("rel") || f.starts_with("relindex") {
        return Some(crate::ir::C4RelKind::Rel);
    }
    None
}

fn parse_sankey_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Sankey;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("sankey") {
            continue;
        }
        let parts = split_args(line);
        if parts.len() < 3 {
            continue;
        }
        let from = strip_quotes(parts[0].trim());
        let to = strip_quotes(parts[1].trim());
        let value = parts[2].trim();
        if from.is_empty() || to.is_empty() {
            continue;
        }
        graph.ensure_node(&from, None, Some(crate::ir::NodeShape::Rectangle));
        graph.ensure_node(&to, None, Some(crate::ir::NodeShape::Rectangle));
        let label = if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        };
        graph.edges.push(crate::ir::Edge {
            from,
            to,
            label,
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
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_quadrant_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Quadrant;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("quadrantchart") {
            continue;
        }
        if lower.starts_with("title") {
            let title = line.get(5..).unwrap_or("").trim();
            if !title.is_empty() {
                graph.quadrant.title = Some(strip_quotes(title));
            }
            continue;
        }
        if lower.starts_with("x-axis") {
            // Format: x-axis Low Reach --> High Reach
            let rest = line.get(6..).unwrap_or("").trim();
            if let Some((left, right)) = rest.split_once("-->") {
                graph.quadrant.x_axis_left = Some(strip_quotes(left.trim()));
                graph.quadrant.x_axis_right = Some(strip_quotes(right.trim()));
            }
            continue;
        }
        if lower.starts_with("y-axis") {
            // Format: y-axis Low Engagement --> High Engagement
            let rest = line.get(6..).unwrap_or("").trim();
            if let Some((bottom, top)) = rest.split_once("-->") {
                graph.quadrant.y_axis_bottom = Some(strip_quotes(bottom.trim()));
                graph.quadrant.y_axis_top = Some(strip_quotes(top.trim()));
            }
            continue;
        }
        if lower.starts_with("quadrant-") {
            // Format: quadrant-1 We should expand
            if let Some(rest) = line.get(10..) {
                let label = strip_quotes(rest.trim());
                if lower.starts_with("quadrant-1") {
                    graph.quadrant.quadrant_labels[0] = Some(label);
                } else if lower.starts_with("quadrant-2") {
                    graph.quadrant.quadrant_labels[1] = Some(label);
                } else if lower.starts_with("quadrant-3") {
                    graph.quadrant.quadrant_labels[2] = Some(label);
                } else if lower.starts_with("quadrant-4") {
                    graph.quadrant.quadrant_labels[3] = Some(label);
                }
            }
            continue;
        }
        // Parse data points: Campaign A: [0.3, 0.6]
        if lower.starts_with("classdef") {
            parse_quadrant_class_def(line, &mut graph);
            continue;
        }
        if let Some(point) = parse_quadrant_point(line) {
            let node_id = format!("quadrant_{}", graph.nodes.len());
            graph.ensure_node(
                &node_id,
                Some(point.label.clone()),
                Some(crate::ir::NodeShape::Rectangle),
            );
            graph.quadrant.points.insert(0, point);
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_quadrant_point(line: &str) -> Option<crate::ir::QuadrantPoint> {
    let open = line.find('[').or_else(|| line.find('('))?;
    let close_ch = if line.as_bytes().get(open) == Some(&b'[') {
        ']'
    } else {
        ')'
    };
    let close = line[open + 1..].find(close_ch)? + open + 1;
    let raw_label = line[..open].trim().trim_end_matches(':').trim();
    let coords = &line[open + 1..close];
    let style_part = line[close + 1..].trim().trim_start_matches(',').trim();

    let (label_part, class_name) = raw_label
        .split_once(":::")
        .map(|(label, class_name)| {
            (
                label.trim(),
                class_name
                    .split(',')
                    .map(str::trim)
                    .find(|class_name| !class_name.is_empty())
                    .map(ToString::to_string),
            )
        })
        .unwrap_or((raw_label, None));

    let label = strip_quotes(label_part);
    if label.is_empty() {
        return None;
    }
    let mut parts = coords.split(',').map(|p| p.trim());
    let x: f32 = parts.next()?.parse().ok()?;
    let y: f32 = parts.next()?.parse().ok()?;
    Some(crate::ir::QuadrantPoint {
        label,
        x,
        y,
        class_name,
        style: parse_quadrant_point_style(style_part),
    })
}

fn parse_quadrant_class_def(line: &str, graph: &mut Graph) {
    let rest = line
        .trim()
        .strip_prefix("classDef")
        .or_else(|| line.trim().strip_prefix("classdef"))
        .unwrap_or("")
        .trim();
    let Some(split_at) = rest.find(char::is_whitespace) else {
        return;
    };
    let (names, styles) = rest.split_at(split_at);
    let styles = styles.trim();
    let style = parse_quadrant_point_style(styles);
    for class_name in names
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        graph
            .quadrant
            .point_classes
            .insert(class_name.to_string(), style.clone());
    }
}

fn parse_quadrant_point_style(input: &str) -> crate::ir::QuadrantPointStyle {
    let mut style = crate::ir::QuadrantPointStyle::default();
    for part in input.trim().trim_end_matches(';').split(',') {
        let Some((key, value)) = part.trim().split_once(':') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = strip_quotes(value.trim().trim_end_matches(';'));
        if value.is_empty() {
            continue;
        }
        match key.as_str() {
            "radius" => {
                let value = value.trim_end_matches("px");
                if let Ok(radius) = value.parse::<f32>() {
                    style.radius = Some(radius);
                }
            }
            "color" => style.color = Some(value),
            "stroke-color" => style.stroke_color = Some(value),
            "stroke-width" => style.stroke_width = Some(value),
            _ => {}
        }
    }
    style
}

fn parse_zenuml_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::ZenUML;
    graph.direction = Direction::LeftRight;
    graph.sequence_autonumber = Some(1);
    let (lines, init_config) = preprocess_input(input)?;
    let mut order: Vec<String> = Vec::new();
    let mut labels: HashMap<String, String> = HashMap::new();
    let mut block_stack: Vec<ZenUmlBlockContext> = Vec::new();
    let mut frames: Vec<crate::ir::SequenceFrame> = Vec::new();

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("zenuml") {
            continue;
        }
        if lower.starts_with("title") {
            let title = line.get(5..).unwrap_or("").trim();
            if !title.is_empty() {
                graph.diagram_title = Some(strip_quotes(title));
            }
            continue;
        }
        if lower.starts_with("//") || lower == "@" || lower == "@return" || lower == "@reply" {
            continue;
        }

        if zenuml_is_section_line(line) {
            zenuml_add_frame_section(&mut block_stack, &graph, line);
            continue;
        }
        if line == "}" {
            zenuml_close_block(&mut block_stack, &mut frames, graph.edges.len());
            continue;
        }

        if let Some((kind, label)) = parse_zenuml_frame_start(line) {
            let start_idx = graph.edges.len();
            block_stack.push(ZenUmlBlockContext::Frame(crate::ir::SequenceFrame {
                kind,
                sections: vec![crate::ir::SequenceFrameSection {
                    label,
                    start_idx,
                    end_idx: start_idx,
                }],
                start_idx,
                end_idx: start_idx,
            }));
            continue;
        }

        if let Some((id, label, shape)) = parse_zenuml_participant_line(line) {
            if !order.contains(&id) {
                order.push(id.clone());
            }
            if let Some(label) = label.clone() {
                labels.insert(id.clone(), label);
            }
            ensure_sequence_node(&mut graph, &labels, &id, Some(shape));
            continue;
        }

        if let Some((participant, label)) = parse_zenuml_creation_line(line) {
            ensure_zenuml_starter(&mut graph, &mut order);
            zenuml_ensure_participant(&mut graph, &labels, &mut order, &participant, None);
            graph.sequence_lifecycle.push(crate::ir::SequenceLifecycle {
                participant: participant.clone(),
                index: graph.edges.len(),
                kind: crate::ir::SequenceLifecycleKind::Create,
            });
            push_zenuml_edge(
                &mut graph,
                "_STARTER_".to_string(),
                participant,
                Some(label),
                crate::ir::EdgeStyle::Dotted,
                crate::ir::SequenceArrowHead::Open,
            );
            continue;
        }

        if let Some((from, to, label, style, arrow_head, open_call_target)) =
            parse_zenuml_message_line(line)
        {
            zenuml_ensure_participant(&mut graph, &labels, &mut order, &from, None);
            zenuml_ensure_participant(&mut graph, &labels, &mut order, &to, None);
            push_zenuml_edge(&mut graph, from, to.clone(), label, style, arrow_head);
            if let Some(target) = open_call_target {
                block_stack.push(ZenUmlBlockContext::Call(target));
            }
            continue;
        }

        if let Some((target, label, opens_block)) = parse_zenuml_call_line(line) {
            let from = zenuml_current_caller(&block_stack).unwrap_or_else(|| {
                ensure_zenuml_starter(&mut graph, &mut order);
                "_STARTER_".to_string()
            });
            zenuml_ensure_participant(&mut graph, &labels, &mut order, &from, None);
            zenuml_ensure_participant(&mut graph, &labels, &mut order, &target, None);
            push_zenuml_edge(
                &mut graph,
                from,
                target.clone(),
                Some(label),
                crate::ir::EdgeStyle::Solid,
                crate::ir::SequenceArrowHead::Filled,
            );
            if opens_block {
                block_stack.push(ZenUmlBlockContext::Call(target));
            }
            continue;
        }

        if let Some(label) = parse_zenuml_return_line(line) {
            if let Some((from, to)) = zenuml_return_participants(&block_stack) {
                zenuml_ensure_participant(&mut graph, &labels, &mut order, &from, None);
                if to == "_STARTER_" {
                    ensure_zenuml_starter(&mut graph, &mut order);
                } else {
                    zenuml_ensure_participant(&mut graph, &labels, &mut order, &to, None);
                }
                push_zenuml_edge(
                    &mut graph,
                    from,
                    to,
                    label,
                    crate::ir::EdgeStyle::Dotted,
                    crate::ir::SequenceArrowHead::Open,
                );
            }
        }
    }

    while !block_stack.is_empty() {
        zenuml_close_block(&mut block_stack, &mut frames, graph.edges.len());
    }

    graph.sequence_participants = order;
    graph.sequence_frames = frames;

    Ok(ParseOutput { graph, init_config })
}

#[derive(Debug, Clone)]
enum ZenUmlBlockContext {
    Call(String),
    Frame(crate::ir::SequenceFrame),
}

fn parse_zenuml_message_line(
    line: &str,
) -> Option<(
    String,
    String,
    Option<String>,
    crate::ir::EdgeStyle,
    crate::ir::SequenceArrowHead,
    Option<String>,
)> {
    let arrows = ["-->>", "->>", "-->", "->", "==>", "=>"];
    let mut found = None;
    for arrow in &arrows {
        if let Some(idx) = line.find(arrow) {
            found = Some((idx, *arrow));
            break;
        }
    }
    let (idx, arrow) = found?;
    let left = line[..idx].trim();
    let rest = line[idx + arrow.len()..].trim();
    if left.is_empty() || rest.is_empty() {
        return None;
    }
    let (right, label) = if let Some((r, l)) = rest.split_once(':') {
        let lbl = l.trim();
        let lbl = if lbl.is_empty() {
            None
        } else {
            Some(lbl.to_string())
        };
        (r.trim(), lbl)
    } else {
        (rest, None)
    };
    if right.is_empty() {
        return None;
    }
    let opens_block = line.trim_end().ends_with('{');
    let (from, _) = zenuml_endpoint_and_method(left);
    let (to, target_method) = zenuml_endpoint_and_method(right);
    if from.is_empty() || to.is_empty() {
        return None;
    }
    let label = label.or(target_method);
    let style = if arrow.contains("--") {
        crate::ir::EdgeStyle::Dotted
    } else {
        crate::ir::EdgeStyle::Solid
    };
    let arrow_head = if arrow.contains(">>") || arrow.contains('=') {
        crate::ir::SequenceArrowHead::Filled
    } else {
        crate::ir::SequenceArrowHead::Open
    };
    let open_call_target = if opens_block { Some(to.clone()) } else { None };
    Some((from, to, label, style, arrow_head, open_call_target))
}

fn parse_zenuml_participant_line(
    line: &str,
) -> Option<(String, Option<String>, crate::ir::NodeShape)> {
    let line = strip_zenuml_block_suffix(line);
    let lower = line.to_ascii_lowercase();
    if line.is_empty()
        || lower.starts_with("return")
        || lower.starts_with("new ")
        || lower.starts_with('@')
            && !lower.starts_with("@actor ")
            && !lower.starts_with("@boundary ")
            && !lower.starts_with("@control ")
            && !lower.starts_with("@database ")
            && !lower.starts_with("@entity ")
            && !lower.starts_with("@collections ")
            && !lower.starts_with("@queue ")
        || line.contains("->")
        || line.contains('.')
        || line.contains(':')
        || line.contains('=')
        || line.contains('(')
        || line.contains(')')
    {
        return None;
    }

    if let Some((keyword, shape)) = zenuml_annotator_shape(&lower) {
        let rest = line.get(keyword.len()..)?.trim();
        if rest.is_empty() {
            return None;
        }
        let id = strip_quotes(rest);
        return Some((id, None, shape));
    }

    if let Some((id, label, shape)) = parse_sequence_participant(&format!("participant {line}")) {
        return Some((id, label, shape));
    }
    None
}

fn zenuml_annotator_shape(lower: &str) -> Option<(&'static str, crate::ir::NodeShape)> {
    let entries = [
        ("@actor ", crate::ir::NodeShape::StickFigure),
        ("@boundary ", crate::ir::NodeShape::Boundary),
        ("@control ", crate::ir::NodeShape::Control),
        ("@database ", crate::ir::NodeShape::Cylinder),
        ("@entity ", crate::ir::NodeShape::Entity),
        ("@collections ", crate::ir::NodeShape::Collections),
        ("@queue ", crate::ir::NodeShape::Queue),
    ];
    entries
        .into_iter()
        .find_map(|(keyword, shape)| lower.starts_with(keyword).then_some((keyword, shape)))
}

fn parse_zenuml_creation_line(line: &str) -> Option<(String, String)> {
    let line = strip_zenuml_block_suffix(line);
    let lower = line.to_ascii_lowercase();
    if !lower.starts_with("new ") {
        return None;
    }
    let rest = line.get(4..)?.trim();
    if rest.is_empty() {
        return None;
    }
    let (id, params) = if let Some(open) = rest.find('(') {
        let id = rest[..open].trim();
        let params = rest
            .get(open + 1..)
            .and_then(|tail| tail.rsplit_once(')').map(|(inside, _)| inside.trim()))
            .unwrap_or_default();
        (id, params)
    } else {
        (rest.split_whitespace().next().unwrap_or_default(), "")
    };
    if id.is_empty() {
        return None;
    }
    let label = if params.is_empty() {
        "«create»".to_string()
    } else {
        format!("«{}»", params)
    };
    Some((strip_quotes(id), label))
}

fn parse_zenuml_call_line(line: &str) -> Option<(String, String, bool)> {
    if line.contains("->") {
        return None;
    }
    let opens_block = line.trim_end().ends_with('{');
    let line = strip_zenuml_block_suffix(line);
    let rhs = line
        .rsplit_once('=')
        .map(|(_, rhs)| rhs.trim())
        .unwrap_or(line);
    let dot = rhs.find('.')?;
    let target = rhs[..dot].trim();
    let signature = rhs[dot + 1..].trim();
    if target.is_empty() || signature.is_empty() {
        return None;
    }
    Some((strip_quotes(target), signature.to_string(), opens_block))
}

fn parse_zenuml_return_line(line: &str) -> Option<Option<String>> {
    let lower = line.to_ascii_lowercase();
    if !lower.starts_with("return") {
        return None;
    }
    let label = line.get(6..).unwrap_or_default().trim();
    Some((!label.is_empty()).then(|| label.to_string()))
}

fn parse_zenuml_frame_start(line: &str) -> Option<(crate::ir::SequenceFrameKind, Option<String>)> {
    if !line.trim_end().ends_with('{') {
        return None;
    }
    let head = strip_zenuml_block_suffix(line);
    let lower = head.to_ascii_lowercase();
    let specs = [
        ("if", crate::ir::SequenceFrameKind::Alt),
        ("while", crate::ir::SequenceFrameKind::Loop),
        ("par", crate::ir::SequenceFrameKind::Par),
        ("opt", crate::ir::SequenceFrameKind::Opt),
        ("try", crate::ir::SequenceFrameKind::Alt),
    ];
    for (keyword, kind) in specs {
        if lower == keyword
            || lower.starts_with(&format!("{keyword}("))
            || lower.starts_with(&format!("{keyword} "))
        {
            let label = zenuml_keyword_label(&head, keyword);
            return Some((kind, label));
        }
    }
    None
}

fn zenuml_is_section_line(line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    lower.starts_with("} else")
        || lower == "else"
        || lower.starts_with("else ")
        || lower.starts_with("} catch")
        || lower == "catch"
        || lower.starts_with("catch ")
        || lower.starts_with("} finally")
        || lower == "finally"
        || lower.starts_with("finally ")
}

fn zenuml_add_frame_section(block_stack: &mut [ZenUmlBlockContext], graph: &Graph, line: &str) {
    let label = zenuml_section_label(line);
    if let Some(ZenUmlBlockContext::Frame(frame)) = block_stack
        .iter_mut()
        .rev()
        .find(|ctx| matches!(ctx, ZenUmlBlockContext::Frame(_)))
    {
        let split_idx = graph.edges.len();
        if let Some(last) = frame.sections.last_mut() {
            last.end_idx = split_idx;
        }
        frame.sections.push(crate::ir::SequenceFrameSection {
            label,
            start_idx: split_idx,
            end_idx: split_idx,
        });
    }
}

fn zenuml_close_block(
    block_stack: &mut Vec<ZenUmlBlockContext>,
    frames: &mut Vec<crate::ir::SequenceFrame>,
    end_idx: usize,
) {
    match block_stack.pop() {
        Some(ZenUmlBlockContext::Frame(mut frame)) => {
            if let Some(last) = frame.sections.last_mut() {
                last.end_idx = end_idx;
            }
            frame.end_idx = end_idx;
            frames.push(frame);
        }
        Some(ZenUmlBlockContext::Call(_)) | None => {}
    }
}

fn zenuml_current_caller(block_stack: &[ZenUmlBlockContext]) -> Option<String> {
    block_stack.iter().rev().find_map(|ctx| match ctx {
        ZenUmlBlockContext::Call(id) => Some(id.clone()),
        ZenUmlBlockContext::Frame(_) => None,
    })
}

fn zenuml_return_participants(block_stack: &[ZenUmlBlockContext]) -> Option<(String, String)> {
    let mut calls = block_stack.iter().rev().filter_map(|ctx| match ctx {
        ZenUmlBlockContext::Call(id) => Some(id.clone()),
        ZenUmlBlockContext::Frame(_) => None,
    });
    let from = calls.next()?;
    let to = calls.next().unwrap_or_else(|| "_STARTER_".to_string());
    Some((from, to))
}

fn zenuml_endpoint_and_method(token: &str) -> (String, Option<String>) {
    let token = strip_zenuml_block_suffix(token)
        .trim_end_matches(';')
        .trim()
        .to_string();
    if let Some(dot) = token.find('.') {
        let id = strip_quotes(token[..dot].trim());
        let signature = token[dot + 1..].trim();
        let signature = (!signature.is_empty()).then(|| signature.to_string());
        (id, signature)
    } else {
        (strip_quotes(&token), None)
    }
}

fn strip_zenuml_block_suffix(line: &str) -> &str {
    line.trim().strip_suffix('{').unwrap_or(line.trim()).trim()
}

fn zenuml_keyword_label(line: &str, keyword: &str) -> Option<String> {
    let rest = line.get(keyword.len()..).unwrap_or_default().trim();
    let label = if rest.starts_with('(') && rest.ends_with(')') && rest.len() >= 2 {
        rest[1..rest.len() - 1].trim()
    } else {
        rest
    };
    (!label.is_empty()).then(|| label.to_string())
}

fn zenuml_section_label(line: &str) -> Option<String> {
    let line = strip_zenuml_block_suffix(line);
    let line = line.trim_start_matches('}').trim();
    let lower = line.to_ascii_lowercase();
    for keyword in ["else", "catch", "finally"] {
        if lower == keyword || lower.starts_with(&format!("{keyword} ")) {
            let label = zenuml_keyword_label(line, keyword).unwrap_or_else(|| keyword.to_string());
            return Some(label);
        }
    }
    None
}

fn ensure_zenuml_starter(graph: &mut Graph, order: &mut Vec<String>) {
    let id = "_STARTER_".to_string();
    if !order.contains(&id) {
        order.push(id.clone());
    }
    graph.ensure_node(
        &id,
        Some(String::new()),
        Some(crate::ir::NodeShape::ActorBox),
    );
}

fn zenuml_ensure_participant(
    graph: &mut Graph,
    labels: &HashMap<String, String>,
    order: &mut Vec<String>,
    id: &str,
    shape: Option<crate::ir::NodeShape>,
) {
    if !order.contains(&id.to_string()) {
        order.push(id.to_string());
    }
    ensure_sequence_node(graph, labels, id, shape);
}

fn push_zenuml_edge(
    graph: &mut Graph,
    from: String,
    to: String,
    label: Option<String>,
    style: crate::ir::EdgeStyle,
    arrow_head: crate::ir::SequenceArrowHead,
) {
    graph.edges.push(crate::ir::Edge {
        from,
        to,
        label,
        start_label: None,
        end_label: None,
        directed: true,
        arrow_start: false,
        arrow_end: true,
        arrow_start_kind: None,
        arrow_end_kind: None,
        start_decoration: None,
        end_decoration: None,
        sequence_arrow_end: Some(arrow_head),
        sequence_arrow_start: None,
        style,
        markdown_label: false,
        id: None,
        curve: None,
        arch_port_from: None,
        arch_port_to: None,
    });
}

fn parse_block_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Block;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;
    let mut block = crate::ir::BlockDiagram::default();
    let mut seen_header = false;
    let mut block_subgraph_stack: Vec<usize> = Vec::new();
    let mut block_group_stack: Vec<String> = Vec::new();
    let mut anonymous_block_count = 0usize;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower == "block" || lower == "block-beta" {
            if !seen_header {
                seen_header = true;
                continue;
            }
            let id = format!("__block_{}", anonymous_block_count);
            anonymous_block_count += 1;
            push_block_node(
                &mut block,
                &block_group_stack,
                crate::ir::BlockNode {
                    id: id.clone(),
                    span: 1,
                    is_space: false,
                },
            );
            block
                .groups
                .entry(id.clone())
                .or_insert_with(crate::ir::BlockGroup::default);
            graph.subgraphs.push(Subgraph {
                id: Some(id),
                label: String::new(),
                nodes: Vec::new(),
                direction: None,
                icon: None,
                markdown_label: false,
            });
            block_subgraph_stack.push(graph.subgraphs.len() - 1);
            block_group_stack.push(
                graph
                    .subgraphs
                    .last()
                    .and_then(|subgraph| subgraph.id.clone())
                    .unwrap_or_default(),
            );
            continue;
        }
        if let Some((id, span)) = parse_block_composite_header(line) {
            push_block_node(
                &mut block,
                &block_group_stack,
                crate::ir::BlockNode {
                    id: id.clone(),
                    span,
                    is_space: false,
                },
            );
            block
                .groups
                .entry(id.clone())
                .or_insert_with(crate::ir::BlockGroup::default);
            graph.subgraphs.push(Subgraph {
                id: Some(id),
                label: String::new(),
                nodes: Vec::new(),
                direction: None,
                icon: None,
                markdown_label: false,
            });
            block_subgraph_stack.push(graph.subgraphs.len() - 1);
            block_group_stack.push(
                graph
                    .subgraphs
                    .last()
                    .and_then(|subgraph| subgraph.id.clone())
                    .unwrap_or_default(),
            );
            continue;
        }
        if lower == "end" {
            block_subgraph_stack.pop();
            block_group_stack.pop();
            continue;
        }
        if lower.starts_with("columns") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let parsed_columns = if parts.len() >= 2 && parts[1].eq_ignore_ascii_case("auto") {
                Some(None)
            } else if parts.len() >= 2 {
                parts[1]
                    .parse::<usize>()
                    .ok()
                    .filter(|cols| *cols > 0)
                    .map(Some)
            } else {
                None
            };
            if let Some(columns) = parsed_columns {
                if let Some(group_id) = block_group_stack.last() {
                    if let Some(group) = block.groups.get_mut(group_id) {
                        group.columns = columns;
                    }
                } else {
                    block.columns = columns;
                }
            }
            continue;
        }
        if line.starts_with("classDef") {
            parse_class_def(line, &mut graph);
            continue;
        }
        if line.starts_with("class ") {
            parse_class_line(line, &mut graph);
            continue;
        }
        if line.starts_with("style ") {
            parse_style_line(line, &mut graph);
            continue;
        }
        if line.starts_with("linkStyle") {
            parse_link_style_line(line, &mut graph);
            continue;
        }
        if let Some((left, label, right, edge_meta)) = parse_edge_line(line) {
            let sources = split_on_ampersand(&left);
            let targets = split_on_ampersand(&right);

            for source in &sources {
                let (source_id, source_label, source_shape, source_classes, _source_md) =
                    parse_node_token(source);
                if is_block_composite_id(&graph, &source_id) {
                    continue;
                }
                graph.ensure_node(&source_id, source_label, source_shape);
                if !source_classes.is_empty() {
                    apply_node_classes(&mut graph, &source_id, &source_classes);
                }
                add_node_to_subgraphs(&mut graph, &block_subgraph_stack, &source_id);
            }
            for target in &targets {
                let (target_id, target_label, target_shape, target_classes, _target_md) =
                    parse_node_token(target);
                if is_block_composite_id(&graph, &target_id) {
                    continue;
                }
                graph.ensure_node(&target_id, target_label, target_shape);
                if !target_classes.is_empty() {
                    apply_node_classes(&mut graph, &target_id, &target_classes);
                }
                add_node_to_subgraphs(&mut graph, &block_subgraph_stack, &target_id);
            }

            for source in &sources {
                let (source_id, _, _, _, _node_md) = parse_node_token(source);
                for target in &targets {
                    let (target_id, _, _, _, _node_md) = parse_node_token(target);
                    graph.edges.push(crate::ir::Edge {
                        from: source_id.clone(),
                        to: target_id.clone(),
                        label: label.clone(),
                        start_label: None,
                        end_label: None,
                        directed: edge_meta.directed,
                        arrow_start: edge_meta.arrow_start,
                        arrow_end: edge_meta.arrow_end,
                        arrow_start_kind: edge_meta.arrow_start_kind,
                        arrow_end_kind: edge_meta.arrow_end_kind,
                        start_decoration: edge_meta.start_decoration,
                        end_decoration: edge_meta.end_decoration,
                        sequence_arrow_end: None,
                        sequence_arrow_start: None,
                        style: edge_meta.style,
                        markdown_label: false,
                        id: None,
                        curve: None,
                        arch_port_from: None,
                        arch_port_to: None,
                    });
                }
            }
            continue;
        }

        let mut tokens = split_block_row_tokens(line);
        if tokens.is_empty() {
            continue;
        }
        for raw in tokens.drain(..) {
            let mut token = raw.trim();
            if token.is_empty() {
                continue;
            }
            let mut span = 1usize;
            if let Some((base, span_str)) = token.rsplit_once(':')
                && let Ok(parsed_span) = span_str.parse::<usize>()
                && parsed_span > 0
            {
                span = parsed_span;
                token = base;
            }
            let is_space = token.eq_ignore_ascii_case("space");
            if is_space {
                push_block_node(
                    &mut block,
                    &block_group_stack,
                    crate::ir::BlockNode {
                        id: "__space".to_string(),
                        span,
                        is_space: true,
                    },
                );
                continue;
            }
            let (id, label, shape, classes, _node_md) = parse_node_token(token);
            if id.is_empty() {
                continue;
            }
            graph.ensure_node(&id, label, shape);
            if !classes.is_empty() {
                apply_node_classes(&mut graph, &id, &classes);
            }
            add_node_to_subgraphs(&mut graph, &block_subgraph_stack, &id);
            push_block_node(
                &mut block,
                &block_group_stack,
                crate::ir::BlockNode {
                    id,
                    span,
                    is_space: false,
                },
            );
        }
    }

    // Keep block metadata even when the DSL only contains edge lines.
    // The layout stage infers an implicit grid from graph topology in that case.
    graph.block = Some(block);

    Ok(ParseOutput { graph, init_config })
}

fn push_block_node(
    block: &mut crate::ir::BlockDiagram,
    group_stack: &[String],
    node: crate::ir::BlockNode,
) {
    if let Some(group_id) = group_stack.last()
        && let Some(group) = block.groups.get_mut(group_id)
    {
        group.nodes.push(node);
        return;
    }
    block.nodes.push(node);
}

fn parse_block_composite_header(line: &str) -> Option<(String, usize)> {
    let rest = line.strip_prefix("block:")?.trim();
    if rest.is_empty() {
        return None;
    }
    let (id, span) = if let Some((base, span_raw)) = rest.rsplit_once(':') {
        if let Ok(span) = span_raw.trim().parse::<usize>() {
            (base.trim(), span.max(1))
        } else {
            (rest, 1)
        }
    } else {
        (rest, 1)
    };
    if id.is_empty() {
        return None;
    }
    Some((id.to_string(), span))
}

fn is_block_composite_id(graph: &Graph, id: &str) -> bool {
    graph
        .subgraphs
        .iter()
        .any(|sub| sub.id.as_deref() == Some(id))
}

fn split_block_row_tokens(line: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = None;
    let mut quote = None;
    let mut escaped = false;
    let mut square_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut asymmetric = false;

    for (idx, ch) in line.char_indices() {
        if start.is_none() {
            if ch.is_whitespace() {
                continue;
            }
            start = Some(idx);
        }

        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == active_quote {
                quote = None;
            }
            continue;
        }

        match ch {
            '"' | '\'' | '`' => quote = Some(ch),
            '[' => square_depth += 1,
            ']' => {
                square_depth = square_depth.saturating_sub(1);
                asymmetric = false;
            }
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '>' if square_depth == 0
                && paren_depth == 0
                && brace_depth == 0
                && line[..idx].chars().next_back() != Some(']') =>
            {
                asymmetric = true;
            }
            _ if ch.is_whitespace()
                && square_depth == 0
                && paren_depth == 0
                && brace_depth == 0
                && !asymmetric =>
            {
                if let Some(token_start) = start.take() {
                    tokens.push(&line[token_start..idx]);
                }
            }
            _ => {}
        }
    }

    if let Some(token_start) = start {
        tokens.push(&line[token_start..]);
    }

    tokens
}

fn parse_packet_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Packet;
    graph.direction = Direction::LeftRight;
    graph.packet.title = extract_yaml_frontmatter_title(input);
    let (lines, init_config) = preprocess_input(input)?;
    let mut last_bit: Option<u32> = None;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("packet") {
            continue;
        }
        if lower.starts_with("title") {
            let title = line.get(5..).unwrap_or("").trim();
            if !title.is_empty() {
                graph.packet.title = Some(strip_quotes(title));
            }
            continue;
        }
        if lower.starts_with("acctitle") {
            if let Some(rest) = line.get(8..) {
                let rest = rest.trim().trim_start_matches(':').trim();
                if !rest.is_empty() {
                    graph.acc_title = Some(rest.to_string());
                }
            }
            continue;
        }
        if lower.starts_with("accdescr") {
            if let Some(rest) = line.get(8..) {
                let rest = rest.trim().trim_start_matches(':').trim();
                if !rest.is_empty() {
                    graph.acc_descr = Some(rest.to_string());
                }
            }
            continue;
        }

        if let Some((range, label)) = line.split_once(':') {
            let range = range.trim();
            let label = strip_quotes(label.trim());
            if range.is_empty() {
                continue;
            }
            if let Some((start, end)) = parse_packet_range(range, last_bit) {
                graph
                    .packet
                    .blocks
                    .push(crate::ir::PacketBlock { start, end, label });
                last_bit = Some(end);
            }
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_packet_range(input: &str, last_bit: Option<u32>) -> Option<(u32, u32)> {
    let range = input.trim();
    if let Some(bits) = range.strip_prefix('+') {
        let bits = bits.trim().parse::<u32>().ok()?;
        if bits == 0 {
            return None;
        }
        let start = last_bit.map_or(0, |bit| bit.saturating_add(1));
        return Some((start, start + bits - 1));
    }

    if let Some((start, end)) = range.split_once('-') {
        let start = start.trim().parse::<u32>().ok()?;
        let end = end.trim().parse::<u32>().ok()?;
        if end < start {
            return None;
        }
        return Some((start, end));
    }

    let bit = range.parse::<u32>().ok()?;
    Some((bit, bit))
}

fn parse_kanban_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Kanban;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input_keep_indent(input)?;
    let mut current_section: Option<usize> = None;
    let mut base_indent: Option<usize> = None;

    for raw_line in lines {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("kanban") {
            continue;
        }
        let indent = count_indent(&raw_line);
        let base = *base_indent.get_or_insert(indent);
        if indent <= base {
            let (id, col_label) = parse_kanban_node_label(trimmed);
            graph.subgraphs.push(Subgraph {
                id: Some(id),
                label: col_label,
                nodes: Vec::new(),
                direction: None,
                icon: None,
                markdown_label: false,
            });
            current_section = Some(graph.subgraphs.len() - 1);
            continue;
        }

        let (task_part, meta) = if let Some((left, right)) = trimmed.split_once("@{") {
            let meta = right.trim_end_matches('}').trim();
            (left.trim(), Some(meta.to_string()))
        } else {
            (trimmed, None)
        };
        let (mut id, mut node_label) = parse_kanban_node_label(task_part);
        if graph.nodes.contains_key(&id) {
            id = format!("{}_{}", id, graph.nodes.len());
        }
        if let Some(meta) = meta
            && !meta.is_empty()
        {
            node_label.push_str(&format!("\n{}", meta));
        }
        graph.ensure_node(&id, Some(node_label), Some(crate::ir::NodeShape::Rectangle));
        if let Some(idx) = current_section
            && let Some(subgraph) = graph.subgraphs.get_mut(idx)
        {
            subgraph.nodes.push(id);
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_kanban_node_label(token: &str) -> (String, String) {
    let trimmed = token.trim();
    if let Some(label) = parse_kanban_node_without_id(trimmed) {
        return (label.clone(), label);
    }

    let (id, label, _shape, _classes, _node_md) = parse_node_token(trimmed);
    let label = label.unwrap_or_else(|| id.clone());
    (id, label)
}

fn parse_kanban_node_without_id(token: &str) -> Option<String> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return Some(parse_shape_from_brackets(trimmed).0);
    }
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        return Some(parse_shape_from_parens(trimmed).0);
    }
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(parse_shape_from_braces(trimmed).0);
    }

    None
}

fn parse_architecture_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Architecture;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;
    let mut groups: HashMap<String, usize> = HashMap::new();

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("architecture") {
            continue;
        }
        if lower.starts_with("junction ") {
            // Junction is an invisible routing point — no icon, no label.
            let id = line.split_whitespace().nth(1).unwrap_or("").to_string();
            if !id.is_empty() {
                graph.ensure_node(
                    &id,
                    Some(String::new()),
                    Some(crate::ir::NodeShape::Rectangle),
                );
                if let Some(node) = graph.nodes.get_mut(&id) {
                    node.label = String::new();
                }
                // Mark as junction via a special class
                graph
                    .node_classes
                    .entry(id)
                    .or_default()
                    .push("__junction__".to_string());
            }
            continue;
        }
        if lower.starts_with("group ") || lower.starts_with("service ") {
            if let Some((kind, id, label, parent, icon)) = parse_architecture_node(line) {
                if kind == "group" {
                    graph.subgraphs.push(Subgraph {
                        id: Some(id.clone()),
                        label: label.clone(),
                        nodes: Vec::new(),
                        direction: None,
                        icon: icon,
                        markdown_label: false,
                    });
                    groups.insert(id, graph.subgraphs.len() - 1);
                } else {
                    graph.ensure_node(&id, Some(label), Some(crate::ir::NodeShape::Rectangle));
                    if let Some(icon_type) = icon {
                        if let Some(node) = graph.nodes.get_mut(&id) {
                            node.icon = Some(icon_type);
                        }
                    }
                    if let Some(parent_id) = parent
                        && let Some(idx) = groups.get(&parent_id).copied()
                        && let Some(subgraph) = graph.subgraphs.get_mut(idx)
                    {
                        subgraph.nodes.push(id.clone());
                    }
                }
            }
            continue;
        }
        if let Some((from, to, port_from, port_to, arrow_start, arrow_end)) =
            parse_architecture_edge(line)
        {
            graph.ensure_node(&from, None, Some(crate::ir::NodeShape::Rectangle));
            graph.ensure_node(&to, None, Some(crate::ir::NodeShape::Rectangle));
            graph.edges.push(crate::ir::Edge {
                from,
                to,
                label: None,
                start_label: None,
                end_label: None,
                directed: arrow_start || arrow_end,
                arrow_start,
                arrow_end,
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
                arch_port_from: port_from,
                arch_port_to: port_to,
            });
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_architecture_node(
    line: &str,
) -> Option<(String, String, String, Option<String>, Option<String>)> {
    let mut parts = line.splitn(2, ' ');
    let kind = parts.next()?.trim().to_ascii_lowercase();
    let rest = parts.next()?.trim();
    let (node_part, parent) = if let Some((left, right)) = rest.split_once(" in ") {
        (left.trim(), Some(right.trim().to_string()))
    } else {
        (rest, None)
    };
    let label = if let Some(start) = node_part.find('[') {
        if let Some(end) = node_part.rfind(']') {
            strip_quotes(node_part[start + 1..end].trim())
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    let id_part = node_part.split('[').next().unwrap_or(node_part).trim();
    let icon = if let Some(paren_start) = id_part.find('(') {
        if let Some(paren_end) = id_part.find(')') {
            Some(id_part[paren_start + 1..paren_end].trim().to_string())
        } else {
            None
        }
    } else {
        None
    };
    let id = id_part
        .split('(')
        .next()
        .unwrap_or(id_part)
        .trim()
        .to_string();
    if id.is_empty() {
        return None;
    }
    let label = if label.is_empty() { id.clone() } else { label };
    Some((kind, id, label, parent, icon))
}

fn parse_architecture_edge(
    line: &str,
) -> Option<(
    String,
    String,
    Option<crate::ir::ArchPort>,
    Option<crate::ir::ArchPort>,
    bool,
    bool,
)> {
    let arrows = [
        ("<-->", true, true),
        ("<--", true, false),
        ("-->", false, true),
        ("->", false, true),
        ("--", false, false),
    ];
    for (arrow, arrow_start, arrow_end) in arrows {
        if let Some(idx) = line.find(arrow) {
            let left = line[..idx].trim();
            let right = line[idx + arrow.len()..].trim();
            // Left side format: ID:Port (e.g., "gateway:R")
            let (from, port_from) = strip_arch_port_left(left);
            // Right side format: Port:ID (e.g., "L:app")
            let (to, port_to) = strip_arch_port_right(right);
            if from.is_empty() || to.is_empty() {
                return None;
            }
            return Some((
                from.to_string(),
                to.to_string(),
                port_from,
                port_to,
                arrow_start,
                arrow_end,
            ));
        }
    }
    None
}

fn parse_arch_port(s: &str) -> Option<crate::ir::ArchPort> {
    match s.trim() {
        "L" => Some(crate::ir::ArchPort::Left),
        "R" => Some(crate::ir::ArchPort::Right),
        "T" => Some(crate::ir::ArchPort::Top),
        "B" => Some(crate::ir::ArchPort::Bottom),
        _ => None,
    }
}

fn strip_arch_port_left(token: &str) -> (&str, Option<crate::ir::ArchPort>) {
    // "gateway:R" -> ("gateway", Some(Right))
    if let Some(idx) = token.rfind(':') {
        let id = token[..idx].trim();
        let port = parse_arch_port(&token[idx + 1..]);
        if port.is_some() {
            return (id, port);
        }
    }
    (token.trim(), None)
}

fn strip_arch_port_right(token: &str) -> (&str, Option<crate::ir::ArchPort>) {
    // "L:app" -> ("app", Some(Left))
    if let Some(idx) = token.find(':') {
        let port = parse_arch_port(&token[..idx]);
        let id = token[idx + 1..].trim();
        if port.is_some() {
            return (id, port);
        }
    }
    (token.trim(), None)
}

fn parse_radar_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Radar;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;
    let mut axes: Vec<(String, String)> = Vec::new();

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("radar") {
            continue;
        }
        if lower.starts_with("title") {
            let title = line.get(5..).unwrap_or("").trim();
            if !title.is_empty() {
                graph.diagram_title = Some(strip_quotes(title));
            }
            continue;
        }
        if lower.starts_with("axis") {
            let rest = line.get(4..).unwrap_or("").trim();
            axes = split_args(rest)
                .into_iter()
                .map(|value| parse_radar_name_label(&value))
                .filter(|(name, _)| !name.is_empty())
                .collect();
            continue;
        }
        if lower.starts_with("curve") {
            let rest = line.get(5..).unwrap_or("").trim();
            for curve_spec in split_radar_curve_specs(rest) {
                if let Some((name, values)) = parse_radar_curve_spec(&curve_spec, &axes) {
                    let node_id = format!("radar_{}", graph.nodes.len());
                    let mut label_lines = Vec::new();
                    label_lines.push(name);
                    for (axis, value) in values {
                        label_lines.push(format!("{}: {}", axis, value));
                    }
                    graph.ensure_node(
                        &node_id,
                        Some(label_lines.join("\n")),
                        Some(crate::ir::NodeShape::Circle),
                    );
                }
            }
            continue;
        }

        let mut words = line.split_whitespace();
        let keyword = words.next().unwrap_or("").to_ascii_lowercase();
        let value = words.next().unwrap_or("");
        match keyword.as_str() {
            "showlegend" => {
                graph.radar.show_legend = value.eq_ignore_ascii_case("true");
            }
            "ticks" => {
                if let Ok(ticks) = value.parse::<usize>() {
                    graph.radar.ticks = ticks.max(1);
                }
            }
            "max" => {
                if let Ok(max) = value.parse::<f32>() {
                    graph.radar.max = Some(max);
                }
            }
            "min" => {
                if let Ok(min) = value.parse::<f32>() {
                    graph.radar.min = min;
                }
            }
            "graticule" => {
                graph.radar.graticule = if value.eq_ignore_ascii_case("polygon") {
                    crate::ir::RadarGraticule::Polygon
                } else {
                    crate::ir::RadarGraticule::Circle
                };
            }
            _ => {}
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_radar_name_label(token: &str) -> (String, String) {
    let trimmed = token.trim();
    if let Some(open) = trimmed.find('[')
        && trimmed.ends_with(']')
        && open < trimmed.len() - 1
    {
        let name = strip_quotes(trimmed[..open].trim());
        let label = strip_quotes(trimmed[open + 1..trimmed.len() - 1].trim());
        let label = if label.is_empty() {
            name.clone()
        } else {
            label
        };
        return (name, label);
    }
    let name = strip_quotes(trimmed);
    (name.clone(), name)
}

fn split_radar_curve_specs(input: &str) -> Vec<String> {
    let mut specs = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut brace_depth = 0usize;
    for ch in input.chars() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            current.push(ch);
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            current.push(ch);
            continue;
        }
        match ch {
            '{' => {
                brace_depth += 1;
                current.push(ch);
            }
            '}' => {
                brace_depth = brace_depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if brace_depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    specs.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        specs.push(trimmed.to_string());
    }
    specs
}

fn parse_radar_curve_spec(
    spec: &str,
    axes: &[(String, String)],
) -> Option<(String, Vec<(String, String)>)> {
    let (name_part, values_part) = spec.split_once('{')?;
    let (_, name) = parse_radar_name_label(name_part.trim());
    let values_raw = values_part.split_once('}')?.0;
    if name.is_empty() {
        return None;
    }
    let entries = split_args(values_raw)
        .into_iter()
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>();
    let keyed_entries = entries.iter().any(|entry| entry.contains(':'));
    let values = if keyed_entries {
        axes.iter()
            .filter_map(|(axis_name, axis_label)| {
                entries.iter().find_map(|entry| {
                    let (entry_axis, entry_value) = entry.split_once(':')?;
                    let entry_axis = strip_quotes(entry_axis.trim());
                    if entry_axis == *axis_name || entry_axis == *axis_label {
                        let value = entry_value.trim();
                        (!value.is_empty()).then(|| (axis_label.clone(), value.to_string()))
                    } else {
                        None
                    }
                })
            })
            .collect()
    } else {
        entries
            .iter()
            .enumerate()
            .filter_map(|(idx, value)| {
                if value.is_empty() {
                    return None;
                }
                let axis = axes
                    .get(idx)
                    .map(|(_, label)| label.clone())
                    .unwrap_or_else(|| format!("axis{}", idx + 1));
                Some((axis, value.to_string()))
            })
            .collect()
    };
    Some((name, values))
}

fn parse_treemap_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Treemap;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input_keep_indent(input)?;
    let mut stack: Vec<String> = Vec::new();
    let mut base_indent: Option<usize> = None;
    let mut indent_unit: Option<usize> = None;

    for raw_line in lines {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("treemap") {
            continue;
        }
        if trimmed.starts_with("classDef") {
            parse_class_def(trimmed, &mut graph);
            continue;
        }
        if trimmed.starts_with("class ") {
            parse_class_line(trimmed, &mut graph);
            continue;
        }
        if trimmed.starts_with("style ") {
            parse_style_line(trimmed, &mut graph);
            continue;
        }
        let indent = count_indent(&raw_line);
        let base = *base_indent.get_or_insert(indent);
        let rel_indent = indent.saturating_sub(base);
        // Auto-detect indentation unit from the first indented line
        if rel_indent > 0 && indent_unit.is_none() {
            indent_unit = Some(rel_indent);
        }
        let unit = indent_unit.unwrap_or(2);
        let mut level = rel_indent / unit;
        if level > stack.len() {
            level = stack.len();
        }

        let (label, value, classes) = parse_treemap_item(trimmed);
        let numeric_value = value
            .as_ref()
            .and_then(|raw| raw.trim().parse::<f32>().ok());
        let node_id = format!("treemap_{}", graph.nodes.len());
        graph.ensure_node(
            &node_id,
            Some(label.clone()),
            Some(crate::ir::NodeShape::Rectangle),
        );
        if let Some(parsed) = numeric_value
            && let Some(node) = graph.nodes.get_mut(&node_id)
        {
            node.value = Some(parsed);
        }
        apply_node_classes(&mut graph, &node_id, &classes);

        if level > 0 {
            if stack.len() > level {
                stack.truncate(level);
            }
            if let Some(parent) = stack.last().cloned() {
                graph.edges.push(crate::ir::Edge {
                    from: parent,
                    to: node_id.clone(),
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
                    style: crate::ir::EdgeStyle::Solid,
                    markdown_label: false,
                    id: None,
                    curve: None,
                    arch_port_from: None,
                    arch_port_to: None,
                });
            }
        } else {
            stack.clear();
        }
        stack.push(node_id);
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_treemap_item(line: &str) -> (String, Option<String>, Vec<String>) {
    let (line, classes) = split_inline_classes(line.trim());
    if let Some((left, right)) = line.split_once(':') {
        let label = strip_quotes(left.trim());
        let value = right.trim();
        let value = if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        };
        return (
            if label.is_empty() {
                left.trim().to_string()
            } else {
                label
            },
            value,
            classes,
        );
    }
    (strip_quotes(line.trim()), None, classes)
}

fn parse_xy_chart_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::XYChart;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("xychart") {
            continue;
        }
        if lower.starts_with("title") {
            let rest = line.get(5..).unwrap_or("").trim();
            if !rest.is_empty() {
                graph.xychart.title = Some(strip_quotes(rest));
            }
            continue;
        }
        if lower.starts_with("x-axis") {
            let rest = line.get(6..).unwrap_or("").trim();
            // Check if it's a label followed by categories or just categories
            if let Some(bracket_idx) = rest.find('[') {
                let label_part = rest[..bracket_idx].trim();
                if !label_part.is_empty() {
                    graph.xychart.x_axis_label = Some(strip_quotes(label_part));
                }
                graph.xychart.x_axis_categories = parse_xy_axis_categories(&rest[bracket_idx..]);
            } else {
                // Just categories without brackets or a label
                graph.xychart.x_axis_categories = parse_xy_axis_categories(rest);
            }
            continue;
        }
        if lower.starts_with("y-axis") {
            let rest = line.get(6..).unwrap_or("").trim();
            if !rest.is_empty() {
                // Parse y-axis which can have label and/or range
                // Format: y-axis "Label" min --> max  OR  y-axis min --> max  OR  y-axis "Label"
                let rest_lower = rest.to_ascii_lowercase();
                if let Some(arrow_idx) = rest_lower.find("-->") {
                    // Has range
                    let before_arrow = rest[..arrow_idx].trim();
                    let after_arrow = rest[arrow_idx + 3..].trim();

                    // Parse min value (might have label before it)
                    let min_str = before_arrow.split_whitespace().last().unwrap_or("0");
                    if let Ok(min) = min_str.parse::<f32>() {
                        graph.xychart.y_axis_min = Some(min);
                    }
                    if let Ok(max) = after_arrow.parse::<f32>() {
                        graph.xychart.y_axis_max = Some(max);
                    }
                    // Check for label before the min value
                    let label_part = before_arrow.trim_end_matches(min_str).trim();
                    if !label_part.is_empty() {
                        graph.xychart.y_axis_label = Some(strip_quotes(label_part));
                    }
                } else {
                    graph.xychart.y_axis_label = Some(strip_quotes(rest));
                }
            }
            continue;
        }
        if let Some((series_kind, label, values)) = parse_xy_series_line_v2(line) {
            graph.xychart.series.push(crate::ir::XYSeries {
                kind: series_kind,
                label,
                values,
            });
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_xy_series_line_v2(
    line: &str,
) -> Option<(crate::ir::XYSeriesKind, Option<String>, Vec<f32>)> {
    let lower = line.to_ascii_lowercase();
    let (kind, rest) = if lower.starts_with("bar") {
        (
            crate::ir::XYSeriesKind::Bar,
            line.get(3..).unwrap_or("").trim(),
        )
    } else if lower.starts_with("line") {
        (
            crate::ir::XYSeriesKind::Line,
            line.get(4..).unwrap_or("").trim(),
        )
    } else {
        return None;
    };

    // Parse optional label and values: [1, 2, 3] or "Label" [1, 2, 3]
    let (label, values_str) = if let Some(bracket_idx) = rest.find('[') {
        let label_part = rest[..bracket_idx].trim();
        let label = if label_part.is_empty() {
            None
        } else {
            Some(strip_quotes(label_part))
        };
        (label, &rest[bracket_idx..])
    } else {
        (None, rest)
    };

    let values: Vec<f32> = values_str
        .trim_matches(|ch| ch == '[' || ch == ']')
        .split(',')
        .filter_map(|s| s.trim().parse::<f32>().ok())
        .collect();

    if values.is_empty() {
        None
    } else {
        Some((kind, label, values))
    }
}

fn parse_xy_axis_categories(rest: &str) -> Vec<String> {
    let trimmed = rest.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let categories = if let Some(open) = trimmed.find('[') {
        if let Some(close) = trimmed.rfind(']') {
            if close > open {
                &trimmed[open + 1..close]
            } else {
                trimmed
            }
        } else {
            trimmed
        }
    } else {
        trimmed
    };

    split_args(categories)
        .into_iter()
        .map(|value| {
            let cleaned = value
                .trim()
                .trim_matches(|ch| ch == '[' || ch == ']')
                .trim();
            strip_quotes(cleaned)
        })
        .filter(|value| !value.is_empty())
        .collect()
}

#[allow(dead_code)]
fn parse_xy_series_line(line: &str) -> Option<(String, Vec<String>)> {
    let mut parts = line.splitn(2, ' ');
    let series = parts.next()?.trim().to_string();
    let rest = parts.next()?.trim();
    let values = rest
        .trim_matches(|ch| ch == '[' || ch == ']')
        .split(',')
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if series.is_empty() {
        None
    } else {
        Some((series, values))
    }
}

fn parse_state_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::State;
    let (lines, init_config) = preprocess_input(input)?;

    let mut labels: HashMap<String, String> = HashMap::new();
    let mut start_states: HashMap<String, String> = HashMap::new();
    let mut end_states: HashMap<String, String> = HashMap::new();
    let mut subgraph_stack: Vec<usize> = Vec::new();
    let mut region_counter: usize = 0;

    #[derive(Debug)]
    struct CompositeContext {
        subgraph_idx: usize,
        regions: Vec<Vec<String>>,
        current_region: usize,
        has_separator: bool,
    }

    let mut composite_stack: Vec<CompositeContext> = Vec::new();
    let mut pending: VecDeque<String> = lines.into();

    let record_region_node = |stack: &mut [CompositeContext], node_id: &str| {
        for ctx in stack.iter_mut() {
            if ctx
                .regions
                .iter()
                .any(|region| region.iter().any(|id| id == node_id))
            {
                continue;
            }
            let region = &mut ctx.regions[ctx.current_region];
            region.push(node_id.to_string());
        }
    };

    let finalize_regions =
        |ctx: CompositeContext, graph: &mut Graph, region_counter: &mut usize| {
            if !ctx.has_separator {
                return;
            }
            let mut regions: Vec<Vec<String>> = ctx
                .regions
                .into_iter()
                .filter(|region| !region.is_empty())
                .collect();
            if regions.len() <= 1 {
                return;
            }
            for region_nodes in regions.drain(..) {
                let id = format!("__region_{}__", *region_counter);
                *region_counter += 1;
                graph.subgraphs.push(Subgraph {
                    id: Some(id.clone()),
                    label: String::new(),
                    nodes: region_nodes,
                    direction: None,
                    icon: None,
                    markdown_label: false,
                });
                // Concurrent regions render with a faint dashed border to
                // visually separate them — JS draws an alt-shaded rect with a
                // dashed divider; we use a single light-gray dashed stroke
                // around each region.
                graph.subgraph_styles.insert(
                    id,
                    NodeStyle {
                        fill: Some("none".to_string()),
                        stroke: Some("#9370DB".to_string()),
                        text_color: None,
                        stroke_width: Some(1.0),
                        stroke_dasharray: Some("10 10".to_string()),
                        line_color: None,
                        font_style: None,
                        font_weight: None,
                    },
                );
            }
        };
    while let Some(raw_line) = pending.pop_front() {
        for raw_statement in split_statements(&raw_line) {
            let raw_line = raw_statement.trim();
            if raw_line.is_empty() {
                continue;
            }
            let (line, state_shape, label_override) = parse_state_stereotype(raw_line);
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("statediagram") {
                continue;
            }

            if let Some(direction) = parse_direction_line(line) {
                graph.direction = direction;
                continue;
            }

            if line.starts_with("classDef") {
                parse_class_def(line, &mut graph);
                continue;
            }

            if line.starts_with("class ") {
                parse_class_line(line, &mut graph);
                continue;
            }

            if line.starts_with("style ") {
                parse_style_line(line, &mut graph);
                continue;
            }

            if line == "}" {
                if let Some(ctx) = composite_stack.pop() {
                    if let Some(idx) = subgraph_stack.pop()
                        && idx != ctx.subgraph_idx
                    {
                        subgraph_stack.push(idx);
                    }
                    finalize_regions(ctx, &mut graph, &mut region_counter);
                }
                continue;
            }

            if line == "--" {
                if let Some(ctx) = composite_stack.last_mut() {
                    ctx.has_separator = true;
                    ctx.regions.push(Vec::new());
                    ctx.current_region = ctx.regions.len().saturating_sub(1);
                }
                continue;
            }

            if let Some((id, label, tail)) = parse_state_container_header(line) {
                if let Some(id) = id.clone() {
                    labels.insert(id.clone(), label.clone());
                }
                graph.subgraphs.push(Subgraph {
                    id: id.clone(),
                    label: label.clone(),
                    nodes: Vec::new(),
                    direction: None,
                    icon: None,
                    markdown_label: false,
                });
                subgraph_stack.push(graph.subgraphs.len() - 1);
                composite_stack.push(CompositeContext {
                    subgraph_idx: graph.subgraphs.len() - 1,
                    regions: vec![Vec::new()],
                    current_region: 0,
                    has_separator: false,
                });

                if !tail.is_empty() {
                    if let Some(close_idx) = tail.find('}') {
                        let body = tail[..close_idx].trim();
                        let after = tail[close_idx + 1..].trim();
                        if !after.is_empty() {
                            pending.push_front(after.to_string());
                        }
                        pending.push_front("}".to_string());
                        if !body.is_empty() {
                            pending.push_front(body.to_string());
                        }
                    } else {
                        pending.push_front(tail);
                    }
                }
                continue;
            }

            if let Some((id, label, classes)) = parse_state_alias_line(line) {
                let label = label_override.clone().unwrap_or(label);
                labels.insert(id.clone(), label);
                graph.ensure_node(
                    &id,
                    labels.get(&id).cloned(),
                    state_shape.or(Some(crate::ir::NodeShape::RoundRect)),
                );
                apply_node_classes(&mut graph, &id, &classes);
                add_node_to_state_subgraphs(&mut graph, &subgraph_stack, &id);
                record_region_node(&mut composite_stack, &id);
                continue;
            }

            if let Some((left, meta, right, label)) = parse_state_transition(line) {
                // Determine current scope for start/end state tracking.
                // When inside a composite state with concurrent regions (`--`),
                // each region needs its own [*] start/end node — otherwise all
                // regions share one start, which produces a single fan-out node
                // instead of independent per-region start dots.
                let base_scope = subgraph_stack
                    .last()
                    .and_then(|&idx| graph.subgraphs.get(idx))
                    .and_then(|sub| sub.id.clone())
                    .unwrap_or_else(|| "root".to_string());
                let scope = match composite_stack.last() {
                    Some(ctx) if ctx.has_separator => {
                        format!("{}__region_{}", base_scope, ctx.current_region)
                    }
                    _ => base_scope,
                };
                let (left_token, left_classes) = split_inline_classes(&left);
                let (right_token, right_classes) = split_inline_classes(&right);
                let (left_id, left_shape, left_label_override) = normalize_state_token(
                    &left_token,
                    true,
                    &mut start_states,
                    &mut end_states,
                    &scope,
                );
                let (right_id, right_shape, right_label_override) = normalize_state_token(
                    &right_token,
                    false,
                    &mut start_states,
                    &mut end_states,
                    &scope,
                );

                let left_label = left_label_override.or_else(|| labels.get(&left_id).cloned());
                let right_label = right_label_override.or_else(|| labels.get(&right_id).cloned());
                let left_shape = if left_shape == crate::ir::NodeShape::RoundRect
                    && graph.nodes.contains_key(&left_id)
                {
                    None
                } else {
                    Some(left_shape)
                };
                let right_shape = if right_shape == crate::ir::NodeShape::RoundRect
                    && graph.nodes.contains_key(&right_id)
                {
                    None
                } else {
                    Some(right_shape)
                };
                graph.ensure_node(&left_id, left_label, left_shape);
                graph.ensure_node(&right_id, right_label, right_shape);
                apply_node_classes(&mut graph, &left_id, &left_classes);
                apply_node_classes(&mut graph, &right_id, &right_classes);
                add_node_to_state_subgraphs(&mut graph, &subgraph_stack, &left_id);
                add_node_to_state_subgraphs(&mut graph, &subgraph_stack, &right_id);
                record_region_node(&mut composite_stack, &left_id);
                record_region_node(&mut composite_stack, &right_id);
                graph.edges.push(crate::ir::Edge {
                    from: left_id,
                    to: right_id,
                    label,
                    start_label: None,
                    end_label: None,
                    directed: meta.directed,
                    arrow_start: meta.arrow_start,
                    arrow_end: meta.arrow_end,
                    arrow_start_kind: meta.arrow_start_kind,
                    arrow_end_kind: meta.arrow_end_kind,
                    start_decoration: meta.start_decoration,
                    end_decoration: meta.end_decoration,
                    sequence_arrow_end: None,
                    sequence_arrow_start: None,
                    style: meta.style,
                    markdown_label: false,
                    id: None,
                    curve: None,
                    arch_port_from: None,
                    arch_port_to: None,
                });
                continue;
            }

            if let Some((id, label, classes)) = parse_state_description_line(line) {
                let label = label_override.clone().unwrap_or(label);
                labels.insert(id.clone(), label);
                graph.ensure_node(
                    &id,
                    labels.get(&id).cloned(),
                    state_shape.or(Some(crate::ir::NodeShape::RoundRect)),
                );
                apply_node_classes(&mut graph, &id, &classes);
                add_node_to_state_subgraphs(&mut graph, &subgraph_stack, &id);
                record_region_node(&mut composite_stack, &id);
                continue;
            }

            if let Some((position, target_raw)) = parse_state_note_block_header(line) {
                let (target, classes) = parse_state_id_with_classes(&target_raw);
                if !target.is_empty() {
                    let shape = if graph.nodes.contains_key(&target) {
                        None
                    } else {
                        Some(crate::ir::NodeShape::RoundRect)
                    };
                    graph.ensure_node(&target, labels.get(&target).cloned(), shape);
                    apply_node_classes(&mut graph, &target, &classes);
                    add_node_to_state_subgraphs(&mut graph, &subgraph_stack, &target);
                    record_region_node(&mut composite_stack, &target);
                }
                let mut body_lines: Vec<String> = Vec::new();
                while let Some(next_raw) = pending.pop_front() {
                    let mut consumed = false;
                    for next_stmt in split_statements(&next_raw) {
                        let next_trim = next_stmt.trim();
                        if next_trim.is_empty() {
                            continue;
                        }
                        if next_trim.eq_ignore_ascii_case("end note") {
                            consumed = true;
                            break;
                        }
                        body_lines.push(next_trim.to_string());
                    }
                    if consumed {
                        break;
                    }
                }
                let label = body_lines.join("\n");
                if !target.is_empty() && !label.is_empty() {
                    graph.state_notes.push(crate::ir::StateNote {
                        position,
                        target,
                        label,
                    });
                }
                continue;
            }

            if let Some((position, target_raw, label)) = parse_state_note(line) {
                let (target, classes) = parse_state_id_with_classes(&target_raw);
                if target.is_empty() {
                    continue;
                }
                let shape = if graph.nodes.contains_key(&target) {
                    None
                } else {
                    Some(crate::ir::NodeShape::RoundRect)
                };
                graph.ensure_node(&target, labels.get(&target).cloned(), shape);
                apply_node_classes(&mut graph, &target, &classes);
                graph.state_notes.push(crate::ir::StateNote {
                    position,
                    target: target.clone(),
                    label,
                });
                add_node_to_state_subgraphs(&mut graph, &subgraph_stack, &target);
                record_region_node(&mut composite_stack, &target);
                continue;
            }

            if let Some((id, classes)) = parse_state_simple(line) {
                if let Some(label) = label_override.clone() {
                    labels.insert(id.clone(), label);
                }
                graph.ensure_node(
                    &id,
                    labels.get(&id).cloned(),
                    state_shape.or(Some(crate::ir::NodeShape::RoundRect)),
                );
                apply_node_classes(&mut graph, &id, &classes);
                add_node_to_state_subgraphs(&mut graph, &subgraph_stack, &id);
                record_region_node(&mut composite_stack, &id);
                continue;
            }

            // Fallback: a bare identifier line (e.g. just `s1` without any
            // declaration keyword or transition) should still create a state
            // node. Without this, orphan states render as an empty diagram.
            // Only accept lines that look like a single valid identifier
            // (alphanumeric + underscore + optional inline classes).
            let (bare_id, bare_classes) = parse_state_id_with_classes(line);
            if !bare_id.is_empty()
                && bare_id
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                graph.ensure_node(
                    &bare_id,
                    labels.get(&bare_id).cloned(),
                    state_shape.or(Some(crate::ir::NodeShape::RoundRect)),
                );
                apply_node_classes(&mut graph, &bare_id, &bare_classes);
                add_node_to_state_subgraphs(&mut graph, &subgraph_stack, &bare_id);
                record_region_node(&mut composite_stack, &bare_id);
                continue;
            }
        }
    }

    // Convert scoped [*] fan-out/fan-in nodes into fork/join bars.
    let mut outgoing_counts: HashMap<&str, usize> = HashMap::new();
    let mut incoming_counts: HashMap<&str, usize> = HashMap::new();
    for edge in &graph.edges {
        *outgoing_counts.entry(edge.from.as_str()).or_insert(0) += 1;
        *incoming_counts.entry(edge.to.as_str()).or_insert(0) += 1;
    }
    let fork_ids: Vec<String> = start_states
        .iter()
        .filter_map(|(scope, id)| {
            if scope == "root" {
                return None;
            }
            if outgoing_counts.get(id.as_str()).copied().unwrap_or(0) > 1 {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect();
    let join_ids: Vec<String> = end_states
        .iter()
        .filter_map(|(scope, id)| {
            if scope == "root" {
                return None;
            }
            if incoming_counts.get(id.as_str()).copied().unwrap_or(0) > 1 {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect();
    for id in fork_ids.into_iter().chain(join_ids.into_iter()) {
        if let Some(node) = graph.nodes.get_mut(&id) {
            node.shape = crate::ir::NodeShape::ForkJoin;
            node.label.clear();
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_sequence_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Sequence;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input(input)?;

    let mut labels: HashMap<String, String> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut open_frames: Vec<crate::ir::SequenceFrame> = Vec::new();
    let mut frames: Vec<crate::ir::SequenceFrame> = Vec::new();
    let mut open_boxes: Vec<crate::ir::SequenceBox> = Vec::new();

    for raw_line in lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("sequencediagram") {
            continue;
        }
        // Handle `create participant`/`create actor` and `destroy` keywords.
        if lower.starts_with("create participant ") || lower.starts_with("create actor ") {
            let is_actor = lower.starts_with("create actor");
            let offset = if is_actor { 13 } else { 19 };
            let rest = line[offset..].trim();
            let prefix = if is_actor { "actor" } else { "participant" };
            let synthetic = format!("{prefix} {rest}");
            if let Some((id, label, shape)) = parse_sequence_participant(&synthetic) {
                if !order.contains(&id) {
                    order.push(id.clone());
                }
                if let Some(label) = label.clone() {
                    labels.insert(id.clone(), label);
                }
                ensure_sequence_node(&mut graph, &labels, &id, Some(shape));
                if let Some(box_ctx) = open_boxes.last_mut()
                    && !box_ctx.participants.contains(&id)
                {
                    box_ctx.participants.push(id.clone());
                }
                // The actor becomes visible at the next message (the one
                // that follows the `create` statement).
                graph.sequence_lifecycle.push(crate::ir::SequenceLifecycle {
                    participant: id,
                    index: graph.edges.len(),
                    kind: crate::ir::SequenceLifecycleKind::Create,
                });
            }
            continue;
        }
        if lower.starts_with("destroy ") {
            let id = strip_quotes(line[8..].trim());
            if !id.is_empty() {
                if !order.contains(&id) {
                    order.push(id.clone());
                }
                ensure_sequence_node(&mut graph, &labels, &id, None);
                // The actor's lifeline ends at the next message (the one
                // that follows the `destroy` statement).
                graph.sequence_lifecycle.push(crate::ir::SequenceLifecycle {
                    participant: id.to_string(),
                    index: graph.edges.len(),
                    kind: crate::ir::SequenceLifecycleKind::Destroy,
                });
            }
            continue;
        }
        if let Some((id, label, shape)) = parse_sequence_participant(line) {
            if !order.contains(&id) {
                order.push(id.clone());
            }
            if let Some(label) = label.clone() {
                labels.insert(id.clone(), label);
            }
            ensure_sequence_node(&mut graph, &labels, &id, Some(shape));
            if let Some(box_ctx) = open_boxes.last_mut()
                && !box_ctx.participants.contains(&id)
            {
                box_ctx.participants.push(id.clone());
            }
            continue;
        }

        if let Some((color, label)) = parse_sequence_box_line(line) {
            open_boxes.push(crate::ir::SequenceBox {
                label,
                color,
                participants: Vec::new(),
            });
            continue;
        }

        if lower == "alt"
            || lower.starts_with("alt ")
            || lower == "opt"
            || lower.starts_with("opt ")
            || lower == "loop"
            || lower.starts_with("loop ")
            || lower == "par"
            || lower.starts_with("par ")
            || lower == "rect"
            || lower.starts_with("rect ")
            || lower == "critical"
            || lower.starts_with("critical ")
            || lower == "break"
            || lower.starts_with("break ")
        {
            let (kind, offset) = if lower.starts_with("opt") {
                (crate::ir::SequenceFrameKind::Opt, 3)
            } else if lower.starts_with("loop") {
                (crate::ir::SequenceFrameKind::Loop, 4)
            } else if lower.starts_with("par") {
                (crate::ir::SequenceFrameKind::Par, 3)
            } else if lower.starts_with("rect") {
                (crate::ir::SequenceFrameKind::Rect, 4)
            } else if lower.starts_with("critical") {
                (crate::ir::SequenceFrameKind::Critical, 8)
            } else if lower.starts_with("break") {
                (crate::ir::SequenceFrameKind::Break, 5)
            } else {
                (crate::ir::SequenceFrameKind::Alt, 3)
            };
            let label = line.get(offset..).map(str::trim).unwrap_or_default();
            let label = if label.is_empty() {
                None
            } else {
                Some(strip_quotes(label))
            };
            let start_idx = graph.edges.len();
            open_frames.push(crate::ir::SequenceFrame {
                kind,
                sections: vec![crate::ir::SequenceFrameSection {
                    label,
                    start_idx,
                    end_idx: start_idx,
                }],
                start_idx,
                end_idx: start_idx,
            });
            continue;
        }

        if lower == "else" || lower.starts_with("else ") {
            if let Some(frame) = open_frames.last_mut() {
                let split_idx = graph.edges.len();
                if let Some(last) = frame.sections.last_mut() {
                    last.end_idx = split_idx;
                }
                let label = line.get(4..).map(str::trim).unwrap_or_default();
                let label = if label.is_empty() {
                    None
                } else {
                    Some(strip_quotes(label))
                };
                frame.sections.push(crate::ir::SequenceFrameSection {
                    label,
                    start_idx: split_idx,
                    end_idx: split_idx,
                });
            }
            continue;
        }

        if lower == "and" || lower.starts_with("and ") {
            if let Some(frame) = open_frames.last_mut()
                && frame.kind == crate::ir::SequenceFrameKind::Par
            {
                let split_idx = graph.edges.len();
                if let Some(last) = frame.sections.last_mut() {
                    last.end_idx = split_idx;
                }
                let label = line.get(3..).map(str::trim).unwrap_or_default();
                let label = if label.is_empty() {
                    None
                } else {
                    Some(strip_quotes(label))
                };
                frame.sections.push(crate::ir::SequenceFrameSection {
                    label,
                    start_idx: split_idx,
                    end_idx: split_idx,
                });
            }
            continue;
        }

        if lower == "option" || lower.starts_with("option ") {
            if let Some(frame) = open_frames.last_mut()
                && frame.kind == crate::ir::SequenceFrameKind::Critical
            {
                let split_idx = graph.edges.len();
                if let Some(last) = frame.sections.last_mut() {
                    last.end_idx = split_idx;
                }
                let label = line.get(6..).map(str::trim).unwrap_or_default();
                let label = if label.is_empty() {
                    None
                } else {
                    Some(strip_quotes(label))
                };
                frame.sections.push(crate::ir::SequenceFrameSection {
                    label,
                    start_idx: split_idx,
                    end_idx: split_idx,
                });
            }
            continue;
        }

        if lower == "end" {
            if let Some(mut frame) = open_frames.pop() {
                let end_idx = graph.edges.len();
                if let Some(last) = frame.sections.last_mut() {
                    last.end_idx = end_idx;
                }
                frame.end_idx = end_idx;
                frames.push(frame);
            } else if let Some(seq_box) = open_boxes.pop() {
                graph.sequence_boxes.push(seq_box);
            }
            continue;
        }

        if let Some((position, participants, label)) = parse_sequence_note(line) {
            for id in &participants {
                if !order.contains(id) {
                    order.push(id.clone());
                }
                ensure_sequence_node(&mut graph, &labels, id, None);
            }
            graph.sequence_notes.push(crate::ir::SequenceNote {
                position,
                participants,
                label,
                index: graph.edges.len(),
            });
            continue;
        }

        if lower.starts_with("activate ") {
            let id = line[9..].trim();
            if !id.is_empty() {
                let id = strip_quotes(id);
                if !order.contains(&id) {
                    order.push(id.clone());
                }
                ensure_sequence_node(&mut graph, &labels, &id, None);
                graph
                    .sequence_activations
                    .push(crate::ir::SequenceActivation {
                        participant: id,
                        // Standalone `activate X` ties the activation start
                        // to the most recent message (matches mermaid.js
                        // behavior). Pre-message activations clamp to 0.
                        index: graph.edges.len().saturating_sub(1),
                        kind: crate::ir::SequenceActivationKind::Activate,
                    });
            }
            continue;
        }
        if lower.starts_with("deactivate ") {
            let id = line[11..].trim();
            if !id.is_empty() {
                let id = strip_quotes(id);
                if !order.contains(&id) {
                    order.push(id.clone());
                }
                ensure_sequence_node(&mut graph, &labels, &id, None);
                graph
                    .sequence_activations
                    .push(crate::ir::SequenceActivation {
                        participant: id,
                        // Standalone `deactivate X` ties the activation end
                        // to the most recent message.
                        index: graph.edges.len().saturating_sub(1),
                        kind: crate::ir::SequenceActivationKind::Deactivate,
                    });
            }
            continue;
        }
        if lower.starts_with("autonumber") {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.len() >= 2 {
                let token = parts[1].to_ascii_lowercase();
                if token == "off" || token == "stop" || token == "disable" {
                    graph.sequence_autonumber = None;
                } else if let Ok(start) = parts[1].parse::<usize>() {
                    graph.sequence_autonumber = Some(start);
                } else {
                    graph.sequence_autonumber = Some(1);
                }
            } else {
                graph.sequence_autonumber = Some(1);
            }
            continue;
        }

        if let Some((
            from,
            to,
            label,
            style,
            activation,
            arrow_head,
            start_arrow,
            start_decoration,
            end_decoration,
        )) = parse_sequence_message(line)
        {
            if !order.contains(&from) {
                order.push(from.clone());
            }
            if !order.contains(&to) {
                order.push(to.clone());
            }
            ensure_sequence_node(&mut graph, &labels, &from, None);
            ensure_sequence_node(&mut graph, &labels, &to, None);
            let has_arrow = arrow_head != crate::ir::SequenceArrowHead::None;
            let has_start_arrow = start_arrow.is_some();
            graph.edges.push(crate::ir::Edge {
                from,
                to,
                label,
                start_label: None,
                end_label: None,
                directed: has_arrow || has_start_arrow,
                arrow_start: has_start_arrow,
                arrow_end: has_arrow,
                arrow_start_kind: None,
                arrow_end_kind: None,
                start_decoration,
                end_decoration,
                sequence_arrow_end: Some(arrow_head),
                sequence_arrow_start: start_arrow,
                style,
                markdown_label: false,
                id: None,
                curve: None,
                arch_port_from: None,
                arch_port_to: None,
            });
            if let Some(kind) = activation
                && let Some(last) = graph.edges.len().checked_sub(1)
            {
                // Upstream mermaid grammar (sequenceDiagram.jison):
                //   `actor signaltype '+' actor text` → activate destination
                //   `actor signaltype '-' actor text` → deactivate SOURCE
                // The previous code always used `to`, which broke stacked
                // activations like `John-->>-Alice` (which deactivates John,
                // not Alice).
                let participant = match kind {
                    crate::ir::SequenceActivationKind::Activate => graph.edges[last].to.clone(),
                    crate::ir::SequenceActivationKind::Deactivate => graph.edges[last].from.clone(),
                };
                graph
                    .sequence_activations
                    .push(crate::ir::SequenceActivation {
                        participant,
                        index: last,
                        kind,
                    });
            }
        }
    }

    while let Some(mut frame) = open_frames.pop() {
        let end_idx = graph.edges.len();
        if let Some(last) = frame.sections.last_mut() {
            last.end_idx = end_idx;
        }
        frame.end_idx = end_idx;
        frames.push(frame);
    }
    while let Some(seq_box) = open_boxes.pop() {
        graph.sequence_boxes.push(seq_box);
    }

    graph.sequence_participants = order;
    graph.sequence_frames = frames;
    Ok(ParseOutput { graph, init_config })
}

fn add_node_to_subgraph(graph: &mut Graph, idx: usize, node_id: &str) {
    if let Some(subgraph) = graph.subgraphs.get_mut(idx)
        && !subgraph.nodes.contains(&node_id.to_string())
    {
        subgraph.nodes.push(node_id.to_string());
    }
}

fn add_node_to_subgraphs(graph: &mut Graph, subgraph_stack: &[usize], node_id: &str) {
    for idx in subgraph_stack {
        add_node_to_subgraph(graph, *idx, node_id);
    }
}

/// State-diagram variant of `add_node_to_subgraphs` that mirrors mermaid-cli's
/// "last reference wins" parentId behavior. When a state name is referenced
/// inside a composite scope, JS's `insertOrUpdateNode` overwrites the node's
/// `parentId` via `Object.assign`. So `second` declared in `state Second {...}`
/// then re-referenced in `state End { [*] --> second }` ends up parented to End,
/// not Second. We model this by REMOVING the node from any subgraph not in the
/// current stack before adding it to the current stack — so the node is
/// visually nested under its most recent scope.
fn add_node_to_state_subgraphs(graph: &mut Graph, subgraph_stack: &[usize], node_id: &str) {
    use std::collections::HashSet;
    let stack_set: HashSet<usize> = subgraph_stack.iter().copied().collect();
    for (idx, sub) in graph.subgraphs.iter_mut().enumerate() {
        if !stack_set.contains(&idx) {
            sub.nodes.retain(|n| n != node_id);
        }
    }
    for idx in subgraph_stack {
        add_node_to_subgraph(graph, *idx, node_id);
    }
}

fn split_statements(line: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in line.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' {
            current.push(ch);
            escaped = true;
            continue;
        }

        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            current.push(ch);
            continue;
        }

        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            current.push(ch);
            continue;
        }

        match ch {
            '[' | '(' | '{' => {
                depth += 1;
                current.push(ch);
            }
            ']' | ')' | '}' => {
                if depth > 0 {
                    depth -= 1;
                }
                current.push(ch);
            }
            ';' if depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        parts.push(trimmed.to_string());
    }
    parts
}

fn strip_trailing_comment(line: &str) -> String {
    let mut quote: Option<char> = None;
    let mut chars = line.chars().peekable();
    let mut out = String::new();
    while let Some(ch) = chars.next() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            out.push(ch);
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            out.push(ch);
            continue;
        }
        if ch == '%'
            && let Some('%') = chars.peek().copied()
        {
            break;
        }
        out.push(ch);
    }
    out.trim().to_string()
}

fn strip_trailing_comment_keep_indent(line: &str) -> String {
    let mut quote: Option<char> = None;
    let mut chars = line.chars().peekable();
    let mut out = String::new();
    while let Some(ch) = chars.next() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            out.push(ch);
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            out.push(ch);
            continue;
        }
        if ch == '%'
            && let Some('%') = chars.peek().copied()
        {
            break;
        }
        out.push(ch);
    }
    out.trim_end().to_string()
}

fn extract_leading_decoration(right: &str) -> Option<(char, String)> {
    let mut chars = right.chars();
    let first = chars.next()?;
    if first != 'o' && first != 'x' {
        return None;
    }
    let rest: String = chars.collect();
    if rest.is_empty() {
        return None;
    }
    if rest
        .chars()
        .next()
        .map(|c| c.is_whitespace())
        .unwrap_or(false)
    {
        return Some((first, rest.trim_start().to_string()));
    }
    None
}

fn parse_subgraph_header(input: &str) -> (Option<String>, String, Vec<String>, bool) {
    let (base, classes) = split_inline_classes(input);
    let trimmed = base.trim();
    if trimmed.is_empty() {
        return (None, "Subgraph".to_string(), classes, false);
    }

    if let Some((id, label, _shape, md)) = split_id_label(trimmed) {
        return (Some(id.to_string()), label, classes, md);
    }

    if !trimmed.contains(['"', '\'']) {
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() == 1 {
            let token = parts[0];
            return (Some(token.to_string()), token.to_string(), classes, false);
        }
    }

    let (label, md) = strip_quotes_markdown(trimmed);
    (None, label, classes, md)
}

fn parse_node_only(line: &str) -> Option<NodeTokenParts> {
    if mask_bracket_content(line).contains("--") {
        return None;
    }
    let (id, label, shape, classes, md) = parse_node_token(line);
    if id.is_empty() {
        None
    } else {
        Some((id, label, shape, classes, md))
    }
}

/// Mask content inside brackets to prevent edge detection from matching dashes in labels.
/// Returns a string where characters inside [...], (...), {...}, and "..." are replaced with spaces.
fn mask_bracket_content(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut depth_square = 0;
    let mut depth_paren = 0;
    let mut depth_curly = 0;
    let mut in_double_quote = false;
    let mut prev_char = '\0';

    for ch in line.chars() {
        let in_bracket = depth_square > 0 || depth_paren > 0 || depth_curly > 0;
        let in_quote = in_double_quote;

        match ch {
            '[' if !in_quote => {
                depth_square += 1;
                result.push(ch);
            }
            ']' if !in_quote && depth_square > 0 => {
                depth_square -= 1;
                result.push(ch);
            }
            '(' if !in_quote && !in_bracket => {
                depth_paren += 1;
                result.push(ch);
            }
            ')' if !in_quote && depth_paren > 0 => {
                depth_paren -= 1;
                result.push(ch);
            }
            '{' if !in_quote && !in_bracket => {
                depth_curly += 1;
                result.push(ch);
            }
            '}' if !in_quote && depth_curly > 0 => {
                depth_curly -= 1;
                result.push(ch);
            }
            '"' if prev_char != '\\' => {
                in_double_quote = !in_double_quote;
                if in_bracket || in_quote {
                    // Preserve byte length by adding spaces equal to character's UTF-8 byte count
                    for _ in 0..ch.len_utf8() {
                        result.push(' ');
                    }
                } else {
                    result.push(ch);
                }
            }
            _ => {
                if in_bracket || in_quote {
                    // Preserve byte length by adding spaces equal to character's UTF-8 byte count
                    for _ in 0..ch.len_utf8() {
                        result.push(' ');
                    }
                } else {
                    result.push(ch);
                }
            }
        }
        prev_char = ch;
    }
    result
}

/// Split `input` on `&` that appear outside brackets, parentheses, braces, and quotes.
///
/// Uses [`mask_bracket_content`] to blank out quoted/bracketed content while
/// preserving byte positions, then splits on `&` positions found in the masked
/// string but slices from the original — so `A["foo & bar"]` is never split.
fn split_on_ampersand<'a>(input: &'a str) -> Vec<&'a str> {
    let masked = mask_bracket_content(input);
    let mut parts = Vec::new();
    let mut start = 0usize;
    for (i, ch) in masked.char_indices() {
        if ch == '&' {
            let part = input[start..i].trim();
            if !part.is_empty() {
                parts.push(part);
            }
            start = i + ch.len_utf8();
        }
    }
    let last = input[start..].trim();
    if !last.is_empty() {
        parts.push(last);
    }
    parts
}

fn split_edge_chain(line: &str) -> Option<Vec<String>> {
    let masked = mask_bracket_content(line);
    if PIPE_LABEL_RE.is_match(&masked)
        || QUOTED_LABEL_ARROW_RE.is_match(line)
        || LABEL_ARROW_RE.is_match(&masked)
        || COMPACT_DOTTED_LABEL_ARROW_RE.is_match(&masked)
    {
        return None;
    }

    let matches: Vec<regex::Match> = ARROW_TOKEN_RE.find_iter(&masked).collect();
    if matches.len() < 2 {
        return None;
    }

    let mut nodes: Vec<String> = Vec::with_capacity(matches.len() + 1);
    let mut arrows: Vec<String> = Vec::with_capacity(matches.len());
    let mut last_idx = 0usize;

    for m in matches {
        nodes.push(line[last_idx..m.start()].trim().to_string());
        arrows.push(line[m.start()..m.end()].trim().to_string());
        last_idx = m.end();
    }
    nodes.push(line[last_idx..].trim().to_string());

    if nodes.len() != arrows.len() + 1 {
        return None;
    }

    // Attach leading pipe labels to the preceding arrow and strip them from the node token.
    for i in 1..nodes.len() {
        let trimmed = nodes[i].trim_start();
        if let Some(stripped) = trimmed.strip_prefix('|')
            && let Some(end_idx) = stripped.find('|')
        {
            let label_len = end_idx + 2;
            let label = &trimmed[..label_len];
            let rest = trimmed[label_len..].trim_start();
            arrows[i - 1].push_str(label);
            nodes[i] = rest.to_string();
        }
    }

    if nodes.iter().any(|node| node.is_empty()) {
        return None;
    }

    let mut statements = Vec::with_capacity(arrows.len());
    for i in 0..arrows.len() {
        statements.push(format!("{} {} {}", nodes[i], arrows[i], nodes[i + 1]));
    }
    Some(statements)
}

fn parse_edge_line(line: &str) -> Option<(String, Option<String>, String, EdgeMeta)> {
    // Mask bracket content to prevent matching dashes inside labels like A[wi-fi]
    let masked = mask_bracket_content(line);

    // Helper to extract from original line using match positions from masked line
    let extract = |m: regex::Match| -> &str { &line[m.start()..m.end()] };

    if let Some(caps) = PIPE_LABEL_RE.captures(&masked) {
        let left_match = caps.name("left")?;
        let right_match = caps.name("right")?;
        let label_match = caps.name("label")?;
        let arrow_match = caps.name("arrow")?;
        let left = extract(left_match).trim();
        let right = extract(right_match).trim();
        let label_clean = extract(label_match).trim();
        if !label_clean.is_empty() && !left.is_empty() && !right.is_empty() {
            let arrow = extract(arrow_match).trim();
            let edge_meta = parse_edge_meta(arrow);
            return Some((
                left.to_string(),
                Some(label_clean.to_string()),
                right.to_string(),
                edge_meta,
            ));
        }
    }

    // Quoted label syntax: -- "text" --> (match on original line, not masked,
    // because mask_bracket_content blanks quoted content).
    if let Some(caps) = QUOTED_LABEL_ARROW_RE.captures(line) {
        let left = caps.name("left")?.as_str().trim();
        let right = caps.name("right")?.as_str().trim();
        let label_clean = caps.name("label")?.as_str().trim();
        if !label_clean.is_empty() && !left.is_empty() && !right.is_empty() {
            let start = caps.name("start").map(|m| m.as_str()).unwrap_or("");
            let dash1 = caps.name("dash1")?.as_str();
            let dash2 = caps.name("dash2")?.as_str();
            let end = caps.name("end").map(|m| m.as_str()).unwrap_or("");
            let arrow = format!("{}{}{}{}", start, dash1, dash2, end);
            let edge_meta = parse_edge_meta(&arrow);
            return Some((
                left.to_string(),
                Some(label_clean.to_string()),
                right.to_string(),
                edge_meta,
            ));
        }
    }

    if let Some(caps) = COMPACT_DOTTED_LABEL_ARROW_RE.captures(&masked) {
        let left_match = caps.name("left")?;
        let right_match = caps.name("right")?;
        let label_match = caps.name("label")?;
        let left = extract(left_match).trim();
        let right = extract(right_match).trim();
        let label_clean = extract(label_match).trim().trim_matches('.');
        if !label_clean.is_empty() && !left.is_empty() && !right.is_empty() {
            let start = caps.name("start").map(|m| m.as_str()).unwrap_or("");
            let dash1 = caps.name("dash1")?.as_str();
            let dash2 = caps.name("dash2")?.as_str();
            let end = caps.name("end").map(|m| m.as_str()).unwrap_or("");
            let arrow = format!("{}{}.{}{}", start, dash1, dash2, end);
            let edge_meta = parse_edge_meta(&arrow);
            return Some((
                left.to_string(),
                Some(label_clean.to_string()),
                right.to_string(),
                edge_meta,
            ));
        }
    }

    if let Some(caps) = LABEL_ARROW_RE.captures(&masked) {
        let left_match = caps.name("left")?;
        let right_match = caps.name("right")?;
        let label_match = caps.name("label")?;
        let left = extract(left_match).trim();
        let right = extract(right_match).trim();
        let label_raw = extract(label_match).trim();
        let label_clean = label_raw.trim_matches('|').trim();
        if !label_clean.is_empty() && !left.is_empty() && !right.is_empty() {
            let start = caps.name("start").map(|m| m.as_str()).unwrap_or("");
            let dash1 = caps.name("dash1")?.as_str();
            let dash2 = caps.name("dash2")?.as_str();
            let end = caps.name("end").map(|m| m.as_str()).unwrap_or("");
            let arrow = format!("{}{}{}{}", start, dash1, dash2, end);
            let edge_meta = parse_edge_meta(&arrow);
            return Some((
                left.to_string(),
                Some(label_clean.to_string()),
                right.to_string(),
                edge_meta,
            ));
        }
    }

    let caps = ARROW_RE.captures(&masked)?;
    let left_match = caps.name("left")?;
    let right_match = caps.name("right")?;
    let left = extract(left_match).trim();
    let mut arrow = caps.name("arrow")?.as_str().trim().to_string();
    let mut right = extract(right_match).trim().to_string();

    if let Some((dec, rest)) = extract_leading_decoration(&right) {
        arrow.push(dec);
        right = rest;
    }

    if left.is_empty() || right.is_empty() || arrow.is_empty() {
        return None;
    }

    let (label, right_token) = if let Some(stripped) = right.strip_prefix('|') {
        if let Some(end) = stripped.find('|') {
            let label = stripped[..end].trim().to_string();
            let rest = stripped[end + 1..].trim();
            (Some(label), rest)
        } else {
            (None, right.as_str())
        }
    } else {
        (None, right.as_str())
    };

    if right_token.is_empty() {
        return None;
    }

    let edge_meta = parse_edge_meta(&arrow);
    Some((left.to_string(), label, right_token.to_string(), edge_meta))
}

#[derive(Debug, Clone, Copy)]
struct EdgeMeta {
    directed: bool,
    arrow_start: bool,
    arrow_end: bool,
    arrow_start_kind: Option<crate::ir::EdgeArrowhead>,
    arrow_end_kind: Option<crate::ir::EdgeArrowhead>,
    start_decoration: Option<crate::ir::EdgeDecoration>,
    end_decoration: Option<crate::ir::EdgeDecoration>,
    style: crate::ir::EdgeStyle,
}

fn parse_edge_meta(arrow: &str) -> EdgeMeta {
    let mut trimmed = arrow.trim().to_string();
    let mut start_decoration = None;
    let mut end_decoration = None;

    if trimmed.starts_with('o') {
        start_decoration = Some(crate::ir::EdgeDecoration::Circle);
        trimmed.remove(0);
    } else if trimmed.starts_with('x') {
        start_decoration = Some(crate::ir::EdgeDecoration::Cross);
        trimmed.remove(0);
    }

    if trimmed.ends_with('o') {
        end_decoration = Some(crate::ir::EdgeDecoration::Circle);
        trimmed.pop();
    } else if trimmed.ends_with('x') {
        end_decoration = Some(crate::ir::EdgeDecoration::Cross);
        trimmed.pop();
    }

    let arrow_start = trimmed.starts_with('<');
    let arrow_end = trimmed.ends_with('>');

    let style = if trimmed.contains('~') {
        crate::ir::EdgeStyle::Invisible
    } else if trimmed.contains('=') {
        crate::ir::EdgeStyle::Thick
    } else if trimmed.contains('.') {
        crate::ir::EdgeStyle::Dotted
    } else {
        crate::ir::EdgeStyle::Solid
    };

    let directed = arrow_start || arrow_end;

    EdgeMeta {
        directed,
        arrow_start,
        arrow_end,
        arrow_start_kind: None,
        arrow_end_kind: None,
        start_decoration,
        end_decoration,
        style,
    }
}

fn parse_direction_line(line: &str) -> Option<Direction> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() == 2 && parts[0] == "direction" {
        return Direction::from_token(parts[1]);
    }
    None
}

fn parse_class_def(line: &str, graph: &mut Graph) {
    let trimmed = line.trim();
    let mut parts = trimmed.splitn(3, char::is_whitespace);
    let _ = parts.next();
    let class_names = parts.next().unwrap_or("").trim();
    let rest = parts.next().unwrap_or("").trim();
    if class_names.is_empty() || rest.is_empty() {
        return;
    }
    let style = parse_node_style(rest);
    for class_name in class_names
        .split(',')
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
    {
        graph
            .class_defs
            .insert(class_name.to_string(), style.clone());
    }
}

fn parse_class_line(line: &str, graph: &mut Graph) {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return;
    }
    let class_name = parts.last().unwrap().to_string();
    let class_names: Vec<String> = class_name
        .split(',')
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect();
    if class_names.is_empty() {
        return;
    }
    let nodes_raw = parts[1..parts.len() - 1].join(" ");
    for node_id in nodes_raw.split(',') {
        let id = node_id.trim();
        if id.is_empty() {
            continue;
        }
        for class_name in &class_names {
            graph
                .node_classes
                .entry(id.to_string())
                .or_default()
                .push(class_name.clone());
            graph
                .subgraph_classes
                .entry(id.to_string())
                .or_default()
                .push(class_name.clone());
        }
    }
}

fn apply_node_classes(graph: &mut Graph, node_id: &str, classes: &[String]) {
    for class_name in classes {
        if class_name.is_empty() {
            continue;
        }
        graph
            .node_classes
            .entry(node_id.to_string())
            .or_default()
            .push(class_name.clone());
    }
}

fn apply_subgraph_classes(graph: &mut Graph, subgraph_id: &str, classes: &[String]) {
    for class_name in classes {
        if class_name.is_empty() {
            continue;
        }
        graph
            .subgraph_classes
            .entry(subgraph_id.to_string())
            .or_default()
            .push(class_name.clone());
    }
}

fn parse_style_line(line: &str, graph: &mut Graph) {
    let mut parts = line.splitn(3, ' ');
    let _ = parts.next();
    let node_id = parts.next().unwrap_or("").trim();
    let rest = parts.next().unwrap_or("").trim();
    if node_id.is_empty() || rest.is_empty() {
        return;
    }
    let style = parse_node_style(rest);
    for raw in node_id.split(',') {
        let id = raw.trim();
        if id.is_empty() {
            continue;
        }
        graph.node_styles.insert(id.to_string(), style.clone());
        graph.subgraph_styles.insert(id.to_string(), style.clone());
    }
}

fn parse_link_style_line(line: &str, graph: &mut Graph) {
    let trimmed = line.trim();
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    if tokens.len() < 3 {
        return;
    }

    let mut style_idx = None;
    for (idx, token) in tokens.iter().enumerate().skip(1) {
        if token.contains(':') {
            style_idx = Some(idx);
            break;
        }
    }
    let Some(style_idx) = style_idx else {
        return;
    };
    let index_tokens = &tokens[1..style_idx];
    let style_str = tokens[style_idx..].join(" ");
    if style_str.is_empty() {
        return;
    }

    let style = parse_edge_style(&style_str);
    if index_tokens.len() == 1 && index_tokens[0] == "default" {
        graph.edge_style_default = Some(style);
        return;
    }

    for raw in index_tokens.iter().flat_map(|token| token.split(',')) {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        if let Ok(index) = token.parse::<usize>() {
            graph.edge_styles.insert(index, style.clone());
        }
    }
}

fn tokenize_quoted(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }
        if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(current);
                current = String::new();
            }
            continue;
        }
        current.push(ch);
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn parse_click_line(line: &str) -> Option<(String, crate::ir::NodeLink)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let keyword_len = if lower.starts_with("click ") {
        5
    } else if lower.starts_with("link ") {
        4
    } else {
        return None;
    };
    let rest = trimmed[keyword_len..].trim();
    let tokens = tokenize_quoted(rest);
    if tokens.len() < 2 {
        return None;
    }
    let id = tokens[0].clone();
    let mut idx = 1usize;
    if tokens[idx].eq_ignore_ascii_case("call") {
        return None;
    }
    if tokens[idx].eq_ignore_ascii_case("href") {
        idx += 1;
    }
    let url = tokens.get(idx)?.clone();
    idx += 1;
    let mut title = None;
    let mut target = None;
    if let Some(token) = tokens.get(idx) {
        if token.starts_with('_') {
            target = Some(token.clone());
            idx += 1;
        } else {
            title = Some(token.clone());
            idx += 1;
        }
    }
    if target.is_none()
        && let Some(token) = tokens.get(idx)
        && token.starts_with('_')
    {
        target = Some(token.clone());
    }

    Some((id, crate::ir::NodeLink { url, title, target }))
}

fn parse_node_style(input: &str) -> crate::ir::NodeStyle {
    let mut style = crate::ir::NodeStyle::default();
    for part in input.split(',') {
        let mut kv = part.splitn(2, ':');
        let key = kv.next().unwrap_or("").trim();
        let value = kv.next().unwrap_or("").trim().trim_end_matches(';').trim();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        match key {
            "fill" => style.fill = Some(value.to_string()),
            "stroke" => style.stroke = Some(value.to_string()),
            "stroke-width" => {
                let width = value.trim_end_matches("px").parse::<f32>().ok();
                style.stroke_width = width;
            }
            "stroke-dasharray" => style.stroke_dasharray = Some(value.to_string()),
            "color" => style.text_color = Some(value.to_string()),
            "font-style" => style.font_style = Some(value.to_string()),
            "font-weight" => style.font_weight = Some(value.to_string()),
            _ => {}
        }
    }
    style
}

fn parse_edge_style(input: &str) -> crate::ir::EdgeStyleOverride {
    let mut style = crate::ir::EdgeStyleOverride::default();
    for part in input.split(',') {
        let mut kv = part.splitn(2, ':');
        let key = kv.next().unwrap_or("").trim();
        let value = kv.next().unwrap_or("").trim();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        match key {
            "stroke" => style.stroke = Some(value.to_string()),
            "stroke-width" => {
                style.stroke_width = value.trim_end_matches("px").parse::<f32>().ok();
            }
            "stroke-dasharray" => style.dasharray = Some(value.to_string()),
            "color" => style.label_color = Some(value.to_string()),
            _ => {}
        }
    }
    style
}

fn parse_node_token(
    token: &str,
) -> (
    String,
    Option<String>,
    Option<crate::ir::NodeShape>,
    Vec<String>,
    bool,
) {
    let (base, classes) = split_inline_classes(token);
    let trimmed = base.trim();

    // Try the v11.3+ `@{ shape: "...", label: "..." }` declarative syntax.
    if let Some(meta) = parse_at_shape_syntax(trimmed) {
        return (meta.id, Some(meta.label), Some(meta.shape), classes, false);
    }

    if let Some((id, label, shape, md)) = split_block_arrow_label(trimmed) {
        return (id, Some(label), Some(shape), classes, md);
    }
    if let Some((id, label, shape, md)) = split_asymmetric_label(trimmed) {
        return (id, Some(label), Some(shape), classes, md);
    }
    if let Some((id, label, shape, md)) = split_id_label(trimmed) {
        return (id.to_string(), Some(label), Some(shape), classes, md);
    }

    let id = trimmed.split_whitespace().next().unwrap_or("").to_string();
    (id, None, None, classes, false)
}

/// Parse the v11.3+ `@{ shape: "name", label: "text" }` syntax.
/// The format is `NodeId@{ shape: name, label: "text" }`.
/// Metadata parsed from `@{ ... }` node syntax.
struct AtNodeMeta {
    id: String,
    label: String,
    shape: crate::ir::NodeShape,
    img: Option<String>,
    img_w: Option<f32>,
    img_h: Option<f32>,
    img_pos: Option<String>,
    constraint: Option<String>,
    icon: Option<String>,
}

fn parse_at_shape_syntax(token: &str) -> Option<AtNodeMeta> {
    let at_pos = token.find("@{")?;
    if !token.trim_end().ends_with('}') {
        return None;
    }
    let id = token[..at_pos].trim().to_string();
    if id.is_empty() {
        return None;
    }
    let block = &token[at_pos + 2..token.len() - 1].trim();
    // Parse key:value pairs from the block (shape, label).
    let mut shape_name: Option<String> = None;
    let mut label: Option<String> = None;
    let mut img: Option<String> = None;
    let mut img_w: Option<f32> = None;
    let mut img_h: Option<f32> = None;
    let mut img_pos: Option<String> = None;
    let mut constraint: Option<String> = None;
    let mut icon: Option<String> = None;
    let mut form: Option<String> = None;
    for pair in block.split(',') {
        let pair = pair.trim();
        if let Some(colon) = pair.find(':') {
            let key = pair[..colon].trim().trim_matches('"').trim_matches('\'');
            let val = pair[colon + 1..]
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            match key {
                "shape" => shape_name = Some(val.to_string()),
                "label" => label = Some(val.to_string()),
                "img" => img = Some(val.to_string()),
                "w" => img_w = val.parse().ok(),
                "h" => img_h = val.parse().ok(),
                "pos" => img_pos = Some(val.to_string()),
                "constraint" => constraint = Some(val.to_string()),
                "icon" => icon = Some(val.to_string()),
                "form" => form = Some(val.to_string()),
                _ => {}
            }
        }
    }
    let shape_name = shape_name?;
    let shape = if shape_name == "icon" {
        match form.as_deref() {
            Some("circle") => crate::ir::NodeShape::IconCircle,
            Some("square") => crate::ir::NodeShape::IconSquare,
            Some("rounded") => crate::ir::NodeShape::IconRounded,
            _ => crate::ir::NodeShape::Icon,
        }
    } else {
        resolve_shape_name(&shape_name)?
    };
    let label = label.unwrap_or_else(|| {
        if icon.is_some() {
            String::new()
        } else {
            id.clone()
        }
    });
    Some(AtNodeMeta {
        id,
        label,
        shape,
        img,
        img_w,
        img_h,
        img_pos,
        constraint,
        icon,
    })
}

fn resolve_shape_name(name: &str) -> Option<crate::ir::NodeShape> {
    use crate::ir::NodeShape;
    match name {
        "rect" | "rectangle" => Some(NodeShape::Rectangle),
        "round" | "rounded" => Some(NodeShape::RoundRect),
        "stadium" => Some(NodeShape::Stadium),
        "subroutine" | "fr-rect" => Some(NodeShape::Subroutine),
        "cyl" | "cylinder" | "database" => Some(NodeShape::Cylinder),
        "circle" => Some(NodeShape::Circle),
        "dbl-circ" | "double-circle" => Some(NodeShape::DoubleCircle),
        "diam" | "diamond" | "decision" => Some(NodeShape::Diamond),
        "hex" | "hexagon" => Some(NodeShape::Hexagon),
        "lean-r" | "lean-right" => Some(NodeShape::LeanRight),
        "lean-l" | "lean-left" => Some(NodeShape::LeanLeft),
        "trap-b" | "trapezoid" => Some(NodeShape::Trapezoid),
        "trap-t" | "trapezoid-alt" => Some(NodeShape::TrapezoidAlt),
        "para" | "parallelogram" => Some(NodeShape::Parallelogram),
        "para-alt" | "parallelogram-alt" => Some(NodeShape::ParallelogramAlt),
        "flag" | "paper-tape" => Some(NodeShape::WavyRect),
        "notch-rect" | "card" => Some(NodeShape::NotchRect),
        "tag-rect" | "tagged-rect" => Some(NodeShape::TagRect),
        "doc" | "document" => Some(NodeShape::Document),
        "lin-doc" | "lined-document" => Some(NodeShape::LinedDocument),
        "tag-doc" | "tagged-document" => Some(NodeShape::TagDocument),
        "docs" | "stacked-document" => Some(NodeShape::StackedDocument),
        "win-pane" | "window-pane" => Some(NodeShape::WindowPane),
        "hourglass" => Some(NodeShape::Hourglass),
        "bolt" | "lightning-bolt" => Some(NodeShape::LightningBolt),
        "brace" | "comment" | "brace-l" => Some(NodeShape::BraceLeft),
        "brace-r" => Some(NodeShape::BraceRight),
        "braces" => Some(NodeShape::BraceBoth),
        "odd" => Some(NodeShape::OddShape),
        "lin-cyl" | "lined-cylinder" => Some(NodeShape::LinedCylinder),
        "curv-trap" | "curved-trapezoid" => Some(NodeShape::CurvedTrapezoid),
        "text" => Some(NodeShape::Text),
        "icon" => Some(NodeShape::Icon),
        "cloud" => Some(NodeShape::Cloud),
        "bang" => Some(NodeShape::Bang),
        "tri" | "triangle" | "extract" => Some(NodeShape::Triangle),
        "flip-tri" | "flipped-triangle" | "manual-file" => Some(NodeShape::FlippedTriangle),
        "sm-circ" | "small-circle" | "start" => Some(NodeShape::SmallCircle),
        "f-circ" | "filled-circle" | "junction" => Some(NodeShape::FilledCircle),
        "delay" | "half-rounded-rect" => Some(NodeShape::HalfRoundedRect),
        "sl-rect" | "sloped-rect" | "sloped-rectangle" | "manual-input" => {
            Some(NodeShape::SlopedRect)
        }
        "notch-pent" | "notched-pentagon" | "loop-limit" => Some(NodeShape::NotchedPentagon),
        "st-rect" | "stacked-rect" | "procs" => Some(NodeShape::StackedRect),
        "bow-rect" | "bow-tie-rect" | "stored-data" => Some(NodeShape::BowTieRect),
        "fr-circ" | "framed-circle" | "stop" => Some(NodeShape::FramedCircle),
        "cross-circ" | "crossed-circle" | "summary" => Some(NodeShape::CrossedCircle),
        "h-cyl" | "horizontal-cylinder" | "das" => Some(NodeShape::HorizontalCylinder),
        "div-rect" | "divided-rect" | "div-proc" => Some(NodeShape::DividedRect),
        "lin-rect" | "lined-rect" | "lin-proc" => Some(NodeShape::LinedRect),
        "wave-rect" | "wavy-rect" => Some(NodeShape::WavyRect),
        "fork" | "join" => Some(NodeShape::ForkJoin),
        _ => None,
    }
}

fn split_block_arrow_label(token: &str) -> Option<(String, String, crate::ir::NodeShape, bool)> {
    let trimmed = token.trim();
    let start = trimmed.find("<[")?;
    let id = trimmed[..start].trim();
    if id.is_empty() {
        return None;
    }
    let label_end = trimmed.rfind("]>(")?;
    if label_end <= start + 2 || !trimmed.ends_with(')') {
        return None;
    }
    let label = trimmed[start + 2..label_end].trim();
    let dirs = &trimmed[label_end + 3..trimmed.len() - 1];
    let shape = block_arrow_shape_from_dirs(dirs)?;
    let (text, md) = strip_quotes_markdown(label);
    Some((id.to_string(), text, shape, md))
}

fn block_arrow_shape_from_dirs(dirs: &str) -> Option<crate::ir::NodeShape> {
    let mut right = false;
    let mut left = false;
    let mut up = false;
    let mut down = false;

    for dir in dirs.split(',') {
        match dir.trim().to_ascii_lowercase().as_str() {
            "right" => right = true,
            "left" => left = true,
            "up" => up = true,
            "down" => down = true,
            "x" => {
                right = true;
                left = true;
            }
            "y" => {
                up = true;
                down = true;
            }
            "" => {}
            _ => return None,
        }
    }

    use crate::ir::NodeShape;
    Some(match (right, left, up, down) {
        (true, true, true, true) => NodeShape::BlockArrowAll,
        (true, true, true, false) => NodeShape::BlockArrowXUp,
        (true, true, false, true) => NodeShape::BlockArrowXDown,
        (true, false, true, true) => NodeShape::BlockArrowYRight,
        (false, true, true, true) => NodeShape::BlockArrowYLeft,
        (true, true, false, false) => NodeShape::BlockArrowX,
        (false, false, true, true) => NodeShape::BlockArrowY,
        (true, false, true, false) => NodeShape::BlockArrowRightUp,
        (true, false, false, true) => NodeShape::BlockArrowRightDown,
        (false, true, true, false) => NodeShape::BlockArrowLeftUp,
        (false, true, false, true) => NodeShape::BlockArrowLeftDown,
        (true, false, false, false) => NodeShape::BlockArrowRight,
        (false, true, false, false) => NodeShape::BlockArrowLeft,
        (false, false, true, false) => NodeShape::BlockArrowUp,
        (false, false, false, true) => NodeShape::BlockArrowDown,
        _ => return None,
    })
}

fn split_asymmetric_label(token: &str) -> Option<(String, String, crate::ir::NodeShape, bool)> {
    let trimmed = token.trim();
    if trimmed.contains('[') {
        return None;
    }
    let Some(pos) = trimmed.find('>') else {
        return None;
    };
    if !trimmed.ends_with(']') {
        return None;
    }
    let id = trimmed[..pos].trim();
    if id.is_empty() {
        return None;
    }
    let label = trimmed[pos + 1..trimmed.len() - 1].trim();
    if label.is_empty() {
        return None;
    }
    let (text, md) = strip_quotes_markdown(label);
    Some((id.to_string(), text, crate::ir::NodeShape::Asymmetric, md))
}

fn split_inline_classes(token: &str) -> (String, Vec<String>) {
    let mut parts = token.split(":::");
    let base = parts.next().unwrap_or("").trim().to_string();
    let classes = parts
        .flat_map(|part| part.split(','))
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    (base, classes)
}

fn split_id_label(token: &str) -> Option<(&str, String, crate::ir::NodeShape, bool)> {
    if let Some(start) = token.find('[')
        && token.ends_with(']')
    {
        let id = token[..start].trim();
        if !id.is_empty() {
            let raw = &token[start..];
            let (label, shape, md) = parse_shape_from_brackets(raw);
            return Some((id, label, shape, md));
        }
    }

    if let Some(start) = token.find('(')
        && token.ends_with(')')
    {
        let id = token[..start].trim();
        if !id.is_empty() {
            let raw = &token[start..];
            let (label, shape, md) = parse_shape_from_parens(raw);
            return Some((id, label, shape, md));
        }
    }

    if let Some(start) = token.find('{')
        && token.ends_with('}')
    {
        let id = token[..start].trim();
        if !id.is_empty() {
            let raw = &token[start..];
            let (label, shape, md) = parse_shape_from_braces(raw);
            return Some((id, label, shape, md));
        }
    }

    None
}

fn parse_shape_from_brackets(raw: &str) -> (String, crate::ir::NodeShape, bool) {
    let trimmed = raw.trim();
    if trimmed.starts_with("[/") && trimmed.ends_with("/]") {
        let (t, md) = strip_quotes_markdown(&trimmed[2..trimmed.len() - 2]);
        return (t, crate::ir::NodeShape::Parallelogram, md);
    }
    if trimmed.starts_with("[\\") && trimmed.ends_with("\\]") {
        let (t, md) = strip_quotes_markdown(&trimmed[2..trimmed.len() - 2]);
        return (t, crate::ir::NodeShape::ParallelogramAlt, md);
    }
    if trimmed.starts_with("[/") && trimmed.ends_with("\\]") {
        let (t, md) = strip_quotes_markdown(&trimmed[2..trimmed.len() - 2]);
        return (t, crate::ir::NodeShape::Trapezoid, md);
    }
    if trimmed.starts_with("[\\") && trimmed.ends_with("/]") {
        let (t, md) = strip_quotes_markdown(&trimmed[2..trimmed.len() - 2]);
        return (t, crate::ir::NodeShape::TrapezoidAlt, md);
    }
    if trimmed.starts_with("[[") && trimmed.ends_with("]]") {
        let (t, md) = strip_quotes_markdown(&trimmed[2..trimmed.len() - 2]);
        return (t, crate::ir::NodeShape::Subroutine, md);
    }
    if trimmed.starts_with("[(") && trimmed.ends_with(")]") {
        let (t, md) = strip_quotes_markdown(&trimmed[2..trimmed.len() - 2]);
        return (t, crate::ir::NodeShape::Cylinder, md);
    }
    if trimmed.starts_with("[") && trimmed.ends_with("]") {
        let inner = &trimmed[1..trimmed.len() - 1];
        if inner.starts_with('(') && inner.ends_with(')') {
            let (t, md) = strip_quotes_markdown(&inner[1..inner.len() - 1]);
            return (t, crate::ir::NodeShape::Stadium, md);
        }
        let (t, md) = strip_quotes_markdown(inner);
        return (t, crate::ir::NodeShape::Rectangle, md);
    }
    let (t, md) = strip_quotes_markdown(trimmed);
    (t, crate::ir::NodeShape::Rectangle, md)
}

fn parse_shape_from_parens(raw: &str) -> (String, crate::ir::NodeShape, bool) {
    let trimmed = raw.trim();
    if trimmed.starts_with("(((") && trimmed.ends_with(")))") {
        let (t, md) = strip_quotes_markdown(&trimmed[3..trimmed.len() - 3]);
        return (t, crate::ir::NodeShape::DoubleCircle, md);
    }
    if trimmed.starts_with("((") && trimmed.ends_with("))") {
        let (t, md) = strip_quotes_markdown(&trimmed[2..trimmed.len() - 2]);
        return (t, crate::ir::NodeShape::Circle, md);
    }
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let inner = &trimmed[1..trimmed.len() - 1];
        if inner.starts_with('[') && inner.ends_with(']') {
            let (t, md) = strip_quotes_markdown(&inner[1..inner.len() - 1]);
            return (t, crate::ir::NodeShape::Stadium, md);
        }
        let (t, md) = strip_quotes_markdown(inner);
        return (t, crate::ir::NodeShape::RoundRect, md);
    }
    let (t, md) = strip_quotes_markdown(trimmed);
    (t, crate::ir::NodeShape::RoundRect, md)
}

fn parse_shape_from_braces(raw: &str) -> (String, crate::ir::NodeShape, bool) {
    let trimmed = raw.trim();
    if trimmed.starts_with("{{") && trimmed.ends_with("}}") {
        let (t, md) = strip_quotes_markdown(&trimmed[2..trimmed.len() - 2]);
        return (t, crate::ir::NodeShape::Hexagon, md);
    }
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        let (t, md) = strip_quotes_markdown(&trimmed[1..trimmed.len() - 1]);
        return (t, crate::ir::NodeShape::Diamond, md);
    }
    let (t, md) = strip_quotes_markdown(trimmed);
    (t, crate::ir::NodeShape::Diamond, md)
}

fn strip_quotes(input: &str) -> String {
    strip_quotes_markdown(input).0
}

/// Strip quotes from a label. Returns `(text, is_markdown)`.
/// Detects the markdown string pattern `` "`...`" `` and returns `true`
/// as the second element when found.
fn strip_quotes_markdown(input: &str) -> (String, bool) {
    let trimmed = input.trim();
    // Detect markdown string: "`...`"
    if trimmed.starts_with("\"`") && trimmed.ends_with("`\"") && trimmed.len() >= 4 {
        let inner = &trimmed[2..trimmed.len() - 2];
        return (inner.to_string(), true);
    }
    // Also detect backtick-only wrapping (outer quotes already stripped by regex):
    // `...`
    if trimmed.starts_with('`') && trimmed.ends_with('`') && trimmed.len() >= 2 {
        let inner = &trimmed[1..trimmed.len() - 1];
        return (inner.to_string(), true);
    }
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        (trimmed[1..trimmed.len() - 1].replace("\"\"", "\""), false)
    } else if trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2 {
        (trimmed[1..trimmed.len() - 1].to_string(), false)
    } else {
        (trimmed.to_string(), false)
    }
}

fn count_indent(line: &str) -> usize {
    let mut count = 0;
    for ch in line.chars() {
        match ch {
            ' ' => count += 1,
            '\t' => count += 2,
            _ => break,
        }
    }
    count
}

// ── TreeView parser ─────────────────────────────────────────────────────

fn find_tree_view_token_outside_quotes(input: &str, token: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (idx, ch) in input.char_indices() {
        if let Some(quote_ch) = quote {
            if ch == quote_ch {
                quote = None;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }
        if input[idx..].starts_with(token) {
            return Some(idx);
        }
    }
    None
}

fn split_tree_view_description(input: &str) -> (&str, Option<String>) {
    let Some(idx) = find_tree_view_token_outside_quotes(input, "##") else {
        return (input, None);
    };
    let description = input[idx + 2..].trim();
    let description = if description.is_empty() {
        None
    } else {
        Some(description.to_string())
    };
    (&input[..idx], description)
}

fn take_tree_view_icon_annotation(mut input: String) -> (String, Option<String>) {
    let Some(start) = find_tree_view_token_outside_quotes(&input, "icon(") else {
        return (input, None);
    };
    let value_start = start + "icon(".len();
    let Some(end_rel) = input[value_start..].find(')') else {
        return (input, None);
    };
    let end = value_start + end_rel;
    let icon = input[value_start..end].trim().to_string();
    input.replace_range(start..end + 1, "");
    (input, Some(icon))
}

fn take_tree_view_class_annotation(mut input: String) -> (String, Option<String>) {
    let Some(start) = find_tree_view_token_outside_quotes(&input, ":::") else {
        return (input, None);
    };
    let value_start = start + 3;
    let end = input[value_start..]
        .char_indices()
        .find_map(|(idx, ch)| ch.is_whitespace().then_some(value_start + idx))
        .unwrap_or(input.len());
    let css_class = input[value_start..end].trim().to_string();
    input.replace_range(start..end, "");
    let css_class = if css_class.is_empty() {
        None
    } else {
        Some(css_class)
    };
    (input, css_class)
}

fn resolve_tree_view_icon_id(name: &str, node_type: crate::ir::TreeViewNodeType) -> &'static str {
    if node_type == crate::ir::TreeViewNodeType::Directory {
        return "folder";
    }

    match name {
        ".gitignore" => return "git",
        ".eslintrc" | ".eslintrc.js" | ".eslintrc.json" | ".prettierrc" | ".prettierrc.json" => {
            return "config";
        }
        "Dockerfile" | "docker-compose.yml" | "docker-compose.yaml" => return "docker",
        "Makefile" => return "terminal",
        "README.md" => return "markdown",
        "package.json" => return "json",
        "package-lock.json" | "yarn.lock" | "pnpm-lock.yaml" => return "lock",
        ".env" | ".env.local" | ".env.production" => return "env",
        "tsconfig.json" => return "typescript",
        "LICENSE" => return "license",
        _ => {}
    }

    let lower = name.to_ascii_lowercase();
    if let Some(dot_idx) = lower.rfind('.')
        && dot_idx > 0
    {
        return match &lower[dot_idx..] {
            ".js" | ".mjs" | ".cjs" => "javascript",
            ".jsx" | ".tsx" => "react",
            ".ts" => "typescript",
            ".py" => "python",
            ".rb" => "ruby",
            ".rs" => "rust",
            ".go" => "go",
            ".java" => "java",
            ".cs" => "csharp",
            ".cpp" => "cpp",
            ".c" | ".h" => "c",
            ".json" => "json",
            ".yaml" | ".yml" => "yaml",
            ".toml" => "config",
            ".xml" => "xml",
            ".html" | ".htm" => "html",
            ".css" | ".scss" | ".less" => "css",
            ".md" | ".mdx" => "markdown",
            ".sh" | ".bash" | ".zsh" | ".ps1" | ".bat" => "terminal",
            ".svg" | ".png" | ".jpg" | ".jpeg" | ".gif" | ".ico" | ".webp" => "image",
            ".sql" | ".db" => "database",
            ".lock" => "lock",
            ".env" => "env",
            ".vue" => "vue",
            ".svelte" => "svelte",
            ".txt" => "file",
            _ => "file",
        };
    }

    "file"
}

fn parse_tree_view_node_line(trimmed: &str) -> Option<crate::ir::TreeViewNode> {
    let (body, description) = split_tree_view_description(trimmed);
    let (body, raw_icon) = take_tree_view_icon_annotation(body.to_string());
    let (body, css_class) = take_tree_view_class_annotation(body);
    let label = body.trim();
    if label.is_empty() {
        return None;
    }

    let (mut name, _) = strip_quotes_markdown(label);
    let is_directory = name.ends_with('/');
    if is_directory {
        name.pop();
    }
    if name.is_empty() {
        return None;
    }

    let node_type = if is_directory {
        crate::ir::TreeViewNodeType::Directory
    } else {
        crate::ir::TreeViewNodeType::File
    };
    let icon_id = raw_icon
        .map(|icon| {
            if icon.is_empty() {
                "none".to_string()
            } else {
                icon
            }
        })
        .or_else(|| Some(resolve_tree_view_icon_id(&name, node_type).to_string()));

    Some(crate::ir::TreeViewNode {
        name,
        node_type,
        icon_id,
        css_class,
        description,
        children: Vec::new(),
    })
}

fn parse_tree_view_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::TreeView;
    let (lines, init_config) = preprocess_input_keep_indent(input)?;

    let mut stack: Vec<(usize, Vec<usize>)> = Vec::new();
    let mut roots: Vec<crate::ir::TreeViewNode> = Vec::new();
    let mut base_indent: Option<usize> = None;

    for line in &lines {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("treeview") {
            continue;
        }
        if trimmed.starts_with("%%") {
            continue;
        }
        if lower.starts_with("title") {
            let rest = trimmed.get(5..).unwrap_or("").trim();
            if !rest.is_empty() {
                graph.tree_view.title = Some(rest.to_string());
            }
            continue;
        }
        if lower.starts_with("acctitle") || lower.starts_with("accdescr") {
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }

        // Calculate indentation
        let indent = line.len() - line.trim_start().len();
        let Some(node) = parse_tree_view_node_line(trimmed) else {
            continue;
        };

        if base_indent.is_none() {
            base_indent = Some(indent);
        }
        let level = if indent >= base_indent.unwrap_or(0) {
            (indent - base_indent.unwrap_or(0)) / 4 // normalize to levels
        } else {
            0
        };

        // Pop stack until we find the parent
        while stack
            .last()
            .is_some_and(|(stack_level, _)| *stack_level >= level)
        {
            stack.pop();
        }

        let path = if let Some((_, parent_path)) = stack.last() {
            let Some(parent) = tree_view_node_mut(&mut roots, parent_path) else {
                continue;
            };
            parent.children.push(node);
            let mut child_path = parent_path.clone();
            child_path.push(parent.children.len() - 1);
            child_path
        } else {
            roots.push(node);
            vec![roots.len() - 1]
        };
        stack.push((level, path));
    }

    graph.tree_view.root = roots;

    Ok(ParseOutput { graph, init_config })
}

fn tree_view_node_mut<'a>(
    roots: &'a mut [crate::ir::TreeViewNode],
    path: &[usize],
) -> Option<&'a mut crate::ir::TreeViewNode> {
    let (first, rest) = path.split_first()?;
    let mut node = roots.get_mut(*first)?;
    for index in rest {
        node = node.children.get_mut(*index)?;
    }
    Some(node)
}

// ── Ishikawa parser ─────────────────────────────────────────────────────

fn parse_ishikawa_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Ishikawa;
    let (lines, init_config) = preprocess_input_keep_indent(input)?;

    // Same indentation-based tree as treeView, but first node = root (effect)
    let mut all_nodes: Vec<(usize, String)> = Vec::new(); // (indent_level, text)
    let mut base_indent: Option<usize> = None;

    for line in &lines {
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("ishikawa") {
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }

        let indent = line.len() - line.trim_start().len();
        let text = line.trim().to_string();
        if text.is_empty() {
            continue;
        }

        if base_indent.is_none() {
            base_indent = Some(indent);
        }
        all_nodes.push((indent, text));
    }

    if all_nodes.is_empty() {
        return Ok(ParseOutput { graph, init_config });
    }

    // Build tree: first node = root (effect), rest = causes.
    // JS sets baseLevel from the FIRST CAUSE (second node), not the root.
    // This handles the case where root and causes have the same indentation.
    fn build_ishikawa_tree(nodes: &[(usize, String)]) -> crate::ir::IshikawaNode {
        let mut root = crate::ir::IshikawaNode {
            text: nodes[0].1.clone(),
            children: Vec::new(),
        };

        if nodes.len() <= 1 {
            return root;
        }

        // baseLevel = indent of first cause (second node)
        let base_level = nodes[1].0;

        // Stack-based tree building for causes (nodes[1..])
        // Level 0 = root's direct children (primary causes)
        // Level 1+ = sub-causes
        let mut stack: Vec<(usize, *mut crate::ir::IshikawaNode)> = Vec::new();
        // Push root at level -1 (below all causes)
        stack.push((usize::MAX, &mut root as *mut _)); // sentinel level

        for &(indent, ref text) in &nodes[1..] {
            let level = if indent >= base_level {
                indent - base_level
            } else {
                0
            };

            // Pop until we find a parent with strictly lower level
            while stack.len() > 1 {
                let top_level = stack.last().map(|(l, _)| *l).unwrap_or(usize::MAX);
                if top_level != usize::MAX && top_level >= level {
                    stack.pop();
                } else {
                    break;
                }
            }

            let new_node = crate::ir::IshikawaNode {
                text: text.clone(),
                children: Vec::new(),
            };

            let parent = stack.last().unwrap().1;
            unsafe {
                (*parent).children.push(new_node);
                let last_child = (*parent).children.last_mut().unwrap() as *mut _;
                stack.push((level, last_child));
            }
        }

        root
    }

    graph.ishikawa.root = Some(build_ishikawa_tree(&all_nodes));

    Ok(ParseOutput { graph, init_config })
}

// ── Event Modeling parser ───────────────────────────────────────────────

fn parse_eventmodeling_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::EventModeling;
    graph.direction = Direction::LeftRight;
    let (lines, init_config) = preprocess_input_keep_indent(input)?;

    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        let lower = trimmed.to_ascii_lowercase();
        if trimmed.is_empty() || lower.starts_with("eventmodeling") {
            i += 1;
            continue;
        }
        if lower.starts_with("acctitle") {
            let val = trimmed
                .strip_prefix("accTitle")
                .or_else(|| trimmed.strip_prefix("acctitle"))
                .unwrap_or("")
                .trim_start_matches(':')
                .trim();
            if !val.is_empty() {
                graph.acc_title = Some(val.to_string());
            }
            i += 1;
            continue;
        }
        if lower.starts_with("accdescr") {
            let val = trimmed
                .strip_prefix("accDescr")
                .or_else(|| trimmed.strip_prefix("accdescr"))
                .unwrap_or("")
                .trim_start_matches(':')
                .trim()
                .trim_start_matches('{')
                .trim_end_matches('}')
                .trim();
            if !val.is_empty() {
                graph.acc_descr = Some(val.to_string());
            }
            i += 1;
            continue;
        }
        if lower.starts_with("title") {
            let rest = trimmed.get(5..).unwrap_or("").trim();
            if !rest.is_empty() {
                graph.diagram_title = Some(strip_quotes(rest));
            }
            i += 1;
            continue;
        }
        if lower.starts_with("data ") {
            if let Some((entity, consumed)) = parse_eventmodeling_data_block(&lines, i) {
                graph.eventmodeling.data_entities.push(entity);
                i = consumed;
                continue;
            }
        }
        if let Some(frame) = parse_eventmodeling_frame_line(trimmed) {
            graph.eventmodeling.frames.push(frame);
        }
        i += 1;
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_eventmodeling_data_block(
    lines: &[String],
    start: usize,
) -> Option<(crate::ir::EventModelingDataEntity, usize)> {
    let header = lines.get(start)?.trim();
    let rest = header.strip_prefix("data ")?.trim();
    let mut parts = rest.splitn(2, char::is_whitespace);
    let name = parts.next()?.trim();
    if name.is_empty() {
        return None;
    }
    let mut after_name = parts.next().unwrap_or("").trim();
    let mut data_type = None;
    if after_name.starts_with('`')
        && let Some(end) = after_name[1..].find('`')
    {
        data_type = Some(after_name[1..end + 1].to_string());
        after_name = after_name[end + 2..].trim();
    }

    let mut body = String::new();
    if let Some(open) = after_name.find('{') {
        let after_open = &after_name[open + 1..];
        if let Some(close) = after_open.rfind('}') {
            body = after_open[..close].trim().to_string();
            return Some((
                crate::ir::EventModelingDataEntity {
                    name: name.to_string(),
                    data_type,
                    value: body,
                },
                start + 1,
            ));
        }
    } else {
        return None;
    }

    let mut i = start + 1;
    while i < lines.len() {
        let line = &lines[i];
        if line.trim() == "}" {
            i += 1;
            break;
        }
        if let Some(close) = line.rfind('}')
            && line[close + 1..].trim().is_empty()
        {
            let before = line[..close].trim_end();
            if !before.trim().is_empty() {
                if !body.is_empty() {
                    body.push('\n');
                }
                body.push_str(before);
            }
            i += 1;
            break;
        }
        if !body.is_empty() {
            body.push('\n');
        }
        body.push_str(line.trim_end());
        i += 1;
    }

    Some((
        crate::ir::EventModelingDataEntity {
            name: name.to_string(),
            data_type,
            value: body.trim().to_string(),
        },
        i,
    ))
}

fn parse_eventmodeling_frame_line(line: &str) -> Option<crate::ir::EventModelingFrame> {
    let mut parts = line.split_whitespace();
    let frame_kind = parts.next()?.to_ascii_lowercase();
    let reset = match frame_kind.as_str() {
        "tf" | "timeframe" => false,
        "rf" | "resetframe" => true,
        _ => return None,
    };
    let name = parts.next()?.to_string();
    let entity_type = parse_eventmodeling_entity_type(parts.next()?);
    let entity_identifier = parts.next()?.to_string();
    let mut rest = parts.collect::<Vec<_>>().join(" ");

    let mut source_frames = Vec::new();
    loop {
        let trimmed = rest.trim_start();
        if !trimmed.starts_with("->>") {
            rest = trimmed.to_string();
            break;
        }
        let after_arrow = trimmed[3..].trim_start();
        let mut after_parts = after_arrow.splitn(2, char::is_whitespace);
        let source = after_parts.next().unwrap_or("").trim();
        if source.is_empty() {
            rest.clear();
            break;
        }
        source_frames.push(source.to_string());
        rest = after_parts.next().unwrap_or("").to_string();
    }

    let mut data_reference = None;
    let trimmed = rest.trim_start();
    if let Some(after_open) = trimmed.strip_prefix("[[")
        && let Some(close) = after_open.find("]]")
    {
        let reference = after_open[..close].trim();
        if !reference.is_empty() {
            data_reference = Some(reference.to_string());
        }
        rest = after_open[close + 2..].trim_start().to_string();
    } else {
        rest = trimmed.to_string();
    }

    let data_inline_value = parse_eventmodeling_inline_data(rest.trim());

    Some(crate::ir::EventModelingFrame {
        name,
        entity_type,
        entity_identifier,
        source_frames,
        data_reference,
        data_inline_value,
        reset,
    })
}

fn parse_eventmodeling_entity_type(token: &str) -> crate::ir::EventModelingEntityType {
    match token.to_ascii_lowercase().as_str() {
        "ui" => crate::ir::EventModelingEntityType::Ui,
        "pcr" | "processor" => crate::ir::EventModelingEntityType::Processor,
        "rmo" | "readmodel" => crate::ir::EventModelingEntityType::ReadModel,
        "cmd" | "command" => crate::ir::EventModelingEntityType::Command,
        "evt" | "event" => crate::ir::EventModelingEntityType::Event,
        _ => crate::ir::EventModelingEntityType::Event,
    }
}

fn parse_eventmodeling_inline_data(input: &str) -> Option<String> {
    let mut trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('`')
        && let Some(end) = trimmed[1..].find('`')
    {
        trimmed = trimmed[end + 2..].trim();
    }
    if trimmed.is_empty() {
        None
    } else {
        Some(strip_quotes(trimmed))
    }
}

// ── Cynefin parser ──────────────────────────────────────────────────────

fn parse_cynefin_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Cynefin;
    let (lines, init_config) = preprocess_input(input)?;
    let mut current_domain: Option<crate::ir::CynefinDomainName> = None;

    for line in &lines {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        if trimmed.is_empty() || lower.starts_with("cynefin-beta") {
            continue;
        }
        if lower.starts_with("acctitle") {
            let val = trimmed
                .strip_prefix("accTitle")
                .or_else(|| trimmed.strip_prefix("acctitle"))
                .unwrap_or("")
                .trim_start_matches(':')
                .trim();
            if !val.is_empty() {
                graph.acc_title = Some(val.to_string());
            }
            continue;
        }
        if lower.starts_with("accdescr") {
            let val = trimmed
                .strip_prefix("accDescr")
                .or_else(|| trimmed.strip_prefix("accdescr"))
                .unwrap_or("")
                .trim_start_matches(':')
                .trim()
                .trim_start_matches('{')
                .trim_end_matches('}')
                .trim();
            if !val.is_empty() {
                graph.acc_descr = Some(val.to_string());
            }
            continue;
        }
        if lower.starts_with("title") {
            let rest = trimmed.get(5..).unwrap_or("").trim();
            if !rest.is_empty() {
                let title = strip_quotes(rest);
                graph.diagram_title = Some(title.clone());
                graph.cynefin.title = Some(title);
            }
            continue;
        }

        if let Some(caps) = CYNEFIN_TRANSITION_RE.captures(trimmed) {
            let from = caps
                .get(1)
                .and_then(|m| parse_cynefin_domain_name(m.as_str()));
            let to = caps
                .get(2)
                .and_then(|m| parse_cynefin_domain_name(m.as_str()));
            if let (Some(from), Some(to)) = (from, to)
                && from != to
            {
                let label = caps
                    .get(3)
                    .map(|m| strip_quotes(m.as_str().trim()))
                    .filter(|label| !label.is_empty());
                graph
                    .cynefin
                    .transitions
                    .push(crate::ir::CynefinTransition { from, to, label });
            }
            current_domain = None;
            continue;
        }

        if let Some(domain_name) = parse_cynefin_domain_name(trimmed) {
            graph
                .cynefin
                .domains
                .entry(domain_name)
                .or_insert_with(|| crate::ir::CynefinDomain {
                    name: domain_name,
                    items: Vec::new(),
                });
            current_domain = Some(domain_name);
            continue;
        }

        if let Some(domain_name) = current_domain {
            let is_string_item =
                matches!(trimmed.chars().next(), Some('"') | Some('\'') | Some('`'));
            if is_string_item {
                let label = strip_quotes(trimmed);
                graph
                    .cynefin
                    .domains
                    .entry(domain_name)
                    .or_insert_with(|| crate::ir::CynefinDomain {
                        name: domain_name,
                        items: Vec::new(),
                    })
                    .items
                    .push(crate::ir::CynefinItem { label });
            }
        }
    }

    Ok(ParseOutput { graph, init_config })
}

fn parse_cynefin_domain_name(input: &str) -> Option<crate::ir::CynefinDomainName> {
    match input.trim().to_ascii_lowercase().as_str() {
        "complex" => Some(crate::ir::CynefinDomainName::Complex),
        "complicated" => Some(crate::ir::CynefinDomainName::Complicated),
        "chaotic" => Some(crate::ir::CynefinDomainName::Chaotic),
        "clear" => Some(crate::ir::CynefinDomainName::Clear),
        "confusion" => Some(crate::ir::CynefinDomainName::Confusion),
        _ => None,
    }
}

// ── Wardley parser ──────────────────────────────────────────────────────

fn parse_wardley_diagram(input: &str) -> Result<ParseOutput> {
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Wardley;
    let (lines, init_config) = preprocess_input(input)?;

    for line in &lines {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();

        if lower.starts_with("wardley") {
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }

        // Title
        if lower.starts_with("title") {
            let rest = trimmed.get(5..).unwrap_or("").trim();
            if !rest.is_empty() {
                graph.wardley.title = Some(strip_quotes(rest));
            }
            continue;
        }

        // Size: size [width, height]
        if lower.starts_with("size") {
            if let Some(coords) = extract_bracket_coords(trimmed.get(4..).unwrap_or("")) {
                graph.wardley.size = Some(coords);
            }
            continue;
        }

        // Evolution stages: evolution S1 -> S2 -> S3 -> S4
        if lower.starts_with("evolution ") {
            let rest = trimmed.get(10..).unwrap_or("").trim();
            let stages: Vec<String> = rest
                .split("->")
                .map(|s| {
                    let s = s.trim();
                    // Strip @boundary notation
                    if let Some(idx) = s.find('@') {
                        s[..idx].trim().to_string()
                    } else {
                        s.to_string()
                    }
                })
                .filter(|s| !s.is_empty())
                .collect();
            if !stages.is_empty() {
                graph.wardley.stages = stages;
            }
            continue;
        }

        // Evolve: evolve ComponentName targetValue
        if lower.starts_with("evolve ") {
            let rest = trimmed.get(7..).unwrap_or("").trim();
            let parts: Vec<&str> = rest.rsplitn(2, ' ').collect();
            if parts.len() == 2 {
                if let Ok(val) = parts[0].parse::<f32>() {
                    let target = wardley_to_percent(val);
                    graph.wardley.trends.push(crate::ir::WardleyTrend {
                        node_id: parts[1].to_string(),
                        target_evolution: target,
                    });
                }
            }
            continue;
        }

        // Note: note "text" [vis, evo]
        if lower.starts_with("note ") {
            let rest = trimmed.get(5..).unwrap_or("").trim();
            if let Some(start) = rest.find('"') {
                if let Some(end) = rest[start + 1..].find('"') {
                    let text = rest[start + 1..start + 1 + end].to_string();
                    let after = rest[start + 1 + end + 1..].trim();
                    if let Some((vis, evo)) = extract_bracket_coords(after) {
                        graph.wardley.notes.push(crate::ir::WardleyNote {
                            text,
                            x: wardley_to_percent(evo),
                            y: wardley_to_percent(vis),
                        });
                    }
                }
            }
            continue;
        }

        // Anchor: anchor Name [vis, evo]
        if lower.starts_with("anchor ") {
            let rest = trimmed.get(7..).unwrap_or("").trim();
            if let Some((name, coords_str)) = rest.split_once('[') {
                let name = name.trim().to_string();
                if let Some((vis, evo)) = extract_bracket_coords(&format!("[{}", coords_str)) {
                    graph.wardley.nodes.push(crate::ir::WardleyNode {
                        id: name.clone(),
                        label: name,
                        visibility: wardley_to_percent(vis),
                        evolution: wardley_to_percent(evo),
                        is_anchor: true,
                        label_offset: None,
                        strategy: None,
                        inertia: false,
                    });
                }
            }
            continue;
        }

        // Component: component Name [vis, evo] (optional decorators)
        if lower.starts_with("component ") {
            let rest = trimmed.get(10..).unwrap_or("").trim();
            if let Some(bracket_start) = rest.find('[') {
                let name = rest[..bracket_start].trim().to_string();
                if let Some((vis, evo)) = extract_bracket_coords(&rest[bracket_start..]) {
                    let after_bracket = rest.find(']').map(|i| rest[i + 1..].trim()).unwrap_or("");

                    // Parse optional label offset: label [dx, dy]
                    let label_offset = if let Some(li) = after_bracket.find("label") {
                        let label_rest = after_bracket[li + 5..].trim();
                        extract_bracket_coords(label_rest)
                    } else {
                        None
                    };

                    // Parse decorators
                    let strategy = if after_bracket.contains("(build)") {
                        Some(crate::ir::WardleyStrategy::Build)
                    } else if after_bracket.contains("(buy)") {
                        Some(crate::ir::WardleyStrategy::Buy)
                    } else if after_bracket.contains("(outsource)") {
                        Some(crate::ir::WardleyStrategy::Outsource)
                    } else if after_bracket.contains("(market)") {
                        Some(crate::ir::WardleyStrategy::Market)
                    } else {
                        None
                    };
                    let inertia = after_bracket.contains("(inertia)");

                    graph.wardley.nodes.push(crate::ir::WardleyNode {
                        id: name.clone(),
                        label: name,
                        visibility: wardley_to_percent(vis),
                        evolution: wardley_to_percent(evo),
                        is_anchor: false,
                        label_offset,
                        strategy,
                        inertia,
                    });
                }
            }
            continue;
        }

        // Links: A -> B, A +> B, A -.-> B, A -> B; label
        if trimmed.contains("->") || trimmed.contains("+>") || trimmed.contains("+<") {
            let (line_part, label) = if let Some(idx) = trimmed.find(';') {
                (
                    trimmed[..idx].trim(),
                    Some(trimmed[idx + 1..].trim().to_string()),
                )
            } else {
                (trimmed, None)
            };

            let dashed = line_part.contains("-.->");
            let flow = if line_part.contains("+<>") {
                Some(crate::ir::WardleyFlow::Bidirectional)
            } else if line_part.contains("+>") {
                Some(crate::ir::WardleyFlow::Forward)
            } else if line_part.contains("+<") {
                Some(crate::ir::WardleyFlow::Backward)
            } else {
                None
            };

            // Extract source and target
            let separator = if dashed {
                "-.->"
            } else if line_part.contains("+<>") {
                "+<>"
            } else if line_part.contains("+>") {
                "+>"
            } else if line_part.contains("+<") {
                "+<"
            } else {
                "->"
            };

            let parts: Vec<&str> = line_part.splitn(2, separator).collect();
            if parts.len() == 2 {
                let source = parts[0].trim().to_string();
                let target = parts[1].trim().to_string();
                if !source.is_empty() && !target.is_empty() {
                    graph.wardley.links.push(crate::ir::WardleyLink {
                        source,
                        target,
                        dashed,
                        label,
                        flow,
                    });
                }
            }
            continue;
        }
    }

    // Default stages if none specified
    if graph.wardley.stages.is_empty() {
        graph.wardley.stages = vec![
            "Genesis".to_string(),
            "Custom Built".to_string(),
            "Product".to_string(),
            "Commodity".to_string(),
        ];
    }

    Ok(ParseOutput { graph, init_config })
}

fn wardley_to_percent(val: f32) -> f32 {
    if val <= 1.0 {
        val * 100.0
    } else {
        val.clamp(0.0, 100.0)
    }
}

fn extract_bracket_coords(s: &str) -> Option<(f32, f32)> {
    let s = s.trim();
    let start = s.find('[')?;
    let end = s.find(']')?;
    let inner = &s[start + 1..end];
    let parts: Vec<&str> = inner.split(',').collect();
    if parts.len() == 2 {
        let a = parts[0].trim().parse::<f32>().ok()?;
        let b = parts[1].trim().parse::<f32>().ok()?;
        Some((a, b))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::DiagramKind;

    #[test]
    fn split_on_ampersand_plain() {
        assert_eq!(split_on_ampersand("A & B & C"), vec!["A", "B", "C"]);
    }

    #[test]
    fn split_on_ampersand_preserves_label_ampersand() {
        let parts = split_on_ampersand(r#"A["foo & bar"]"#);
        assert_eq!(parts, vec![r#"A["foo & bar"]"#]);
    }

    #[test]
    fn split_on_ampersand_mixed() {
        let parts = split_on_ampersand(r#"A["foo & bar"] & B"#);
        assert_eq!(parts, vec![r#"A["foo & bar"]"#, "B"]);
    }

    #[test]
    fn parse_ampersand_in_node_label_not_split() {
        let input = r#"flowchart LR
A["reads artifacts & computes deps"] --> B"#;
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(
            parsed.graph.nodes.len(),
            2,
            "ampersand in label must not create extra nodes"
        );
        assert_eq!(parsed.graph.edges.len(), 1);
        assert!(parsed.graph.nodes.contains_key("A"));
        assert!(parsed.graph.nodes.contains_key("B"));
        assert_eq!(
            parsed.graph.nodes["A"].label,
            "reads artifacts & computes deps"
        );
    }

    #[test]
    fn parse_parallel_ampersand_with_label_ampersand() {
        let input = r#"flowchart LR
A["foo & bar"] & B --> C"#;
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 2, "two parallel edges expected");
        assert_eq!(parsed.graph.nodes.len(), 3);
        assert_eq!(parsed.graph.nodes["A"].label, "foo & bar");
    }

    #[test]
    fn parse_simple_flowchart() {
        let input = "flowchart lr\nA[Start] -->|go| B(End)";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.nodes.len(), 2);
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("go"));
        assert_eq!(parsed.graph.direction, Direction::LeftRight);
        assert_eq!(
            parsed.graph.nodes.get("B").unwrap().shape,
            crate::ir::NodeShape::RoundRect
        );
    }

    #[test]
    fn parse_subgraph() {
        let input = "flowchart TD\nsubgraph Group[\"My Group\"]\nA --> B\nend";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.subgraphs.len(), 1);
        let sg = &parsed.graph.subgraphs[0];
        assert_eq!(sg.label, "My Group");
        assert_eq!(sg.nodes.len(), 2);
    }

    #[test]
    fn parse_node_label_with_dash_dash_inside_subgraph() {
        let input = "flowchart TB\nsubgraph PT[\"Teardown\"]\nTD[\"kill processes;<br/>(--leave-up skips this)\"]\nend";
        let parsed = parse_mermaid(input).unwrap();

        assert!(parsed.graph.nodes.contains_key("TD"));
        assert_eq!(
            parsed.graph.nodes["TD"].label,
            "kill processes;<br/>(--leave-up skips this)"
        );
        assert_eq!(parsed.graph.subgraphs[0].nodes, vec!["TD"]);
    }

    #[test]
    fn parse_multiline_flowchart_node_label() {
        let input = "flowchart TB\nsubgraph P7[\"Phase 7\"]\nMD[\"MetaDeployment hello-fed<br/>vcs: [vc-a1,\n  vc-b1]<br/>deploy: hello-world\"]\nend";
        let parsed = parse_mermaid(input).unwrap();

        assert!(parsed.graph.nodes.contains_key("MD"));
        assert_eq!(parsed.graph.subgraphs[0].nodes, vec!["MD"]);
        assert!(parsed.graph.nodes["MD"].label.contains("vc-a1,\nvc-b1"));
        assert!(!parsed.graph.nodes.contains_key("vc"));
        assert!(!parsed.graph.nodes.contains_key("world"));
    }

    #[test]
    fn parse_nested_subgraphs() {
        let input = "flowchart LR\nsubgraph Outer\n  subgraph Inner\n    A --> B\n  end\nend";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.subgraphs.len(), 2);
        let outer = &parsed.graph.subgraphs[0];
        let inner = &parsed.graph.subgraphs[1];
        assert!(outer.nodes.contains(&"A".to_string()));
        assert!(outer.nodes.contains(&"B".to_string()));
        assert!(inner.nodes.contains(&"A".to_string()));
        assert!(inner.nodes.contains(&"B".to_string()));
    }

    #[test]
    fn parse_edge_styles() {
        let input = "flowchart LR\nA -.-> B\nC ==> D\nE <--> F\nG --- H\nlinkStyle 0 stroke:#0ff,stroke-width:2,color:#f00";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 4);
        assert_eq!(parsed.graph.edges[0].style, crate::ir::EdgeStyle::Dotted);
        assert_eq!(parsed.graph.edges[1].style, crate::ir::EdgeStyle::Thick);
        assert_eq!(parsed.graph.edges[2].arrow_start, true);
        assert_eq!(parsed.graph.edges[2].arrow_end, true);
        assert_eq!(parsed.graph.edges[3].directed, false);
        let style = parsed.graph.edge_styles.get(&0).unwrap();
        assert_eq!(style.label_color.as_deref(), Some("#f00"));
    }

    #[test]
    fn parse_invisible_flowchart_edge() {
        let input = "flowchart TD\nA~~~B";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].from, "A");
        assert_eq!(parsed.graph.edges[0].to, "B");
        assert_eq!(parsed.graph.edges[0].style, crate::ir::EdgeStyle::Invisible);
    }

    #[test]
    fn parse_class_and_styles() {
        let input = "flowchart LR\nclassDef hot fill:#f00,stroke:#000,color:#fff,stroke-width:2\nA[One]\nclass A hot\nstyle A fill:#0f0,stroke:#00f,stroke-width:3,color:#111\nA --> B\nlinkStyle 0 stroke:#0ff,stroke-width:4,stroke-dasharray:5 5";
        let parsed = parse_mermaid(input).unwrap();
        assert!(parsed.graph.class_defs.contains_key("hot"));
        assert!(parsed.graph.node_classes.contains_key("A"));
        assert!(parsed.graph.node_styles.contains_key("A"));
        assert!(parsed.graph.edge_styles.contains_key(&0));
        let edge_style = parsed.graph.edge_styles.get(&0).unwrap();
        assert_eq!(edge_style.stroke.as_deref(), Some("#0ff"));
    }

    #[test]
    fn parse_inline_class_and_linkstyle_default() {
        let input = "flowchart LR\nclassDef hot fill:#f00\nA[Alpha]:::hot --> B\nB --> C\nlinkStyle default stroke:#0ff,stroke-width:3\nlinkStyle 1 stroke:#00f";
        let parsed = parse_mermaid(input).unwrap();
        let classes = parsed
            .graph
            .node_classes
            .get("A")
            .cloned()
            .unwrap_or_default();
        assert!(classes.iter().any(|c| c == "hot"));
        assert!(parsed.graph.edge_style_default.is_some());
        let edge_style = parsed.graph.edge_styles.get(&1).unwrap();
        assert_eq!(edge_style.stroke.as_deref(), Some("#00f"));
    }

    #[test]
    fn parse_edge_label_in_arrow() {
        let input = "flowchart LR\nA -- needs review --> B\nC --|ship it|--> D";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 2);
        assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("needs review"));
        assert_eq!(parsed.graph.edges[1].label.as_deref(), Some("ship it"));
    }

    #[test]
    fn parse_html_edge_label_in_arrow_does_not_create_phantom_node() {
        let input = "flowchart LR\nod>Odd shape]-- Two line<br/>edge comment --> ro";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].from, "od");
        assert_eq!(parsed.graph.edges[0].to, "ro");
        assert_eq!(
            parsed.graph.edges[0].label.as_deref(),
            Some("Two line<br/>edge comment")
        );
        assert!(!parsed.graph.nodes.contains_key("Two"));
        assert!(!parsed.graph.nodes.contains_key("line"));
    }

    #[test]
    fn parse_compact_dotted_edge_label_without_spaces() {
        let input = "flowchart LR\nN01 -.audit.-> N16";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("audit"));
        assert_eq!(parsed.graph.edges[0].style, crate::ir::EdgeStyle::Dotted);
        assert!(parsed.graph.edges[0].arrow_end);
        assert!(parsed.graph.nodes.contains_key("N01"));
        assert!(parsed.graph.nodes.contains_key("N16"));
        assert!(!parsed.graph.nodes.contains_key(".audit"));
    }

    #[test]
    fn parse_compact_dotted_edge_label_with_dotted_ids() {
        let input = "flowchart LR\nsvc.api -.db-sync.-> db.main";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("db-sync"));
        assert!(parsed.graph.nodes.contains_key("svc.api"));
        assert!(parsed.graph.nodes.contains_key("db.main"));
        assert!(!parsed.graph.nodes.contains_key(".db-sync"));
    }

    #[test]
    fn parse_spaced_dotted_edge_label_with_double_dash_text() {
        let input = r#"flowchart LR
kc1["~/.kube/fed0cluster1-kmaster1"] -. import --overwrite .-> c1"#;
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(
            parsed.graph.edges[0].label.as_deref(),
            Some("import --overwrite")
        );
        assert_eq!(parsed.graph.edges[0].from, "kc1");
        assert_eq!(parsed.graph.edges[0].to, "c1");
        assert!(parsed.graph.nodes.contains_key("kc1"));
        assert!(parsed.graph.nodes.contains_key("c1"));
        assert!(!parsed.graph.nodes.contains_key("verwrite"));
    }

    #[test]
    fn parse_flowchart_chain_with_apostrophes_in_node_label() {
        let input = r#"flowchart TB
b1[republish builds fresh VCLocation<br/>vclock=&#123;&#125;] --> b2[peer's MergeReplicated:<br/>'older than what I have'] --> b3[rejected - foreign view stays stale]"#;
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 2);
        assert_eq!(parsed.graph.edges[0].from, "b1");
        assert_eq!(parsed.graph.edges[0].to, "b2");
        assert_eq!(parsed.graph.edges[1].from, "b2");
        assert_eq!(parsed.graph.edges[1].to, "b3");
        assert_eq!(
            parsed.graph.nodes["b2"].label,
            "peer's MergeReplicated:<br/>'older than what I have'"
        );
    }

    #[test]
    fn parse_pipe_edge_label() {
        let input = "flowchart LR\nA -->|yes| B";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("yes"));
    }

    #[test]
    fn parse_pipe_edge_label_with_hyphen_does_not_create_phantom_nodes() {
        let input = "flowchart LR\nC3 -->|high-risk order| D2";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(
            parsed.graph.edges[0].label.as_deref(),
            Some("high-risk order")
        );
        assert!(parsed.graph.nodes.contains_key("C3"));
        assert!(parsed.graph.nodes.contains_key("D2"));
        assert!(!parsed.graph.nodes.contains_key("risk"));
        assert!(!parsed.graph.nodes.contains_key("|high"));
    }

    #[test]
    fn parse_quoted_inline_edge_label() {
        let input = "flowchart LR\n  A[Node 1] -- \"Some text\" --> B[Node 2]";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("Some text"));
        assert!(parsed.graph.nodes.contains_key("A"));
        assert!(parsed.graph.nodes.contains_key("B"));
    }

    #[test]
    fn parse_multi_target_edges() {
        let input = "flowchart LR\nA --> B & C";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 2);
        assert!(parsed.graph.nodes.contains_key("B"));
        assert!(parsed.graph.nodes.contains_key("C"));
    }

    #[test]
    fn parse_multi_source_edges() {
        let input = "flowchart LR\nA & B --> C";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 2);
        assert!(parsed.graph.nodes.contains_key("A"));
        assert!(parsed.graph.nodes.contains_key("B"));
        assert!(parsed.graph.nodes.contains_key("C"));
    }

    #[test]
    fn parse_subgraph_style() {
        let input = "flowchart LR\nclassDef hot fill:#f00,stroke:#0f0\nsubgraph SG[Group]:::hot\nA --> B\nend\nclass SG hot\nstyle SG fill:#faf,stroke:#111";
        let parsed = parse_mermaid(input).unwrap();
        let style = parsed.graph.subgraph_styles.get("SG").unwrap();
        assert_eq!(style.fill.as_deref(), Some("#faf"));
        assert_eq!(style.stroke.as_deref(), Some("#111"));
        let classes = parsed.graph.subgraph_classes.get("SG").unwrap();
        assert!(classes.iter().any(|c| c == "hot"));
    }

    #[test]
    fn parse_semicolon_statements() {
        let input = "flowchart LR; A --> B; B --> C";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 2);
    }

    #[test]
    fn parse_subgraph_single_token_id() {
        let input = "flowchart LR\nsubgraph Alpha\nA --> B\nend\nstyle Alpha fill:#fff";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.subgraphs.len(), 1);
        assert_eq!(parsed.graph.subgraphs[0].id.as_deref(), Some("Alpha"));
        assert!(parsed.graph.subgraph_styles.contains_key("Alpha"));
    }

    #[test]
    fn parse_style_multiple_nodes() {
        let input = "flowchart LR\nA-->B\nstyle A,B fill:#f00";
        let parsed = parse_mermaid(input).unwrap();
        assert!(parsed.graph.node_styles.contains_key("A"));
        assert!(parsed.graph.node_styles.contains_key("B"));
    }

    #[test]
    fn parse_edge_decorations() {
        let input = "flowchart LR\nA o--o B\nC x--> D";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 2);
        assert_eq!(
            parsed.graph.edges[0].start_decoration,
            Some(crate::ir::EdgeDecoration::Circle)
        );
        assert_eq!(
            parsed.graph.edges[0].end_decoration,
            Some(crate::ir::EdgeDecoration::Circle)
        );
        assert_eq!(
            parsed.graph.edges[1].start_decoration,
            Some(crate::ir::EdgeDecoration::Cross)
        );
        assert!(parsed.graph.edges[1].arrow_end);
    }

    #[test]
    fn parse_class_diagram_basic() {
        let input = "classDiagram\nclass Animal {\n+String name\n+eat()\n}\nclass Dog\nAnimal <|-- Dog : inherits";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Class);
        assert!(parsed.graph.nodes.contains_key("Animal"));
        assert!(parsed.graph.nodes.contains_key("Dog"));
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("inherits"));
        let label = &parsed.graph.nodes.get("Animal").unwrap().label;
        assert!(label.contains("Animal"));
        assert!(label.contains("name"));
    }

    #[test]
    fn parse_class_declarations_with_bracket_labels_do_not_create_extra_nodes() {
        let input = "classDiagram\nclass Animal[\"Animal with a label\"]\nclass Car[\"Car with *! symbols\"]\nAnimal --> Car";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(parsed.graph.nodes.len(), 2);
        assert!(parsed.graph.nodes.contains_key("Animal"));
        assert!(parsed.graph.nodes.contains_key("Car"));
        assert!(
            !parsed
                .graph
                .nodes
                .contains_key("Animal[\"Animal with a label\"]")
        );
        assert!(
            !parsed
                .graph
                .nodes
                .contains_key("Car[\"Car with *! symbols\"]")
        );

        assert_eq!(
            parsed.graph.nodes.get("Animal").unwrap().label,
            "Animal with a label\n---\n---"
        );
        assert_eq!(
            parsed.graph.nodes.get("Car").unwrap().label,
            "Car with *! symbols\n---\n---"
        );
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].from, "Animal");
        assert_eq!(parsed.graph.edges[0].to, "Car");
    }

    #[test]
    fn parse_class_declaration_label_with_inline_class() {
        let input =
            "classDiagram\nclassDef hot fill:#f00\nclass C1[\"Class 1 with text label\"]:::hot";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(
            parsed.graph.nodes.get("C1").unwrap().label,
            "Class 1 with text label\n---\n---"
        );
        let classes = parsed.graph.node_classes.get("C1").unwrap();
        assert!(classes.iter().any(|class_name| class_name == "hot"));
    }

    #[test]
    fn parse_class_inline_annotation_uses_annotation_row() {
        let input = "classDiagram\nclass Shape <<interface>>";
        let parsed = parse_mermaid(input).unwrap();

        assert!(parsed.graph.nodes.contains_key("Shape"));
        assert!(!parsed.graph.nodes.contains_key("Shape <<interface>>"));
        assert_eq!(
            parsed.graph.nodes.get("Shape").unwrap().label,
            "\u{00ab}interface\u{00bb}\nShape\n---\n---"
        );
    }

    #[test]
    fn parse_class_separate_annotation_line_uses_annotation_row() {
        let input = "classDiagram\nclass Shape\n<<interface>> Shape";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(
            parsed.graph.nodes.get("Shape").unwrap().label,
            "\u{00ab}interface\u{00bb}\nShape\n---\n---"
        );
    }

    #[test]
    fn parse_backticked_class_names_share_relation_ids() {
        let input = "classDiagram\nclass `Animal Class!`\nclass `Car Class`\n`Animal Class!` --> `Car Class`";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(parsed.graph.nodes.len(), 2);
        assert!(parsed.graph.nodes.contains_key("Animal Class!"));
        assert!(parsed.graph.nodes.contains_key("Car Class"));
        assert!(!parsed.graph.nodes.contains_key("`Animal Class!`"));
        assert!(!parsed.graph.nodes.contains_key("`Car Class`"));
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].from, "Animal Class!");
        assert_eq!(parsed.graph.edges[0].to, "Car Class");
    }

    #[test]
    fn parse_empty_class_keeps_compartment_dividers() {
        let input = "classDiagram\nclass Empty";
        let parsed = parse_mermaid(input).unwrap();
        let label = &parsed.graph.nodes.get("Empty").unwrap().label;
        assert_eq!(label, "Empty\n---\n---");
    }

    #[test]
    fn parse_class_generic_types_use_mermaid_display_syntax() {
        let input = r#"classDiagram
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
        let parsed = parse_mermaid(input).unwrap();
        assert!(!parsed.graph.nodes.contains_key("Square~Shape~"));
        let label = parsed.graph.nodes.get("Square").unwrap().label.as_str();

        assert!(label.contains("Square<Shape>"));
        assert!(label.contains("List<int> position"));
        assert!(label.contains("setPoints(List<int> points)"));
        assert!(label.contains("getPoints() : List<int>"));
        assert!(label.contains("-List<string> messages"));
        assert!(label.contains("+setMessages(List<string> messages)"));
        assert!(label.contains("+getMessages() : List<string>"));
        assert!(label.contains("+getDistanceMatrix() : List<List<int>>"));
    }

    #[test]
    fn parse_class_notes_as_note_nodes() {
        let input = "classDiagram\nnote \"This is a general note\"\nnote for MyClass \"This is a note for a class\"\nclass MyClass";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(
            parsed.graph.nodes.get("note0").unwrap().shape,
            crate::ir::NodeShape::Note
        );
        assert_eq!(
            parsed.graph.nodes.get("note0").unwrap().label,
            "This is a general note"
        );
        assert_eq!(
            parsed.graph.nodes.get("note1").unwrap().shape,
            crate::ir::NodeShape::Note
        );
        assert_eq!(
            parsed.graph.nodes.get("note1").unwrap().label,
            "This is a note for a class"
        );

        let edge = parsed
            .graph
            .edges
            .iter()
            .find(|edge| edge.id.as_deref() == Some("edgeNote1"))
            .expect("note connector edge");
        assert_eq!(edge.from, "note1");
        assert_eq!(edge.to, "MyClass");
        assert_eq!(edge.style, crate::ir::EdgeStyle::Dotted);
    }

    #[test]
    fn parse_class_style_lines_apply_node_styles() {
        let input = "classDiagram\nclass Animal\nclass Mineral\nstyle Animal fill:#f9f,stroke:#333,stroke-width:4px\nstyle Mineral fill:#bbf,stroke:#f66,stroke-width:2px,color:#fff,stroke-dasharray: 5 5";
        let parsed = parse_mermaid(input).unwrap();

        let animal = parsed.graph.node_styles.get("Animal").unwrap();
        assert_eq!(animal.fill.as_deref(), Some("#f9f"));
        assert_eq!(animal.stroke.as_deref(), Some("#333"));
        assert_eq!(animal.stroke_width, Some(4.0));

        let mineral = parsed.graph.node_styles.get("Mineral").unwrap();
        assert_eq!(mineral.fill.as_deref(), Some("#bbf"));
        assert_eq!(mineral.stroke.as_deref(), Some("#f66"));
        assert_eq!(mineral.stroke_width, Some(2.0));
        assert_eq!(mineral.text_color.as_deref(), Some("#fff"));
        assert_eq!(mineral.stroke_dasharray.as_deref(), Some("5 5"));
    }

    #[test]
    fn parse_class_two_way_extension_relation_keeps_both_markers() {
        let input = "classDiagram\nAnimal <|--|> Zebra";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 1);
        let edge = &parsed.graph.edges[0];
        assert_eq!(edge.from, "Animal");
        assert_eq!(edge.to, "Zebra");
        assert!(edge.arrow_start);
        assert!(edge.arrow_end);
        assert_eq!(
            edge.arrow_start_kind,
            Some(crate::ir::EdgeArrowhead::OpenTriangle)
        );
        assert_eq!(
            edge.arrow_end_kind,
            Some(crate::ir::EdgeArrowhead::OpenTriangle)
        );
    }

    #[test]
    fn parse_class_relation_multiplicity() {
        let input = "classDiagram\nClass01 \"1\" *-- \"many\" Class02 : contains";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 1);
        let edge = &parsed.graph.edges[0];
        assert_eq!(edge.start_label.as_deref(), Some("1"));
        assert_eq!(edge.end_label.as_deref(), Some("many"));
        assert_eq!(edge.label.as_deref(), Some("contains"));
    }

    #[test]
    fn parse_class_lollipop_relations_create_interface_nodes() {
        let input = "classDiagram\nbar ()-- foo\nClass01 --() bar";
        let parsed = parse_mermaid(input).unwrap();

        assert!(parsed.graph.nodes.contains_key("foo"));
        assert!(parsed.graph.nodes.contains_key("Class01"));
        assert!(!parsed.graph.nodes.contains_key("bar ()"));

        let interface0 = parsed.graph.nodes.get("interface0").unwrap();
        assert_eq!(interface0.label, "bar");
        assert_eq!(interface0.shape, crate::ir::NodeShape::Text);
        let interface1 = parsed.graph.nodes.get("interface1").unwrap();
        assert_eq!(interface1.label, "bar");
        assert_eq!(interface1.shape, crate::ir::NodeShape::Text);

        assert_eq!(parsed.graph.edges.len(), 2);
        let start = &parsed.graph.edges[0];
        assert_eq!(start.from, "interface0");
        assert_eq!(start.to, "foo");
        assert_eq!(
            start.start_decoration,
            Some(crate::ir::EdgeDecoration::Lollipop)
        );
        assert_eq!(start.end_decoration, None);

        let end = &parsed.graph.edges[1];
        assert_eq!(end.from, "Class01");
        assert_eq!(end.to, "interface1");
        assert_eq!(end.start_decoration, None);
        assert_eq!(
            end.end_decoration,
            Some(crate::ir::EdgeDecoration::Lollipop)
        );
    }

    #[test]
    fn parse_class_lollipop_complex_fixture_uses_text_interfaces() {
        let input = "classDiagram
    class Class01 {
        int amount
        draw()
    }
    Class01 --() bar
    Class02 --() bar

    foo ()-- Class01";
        let parsed = parse_mermaid(input).unwrap();

        assert!(parsed.graph.nodes.contains_key("Class01"));
        assert!(parsed.graph.nodes.contains_key("Class02"));
        assert!(!parsed.graph.nodes.contains_key("() bar"));
        assert!(!parsed.graph.nodes.contains_key("foo ()"));

        let interface0 = parsed.graph.nodes.get("interface0").unwrap();
        assert_eq!(interface0.label, "bar");
        assert_eq!(interface0.shape, crate::ir::NodeShape::Text);
        let interface1 = parsed.graph.nodes.get("interface1").unwrap();
        assert_eq!(interface1.label, "bar");
        assert_eq!(interface1.shape, crate::ir::NodeShape::Text);
        let interface2 = parsed.graph.nodes.get("interface2").unwrap();
        assert_eq!(interface2.label, "foo");
        assert_eq!(interface2.shape, crate::ir::NodeShape::Text);

        assert_eq!(parsed.graph.edges.len(), 3);
        assert_eq!(parsed.graph.edges[0].from, "Class01");
        assert_eq!(parsed.graph.edges[0].to, "interface0");
        assert_eq!(
            parsed.graph.edges[0].end_decoration,
            Some(crate::ir::EdgeDecoration::Lollipop)
        );
        assert_eq!(parsed.graph.edges[1].from, "Class02");
        assert_eq!(parsed.graph.edges[1].to, "interface1");
        assert_eq!(
            parsed.graph.edges[1].end_decoration,
            Some(crate::ir::EdgeDecoration::Lollipop)
        );
        assert_eq!(parsed.graph.edges[2].from, "interface2");
        assert_eq!(parsed.graph.edges[2].to, "Class01");
        assert_eq!(
            parsed.graph.edges[2].start_decoration,
            Some(crate::ir::EdgeDecoration::Lollipop)
        );
    }

    #[test]
    fn parse_er_diagram_basic() {
        let input =
            "erDiagram\nCUSTOMER ||--o{ ORDER : places\nCUSTOMER {\nstring id\nstring name\n}";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Er);
        assert_eq!(parsed.graph.edges.len(), 1);
        let edge = &parsed.graph.edges[0];
        assert_eq!(edge.label.as_deref(), Some("places"));
        // ER diagrams use crow's foot decorations, not text labels
        assert_eq!(edge.start_label, None);
        assert_eq!(edge.end_label, None);
        assert_eq!(
            edge.start_decoration,
            Some(crate::ir::EdgeDecoration::CrowsFootOne)
        );
        assert_eq!(
            edge.end_decoration,
            Some(crate::ir::EdgeDecoration::CrowsFootZeroMany)
        );
        let customer = parsed.graph.nodes.get("CUSTOMER").unwrap();
        assert!(customer.label.contains("CUSTOMER"));
        assert!(customer.label.contains("string id"));
    }

    #[test]
    fn parse_er_styling_lines_and_inline_classes_do_not_create_entities() {
        let input = "erDiagram\nPERSON:::foo ||--|| HOUSE:::bar : has\nclassDef foo stroke:#f00\nclassDef bar stroke:#0f0\nstyle HOUSE fill:#bbf";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(parsed.graph.nodes.len(), 2);
        assert!(parsed.graph.nodes.contains_key("PERSON"));
        assert!(parsed.graph.nodes.contains_key("HOUSE"));
        assert!(!parsed.graph.nodes.contains_key("classDef foo stroke:#f00"));
        assert!(!parsed.graph.nodes.contains_key("style HOUSE fill:#bbf"));

        let person_classes = parsed.graph.node_classes.get("PERSON").unwrap();
        assert_eq!(person_classes.first().map(String::as_str), Some("default"));
        assert!(person_classes.iter().any(|class_name| class_name == "foo"));
        let house_classes = parsed.graph.node_classes.get("HOUSE").unwrap();
        assert!(house_classes.iter().any(|class_name| class_name == "bar"));
        assert_eq!(
            parsed
                .graph
                .node_styles
                .get("HOUSE")
                .and_then(|style| style.fill.as_deref()),
            Some("#bbf")
        );
    }

    #[test]
    fn parse_er_entity_aliases_share_relation_ids() {
        let input = "erDiagram\np[Person] {\nstring firstName\n}\na[\"Customer Account\"] {\nstring email\n}\np ||--o| a : has";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(parsed.graph.nodes.len(), 2);
        assert!(parsed.graph.nodes.contains_key("p"));
        assert!(parsed.graph.nodes.contains_key("a"));
        assert!(!parsed.graph.nodes.contains_key("p[Person]"));
        assert_eq!(
            parsed.graph.nodes.get("p").unwrap().label,
            "Person\n---\nstring firstName"
        );
        assert_eq!(
            parsed.graph.nodes.get("a").unwrap().label,
            "Customer Account\n---\nstring email"
        );
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].from, "p");
        assert_eq!(parsed.graph.edges[0].to, "a");
    }

    #[test]
    fn parse_er_word_cardinality_relationships() {
        let input = "erDiagram\nCAR 1 to zero or more NAMED-DRIVER : allows\nPERSON many(0) optionally to 0+ NAMED-DRIVER : is";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(parsed.graph.nodes.len(), 3);
        assert_eq!(parsed.graph.edges.len(), 2);
        assert_eq!(parsed.graph.edges[0].from, "CAR");
        assert_eq!(parsed.graph.edges[0].to, "NAMED-DRIVER");
        assert_eq!(parsed.graph.edges[0].style, crate::ir::EdgeStyle::Solid);
        assert_eq!(
            parsed.graph.edges[0].start_decoration,
            Some(crate::ir::EdgeDecoration::CrowsFootOne)
        );
        assert_eq!(
            parsed.graph.edges[0].end_decoration,
            Some(crate::ir::EdgeDecoration::CrowsFootZeroMany)
        );
        assert_eq!(parsed.graph.edges[1].style, crate::ir::EdgeStyle::Dotted);
        assert_eq!(
            parsed.graph.edges[1].start_decoration,
            Some(crate::ir::EdgeDecoration::CrowsFootZeroMany)
        );
        assert_eq!(
            parsed.graph.edges[1].end_decoration,
            Some(crate::ir::EdgeDecoration::CrowsFootZeroMany)
        );
    }

    #[test]
    fn parse_er_frontmatter_title_with_config() {
        let input = "---\ntitle: Order example\nconfig:\n  layout: elk\n---\nerDiagram\nCUSTOMER ||--o{ ORDER : places";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(parsed.graph.diagram_title.as_deref(), Some("Order example"));
        assert!(parsed.init_config.is_some());
    }

    #[test]
    fn parse_pie_diagram_basic() {
        let input = "pie showData\n  title Pets\n  \"Dogs\" : 10\n  Cats : 5";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Pie);
        assert!(parsed.graph.pie_show_data);
        assert_eq!(parsed.graph.pie_title.as_deref(), Some("Pets"));
        assert_eq!(parsed.graph.pie_slices.len(), 2);
        assert_eq!(parsed.graph.pie_slices[0].label, "Dogs");
        assert_eq!(parsed.graph.pie_slices[0].value, 10.0);
    }

    #[test]
    fn parse_mindmap_basic() {
        let input = "mindmap\n  root((Root))\n    Child A\n    Child B\n      Grandchild";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Mindmap);
        assert!(parsed.graph.nodes.len() >= 4);
        assert_eq!(parsed.graph.edges.len(), 3);
    }

    #[test]
    fn parse_journey_basic() {
        let input = "journey\n  title My Journey\n  section Start\n    Step one: 5: Alice\n    Step two: 3: Alice, Bob";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Journey);
        assert_eq!(parsed.graph.journey_title.as_deref(), Some("My Journey"));
        assert_eq!(parsed.graph.subgraphs.len(), 1);
        assert_eq!(parsed.graph.nodes.len(), 2);
        assert_eq!(parsed.graph.edges.len(), 1);
        let node = parsed.graph.nodes.get("journey_0").unwrap();
        assert_eq!(node.value, Some(5.0));
        assert!(node.label.contains("Step one"));
        assert!(node.label.contains("Alice"));
        assert!(!node.label.contains("score:"));
    }

    #[test]
    fn parse_journey_accessibility_directives() {
        let input = "journey\n  accTitle: My daily workflow diagram\n  accDescr: A user journey showing the steps in a typical work day\n  title My Daily Workflow\n  section Morning\n    Check email: 3: Me";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Journey);
        assert_eq!(
            parsed.graph.acc_title.as_deref(),
            Some("My daily workflow diagram")
        );
        assert_eq!(
            parsed.graph.acc_descr.as_deref(),
            Some("A user journey showing the steps in a typical work day")
        );
        assert_eq!(parsed.graph.nodes.len(), 1);
        assert_eq!(parsed.graph.subgraphs.len(), 1);
    }

    #[test]
    fn parse_journey_multiline_accessible_description() {
        let input = "journey\n  accTitle: Customer onboarding journey\n  accDescr {\n    This diagram shows the complete\n    customer onboarding process.\n  }\n  title Customer Onboarding\n  section Signup\n    Create account: 4: Customer";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Journey);
        assert_eq!(
            parsed.graph.acc_descr.as_deref(),
            Some("This diagram shows the complete\ncustomer onboarding process.")
        );
        assert_eq!(parsed.graph.nodes.len(), 1);
        assert_eq!(parsed.graph.subgraphs.len(), 1);
    }

    #[test]
    fn parse_timeline_basic() {
        let input = "timeline\n  title History\n  2020 : Launch\n  2021 : Growth";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Timeline);
        assert_eq!(parsed.graph.timeline.events.len(), 2);
        assert_eq!(parsed.graph.timeline.title.as_deref(), Some("History"));
        assert_eq!(parsed.graph.timeline.events[0].time, "2020");
        assert_eq!(parsed.graph.timeline.events[0].events, vec!["Launch"]);
    }

    #[test]
    fn parse_gantt_basic() {
        let input = "gantt\n  title Plan\n  section Alpha\n  Task A : done, a1, 2020-01-01, 5d\n  Task B : after a1, 3d";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Gantt);
        assert!(parsed.graph.nodes.len() >= 2);
        assert_eq!(parsed.graph.edges.len(), 1);
    }

    #[test]
    fn parse_requirement_basic() {
        let input = "requirementDiagram\n  requirement req1 {\n    id: 1\n    text: Login\n  }\n  requirement req2\n  req1 - satisfies -> req2";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Requirement);
        assert_eq!(parsed.graph.nodes.len(), 2);
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("satisfies"));
        assert_eq!(parsed.graph.edges[0].style, crate::ir::EdgeStyle::Dotted);
        assert!(parsed.graph.edges[0].arrow_end);
        assert!(!parsed.graph.edges[0].arrow_start);
    }

    #[test]
    fn parse_requirement_contains_relation_uses_start_marker() {
        let input =
            "requirementDiagram\n  requirement req1\n  requirement req2\n  req1 - contains -> req2";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("contains"));
        assert_eq!(parsed.graph.edges[0].style, crate::ir::EdgeStyle::Solid);
        assert!(parsed.graph.edges[0].arrow_start);
        assert!(!parsed.graph.edges[0].arrow_end);
    }

    #[test]
    fn parse_requirement_left_arrow_relation_reverses_direction() {
        let input = "requirementDiagram\n  requirement req\n  element elem\n  req <- copies - elem";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(parsed.graph.kind, DiagramKind::Requirement);
        assert_eq!(parsed.graph.nodes.len(), 2);
        assert!(!parsed.graph.nodes.contains_key("<-"));
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].from, "elem");
        assert_eq!(parsed.graph.edges[0].to, "req");
        assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("copies"));
    }

    #[test]
    fn parse_requirement_labels_match_mermaid_names() {
        let input = "requirementDiagram\n  functionalRequirement req2 {\n    id: 1.1\n    text: Login\n    risk: high\n    verifymethod: test\n  }\n  element test_entity {\n    type: \"test suite\"\n    docRef: github.com/all_the_tests\n  }";
        let parsed = parse_mermaid(input).unwrap();

        let req = parsed.graph.nodes.get("req2").unwrap();
        assert!(req.label.contains("<<Functional Requirement>>"));
        assert!(req.label.contains("Risk: High"));
        assert!(req.label.contains("Verification: Test"));

        let elem = parsed.graph.nodes.get("test_entity").unwrap();
        assert!(elem.label.contains("Type: test suite"));
        assert!(elem.label.contains("Doc Ref: github.com/all_the_tests"));
    }

    #[test]
    fn parse_requirement_styling_directives_do_not_create_nodes() {
        let input = "requirementDiagram\n  direction LR\n  requirement req1:::important {\n    id: 1\n    text: Login\n  }\n  element elem1:::first,second {\n    type: simulation\n  }\n  classDef important,first fill:#f96,stroke:#333,stroke-width:4px\n  classDef second color:blue\n  class elem1 important\n  style req1,elem1 fill:#ffa,stroke:#000,color:green\n  elem1:::second\n  req1 - satisfies -> elem1";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(parsed.graph.kind, DiagramKind::Requirement);
        assert_eq!(parsed.graph.direction, Direction::LeftRight);
        assert_eq!(parsed.graph.nodes.len(), 2);
        assert!(parsed.graph.nodes.contains_key("req1"));
        assert!(parsed.graph.nodes.contains_key("elem1"));
        assert!(!parsed.graph.nodes.contains_key("important"));
        assert!(!parsed.graph.nodes.contains_key("first"));
        assert!(!parsed.graph.nodes.contains_key("second"));

        assert!(parsed.graph.class_defs.contains_key("important"));
        assert!(parsed.graph.class_defs.contains_key("first"));
        assert!(parsed.graph.class_defs.contains_key("second"));
        assert_eq!(
            parsed
                .graph
                .class_defs
                .get("important")
                .unwrap()
                .fill
                .as_deref(),
            Some("#f96")
        );

        let req_classes = parsed.graph.node_classes.get("req1").unwrap();
        assert!(
            req_classes
                .iter()
                .any(|class_name| class_name == "important")
        );
        let elem_classes = parsed.graph.node_classes.get("elem1").unwrap();
        assert!(elem_classes.iter().any(|class_name| class_name == "first"));
        assert!(elem_classes.iter().any(|class_name| class_name == "second"));
        assert!(
            elem_classes
                .iter()
                .any(|class_name| class_name == "important")
        );

        assert!(parsed.graph.node_styles.contains_key("req1"));
        assert!(parsed.graph.node_styles.contains_key("elem1"));
    }

    #[test]
    fn parse_gitgraph_basic() {
        let input = "gitGraph\n  commit\n  branch feature\n  checkout feature\n  commit id:\"F1\"\n  checkout main\n  merge feature";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::GitGraph);
        assert!(parsed.graph.gitgraph.commits.len() >= 3);
        assert!(parsed.graph.gitgraph.branches.len() >= 2);
    }

    #[test]
    fn parse_gitgraph_branch_orders_match_mermaid_ast() {
        let input = "---\nconfig:\n  gitGraph:\n    mainBranchOrder: 2\n---\ngitGraph\n  commit\n  branch test1 order: 3\n  branch test2\n  branch test4 order: 1";
        let parsed = parse_mermaid(input).unwrap();

        let branches = &parsed.graph.gitgraph.branches;
        let main = branches
            .iter()
            .find(|branch| branch.name == "main")
            .unwrap();
        let test1 = branches
            .iter()
            .find(|branch| branch.name == "test1")
            .unwrap();
        let test2 = branches
            .iter()
            .find(|branch| branch.name == "test2")
            .unwrap();
        let test4 = branches
            .iter()
            .find(|branch| branch.name == "test4")
            .unwrap();

        assert_eq!(main.order, Some(2.0));
        assert_eq!(test1.order, Some(3.0));
        assert_eq!(test2.order, Some(0.0));
        assert_eq!(test4.order, Some(1.0));
        assert!(
            branches
                .iter()
                .all(|branch| !branch.name.contains("order:"))
        );
    }

    #[test]
    fn parse_gitgraph_duplicate_commit_id_overwrites_visible_commit() {
        let input = "gitGraph\n  commit id:\"A\"\n  branch feature\n  commit id:\"B\"\n  checkout main\n  commit id:\"B\"";
        let parsed = parse_mermaid(input).unwrap();

        let commits = &parsed.graph.gitgraph.commits;
        assert_eq!(
            commits
                .iter()
                .filter(|commit| commit.id.as_str() == "B")
                .count(),
            1
        );
        assert_eq!(commits.len(), 2);

        let duplicate = commits
            .iter()
            .find(|commit| commit.id.as_str() == "B")
            .unwrap();
        assert_eq!(duplicate.branch, "main");
        assert_eq!(duplicate.seq, 2);
        assert_eq!(duplicate.parents, vec!["A".to_string()]);
    }

    #[test]
    fn parse_gitgraph_merge_attributes_do_not_pollute_branch_name() {
        let input = "gitGraph\n  commit id:\"A\"\n  branch feature\n  commit id:\"F\"\n  checkout main\n  merge feature tag:\"T\" id:\"M\" type:HIGHLIGHT";
        let parsed = parse_mermaid(input).unwrap();

        let merge = parsed
            .graph
            .gitgraph
            .commits
            .iter()
            .find(|commit| commit.id.as_str() == "M")
            .unwrap();
        assert_eq!(merge.parents, vec!["A".to_string(), "F".to_string()]);
        assert_eq!(merge.tags, vec!["T".to_string()]);
        assert_eq!(
            merge.custom_type,
            Some(crate::ir::GitGraphCommitType::Highlight)
        );
        assert_eq!(
            merge.message.as_deref(),
            Some("merged branch feature into main")
        );
    }

    #[test]
    fn parse_gitgraph_header_direction() {
        let tb = parse_mermaid("gitGraph TB:\n  commit").unwrap();
        let bt = parse_mermaid("gitGraph BT:\n  commit").unwrap();
        let lr = parse_mermaid("gitGraph LR:\n  commit").unwrap();

        assert_eq!(tb.graph.direction, Direction::TopDown);
        assert_eq!(bt.graph.direction, Direction::BottomTop);
        assert_eq!(lr.graph.direction, Direction::LeftRight);
    }

    #[test]
    fn parse_c4_basic() {
        let input = "C4Context\n  Person(admin, \"Admin\")\n  System(sys, \"System\")\n  Rel(admin, sys, \"Uses\")\n  Boundary(b0, \"Boundary\") { SystemDb(db, \"DB\") }";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::C4);
        assert!(parsed.graph.c4.shapes.len() >= 3);
        assert_eq!(parsed.graph.c4.rels.len(), 1);
        assert!(parsed.graph.c4.boundaries.len() >= 2);
    }

    #[test]
    fn parse_c4_person_system_third_arg_is_description() {
        let input = "C4Context\n  System_Ext(mbs, \"Mainframe Banking System\", \"Stores all of the core banking information\")";
        let parsed = parse_mermaid(input).unwrap();
        let shape = parsed
            .graph
            .c4
            .shapes
            .iter()
            .find(|shape| shape.id == "mbs")
            .unwrap();
        assert_eq!(
            shape.descr.as_deref(),
            Some("Stores all of the core banking information")
        );
        assert_eq!(shape.type_label, None);
    }

    #[test]
    fn parse_c4_named_update_args_strip_quotes() {
        let input = r#"C4Context
  title Example C4
  Person(admin, "Admin")
  System(sys, "System")
  Rel(admin, sys, "Uses")
  UpdateElementStyle(admin, $fontColor="red", $bgColor="grey", $borderColor="red")
  UpdateRelStyle(admin, sys, $textColor="blue", $lineColor="blue", $offsetX="5", $offsetY="-10")
  UpdateLayoutConfig($c4ShapeInRow="3", $c4BoundaryInRow="1")"#;
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.diagram_title.as_deref(), Some("Example C4"));
        let c4 = &parsed.graph.c4;
        let shape = c4.shapes.iter().find(|shape| shape.id == "admin").unwrap();
        assert_eq!(shape.bg_color.as_deref(), Some("grey"));
        assert_eq!(shape.font_color.as_deref(), Some("red"));
        assert_eq!(shape.border_color.as_deref(), Some("red"));
        let rel = c4.rels.first().unwrap();
        assert_eq!(rel.text_color.as_deref(), Some("blue"));
        assert_eq!(rel.line_color.as_deref(), Some("blue"));
        assert_eq!(rel.offset_x, 5.0);
        assert_eq!(rel.offset_y, -10.0);
        assert_eq!(c4.c4_shape_in_row_override, Some(3));
        assert_eq!(c4.c4_boundary_in_row_override, Some(1));
    }

    #[test]
    fn parse_sankey_basic() {
        let input = "sankey\n  A, B, 10\n  B, C, 5";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Sankey);
        assert_eq!(parsed.graph.edges.len(), 2);
    }

    #[test]
    fn parse_sankey_csv_doubled_quotes() {
        let input = "sankey-beta\nPumped heat,\"Heating and cooling, \"\"homes\"\"\",193.026";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Sankey);
        assert_eq!(parsed.graph.edges.len(), 1);
        assert!(
            parsed
                .graph
                .nodes
                .contains_key("Heating and cooling, \"homes\"")
        );
    }

    #[test]
    fn parse_quadrant_basic() {
        let input = "quadrantChart\n  title Sample\n  A : [0.2, 0.8]\n  B : [0.7, 0.3]";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Quadrant);
        assert_eq!(parsed.graph.nodes.len(), 2);
    }

    #[test]
    fn parse_quadrant_axis_quotes_match_mermaid() {
        let input = "quadrantChart\n  x-axis Urgent --> Not Urgent\n  y-axis Low --> \"Important ❤\"\n  quadrant-1 \"Plan\"";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(
            parsed.graph.quadrant.y_axis_top.as_deref(),
            Some("Important ❤")
        );
        assert_eq!(
            parsed.graph.quadrant.quadrant_labels[0].as_deref(),
            Some("Plan")
        );
    }

    #[test]
    fn parse_quadrant_point_styles_and_classes() {
        let input = "quadrantChart
    Campaign A: [0.9, 0.0] radius: 12
    Campaign B:::class1: [0.8, 0.1] color: #ff3300, radius: 10
    Campaign E:::class2: [0.5, 0.4]
    classDef class1 color: #109060
    classDef class2 color: #908342, radius: 10, stroke-color: #310085, stroke-width: 10px";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(parsed.graph.quadrant.points.len(), 3);
        let campaign_a = parsed
            .graph
            .quadrant
            .points
            .iter()
            .find(|point| point.label == "Campaign A")
            .unwrap();
        assert_eq!(campaign_a.label, "Campaign A");
        assert_eq!(campaign_a.style.radius, Some(12.0));

        let campaign_b = parsed
            .graph
            .quadrant
            .points
            .iter()
            .find(|point| point.label == "Campaign B")
            .unwrap();
        assert_eq!(campaign_b.class_name.as_deref(), Some("class1"));
        assert_eq!(campaign_b.style.color.as_deref(), Some("#ff3300"));
        assert_eq!(campaign_b.style.radius, Some(10.0));

        let class2 = parsed.graph.quadrant.point_classes.get("class2").unwrap();
        assert_eq!(class2.color.as_deref(), Some("#908342"));
        assert_eq!(class2.radius, Some(10.0));
        assert_eq!(class2.stroke_color.as_deref(), Some("#310085"));
        assert_eq!(class2.stroke_width.as_deref(), Some("10px"));
    }

    #[test]
    fn parse_zenuml_basic() {
        let input = "zenuml\n  Alice->Bob: Hello\n  Bob-->Alice: Reply";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::ZenUML);
        assert_eq!(parsed.graph.sequence_participants.len(), 2);
        assert_eq!(parsed.graph.edges.len(), 2);
    }

    #[test]
    fn parse_zenuml_method_calls() {
        let input =
            "zenuml\nA.SyncMessage\nA.SyncMessage(with, parameters) {\n  B.nestedSyncMessage()\n}";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(parsed.graph.kind, DiagramKind::ZenUML);
        assert_eq!(
            parsed.graph.sequence_participants,
            vec!["_STARTER_", "A", "B"]
        );
        assert_eq!(parsed.graph.edges.len(), 3);
        assert_eq!(parsed.graph.edges[0].from, "_STARTER_");
        assert_eq!(parsed.graph.edges[0].to, "A");
        assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("SyncMessage"));
        assert_eq!(parsed.graph.edges[2].from, "A");
        assert_eq!(parsed.graph.edges[2].to, "B");
        assert_eq!(
            parsed.graph.edges[2].label.as_deref(),
            Some("nestedSyncMessage()")
        );
    }

    #[test]
    fn parse_zenuml_creation_and_aliases() {
        let input =
            "zenuml\nA as Alice\n@Database Bob\nnew A1\nnew A2(with, parameters)\nA->Bob: Hi";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(parsed.graph.nodes.get("A").unwrap().label, "Alice");
        assert_eq!(
            parsed.graph.nodes.get("Bob").unwrap().shape,
            crate::ir::NodeShape::Cylinder
        );
        assert_eq!(
            parsed.graph.sequence_participants,
            vec!["A", "Bob", "_STARTER_", "A1", "A2"]
        );
        assert_eq!(parsed.graph.sequence_lifecycle.len(), 2);
        assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("«create»"));
        assert_eq!(
            parsed.graph.edges[1].label.as_deref(),
            Some("«with, parameters»")
        );
    }

    #[test]
    fn parse_block_basic() {
        let input = "block\n  A --> B";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Block);
        assert_eq!(parsed.graph.edges.len(), 1);
    }

    #[test]
    fn parse_block_asymmetric_label_with_spaces() {
        let input = "block\n  id1>\"This is the text in the box\"]";
        let parsed = parse_mermaid(input).unwrap();
        let block = parsed.graph.block.as_ref().unwrap();
        let node = parsed.graph.nodes.get("id1").unwrap();

        assert_eq!(parsed.graph.kind, DiagramKind::Block);
        assert_eq!(parsed.graph.nodes.len(), 1);
        assert_eq!(block.nodes.len(), 1);
        assert_eq!(block.nodes[0].id, "id1");
        assert_eq!(node.label, "This is the text in the box");
        assert_eq!(node.shape, crate::ir::NodeShape::Asymmetric);
    }

    #[test]
    fn parse_block_directives_do_not_create_nodes() {
        let input = "block\n A space B\n classDef blue fill:#6e6ce6,stroke:#333,stroke-width:4px;\n class A blue\n style B fill:#bbf,stroke:#f66,stroke-width:2px,color:#fff,stroke-dasharray: 5 5";
        let parsed = parse_mermaid(input).unwrap();

        assert!(parsed.graph.nodes.contains_key("A"));
        assert!(parsed.graph.nodes.contains_key("B"));
        assert!(!parsed.graph.nodes.contains_key("classDef"));
        assert!(!parsed.graph.nodes.contains_key("style"));
        assert!(parsed.graph.class_defs.contains_key("blue"));
        assert_eq!(
            parsed.graph.class_defs.get("blue").unwrap().stroke_width,
            Some(4.0)
        );
        assert_eq!(
            parsed.graph.node_classes.get("A").unwrap(),
            &vec!["blue".to_string()]
        );
        assert!(parsed.graph.node_styles.contains_key("B"));
    }

    #[test]
    fn parse_block_arrow_shapes() {
        let input = r#"block
 blockArrowId<["Label"]>(right)
 blockArrowId7<["Label"]>(x, down)"#;
        let parsed = parse_mermaid(input).unwrap();

        let right = parsed.graph.nodes.get("blockArrowId").unwrap();
        let x_down = parsed.graph.nodes.get("blockArrowId7").unwrap();
        assert_eq!(right.label, "Label");
        assert_eq!(right.shape, crate::ir::NodeShape::BlockArrowRight);
        assert_eq!(x_down.label, "Label");
        assert_eq!(x_down.shape, crate::ir::NodeShape::BlockArrowXDown);
    }

    #[test]
    fn parse_block_composite_does_not_create_container_text_node() {
        let input = "block\n columns 1\n block:ID\n A\n B\n end\n D\n ID --> D";
        let parsed = parse_mermaid(input).unwrap();

        assert!(!parsed.graph.nodes.contains_key("ID"));
        let block = parsed.graph.block.as_ref().unwrap();
        assert_eq!(block.nodes.len(), 2);
        assert_eq!(block.nodes[0].id, "ID");
        assert_eq!(block.groups.get("ID").unwrap().nodes[0].id, "A".to_string());
        assert_eq!(block.groups.get("ID").unwrap().nodes[1].id, "B".to_string());
        assert_eq!(parsed.graph.subgraphs.len(), 1);
        assert_eq!(parsed.graph.subgraphs[0].id.as_deref(), Some("ID"));
        assert_eq!(
            parsed.graph.subgraphs[0].nodes,
            vec!["A".to_string(), "B".to_string()]
        );
        assert_eq!(parsed.graph.edges.len(), 1);
        assert_eq!(parsed.graph.edges[0].from, "ID");
        assert_eq!(parsed.graph.edges[0].to, "D");
    }

    #[test]
    fn split_block_row_tokens_respects_node_syntax() {
        let tokens = split_block_row_tokens(
            r#"a["A label"] b:2 id1>"This is the text in the box"] arrow<[" "]>(right) c@{ shape: odd, label: "Odd Label" }"#,
        );

        assert_eq!(
            tokens,
            vec![
                r#"a["A label"]"#,
                "b:2",
                r#"id1>"This is the text in the box"]"#,
                r#"arrow<[" "]>(right)"#,
                r#"c@{ shape: odd, label: "Odd Label" }"#,
            ]
        );
    }

    #[test]
    fn parse_packet_basic() {
        let input = "packet\n  0-7: \"Type\"\n  8-15: \"Len\"";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Packet);
        assert_eq!(parsed.graph.packet.blocks.len(), 2);
        assert_eq!(parsed.graph.packet.blocks[0].start, 0);
        assert_eq!(parsed.graph.packet.blocks[0].end, 7);
        assert_eq!(parsed.graph.packet.blocks[0].label, "Type");
        assert_eq!(parsed.graph.packet.blocks[1].start, 8);
        assert_eq!(parsed.graph.packet.blocks[1].end, 15);
        assert_eq!(parsed.graph.packet.blocks[1].label, "Len");
    }

    #[test]
    fn parse_eventmodeling_frames_data_and_sources() {
        let input = "eventmodeling\n\
tf 01 ui CartUI\n\
tf 02 cmd AddItem [[AddItem01]]\n\
data AddItem01 {\n\
  productId: 7\n\
}\n\
tf 03 evt ItemAdded ->> 01 ->> 02\n";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::EventModeling);
        assert_eq!(parsed.graph.eventmodeling.frames.len(), 3);
        assert_eq!(parsed.graph.eventmodeling.data_entities.len(), 1);
        assert_eq!(
            parsed.graph.eventmodeling.frames[2].source_frames,
            vec!["01".to_string(), "02".to_string()]
        );
    }

    #[test]
    fn parse_cynefin_domains_items_and_transitions() {
        let input = "cynefin-beta\n\
title Decision space\n\
complex\n\
  \"Probe market\"\n\
complicated\n\
  \"Analyse telemetry\"\n\
clear\n\
chaotic\n\
confusion\n\
  \"Unknown\"\n\
  \"Too much\"\n\
  \"Ambiguous\"\n\
  \"Mixed\"\n\
complex --> complicated: \"clarify\"\n\
clear --> clear: \"ignored self loop\"\n";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Cynefin);
        assert_eq!(
            parsed.graph.cynefin.title.as_deref(),
            Some("Decision space")
        );
        assert_eq!(parsed.graph.cynefin.domains.len(), 5);
        assert_eq!(
            parsed
                .graph
                .cynefin
                .domains
                .get(&crate::ir::CynefinDomainName::Complex)
                .unwrap()
                .items[0]
                .label,
            "Probe market"
        );
        assert_eq!(parsed.graph.cynefin.transitions.len(), 1);
        assert_eq!(
            parsed.graph.cynefin.transitions[0].label.as_deref(),
            Some("clarify")
        );
    }

    #[test]
    fn parse_tree_view_preserves_indentation() {
        let input = "treeView-beta\n    \"project\"\n        \"src\"\n            \"main.rs\"\n            \"lib.rs\"\n        \"README.md\"";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(parsed.graph.kind, DiagramKind::TreeView);
        assert_eq!(parsed.graph.tree_view.root.len(), 1);
        let project = &parsed.graph.tree_view.root[0];
        assert_eq!(project.name, "project");
        assert_eq!(project.children.len(), 2);
        assert_eq!(project.children[0].name, "src");
        assert_eq!(project.children[0].children[0].name, "main.rs");
        assert_eq!(project.children[0].children[1].name, "lib.rs");
        assert_eq!(project.children[1].name, "README.md");
    }

    #[test]
    fn parse_tree_view_annotations_and_node_types() {
        let input = "treeView-beta\n    src/\n        App.tsx :::highlight icon(react) ## main component\n        config.toml\n        secret icon()";
        let parsed = parse_mermaid(input).unwrap();

        let src = &parsed.graph.tree_view.root[0];
        assert_eq!(src.name, "src");
        assert_eq!(src.node_type, crate::ir::TreeViewNodeType::Directory);
        assert_eq!(src.icon_id.as_deref(), Some("folder"));

        let app = &src.children[0];
        assert_eq!(app.name, "App.tsx");
        assert_eq!(app.node_type, crate::ir::TreeViewNodeType::File);
        assert_eq!(app.icon_id.as_deref(), Some("react"));
        assert_eq!(app.css_class.as_deref(), Some("highlight"));
        assert_eq!(app.description.as_deref(), Some("main component"));

        let config = &src.children[1];
        assert_eq!(config.icon_id.as_deref(), Some("config"));

        let secret = &src.children[2];
        assert_eq!(secret.icon_id.as_deref(), Some("none"));
    }

    #[test]
    fn parse_kanban_basic() {
        let input = "kanban\n  todo[To Do]\n    t1[Task 1]\n  done[Done]\n    t2[Task 2]";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Kanban);
        assert_eq!(parsed.graph.subgraphs.len(), 2);
        assert_eq!(parsed.graph.nodes.len(), 2);
    }

    #[test]
    fn parse_kanban_nodes_without_explicit_ids() {
        let input = "kanban\n  [In progress]\n    [Create Documentation]\n    docs[Create Blog about the new diagram]";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(parsed.graph.subgraphs[0].id.as_deref(), Some("In progress"));
        assert_eq!(parsed.graph.subgraphs[0].label, "In progress");
        assert!(parsed.graph.nodes.contains_key("Create Documentation"));
        assert_eq!(
            parsed
                .graph
                .nodes
                .get("Create Documentation")
                .map(|node| node.label.as_str()),
            Some("Create Documentation")
        );
        assert_eq!(
            parsed
                .graph
                .nodes
                .get("docs")
                .map(|node| node.label.as_str()),
            Some("Create Blog about the new diagram")
        );
    }

    #[test]
    fn parse_architecture_basic() {
        let input = "architecture-beta\n  group api(icon)[API]\n  service web(icon)[Web] in api\n  service db(icon)[DB] in api\n  web:R --> L:db";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Architecture);
        assert_eq!(parsed.graph.subgraphs.len(), 1);
        assert_eq!(parsed.graph.edges.len(), 1);
    }

    #[test]
    fn parse_radar_basic() {
        let input = "radar-beta\n  axis A, B, C\n  curve Alpha {1,2,3}";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Radar);
        assert_eq!(parsed.graph.nodes.len(), 1);
    }

    #[test]
    fn parse_radar_labeled_axes_and_curves() {
        let input = r#"radar-beta
  title Product Comparison
  axis perf["Performance"], rel["Reliability"], cost
  curve p1["Product A"]{4, 3, 2}
  curve p2["Product B"]{ cost: 1, perf: 5, rel: 4 }
  showLegend false
  ticks 8
  max 10
  min 1
  graticule polygon
"#;
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Radar);
        assert_eq!(
            parsed.graph.diagram_title.as_deref(),
            Some("Product Comparison")
        );
        assert!(!parsed.graph.radar.show_legend);
        assert_eq!(parsed.graph.radar.ticks, 8);
        assert_eq!(parsed.graph.radar.max, Some(10.0));
        assert_eq!(parsed.graph.radar.min, 1.0);
        assert_eq!(
            parsed.graph.radar.graticule,
            crate::ir::RadarGraticule::Polygon
        );
        assert_eq!(parsed.graph.nodes.len(), 2);
        let first = parsed.graph.nodes.get("radar_0").unwrap();
        assert!(first.label.contains("Product A"));
        assert!(first.label.contains("Performance: 4"));
        assert!(first.label.contains("Reliability: 3"));
        let second = parsed.graph.nodes.get("radar_1").unwrap();
        assert!(second.label.contains("Product B"));
        assert!(second.label.contains("Performance: 5"));
        assert!(second.label.contains("Reliability: 4"));
        assert!(second.label.contains("cost: 1"));
    }

    #[test]
    fn parse_treemap_basic() {
        let input = "treemap-beta\n  Root: 100\n    Child: 40";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Treemap);
        assert_eq!(parsed.graph.nodes.len(), 2);
        assert_eq!(parsed.graph.edges.len(), 1);
    }

    #[test]
    fn parse_treemap_styling_lines_and_inline_classes_do_not_create_nodes() {
        let input = "treemap-beta\n\"Main\"\n \"A\": 20:::important\n \"B\":::important\n  \"B1\": 10\nclassDef important fill:#f96,stroke:#333,stroke-width:2px;\nstyle treemap_0 fill:#bbf";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(parsed.graph.kind, DiagramKind::Treemap);
        assert_eq!(parsed.graph.nodes.len(), 4);
        assert!(!parsed.graph.nodes.contains_key("classDef important fill"));
        assert!(parsed.graph.class_defs.contains_key("important"));
        assert!(
            parsed
                .graph
                .node_classes
                .get("treemap_1")
                .is_some_and(|classes| classes.iter().any(|class_name| class_name == "important"))
        );
        assert!(
            parsed
                .graph
                .node_classes
                .get("treemap_2")
                .is_some_and(|classes| classes.iter().any(|class_name| class_name == "important"))
        );
        assert!(parsed.graph.node_styles.contains_key("treemap_0"));
    }

    #[test]
    fn parse_xy_chart_basic() {
        let input = "xychart-beta\n  x-axis Q1, Q2\n  y-axis Units\n  bar [10, 20]";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::XYChart);
        let xychart = &parsed.graph.xychart;
        assert_eq!(xychart.x_axis_categories, vec!["Q1", "Q2"]);
        assert_eq!(xychart.y_axis_label.as_deref(), Some("Units"));
        assert_eq!(xychart.series.len(), 1);
    }

    #[test]
    fn parse_state_diagram_basic() {
        let input = "stateDiagram-v2\n[*] --> Idle\nIdle --> Active : start\nstate \"Waiting\" as Wait\nWait --> Active";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::State);
        assert!(parsed.graph.nodes.contains_key("Idle"));
        assert!(parsed.graph.nodes.contains_key("Active"));
        assert!(parsed.graph.nodes.contains_key("Wait"));
        let wait_label = &parsed.graph.nodes.get("Wait").unwrap().label;
        assert_eq!(wait_label, "Waiting");
        assert!(parsed.graph.edges.len() >= 2);
    }

    #[test]
    fn parse_state_description_line() {
        let input = "stateDiagram-v2\nstate Idle : Waiting\nIdle --> Done";
        let parsed = parse_mermaid(input).unwrap();
        let node = parsed.graph.nodes.get("Idle").unwrap();
        assert_eq!(node.label, "Waiting");
    }

    #[test]
    fn parse_state_choice_stereotype() {
        let input = "stateDiagram-v2\nstate Decide <<choice>>\n[*] --> Decide";
        let parsed = parse_mermaid(input).unwrap();
        let node = parsed.graph.nodes.get("Decide").unwrap();
        assert_eq!(node.shape, crate::ir::NodeShape::Diamond);
    }

    #[test]
    fn parse_state_fork_stereotype() {
        let input = "stateDiagram-v2\nstate Fork <<fork>>\n[*] --> Fork";
        let parsed = parse_mermaid(input).unwrap();
        let node = parsed.graph.nodes.get("Fork").unwrap();
        assert_eq!(node.shape, crate::ir::NodeShape::ForkJoin);
        assert!(node.label.trim().is_empty());
    }

    #[test]
    fn parse_state_inline_class() {
        let input = "stateDiagram-v2\nclassDef hot fill:#f00\nstate Idle:::hot";
        let parsed = parse_mermaid(input).unwrap();
        let classes = parsed.graph.node_classes.get("Idle").unwrap();
        assert!(classes.iter().any(|c| c == "hot"));
    }

    #[test]
    fn parse_state_note() {
        let input = "stateDiagram-v2\nstate Idle\nnote right of Idle: waiting";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.state_notes.len(), 1);
        let note = &parsed.graph.state_notes[0];
        assert_eq!(note.target, "Idle");
        assert_eq!(note.label, "waiting");
        assert_eq!(note.position, crate::ir::StateNotePosition::RightOf);
    }

    #[test]
    fn parse_sequence_diagram_basic() {
        let input = "sequenceDiagram\nparticipant A as Alice\nparticipant Bob\nA->>Bob: Hello\nBob-->>A: Hi";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Sequence);
        assert_eq!(parsed.graph.sequence_participants.len(), 2);
        assert_eq!(parsed.graph.sequence_participants[0], "A");
        assert_eq!(parsed.graph.sequence_participants[1], "Bob");
        // Verify the display label is "Alice" (right side of "as")
        let node = parsed.graph.nodes.get("A").unwrap();
        assert_eq!(node.label, "Alice");
        assert_eq!(parsed.graph.edges.len(), 2);
        assert_eq!(parsed.graph.edges[1].style, crate::ir::EdgeStyle::Dotted);
    }

    #[test]
    fn parse_sequence_database_participant() {
        let input = "sequenceDiagram\ndatabase DB\nDB->>DB: ping";
        let parsed = parse_mermaid(input).unwrap();
        let node = parsed.graph.nodes.get("DB").unwrap();
        assert_eq!(node.shape, crate::ir::NodeShape::Cylinder);
    }

    #[test]
    fn parse_sequence_autonumber_off() {
        let input = "sequenceDiagram\nautonumber off\nA->>B: ping";
        let parsed = parse_mermaid(input).unwrap();
        assert!(parsed.graph.sequence_autonumber.is_none());
    }

    #[test]
    fn parse_sequence_alt_sections() {
        let input = "sequenceDiagram\nA->>B: req\nalt ok\nB-->>A: yes\nelse bad\nB-->>A: no\nend";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.kind, DiagramKind::Sequence);
        assert_eq!(parsed.graph.edges.len(), 3);
        assert_eq!(parsed.graph.sequence_frames.len(), 1);
        let frame = &parsed.graph.sequence_frames[0];
        assert_eq!(frame.sections.len(), 2);
        assert_eq!(frame.sections[0].label.as_deref(), Some("ok"));
        assert_eq!(frame.sections[0].start_idx, 1);
        assert_eq!(frame.sections[0].end_idx, 2);
        assert_eq!(frame.sections[1].label.as_deref(), Some("bad"));
        assert_eq!(frame.sections[1].start_idx, 2);
        assert_eq!(frame.sections[1].end_idx, 3);
    }

    #[test]
    fn parse_sequence_par_sections() {
        let input =
            "sequenceDiagram\nA->>B: req\npar first\nB-->>A: yes\nand second\nB-->>A: no\nend";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.sequence_frames.len(), 1);
        let frame = &parsed.graph.sequence_frames[0];
        assert_eq!(frame.kind, crate::ir::SequenceFrameKind::Par);
        assert_eq!(frame.sections.len(), 2);
        assert_eq!(frame.sections[0].label.as_deref(), Some("first"));
        assert_eq!(frame.sections[1].label.as_deref(), Some("second"));
    }

    #[test]
    fn parse_sequence_critical_sections() {
        let input =
            "sequenceDiagram\nA->>B: req\ncritical ok\nB-->>A: yes\noption fail\nB-->>A: no\nend";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.sequence_frames.len(), 1);
        let frame = &parsed.graph.sequence_frames[0];
        assert_eq!(frame.kind, crate::ir::SequenceFrameKind::Critical);
        assert_eq!(frame.sections.len(), 2);
        assert_eq!(frame.sections[0].label.as_deref(), Some("ok"));
        assert_eq!(frame.sections[1].label.as_deref(), Some("fail"));
    }

    #[test]
    fn parse_sequence_box() {
        let input = "sequenceDiagram\nbox Aqua Group\nparticipant A\nparticipant B\nend";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.sequence_boxes.len(), 1);
        let seq_box = &parsed.graph.sequence_boxes[0];
        assert_eq!(seq_box.color.as_deref(), Some("Aqua"));
        assert_eq!(seq_box.label.as_deref(), Some("Group"));
        assert_eq!(seq_box.participants.len(), 2);
        assert!(seq_box.participants.iter().any(|id| id == "A"));
        assert!(seq_box.participants.iter().any(|id| id == "B"));
    }

    #[test]
    fn parse_sequence_notes() {
        let input = "sequenceDiagram\nparticipant Alice\nparticipant Bob\nAlice->>Bob: Hello\nNote over Alice,Bob: ping\nBob-->>Alice: Hi\nNote right of Bob: done";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.sequence_notes.len(), 2);
        let first = &parsed.graph.sequence_notes[0];
        assert_eq!(first.index, 1);
        assert_eq!(first.label, "ping");
        assert_eq!(first.position, crate::ir::SequenceNotePosition::Over);
        let second = &parsed.graph.sequence_notes[1];
        assert_eq!(second.index, 2);
        assert_eq!(second.label, "done");
        assert_eq!(second.position, crate::ir::SequenceNotePosition::RightOf);
    }

    #[test]
    fn parse_multiple_classes() {
        let input =
            "flowchart LR\nclassDef hot fill:#f00\nclassDef cold fill:#00f\nA\nclass A hot,cold";
        let parsed = parse_mermaid(input).unwrap();
        let classes = parsed.graph.node_classes.get("A").unwrap();
        assert!(classes.iter().any(|c| c == "hot"));
        assert!(classes.iter().any(|c| c == "cold"));
    }

    #[test]
    fn parse_node_id_with_dot() {
        let input = "flowchart LR\nsvc.api[Service] --> db.main[(DB)]";
        let parsed = parse_mermaid(input).unwrap();
        assert!(parsed.graph.nodes.contains_key("svc.api"));
        assert!(parsed.graph.nodes.contains_key("db.main"));
    }

    #[test]
    fn parse_flowchart_legacy_fontawesome_icon_labels() {
        let input = "flowchart TD\nB[fa:fa-twitter]\nB-->E(fak:fa-custom-icon-name)";
        let parsed = parse_mermaid(input).unwrap();
        let b = parsed.graph.nodes.get("B").unwrap();
        let e = parsed.graph.nodes.get("E").unwrap();

        assert_eq!(b.label, "");
        assert_eq!(b.icon.as_deref(), Some("fa:fa-twitter"));
        assert_eq!(e.label, "");
        assert_eq!(e.icon.as_deref(), Some("fak:fa-custom-icon-name"));
    }

    #[test]
    fn parse_flowchart_declarative_icon_shape_metadata() {
        let input = r#"flowchart TD
    A@{ shape: icon, icon: "fa:fa-heart", form: "circle", label: "Heart" }"#;
        let parsed = parse_mermaid(input).unwrap();
        let node = parsed.graph.nodes.get("A").unwrap();

        assert_eq!(node.shape, crate::ir::NodeShape::IconCircle);
        assert_eq!(node.label, "Heart");
        assert_eq!(node.icon.as_deref(), Some("fa:fa-heart"));
    }

    #[test]
    fn parse_flowchart_flag_shape_uses_paper_tape_alias() {
        let input = "flowchart TD\nA@{ shape: flag }";
        let parsed = parse_mermaid(input).unwrap();
        let node = parsed.graph.nodes.get("A").unwrap();

        assert_eq!(node.shape, crate::ir::NodeShape::WavyRect);
    }

    #[test]
    fn parse_flowchart_sloped_rectangle_alias_uses_sloped_rect() {
        let input = "flowchart TD\nA@{ shape: sloped-rectangle }";
        let parsed = parse_mermaid(input).unwrap();
        let node = parsed.graph.nodes.get("A").unwrap();

        assert_eq!(node.shape, crate::ir::NodeShape::SlopedRect);
    }

    #[test]
    fn parse_flowchart_circle_and_double_circle_match_mermaid_grammar() {
        let input = "flowchart TD\nA((Circle))\nB(((Double)))";
        let parsed = parse_mermaid(input).unwrap();

        assert_eq!(
            parsed.graph.nodes.get("A").unwrap().shape,
            crate::ir::NodeShape::Circle
        );
        assert_eq!(
            parsed.graph.nodes.get("B").unwrap().shape,
            crate::ir::NodeShape::DoubleCircle
        );
    }

    #[test]
    fn parse_flowchart_comment_brace_aliases_match_mermaid_shapes() {
        let input = "flowchart TD\nA@{ shape: comment }\nB@{ shape: braces }";
        let parsed = parse_mermaid(input).unwrap();
        let a = parsed.graph.nodes.get("A").unwrap();
        let b = parsed.graph.nodes.get("B").unwrap();

        assert_eq!(a.shape, crate::ir::NodeShape::BraceLeft);
        assert_eq!(b.shape, crate::ir::NodeShape::BraceBoth);
    }

    #[test]
    fn parse_init_with_single_quotes() {
        let input = "%%{init: {'themeVariables': {'primaryColor': '#fff'}}}%%\nflowchart LR\nA-->B";
        let parsed = parse_mermaid(input).unwrap();
        assert!(parsed.init_config.is_some());
    }

    #[test]
    fn parses_click_directive() {
        let input = "flowchart LR\nA-->B\nclick A \"https://example.com\"";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.nodes.len(), 2);
        assert_eq!(parsed.graph.edges.len(), 1);
        let link = parsed.graph.node_links.get("A").unwrap();
        assert_eq!(link.url, "https://example.com");
        assert!(link.title.is_none());
        assert!(link.target.is_none());
    }

    #[test]
    fn strips_inline_comments() {
        let input = "flowchart LR\nA-->B %% comment\nB-->C";
        let parsed = parse_mermaid(input).unwrap();
        assert_eq!(parsed.graph.edges.len(), 2);
    }

    #[test]
    fn parse_link_style_whitespace_indexes() {
        let input = "flowchart LR\nA-->B\nB-->C\nlinkStyle 0 1 stroke:#0f0";
        let parsed = parse_mermaid(input).unwrap();
        assert!(parsed.graph.edge_styles.contains_key(&0));
        assert!(parsed.graph.edge_styles.contains_key(&1));
    }

    #[test]
    fn parse_emoji_in_node_label() {
        // Emoji characters are multi-byte UTF-8, this tests that mask_bracket_content
        // preserves byte positions correctly when masking content inside brackets
        let input = r#"flowchart LR
    YT -->|"Streams audio"| Speaker["🔊"]
    A["🎵 Music"] --> B["🔈 Sound"]"#;
        let parsed = parse_mermaid(input).unwrap();
        assert!(parsed.graph.nodes.contains_key("Speaker"));
        assert!(parsed.graph.nodes.contains_key("A"));
        assert!(parsed.graph.nodes.contains_key("B"));
        assert!(parsed.graph.nodes.contains_key("YT"));
        assert_eq!(parsed.graph.edges.len(), 2);
    }

    #[test]
    fn mask_bracket_content_preserves_byte_positions() {
        // Test that masking preserves byte length for proper regex extraction
        let line = r#"Speaker["🔊"]"#;
        let masked = super::mask_bracket_content(line);
        assert_eq!(
            line.len(),
            masked.len(),
            "masked string should have same byte length as original"
        );
    }

    #[test]
    fn strip_quotes_markdown_detects_backtick_syntax() {
        let (text, md) = super::strip_quotes_markdown(r#""`**bold**`""#);
        assert_eq!(text, "**bold**");
        assert!(md, "should detect markdown");
    }

    #[test]
    fn strip_quotes_markdown_plain_quotes() {
        let (text, md) = super::strip_quotes_markdown(r#""plain""#);
        assert_eq!(text, "plain");
        assert!(!md, "should not be markdown");
    }

    #[test]
    fn markdown_node_label_sets_flag() {
        let input = "flowchart LR\n    A[\"`**bold**`\"]";
        let parsed = parse_mermaid(input).unwrap();
        let node = parsed.graph.nodes.get("A").unwrap();
        assert!(node.markdown_label, "node should have markdown_label=true");
        assert_eq!(node.label, "**bold**");
    }

    #[test]
    fn markdown_edge_label_sets_flag() {
        let input = "flowchart LR\n    A -- \"`**bold**`\" --> B";
        let parsed = parse_mermaid(input).unwrap();
        assert!(
            parsed.graph.edges[0].markdown_label,
            "edge should have markdown_label=true"
        );
        assert_eq!(parsed.graph.edges[0].label.as_deref(), Some("**bold**"));
    }

    #[test]
    fn markdown_subgraph_label_sets_flag() {
        let input = "flowchart LR\nsubgraph s1[\"`**bold sub**`\"]\nA\nend";
        let parsed = parse_mermaid(input).unwrap();
        assert!(
            parsed.graph.subgraphs[0].markdown_label,
            "subgraph should have markdown_label=true"
        );
        assert_eq!(parsed.graph.subgraphs[0].label, "**bold sub**");
    }

    #[test]
    fn non_markdown_node_has_flag_false() {
        let input = "flowchart LR\n    A[\"plain text\"]";
        let parsed = parse_mermaid(input).unwrap();
        let node = parsed.graph.nodes.get("A").unwrap();
        assert!(!node.markdown_label);
    }
}
