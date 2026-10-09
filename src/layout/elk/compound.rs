//! Retain ordinary container ports when the native hierarchical transfer
//! mistakes a single edge-less external dummy for a self-loop source.
//!
//! Lay out the affected child as a root to recover that dummy's position.
//! Accept it only when its contents and every connected boundary port stay
//! put, then reroute the parent with those ports fixed. Both stages use ELK;
//! internal routes and semantic relationships are retained.
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

type Point = (f64, f64);

struct Endpoint {
    child: String,
    key: &'static str,
    port: Value,
    direct: bool,
    inside: Vec<Point>,
    original: String,
}

fn num(value: &Value, key: &str) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.01
}

fn points(edge: &Value) -> Option<Vec<Point>> {
    let sections = edge.get("sections")?.as_array()?;
    if sections.len() != 1 {
        return None;
    }
    let section = &sections[0];
    Some(
        section
            .get("startPoint")
            .into_iter()
            .chain(
                section
                    .get("bendPoints")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten(),
            )
            .chain(section.get("endPoint").into_iter())
            .map(|p| (num(p, "x"), num(p, "y")))
            .collect(),
    )
}

fn side(a: Point, b: Point) -> &'static str {
    if (b.0 - a.0).abs() > (b.1 - a.1).abs() {
        if b.0 > a.0 { "EAST" } else { "WEST" }
    } else if b.1 > a.1 {
        "SOUTH"
    } else {
        "NORTH"
    }
}

fn run_elk(input: &Value) -> Option<Value> {
    let result = elk_native::org::eclipse::elk::graph::json::layout_api::layout_json(
        &input.to_string(),
        "{}",
    )
    .ok()?;
    serde_json::from_str(&result).ok()
}

fn same_contents(a: &Value, b: &Value) -> bool {
    if !["width", "height"]
        .iter()
        .all(|key| close(num(a, key), num(b, key)))
    {
        return false;
    }
    match (
        a.get("children").and_then(Value::as_array),
        b.get("children").and_then(Value::as_array),
    ) {
        (None, None) => true,
        (Some(a), Some(b)) if a.len() == b.len() => a.iter().all(|child| {
            b.iter()
                .find(|other| other.get("id") == child.get("id"))
                .is_some_and(|other| {
                    close(num(child, "x"), num(other, "x"))
                        && close(num(child, "y"), num(other, "y"))
                        && same_contents(child, other)
                })
        }),
        _ => false,
    }
}

fn unique(used: &mut HashSet<String>, mut id: String) -> String {
    while !used.insert(id.clone()) {
        id.push('_');
    }
    id
}

