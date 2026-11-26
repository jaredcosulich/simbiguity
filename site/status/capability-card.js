// One capability: what it does, its live demo, and the tests that prove it.

import { renderCardHeader } from "./card-header.js";
import { renderDemo } from "./demo-embed.js";
import { el } from "./dom.js";
import { renderProofList } from "./proof-list.js";
import { provingTests } from "./report-view.js";
import { renderUnprovenNote } from "./unproven-note.js";

export function renderCard(report, capability) {
  const tests = provingTests(report, capability);
  return el(
    "article",
    { class: `card ${capability.status}`, "data-capability": capability.name },
    renderCardHeader(capability),
    el("p", {}, capability.description),
    renderDemo(capability),
    tests.length ? renderProofList(report, capability, tests) : renderUnprovenNote(capability),
  );
}
