import { describe, it } from "node:test";
import assert from "node:assert/strict";

import { Pick, READOUT_COUNT, readoutsFromArray } from "./simbiguity-render.js";

describe("readoutsFromArray", () => {
  // Every slot gets its name, flags become booleans and the camera is grouped.
  it("names every slot", () => {
    const r = readoutsFromArray(Float64Array.from([1, 12, 0.5, 12, 6, 1, 0.8, 1.2, 0, 1, 25, 38, 9.5, 2.5]));
    assert.deepEqual(r, {
      closed: true,
      batteryVolts: 12,
      current: 0.5,
      bulbVolts: 12,
      bulbPower: 6,
      brightness: 1,
      glow: 0.8,
      particleSpeed: 1.2,
      blown: false,
      solved: true,
      camera: { yaw: 25, pitch: 38, distance: 9.5 },
      time: 2.5,
    });
  });

  // Zero flags read as false and one as true.
  it("reads flags as booleans", () => {
    const r = readoutsFromArray([0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0]);
    assert.equal(r.closed, false);
    assert.equal(r.blown, true);
    assert.equal(r.solved, false);
  });

  // A short buffer is an error rather than undefined readouts.
  it("rejects a short buffer", () => {
    assert.throws(() => readoutsFromArray(new Float64Array(READOUT_COUNT - 1)), RangeError);
  });
});

describe("Pick", () => {
  // The pick codes match the Rust pick tags.
  it("matches the Rust pick tags", () => {
    assert.deepEqual({ ...Pick }, { NOTHING: 0, BATTERY: 1, SWITCH: 2, BULB: 3 });
    assert.ok(Object.isFrozen(Pick));
  });
});
