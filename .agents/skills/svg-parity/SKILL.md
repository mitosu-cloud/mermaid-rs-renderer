---
name: svg-parity
description: "Use when someone asks to run svg parity, close the JS/RS render gap, compare a Rust SVG against the golden mermaid SVG, or patch the Rust engine (mmdr) to match mermaid-cli output."
argument-hint: "<image-name> <folder>"
disable-model-invocation: true
---

## What This Skill Does

One-pass auto-fix loop for SVG render parity between `mmdr` (this repo's Rust renderer) and `mermaid-cli` (the golden JS reference).

1. Reads two pre-rendered SVGs from a user-supplied folder directly with the `Read` tool.
2. Produces a structured visual comparison inline.
3. Patches the Rust renderer to close the identified gap.
4. Rebuilds, re-renders the RS SVG, and re-compares.
5. Logs every stage to `COMPARISON_CHANGES.md` at repo root.

Exactly **one** fix pass. If discrepancies remain after pass 2, the user re-invokes the skill.

## Inputs

- `` = IMAGE-NAME (e.g., `flowchart-basic`, `block-asymmetric-shape`)
- `$2` = folder path (relative to repo root) containing the two rendered SVGs

Expected files:
- `$2/-rs.svg` — Rust renderer output
- `$2/-js.svg` — golden mermaid-cli output
- `tests/mermaid-js-comparison/reference/.mmd` — source the RS SVG will be re-rendered from

## Preconditions (abort without logging if any fail)

1. Working directory is the `mermaid-rs-renderer` repo root (verify `Cargo.toml` present with `name = "mmdr"` or similar).
2. `` and `$2` are both non-empty. If not, print `Usage: /svg-parity <image-name> <folder>` and stop.
3. All three expected files exist. Missing any → abort with a clear message naming the missing file. Do **not** write anything to `COMPARISON_CHANGES.md`.
4. Sibling `../mermaid` checkout exists (used in step 4).

## Steps

### Step 1 — Validate

Check all preconditions above. Abort immediately if any fail.

### Step 2 — Pass-1 comparison (inline)

Use the `Read` tool to read both SVGs directly:
- Golden (expected): `<absolute path to $2/-js.svg>`
- Rust (actual): `<absolute path to $2/-rs.svg>`

**Your job is to compare what these would LOOK LIKE if rendered side-by-side**, not just to count matching elements. Element-count parity does NOT imply visual parity. A diagram with the right number of rects/circles/edges can still look totally different if the layout geometry is wrong.

Report findings as bulleted markdown under the headers below. Report only discrepancies; do not list matching elements.

#### Mandatory visual-appearance checks (do these FIRST)

Before any element-counting, evaluate whether the two diagrams **would look like the same picture** to a human viewer:

1. **Aspect ratio + size class** — JS viewBox vs RS viewBox. If aspect ratios differ by more than 30% (e.g. JS is wide-landscape 1194×573, RS is small-portrait 514×298), that is a major visual defect even if element counts match. Note: "RS uses tighter spacing" or "RS more compact" is NOT an acceptable explanation if a viewer would see two visibly different layouts.

2. **Layout topology** — Are equivalent elements arranged in the same gross spatial pattern? Side-by-side columns vs vertical stack? Inner states arranged top-to-bottom vs in a grid? Composite nesting visually similar?

3. **Edge shape** — JS edges may be CURVED (cubic Béziers, S-curves bulging outward) while RS edges are STRAIGHT lines. Bidirectional pairs in JS often spread apart via opposite-direction curves, while RS may stack them on a single straight line. List for each edge: command sequence (`M..L` vs `M..C..C..L`), and whether bulge direction matters. If JS uses a C-curve and RS uses an L, flag it.

4. **Inter-element spacing ratios** — Compute ratios that should be size-independent:
   - Inter-state spacing / state height
   - Region width / state width (does each region have ROOM around its states, or are states crammed edge-to-edge?)
   - Label width / containing-region width (if labels are wider than ~80% of their region, they will visually overflow even if element counts are right)
   - Edge label spread for bidirectional pairs (do the two labels sit on opposite sides of the column, or stacked at the same x?)

5. **Label-vs-container fit** — For every label, does it fit inside its container with reasonable margin (≥10% of label width on each side)? If a label rect is wider than its containing region, the label visually overflows the border — flag it explicitly.

6. **State of the art comparison summary** — Conclude with one or two sentences: "If displayed side-by-side, would these look like the same diagram? Why or why not?" This summary must be HONEST. Do not declare visual parity if the layouts are visibly different.

#### Structural diffs (after the visual-appearance checks)

- Missing elements in RS (element type, id, visible text)
- Extra elements in RS that don't appear in JS
- Mismatched attributes (position, size, fill, stroke, font-family, font-size)

#### Visual defects in RS

- Element overlaps (list element pairs with coordinates)
- Elements outside the SVG viewBox / clipping region
- Lines or edges crossing each other
- Text overlapping other text
- Text overlapping shape boundaries
- Text overflowing its containing shape's boundary (e.g. label rect wider than the cluster rect that contains it — even if the text "background fill" hides the underlying border)
- Invisible text: text whose fill color is identical or within ΔE < 10 of its background / containing shape. Report the fill color, background color, and contrast ratio.

Be precise. Include element ids or coordinates. Keep the full report under 200 lines.

For very large SVGs (>2000 lines), use `Read` with `offset`/`limit` to scan in chunks, or use `Bash` with `grep` to extract specific element classes (e.g. `grep -oE '<rect[^/]+/>' file.svg`) so the entire raw payload doesn't need to sit in context at once.

#### Anti-patterns to avoid in your reporting

- **Don't declare "structural parity" or "element parity" from counts alone.** Element counts matching while geometry differs is the WORST kind of false positive — it invites premature claims of success. If counts match but the diagrams look different, the report must lead with "the layouts look different" not "structurally equivalent".
- **Don't excuse visual differences as "just spacing constants" or "tighter layout".** If the user would see a different picture, that IS the defect, not a stylistic preference.
- **Don't report "0 defects" if you haven't actually compared what the rendered image would look like.** Counting absences (e.g. "no text-on-edge") is necessary but not sufficient.
- **Don't claim a fix is "fully resolved" unless the post-fix RS visibly matches JS.** If RS still looks compressed/different/crowded after a fix, the fix is incomplete — say so.

### Step 3 — Append Pass-1 findings to `COMPARISON_CHANGES.md`

Create the file if it doesn't exist. Append:

```
## <> — Pass 1 findings — <ISO 8601 UTC timestamp>

<your Step 2 findings, verbatim>
```

### Step 4 — Diagnose and edit the Rust engine

- Infer diagram type from the `` prefix before the first `-` (e.g., `flowchart-basic` → `flowchart`; `architecture-icons-aws` → `architecture`).
- Read relevant Rust source under `src/` for that diagram type (use `find src -type d -iname "*<diagram-type>*"` or grep).
- Read corresponding mermaid JS source under `../mermaid/packages/mermaid/src/diagrams/<diagram-type>/` to understand the golden code path.
- Edit `src/` to close gaps identified in Pass 1. Favor targeted changes over sweeping rewrites.
- **Hard constraint:** no edits outside `src/`. Do not touch `Cargo.toml`, `tests/`, `docs/`, or anything else.

### Step 5 — Append "Changes applied" to `COMPARISON_CHANGES.md`

```
## <> — Changes applied — <ISO 8601 UTC timestamp>

- `<file:line>` — <what changed and why>
- ...
```

### Step 6 — Rebuild

Run `cargo build --release`. If it fails, append:

```
## <> — Build failed — <ISO 8601 UTC timestamp>

<first ~50 lines of cargo stderr>
```

Then abort. Do not proceed to re-render or Pass 2.

### Step 7 — Re-render the RS SVG

Run (from repo root):

```
./target/release/mmdr -i tests/mermaid-js-comparison/reference/.mmd -o $2/-rs.svg -e svg
```

If the command fails (non-zero exit), append:

```
## <> — Re-render failed — <ISO 8601 UTC timestamp>

<stderr from mmdr>
```

Then abort.

### Step 8 — Pass-2 comparison (inline)

Re-read the freshly-rendered `-rs.svg` (the `-js.svg` is unchanged from Step 2). Produce the same headed comparison as Step 2 — including the Mandatory visual-appearance checks, Structural diffs, and Visual defects in RS.

Pass 2 must explicitly answer: **does RS now visibly match JS?** If not — even if Pass 1's individual flagged items are gone — the work is incomplete. Don't shrink the gap from "totally different layout" to "right element counts but still totally different layout" and call it done.

### Step 9 — Append Pass-2 findings

```
## <> — Pass 2 findings — <ISO 8601 UTC timestamp>

<your Step 8 findings, verbatim>
```

### Step 10 — Summarize to user

Print a short status block:

```
svg-parity <> — done
  Pass 1 issues:  <count>
  Changes:        <N file(s) edited: list>
  Build:          ok
  Re-render:      ok
  Pass 2 issues:  <count>
  Visual match:   <yes / partial / no — honest assessment from Pass 2 visual-appearance check>

Log: COMPARISON_CHANGES.md
```

The `Visual match` line is **mandatory and must be honest**:
- **yes** — Pass 2 RS would look essentially the same as JS to a viewer (matching layout topology, similar size class, similar edge shapes, no visible overflow/cramming).
- **partial** — Some visible differences remain but the diagrams are recognizably similar. Briefly state what still differs (e.g. "regions still 2× narrower than JS, edges still straight instead of curved").
- **no** — Layouts visibly differ. Briefly state the dominant difference. This is NOT a failure of the skill — it is honest reporting that the user can act on. Never inflate "no" to "partial" or "partial" to "yes" because the element counts match.

If `Visual match: no` or `partial`, the status block must NOT use the word "resolved", "fixed", "done", or "parity" in any subsequent narration. Use "improved" or "mitigated" instead.

## Notes

- `COMPARISON_CHANGES.md` is append-only. Never overwrite. Create if missing.
- One fix pass, not a loop. If the user wants more, they re-invoke.
- Comparisons run inline in the main agent (no subagents). Be deliberate with `Read`/`grep` slicing for large SVGs so the raw payload doesn't dominate context.
- When abort conditions fire at preconditions (missing files), **nothing** is written to `COMPARISON_CHANGES.md`. When abort fires at build or re-render failure (steps 6, 7), the Pass-1 findings and Changes-applied sections are already committed and a failure marker is appended before stopping.
- Use absolute paths when running the `mmdr` binary to avoid CWD surprises.
- The skill does not run tests, does not commit, does not push. It edits source, rebuilds, re-renders, logs.
