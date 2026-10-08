//! Logical statements and metadata shared by Mermaid's extended graph syntaxes.
use super::*;
use serde::de::Error as _;

pub(super) fn statements(input: &str) -> Result<(Vec<String>, Option<serde_json::Value>)> {
    let (_, config) = preprocess_input(input)?;
    let body = extract_yaml_frontmatter(input).1;
    let mut result = Vec::new();
    let mut pending = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 0i32;
    for raw in body.lines() {
        if pending.is_empty() && (raw.trim().is_empty() || raw.trim().starts_with("%%")) {
            continue;
        }
        if !pending.is_empty() {
            pending.push('\n');
        }
        pending.push_str(raw);
        for ch in raw.chars() {
            if escaped {
                escaped = false;
                continue;
            }
            if let Some(q) = quote {
                if ch == '\\' {
                    escaped = true;
                } else if ch == q {
                    quote = None;
                }
            } else {
                match ch {
                    '"' | '\'' => quote = Some(ch),
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
            }
        }
        if depth < 0 {
            anyhow::bail!("Unexpected closing metadata brace");
        }
        if quote.is_none() && depth == 0 {
            result.push(pending.trim().to_string());
            pending.clear();
        }
    }
    if !pending.is_empty() {
        anyhow::bail!("Unterminated quoted label or metadata block");
    }
    Ok((result, config))
}

pub(super) fn outside_token(text: &str, token: &str) -> Option<usize> {
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if let Some(q) = quote {
            if ch == '\\' {
                escaped = true;
            } else if ch == q {
                quote = None;
            }
        } else if text[index..].starts_with(token) {
            return Some(index);
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch);
        }
    }
    None
}

