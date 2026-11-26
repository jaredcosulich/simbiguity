// The build identity and headline numbers.

import { renderBuildLine } from "./build-line.js";
import { summarize } from "./report-view.js";
import { renderStats } from "./stats.js";

export function renderHeader(report) {
  const summary = summarize(report);
  return [renderBuildLine(report, summary), renderStats(summary)];
}
