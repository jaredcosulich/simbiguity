// A test's plain-English description, or a muted "No description".

import { el } from "./dom.js";

export function renderTestDescription(test) {
  return test.description || el("span", { class: "no-description" }, "No description");
}
