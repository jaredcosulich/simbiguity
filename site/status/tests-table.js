// The every-test table; `show(rows)` replaces its body.

import { el } from "./dom.js";

export function renderTestsTable() {
  const body = el("tbody");
  const node = el(
    "table",
    { class: "tests" },
    el("thead", {}, el("tr", {}, el("th", {}, "Result"), el("th", {}, "What it checks"), el("th", {}, "Test"))),
    body,
  );
  function show(rows) {
    body.replaceChildren(...rows);
    if (!rows.length) body.append(el("tr", {}, el("td", { colspan: "3" }, "No tests match these filters.")));
  }
  return { node, show };
}
