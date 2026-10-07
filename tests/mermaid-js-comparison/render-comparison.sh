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
rs_png_ok=0
rs_png_fail=0

for mmd in "${files[@]}"; do
    name="$(basename "$mmd" .mmd)"
    total=$((total + 1))
    # A failed render must not leave an image from an earlier run behind.
    rm -f "$OUT_DIR/${name}-rs.svg" "$OUT_DIR/${name}-rs.png"

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

    # Progress every 50 files
    if [ $((total % 50)) -eq 0 ]; then
        echo "  ... processed $total files"
    fi
done

# Mermaid CLI's API uses the same rendering path, with a shared browser.
js_status=0
node "$SCRIPT_DIR/render-mermaid-js.mjs" "${files[@]}" || js_status=$?
python3 "$SCRIPT_DIR/generate-gallery.py" "$REF_DIR" "$OUT_DIR"

echo ""
echo "Done. $total source files processed."
echo "  mermaid-rs: $rs_ok ok, $rs_fail failed"
echo "  PNGs: mermaid-rs $rs_png_ok ok, $rs_png_fail failed"
echo "  Output: $OUT_DIR/"
echo "  Gallery: $OUT_DIR/index.html"
if [ "$js_status" -ne 0 ] || [ "$rs_fail" -ne 0 ] || [ "$rs_png_fail" -ne 0 ]; then
    exit 1
fi
