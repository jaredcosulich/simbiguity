// One proving test inside a capability card: result dot, description, path.

import { el, link } from "./dom.js";
import { rustPath, sourceUrl } from "./report-view.js";
import { renderTestDescription } from "./test-description.js";

export function renderProof(report, test) {
  return el(
    "li",
    { class: test.status },
    el("span", { class: `dot ${test.status}`, title: test.status }),
    el(
      "span",
      {},
      renderTestDescription(test),
      el("span", { class: "name mono" }, link(sourceUrl(report, test), rustPath(test))),
    ),
  );
}
