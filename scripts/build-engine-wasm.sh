#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

wasm-pack build "$ROOT_DIR/crates/chemcore-engine" \
  --target web \
  --out-dir "$ROOT_DIR/viewer/engine" \
  --features wasm

# wasm-pack writes an ignore-all file for publishable packages. In this repo the
# viewer consumes these runtime artifacts directly, so they need to stay tracked.
rm -f "$ROOT_DIR/viewer/engine/.gitignore"
