// One row of the every-test table.

import { el, link } from "./dom.js";
import { renderPill } from "./pill.js";
import { rustPath, sourceUrl } from "./report-view.js";
import { renderTestDescription } from "./test-description.js";

export function renderTestRow(report, t) {
  return el(
    "tr",
    { class: t.status, "data-test": t.id },
    el("td", {}, renderPill(t.status, t.status)),
    el("td", { class: "what" }, renderTestDescription(t)),
    el("td", {}, el("code", {}, t.crate), el("div", { class: "rust mono" }, link(sourceUrl(report, t), rustPath(t)))),
  );
}
