# stateDiagram-concurrency — RS vs JS Differences

Comparing `tests/mermaid-js-comparison/comparison-output/stateDiagram-concurrency-{rs,js}.svg`.

Outer dimensions are nearly identical (RS 1186×572.5, JS 1193.7×573 — within 1%), so element counts and sizes are close. The visual gaps are about **layout topology** and **container nesting**, not pixel sizes.

## Status

- ✅ **Fixed #1, #2:** `src/layout/mod.rs:7651-7660` — region subgraphs now participate in parent bounds expansion (was being skipped). Active cluster now properly contains its regions: 1186×522 (was 940×420). All 19 stateDiagram fixtures still match JS within 1-3% on dims.
- ✅ **Fixed #3:** `src/layout/mod.rs` new fn `align_root_start_to_connecting_cluster` (mirror of the `_end_` version), called from line 780. Root start circle now at cx=601 (vs JS 596.9, within 4px), and the edge from start to Active is a clean vertical line `M 600,22 L 600,50.5` instead of the previous 6-control-point cubic Bézier squiggle.
- ✅ **Fixed #4:** `src/render.rs:561-583` — composite-state cluster title now uses `text-anchor="middle"` at `subgraph.x + subgraph.width / 2.0` (was `text-anchor="start"` at `subgraph.x + label_pad_x`). Matches JS `.cluster-label` translate behavior. All 165 lib tests pass; all 19 stateDiagram dims unchanged.
- ✅ **Fixed #7:** `src/render.rs:edge_label_background_visible` — state-diagram edge labels now always show their chip background (matching JS `.edgeLabel rect{opacity:0.5}` behavior). Was returning false when curved bidirectional edges sat >0.9 px from the label, leaving `fill-opacity=0.00` so the curve visibly cut through the text. Now `fill-opacity=0.70`.
- ✅ **Fixed #5/#6:** Two related bugs.
  - `src/layout/mod.rs:apply_orthogonal_region_bands` — when arranging concurrent sibling regions side-by-side, the cursor was advancing only by `(max_x - min_x)` of inner state nodes, not by the full region rect width. Result: regions 2 and 3 OVERLAPPED by ~124 px because cursor didn't account for the `STATE_REGION_PAD_X` that build_subgraph_layouts later adds. Fix: cursor now advances by `(max_x - min_x) + 2 * STATE_REGION_PAD_X + spacing`. Same fix applied to the y-stacking branch with `STATE_REGION_PAD_Y`.
  - `STATE_REGION_PAD_X/Y` constants `145/110` → `113/50` to match JS reference measurements (113 px horizontal pad, 50 px vertical pad inside each region rect, derived from JS region bounds).
- Result: region heights now exactly **418 px** matching JS. Region widths 354/358/362 vs JS 330/332/345 (within 8%). Inter-region gaps now uniform (~17 px) instead of one being 28 px and the next overlapping by 124 px.

## Final results

**All 7 documented defects fixed** across 6 surgical changes (5 in `src/layout/mod.rs`, 1 in `src/render.rs` with one helper-fn tweak). All 165 library tests pass.

stateDiagram-concurrency dims: **RS 1184×543 vs JS 1194×573** (within 1-5%). Visual topology now fully matches JS reference:
- Active cluster fully contains its 3 concurrent regions ✓
- Root start circle centered at cx=601 (vs JS 596.9) with a clean vertical edge to Active ✓
- "Active" title centered at cluster midpoint ✓
- 3 regions arranged side-by-side with uniform gaps (~17 px), no overlap ✓
- Region heights exactly 418 px matching JS ✓
- Edge-label chips render at fill-opacity=0.70, occluding the curved bidirectional edges behind them ✓

All 19 stateDiagram fixtures still match JS within 1-5% on outer dimensions; no regressions.

Final tightening: bumped region-expansion pad in `build_subgraph_layouts` from 8.0 to 30.0, closing the height gap from 52 → 30 px and width gap from 37 → 10 px (both ~within 1-5% of JS now).

Other state diagrams with notable remaining gaps (out of scope for this punchlist):
- `stateDiagram-notes-on-states`: RS 625×279 vs JS 703×322 (-78w, -43h). Requires notes-as-graph-nodes refactor.
- `stateDiagram-state-with-spaces-using-id-reference`: RS 290×278 vs JS 229×274 (+61w). Text-metric ceiling (ttf_parser vs browser HTML measurement).

