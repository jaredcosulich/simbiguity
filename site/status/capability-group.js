// One crate's capability cards under the crate's name.

import { renderCard } from "./capability-card.js";
import { el } from "./dom.js";

export function renderCapabilityGroup(report, crate, capabilities) {
  return [
    el("h3", {}, el("code", {}, crate)),
    el("div", { class: "cards" }, capabilities.map((c) => renderCard(report, c))),
  ];
}
