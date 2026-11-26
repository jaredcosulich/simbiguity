// Shown when the report cannot be fetched, with the command that builds it.

import { el } from "./dom.js";

export function renderLoadError(url, err) {
  return el(
    "div",
    { class: "error" },
    el("p", {}, `Couldn't load the test report (${url}): ${err.message}.`),
    el("p", {}, "It is generated on deploy. To build it locally, run ", el("code", {}, "cargo run -p xtask -- site-report"), "."),
  );
}