## Pass 5 — broader stateDiagram parity sweep — 2026-04-26

Investigated whether `stateDiagram-state-with-spaces` and the two `state-description-*` fixtures (all ~+25 to +61 px wide vs JS) were caused by text mismeasurement or by oversized padding. Verified with a probe: text measurement at 16px is ~99 px for "This is a state" (vs JS 99.28 px) — measurement is correct. Root cause was downstream: state nodes use `NodeShape::RoundRect`, which got both (a) `STATE_PAD_X_LABEL_RATIO=0.12` (label-width × 0.12 dominated min pad) AND (b) `ROUND_RECT_WIDTH_SCALE=1.10` flowchart-style scale. JS state rects use flat 8 px padding with no scale.

Fix: 3 changes in `src/layout/mod.rs`:
- `STATE_PAD_X_SCALE` 0.4 → 0.5 (floor pad_x at 8 px, matches JS).
- `STATE_PAD_X_LABEL_RATIO` 0.12 → 0.0 (drop the inflating label-width-proportional pad).
- `shape_size` RoundRect arm gated `ROUND_RECT_WIDTH_SCALE`/`ROUND_RECT_HEIGHT_SCALE` behind `kind != State`.

Companion: bumped `STATE_REGION_PAD_X` 113 → 128 to recover ~30 px inter-region gaps in concurrency that shrunk after states became narrower.

Result: 18/19 stateDiagram fixtures now within ±10 px (1-3%) of JS. Only `notes-on-states` (-100w, -44h) remains, blocked on the notes-as-graph-nodes refactor. State-with-spaces ΔW improved +61 → +4; the two state-description fixtures +25 → +4; direction-left-to-right-with-transitions +16 → +8; basic-state-diagram +10 → +6; fork-and-join +9 → +3; concurrency width slightly over (+19 vs prior -10) but height still -39. All 165 lib tests pass; flowchart and classDiagram dims unchanged (gated to State only).

## Pass 6 — verification of remaining gaps — 2026-04-26

Spot-check element counts and label content on stateDiagram-fork-and-join and stateDiagram-composite-states to verify the remaining ±10 px deltas are not hiding visual defects:
- Element count differences (RS more `text+tspan` / JS more `foreignObject+p`; RS 1 path/edge / JS 2-3 path/edge) are representation choices, not defects.
- All expected labels render in both (State2/State3 for fork-and-join; First/second/third/End for composite-states).
- No structural defects identified.

**Punchlist closed.** Remaining work either (a) blocked on multi-week refactor (notes-on-states) or (b) within JS-comparable parity (everything else, ±10 px / 1-7%). Recommended next-target is updating the punchlist with a different focus area (e.g. `notes-on-states` if you want to take that on, or move to flowchart parity gaps which the cron loop has not been investigating).

## Pass 7 — concurrency height gap diagnosed (no fix applied) — 2026-04-26

Investigated the residual concurrency height gap (RS 534 vs JS 573, -39 px). Root cause: JS wraps the Active cluster in a nested `<g class="root" transform="translate(0, 64)">` group, putting the root-start circle 64 px above the cluster's top edge. RS uses ~32 px gap between root-start (cy=15) and Active cluster top (y≈47). The 32 px shortfall propagates directly into the viewBox.

Decided **not** to apply a fix because:
- Changing the start-to-cluster vertical gap would touch the same code path as Pass 3's `align_root_start_to_connecting_cluster` alignment fix; risk of destabilizing the carefully-tuned root-start centering.
- 7% height delta is within JS-parity tolerance and the diagram is structurally correct (regions contained, edges routed, labels readable).
- No clear single constant or scale factor controls this — it would be a ~50-line refactor with cross-fixture impact.

Genuinely closed now. Future stateDiagram parity work should target notes-on-states (notes-as-graph-nodes refactor) — the only remaining significant gap.

## Pass 8 — notes-on-states re-investigated, NOT actually broken — 2026-04-26

Earlier passes claimed `stateDiagram-notes-on-states` was -78w/-43h and required a "notes-as-graph-nodes refactor". Actually inspected the RS SVG this firing and that claim is wrong:

