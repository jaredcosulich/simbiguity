import { describe, it } from "node:test";
import assert from "node:assert/strict";

import { formatReading, readouts } from "./readouts.js";

const ids = { battery: 0, mainSwitch: 1, bulb: 2, bypass: 3 };
const none = { across: null, current: null, power: null, brightness: null };
const closed = { closed: true, short: false, volts: 12 };

/** The four parts as the engine returns them for a lit 12 V loop. */
function litParts() {
  return [
    { across: 12, current: -0.5, power: -6, brightness: null },
    { across: 0, current: 0.5, power: 0, brightness: null },
    { across: 12, current: 0.5, power: 6, brightness: 1 },
    { across: 0, current: 0, power: 0, brightness: null },
  ];
}

describe("formatReading", () => {
  // A number gets its unit and two decimals by default.
  it("formats a number with its unit", () => {
    assert.equal(formatReading(0.5, "A"), "0.50 A");
  });

  // The digits argument controls the decimals.
  it("honours the digits argument", () => {
    assert.equal(formatReading(12, "V", 1), "12.0 V");
  });

  // No value is an em dash, never "null A" or "NaN A".
  it("shows an em dash for null or undefined", () => {
    assert.equal(formatReading(null, "W"), "—");
    assert.equal(formatReading(undefined, "W"), "—");
  });

  // Zero is a real reading and must not read as missing.
  it("keeps zero as a reading", () => {
    assert.equal(formatReading(0, "A"), "0.00 A");
  });
});

describe("readouts", () => {
  // A lit loop shows the engine's numbers and marks the wires live.
  it("reads a lit loop", () => {
    const r = readouts(closed, litParts(), [], ids);
    assert.equal(r.volts, "12.0 V");
    assert.equal(r.current, "0.50 A");
    assert.equal(r.power, "6.00 W");
    assert.equal(r.bright, "100 %");
    assert.equal(r.flowing, true);
    assert.equal(r.blown, false);
    assert.equal(r.toggleLabel, "Open the switch");
  });

  // An open switch: zero current, dark bulb, wires not live.
  it("reads an open loop as dark and not flowing", () => {
    const parts = litParts().map((p) => ({ ...p, current: 0, power: 0 }));
    parts[2].brightness = 0;
    const r = readouts({ ...closed, closed: false }, parts, [], ids);
    assert.equal(r.flowing, false);
    assert.equal(r.bright, "0 %");
    assert.equal(r.toggleLabel, "Close the switch");
  });

  // No solution, as with a short: every readout is a dash.
  it("shows dashes when there is no solution", () => {
    const issues = [{ kind: "short-circuit", part: 0, message: "short" }];
    const r = readouts({ ...closed, short: true }, null, issues, ids);
    assert.equal(r.shorted, true);
    assert.equal(r.volts, "—");
    assert.equal(r.current, "—");
    assert.equal(r.power, "—");
    assert.equal(r.bright, "—");
    assert.equal(r.brightness, 0);
    assert.equal(r.amps, null);
    assert.equal(r.flowing, false);
  });

  // Only a blown-bulb issue naming this bulb counts as blown.
  it("flags blown only for this bulb", () => {
    const parts = [litParts()[0], none, none, none];
    assert.equal(readouts(closed, parts, [{ kind: "bulb-blown", part: 2, message: "" }], ids).blown, true);
    assert.equal(readouts(closed, parts, [{ kind: "bulb-blown", part: 7, message: "" }], ids).blown, false);
  });

  // The battery label tracks the slider, not the engine.
  it("labels the battery from the state voltage", () => {
    assert.equal(readouts({ ...closed, volts: 17.5 }, null, [], ids).batteryLabel, "17.5 V");
  });
});
