// The collapsible list of tests proving a capability; open when one fails.

import { el } from "./dom.js";
import { renderProof } from "./proof-item.js";
import { proofSummary } from "./report-view.js";

export function renderProofList(report, capability, tests) {
  return el(
    "details",
    { open: capability.status === "failing" },
    el("summary", {}, proofSummary(tests)),
    el("ul", { class: "proofs" }, tests.map((t) => renderProof(report, t))),
  );
}
