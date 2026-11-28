#!/usr/bin/env sh
set -eu

budget_file="$1"
pkg_dir="$2"

if [ ! -f "$budget_file" ]; then
  echo "ERROR: $budget_file is missing"
  exit 1
fi

bridges="simbiguity-circuits-wasm simbiguity-render-wasm simbiguity-physics-wasm"
for crate in $bridges; do
  file="$(echo "$crate" | tr - _).wasm"
  
  if [ ! -f "$pkg_dir/$file" ]; then
    # if it's missing entirely, that should probably fail as well? The original relies on wc failing or size being small.
    # original logic: wc -c < "site/pkg/$file". If missing, wc fails and size is empty.
    # let's just let wc fail.
    :
  fi

  size=$(wc -c < "$pkg_dir/$file")
  budget=$(awk -v f="$file" '$1 == f {print $2}' "$budget_file")
  if [ -z "$budget" ]; then
    echo "ERROR: Missing budget entry for $file in $budget_file"
    exit 1
  fi
  if [ "$size" -gt "$budget" ]; then
    echo "ERROR: $file size ($size) exceeds budget ($budget)"
    exit 1
  fi
done

bench_budget="site/bench-budget.txt"
if [ -f "$bench_budget" ]; then
  cargo run -q --release -p xtask bench > site/bench.json
  cargo run -q --release -p xtask check-bench "$bench_budget" site/bench.json
fi
