import { describe, it } from "node:test";
import assert from "node:assert/strict";

import { explanation } from "./explain.js";

const flowing = { blown: false, closed: true, brightness: 1, batteryVolts: 12, current: 0.5 };

describe("explanation", () => {
  // A blown bulb wins over everything and is an issue that quotes the voltage.
  it("explains a blown bulb first", () => {
    const e = explanation({ ...flowing, blown: true, closed: false, batteryVolts: 24, current: 0 });
    assert.equal(e.title, "The bulb blew");
    assert.match(e.body, /At 24\.0 V/);
    assert.equal(e.issue, true);
  });

  // An open switch explains the gap.
  it("explains an open switch as a gap that stops the charges", () => {
    const e = explanation({ ...flowing, closed: false, brightness: 0, current: 0 });
    assert.equal(e.title, "The switch is open");
    assert.equal(e.issue, false);
  });

  // Full brightness above the rated voltage is overdriven.
  it("explains an overdriven bulb", () => {
    const e = explanation({ ...flowing, batteryVolts: 16, current: 0.67 });
    assert.equal(e.title, "Overdriven");
    assert.match(e.body, /^0\.67 A/);
  });

  // Full brightness at exactly the rated voltage is still just flowing.
  it("does not call the rated voltage overdriven", () => {
    assert.equal(explanation(flowing).title, "Current is flowing");
  });

  // Normal flow quotes the current.
  it("explains current flowing with the amps", () => {
    const e = explanation({ ...flowing, brightness: 0.4, batteryVolts: 8, current: 0.33 });
    assert.equal(e.title, "Current is flowing");
    assert.match(e.body, /pushes 0\.33 A/);
    assert.equal(e.issue, false);
  });
});
