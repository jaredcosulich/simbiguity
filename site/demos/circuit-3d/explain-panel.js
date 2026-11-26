// The explanation panel under the 3D view. Text goes in as text, never HTML.

const box = () => document.getElementById("explain");

function heading(text, className) {
  const h2 = document.createElement("h2");
  if (className) h2.className = className;
  h2.textContent = text;
  return h2;
}

function paragraph(text) {
  const p = document.createElement("p");
  p.textContent = text;
  return p;
}

/**
 * Shows what the circuit is doing; an issue gets the warning border.
 * @param {{ title: string, body: string, issue: boolean }} e
 */
export function showExplanation({ title, body, issue }) {
  const b = box();
  b.className = `explain panel${issue ? " issue" : ""}`;
  b.replaceChildren(heading(title), paragraph(body));
}

/** Replaces the panel with the reason the 3D view could not start. */
export function showLoadError(err) {
  box().replaceChildren(heading("The 3D view did not load", "error"), paragraph(err.message));
}
