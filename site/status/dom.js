// DOM helpers shared by every status-page component.

/** `el("p", { class: "x" }, "text", child)`: text is always set as text. */
export function el(tag, attrs = {}, ...children) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v === null || v === undefined || v === false) continue;
    if (k === "class") node.className = v;
    else node.setAttribute(k, v === true ? "" : v);
  }
  for (const child of children.flat()) {
    if (child === null || child === undefined || child === false) continue;
    node.append(child instanceof Node ? child : document.createTextNode(String(child)));
  }
  return node;
}

/** A link when there is somewhere to go, otherwise just the content. */
export function link(href, text) {
  return href ? el("a", { href }, text) : text;
}
