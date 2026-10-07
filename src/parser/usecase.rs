use super::{extended::*, *};
use crate::ir::{EdgeArrowhead, NodeShape, UseCaseActorType, UseCaseNodeData};
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::HashSet;

pub(super) fn parse(input: &str) -> Result<ParseOutput> {
    let (lines, mut init_config) = statements(input)?;
    map_config(&mut init_config, "usecase");
    let mut graph = Graph::new();
    graph.kind = DiagramKind::UseCase;
    graph.direction = Direction::LeftRight;
    graph.usecase.config = init_config
        .as_ref()
        .and_then(|value| value.get("usecase"))
        .cloned()
        .unwrap_or_default();
    let mut boundary = None;
    let mut declared = HashMap::<String, &'static str>::new();
    let mut relationships = Vec::new();
    let mut notes = Vec::new();
    for line in lines {
        if line == "usecase-beta" {
            continue;
        }
        if line == "end" {
            if boundary.take().is_none() {
                anyhow::bail!("Unexpected use case boundary end");
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("accTitle:") {
            graph.acc_title = Some(rest.trim().to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("accDescr:") {
            graph.acc_descr = Some(rest.trim().to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("accDescr") {
            if let Some(body) = rest
                .trim()
                .strip_prefix('{')
                .and_then(|s| s.strip_suffix('}'))
            {
                graph.acc_descr = Some(body.trim().to_string());
                continue;
            }
        }
        if let Some(direction) = parse_direction_line(&line) {
            if boundary.is_some() {
                anyhow::bail!("Direction statements must be outside a system boundary");
            }
            graph.direction = direction;
            continue;
        }
        for prefix in ["classDef ", "class ", "style "] {
            if line.starts_with(prefix) && boundary.is_some() {
                anyhow::bail!("Styles must be outside a system boundary");
            }
        }
        if line.starts_with("classDef ") {
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

        if let Some(rest) = line.strip_prefix("json ") {
            if boundary.is_some() {
                anyhow::bail!("JSON tables must be outside a system boundary");
            }
            let (base, classes) = split_classes(rest);
            let (id, tail) = identifier(base.trim());
            let body = tail
                .trim()
                .strip_prefix("@{")
                .and_then(|s| s.strip_suffix('}'))
                .ok_or_else(|| anyhow::anyhow!("JSON table requires an object: {line}"))?;
            register(&mut declared, id, "json")?;
            let value: OrderedJson = serde_json::from_str(&format!("{{{body}}}"))?;
            let OrderedJson::Object(_) = value else {
                anyhow::bail!("JSON table root must be an object");
            };
            let mut rows = Vec::new();
            value.flatten("", &mut rows);
            graph.usecase.json_tables.insert(id.to_string(), rows);
            graph.ensure_node(id, Some(id.to_string()), Some(NodeShape::Rectangle));
            apply_node_classes(&mut graph, id, &classes);
            continue;
        }

        if let Some(rest) = line.strip_prefix("note for ") {
            if boundary.is_some() {
                anyhow::bail!("Notes must be outside a system boundary");
            }
            let (target, text) = identifier(rest.trim());
            let text = text.trim();
            if target.is_empty() || !text.starts_with(['"', '\'']) {
                anyhow::bail!("Invalid use case note: {line}");
            }
            let (label, md) = strip_quotes_markdown(text);
            let id = format!("__usecase_note_{}", notes.len());
            graph.ensure_node_md(&id, Some(label), Some(NodeShape::Note), md);
            ensure_case(&mut graph, target);
            if !add_flowchart_edge(&format!("{id} -.- {target}"), &mut graph, &[]) {
                anyhow::bail!("Invalid use case note attachment");
            }
            notes.push(target.to_string());
            continue;
        }

        if let Some(rest) = line.strip_prefix("systemBoundary ") {
            if boundary.is_some() {
                anyhow::bail!("System boundaries cannot be nested");
            }
            let (base, metadata) = metadata(rest)?;
            let (id, label, md, _, classes, stereotype) = declaration(&base)?;
            if stereotype.is_some() {
                anyhow::bail!("System boundaries cannot have stereotypes");
            }
            register(&mut declared, &id, "boundary")?;
            if let Some(metadata) = metadata {
                merge_metadata(&mut graph, &id, metadata);
            }
            graph.subgraphs.push(Subgraph {
                id: Some(id.clone()),
                label,
                nodes: Vec::new(),
                direction: None,
                icon: None,
                markdown_label: md,
            });
            apply_subgraph_classes(&mut graph, &id, &classes);
            boundary = Some(graph.subgraphs.len() - 1);
            continue;
        }

        if let Some(edge) = relationship(&line)? {
            if boundary.is_some() {
                anyhow::bail!("Relationships must be outside a system boundary");
            }
            let (normalized, kind) = edge;
            let index = graph.edges.len();
            if !add_flowchart_edge(&normalized, &mut graph, &[]) {
                anyhow::bail!("Invalid use case relationship: {line}");
            }
            if graph.edges.len() != index + 1 {
                anyhow::bail!("Use case relationships require exactly two endpoints");
            }
            let edge = &mut graph.edges[index];
            if let Some(id) = edge.id.as_ref() {
                if declared.contains_key(id) {
                    anyhow::bail!("Duplicate use case edge identifier: {id}");
                }
                register(&mut declared, id, "edge")?;
            }
            if kind == "generalization" {
                edge.arrow_end_kind = Some(EdgeArrowhead::OpenTriangle);
            }
            if kind == "include" || kind == "extend" {
                edge.style = crate::ir::EdgeStyle::Dotted;
            }
            relationships.push((index, kind));
            continue;
        }

        let actor = line.starts_with("actor ");
        let rest = line.strip_prefix("actor ").unwrap_or(&line);
        let (base, metadata) = metadata(rest)?;
        let (id, label, md, rectangular, classes, stereotype) = declaration(&base)?;
        if let Some(metadata) = metadata {
            merge_metadata(&mut graph, &id, metadata);
        }
        let metadata_only = !actor
            && label == id
            && !rectangular
            && stereotype.is_none()
            && graph.element_metadata.contains_key(&id)
            && base.trim() == id;
        if metadata_only {
            continue;
        }
        register(&mut declared, &id, if actor { "actor" } else { "usecase" })?;
        let shape = if actor {
            NodeShape::StickFigure
        } else if rectangular {
            NodeShape::Rectangle
        } else {
            NodeShape::Ellipse
        };
        graph.ensure_node_md(&id, Some(label), Some(shape), md);
        let details = graph.usecase.nodes.entry(id.clone()).or_default();
        details.actor_type = actor.then_some(UseCaseActorType::Normal);
        if stereotype.is_some() {
            details.stereotype = stereotype;
        }
        apply_node_classes(&mut graph, &id, &classes);
        if let Some(index) = boundary {
            if graph
                .subgraphs
                .iter()
                .enumerate()
                .any(|(i, sub)| i != index && sub.nodes.contains(&id))
            {
                anyhow::bail!("An element can belong to only one system boundary: {id}");
            }
            if !graph.subgraphs[index].nodes.contains(&id) {
                graph.subgraphs[index].nodes.push(id);
            }
        }
    }
    if boundary.is_some() {
        anyhow::bail!("Unterminated system boundary");
    }
    for node in graph.nodes.values_mut() {
        if !declared.contains_key(&node.id) && !node.id.starts_with("__usecase_note_") {
            node.shape = NodeShape::Ellipse;
        }
    }
    apply_metadata(&mut graph)?;
    for target in notes {
        if graph.usecase.json_tables.contains_key(&target)
            || declared.get(&target) == Some(&"boundary")
            || !graph.nodes.contains_key(&target)
        {
            anyhow::bail!("Notes must attach to an actor or use case: {target}");
        }
    }
    for (index, kind) in relationships {
        let edge = &graph.edges[index];
        if declared.get(&edge.from) == Some(&"boundary")
            || declared.get(&edge.to) == Some(&"boundary")
        {
            anyhow::bail!("Relationships cannot attach to a system boundary");
        }
        let from_actor = graph
            .usecase
            .nodes
            .get(&edge.from)
            .and_then(|n| n.actor_type)
            .is_some();
        let to_actor = graph
            .usecase
            .nodes
            .get(&edge.to)
            .and_then(|n| n.actor_type)
            .is_some();
        let json = graph.usecase.json_tables.contains_key(&edge.from)
            || graph.usecase.json_tables.contains_key(&edge.to);
        if (kind == "include" || kind == "extend") && (from_actor || to_actor || json) {
            anyhow::bail!("{kind} requires two use case endpoints");
        }
        if kind == "generalization" && (from_actor != to_actor || json) {
            anyhow::bail!("Generalization requires two actors or two use cases");
        }
        if json && (edge.start_decoration.is_some() || edge.end_decoration.is_some()) {
            anyhow::bail!("JSON tables support only point or markerless associations");
        }
    }
    edge_styles(&mut graph);
    Ok(ParseOutput { graph, init_config })
}

fn register(
    symbols: &mut HashMap<String, &'static str>,
    id: &str,
    kind: &'static str,
) -> Result<()> {
    if id.is_empty() {
        anyhow::bail!("Expected an element identifier");
    }
    if let Some(previous) = symbols.insert(id.to_string(), kind)
        && previous != kind
    {
        anyhow::bail!("Identifier {id} is already declared as {previous}");
    }
    Ok(())
}

fn ensure_case(graph: &mut Graph, id: &str) {
    if !graph.nodes.contains_key(id) {
        graph.ensure_node(id, None, Some(NodeShape::Ellipse));
    }
}

fn relationship(line: &str) -> Result<Option<(String, &'static str)>> {
    if let Some(at) = outside_token(line, "..>") {
        let left = line[..at].trim();
        let right = line[at + 3..]
            .trim()
            .strip_prefix(':')
            .map(str::trim)
            .ok_or_else(|| anyhow::anyhow!("Expected include or extend after ..>"))?;
        let (kind, target) = right
            .split_once(char::is_whitespace)
            .ok_or_else(|| anyhow::anyhow!("Expected a use case relationship target"))?;
        if !matches!(kind, "include" | "extend") {
            anyhow::bail!("Expected include or extend after ..>");
        }
        let kind = if kind == "include" {
            "include"
        } else {
            "extend"
        };
        // The shared edge reader understands IDs before a dash-based operator.
        let normalized = format!("{left} -- {kind} --> {target}").replace("@ --", "@--");
        return Ok(Some((normalized, kind)));
    }
    if let Some(at) = outside_token(line, "--|>") {
        return Ok(Some((
            format!("{} --> {}", line[..at].trim(), line[at + 4..].trim()).replace("@ -->", "@-->"),
            "generalization",
        )));
    }
    if ["--", "<--", "o--", "x--"]
        .iter()
        .any(|token| outside_token(line, token).is_some())
    {
        return Ok(Some((line.to_string(), "association")));
    }
    Ok(None)
}

fn apply_metadata(graph: &mut Graph) -> Result<()> {
    for (id, metadata) in graph.element_metadata.clone() {
        if graph
            .subgraphs
            .iter()
            .any(|sub| sub.id.as_deref() == Some(&id))
        {
            if metadata.get("type").and_then(|v| v.as_str()) == Some("package") {
                graph.usecase.packages.insert(id);
            } else if metadata.get("type").is_some()
                && metadata.get("type").and_then(|v| v.as_str()) != Some("rect")
            {
                anyhow::bail!("Unknown system boundary type");
            }
            continue;
        }
        if let Some(edge) = graph
            .edges
            .iter_mut()
            .find(|edge| edge.id.as_deref() == Some(&id))
        {
            edge.curve = metadata
                .get("curve")
                .and_then(|v| v.as_str())
                .and_then(crate::ir::CurveType::from_name);
            continue;
        }
        let Some(node) = graph.nodes.get_mut(&id) else {
            anyhow::bail!("Metadata references an undeclared element: {id}");
        };
        let details = graph
            .usecase
            .nodes
            .entry(id.clone())
            .or_insert_with(UseCaseNodeData::default);
        details.business = metadata
            .get("business")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if details.actor_type.is_some() {
            let allowed: HashSet<_> = ["type", "icon", "business"].into_iter().collect();
            if metadata
                .as_object()
                .unwrap()
                .keys()
                .any(|key| !allowed.contains(key.as_str()))
            {
                anyhow::bail!("Actor metadata accepts only type, icon, and business");
            }
            details.actor_type = Some(
                match metadata
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("normal")
                {
                    "normal" => UseCaseActorType::Normal,
                    "hollow" => UseCaseActorType::Hollow,
                    "awesome" => UseCaseActorType::Awesome,
                    value => anyhow::bail!("Unknown actor type: {value}"),
                },
            );
            if let Some(icon) = metadata.get("icon").and_then(|v| v.as_str()) {
                if details.actor_type != Some(UseCaseActorType::Normal) || details.business {
                    anyhow::bail!("Icon actors cannot combine type or business variants");
                }
                details.actor_type = Some(UseCaseActorType::Icon);
                node.icon = Some(icon.to_string());
            }
            if details.business && details.actor_type == Some(UseCaseActorType::Awesome) {
                anyhow::bail!("Awesome actors cannot be business actors");
            }
        } else if details.business && node.shape != NodeShape::Ellipse {
            anyhow::bail!("Only ellipse use cases can be business elements");
        }
    }
    Ok(())
}

// serde_json's default map sorts keys. A visitor retains source order, including
// integer-like keys, and replaces duplicate values without moving their row.
#[derive(Debug)]
enum OrderedJson {
    Object(Vec<(String, OrderedJson)>),
    Array(Vec<OrderedJson>),
    Scalar(serde_json::Value),
}
impl<'de> Deserialize<'de> for OrderedJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct OrderedVisitor;
        impl<'de> Visitor<'de> for OrderedVisitor {
            type Value = OrderedJson;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a JSON value")
            }
            fn visit_map<M: MapAccess<'de>>(
                self,
                mut map: M,
            ) -> std::result::Result<Self::Value, M::Error> {
                let mut values = Vec::<(String, OrderedJson)>::new();
                while let Some((key, value)) = map.next_entry::<String, OrderedJson>()? {
                    if let Some(entry) = values.iter_mut().find(|entry| entry.0 == key) {
                        entry.1 = value;
                    } else {
                        values.push((key, value));
                    }
                }
                Ok(OrderedJson::Object(values))
            }
            fn visit_seq<S: SeqAccess<'de>>(
                self,
                mut seq: S,
            ) -> std::result::Result<Self::Value, S::Error> {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element()? {
                    values.push(value);
                }
                Ok(OrderedJson::Array(values))
            }
            fn visit_bool<E: serde::de::Error>(
                self,
                value: bool,
            ) -> std::result::Result<Self::Value, E> {
                Ok(OrderedJson::Scalar(value.into()))
            }
            fn visit_i64<E: serde::de::Error>(
                self,
                value: i64,
            ) -> std::result::Result<Self::Value, E> {
                Ok(OrderedJson::Scalar(value.into()))
            }
            fn visit_u64<E: serde::de::Error>(
                self,
                value: u64,
            ) -> std::result::Result<Self::Value, E> {
                Ok(OrderedJson::Scalar(value.into()))
            }
            fn visit_f64<E: serde::de::Error>(
                self,
                value: f64,
            ) -> std::result::Result<Self::Value, E> {
                Ok(OrderedJson::Scalar(value.into()))
            }
            fn visit_str<E: serde::de::Error>(
                self,
                value: &str,
            ) -> std::result::Result<Self::Value, E> {
                Ok(OrderedJson::Scalar(value.into()))
            }
            fn visit_string<E: serde::de::Error>(
                self,
                value: String,
            ) -> std::result::Result<Self::Value, E> {
                Ok(OrderedJson::Scalar(value.into()))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(OrderedJson::Scalar(serde_json::Value::Null))
            }
        }
        deserializer.deserialize_any(OrderedVisitor)
    }
}
impl OrderedJson {
    fn flatten(&self, path: &str, rows: &mut Vec<(String, String)>) {
        match self {
            Self::Object(entries) if !entries.is_empty() => {
                for (key, value) in entries {
                    value.flatten(
                        &if path.is_empty() {
                            key.clone()
                        } else {
                            format!("{path}.{key}")
                        },
                        rows,
                    );
                }
            }
            Self::Array(values) if !values.is_empty() => {
                let scalar_array = values.iter().all(|value| matches!(value, Self::Scalar(_)));
                for (index, value) in values.iter().enumerate() {
                    if scalar_array {
                        value.flatten(if index == 0 { path } else { "" }, rows);
                    } else {
                        value.flatten(&format!("{path}[{index}]"), rows);
                    }
                }
            }
            Self::Object(_) => rows.push((path.to_string(), "{}".to_string())),
            Self::Array(_) => rows.push((path.to_string(), "[]".to_string())),
            Self::Scalar(value) => rows.push((
                path.to_string(),
                value
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| value.to_string()),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forward_actor_and_uml_relationships_keep_their_semantics() {
        let parsed = parse(
            "usecase-beta\nUser --> Login\nactor User\nLogin ..> : include Check\nLogin --|> Check",
        )
        .unwrap();
        assert_eq!(parsed.graph.nodes["User"].shape, NodeShape::StickFigure);
        assert_eq!(parsed.graph.nodes["Login"].shape, NodeShape::Ellipse);
        assert_eq!(parsed.graph.edges[1].label.as_deref(), Some("include"));
        assert_eq!(parsed.graph.edges[1].style, crate::ir::EdgeStyle::Dotted);
        assert_eq!(
            parsed.graph.edges[2].arrow_end_kind,
            Some(EdgeArrowhead::OpenTriangle)
        );
        assert!(parse("usecase-beta\nactor User\nUser ..> : include Login").is_err());
    }
    #[test]
    fn json_rows_preserve_order_duplicates_and_nested_leaves() {
        let parsed = parse("usecase-beta\njson Data@{\"2\":1,\"1\":2,\"2\":3,\"items\":[{\"name\":\"Book\"}],\"empty\":[]}").unwrap();
        assert_eq!(
            parsed.graph.usecase.json_tables["Data"],
            vec![
                ("2".into(), "3".into()),
                ("1".into(), "2".into()),
                ("items[0].name".into(), "Book".into()),
                ("empty".into(), "[]".into())
            ]
        );
    }
    #[test]
    fn boundary_and_actor_metadata_are_typed() {
        let parsed = parse("usecase-beta\nsystemBoundary App[Application]@{type: package}\nactor Staff@{type: hollow, business: true} <<Employee>>\nLogin(Sign in)\nend\nStaff --> Login").unwrap();
        assert!(parsed.graph.usecase.packages.contains("App"));
        assert_eq!(
            parsed.graph.usecase.nodes["Staff"].actor_type,
            Some(UseCaseActorType::Hollow)
        );
        assert!(parsed.graph.usecase.nodes["Staff"].business);
        assert!(parse("usecase-beta\nactor Staff@{fillColor: red}").is_err());
        assert!(parse("usecase-beta\nLogin[Sign in]@{business: true}").is_err());
    }
}
