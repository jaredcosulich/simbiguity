// A page section's heading and the muted sentence beneath it.

import { el } from "./dom.js";

export function renderSectionIntro(id, title, ...lede) {
  return [el("h2", { id }, title), el("p", { class: "section-lede" }, ...lede)];
}
