// A capability card's title and status pill.

import { el } from "./dom.js";
import { renderPill } from "./pill.js";
import { capabilityLabel } from "./report-view.js";

export function renderCardHeader(capability) {
  return el(
    "header",
    {},
    el("h4", {}, capability.name),
    renderPill(capability.status, capabilityLabel(capability.status)),
  );
}
