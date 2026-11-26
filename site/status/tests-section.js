// "Every test": the filter bar, the match count and the filtered table,
// with the filters mirrored into the URL hash.

import { el } from "./dom.js";
import { countLabel, filterTests, nextHash, parseFilters } from "./report-view.js";
import { renderSectionIntro } from "./section-intro.js";
import { renderTestFilters } from "./test-filters.js";
import { renderTestRow } from "./test-row.js";
import { renderTestsTable } from "./tests-table.js";

export function renderTests(report) {
  const tests = report.tests || [];
  const crates = [...new Set(tests.map((t) => t.crate))];
  const initial = parseFilters(location.hash);
  if (initial.crate && !crates.includes(initial.crate)) initial.crate = "";

  const count = el("p", { class: "count" });
  const table = renderTestsTable();

  function update(state) {
    const shown = filterTests(tests, state);
    count.textContent = countLabel(shown.length, tests.length);
    table.show(shown.map((t) => renderTestRow(report, t)));
    const hash = nextHash(state, location.hash);
    if (hash !== null) history.replaceState(null, "", `${location.pathname}${location.search}${hash}`);
  }

  const filters = renderTestFilters(crates, initial, update);
  update(filters.state());

  return [
    renderSectionIntro(
      "tests",
      "Every test",
      "Each test with the plain-English comment written above it, its result from this build, and a link to its source at this exact commit.",
    ),
    filters.node,
    count,
    table.node,
  ];
}
