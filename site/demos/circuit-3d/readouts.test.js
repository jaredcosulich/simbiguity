import { describe, it } from "node:test";
import assert from "node:assert/strict";

import { formatReadouts } from "./readouts.js";

const lit = {
  closed: true, blown: false, batteryVolts: 12, current: 0.5, bulbPower: 6, brightness: 1, particleSpeed: 1.2,
};

describe("formatReadouts", () => {
  // Each number gets its unit and precision.
  it("formats every reading with its unit", () => {
    assert.deepEqual(formatReadouts(lit), {
      volts: "12.0 V",
      current: "0.50 A",
      power: "6.00 W",
      bright: "100 %",
      speed: "1.20 u/s",
      toggleLabel: "Open the switch",
      showReplace: false,
    });
  });

  // An open switch offers to close it.
  it("offers to close an open switch", () => {
    assert.equal(formatReadouts({ ...lit, closed: false }).toggleLabel, "Close the switch");
  });

  // Brightness rounds to a whole percent.
  it("rounds brightness to a whole percent", () => {
    assert.equal(formatReadouts({ ...lit, brightness: 0.4449 }).bright, "44 %");
    assert.equal(formatReadouts({ ...lit, brightness: 0 }).bright, "0 %");
  });

  // Only a blown bulb shows the replace button.
  it("shows replace only for a blown bulb", () => {
    assert.equal(formatReadouts({ ...lit, blown: true }).showReplace, true);
  });
});
