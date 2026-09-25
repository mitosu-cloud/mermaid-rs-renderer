#!/usr/bin/env python3
"""Build a local side-by-side gallery from comparison PNGs and SVGs."""

from html import escape
from pathlib import Path
import sys
from urllib.parse import quote


def image_panel(output: Path, stem: str, suffix: str, label: str) -> str:
    png = output / f"{stem}-{suffix}.png"
    svg = output / f"{stem}-{suffix}.svg"
    # Compare vectors in the same browser; native PNG rasterizers differ in
    # font antialiasing. Keep the PNG export available for separate inspection.
    image = svg if svg.exists() else png if png.exists() else None
    if image is None:
        content = '<p class="missing">No image generated</p>'
    else:
        url = quote(image.name)
        content = (
            f'<a href="{url}" target="_blank">'
            f'<img loading="lazy" src="{url}" alt="{escape(stem)} — {label}"></a>'
        )
        if png.exists():
            content += f'<a href="{quote(png.name)}" target="_blank">PNG</a>'
    return f'<div class="panel"><h3>{label}</h3>{content}</div>'


def main() -> None:
    reference, output = map(Path, sys.argv[1:3])
    sources = sorted(reference.glob("*.mmd"))
    pairs = []
    missing = 0
    for source in sources:
        stem = source.stem
        has_rs = (output / f"{stem}-rs.png").exists() or (output / f"{stem}-rs.svg").exists()
        has_js = (output / f"{stem}-js.png").exists() or (output / f"{stem}-js.svg").exists()
        if not has_rs or not has_js:
            missing += 1
        source_url = "../reference/" + quote(source.name)
        pairs.append(
            f'<section class="pair" data-name="{escape(stem)}" data-missing="{int(not has_rs or not has_js)}">'
            f'<header><h2>{escape(stem)}</h2><a href="{source_url}" target="_blank">Source .mmd</a></header>'
            f'<div class="images">'
            f'{image_panel(output, stem, "rs", "mermaid-rs")}'
            f'{image_panel(output, stem, "js", "mermaid-js")}'
            f'</div></section>'
        )
    html = f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Mermaid render comparison</title>
<style>
  * {{ box-sizing: border-box; }}
  body {{ margin: 0; font: 15px system-ui, sans-serif; color: #243041; background: #f2f5f8; }}
  .toolbar {{ position: sticky; top: 0; z-index: 2; padding: 14px max(20px, calc((100vw - 1600px)/2));
    background: #fff; border-bottom: 1px solid #d9e0e8; display: flex; gap: 16px; align-items: center; flex-wrap: wrap; }}
  h1 {{ margin: 0; font-size: 20px; }}
  .toolbar input[type=search] {{ min-width: 240px; padding: 8px 10px; border: 1px solid #aeb9c6; border-radius: 5px; }}
  .toolbar label {{ white-space: nowrap; }}
  .count {{ color: #5c6775; }}
  main {{ max-width: 1640px; margin: 20px auto; padding: 0 20px; }}
  .pair {{ margin-bottom: 20px; background: #fff; border: 1px solid #d9e0e8; border-radius: 7px; overflow: hidden; }}
  .pair header {{ display: flex; align-items: baseline; justify-content: space-between; gap: 10px;
    padding: 12px 16px; border-bottom: 1px solid #e5eaf0; }}
  h2 {{ font-size: 16px; margin: 0; overflow-wrap: anywhere; }}
  a {{ color: #175d9d; }}
  .images {{ display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); }}
  .panel {{ min-width: 0; padding: 12px; text-align: center; }}
  .panel + .panel {{ border-left: 1px solid #e5eaf0; }}
  h3 {{ font-size: 13px; margin: 0 0 10px; color: #637285; }}
  .panel img {{ display: block; width: 100%; max-height: 550px; object-fit: contain; background: #fff; }}
  .missing {{ display: grid; place-items: center; min-height: 120px; color: #a14242; background: #fff8f8; }}
  @media (max-width: 800px) {{ .images {{ grid-template-columns: 1fr; }} .panel + .panel {{ border-left: 0; border-top: 1px solid #e5eaf0; }} }}
</style>
</head>
<body>
<div class="toolbar">
  <h1>Mermaid render comparison</h1>
  <input id="search" type="search" placeholder="Filter by fixture name" aria-label="Filter fixtures">
  <label><input id="missing" type="checkbox"> Show missing renders only</label>
  <span id="count" class="count">{len(sources)} fixtures · {missing} missing pairs</span>
</div>
<main>{''.join(pairs)}</main>
<script>
const cards = [...document.querySelectorAll('.pair')];
const search = document.querySelector('#search');
const missing = document.querySelector('#missing');
const count = document.querySelector('#count');
function filter() {{
  const needle = search.value.trim().toLowerCase();
  let shown = 0;
  for (const card of cards) {{
    const visible = card.dataset.name.toLowerCase().includes(needle) && (!missing.checked || card.dataset.missing === '1');
    card.hidden = !visible;
    if (visible) shown++;
  }}
  count.textContent = `${{shown}} of ${{cards.length}} fixtures · {missing} missing pairs`;
}}
search.addEventListener('input', filter);
missing.addEventListener('change', filter);
</script>
</body>
</html>
"""
    (output / "index.html").write_text(html, encoding="utf-8")


if __name__ == "__main__":
    main()
