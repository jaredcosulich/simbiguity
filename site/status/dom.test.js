import { describe, it, before } from "node:test";
import assert from "node:assert/strict";

/** Just enough of the DOM for el and link: elements, text nodes, attributes. */
class FakeNode {}
class FakeText extends FakeNode {
  constructor(text) {
    super();
    this.textContent = text;
  }
}
class FakeElement extends FakeNode {
  constructor(tag) {
    super();
    this.tagName = tag;
    this.className = "";
    this.attributes = {};
    this.children = [];
  }
  setAttribute(k, v) {
    this.attributes[k] = String(v);
  }
  append(child) {
    this.children.push(child);
  }
}

let el;
let link;

before(async () => {
  globalThis.Node = FakeNode;
  globalThis.document = {
    createElement: (tag) => new FakeElement(tag),
    createTextNode: (text) => new FakeText(text),
  };
  ({ el, link } = await import("./dom.js"));
});

describe("el", () => {
  // The class attribute becomes the element's class name.
  it("sets the class name from the class attribute", () => {
    const node = el("p", { class: "build" });
    assert.equal(node.tagName, "p");
    assert.equal(node.className, "build");
    assert.deepEqual(node.attributes, {});
  });

  // Other attributes are set as strings; true becomes an empty attribute.
  it("sets other attributes, true as empty", () => {
    const node = el("details", { open: true, "data-test": "x", colspan: 3 });
    assert.deepEqual(node.attributes, { open: "", "data-test": "x", colspan: "3" });
  });

  // Null, undefined and false attributes are left off entirely.
  it("skips null, undefined and false attributes", () => {
    const node = el("a", { href: null, title: undefined, open: false });
    assert.deepEqual(node.attributes, {});
  });

  // Strings and numbers become text nodes, so markup is never parsed.
  it("appends strings and numbers as text", () => {
    const node = el("b", {}, "<i>x</i>", 3);
    assert.deepEqual(
      node.children.map((c) => [c instanceof FakeText, c.textContent]),
      [
        [true, "<i>x</i>"],
        [true, "3"],
      ],
    );
  });

  // Child elements are appended as they are, and an array of children is spread.
  it("appends nodes and spreads an array of children", () => {
    const child = el("span");
    const node = el("div", {}, [child, "a", "b"], "c");
    assert.equal(node.children[0], child);
    assert.deepEqual(node.children.slice(1).map((c) => c.textContent), ["a", "b", "c"]);
  });

  // Empty children are dropped so conditionals can return null.
  it("drops null, undefined and false children", () => {
    const node = el("div", {}, null, undefined, false, "kept");
    assert.equal(node.children.length, 1);
  });
});

describe("link", () => {
  // With a destination the content is wrapped in a link.
  it("wraps content in a link when there is a destination", () => {
    const a = link("https://example.com", "text");
    assert.equal(a.tagName, "a");
    assert.equal(a.attributes.href, "https://example.com");
  });

  // Without a destination the content is returned unwrapped.
  it("returns bare content without a destination", () => {
    assert.equal(link(null, "text"), "text");
  });
});
