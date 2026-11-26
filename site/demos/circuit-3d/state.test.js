import { describe, it } from "node:test";
import assert from "node:assert/strict";

import { BULB_VOLTS, DEFAULT_CAMERA, VOLTS_MAX, hashFromState, stateFromHash } from "./state.js";

describe("stateFromHash", () => {
  // An empty hash is the default: open switch, rated volts, live animation, default camera.
  it("reads an empty or bare hash as the 3D default state", () => {
    assert.deepEqual(stateFromHash(""), { closed: false, volts: BULB_VOLTS, fixedTime: null, camera: null });
    assert.deepEqual(stateFromHash("#"), stateFromHash(""));
  });

  // Every token is read: closed, volts, frozen time and camera.
  it("reads every token", () => {
    assert.deepEqual(stateFromHash("#closed&v=16&t=2.5&cam=-50,20,7"), {
      closed: true,
      volts: 16,
      fixedTime: 2.5,
      camera: { yaw: -50, pitch: 20, distance: 7 },
    });
  });

  // Volts are clamped to the slider and nonsense falls back to the rated volts.
  it("clamps or ignores bad volts", () => {
    assert.equal(stateFromHash("#v=99").volts, VOLTS_MAX);
    assert.equal(stateFromHash("#v=-3").volts, 0);
    assert.equal(stateFromHash("#v=abc").volts, BULB_VOLTS);
    assert.equal(stateFromHash("#v=").volts, BULB_VOLTS);
  });

  // A negative or unreadable time means live animation; a camera needs three finite numbers.
  it("rejects bad times and incomplete cameras", () => {
    assert.equal(stateFromHash("#t=-1").fixedTime, null);
    assert.equal(stateFromHash("#t=x").fixedTime, null);
    assert.equal(stateFromHash("#t=0").fixedTime, 0);
    assert.equal(stateFromHash("#cam=1,2").camera, null);
    assert.equal(stateFromHash("#cam=1,2,x").camera, null);
  });

  // A cache-buster appended after a question mark inside the hash is ignored.
  it("ignores anything after a question mark", () => {
    assert.deepEqual(stateFromHash("#closed&t=2.5?_cb=123"), stateFromHash("#closed&t=2.5"));
  });
});

describe("hashFromState", () => {
  // The default state has no hash at all.
  it("writes nothing for the default state", () => {
    assert.equal(hashFromState({ closed: false, volts: BULB_VOLTS }), "");
  });

  // Every non-default field is written, with the camera rounded.
  it("writes every non-default field", () => {
    const h = hashFromState({ closed: true, volts: 16, fixedTime: 2.5, camera: { yaw: 25.04, pitch: 38.06, distance: 9.5049 } });
    assert.equal(h, "#closed&v=16&t=2.5&cam=25,38.1,9.5");
  });

  // Writing then reading gives the same state back.
  it("round-trips a state with a camera through stateFromHash", () => {
    const s = { closed: true, volts: 7.5, fixedTime: 1, camera: { ...DEFAULT_CAMERA } };
    assert.deepEqual(stateFromHash(hashFromState(s)), s);
  });
});
