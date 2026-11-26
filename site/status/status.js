// The engine status page: fetch report.json and render the build identity,
// capability cards with live demos, the crate table and every test.
// Each section is its own module; pure logic lives in report-view.js.

import { renderCapabilities } from "./capabilities-section.js";
import { renderCrates } from "./crates-section.js";
import { renderHeader } from "./header.js";
import { renderLoadError } from "./load-error.js";
import { loadReport } from "./load-report.js";
import { parseFilters, reportUrl } from "./report-view.js";
import { renderTests } from "./tests-section.js";

const root = document.getElementById("status");

async function main() {
  const url = reportUrl(location.search);
  let report;
  try {
    report = await loadReport(url);
  } catch (err) {
    root.replaceChildren(renderLoadError(url, err));
    return;
  }
  root.replaceChildren(...[renderHeader(report), renderCapabilities(report), renderCrates(report), renderTests(report)].flat(Infinity));
  // When the page opens with a filter in the URL, jump to the table.
  if (parseFilters(location.hash).crate) document.getElementById("tests")?.scrollIntoView();
}

main();
