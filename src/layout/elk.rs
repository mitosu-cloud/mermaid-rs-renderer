//! Adapter to the native ELK port. Keep Mermaid's compound graph and its
//! directed reference edges intact through the complete layered pipeline.
use super::{
    Layout,
    routing::{self, EdgeSide},
};
use crate::ir::{DiagramKind, Direction, Graph};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

mod compound;
mod ports;

fn direction(dir: Direction) -> &'static str {
    match dir {
        Direction::TopDown => "DOWN",
        Direction::BottomTop => "UP",
        Direction::LeftRight => "RIGHT",
        Direction::RightLeft => "LEFT",
    }
}

fn number(v: &Value, key: &str) -> f32 {
    v.get(key).and_then(Value::as_f64).unwrap_or(0.0) as f32
}

fn layout_options(graph: &Graph, dir: Direction, group: bool) -> Value {
    let config = graph
        .agentflow_config
        .as_ref()
        .unwrap_or(&graph.appearance_config);
    let elk = config.get("elk").unwrap_or(&Value::Null);
    let preset = elk
        .get("preset")
        .and_then(Value::as_str)
        .unwrap_or("default");
    let (placement, alignment, cycles) = match preset {
        "legacy" => ("BRANDES_KOEPF", "NONE", "GREEDY"),
        "modelOrder" => (
            if group {
                "BRANDES_KOEPF"
            } else {
                "NETWORK_SIMPLEX"
            },
            "NONE",
            "GREEDY_MODEL_ORDER",
        ),
        "depthFirst" => (
            if group {
                "BRANDES_KOEPF"
            } else {
                "NETWORK_SIMPLEX"
            },
            "NONE",
            "DEPTH_FIRST",
        ),
        _ => ("BRANDES_KOEPF", "BALANCED", "DEPTH_FIRST"),
    };
    let option = |key: &str, default: &str| elk.get(key).cloned().unwrap_or_else(|| json!(default));
    let mut result = json!({
        "elk.algorithm": "org.eclipse.elk.layered",
        "elk.hierarchyHandling": "INCLUDE_CHILDREN",
        "elk.direction": direction(dir),
        "spacing.baseValue": if group { 24 } else { 40 },
        "elk.layered.nodePlacement.strategy": option("nodePlacementStrategy", placement),
        "elk.layered.nodePlacement.bk.fixedAlignment": option("nodePlacementAlignment", alignment),
        "elk.layered.cycleBreaking.strategy": option("cycleBreakingStrategy", cycles),
        "elk.layered.layering.strategy": option("layeringStrategy", "NETWORK_SIMPLEX"),
        "elk.layered.unnecessaryBendpoints": true,
        "elk.layered.considerModelOrder.strategy": "NODES_AND_EDGES",
        "elk.layered.crossingMinimization.forceNodeModelOrder": false,
        "elk.layered.mergeHierarchyEdges": true,
        "elk.layered.wrapping.multiEdge.improveCuts": true,
        "elk.layered.wrapping.multiEdge.improveWrappedEdges": true,
        "elk.layered.edgeRouting.selfLoopDistribution": "EQUALLY",
        "elk.spacing.portsSurrounding": "[top=12,left=12,bottom=12,right=12]"
    });
    // The native port exposes baseValue but does not expand its dependent
    // spacings yet. Materialize ELK's ratios so the same Mermaid option has
    // the same effect in both engines.
    let base = if group { 24.0 } else { 40.0 };
    for (key, factor) in [
        ("spacing.nodeNode", 1.0),
        ("elk.spacing.componentComponent", 1.0),
        ("elk.spacing.edgeEdge", 0.5),
        ("elk.spacing.edgeLabel", 0.1),
        ("elk.spacing.edgeNode", 0.5),
        ("elk.spacing.labelLabel", 0.0),
        ("elk.spacing.labelNode", 0.25),
        ("elk.spacing.labelPortHorizontal", 0.05),
        ("elk.spacing.labelPortVertical", 0.05),
        ("elk.spacing.nodeSelfLoop", 0.5),
        ("elk.spacing.portPort", 0.5),
        ("elk.layered.spacing.edgeEdgeBetweenLayers", 0.5),
        ("elk.layered.spacing.edgeNodeBetweenLayers", 0.5),
        ("elk.layered.spacing.nodeNodeBetweenLayers", 1.0),
    ] {
        result[key] = json!(base * factor);
    }
    for (key, target) in [
        ("mergeEdges", "elk.layered.mergeEdges"),
        (
            "forceNodeModelOrder",
            "elk.layered.crossingMinimization.forceNodeModelOrder",
        ),
        (
            "considerModelOrder",
            "elk.layered.considerModelOrder.strategy",
        ),
        (
            "layeringLayerBound",
            "elk.layered.layering.coffmanGraham.layerBound",
        ),
    ] {
        if let Some(value) = elk.get(key) {
            result[target] = value.clone();
        }
    }
    if group {
        // Mermaid configures these on the root only. Repeating model-order
        // options on every container changes compound crossing minimization.
        for key in [
            "elk.algorithm",
            "elk.direction",
            "elk.layered.layering.strategy",
            "elk.layered.layering.coffmanGraham.layerBound",
            "elk.layered.considerModelOrder.strategy",
            "elk.layered.crossingMinimization.forceNodeModelOrder",
            "elk.layered.unnecessaryBendpoints",
            "elk.layered.mergeHierarchyEdges",
            "elk.layered.wrapping.multiEdge.improveCuts",
            "elk.layered.wrapping.multiEdge.improveWrappedEdges",
            "elk.layered.edgeRouting.selfLoopDistribution",
        ] {
            result.as_object_mut().unwrap().remove(key);
        }
        result["elk.padding"] = json!("[top=24,left=24,bottom=24,right=24]");
        result["spacing.nodeNode"] = json!(50);
        result["elk.spacing.edgeEdge"] = json!(20);
        result["elk.layered.spacing.edgeNodeBetweenLayers"] = json!(30);
        result["nodeLabels.placement"] = json!("[H_CENTER V_TOP, INSIDE]");
        result["nodeSize.constraints"] = json!("[MINIMUM_SIZE, NODE_LABELS]");
        result["elk.layered.nodePlacement.networkSimplex.nodeFlexibility"] = json!("PORT_POSITION");
    }
    result
}