pub(super) fn metadata(text: &str) -> Result<(String, Option<serde_json::Value>)> {
    let Some(start) = outside_token(text, "@{") else {
        return Ok((text.to_string(), None));
    };
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 1;
    let mut end = None;
    for (offset, ch) in text[start + 2..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if let Some(q) = quote {
            if ch == '\\' {
                escaped = true;
            } else if ch == q {
                quote = None;
            }
        } else {
            match ch {
                '"' | '\'' => quote = Some(ch),
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(start + 2 + offset);
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    let end = end.ok_or_else(|| anyhow::anyhow!("Unterminated metadata block"))?;
    let body = &text[start + 2..end];
    let wrapped = format!("{{{body}}}");
    let value = serde_yaml::from_str::<serde_json::Value>(&wrapped)
        .or_else(|_| serde_yaml::from_str(body))
        .or_else(|_| json5::from_str(&wrapped).map_err(serde_yaml::Error::custom));
    let value = value.map_err(|error| anyhow::anyhow!("Invalid metadata: {error}"))?;
    if !value.is_object() {
        anyhow::bail!("Metadata must be a mapping");
    }
    Ok((
        format!("{}{}", &text[..start], &text[end + 1..]),
        Some(value),
    ))
}

pub(super) fn merge_metadata(graph: &mut Graph, id: &str, mut value: serde_json::Value) {
    fn clean(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                for key in ["__proto__", "constructor", "prototype"] {
                    map.remove(key);
                }
                for child in map.values_mut() {
                    clean(child);
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    clean(child);
                }
            }
            _ => {}
        }
    }
    clean(&mut value);
    let entry = graph
        .element_metadata
        .entry(id.to_string())
        .or_insert_with(|| serde_json::json!({}));
    if let (Some(current), Some(incoming)) = (entry.as_object_mut(), value.as_object()) {
        current.extend(incoming.clone());
    }
}

pub(super) fn map_config(config: &mut Option<serde_json::Value>, namespace: &str) {
    if let Some(config) = config.as_mut()
        && let Some(options) = config.get(namespace).and_then(|v| v.as_object()).cloned()
        && let Some(root) = config.as_object_mut()
    {
        let flowchart = root
            .entry("flowchart")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(flowchart) = flowchart.as_object_mut() {
            for key in ["nodeSpacing", "rankSpacing", "wrappingWidth"] {
                if let Some(value) = options.get(key) {
                    flowchart.insert(key.to_string(), value.clone());
                }
            }
        }
    }
}

pub(super) fn identifier(text: &str) -> (&str, &str) {
    let end = text
        .char_indices()
        .find(|(_, ch)| !ch.is_ascii_alphanumeric() && *ch != '_')
        .map(|(index, _)| index)
        .unwrap_or(text.len());
    (&text[..end], &text[end..])
}

pub(super) fn split_classes(text: &str) -> (String, Vec<String>) {
    let Some(start) = outside_token(text, ":::") else {
        return (text.trim().to_string(), Vec::new());
    };
    let classes = text[start + 3..]
        .split(":::")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect();
    (text[..start].trim().to_string(), classes)
}

pub(super) fn declaration(
    text: &str,
) -> Result<(String, String, bool, bool, Vec<String>, Option<String>)> {
    let (without_classes, classes) = split_classes(text);
    let mut base = without_classes.trim().to_string();
    let stereotype = if let Some(start) = outside_token(&base, "<<") {
        let end = base[start + 2..]
            .find(">>")
            .map(|n| start + 2 + n)
            .ok_or_else(|| anyhow::anyhow!("Unterminated stereotype"))?;
        let value = base[start + 2..end].trim().to_string();
        if !base[end + 2..].trim().is_empty() {
            anyhow::bail!("Unexpected text after stereotype");
        }
        base.truncate(start);
        Some(value)
    } else {
        None
    };
    let base = base.trim();
    if base.starts_with(['"', '\'']) {
        let (label, tail) = split_leading_quoted(base)
            .ok_or_else(|| anyhow::anyhow!("Invalid quoted declaration"))?;
        if !tail.trim().is_empty() {
            anyhow::bail!("Unexpected text after declaration");
        }
        let id = label
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    ch
                } else {
                    '_'
                }
            })
            .collect();
        let (label, md) = strip_quotes_markdown(base);
        return Ok((id, label, md, false, classes, stereotype));
    }
    let (id, tail) = identifier(base);
    if id.is_empty() {
        anyhow::bail!("Expected an element identifier: {base}");
    }
    let tail = tail.trim();
    let rectangular = tail.starts_with('[');
    let (label, md) = if tail.starts_with(['(', '[']) {
        let closing = if rectangular { ']' } else { ')' };
        if !tail.ends_with(closing) {
            anyhow::bail!("Unterminated element label: {base}");
        }
        strip_quotes_markdown(tail[1..tail.len() - 1].trim())
    } else if tail.is_empty() {
        (id.to_string(), false)
    } else {
        anyhow::bail!("Unexpected text after element identifier: {base}");
    };
    Ok((id.to_string(), label, md, rectangular, classes, stereotype))
}

pub(super) fn edge_styles(graph: &mut Graph) {
    for (index, edge) in graph.edges.iter().enumerate() {
        let classes = std::iter::once("default").chain(
            edge.id
                .as_ref()
                .and_then(|id| graph.node_classes.get(id))
                .into_iter()
                .flatten()
                .map(String::as_str),
        );
        let styles = classes
            .filter_map(|name| graph.class_defs.get(name))
            .chain(edge.id.as_ref().and_then(|id| graph.node_styles.get(id)));
        let target = graph.edge_styles.entry(index).or_default();
        for style in styles {
            if style.stroke.is_some() {
                target.stroke = style.stroke.clone();
            }
            if style.stroke_width.is_some() {
                target.stroke_width = style.stroke_width;
            }
            if style.stroke_dasharray.is_some() {
                target.dasharray = style.stroke_dasharray.clone();
            }
            if style.text_color.is_some() {
                target.label_color = style.text_color.clone();
            }
        }
    }
}
