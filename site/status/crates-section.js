// "Zero dependencies, checked": every crate and what it depends on.

import { renderCrateRow } from "./crate-row.js";
import { el } from "./dom.js";
import { renderSectionIntro } from "./section-intro.js";

export function renderCrates(report) {
  return [
    renderSectionIntro(
      "dependencies",
      "Zero dependencies, checked",
      "Read from every crate's Cargo.toml at build time. Workspace crates depend only on each other.",
    ),
    el(
      "table",
      { class: "crates" },
      el("thead", {}, el("tr", {}, el("th", {}, "Crate"), el("th", {}, "Builds on"), el("th", {}, "External"), el("th", { class: "num" }, "Tests"))),
      el("tbody", {}, (report.crates || []).map(renderCrateRow)),
    ),
  ];
}