/// Mermaid orients cycles caused by collapsing a compound node before ELK,
/// then restores their semantic direction after routing.
fn feedback_edges(graph: &Graph, parents: &HashMap<String, String>) -> Vec<bool> {
    fn ancestry(id: &str, parents: &HashMap<String, String>) -> Vec<String> {
        let mut chain = vec![id.to_string()];
        while let Some(parent) = parents.get(chain.last().unwrap()) {
            chain.push(parent.clone());
        }
        chain.push("root".into());
        chain
    }
    fn child(id: &str, ancestor: &str, parents: &HashMap<String, String>) -> String {
        let mut id = id.to_string();
        while parents.get(&id).map(String::as_str).unwrap_or("root") != ancestor {
            let Some(parent) = parents.get(&id) else {
                break;
            };
            id = parent.clone();
        }
        id
    }
    let mut flags = vec![false; graph.edges.len()];
    let mut levels: HashMap<String, Vec<(usize, String, String, bool)>> = HashMap::new();
    for (index, e) in graph.edges.iter().enumerate() {
        if e.from == e.to {
            continue;
        }
        let target = ancestry(&e.to, parents);
        let source = ancestry(&e.from, parents);
        let ancestor = source.iter().find(|id| target.contains(id)).unwrap();
        if ancestor == &e.from || ancestor == &e.to {
            continue;
        }
        let from = child(&e.from, ancestor, parents);
        let to = child(&e.to, ancestor, parents);
        let collapsed = from != e.from || to != e.to;
        levels
            .entry(ancestor.clone())
            .or_default()
            .push((index, from, to, collapsed));
    }
    for level in levels.values() {
        let mut nodes = Vec::new();
        let mut degree = HashMap::<String, usize>::new();
        let mut out = HashMap::<String, Vec<usize>>::new();
        for (i, (_, from, to, _)) in level.iter().enumerate() {
            for node in [from, to] {
                if !degree.contains_key(node) {
                    degree.insert(node.clone(), 0);
                    nodes.push(node.clone());
                }
            }
            *degree.get_mut(to).unwrap() += 1;
            out.entry(from.clone()).or_default().push(i);
        }
        let roots: Vec<_> = nodes
            .iter()
            .filter(|id| degree[*id] == 0)
            .chain(nodes.iter())
            .collect();
        let mut state = HashMap::<String, u8>::new();
        for root in roots {
            if state.contains_key(root) {
                continue;
            }
            state.insert(root.clone(), 1);
            let mut stack = vec![(root.clone(), 0)];
            while let Some((node, next)) = stack.last_mut() {
                let successors = out.get(node).map(Vec::as_slice).unwrap_or(&[]);
                if *next >= successors.len() {
                    state.insert(node.clone(), 2);
                    stack.pop();
                    continue;
                }
                let (index, _, to, collapsed) = &level[successors[*next]];
                *next += 1;
                match state.get(to) {
                    Some(1) => flags[*index] = *collapsed,
                    None => {
                        state.insert(to.clone(), 1);
                        stack.push((to.clone(), 0));
                    }
                    _ => {}
                }
            }
        }
    }
    flags
}