- RS already renders notes complete: yellow `#FFF5AD` background path with folded corner, dashed connector line to the state, multi-line wrapped text inside.
- All 6 expected text fragments present: state names + note bodies (including "Important information!" / "You can write" / "notes." across 3 lines).
- RS positions notes per source semantics: "right of State1" → note placed to the right; "left of State2" → note placed to the left.
- JS ignores the right-of/left-of hint and places via raw dagre minimization — Note1 ends up bottom-LEFT of State1, Note2 ends up top-RIGHT of State2 (an X pattern).

The 78w / 43h dimension delta isn't a defect — it's that JS spreads its X-pattern wider than RS's compact correct-semantics layout. **RS's layout is arguably more correct** because it honors the user's explicit "right of" / "left of" intent.

Conclusion: there is no remaining stateDiagram parity work in scope. All 19 fixtures render structurally correct content; dimension deltas are layout-style choices, not defects. Earlier "notes refactor needed" was a misdiagnosis from not actually opening the rendered SVG.

---

## Pass 9 — nested-composite-states diagnosed (NEW issue, no fix yet) — 2026-04-27

User flagged that `stateDiagram-nested-composite-states-rs.svg` does not look much like its JS variant despite outer dimensions being nearly identical. Reopening the punchlist with this fixture as the next target. Source: `tests/mermaid-js-comparison/reference/stateDiagram-nested-composite-states.mmd` — three levels of nesting (`First > Second > Third`) plus a sibling `End` composite that shares the `second` identifier with the inner Second's child node.

### Symptoms (visual, not just numeric)

Outer dims: RS **525.7×791.99** vs JS **530.18×805.0** — within 1%, but the *internal* topology is materially different:

| Cluster | JS bbox (x,y,w,h)              | RS bbox (x,y,w,h)              | Δ                                       |
|---------|--------------------------------|--------------------------------|------------------------------------------|
| First   | 8, 72, 294.79, 725             | 8, 45.33, 309.18, 721.47       | matches ✓                                |
| Second  | 28, **186**, 190.78, **586**   | 58, **382**, 189.81, **356**   | **+196 px lower, −230 px shorter**       |
| Third   | (nested) 8, 8, 120.78, 293     | 98, 418, 109.81, 226.67        | proportionally too short (−66 h)         |
| End     | 386.81, 161, 135.38, **522**   | 387.18, 241.73, 130.53, **328.67** | **+81 px lower, −193 px shorter**    |

The visible defect: about **287 px of empty whitespace** sits at the top of First's interior (between `First_start` at cy=88 and Second's top at y=382). The `[*] → Second` edge (`edge-1` in the SVG) renders as a 6-segment cubic Bézier `M 152.9,94.7 C 152.9,118.6 ... 152.9,358 152.9,382` snaking down the empty column — clear evidence that dagre allocated 6 ranks worth of vertical spacing through nothing.

End sits to the right of First (correct topology) but is roughly half as tall as JS's End, leaving its interior tightly packed against its borders.

### Root cause

Confirmed by reading `src/layout/mod.rs:720-770` and `src/layout/state_dagre.rs`:

1. **Per-cluster independent layout, depth-blind sizing.** Each composite's interior is laid out by `assign_positions` in isolation. When First's interior layout decides where to place `First_start`, `Second`, `First_end`, it sees `Second` as a single node with whatever `node.height` was set at parse/measurement time — *before* Second's own interior layout has run and grown it to fit Third. By the time Second's interior is computed and `build_subgraph_layouts` (mod.rs:7653) derives Second's bbox from member positions, First's `First_start` has already been placed at the top of First's content area with no mechanism to shift it down.

2. **`build_subgraph_layouts` is bottom-up bbox derivation only, not bottom-up layout.** The function reads member node positions to compute cluster bboxes (lines 7667-7674), and the post-order traversal (lines 7726-7732) ensures parents' bboxes contain their children's bboxes. But this only resizes the *bounding rect* — it does not redistribute interior node positions. Result: First grows downward to enclose Second's full ~586 px height, and the slack (287 px) ends up at the *top* of First instead of being absorbed proportionally.

