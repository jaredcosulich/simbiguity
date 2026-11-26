// The row of headline numbers at the top of the page.

import { el } from "./dom.js";
import { renderStat } from "./stat.js";

export function renderStats(s) {
  return el(
    "div",
    { class: "stats" },
    renderStat(s.passed, "tests passing", s.failed ? "" : "good"),
    renderStat(s.failed, s.failed === 1 ? "test failing" : "tests failing", s.failed ? "bad" : ""),
    s.ignored ? renderStat(s.ignored, "ignored") : null,
    renderStat(`${s.proven}/${s.capabilities}`, "capabilities proven", s.failing ? "bad" : s.untested ? "warn" : "good"),
    s.untested ? renderStat(s.untested, "untested capabilities", "warn") : null,
    renderStat(s.externalDependencies, "external dependencies", s.externalDependencies ? "bad" : "good"),
  );
}
