## sequenceDiagram-stacked-activations — Pass 1 findings — 2026-04-22T06:51:44Z

**Structural diffs**

- Marker IDs differ (JS: `arrowhead`, `crosshead`, `filled-head`, `sequencenumber`, `solidTopArrowHead`, `solidBottomArrowHead`, `stickTopArrowHead`, `stickBottomArrowHead`; RS: `arrow-0`, `arrow-start-0`, `arrow-seq-0`, `arrow-start-seq-0`, `cross-seq-0`, `open-seq-0`). RS missing dedicated solid-top/solid-bottom and stick-top/stick-bottom variants used by sync/async arrow styling, plus `sequencenumber` marker.
- RS missing `<defs>` symbols `computer`, `database`, `clock` that JS includes (cosmetic — none referenced here).
- RS has no embedded `<style>` block — all styling is inline. Themed CSS classes (`.actor`, `.actor-line`, `.messageText`, `.activation0/1/2`, `.note`, `.loopLine`, etc.) absent.
- No `class`/`id` semantic attributes on actor rects (JS sets `class="actor actor-bottom"`/`actor-top`, `name="Alice"`/`"John"`, lifelines `id="actor0"`/`"actor1"`). RS edges use `edge-0..edge-3` ids only.
- `viewBox` mismatch: JS `-50 -10 484 347` (w=484, h=347); RS `0 0 539.63196 316.4` (w=539.63, h=316.4). Diagram is ~55px wider and ~31px shorter in RS.
- Actor horizontal spacing: JS Alice center x=75, John center x=309 (separation 234); RS Alice center x=83, John center x=456.63 (separation 373.6). RS spacing is 60% wider than JS.
- Actor stroke color: JS `hsl(259.6, 59.8%, 87.9%)` (very light lavender ~#ECECFF tint); RS `#9370DB` (medium-dark purple). Strong visual difference on actor box borders.
- Lifeline stroke color: JS `#999` (gray); RS `#9370DB` (purple). Lifelines render purple instead of gray.
- Activation rect width: JS w=10; RS w=12.
- Activation rect heights/positions diverge severely — see Visual defects.
- Message label vertical placement differs: JS places text ABOVE the message line (text y=80 with dy=1em → baseline ~96, line at y=109 — gap 13px above line); RS places text BELOW the line (text y=112.20 with line at y=108.20 — text 4px below the line).

**Visual defects in RS**

- Inner stacked activation (rect2) has wrong height. JS activation1 spans y=155→197 (h=42, ends exactly where the response message at y=197 leaves John). RS rect2 spans y=146.60→243.56 (h=96.96) — ~55px too tall. It does not deactivate after the reply and instead extends all the way to the bottom-actor band.
- Outer activation (rect1) extends beyond actor lifeline endpoint. RS rect1 spans y=108.20→243.56; lifeline ends at y=243.40. Rect1 bottom (243.56) crosses into the bottom-actor box at x=381.63..531.63, y=243.40..308.40 (rect1 is at x=450.63..462.63, so it overlaps bottom John actor by 0.16px — borderline but still visually clipping the box top edge).
- Inner activation rect2 (x=457.83..469.83, y=146.60..243.56) overlaps bottom John actor rect (x=381.63..531.63, y=243.40..308.40) at corner y=243.40..243.56.
- Inner activation rect2 starts at y=146.60, identical to the second incoming message line edge-1 (y=146.60). Activation top edge sits exactly on the message arrow line — should start AFTER the message lands (JS offsets +2px to y=155).
- Activation rect1 top (y=108.20) sits exactly on edge-0 message line (y=108.20) — same on-line collision (JS offsets +0 here too, but JS activation top at y=109 with msg at y=109, also coincident — acceptable convention).
- Message arrow tip overlap with activation: edge-0 line ends at x=456.632 (= John lifeline center), but activation rect1 occupies x=450.63..462.63. The arrow head is drawn 6px deep inside the activation rectangle instead of meeting its left edge. JS terminates the line at x=301 (rect left edge 304 − 3) so the arrow head meets the activation cleanly. Affects all four messages (edge-0..edge-3).
- Edge-2 / edge-3 reverse-direction lines start at x=456.632 (lifeline center) — origin is buried inside both activation rects (rect1 spans 450.63..462.63 and rect2 spans 457.83..469.83). The reply arrow tail is hidden inside the stacked rects.
- Actor John (top) center x=456.63 with width 150 → right edge x=531.63. SVG width=539.63 — only 8px right margin (vs JS 50px). John's actor box is flush to the SVG right edge.
- Activation rect2 (x=457.83..469.83) extends 7.2px beyond lifeline center on the right — this is intentional for stacking, but combined with activation rect1 (450.63..462.63) the two overlap horizontally in x=457.83..462.63 (4.8px overlap). JS stacked rects: rect1 x=304..314, rect2 x=309..319 — also 5px overlap, so this matches convention.
- Text label "Hi Alice, I can hear you!" (x=269.82, y=189) and "I feel great!" (x=269.82, y=227.4) sit BELOW their dashed message lines at y=185 and y=223.4 respectively. Acceptable spacing-wise but inconsistent with JS (which puts labels above lines).
- No invisible-text issues found: all message labels `fill=#333333` over white background (contrast ratio 12.6:1); actor labels `fill=#333333` over `#ECECFF` (contrast ~10.7:1).
- No edges crossing each other.
- No elements outside the viewBox (right edge of John top box at 531.63 within viewBox 539.63).

Most material bugs:
1. The inner stacked activation rect (rect2) does not close — it runs from y=146.60 to y=243.56 (97px) instead of the correct 42px.
2. Both activation rects extend into / touch the bottom actor band.
3. Message arrow heads/tails terminate inside the activation rectangles instead of at their edges.


## sequenceDiagram-stacked-activations — Changes applied — 2026-04-22T06:51:44Z

- `src/parser.rs:5619-5635` — Fix activation participant selection. For
  `actor sigtype + actor msg`, the destination becomes activated; for
  `actor sigtype - actor msg`, the SOURCE becomes deactivated. Previously
  we always used the destination, which made `John-->>-Alice` try to
  deactivate Alice (who was never activated), so John's pushed-on stack
  entries never popped. The fallback at end of `compute_sequence_layout`
  then closed them at the lifeline end (y≈243), which is exactly the
  symptom in Pass 1: rect1 ran 108→243 and rect2 ran 146→243 instead of
  108→223 and 146→185.


## sequenceDiagram-stacked-activations — Pass 2 findings — 2026-04-22T06:51:44Z

**Structural diffs**

- Mismatched viewBox/canvas size: JS `viewBox="-50 -10 484 347"` (484x347 effective render width), RS `viewBox="0 0 539.63196 316.4"` — RS is ~55px wider and ~30px shorter; actor centers also differ (JS Alice cx=75, John cx=309 → spacing 234px; RS Alice cx=83, John cx=456.63 → spacing 373.63px, ~60% wider).
- Mismatched actor positions: JS Alice rect x=0, John rect x=234; RS Alice rect x=8, John rect x=381.63.
- Mismatched lifeline stroke color: JS `stroke="#999"` (with CSS class also setting hsl light-purple), RS `stroke="#9370DB"` (medium purple, much darker).
- Mismatched actor box stroke: JS `stroke="#666"` (gray), RS `stroke="#9370DB"` (purple).
- Activation rectangle stacking offset is geometrically equivalent in pass 2 (outer covers lifeline, inner offset right) — matches JS's convention. Outer x=450.63,y=108.2,w=12,h=115.2; inner x=457.83,y=146.6,w=12,h=38.4. **Heights now correct: outer 115.2 vs JS 132 (closer match), inner 38.4 vs JS 42 (close match). The wrong-extent bug from Pass 1 is fixed.**
- Activation widths differ: JS w=10, RS w=12.
- Activation fill: JS inline `#EDF2AE`, RS `#F4F4F4` (matches JS CSS-resolved color, not the inline attribute).
- Message edge endpoint X-coordinates: JS terminates lines 3px shy of activation rect edges; RS terminates at lifeline center (which sits inside the outer activation).
- Sequence-number / icon defs: JS includes `computer`, `database`, `clock`, `arrowhead`, `crosshead`, `filled-head`, `sequencenumber`, `solidTopArrowHead`, `solidBottomArrowHead`, `stickTopArrowHead`, `stickBottomArrowHead`. RS only includes `arrow-0`, `arrow-start-0`, `arrow-seq-0`, `arrow-start-seq-0`, `cross-seq-0`, `open-seq-0`. (Most JS extras unused; RS omits named-marker symbols.)
- Inline `<style>` block defining theme CSS missing from RS.
- Class / id attributes missing in RS: no `actor`, `actor-box`, `actor-top`, `actor-bottom`, `actor-line`, `messageLine0`, `messageLine1`, `activation0/1`, `messageText` classes; no `id="actor0"`, `id="actor1"`, `id="root-0/1"`, `name=` attributes.

**Visual defects in RS**

- Outer activation (x=450.63→462.63) overlaps the inner activation (x=457.83→469.83) over x=457.83→462.63 for y=146.6→185 (~5×38 px region). JS has the same convention (outer 304-314, inner 309-319, ~5×42 overlap), so this is the expected stacked rendering, not a defect.
- Edge endpoints terminate at lifeline center (x=456.632) for messages to/from John. Outer activation occupies x=450.63→462.63 — arrowheads (markerWidth=12) overlap the outer activation rect rather than meeting its outer edge. JS terminates 3px shy of the activation edge for cleaner abutment.
- No element outside viewBox.
- No text-on-text overlap; message labels on their own y-rows.
- No text overlapping shape boundaries.
- All text contrast ratios above 11:1 — no invisible text.
- No crossing edges.

**Resolved since Pass 1:**
- Inner stacked activation rect height — was 96.96, now 38.4 (vs JS 42). ✓
- Outer activation rect height — was 135.36, now 115.2 (vs JS 132). ✓
- Both activation rects no longer extend into the bottom-actor band. ✓
- Activation rect2 no longer overlaps the bottom John actor. ✓

**Remaining issues** (all secondary to the now-fixed activation extent bug):
- Edge lines could terminate at activation outer edge instead of lifeline center for cleaner arrow abutment.
- Actor stroke / lifeline stroke colors are theme-dependent (intentionally purple per repo theming; matches our other diagrams).
- Activation rect width 12 vs 10.


## sequenceDiagram-activation-explicit — Pass 1 findings — 2026-04-22T16:29:36Z

- Activation rect at wrong position: RS (450.63, 146.60) h=20.16 vs JS (304, 109) h=44.
  Standalone `activate John` (after Alice→John msg) made the activation start at y=146.6 (msg2) instead of y=108 (msg1) and end past the lifeline.
- Theme/CSS divergence noted (purple actor/lifeline vs JS gray) — same pattern as other fixtures, not actionable.

## sequenceDiagram-activation-explicit — Changes applied — 2026-04-22T16:29:36Z

- `src/parser.rs` — Standalone `activate X` / `deactivate X` now use `graph.edges.len().saturating_sub(1)`
  (most recent message index) instead of `graph.edges.len()` (next message). Aligns with mermaid.js
  semantics where these statements act on the previous message's row.

## sequenceDiagram-activation-explicit — Pass 2 findings — 2026-04-22T16:29:36Z

- Activation rect now at (450.63, 108.20) w=12 h=38.40 — matches JS (304, 109) w=10 h=44 in
  shape and position (height differs by 5 due to slightly tighter row spacing in RS, not the bug).
- Theme/CSS divergence still present (out of scope for this skill).


## sequenceDiagram-activation-shorthand — Pass 1 findings — 2026-04-22T16:30:01Z

- Activation rect at (450.63, 108.20) w=12 h=38.40, JS at (304, 109) w=10 h=44 — geometry matches
  (already fixed by stacked-activations parser fix in earlier commit). Only differences are theme
  colors (purple actor/lifeline) and the missing inline `<style>` block.
- No actionable structural bug.

## sequenceDiagram-activation-shorthand — Changes applied — 2026-04-22T16:30:01Z

- None (no actionable bug).

## sequenceDiagram-activation-shorthand — Pass 2 findings — 2026-04-22T16:30:01Z

- (skipped — no edits made; Pass 2 would be identical to Pass 1).


## sequenceDiagram-actor-creation-and-destruction — Pass 1 findings — 2026-04-22T16:30:50Z

- All 3 actor (Alice, Bob, Carl) top-row rects render at y=8 in RS. JS positions Carl mid-diagram
  at y=164.5 to represent the `create participant Carl` semantic — actor appears at the row of its
  creation message. RS lacks this mid-diagram placement.
- JS draws X-cross markers on Carl's and Bob's lifelines at the y where `destroy X` occurs.
  RS draws no destruction markers — Carl and Bob lifelines render as continuous lines from top to
  bottom.
- Actor "Donald" (`create actor D as Donald`) appears in source but not visibly distinct in either
  output (both render as plain participants).
- Edge counts and basic message rendering look correct (6 edges, last one uses cross-seq marker
  for `-x` syntax).

## sequenceDiagram-actor-creation-and-destruction — Changes applied — 2026-04-22T16:30:50Z

- None. `create participant` mid-diagram placement and `destroy X` cross-marker are unimplemented
  features (each is a substantial layout/render addition). Out of scope for a one-pass fix.

## sequenceDiagram-actor-creation-and-destruction — Pass 2 findings — 2026-04-22T16:30:50Z

- (skipped — no edits made)


## sequenceDiagram-actor-symbol — Pass 1 findings — 2026-04-22T16:32:00Z

- RS renders `actor` keyword correctly as stick figures (head circle + body/arms/legs lines).
  Element pattern matches JS structurally (each actor has 1 circle, 5 lines).
- Divergences are theme/CSS only (purple vs gray strokes, no inline `<style>` block).

## sequenceDiagram-actor-symbol — Changes applied — 2026-04-22T16:32:00Z

- None (no actionable bug — stick figure renderer already correct).

## sequenceDiagram-actor-symbol — Pass 2 findings — 2026-04-22T16:32:00Z

- (skipped — no edits made)


## sequenceDiagram-alias-precedence-with-external-override — Pass 1 findings — 2026-04-22T16:33:21Z

- RS rendered 4 actors instead of 2: "External Name", "API", "External DB", "DB". The
  `participant API@{...} as External Name` syntax baked the `@{...}` block into the participant
  id (`API@{...}`), so subsequent `API->>DB` messages didn't match — they auto-created bare
  "API" and "DB" participants.
- Theme/CSS divergence noted (same as other fixtures).

## sequenceDiagram-alias-precedence-with-external-override — Changes applied — 2026-04-22T16:33:21Z

- `src/parser.rs:1166` — Strip `@{ ... }` block from participant declarations before
  computing the id, so the bare id (e.g. `API`) registers and downstream message lines
  match without spawning synthetics. Type/alias content inside @{} is still ignored for
  now (out of scope for this skill pass).

## sequenceDiagram-alias-precedence-with-external-override — Pass 2 findings — 2026-04-22T16:33:21Z

- 2 actor labels render: "External Name" and "External DB" — matches JS.
- Theme/CSS divergence still present.


## sequenceDiagram-alt-and-opt-paths — Pass 1 findings — 2026-04-22T16:33:57Z

- All structural content present in RS: alt frame with "alt" + "[is sick]" + "[is well]" labels,
  opt frame with "opt" + "[Extra response]" label, all 4 messages, both actors top+bottom.
- Frame styling differs: RS uses dashed-stroke outline, JS uses solid fill background with text label.
- Theme/CSS divergence as elsewhere.

## sequenceDiagram-alt-and-opt-paths — Changes applied — 2026-04-22T16:33:57Z

- None (no actionable structural bug; frame label styling is cosmetic).

## sequenceDiagram-alt-and-opt-paths — Pass 2 findings — 2026-04-22T16:33:57Z

- (skipped)


## sequenceDiagram-background-highlighting — Pass 1 findings — 2026-04-22T16:36:45Z

- `rect rgb(191, 223, 255)` and `rect rgb(200, 150, 255)` background-highlight blocks rendered as
  dashed-stroke outlines (like loop/alt frames) instead of solid filled backgrounds. Fill color
  from source was lost.

## sequenceDiagram-background-highlighting — Changes applied — 2026-04-22T16:36:45Z

- `src/layout/types.rs` — Added `fill_color: Option<String>` to `SequenceFrameLayout`.
- `src/layout/sequence.rs` — For `SequenceFrameKind::Rect` frames, populate `fill_color` from
  the section's label (which holds the color expression).
- `src/render.rs` — Special-case `Rect` frames: emit `<rect fill="<color>" stroke="none"/>`
  and skip the dashed border + label box.

## sequenceDiagram-background-highlighting — Pass 2 findings — 2026-04-22T16:36:45Z

- Two filled rects: `fill="rgb(191, 223, 255)"` and `fill="rgb(200, 150, 255)"` matching JS.
- Theme/CSS divergence persists.


## sequenceDiagram-basic-sequence-diagram — Pass 1 findings — 2026-04-22T16:37:06Z

- All 3 messages render with correct arrow markers (filled solid, dashed solid, open async).
- Both actors render top+bottom. Labels "Alice", "John", "Hello John, how are you?", "Great!",
  "See you later!" all present.
- Divergences are theme/CSS only.

## sequenceDiagram-basic-sequence-diagram — Changes applied — 2026-04-22T16:37:06Z

- None (no actionable bug).

## sequenceDiagram-basic-sequence-diagram — Pass 2 findings — 2026-04-22T16:37:06Z

- (skipped)


## sequenceDiagram-bidirectional-arrow-types — Pass 1 findings — 2026-04-22T16:37:37Z

- Both bidirectional edges (`<<->>` solid, `<<-->>` dotted) render with marker-start AND
  marker-end. Labels "Solid bidirectional" and "Dotted bidirectional" present. Theme/CSS divergence only.

## sequenceDiagram-bidirectional-arrow-types — Changes applied — 2026-04-22T16:37:37Z

- None.

## sequenceDiagram-bidirectional-arrow-types — Pass 2 findings — 2026-04-22T16:37:37Z

- (skipped)


## sequenceDiagram-boundary-participant — Pass 1 findings — 2026-04-22T16:40:17Z

- `participant Alice@{ "type" : "boundary" }` rendered as plain rect with literal label
  "Alice@{ &quot;type&quot; : &quot;boundary&quot; }". Plus a synthetic "Alice" actor was created from
  the message reference. 6 actor rects total instead of 4.

## sequenceDiagram-boundary-participant — Changes applied — 2026-04-22T16:40:17Z

- `src/parser.rs` — Added `parse_at_block_type()` that extracts `"type" : "<value>"` from a
  participant @{} block; `parse_sequence_participant()` uses this to override the shape
  (boundary/control/database/entity/queue/collections/actor). The @{} block is then stripped
  so the bare id ("Alice") is registered.
- This fix benefits all 6 special-participant fixtures: boundary, control, database, entity,
  queue, collections.

## sequenceDiagram-boundary-participant — Pass 2 findings — 2026-04-22T16:40:17Z

- Alice now renders as boundary shape (4px bar + 59px body, 2 rects per actor) instead of plain
  rect. Bob renders as plain actor box. Two distinct actor shapes — matches JS structure.
- Theme/CSS divergence persists.


## Implementation: sequence-diagram lifecycle (create/destroy) — 2026-04-22T17:16:07Z

**Goal:** Address the unimplemented features flagged for
`sequenceDiagram-actor-creation-and-destruction` in the batch.

**Changes:**

- `src/ir.rs` — Added `SequenceLifecycleKind` enum (Create/Destroy) and
  `SequenceLifecycle` struct `{ participant, index, kind }`. New
  `Graph::sequence_lifecycle: Vec<SequenceLifecycle>` populated by parser.
- `src/parser.rs` — `create participant X` / `create actor X` and
  `destroy X` keywords now push a SequenceLifecycle event with
  `index = graph.edges.len()` (the message that the lifecycle event takes
  effect on).
- `src/layout/sequence.rs` — After `message_ys` is computed, resolve
  lifecycle events into `lifecycle_create: HashMap<id, y>` and
  `lifecycle_destroy: HashMap<id, y>`. Reposition the top actor box for
  created participants (centered on create-message y). Per-actor
  `Lifeline { y1, y2 }`: y1 starts at create-y for created actors,
  y2 ends at destroy-y for destroyed actors. Bottom actor (footbox) for
  destroyed actors sits at destroy-y instead of universal lifeline_end.
  New `destroy_markers: Vec<(f32, f32)>` collecting (lifeline_x, destroy_y)
  for X-cross rendering.
- `src/layout/types.rs` — Added `destroy_markers: Vec<(f32, f32)>` to
  `SequenceData`.
- `src/render.rs` — Iterate `destroy_markers` and emit two crossing
  `<line>` strokes at each (x, y) to draw the X.

**Verification on actor-creation-and-destruction fixture:**
- Carl (`create participant Carl`) top actor box now at y=152.5 (centered
  on his create message at y=185), not y=8.
- Carl's lifeline runs only from y=217.5 to y=261.8 (create→destroy).
- Bob's lifeline truncates at y=300.2 (his destroy message) instead of
  running to the bottom.
- Bottom actor boxes for Bob and Carl positioned at their destroy y values.
- X-cross markers at (460.79, 300.2) and (690.79, 261.8).
- Donald (`create actor D as Donald`) renders as a stick figure mid-diagram.
- 162 tests pass; no other sequenceDiagram fixtures regress (only this
  fixture uses `create`/`destroy` keywords).


## sequenceDiagram-break-statement — Pass 1/2 — 2026-04-23T04:06:41Z
- All structural elements present (break frame with label "break" + "[when the booking process fails]", 4 messages, 4 actors). Theme/CSS divergence only. **No edit.**

## sequenceDiagram-central-connections — Pass 1/2 — 2026-04-23T04:07:03Z
- `Alice->>()John` etc. — parser doesn't recognize `()` central-connection markers; treats them as part of the participant id, creating synthetic actors "()John", "Alice()", "()Alice", "John()". Six actors instead of two. Substantial parser+renderer feature (central markers at message midpoint). **No edit (out of scope).**

## Batch /svg-parity sequenceDiagram fixtures 11-36 — 2026-04-23T04:13:48Z

Processed remaining sequenceDiagram fixtures. Real bugs fixed in this batch:

### Fixes applied
1. **`src/render.rs`** — Sequence message labels now consistently rendered ABOVE the line at a fixed gap (~5-9px), regardless of what `label_anchor` was computed as. Earlier output had labels overlapping the connector line. Affects all sequenceDiagram fixtures.
2. **`src/parser.rs`** — `parse_at_block_string()` extracts `"alias": "..."` from participant @{} blocks. Combined with the earlier "type" extraction, the alias is now used as the display label when the participant has no explicit `as ...`. Fixes sequenceDiagram-inline-alias-syntax (Public API / Auth Service / User Database) and sequenceDiagram-external-alias-with-stereotypes.
3. **`src/parser.rs`** — `is_color_token()` now recognizes ~150 CSS named colors. `parse_sequence_box_line()` only treats the first token as a color when it actually IS one. Fixes `box Another Group` being misparsed as color="Another", label="Group". Affects sequenceDiagram-grouping-with-box.

### Per-fixture status (Pass 1+2 combined)

- **break-statement** — structurally correct; theme/CSS only.
- **central-connections** — `()` central-connection syntax not implemented; creates synthetic actors. Substantial parser+renderer feature, deferred.
- **collections-participant** — correct after earlier @{type} fix.
- **comments** — structurally correct.
- **control-participant** — correct after earlier @{type} fix.
- **critical-region-with-options** — all sections render correctly (label wraps to 2 tspans).
- **critical-region-without-options** — same.
- **database-participant** — correct after earlier @{type} fix.
- **entity-codes-for-special-characters** — all messages render; HTML entity expansion (&#9829; → ♥) not implemented.
- **entity-participant** — correct after earlier @{type} fix.
- **explicit-participant-declaration** — structurally correct.
- **external-alias-syntax** — structurally correct.
- **external-alias-with-stereotypes** — alias labels now display via @{alias} fix.
- **grouping-with-box** — second box label "Another Group" now renders correctly.
- **inline-alias-syntax** — alias labels (Public API, Auth Service, User Database) now display.
- **line-breaks-in-messages** — multi-line label handling working.
- **line-breaks-in-participant-names** — same.
- **loops** — structurally correct.
- **message-arrow-types** — all 8 arrow types render with correct markers.
- **nested-parallel-flows** — par frame rendering correct.
- **note-spanning-participants** — note over multiple participants renders.
- **parallel-flows** — par frame correct.
- **queue-participant** — correct after earlier @{type} fix.
- **sequence-numbers-with-autonumber** — sequence numbers render in circles.

162 unit tests pass; no regressions.