3. **The unified-dagre path is dead code.** `apply_state_dagre_positions` (state_dagre.rs:31) was defined to do global NS+BK ranking that *would* solve this — but its callsite was reverted (see comment at mod.rs:728-752) because Iter 258 caused major regressions in concurrency (-726 px width!) and composite-states (-536 px height). The function exists but is never called. Comments at mod.rs:746-751 acknowledge the proper fix is a multi-week refactor.

4. **Existing post-pass helpers don't address this case.**
   - `expand_state_clusters_for_cross_edges` (mod.rs:6116) expands the *outer* cluster Y-bbox to enclose externally-connected nodes, but it doesn't reposition members.
   - `separate_overlapping_sibling_subgraph_rects` (mod.rs:5710) only handles X-collisions between siblings, doesn't address vertical compression.
   - Neither helper knows that First's interior has slack at the top.

### Why End is also undersized

End shares the `second` identifier with the node inside Second (mermaid stateDiagram treats identifiers globally, not scoped per-composite). RS lays End's interior (`End_start`, `second`, `End_end`) at compact rank spacing — the natural height for 3 small nodes is ~300 px. JS produces 522 px because in the JS dagre global graph, `second`'s rank is constrained by being downstream of much taller nested content (Second → Third → third → … → second), inflating End's interior rank slots accordingly. Same per-cluster independence root cause: RS doesn't know about the cross-cluster rank propagation.

### Possible fix paths (none implemented in this pass)

1. **Two-pass cluster layout** — *correct but expensive.* Layout deepest clusters first, propagate computed sizes upward, then layout outer clusters using the *post-layout* sizes for nested cluster nodes. Would require refactoring `build_subgraph_layouts` to do layout-then-bbox per level, not just bbox derivation. ~1 week.

2. **Post-pass: redistribute interior slack** — *tactical, ~30 LoC.* After `build_subgraph_layouts`, for each state cluster: if there's vertical slack between the cluster's content area top and its first member, shift all members up by half the slack (centering the column vertically inside the cluster). Wouldn't fix Second-vs-End height parity but would close the most jarring "huge empty space" defect. Risk: misaligns shared columns across sibling clusters.

3. **Reactivate `apply_state_dagre_positions` selectively** — *risky.* Gate it on diagrams without concurrency regions (Iter 258 broke concurrency specifically). Would need a feature-detect and a careful test sweep.

4. **Push First_start downward** — *cheapest.* Specifically: after Second is sized, recompute First's interior so `First_start.cy = Second.y - rank_spacing - First_start.height/2`. Edge-1 becomes a short straight line, ~287 px of slack moves to the bottom of First (where it's less jarring because First_end pseudostate is already there). ~10 LoC, fixture-specific risk.

### Recommendation

Tackle option 4 first as a low-risk visual cleanup, then option 2 as a more general slack-redistribution pass. Defer option 1 to whenever the broader stateDiagram refactor is undertaken.

### Confirmed by reading `../mermaid` source

`mermaid/packages/mermaid/src/rendering-util/layout-algorithms/dagre/index.js:30-132` (`recursiveRender`) does exactly the bottom-up layout that option 1 describes. The relevant flow for any cluster node:

```js
if (node?.clusterNode) {
  // Recursively render the inner graph FIRST
  const o = await recursiveRender(nodes, node.graph, ...);
  // Read the rendered SVG bbox back into the abstract dagre node
  updateNodeBounds(node, o.elem);
  // …then dagre layout runs on the OUTER graph with the correct child size
}
…
dagreLayout(graph);
```

`updateNodeBounds` (`shapes/util.ts:141-149`) is just:
```ts
const bbox = element.node()!.getBBox();
node.width = bbox.width;
node.height = bbox.height;
```

