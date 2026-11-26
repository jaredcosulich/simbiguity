// A capability's live demo, embedded and lazy-loaded, with an open link.

import { el } from "./dom.js";
import { observeDemo } from "./lazy-demos.js";
import { demoEmbedUrl, demoUrl } from "./report-view.js";

export function renderDemo(capability) {
  const url = demoUrl(capability);
  if (!url) return null;
  const frame = el("iframe", {
    title: `Live demo: ${capability.name}`,
    loading: "lazy",
    tabindex: "-1",
    "aria-hidden": "true",
    "data-src": demoEmbedUrl(capability),
  });
  frame.addEventListener("load", () => frame.parentElement?.querySelector(".placeholder")?.remove());
  const box = el(
    "div",
    { class: "demo" },
    el("div", { class: "placeholder" }, "Loading live demo…"),
    frame,
    el("a", { class: "open", href: url, "aria-label": `Open the ${capability.name} demo` }, el("span", {}, "Open demo ↗")),
  );
  observeDemo(box, frame);
  return box;
}
