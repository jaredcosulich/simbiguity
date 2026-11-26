#!/usr/bin/env sh
# Builds the WASM bridges and copies them into site/pkg/ (gitignored).
# Usage: scripts/build-site.sh   (from anywhere in the repo)
set -eu

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

bridges="simbiguity-circuits-wasm simbiguity-render-wasm"
for crate in $bridges; do
  cargo build -p "$crate" --target wasm32-unknown-unknown --release
done

# Respect CARGO_TARGET_DIR / .cargo/config instead of assuming ./target.
target_dir="$(cargo metadata --format-version 1 --no-deps \
  | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')"

mkdir -p site/pkg
for crate in $bridges; do
  file="$(echo "$crate" | tr - _).wasm"
  cp "$target_dir/wasm32-unknown-unknown/release/$file" site/pkg/
  echo "site/pkg/$file: $(wc -c < "site/pkg/$file") bytes"
done