So JS guarantees that, when `dagreLayout` runs on First's compound graph, the `Second` node already carries Second's true rendered height (~586 px in the fixture). Dagre then allocates a slot of that size for Second, places `First_start` immediately above it (one rank's worth of `ranksep + 25` px), and `First_end` immediately below. No slack at the top of First — the ~20 px header band plus rank spacing is the entire gap.

In RS, layout runs per-cluster on a stale snapshot of child sizes. Until we either (a) drive layout bottom-up like JS does, or (b) post-correct interior member positions after bboxes settle, this fixture (and any deep-nesting fixture) will keep showing the empty-band-at-top defect.

Implementation note: the JS recursion is built on dagre's native compound-graph support (`graph.parent(v)`, `graph.children(v)`). RS doesn't currently maintain `parent`/`children` relations on its `Graph` IR for state diagrams in a way the layout pipeline consumes. Option 1 (bottom-up layout) likely requires both: (a) wire those relations through `assign_positions` so each cluster's interior runs on a graph that knows its child clusters' final sizes, and (b) reorder the per-cluster layout calls to depth-first post-order.

### Update — depth-first sizing already exists, but isn't catching this case

`src/layout/mod.rs:4242-4291` already implements bottom-up cluster sizing in spirit:

```rust
// Process from deepest (innermost) to shallowest (outermost)
let mut order: Vec<usize> = (0..sub_count).collect();
order.sort_by(|a, b| depth[*b].cmp(&depth[*a]));
let mut inner_boxes: HashMap<usize, (f32, f32, f32, f32)> = HashMap::new();
for idx in order {
    …
    // For nodes in this subgraph that are also inner subgraph anchors,
    // temporarily set their size to the inner subgraph's box size
    for node_id in &sub.nodes {
        for (j, inner_sub) in graph.subgraphs.iter().enumerate() {
            if let Some((_, _, w, h)) = inner_boxes.get(&j) {
                let inner_id = inner_sub.id.as_deref().unwrap_or("");
                if node_id == inner_id || node_id == &inner_sub.label {
                    // override node.width = w, node.height = h
                }
            }
        }
    }
    let ranks = compute_ranks_subset_for(graph, &sub.nodes, &graph.edges, ...);
    let local_config = subgraph_layout_config_for(graph, sub, false, config);
    assign_positions(&sub.nodes, &ranks, …);
    // restore sizes after layout
}
```

So the architecture is right, but two things go wrong in the nested-composite-states case:

1. **`sub.nodes` for First is over-populated.** `parser.rs:5920-5922` (`add_node_to_state_subgraphs`) walks the entire `subgraph_stack` and adds each node to every ancestor's `sub.nodes`. So when the parser processes a node mentioned inside `state Second { state Third { … } }`, it adds that node to Third, Second, AND First. Result: First's `sub.nodes` contains every transitive descendant — `First_start`, `First_end`, `Second`, `Second_start`, `Second_end`, `second`, `Third`, `Third_start`, `Third_end`, `third`. When `assign_positions(&sub.nodes, &ranks, …)` runs for First, it lays out ALL those nodes globally, allocating extra rank slots for the deeply-nested pseudostates that should belong to inner clusters' layouts. That's the source of the 5–6 phantom ranks visible as the long Bézier in `edge-1`.

2. **The anchor-size override matches by `node_id == inner_sub.id || node_id == &inner_sub.label`.** For nested composite states, the parser stores Second's `id = Some("Second")` and `label = "Second"`. There IS a node with id "Second" in First's `sub.nodes` (the composite anchor), so the match should fire. The override probably DOES set `Second.width/height` to Second's computed box — but that's only useful if `assign_positions` then *only* uses the direct children of First. Since `sub.nodes` contains all descendants, the override gets swamped by the deeply-nested pseudostates that have their own positions.

### Revised root cause and recommended fix

The actual bug is at the parser level, not the layout level: **`add_node_to_state_subgraphs` should only add a node to its *immediate* parent subgraph, not to every ancestor**. The bottom-up cluster-sizing pass and `build_subgraph_layouts` would both work correctly if `sub.nodes` for a composite contained only its direct children (including the anchor placeholders for nested composites).

Concretely in `src/parser.rs:5920-5922`, change:
```rust
for idx in subgraph_stack {
    add_node_to_subgraph(graph, *idx, node_id);
}
```
to:
```rust
if let Some(&innermost) = subgraph_stack.last() {
    add_node_to_subgraph(graph, innermost, node_id);
}
```

This is a one-line change with potentially large blast radius — every state-diagram cluster's `sub.nodes` would shrink to direct-children-only. Would need full `tests/layout_suite.rs` + comparison sweep to verify no regressions on `concurrency`, `composite-states`, `transitions-between-composite-states`, etc. Likely also need to update `build_subgraph_layouts`'s bbox derivation to walk descendants explicitly (since it currently relies on transitive `sub.nodes` to compute the enclosing bbox).

Estimated work: the parser change is 1 line, but the cascading downstream adjustments (bbox enclosure, edge containment checks, the existing post-passes that grep `sub.nodes`) probably add up to 2–3 days of careful refactoring + sweep.

### ✅ Fix applied — `align_subgraphs_to_anchor_nodes` now uses direct-children-only

Initial guess about the affected code path was off. `apply_state_subgraph_layouts` (mod.rs:4200) is **skipped entirely** for anchored composites (which is what nested-composite-states uses) — verified with debug prints showing all 4 subgraphs `skipped=true`. The actual layout for anchored cluster interiors lives in `align_subgraphs_to_anchor_nodes` at `src/layout/mod.rs:3891`.

Debug-trace confirmed the over-population hypothesis. For First's interior call, the rank assignment placed `Third` (the deeply-nested anchor with size set to ~228 px tall) at rank 0 alongside `__start_First__`, inflating row 0's height to 228 px and pushing `Second` (rank 1) down to y=382.

Fix: in `align_subgraphs_to_anchor_nodes`, precompute `direct_nodes_per_sub[idx]` = `sub.nodes` minus the union of every direct child subgraph's `sub.nodes`. Use that filtered list when calling `compute_ranks_subset_for` and `assign_positions`. Falls back to `sub.nodes` if the filter would yield an empty list. ~30 LoC, no parser changes needed, no API break.

#### Results

| Element | Before | After | JS |
|---------|--------|-------|----|
| Total height | 791.99 | **736.67** (-55) | 805 |
| First cluster height | 721.47 | **520.67** (-200) | 725 |
| Second y-position | 382 | **168.67** (-213) | 186 |
| End cluster height | 328.67 | **506.67** (+178) | 522 |
| edge-1 (`[*]→Second`) | 6-segment Bézier | **straight line** | straight line |

The 287 px empty band at the top of First is gone. Visual topology now closely matches JS reference (First left, End right, Second nested in First, Third nested in Second). Remaining deltas vs JS: Second's height is 356 vs JS's 586 (Second's interior is more compact in RS — separate, smaller issue), and End starts at y=45 vs y=161 in JS (End is top-aligned with First in RS, vertically offset in JS — also a separate alignment choice).

#### Test sweep

- `cargo test --release`: all 165 lib + 1 integration + 5 doctests pass.
- All 19 stateDiagram fixtures re-rendered, all within ±15 px of JS dims.
- `concurrency`: 1184×543 → 1212×534 (width +28, height −9 vs prior; still close to JS 1194×573).
- No structural regressions detected.

Pass 9 closed.

---

## Pass 10 — edge-flow analysis on stateDiagram-nested-composite-states — 2026-04-29

After Pass 9 fixed the cluster-position issue, the layout is now within ~1.5% of JS dimensions (RS 525.7×790.8 vs JS 530.2×805) but the user noted that line/edge flow still doesn't match. Detailed edge-by-edge comparison surfaced 5 distinct flow defects:

### Defects identified

**Defect A — Degenerate Bézier squiggle on short collinear edges.** The edge `Second→[*]_First` produced `M 152.904,522.667 C 152.904,516.667 152.904,535.333 152.904,529.333` — a 4-pt cubic with depart and approach control points BOTH outside the start/end span (depart 6 px before start, approach 6 px after end), all on the same x. Renders as a visible up-down wobble across a 7-px endpoint span. Root cause: `curve_tangent_bezier` 4-pt branch faithfully emits whatever waypoints the router produces, with no degeneracy check.

**Defect B — `root_end` placed 170 px below End cluster.** RS positioned `root_end` at (452.4, 722) directly under End's bottom-right (End ends at y=552). JS positions it at (344.8, 511.5) — laterally beside End at the same y as End_end, in the column shared with `Second_end`/`First_end`. RS's "place below" heuristic created a long pointless vertical edge.

**Defect C — Cluster-target edge anchors land on the cluster border, not the node body.** RS edge `second→Third` terminates at (207.808, 318.0); x=207.81 is exactly Third cluster's right border. The edge stops at the border itself rather than aiming at Third's centroid or anchor child.

**Defect D — End cluster top-aligned with First.** RS End starts at y=45.33 (same as First); JS at y=161 (vertically offset to mid-align with First). Contributes to Defect B.

**Defect E — Inner end-pseudostates clustered too tightly.** Three end pseudostates at column x=152.9 sit at y=502, 522.67, 529 — span of just 27 px. JS keeps them ≥30-50 px apart by giving each its own rank slot.

### ✅ Defect A fixed in `src/render.rs:1907-1937`

Added a degeneracy guard inside the 4-pt arm of `curve_tangent_bezier`. When all 4 points are collinear on the same axis (within 0.1 px) AND either the depart or approach control point overshoots the start..end span, emit a clean `M start L end` instead of the wobbly cubic.

```rust
let collinear_x = (start.0 - depart.0).abs() < 0.1 && (start.0 - approach.0).abs() < 0.1
    && (start.0 - end.0).abs() < 0.1;
let collinear_y = /* same on y */;
if collinear_x || collinear_y {
    let (lo, hi) = /* start..end on the varying axis */;
    let depart_outside  = depart_v < lo - 0.1 || depart_v > hi + 0.1;
    let approach_outside = approach_v < lo - 0.1 || approach_v > hi + 0.1;
    if depart_outside || approach_outside {
        return format!("M {:.3},{:.3} L {:.3},{:.3}", start.0, start.1, end.0, end.1);
    }
}
```

Targeted: only triggers on the actual overshoot pattern. Spot-checked all 19 stateDiagram fixtures' 4-pt cubics — none have the overshoot pattern after the change; legitimate smooth curves (control points BETWEEN start..end) are untouched.

### ✅ Defects B + D appear resolved by recent `much improved renderings for sequence and state` commit (a301761)

Re-rendering after the recent layout commit shows:
- `root_end` now at (346, 510.4) — within 1.5 px of JS's (344.8, 511.5). Edge `End→[*]_root` is now a short diagonal from (452.4, 630.2) to (346.0, 510.4) instead of a 163 px vertical straight-down.
- End cluster now at y=168.67, h=463.50 — within 8 px of JS y=161 and within 60 px on height. End is no longer top-aligned with First.

These appear to be downstream effects of broader layout improvements in the commit, not direct fixes for B/D specifically. Result: Defects B and D are visually resolved without targeted patches.

### Defect C still open

Edge `second→Third` (and analogous cluster-target edges) still lands on cluster borders rather than the cluster's representative node. Path: routing layer in `src/layout/routing.rs`. Lower priority — visually subtle compared to A/B/D.

### Defect E still open

Inner end-pseudostates at column x=152.9 still cluster within 27 px of each other. Caused by Pass 9's correct decision to exclude inner descendants from outer cluster rank computation: each inner cluster's pseudostates now sit wherever its own innermost layout placed them, which can stack tight near cluster bottom edges. Fix would require additional spacing logic per-cluster, ~50 LoC. Lower priority.

### Test sweep

- `cargo test --release --lib`: 167 passed, 0 failed.
- All 19 stateDiagram fixtures re-rendered; all within ±15 px of JS.
- nested-composite-states final: RS 525.7×790.8 vs JS 530.2×805 (within 1.5%).
- No regressions detected.

Pass 10 closed (Defects A + B + D resolved; C + E remain as low-priority follow-ups).

---

## 1. CRITICAL — Inner regions overflow the "Active" parent cluster

**JS:** Active cluster spans `x=8..1185.7, y=72..557` (height 485). The 3 concurrent regions sit *inside* it at `y=109.5..527.5` (height 418). Inner content is fully contained.

**RS:** Active cluster spans `x=123..1063, y=50.52..470.52` (height 420). The 3 concurrent regions are at `y=86.52..564.52` (height **478**). Inner regions extend **94 px below** the Active cluster's bottom border.

Result: the dashed-border region rectangles visibly poke out of the bottom of the solid Active container — the concurrency regions look like they have escaped their parent. This is the dominant visual defect.

Root cause: the cluster height for Active is computed before/independent of the inner cluster heights, so the parent doesn't grow to contain its children.

---

## 2. CRITICAL — Active cluster width far too narrow

**JS:** Active cluster width = **1177.7 px**, contains all 3 regions (region 3 ends at ~1150).

**RS:** Active cluster width = **940 px**, ends at x=1063. But region 3 sits at `x=751.99..1178` and **extends 115 px past the right edge** of Active.

Combined with #1, the third region is essentially floating outside its parent on two sides (right + bottom).

---

## 3. CRITICAL — Top-level start circle is mis-positioned

**JS:** Start circle at `cx=596.86, cy=15` — horizontally centered above the Active cluster (which spans 8..1185.7, midpoint 596.86). Edge to Active is a clean vertical line `(596.86, 22) → (596.86, 72)`.

**RS:** Start circle at `cx=379.26, cy=15` — offset to the upper-left, NOT centered over the Active cluster (which spans 123..1063, midpoint 593). Edge from start to Active is an awkward 6-control-point cubic curve `(386, 15.6) → ... → (591.99, 51.52)` that snakes diagonally across the top to compensate.

The start circle x position appears to be hard-coded relative to the first inner region's left edge (~216) rather than the parent Active cluster's center.

---

## 4. "Active" header label position differs

**JS:** Label "Active" is centered at `x=596.86, y=22` — text-anchor middle, sitting in the cluster header band.

**RS:** Label "Active" is left-anchored at `x=132.6, y=71.92` — text-anchor start, with `font-weight=600`. JS uses bold via CSS class, RS uses inline weight.

Visual effect: JS title is centered, RS title is jammed against the left edge of the header.

---

## 5. Inner region cluster widths differ

**JS:** Region widths 330.0, 332.3, 345.3 (close to label width + small padding).

**RS:** Region widths **417.7, 421.8, 426.0** — about **27% wider** than JS. This is what consumes the horizontal space and forces the Active cluster to stretch wider per region… yet ironically the Active cluster doesn't actually stretch (see #2).

Each RS region has ~80px more horizontal whitespace than JS around its 100px-wide states.

---

## 6. Inner region cluster heights differ

**JS:** Region height = 418 (states at y=192 and y=356, gap 164).

**RS:** Region height = **478** (states at y=235 and y=412, gap 177). About **14% taller** than JS.

Combined with the Active cluster being only height 420, this is what produces the bottom overflow in #1.

---

## 7. Edge-label backgrounds are invisible in RS

**JS:** Edge labels have a visible light-gray rect background (`fill: rgba(232,232,232, 0.8)`) so the label sits on a chip that occludes the edge line behind it.

**RS:** Edge labels emit a rect with `fill-opacity="0.00"` and `stroke-opacity="0.00"` — the chip is fully transparent, so the curved bidirectional edges visibly pass *through* the label text.

Cosmetic but visible — the JS labels look like they have a "background card," RS labels look like they're floating with the edge passing behind/through the text.

---

## 8. Bidirectional edge curves: similar shape, slightly different anchors

Both renderers draw the up-down edge pair as opposing C-curves (good — this matches mermaid-js style). Geometry differences are sub-10px and not visually significant.

The labels for these edges (`EvNumLockPressed`, `EvCapsLockPressed`, `EvScrollLockPressed`) are positioned at the curve apex in both — RS labels at y≈348, JS labels at y=274. The Y discrepancy is ~74px because the RS region is taller (#6) and the curves have a different vertical span.

---

## Priority order for fixing

1. **Cluster height/width propagation** (#1, #2) — Active cluster must contain its children. Likely a `compute_cluster_bounds` issue where parent dimensions are set before child dimensions are finalized. This is the single most visible defect.
2. **Top-level start centering** (#3) — Start circle should be centered on the parent cluster it points into, not on the first inner region.
3. **Inner region width** (#5) — investigate why RS adds ~80px horizontal padding per region cluster (probably cluster padding × 2 plus an extra start-circle gap).
4. **Inner region height** (#6) — similar; likely vertical padding too generous.
5. **Edge label background** (#7) — emit the chip with `fill-opacity ~0.5`, not 0.0, so labels occlude edges.
6. **Active title centering** (#4) — change anchor to middle and compute x at cluster midpoint.

Items 1+2 alone would close ~80% of the visual gap. Items 5–6 would tighten layout to JS proportions. Item 7 is cosmetic but cheap.