fn group_title_width(graph: &Graph, label_width: f32) -> f32 {
    label_width
        + if graph.kind == crate::ir::DiagramKind::Flowchart {
            8.0
        } else {
            30.0
        }
}

/// Pull painted frames in around their contents, as Mermaid's
/// `evenGroupFrames` does. Node positions and ELK section origins stay put.
fn tighten_group_frames(output: &Value, graph: &Graph, layout: &mut Layout) {
    let Some(children) = output.get("children").and_then(Value::as_array) else {
        return;
    };
    for child in children {
        tighten_group_frames(child, graph, layout);
    }
    let Some(id) = output.get("id").and_then(Value::as_str) else {
        return;
    };
    let Some(group) = layout
        .subgraphs
        .iter()
        .find(|s| s.id.as_deref() == Some(id))
    else {
        return;
    };
    let original = (group.x, group.y, group.width, group.height);
    let title_floor = group_title_width(graph, group.label_block.width);
    let mut bounds = (
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );
    let mut has_box = false;
    for child in children {
        let Some(id) = child.get("id").and_then(Value::as_str) else {
            continue;
        };
        let rect = layout
            .subgraphs
            .iter()
            .find(|s| s.id.as_deref() == Some(id))
            .map(|s| (s.x, s.y, s.width, s.height))
            .or_else(|| layout.nodes.get(id).map(|n| (n.x, n.y, n.width, n.height)));
        if let Some((x, y, w, h)) = rect.filter(|r| r.2 > 0.0 && r.3 > 0.0) {
            bounds.0 = bounds.0.min(x);
            bounds.1 = bounds.1.min(y);
            bounds.2 = bounds.2.max(x + w);
            bounds.3 = bounds.3.max(y + h);
            has_box = true;
        }
    }
    if !has_box {
        return;
    }
    fn descendants<'a>(v: &'a Value, ids: &mut HashSet<&'a str>) {
        if let Some(children) = v.get("children").and_then(Value::as_array) {
            for child in children {
                if let Some(id) = child.get("id").and_then(Value::as_str) {
                    ids.insert(id);
                }
                descendants(child, ids);
            }
        }
    }
    let mut members = HashSet::new();
    descendants(output, &mut members);
    let mut include = |p: (f32, f32)| {
        bounds.0 = bounds.0.min(p.0);
        bounds.1 = bounds.1.min(p.1);
        bounds.2 = bounds.2.max(p.0);
        bounds.3 = bounds.3.max(p.1);
    };
    for edge in &layout.edges {
        if members.contains(edge.from.as_str()) && members.contains(edge.to.as_str()) {
            for &point in &edge.points {
                include(point);
            }
        } else {
            if edge.from == id
                && let Some(&p) = edge.points.first()
            {
                include(p);
            }
            if edge.to == id
                && let Some(&p) = edge.points.last()
            {
                include(p);
            }
        }
    }
    let mut x = original.0.max(bounds.0 - 24.0);
    let right = (original.0 + original.2).min(bounds.2 + 24.0);
    let bottom = (original.1 + original.3).min(bounds.3 + 24.0);
    let mut width = right - x;
    if width < title_floor {
        x -= (title_floor - width) / 2.0;
        width = title_floor.min(original.2);
        x = x.clamp(original.0, original.0 + original.2 - width);
    }
    let height = bottom - original.1;
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let group = layout
        .subgraphs
        .iter_mut()
        .find(|s| s.id.as_deref() == Some(id))
        .unwrap();
    group.x = x;
    group.width = width;
    group.height = height;
    if let Some(anchor) = layout.nodes.get_mut(id) {
        anchor.x = x;
        anchor.width = width;
        anchor.height = height;
    }
}

