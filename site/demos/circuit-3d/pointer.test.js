import { describe, it } from "node:test";
import assert from "node:assert/strict";

import { Pick } from "../../js/simbiguity-render.js";
import { canvasPixel, canvasSize, clickAction, cursorFor } from "./pointer.js";

describe("canvasSize", () => {
  // The backing store is the CSS size times the pixel ratio, rounded.
  it("scales by the device pixel ratio", () => {
    assert.deepEqual(canvasSize(480, 300, 2), [960, 600]);
    assert.deepEqual(canvasSize(100.4, 50.6, 1), [100, 51]);
  });

  // A missing ratio counts as one and a collapsed canvas is still one pixel.
  it("defaults the ratio and never returns zero", () => {
    assert.deepEqual(canvasSize(10, 20, undefined), [10, 20]);
    assert.deepEqual(canvasSize(10, 20, 0), [10, 20]);
    assert.deepEqual(canvasSize(0, 0, 2), [1, 1]);
  });
});

describe("canvasPixel", () => {
  // Client coordinates become canvas pixels, offset by the rectangle and scaled to the backing store.
  it("maps client coordinates into the backing store", () => {
    const rect = { left: 10, top: 20, width: 480, height: 300 };
    assert.deepEqual(canvasPixel(rect, 960, 600, 10, 20), [0, 0]);
    assert.deepEqual(canvasPixel(rect, 960, 600, 250, 170), [480, 300]);
  });
});

describe("clickAction", () => {
  // Clicking the switch flips it, blown or not.
  it("flips the switch", () => {
    assert.equal(clickAction(Pick.SWITCH, false), "toggle");
    assert.equal(clickAction(Pick.SWITCH, true), "toggle");
  });

  // Clicking the bulb replaces it only when it is blown.
  it("replaces only a blown bulb", () => {
    assert.equal(clickAction(Pick.BULB, true), "replace");
    assert.equal(clickAction(Pick.BULB, false), null);
  });

  // The battery and empty space do nothing.
  it("does nothing elsewhere", () => {
    assert.equal(clickAction(Pick.BATTERY, true), null);
    assert.equal(clickAction(Pick.NOTHING, false), null);
  });
});

describe("cursorFor", () => {
  // The cursor is a pointer exactly where a click does something.
  it("points where a click acts and grabs elsewhere", () => {
    assert.equal(cursorFor(Pick.SWITCH, false), "pointer");
    assert.equal(cursorFor(Pick.BULB, true), "pointer");
    assert.equal(cursorFor(Pick.BULB, false), "grab");
    assert.equal(cursorFor(Pick.NOTHING, false), "grab");
  });
});
