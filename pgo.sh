#!/usr/bin/env bash
# Profile-Guided Optimization build for Silicon
# This script builds an optimized binary using PGO, which can provide 10-20% speedup.
#
# Usage: ./pgo.sh [FILE_TO_PROFILE]
#   FILE_TO_PROFILE: optional large file to open during profiling (default: silicon-term/src/commands.rs)

set -euo pipefail

PGO_DIR="/tmp/silicon-pgo-data"
PROFILE_FILE="${1:-silicon-term/src/commands.rs}"
INSTALL_DIR="$HOME/.cargo/bin"

echo "=== Silicon PGO Build ==="
echo "Profile data dir: $PGO_DIR"
echo "Profiling file: $PROFILE_FILE"
echo ""

# Clean previous PGO data
rm -rf "$PGO_DIR"
mkdir -p "$PGO_DIR"

# Step 1: Build instrumented binary
echo "[1/4] Building instrumented binary..."
RUSTFLAGS="-Cprofile-generate=$PGO_DIR" cargo build --release 2>&1

echo "[2/4] Running profiling workload..."
echo "  Opening $PROFILE_FILE - please interact with the editor:"
echo "  - Scroll around (j/k/Ctrl-d/Ctrl-u)"
echo "  - Search for text (/)"
echo "  - Open file picker (Space+f)"
echo "  - Type some text (i + type + Esc)"
echo "  - Then quit (:q!)"
echo ""
./target/release/si "$PROFILE_FILE"

# Step 3: Merge profile data
echo "[3/4] Merging profile data..."
llvm-profdata merge -o "$PGO_DIR/merged.profdata" "$PGO_DIR"

# Step 4: Rebuild with profile data
echo "[4/4] Building optimized binary with PGO..."
RUSTFLAGS="-Cprofile-use=$PGO_DIR/merged.profdata" cargo build --release 2>&1

# Install
cp target/release/si "$INSTALL_DIR/si"
echo ""
echo "=== Done! PGO-optimized binary installed to $INSTALL_DIR/si ==="
echo "Binary size: $(du -h "$INSTALL_DIR/si" | cut -f1)"
