# Flowchart Fix Punchlist

Rendering differences between our Rust renderer (RS) and mermaid-js (JS) for flowchart diagrams. JS source at `../mermaid/packages/mermaid/src/rendering-util/rendering-elements/shapes/`. Analysis from 117 pairs in `tests/mermaid-js-comparison/output/`.

---

## Systematic: Node padding too large (affects ~80% of diagrams)

**RS**: `node_padding_x=30, node_padding_y=15` (`config.rs:763-764`)
**JS**: default padding=10 (`defaultConfig.ts`). Rect formula: `w = bbox.width + padding*2*2, h = bbox.height + padding*2` (`drawRect.ts`)

RS nodes are consistently 1.3–2.3× larger than JS. Reduce `node_padding_x` to ~20 and `node_padding_y` to ~10, or adopt JS formula: `w = bbox.width + padding*4, h = bbox.height + padding*2`.

---

## Circle family: oversized (7 shapes)

| Shape | RS size | JS size | JS file | JS formula |
|-------|---------|---------|---------|------------|
| `circle` | 356×356 | 230×230 | `circle.ts` | `r = bbox.width/2 + padding` |
| `dbl-circ` | 356×356 | 240×240 | `doubleCircle.ts` | same + outer ring |
| `sm-circ` | 69×69 | 30×30 | `stateStart.ts` | **fixed 14×14**, `node.label=''` |
| `f-circ` | 69×69 | 30×30 | `filledCircle.ts` | **fixed 14×14**, `radius=7` |
| `fr-circ` | 87×87 | 30×30 | `stateEnd.ts` | **fixed 14×14** |
| `circle` (start) | 87×87 | 40×40 | `circle.ts` | `r = bbox.width/2 + padding` |
| `dbl-circ` (stop) | 87×87 | 50×50 | `doubleCircle.ts` | similar |

Fix: `sm-circ`, `f-circ`, `fr-circ` should be **fixed 14×14** with no label. Larger circles need reduced padding matching `r = bbox.width/2 + 10`.

---

## Fork/join: completely wrong

**RS**: 171×171 (square). **JS**: 86×26 (thin bar). JS file: `forkJoin.ts`.
JS formula: `node.label=''`, TB direction: `w=max(70, node.width), h=max(10, node.height)`.

Fix: Render as a thin horizontal bar (70×10 for TB) or thin vertical bar (10×70 for LR). No label.

---

## Brace shape: 3.6× too wide

**RS**: 179×179. **JS**: 50×65. JS file: `curlyBraceLeft.ts`.
JS formula: `w = bbox.width + paddingX, h = bbox.height + paddingY, radius = max(5, h*0.1)`.

Fix: Rewrite brace to use JS proportions — compact around label, not inflated.

---

## Bolt shape: wrong aspect ratio

**RS**: 87×70 (landscape). **JS**: 51×86 (portrait). JS file: `lightningBolt.ts`.
JS formula: `node.label='', w=max(35,node.width), h=max(35,node.height)`, drawn 2:1 height ratio.

Fix: Render as portrait orientation, minimum 35×35, no label, with zigzag path.

---

## Sloped rectangle: wrong aspect ratio

**RS**: 87×78 (square). **JS**: 55×97 (portrait). JS file: `slopedRect.ts`.
JS formula: `totalHeight = (bbox.height + paddingY*2) * 1.5`, then shape height = totalHeight/1.5 with slope extending up.

Fix: Apply the 1.5× height multiplier from JS.

---

## Cylinder shapes: oversized, wrong element type

| Shape | RS size | JS size | JS file |
|-------|---------|---------|---------|
| `cyl` (database) | 94×75 | 40×67 | `cylinder.ts` |
| `h-cyl` | 94×75 | 48×48 | `horizontalCylinder.ts` |
| `lin-cyl` | 87×70 | 40×67 | `linedCylinder.ts` |
| `[(Database)]` | 164×75 | 96×84 | `cylinder.ts` |

