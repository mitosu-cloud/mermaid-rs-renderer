#!/bin/bash
# Render comparison: mermaid-rs vs mermaid-js
# Usage:
#   bash render-comparison.sh                              # all files
#   bash render-comparison.sh flowchart-k3s-cluster-wireguard  # one file (no .mmd extension)
#   bash render-comparison.sh flowchart-k3s-cluster-wireguard.mmd  # also works
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
REF_DIR="$SCRIPT_DIR/reference"
OUT_DIR="$SCRIPT_DIR/comparison-output"
MMDR="$REPO_DIR/target/release/mmdr"
MMDC="$SCRIPT_DIR/node_modules/.bin/mmdc"

mkdir -p "$OUT_DIR"

# Build our renderer
echo "Building mermaid-rs..."
cd "$REPO_DIR"
cargo build --release 2>/dev/null
# Determine which files to process
if [ -n "$1" ]; then
    # Single file mode — strip .mmd if provided
    name="${1%.mmd}"
    mmd="$REF_DIR/${name}.mmd"
    if [ ! -f "$mmd" ]; then
        echo "Error: $mmd not found"
        exit 1
    fi
    files=("$mmd")
else
    files=("$REF_DIR"/*.mmd)
fi

total=0
rs_ok=0
rs_fail=0
js_ok=0
js_fail=0
rs_png_ok=0
rs_png_fail=0
js_png_ok=0
js_png_fail=0

for mmd in "${files[@]}"; do
    name="$(basename "$mmd" .mmd)"
    total=$((total + 1))
    # A failed render must not leave an image from an earlier run behind.
    rm -f "$OUT_DIR/${name}-rs.svg" "$OUT_DIR/${name}-js.svg" \
          "$OUT_DIR/${name}-rs.png" "$OUT_DIR/${name}-js.png"

    # Render with mermaid-rs
    if "$MMDR" -i "$mmd" -o "$OUT_DIR/${name}-rs.svg" 2>/dev/null; then
        rs_ok=$((rs_ok + 1))
        if "$MMDR" -i "$mmd" -o "$OUT_DIR/${name}-rs.png" 2>/dev/null; then
            rs_png_ok=$((rs_png_ok + 1))
        else
            rs_png_fail=$((rs_png_fail + 1))
            echo "  [rs PNG FAIL] $name"
        fi
    else
        rs_fail=$((rs_fail + 1))
        echo "  [rs FAIL] $name"
    fi

    # Render with mermaid-js
    if "$MMDC" -i "$mmd" -o "$OUT_DIR/${name}-js.svg" --quiet 2>/dev/null \
       && ! grep -q 'Syntax error in text' "$OUT_DIR/${name}-js.svg"; then
        js_ok=$((js_ok + 1))
        if node "$SCRIPT_DIR/rasterize-svg.mjs" "$OUT_DIR/${name}-js.svg" 2>/dev/null; then
            js_png_ok=$((js_png_ok + 1))
        else
            js_png_fail=$((js_png_fail + 1))
            echo "  [js PNG FAIL] $name"
        fi
    else
        rm -f "$OUT_DIR/${name}-js.svg"
        js_fail=$((js_fail + 1))
        echo "  [js FAIL] $name"
    fi

    # Progress every 50 files
    if [ $((total % 50)) -eq 0 ]; then
        echo "  ... processed $total files"
    fi
done

python3 "$SCRIPT_DIR/generate-gallery.py" "$REF_DIR" "$OUT_DIR"

echo ""
echo "Done. $total source files processed."
echo "  mermaid-rs: $rs_ok ok, $rs_fail failed"
echo "  mermaid-js: $js_ok ok, $js_fail failed"
echo "  PNGs: mermaid-rs $rs_png_ok ok, $rs_png_fail failed; mermaid-js $js_png_ok ok, $js_png_fail failed"
echo "  Output: $OUT_DIR/"
echo "  Gallery: $OUT_DIR/index.html"
