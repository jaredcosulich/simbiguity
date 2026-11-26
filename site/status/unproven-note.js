// "No test proves this yet" for an untested capability.

import { el } from "./dom.js";

export function renderUnprovenNote(capability) {
  return el(
    "p",
    { class: "unproven" },
    "No test proves this yet",
    capability.unmatched?.length ? [" (expected in ", el("code", {}, capability.unmatched.join(", ")), ")"] : "",
    ".",
  );
}
