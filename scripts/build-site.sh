#!/usr/bin/env sh
# Builds the WASM bridges and copies them into site/pkg/ (gitignored).
# Usage: scripts/build-site.sh   (from anywhere in the repo)
set -eu

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

cargo build -p simbiguity-circuits-wasm --target wasm32-unknown-unknown --release

# Respect CARGO_TARGET_DIR / .cargo/config instead of assuming ./target.
target_dir="$(cargo metadata --format-version 1 --no-deps \
  | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')"

mkdir -p site/pkg
cp "$target_dir/wasm32-unknown-unknown/release/simbiguity_circuits_wasm.wasm" site/pkg/
echo "site/pkg/simbiguity_circuits_wasm.wasm: $(wc -c < site/pkg/simbiguity_circuits_wasm.wasm) bytes"
