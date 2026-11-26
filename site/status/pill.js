// A coloured status pill: passed/failed/ignored or passing/failing/untested.

import { el } from "./dom.js";

export function renderPill(status, text) {
  return el("span", { class: `pill ${status}` }, text);
}
