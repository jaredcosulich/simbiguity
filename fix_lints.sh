#!/bin/bash
for file in crates/simbiguity-puzzle/src/lib.rs crates/simbiguity-circuits-puzzle/src/lib.rs crates/simbiguity-physics/src/lib.rs crates/simbiguity-physics-wasm/src/lib.rs crates/simbiguity-physics-view/src/lib.rs crates/simbiguity-plot/src/lib.rs; do
  if ! grep -q "cfg_attr" "$file"; then
    echo -e '#![cfg_attr(\n    not(test),\n    deny(\n        clippy::unwrap_used,\n        clippy::expect_used,\n        clippy::panic,\n        clippy::unreachable\n    )\n)]\n'"$(cat $file)" > "$file"
  fi
done
