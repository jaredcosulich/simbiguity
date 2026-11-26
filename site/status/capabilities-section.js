// "What it can do": every capability, grouped by crate.

import { renderCapabilityGroup } from "./capability-group.js";
import { capabilitiesByCrate } from "./report-view.js";
import { renderSectionIntro } from "./section-intro.js";

export function renderCapabilities(report) {
  return [
    renderSectionIntro(
      "capabilities",
      "What it can do",
      "Each card is one thing an engine can do, the demo that shows it, and the tests that prove it. ",
      "A capability with no test is marked untested rather than hidden.",
    ),
    capabilitiesByCrate(report).map(({ crate, capabilities }) => renderCapabilityGroup(report, crate, capabilities)),
  ];
}
