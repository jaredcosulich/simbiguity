// The crate / status / search controls above the every-test table.

import { el } from "./dom.js";
import { TEST_STATUSES } from "./report-view.js";

/**
 * Builds the filter bar preset to `initial`; calls `onChange(state)` on
 * every edit. Returns `{ node, state }`, where `state()` reads the controls.
 */
export function renderTestFilters(crates, initial, onChange) {
  const crateSelect = el("select", { name: "crate" }, el("option", { value: "" }, "All crates"), crates.map((c) => el("option", { value: c }, c)));
  const statusSelect = el("select", { name: "status" }, el("option", { value: "" }, "Any status"), TEST_STATUSES.map((s) => el("option", { value: s }, s)));
  const search = el("input", { type: "search", name: "q", placeholder: "e.g. Wheatstone, snapshot, spring" });
  const clear = el("button", { type: "button" }, "Clear");
  crateSelect.value = initial.crate;
  statusSelect.value = initial.status;
  search.value = initial.q;

  const state = () => ({ crate: crateSelect.value, status: statusSelect.value, q: search.value });
  const changed = () => onChange(state());
  crateSelect.addEventListener("change", changed);
  statusSelect.addEventListener("change", changed);
  search.addEventListener("input", changed);
  clear.addEventListener("click", () => {
    crateSelect.value = "";
    statusSelect.value = "";
    search.value = "";
    changed();
  });

  const node = el(
    "div",
    { class: "filters", role: "search" },
    el("label", {}, "Crate", crateSelect),
    el("label", {}, "Status", statusSelect),
    el("label", {}, "Search", search),
    clear,
  );
  return { node, state };
}
