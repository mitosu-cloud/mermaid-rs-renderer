#!/bin/bash
# Render comparison: mermaid-rs vs mermaid-js
# Different: 
#   bas render-comparison.sh                              # all files
#   bash render-comparison.sh flowchart-k3s-cluster-wireguard  # one file (no .mmd extension)
#   bash render-comparison.sh flowchart-k3s-cluster-wireguard.mmd  # also works
#
set -e
# test
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
REF_DIR="$SCRIPT_DIR/reference"
OUT_DIR="$SCRIPT_DIR/output"
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

for mmd in "${files[@]}"; do
    name="$(basename "$mmd" .mmd)"
    total=$((total + 1))

    # Render with mermaid-rs
    if "$MMDR" -i "$mmd" -o "$OUT_DIR/${name}-rs.svg" 2>/dev/null; then
        rs_ok=$((rs_ok + 1))
    else
        rs_fail=$((rs_fail + 1))
        echo "  [rs FAIL] $name"
    fi

    # Render with mermaid-js
    if "$MMDC" -i "$mmd" -o "$OUT_DIR/${name}-js.svg" --quiet 2>/dev/null; then
        js_ok=$((js_ok + 1))
    else
        js_fail=$((js_fail + 1))
        echo "  [js FAIL] $name"
    fi

    # Progress every 50 files
    if [ $((total % 50)) -eq 0 ]; then
        echo "  ... processed $total files"
    fi
done

echo ""
echo "Done. $total source files processed."
echo "  mermaid-rs: $rs_ok ok, $rs_fail failed"
echo "  mermaid-js: $js_ok ok, $js_fail failed"
echo "  Output: $OUT_DIR/"