type Point = (f32, f32);
type MovedRun = ((Point, Point), (Point, Point));

fn segment_axis(a: Point, b: Point) -> Option<bool> {
    let dx = (b.0 - a.0).abs();
    let dy = (b.1 - a.1).abs();
    if dx > 0.01 && dy <= 0.01 {
        Some(true)
    } else if dy > 0.01 && dx <= 0.01 {
        Some(false)
    } else {
        None
    }
}

/// Mermaid straightens short port-to-channel steps without moving either
/// port. The complete channel run moves so no diagonal segment is introduced.
fn straighten_front(points: &[Point]) -> Option<(Vec<Point>, MovedRun)> {
    if points.len() < 5 {
        return None;
    }
    let [p0, p1, p2, p3] = [points[0], points[1], points[2], points[3]];
    let horizontal = segment_axis(p0, p1)?;
    if segment_axis(p2, p3) != Some(horizontal)
        || segment_axis(p1, p2) != Some(!horizontal)
        || (p1.0 - p0.0).hypot(p1.1 - p0.1) > 30.0
    {
        return None;
    }
    let jog = if horizontal {
        (p2.1 - p1.1).abs()
    } else {
        (p2.0 - p1.0).abs()
    };
    let forward = if horizontal {
        (p1.0 - p0.0).signum() == (p3.0 - p2.0).signum()
    } else {
        (p1.1 - p0.1).signum() == (p3.1 - p2.1).signum()
    };
    if !(0.01..=16.0).contains(&jog) || !forward {
        return None;
    }
    let mut last = 3;
    while last + 1 < points.len()
        && segment_axis(points[last], points[last + 1]) == Some(horizontal)
    {
        last += 1;
    }
    if last == points.len() - 1 {
        return None;
    }
    let mut moved = points.to_vec();
    for point in &mut moved[2..=last] {
        if horizontal {
            point.1 = p0.1;
        } else {
            point.0 = p0.0;
        }
    }
    let run = ((points[2], points[last]), (moved[2], moved[last]));
    moved.drain(1..3);
    Some((moved, run))
}

fn crossing_count(a: &[Point], b: &[Point]) -> usize {
    fn side(o: Point, p: Point, q: Point) -> f32 {
        (p.0 - o.0) * (q.1 - o.1) - (p.1 - o.1) * (q.0 - o.0)
    }
    fn opposite(a: f32, b: f32) -> bool {
        (a > 0.0 && b < 0.0) || (a < 0.0 && b > 0.0)
    }
    a.windows(2)
        .map(|a| {
            b.windows(2)
                .filter(|b| {
                    opposite(side(b[0], b[1], a[0]), side(b[0], b[1], a[1]))
                        && opposite(side(a[0], a[1], b[0]), side(a[0], a[1], b[1]))
                })
                .count()
        })
        .sum()
}

