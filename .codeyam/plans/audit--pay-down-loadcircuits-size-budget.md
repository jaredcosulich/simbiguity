---
title: "audit -- Pay down loadCircuits size budget"
mode: backend
createdAt: "2026-11-28T17:27:52Z"
prefix: "audit"
source: codeyam-audit
owns: ["SOURCE_EXCEEDS_SIZE_BUDGET_ADVISORY"]
---

Extract helpers from `loadCircuits` in `site/js/simbiguity.js` to fit its size budget of 150 lines.