// One crate in the dependency table: what it builds on and its test count.

import { el } from "./dom.js";
import { renderPill } from "./pill.js";

export function renderCrateRow(c) {
  return el(
    "tr",
    { class: c.failed ? "failed" : "" },
    el("td", {}, el("code", {}, c.name), el("div", { class: "rust" }, c.description)),
    el("td", {}, c.workspaceDependencies.length ? c.workspaceDependencies.map((d) => el("div", {}, el("code", {}, d))) : "nothing"),
    el("td", {}, c.externalDependencies.length ? c.externalDependencies.join(", ") : el("span", { class: "zero" }, "none")),
    el(
      "td",
      { class: "num" },
      c.buildFailed ? renderPill("failed", "build failed") : `${c.passed}/${c.passed + c.failed + c.ignored}`,
    ),
  );
}
