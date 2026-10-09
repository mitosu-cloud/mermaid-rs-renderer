use super::{extended::*, *};
use crate::ir::{CurveType, NodeShape};
use std::collections::{BTreeMap, HashSet};

pub(super) fn parse(input: &str) -> Result<ParseOutput> {
    let (lines, mut init_config) = statements(input)?;
    map_config(&mut init_config, "agentflow");
    let mut normalized = String::new();
    let mut metadata_by_id = BTreeMap::new();
    let mut stack = Vec::<Option<String>>::new();
    let mut parents = BTreeMap::<String, Option<String>>::new();
    let mut globals = HashSet::new();
    let mut global_definition = String::from("flowchart TB\n");
    let mut connectors = HashSet::new();
    let mut acc_title = None;
    let mut acc_descr = None;
    for line in lines {
        if let Some(rest) = line.strip_prefix("accTitle:") {
            acc_title = Some(rest.trim().to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("accDescr:") {
            acc_descr = Some(rest.trim().to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("accDescr") {
            if let Some(body) = rest
                .trim()
                .strip_prefix('{')
                .and_then(|s| s.strip_suffix('}'))
            {
                acc_descr = Some(body.trim().to_string());
                continue;
            }
        }
        if let Some(rest) = line.strip_prefix("agentflow-beta") {
            let direction = if rest.trim().is_empty() {
                "TB"
            } else {
                rest.trim()
            };
            if Direction::from_token(direction).is_none() {
                anyhow::bail!("Unknown agentflow direction: {direction}");
            }
            normalized.push_str(&format!("flowchart {direction}\n"));
            continue;
        }
        if line == "global" {
            stack.push(None);
            continue;
        }
        if line == "end" {
            let context = stack
                .pop()
                .ok_or_else(|| anyhow::anyhow!("Unexpected agentflow container end"))?;
            if context.is_some() {
                normalized.push_str("end\n");
            }
            continue;
        }
        let (base, meta) = metadata(&line)?;
        let rest = base
            .strip_prefix("flow ")
            .or_else(|| base.strip_prefix("connector "))
            .unwrap_or(&base)
            .trim();
        let (id, _) = identifier(rest);
        if let Some(meta) = meta {
            if id.is_empty() {
                anyhow::bail!("Metadata requires an element identifier");
            }
            let entry = metadata_by_id
                .entry(id.to_string())
                .or_insert_with(|| serde_json::json!({}));
            if let Some(map) = entry.as_object_mut() {
                map.extend(meta.as_object().unwrap().clone());
            }
        }
        if base.starts_with("flow ") {
            let (id, _, _, _, _, _) = declaration(rest)?;
            if parents.contains_key(&id) {
                anyhow::bail!("Duplicate agentflow container: {id}");
            }
            parents.insert(id.clone(), stack.iter().rev().find_map(Clone::clone));
            stack.push(Some(id));
            normalized.push_str("subgraph ");
            normalized.push_str(rest);
            normalized.push('\n');
            continue;
        }
        if base.starts_with("connector ") {
            connectors.insert(id.to_string());
        }
        if stack.iter().any(Option::is_none)
            && !id.is_empty()
            && ![
                "classDef",
                "class",
                "style",
                "linkStyle",
                "direction",
                "accTitle",
                "accDescr",
            ]
            .contains(&id)
        {
            globals.insert(id.to_string());
            global_definition.push_str(rest);
            global_definition.push('\n');
        }
        if rest == id && metadata_by_id.contains_key(id) {
            continue;
        }
        normalized.push_str(rest);
        normalized.push('\n');
    }
    if !stack.is_empty() {
        anyhow::bail!("Unterminated agentflow container");
    }
    let mut parsed = parse_flowchart(&normalized)?;
    let graph = &mut parsed.graph;
    graph.kind = DiagramKind::Agentflow;
    graph.agentflow_config = Some(init_config.clone().unwrap_or_else(|| serde_json::json!({})));
    graph.acc_title = acc_title;
    graph.acc_descr = acc_descr;
    parsed.init_config = init_config;
    globals.extend(parse_flowchart(&global_definition)?.graph.nodes.into_keys());
    for sub in &mut graph.subgraphs {
        sub.nodes.retain(|id| !globals.contains(id));
    }
    let containers: HashSet<_> = graph
        .subgraphs
        .iter()
        .filter_map(|sub| sub.id.clone())
        .collect();
    let container_order: Vec<_> = graph
        .subgraphs
        .iter()
        .enumerate()
        .filter_map(|(index, sub)| sub.id.as_ref().map(|id| (id.clone(), index)))
        .collect();
    for (id, index) in container_order {
        merge_metadata(graph, &id, serde_json::json!({"_containerIndex": index}));
    }
    for (id, parent) in &parents {
        merge_metadata(graph, id, serde_json::json!({"_parent": parent}));
    }
    for node in graph.nodes.values_mut() {
        if !containers.contains(&node.id) {
            node.shape = NodeShape::RoundRect;
        }
    }
    for id in connectors {
        graph.ensure_node(&id, None, Some(NodeShape::RoundRect));
        merge_metadata(
            graph,
            &id,
            serde_json::json!({"_agentflowKind": "connector"}),
        );
    }
    for (id, value) in metadata_by_id {
        merge_metadata(graph, &id, value);
    }
    for (id, metadata) in graph.element_metadata.clone() {
        if let Some(edge) = graph
            .edges
            .iter_mut()
            .find(|edge| edge.id.as_deref() == Some(&id))
        {
            edge.curve = metadata
                .get("curve")
                .and_then(|value| value.as_str())
                .and_then(CurveType::from_name);
            continue;
        }
        if containers.contains(&id) {
            continue;
        }
        graph.ensure_node(&id, None, None);
        let node = graph.nodes.get_mut(&id).unwrap();
        if let Some(label) = metadata.get("label").and_then(|value| value.as_str()) {
            let (label, markdown) = strip_quotes_markdown(label);
            node.label = label;
            node.markdown_label =
                markdown || metadata.get("labelType").and_then(|v| v.as_str()) == Some("markdown");
        }
        if let Some(shape) = metadata.get("shape").and_then(|value| value.as_str()) {
            node.shape = match shape {
                "tool" | "subroutine" | "subprocess" | "subproc" | "framed-rectangle" => {
                    NodeShape::Subroutine
                }
                "input" | "lean-right" => NodeShape::Parallelogram,
                "decision" | "diamond" => NodeShape::Diamond,
                "refdoc" | "lin-doc" | "lined-document" => NodeShape::ReferenceDocument,
                "action" | "hexagon" | "hex" => NodeShape::Hexagon,
                "collapsedGroup" => NodeShape::CollapsedGroup,
                "connector" => NodeShape::RoundRect,
                // Unsupported and removed shapes are rendered with Mermaid's
                // rounded fallback, rather than as a different flowchart glyph.
                _ => NodeShape::RoundRect,
            };
        }
        if let Some(icon) = metadata.get("icon").and_then(|value| value.as_str()) {
            node.icon = Some(icon.to_string());
        }
    }
    collapse_flows(graph, &parents);
    edge_styles(graph);
    Ok(parsed)
}

fn collapse_flows(graph: &mut Graph, parents: &BTreeMap<String, Option<String>>) {
    let collapsed: Vec<_> = graph
        .element_metadata
        .iter()
        .filter(|(_, value)| value.get("view").and_then(|v| v.as_str()) == Some("collapsed"))
        .map(|(id, _)| id.clone())
        .collect();
    for id in collapsed {
        let Some(sub) = graph
            .subgraphs
            .iter()
            .find(|sub| sub.id.as_ref() == Some(&id))
            .cloned()
        else {
            continue;
        };
        let mut descendants: HashSet<String> = sub.nodes.iter().cloned().collect();
        for child in parents.keys() {
            let mut current = parents.get(child).and_then(|value| value.as_ref());
            while let Some(parent) = current {
                if parent == &id {
                    descendants.insert(child.clone());
                    break;
                }
                current = parents.get(parent).and_then(|value| value.as_ref());
            }
        }
        graph.nodes.retain(|node, _| !descendants.contains(node));
        graph.ensure_node_md(
            &id,
            Some(sub.label),
            Some(NodeShape::CollapsedGroup),
            sub.markdown_label,
        );
        for edge in &mut graph.edges {
            if descendants.contains(&edge.from) {
                edge.from = id.clone();
            }
            if descendants.contains(&edge.to) {
                edge.to = id.clone();
            }
        }
        graph.edges.retain(|edge| edge.from != id || edge.to != id);
        graph.subgraphs.retain(|sub| {
            sub.id.as_ref() != Some(&id)
                && !sub
                    .id
                    .as_ref()
                    .is_some_and(|child| descendants.contains(child))
        });
        for parent in &mut graph.subgraphs {
            if parent.nodes.iter().any(|node| descendants.contains(node)) {
                parent.nodes.retain(|node| !descendants.contains(node));
                if !parent.nodes.contains(&id) {
                    parent.nodes.push(id.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_nodes_stay_global_and_metadata_is_retained() {
        let input = "agentflow-beta LR\nglobal\ncorpus[Shared]@{shape: refdoc}\nend\nflow worker[Worker]\ntask[Read]@{shape: task, instruction: 'Read, then report', params: [a,b], __proto__: bad}\ntask -.- corpus\nend";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.graph.nodes["corpus"].shape,
            NodeShape::ReferenceDocument
        );
        assert!(
            !parsed.graph.subgraphs[0]
                .nodes
                .contains(&"corpus".to_string())
        );
        assert_eq!(
            parsed.graph.element_metadata["task"]["instruction"],
            "Read, then report"
        );
        assert!(
            parsed.graph.element_metadata["task"]
                .get("__proto__")
                .is_none()
        );
    }
    #[test]
    fn collapsing_a_flow_redirects_external_edges() {
        let parsed = parse("agentflow-beta\nA --> X\nflow work[Work]\nX --> Y\nend\nY --> B\nwork@{view: collapsed}").unwrap();
        assert!(!parsed.graph.nodes.contains_key("X"));
        assert!(!parsed.graph.nodes.contains_key("Y"));
        assert!(parsed.graph.subgraphs.is_empty());
        assert_eq!(parsed.graph.edges.len(), 2);
        assert!(
            parsed
                .graph
                .edges
                .iter()
                .any(|edge| edge.from == "A" && edge.to == "work")
        );
        assert!(
            parsed
                .graph
                .edges
                .iter()
                .any(|edge| edge.from == "work" && edge.to == "B")
        );
        assert!(parse("agentflow-beta\nflow work\nA").is_err());
    }
}