/// Recover free source ports of top-level layered containers. Nested contents
/// are retained, but a nested affected container is left to the native engine.
pub(super) fn recover_container_ports(input: &Value, output: &Value) -> Option<Value> {
    let root = input.get("id")?.as_str()?;
    let children = input.get("children")?.as_array()?;
    let old_children: HashMap<_, _> = output
        .get("children")?
        .as_array()?
        .iter()
        .filter_map(|c| Some((c.get("id")?.as_str()?, c)))
        .collect();
    let old_edges: HashMap<_, _> = output
        .get("edges")?
        .as_array()?
        .iter()
        .filter_map(|e| Some((e.get("id")?.as_str()?, e)))
        .collect();
    let edges = input.get("edges")?.as_array()?;
    let mut used = HashSet::new();
    let mut owners = HashMap::<String, (String, String)>::new();
    fn collect(
        v: &Value,
        owner: &str,
        used: &mut HashSet<String>,
        owners: &mut HashMap<String, (String, String)>,
    ) {
        let Some(id) = v.get("id").and_then(Value::as_str) else {
            return;
        };
        used.insert(id.to_string());
        owners.insert(id.to_string(), (owner.to_string(), id.to_string()));
        for port in v
            .get("ports")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(p) = port.get("id").and_then(Value::as_str) {
                used.insert(p.to_string());
                owners.insert(p.to_string(), (owner.to_string(), id.to_string()));
            }
        }
        for child in v
            .get("children")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            collect(child, owner, used, owners);
        }
    }
    used.insert(root.to_string());
    for child in children {
        collect(child, child.get("id")?.as_str()?, &mut used, &mut owners);
    }
    for edge in edges {
        used.insert(edge.get("id")?.as_str()?.to_string());
    }
    let mut endpoints = HashMap::<String, Vec<Endpoint>>::new();
    let mut parent_edges = Vec::new();
    for edge in edges {
        let id = edge.get("id")?.as_str()?;
        let old = *old_edges.get(id)?;
        if old.get("container")?.as_str()? != root {
            continue;
        }
        let route = points(old)?;
        if route.len() < 2 {
            return None;
        }
        let mut collapsed = edge.clone();
        for key in ["sources", "targets"] {
            let original = edge.get(key)?.get(0)?.as_str()?;
            let (owner, node) = owners.get(original)?;
            let child = *old_children.get(owner.as_str())?;
            if child.get("children").is_none() {
                continue;
            }
            let direct = node == owner;
            let mut oriented = route.clone();
            if key == "targets" {
                oriented.reverse();
            }
            let x = num(child, "x");
            let y = num(child, "y");
            let width = num(child, "width");
            let height = num(child, "height");
            let boundary = if direct {
                0
            } else {
                oriented.iter().position(|&(px, py)| {
                    px < x - 0.01
                        || px > x + width + 0.01
                        || py < y - 0.01
                        || py > y + height + 0.01
                })?
            };
            let p = oriented[boundary];
            let port_side = if direct {
                side(p, oriented[1])
            } else {
                if boundary == 0 {
                    return None;
                }
                side(oriented[boundary - 1], p)
            };
            let border_offset = match port_side {
                "EAST" => p.0 - x - width,
                "WEST" => x - p.0,
                "SOUTH" => p.1 - y - height,
                _ => y - p.1,
            };
            let port_id = unique(&mut used, format!("__mmdr_compound_{id}_{key}"));
            collapsed[key] = json!([port_id]);
            endpoints.entry(id.to_string()).or_default().push(Endpoint {
                child: owner.clone(), key, direct, original: original.to_string(),
                port: json!({"id":port_id,"width":0,"height":0,"x":p.0-x,"y":p.1-y,
                    "layoutOptions":{"elk.port.side":port_side,"elk.port.borderOffset":border_offset}}),
                inside: oriented[..=boundary].to_vec(),
            });
        }
        parent_edges.push(collapsed);
    }
    let mut recovered = false;
    for child in children {
        let id = child.get("id")?.as_str()?;
        let Some(contents) = child.get("children").and_then(Value::as_array) else {
            continue;
        };
        if contents.iter().any(|c| c.get("children").is_some()) {
            continue;
        }
        // Follow declaration order, with connected boundary dummies before
        // the ordinary container ports, as the compound preprocessor does.
        let mut slots: Vec<_> = parent_edges
            .iter()
            .flat_map(|edge| {
                endpoints
                    .get(edge["id"].as_str().unwrap())
                    .into_iter()
                    .flatten()
            })
            .filter(|p| p.child == id)
            .collect();
        slots.sort_by_key(|p| p.direct);
        let outgoing: Vec<_> = slots
            .iter()
            .copied()
            .filter(|p| p.direct && p.key == "sources")
            .collect();
        if outgoing.len() != 1 {
            continue;
        }
        let ordinary = outgoing[0];
        // A real self-loop also has a target port on this container. It needs
        // the native self-loop handling, rather than this recovery.
        if endpoints.values().any(|ports| {
            ports.iter().any(|p| std::ptr::eq(p, ordinary))
                && ports.iter().any(|p| p.key == "targets" && p.child == id)
        }) {
            continue;
        }
        let direction = child
            .pointer("/layoutOptions/elk.direction")
            .or_else(|| input.pointer("/layoutOptions/elk.direction"))
            .and_then(Value::as_str)
            .unwrap_or("RIGHT");
        let flow_side = match direction {
            "DOWN" => "SOUTH",
            "UP" => "NORTH",
            "LEFT" => "WEST",
            _ => "EAST",
        };
        if ordinary
            .port
            .pointer("/layoutOptions/elk.port.side")?
            .as_str()?
            != flow_side
        {
            continue;
        }
        if child
            .pointer("/layoutOptions/elk.algorithm")
            .and_then(Value::as_str)
            .is_some_and(|a| a != "org.eclipse.elk.layered" && a != "elk.layered")
        {
            continue;
        }
        let ordinary_id = ordinary.port["id"].as_str()?.to_string();
        let mut probe = child.clone();
        probe["layoutOptions"]["elk.algorithm"] = json!("org.eclipse.elk.layered");
        probe["layoutOptions"]["elk.direction"] = json!(direction);
        probe["layoutOptions"]["elk.portConstraints"] = json!("FIXED_ORDER");
        probe["ports"] = Value::Array(
            slots
                .iter()
                .map(|p| {
                    let mut port = p.port.clone();
                    port.as_object_mut().unwrap().remove("x");
                    port.as_object_mut().unwrap().remove("y");
                    port
                })
                .collect(),
        );
        let mut internal: Vec<_> = edges
            .iter()
            .filter(|e| {
                old_edges
                    .get(e["id"].as_str().unwrap())
                    .is_some_and(|e| e.get("container").and_then(Value::as_str) == Some(id))
            })
            .cloned()
            .collect();
        for slot in slots.iter().filter(|p| !p.direct) {
            let port_id = slot.port["id"].as_str()?;
            let (source, target) = if slot.key == "sources" {
                (slot.original.as_str(), port_id)
            } else {
                (port_id, slot.original.as_str())
            };
            internal.push(json!({"id":unique(&mut used, format!("{port_id}_inside")),"sources":[source],"targets":[target]}));
        }
        probe["edges"] = json!(internal);
        let Some(probed) = run_elk(&probe) else {
            continue;
        };
        let old = *old_children.get(id)?;
        if !same_contents(old, &probed) {
            continue;
        }
        let Some(ports) = probed.get("ports").and_then(Value::as_array) else {
            continue;
        };
        let port_by_id: HashMap<_, _> = ports
            .iter()
            .filter_map(|p| Some((p.get("id")?.as_str()?, p)))
            .collect();
        if !slots.iter().filter(|p| !p.direct).all(|p| {
            port_by_id
                .get(p.port["id"].as_str().unwrap())
                .is_some_and(|other| {
                    close(num(&p.port, "x"), num(other, "x"))
                        && close(num(&p.port, "y"), num(other, "y"))
                })
        }) {
            continue;
        }
        let Some(correct) = port_by_id.get(ordinary_id.as_str()) else {
            continue;
        };
        let position = (num(correct, "x"), num(correct, "y"));
        if close(position.0, num(&ordinary.port, "x"))
            && close(position.1, num(&ordinary.port, "y"))
        {
            continue;
        }
        for slot in endpoints.values_mut().flatten() {
            if slot.port["id"].as_str() == Some(&ordinary_id) {
                slot.port["x"] = json!(position.0);
                slot.port["y"] = json!(position.1);
            }
        }
        recovered = true;
    }
    if !recovered {
        return None;
    }
    let mut flat = input.clone();
    flat["edges"] = json!(parent_edges);
    let collapsed_edges = flat["edges"].clone();
    for child in flat["children"].as_array_mut()? {
        let id = child["id"].as_str()?;
        let old = *old_children.get(id)?;
        if old.get("children").is_some() {
            let ports: Vec<_> = flat_ports(&endpoints, &collapsed_edges, id);
            let child = child.as_object_mut()?;
            child.remove("children");
            child.insert("width".into(), old["width"].clone());
            child.insert("height".into(), old["height"].clone());
            child.insert("ports".into(), json!(ports));
            child.insert(
                "layoutOptions".into(),
                json!({"elk.portConstraints":"FIXED_POS"}),
            );
        }
    }
    let parent = run_elk(&flat)?;
    let mut result = output.clone();
    for key in ["width", "height", "x", "y"] {
        if let Some(value) = parent.get(key) {
            result[key] = value.clone();
        }
    }
    let parent_children: HashMap<_, _> = parent
        .get("children")?
        .as_array()?
        .iter()
        .filter_map(|c| Some((c.get("id")?.as_str()?, c)))
        .collect();
    for child in result["children"].as_array_mut()? {
        let new = *parent_children.get(child["id"].as_str()?)?;
        if !close(num(child, "width"), num(new, "width"))
            || !close(num(child, "height"), num(new, "height"))
        {
            return None;
        }
        child["x"] = new["x"].clone();
        child["y"] = new["y"].clone();
    }
    let parent_edges: HashMap<_, _> = parent
        .get("edges")?
        .as_array()?
        .iter()
        .filter_map(|e| Some((e.get("id")?.as_str()?, e)))
        .collect();
    for edge in result["edges"].as_array_mut()? {
        let id = edge["id"].as_str()?;
        let Some(new) = parent_edges.get(id) else {
            continue;
        };
        let mut route = points(new)?;
        for slot in endpoints
            .get(id)
            .into_iter()
            .flatten()
            .filter(|p| !p.direct)
        {
            let old_child = *old_children.get(slot.child.as_str())?;
            let new_child = *parent_children.get(slot.child.as_str())?;
            let delta = (
                num(new_child, "x") - num(old_child, "x"),
                num(new_child, "y") - num(old_child, "y"),
            );
            let mut inside: Vec<_> = slot
                .inside
                .iter()
                .map(|p| (p.0 + delta.0, p.1 + delta.1))
                .collect();
            let boundary = if slot.key == "sources" {
                *route.first()?
            } else {
                *route.last()?
            };
            let end = *inside.last()?;
            if !close(boundary.0, end.0) || !close(boundary.1, end.1) {
                return None;
            }
            inside.pop();
            if slot.key == "sources" {
                inside.extend(route);
                route = inside;
            } else {
                inside.reverse();
                route.extend(inside);
            }
        }
        let section = &mut edge["sections"][0];
        let point = |p: Point| json!({"x":p.0,"y":p.1});
        section["startPoint"] = point(*route.first()?);
        section["endPoint"] = point(*route.last()?);
        section["bendPoints"] = json!(
            route[1..route.len() - 1]
                .iter()
                .copied()
                .map(point)
                .collect::<Vec<_>>()
        );
        if let Some(labels) = new.get("labels") {
            edge["labels"] = labels.clone();
        }
    }
    Some(result)
}

fn flat_ports(
    endpoints: &HashMap<String, Vec<Endpoint>>,
    edges: &Value,
    child: &str,
) -> Vec<Value> {
    edges
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|e| {
            endpoints
                .get(e["id"].as_str().unwrap())
                .into_iter()
                .flatten()
        })
        .filter(|p| p.child == child)
        .map(|p| p.port.clone())
        .collect()
}
