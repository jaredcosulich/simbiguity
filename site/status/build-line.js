// "Built from commit abc1234 on 26 Nov 2026 · 9 crates · 400 tests".

import { el, link } from "./dom.js";
import { commitUrl, formatDate } from "./report-view.js";

export function renderBuildLine(report, summary) {
  const short = (report.commit || "").slice(0, 7);
  return el(
    "p",
    { class: "build" },
    "Built from commit ",
    link(commitUrl(report), el("code", {}, short || "unknown")),
    report.committedAt ? ` on ${formatDate(report.committedAt)}` : "",
    ` · ${summary.crates} crates · ${summary.tests} tests`,
  );
}
