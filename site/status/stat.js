// One headline number with its label, toned good/bad/warn.

import { el } from "./dom.js";

export function renderStat(value, label, tone) {
  return el("div", { class: `stat ${tone || ""}` }, el("b", {}, value), el("span", {}, label));
}
