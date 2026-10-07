# Mermaid comparison suite

The suite is pinned to **Mermaid 12.1.0** and **Mermaid CLI 12.0.0**, the npm stable releases verified on 2026-10-07. Both the root and this directory have matching dependencies and lockfiles. Node 22.13 or newer is required by this CLI release.

## Run

From the repository root:

```sh
npm ci --prefix tests/mermaid-js-comparison
bash tests/mermaid-js-comparison/render-comparison.sh
```

For one fixture:

```sh
bash tests/mermaid-js-comparison/render-comparison.sh usecase-complete-example
```

The script builds Rust, renders every selected source into SVG and PNG for both engines, and regenerates `comparison-output/index.html`. The Mermaid batch uses the CLI's official `renderMermaid` API with a shared Chromium browser. Each diagram gets a separate render page. PNGs are rasterized from the generated SVG after fonts are ready. `comparison-output/mermaid-js-run.json` records the Mermaid versions, selected fixture count, output counts, and errors for the most recent run. A failed render removes its prior outputs, and any render failure makes the comparison command fail after building the gallery.

The gallery is generated locally and is ignored by Git. Mermaid's current defaults are retained: ELK, neo, and redux-color where applicable. Sources with explicit appearance settings keep those settings.

## Corpus

There are **458 sources**: the previous 422 comparisons plus **24 use case** and **12 agentflow** examples extracted from the official `mermaid@12.1.0` documentation tag. Fixture names follow documentation headings. All sources run through both engines.

Nine older sources needed repairs to run with the latest CLI: six edge-ID/property examples, one mindmap class example, and two image examples. Edge declarations now use `A e1@--> B` and `e1@{...}`; mindmap classes use a separate `:::` line. Image fixtures embed the same deterministic 160×80 PNG instead of an unavailable placeholder URL, so image layout and aspect-ratio comparisons work offline.

The image comparisons also exposed a Rust parser gap: image declarations without an explicit `shape` were discarded as edge metadata. Native image nodes now retain complete data URLs, reserve image dimensions in layout, place captions above/below, and honor aspect-ratio constraints. Embedded image dimensions are decoded by the existing SVG backend when the `png` feature is enabled.

## Diagram-family audit

The Mermaid 12.1 detector registry and Rust's parser dispatch were compared. **Use case** (`usecase-beta`) and **agentflow** (`agentflow-beta`) were the two missing diagram families; both now have native parsing, layout, and SVG rendering. Rust's 33 families cover the diagram families in this registry, consolidating the class/state/flowchart renderer variants and railroad grammar variants. Mermaid's `info` command is a version diagnostic, not a diagram family.

### Use case support

- Actors: normal, hollow, awesome, and native icon lookup; business glyphs and stereotypes.
- Ellipse and rectangular use cases, forward references, five directions (default LR).
- System boundaries and package tabs; classes and direct styles.
- Associations, include, extend, and hollow generalization triangles; edge IDs and curve/style overrides.
- Notes and ordered JSON tables, including nested arrays/objects and duplicate-key replacement.
- Plain labels retain literal backslash escapes; entity codes and Markdown labels render through the text pipeline.
- Accessibility titles/descriptions and per-node accessible names; role font sizes, families, and weights.

### Agentflow support

- Nested `flow` containers, directions, edges, classes, direct styles, and accessibility descriptions.
- `global` nodes stay outside flows even when referenced from inside them.
- Connectors render as rounded nodes; authored metadata and connector references are retained in the graph.
- Task/tool/input/decision/refdoc/action shape aliases, reference-document wave and left margin.
- Collapsed containers replace hidden descendants with a title, separator, and ellipsis, redirecting external edges.
- Arbitrary metadata is retained; prototype keys are removed recursively.

## Remaining gaps

**Successful rendering does not establish visual parity.** The new families have been inspected alongside the latest Mermaid output; the following gaps remain:

| Gap | Effect |
| --- | --- |
| ELK layout | Rust uses its native graph layout. Branch order, container placement, isolated-node placement, edge bends, and overall aspect ratios can differ substantially. |
| Neo look and redux themes | Rust retains its existing themes and classic shape treatment. The latest palette rotation, role colors, shadows, rounding, and shape sizes are not reproduced. |
| Use case configuration | Palette schemes, diagram padding, and useMaxWidth are not yet applied from the usecase namespace. |
| Extended edge presentation | Animation metadata is retained for the new syntaxes but SVG edge animation is not emitted; extra dashes do not add minimum-rank length in the native layout. |
| Icon packs and fonts | Rust uses its native icon/font registry. Mermaid CLI's registered packs and embedded Open Sans may produce different glyphs or label widths. |
| External image assets | Rust does not fetch URLs to infer natural dimensions or rasterize remote assets. Use embedded images for offline PNG comparison; authored dimensions provide the fallback for external references and builds without the `png` feature. Image-node wrapping-width configuration still uses Mermaid 12's default 120 px. |
| Agentflow tooling | Source mappings, semantic-model projections, connector-reference diagnostics, and Mermaid's warning/error catalog are not implemented. Metadata is available through `Graph::element_metadata`. |
| Grammar conformance | The suite covers the 36 documentation examples and targeted negative cases. Full upstream grammar/conformance suites, including all unusual metadata/comment/label combinations, remain to be ported. |
| Railroad grammar variants | Existing Rust support remains a documented core subset of the upstream variants. |

Official sources: [Mermaid 12.1 release](https://github.com/mermaid-js/mermaid/releases/tag/mermaid%4012.1.0), [Mermaid 12.0 appearance changes](https://github.com/mermaid-js/mermaid/releases/tag/mermaid%4012.0.0), [use case syntax](https://mermaid.js.org/syntax/usecase.html), [agentflow syntax](https://mermaid.js.org/syntax/agentflow.html).