JS formula: `w = bbox.width + padding, rx = w/2, ry = rx/(2.5 + w/50), h = bbox.height + padding + ry`.

Fix: Adopt JS cylinder formula. Consider using `<path>` instead of `<ellipse>` for caps.

---

## Document shapes: wrong aspect ratio

| Shape | RS size | JS size | JS file |
|-------|---------|---------|---------|
| `lin-doc` | 87×70 | 59×97 | `linedDocument.ts` |
| `docs` (stacked) | 87×70 | 65×107 | `stackedDocument.ts` (curvedTrapezoid-based) |
| `tag-doc` | 87×70 | 59×97 | `taggedDocument.ts` |

RS renders these in landscape; JS renders in portrait. The curvedTrapezoid base shape naturally creates a taller-than-wide shape because `radius = h/2` consumes half the width.

Fix: The curvedTrapezoid shape implementation needs the width formula: `w = (bbox.width + padding*2) * 1.25` (from `curvedTrapezoid.ts` line 26). The 1.25× multiplier and h/2 radius create the correct proportions.

---

## Other individual shapes

| Shape | RS | JS | Issue | Fix reference |
|-------|----|----|-------|---------------|
| `notch-rect` | 87×70 | 52×55 | 1.7× too wide | Reduce padding |
| `hourglass` | 87×70 | 46×46 | 1.9× too wide | Should be square, smaller |
| `braces` | 87×70 | 53×65 | 1.6× too wide | Reduce padding |
| `brace-r` | 87×70 | 50×65 | 1.7× too wide | Reduce padding |
| `tri` | 108×81 | 64×64 | 1.7× too wide | Should be square |
| `flip-tri` | 108×81 | 64×64 | 1.7× too wide | Should be square |
| `hex` | 101×75 | 60×55 | 1.7× too wide | JS: `m=h/4, w=bbox.width+2*m+padding` |
| `odd` | 87×70 | 50×55 | 1.7× too wide | Reduce padding |
| `flag` | 87×70 | 116×113 | **Too small** (0.7×) | Increase to match JS |
| `text` | 87×70 | 40×55 | 2.1× too wide | Minimal padding for text blocks |
| `div-rect` | 87×70 | 40×63 | 2.1× too wide | Reduce padding |
| `rounded` (event) | 94×73 | 55×70 | 1.7× too wide | Reduce padding |
| `win-pane` | 87×70 | 60×75 | 1.4× too wide | Slight padding reduction |
| `bow-rect` | 87×70 | 66×55 | 1.3× too wide | Slight padding reduction |
| `tag-rect` | 87×70 | 66×70 | 1.3× too wide | Slight padding reduction |

---

## Layout issues (4 diagrams)

### Subgraphs render vertically instead of horizontally
**Diagrams**: `basic-subgraph` (RS 129×232 vs JS 304×140), `subgraph-with-explicit-id`, `flowchart-with-multiple-subgraphs` (RS 197×923 vs JS 502×527), `direction-in-subgraphs`

RS stacks subgraphs vertically; JS arranges them based on the diagram's direction. Fix: respect `direction` declarations within subgraphs.

### Direction parsing issues
**Diagrams**: `markdown-strings` (RS portrait vs JS landscape), `special-characters-in-nodes`, `new-shapes-syntax`

These should be horizontal (LR) layouts but render as vertical (TD). Fix: check direction parsing for these specific syntaxes.

---

## Priority order

1. **Node padding reduction** — single change fixes ~80% of size differences
2. **Fixed-size shapes** (sm-circ=14×14, f-circ=14×14, fr-circ=14×14, fork=70×10) — 4 constant changes
3. **Circle padding** — match `r = bbox.width/2 + padding`
4. **Brace/bolt/sloped-rect** — wrong proportions, need rewrite
5. **Cylinder formula** — adopt JS `ry = rx/(2.5 + w/50)`
6. **Document 1.25× width multiplier** — one constant
7. **Subgraph direction** — architectural fix