fn straighten_edge_terminals(layout: &mut Layout) {
    for index in 0..layout.edges.len() {
        let original = &layout.edges[index].points;
        let mut points = original.clone();
        let mut runs = Vec::new();
        if let Some((moved, run)) = straighten_front(&points) {
            points = moved;
            runs.push(run);
        }
        points.reverse();
        if let Some((moved, run)) = straighten_front(&points) {
            points = moved;
            runs.push(run);
        }
        points.reverse();
        if runs.is_empty() {
            continue;
        }
        let (before, after) = layout
            .edges
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .fold((0, 0), |(before, after), (_, edge)| {
                (
                    before + crossing_count(original, &edge.points),
                    after + crossing_count(&points, &edge.points),
                )
            });
        if after > before {
            continue;
        }
        let edge = &mut layout.edges[index];
        edge.points = points;
        if let Some(label) = edge.label_anchor {
            let projected = runs
                .iter()
                .map(|&((a, b), (c, d))| {
                    let ab = (b.0 - a.0, b.1 - a.1);
                    let len = ab.0 * ab.0 + ab.1 * ab.1;
                    let t = if len == 0.0 {
                        0.0
                    } else {
                        (((label.0 - a.0) * ab.0 + (label.1 - a.1) * ab.1) / len).clamp(0.0, 1.0)
                    };
                    let old = (a.0 + t * ab.0, a.1 + t * ab.1);
                    (
                        (old.0 - label.0).hypot(old.1 - label.1),
                        (c.0 + t * (d.0 - c.0), c.1 + t * (d.1 - c.1)),
                    )
                })
                .min_by(|a, b| a.0.total_cmp(&b.0));
            edge.label_anchor = projected.map(|p| p.1);
        }
    }
}

pub(super) fn apply(graph: &Graph, layout: &mut Layout) -> Result<(), String> {
    let mut parent = HashMap::<String, String>::new();
    for sub in &graph.subgraphs {
        if let Some(id) = &sub.id {
            if let Some(ancestor) = graph
                .element_metadata
                .get(id)
                .and_then(|meta| meta.get("_parent"))
                .and_then(Value::as_str)
            {
                parent.insert(id.clone(), ancestor.to_string());
            }
        }
    }
    for (id, node) in &layout.nodes {
        if node.anchor_subgraph.is_some() {
            continue;
        }
        if let Some(sub) = graph
            .subgraphs
            .iter()
            .filter(|sub| sub.nodes.contains(id))
            .min_by_key(|sub| sub.nodes.len())
        {
            if let Some(sub_id) = &sub.id {
                parent.insert(id.clone(), sub_id.clone());
            }
        }
    }
    let mut ordered: Vec<_> = layout.nodes.keys().cloned().collect();
    for sub in &graph.subgraphs {
        if let Some(id) = &sub.id {
            if !ordered.contains(id) {
                ordered.push(id.clone());
            }
        }
    }
    ordered.sort_by_key(|id| {
        if let Some(index) = graph
            .element_metadata
            .get(id)
            .and_then(|m| m.get("_containerIndex"))
            .and_then(Value::as_u64)
        {
            (0, usize::MAX - index as usize)
        } else {
            (
                if id.starts_with("__class_interface_") {
                    2
                } else {
                    1
                },
                graph.node_order.get(id).copied().unwrap_or(usize::MAX),
            )
        }
    });
    fn children(
        graph: &Graph,
        layout: &Layout,
        ids: &[String],
        parents: &HashMap<String, String>,
        current: Option<&str>,
    ) -> Vec<Value> {
        ids.iter().filter(|id| parents.get(*id).map(String::as_str) == current).filter_map(|id| {
            if let Some(sub) = layout.subgraphs.iter().find(|sub| sub.id.as_ref() == Some(id)) {
                let dir = graph.subgraphs.iter().find(|s| s.id.as_ref() == Some(id))
                    .and_then(|s| s.direction).unwrap_or(graph.direction);
                let mut options = layout_options(graph, dir, true);
                if graph.subgraphs.iter().any(|s| s.id.as_ref() == Some(id) && s.direction.is_some()) {
                    options["elk.direction"] = json!(direction(dir));
                    options["elk.algorithm"] = json!("org.eclipse.elk.layered");
                }
                options["nodeSize.minimum"] = json!(format!("({},0)", group_title_width(graph, sub.label_block.width)));
                let members:std::collections::HashSet<_>=ids.iter().filter(|candidate| {
                    let mut ancestor=parents.get(*candidate);
                    while let Some(p)=ancestor {if p==id {return true;}ancestor=parents.get(p);}
                    false
                }).collect();
                let external=graph.edges.iter().any(|e|members.contains(&e.from)!=members.contains(&e.to));
                if !external && graph.subgraphs.iter().any(|s| s.id.as_ref() == Some(id) && s.direction.is_some()) {
                    options["elk.hierarchyHandling"] = json!("SEPARATE_CHILDREN");
                }
                if !external && let Some(algorithm)=graph.element_metadata.get(id).and_then(|m|m.get("algorithm")).and_then(Value::as_str) {
                    options["elk.algorithm"]=json!(algorithm);
                    options["elk.hierarchyHandling"]=json!("SEPARATE_CHILDREN");
                    let padding=if algorithm=="elk.rectpacking" {10.0}else{15.0};
                    options["elk.padding"]=json!(format!("[top={},left={padding},bottom={padding},right={padding}]",sub.label_block.height+padding));
                    options["nodeSize.minimum"]=json!(format!("({},{})",sub.label_block.width+16.0,sub.label_block.height+padding*2.0));
                    options["elk.aspectRatio"]=json!(if algorithm=="elk.rectpacking" {"1.6"}else{"2.0"});
                    options["elk.expandNodes"]=json!(true);
                    options["elk.contentAlignment"]=json!("H_CENTER V_TOP");
                    if algorithm=="elk.rectpacking" {
                        options["spacing.baseValue"]=json!(15);options["spacing.nodeNode"]=json!(15);
                        options["elk.rectpacking.trybox"]=json!(true);
                        options["elk.rectpacking.packing.compaction.rowHeightReevaluation"]=json!(true);
                        options["elk.rectpacking.packing.compaction.iterations"]=json!(10);
                        options["elk.rectpacking.whiteSpaceElimination.strategy"]=json!("EQUAL_BETWEEN_STRUCTURES");
                        options["elk.rectpacking.widthApproximation.strategy"]=json!("SCANLINE");
                    }
                }
                return Some(json!({ "id": id, "labels": [{ "text": sub.label, "width": sub.label_block.width, "height": (sub.label_block.height-2.0).max(0.0) }],
                    "layoutOptions": options, "children": children(graph, layout, ids, parents, Some(id)) }));
            }
            layout.nodes.get(id).filter(|node| !node.hidden && node.anchor_subgraph.is_none()).map(|node|
                json!({"id": id, "width": node.width, "height": node.height}))
        }).collect()
    }
    let reversed = if graph
        .agentflow_config
        .as_ref()
        .or(Some(&graph.appearance_config))
        .and_then(|v| v.get("elk"))
        .and_then(|v| v.get("orientFeedbackEdges"))
        .and_then(Value::as_bool)
        != Some(false)
    {
        feedback_edges(graph, &parent)
    } else {
        vec![false; graph.edges.len()]
    };
    let edges: Vec<_> = graph.edges.iter().enumerate().map(|(index, edge)| {
        let (source,target)=if reversed[index] {(&edge.to,&edge.from)}else{(&edge.from,&edge.to)};
        let mut value = json!({"id": format!("edge-{index}"), "sources": [source], "targets": [target]});
        if let Some(label) = &layout.edges[index].label {
            value["labels"] = json!([{"text": edge.label, "width": label.width, "height": label.height,
                "layoutOptions": {"edgeLabels.placement": "CENTER", "edgeLabels.inline": true}}]);
        }
        else if graph.kind == DiagramKind::Class {
            // Preserve Mermaid's relation input, which includes a label entry
            // even when the relationship has no visible text.
            value["labels"] = json!([{"text": "", "width": 0, "height": 0,
                "layoutOptions": {"edgeLabels.placement": "CENTER", "edgeLabels.inline": true}}]);
        }
        value
    }).collect();
    let mut input = json!({"id":"root", "layoutOptions": layout_options(graph, graph.direction, false),
        "children": children(graph, layout, &ordered, &parent, None), "edges": edges});
    ports::preserve_hierarchy_port_order(&mut input, &parent);
    let output = elk_native::org::eclipse::elk::graph::json::layout_api::layout_json(
        &input.to_string(),
        "{}",
    )?;
    let output: Value = serde_json::from_str(&output).map_err(|err| err.to_string())?;
    let output = compound::recover_container_ports(&input, &output).unwrap_or(output);
    let pad = graph
        .agentflow_config
        .as_ref()
        .map(crate::agentflow::options)
        .and_then(|v| v.get("diagramPadding"))
        .and_then(Value::as_f64)
        .unwrap_or(8.0) as f32;
    let mut positions = HashMap::new();
    positions.insert("root".to_string(), (0.0, 0.0));
    fn collect(
        v: &Value,
        offset: (f32, f32),
        positions: &mut HashMap<String, (f32, f32)>,
        layout: &mut Layout,
    ) {
        if let Some(nodes) = v.get("children").and_then(Value::as_array) {
            for node in nodes {
                let Some(id) = node.get("id").and_then(Value::as_str) else {
                    continue;
                };
                let point = (offset.0 + number(node, "x"), offset.1 + number(node, "y"));
                positions.insert(id.to_string(), point);
                if let Some(sub) = layout
                    .subgraphs
                    .iter_mut()
                    .find(|s| s.id.as_deref() == Some(id))
                {
                    sub.x = point.0;
                    sub.y = point.1;
                    sub.width = number(node, "width");
                    sub.height = number(node, "height");
                }
                if let Some(native) = layout.nodes.get_mut(id) {
                    native.x = point.0;
                    native.y = point.1;
                    native.width = number(node, "width");
                    native.height = number(node, "height");
                }
                collect(node, point, positions, layout);
            }
        }
    }
    collect(&output, (0.0, 0.0), &mut positions, layout);
    fn edge_data(
        v: &Value,
        position: (f32, f32),
        positions: &HashMap<String, (f32, f32)>,
        layout: &mut Layout,
    ) {
        if let Some(edges) = v.get("edges").and_then(Value::as_array) {
            for edge in edges {
                let Some(index) = edge
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|id| id.strip_prefix("edge-"))
                    .and_then(|id| id.parse::<usize>().ok())
                else {
                    continue;
                };
                let offset = edge
                    .get("container")
                    .and_then(Value::as_str)
                    .and_then(|id| positions.get(id))
                    .copied()
                    .unwrap_or(position);
                let native = &mut layout.edges[index];
                native.points.clear();
                if let Some(sections) = edge.get("sections").and_then(Value::as_array) {
                    for section in sections {
                        for point in section
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
                        {
                            native.points.push((
                                number(point, "x") + offset.0,
                                number(point, "y") + offset.1,
                            ));
                        }
                    }
                }
                if let Some(label) = edge
                    .get("labels")
                    .and_then(Value::as_array)
                    .and_then(|a| a.first())
                {
                    native.label_anchor = Some((
                        number(label, "x") + number(label, "width") / 2.0 + offset.0,
                        number(label, "y") + number(label, "height") / 2.0 + offset.1,
                    ));
                }
            }
        }
        if let Some(nodes) = v.get("children").and_then(Value::as_array) {
            for node in nodes {
                let offset = node
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|id| positions.get(id))
                    .copied()
                    .unwrap_or(position);
                edge_data(node, offset, positions, layout);
            }
        }
    }
    edge_data(&output, (0.0, 0.0), &positions, layout);
    for (index, edge) in layout.edges.iter_mut().enumerate() {
        if reversed[index] {
            edge.points.reverse();
        }
        if edge.points.len() < 2 {
            return Err(format!(
                "ELK returned no route for {} -> {}",
                edge.from, edge.to
            ));
        }
    }
    tighten_group_frames(&output, graph, layout);
    for edge in &mut layout.edges {
        for (id, index, neighbor) in [
            (&edge.from, 0, 1),
            (&edge.to, edge.points.len() - 1, edge.points.len() - 2),
        ] {
            if let Some(node) = layout
                .nodes
                .get(id)
                .filter(|node| node.anchor_subgraph.is_none())
            {
                let p = edge.points[index];
                let q = edge.points[neighbor];
                let horizontal = (p.0 - q.0).abs() > (p.1 - q.1).abs();
                let side = if horizontal {
                    if p.0 < q.0 {
                        EdgeSide::Right
                    } else {
                        EdgeSide::Left
                    }
                } else if p.1 < q.1 {
                    EdgeSide::Bottom
                } else {
                    EdgeSide::Top
                };
                let offset = if horizontal {
                    p.1 - node.y - node.height / 2.0
                } else {
                    p.0 - node.x - node.width / 2.0
                };
                edge.points[index] = if graph.kind == DiagramKind::Flowchart {
                    crate::flowchart_shapes::outline(node, &graph.appearance_config)
                        .map(|outline| {
                            routing::anchor_point_for_outline(node, side, offset, &outline)
                        })
                        .unwrap_or_else(|| routing::anchor_point_for_node(node, side, offset))
                } else {
                    routing::anchor_point_for_node(node, side, offset)
                };
            }
        }
    }
    if graph
        .agentflow_config
        .as_ref()
        .unwrap_or(&graph.appearance_config)
        .get("elk")
        .and_then(|v| v.get("straightenEdges"))
        .and_then(Value::as_bool)
        != Some(false)
    {
        straighten_edge_terminals(layout);
    }
    ports::reduce_container_crossings(layout, &parent);
    // Match the viewport to painted content, removing ELK's root padding.
    let mut min_x = layout
        .nodes
        .values()
        .filter(|n| !n.hidden && n.anchor_subgraph.is_none())
        .map(|n| n.x)
        .chain(layout.subgraphs.iter().map(|s| s.x))
        .fold(f32::INFINITY, f32::min);
    let mut min_y = layout
        .nodes
        .values()
        .filter(|n| !n.hidden && n.anchor_subgraph.is_none())
        .map(|n| n.y)
        .chain(layout.subgraphs.iter().map(|s| s.y))
        .fold(f32::INFINITY, f32::min);
    if graph.kind == crate::ir::DiagramKind::Flowchart {
        // ELK may place a long inline label left of every node. Mermaid uses
        // the painted SVG bounds; include routes and labels before translating.
        for edge in &layout.edges {
            for &(x, y) in &edge.points {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
            }
            if let (Some(label), Some((x, y))) = (&edge.label, edge.label_anchor) {
                min_x = min_x.min(x - label.width / 2.0);
                min_y = min_y.min(y - label.height / 2.0);
            }
        }
    }
    if !min_x.is_finite() || !min_y.is_finite() {
        return Ok(());
    }
    let shift = (pad - min_x, pad - min_y);
    for node in layout.nodes.values_mut() {
        node.x += shift.0;
        node.y += shift.1;
    }
    for sub in &mut layout.subgraphs {
        sub.x += shift.0;
        sub.y += shift.1;
    }
    for edge in &mut layout.edges {
        for p in &mut edge.points {
            p.0 += shift.0;
            p.1 += shift.1;
        }
        if let Some(p) = &mut edge.label_anchor {
            p.0 += shift.0;
            p.1 += shift.1;
        }
    }
    let (w, h) =
        super::bounds_with_edges_capped(&layout.nodes, &layout.subgraphs, &layout.edges, Some(0.0));
    let (w, h) = if graph.kind == DiagramKind::Flowchart {
        // The generic bounds helper pads labels before padding the whole
        // canvas. Mermaid measures the painted content, then adds padding once.
        let mut bounds = (0.0_f32, 0.0_f32);
        for node in layout
            .nodes
            .values()
            .filter(|n| !n.hidden && n.anchor_subgraph.is_none())
        {
            bounds.0 = bounds.0.max(node.x + node.width);
            bounds.1 = bounds.1.max(node.y + node.height);
        }
        for sub in &layout.subgraphs {
            bounds.0 = bounds.0.max(sub.x + sub.width);
            bounds.1 = bounds.1.max(sub.y + sub.height);
        }
        for edge in &layout.edges {
            for &(x, y) in &edge.points {
                bounds.0 = bounds.0.max(x);
                bounds.1 = bounds.1.max(y);
            }
            if let (Some(label), Some((x, y))) = (&edge.label, edge.label_anchor) {
                bounds.0 = bounds.0.max(x + label.width / 2.0);
                bounds.1 = bounds.1.max(y + label.height / 2.0);
            }
        }
        bounds
    } else {
        (w, h)
    };
    layout.width = w + pad;
    layout.height = h + pad;
    Ok(())
}
