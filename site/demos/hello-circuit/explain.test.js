import { describe, it } from "node:test";
import assert from "node:assert/strict";

import { BULB_OHMS, explanation } from "./explain.js";

const closed = { closed: true, volts: 12 };
const quiet = { blown: false, shorted: false, brightness: 0, amps: null };

describe("explanation", () => {
  // A short wins over everything else and is marked as an issue.
  it("explains a short circuit first", () => {
    const e = explanation(closed, { ...quiet, shorted: true, blown: true }, [{ message: "m" }]);
    assert.equal(e.title, "Short circuit");
    assert.equal(e.issue, true);
  });

  // A blown bulb quotes the wattage it would have drawn.
  it("explains a blown bulb with the power it would draw", () => {
    const e = explanation({ closed: true, volts: 20 }, { ...quiet, blown: true }, []);
    assert.equal(e.title, "The bulb blew");
    assert.match(e.body, /At 20\.0 V the bulb would draw 16\.7 W/);
    assert.equal(e.issue, true);
  });

  // An open switch explains the gap in the loop.
  it("explains an open switch", () => {
    const e = explanation({ closed: false, volts: 12 }, quiet, []);
    assert.equal(e.title, "Open circuit");
    assert.equal(e.issue, false);
  });

  // Full brightness works through Ohm's law with the engine's current.
  it("explains a fully lit bulb with Ohm's law", () => {
    const e = explanation(closed, { ...quiet, brightness: 1, amps: 0.5 }, []);
    assert.equal(e.title, "Full brightness");
    assert.match(e.body, /I = V \/ R = 12\.0 \/ 24 = 0\.50 A/);
    assert.match(e.body, /P = V × I = 6\.00 W/);
    assert.match(e.body, /Push much further/);
  });

  // Partial brightness reports the percentage of the rating.
  it("titles partial brightness as lit or dim", () => {
    assert.equal(explanation(closed, { ...quiet, brightness: 0.5, amps: 0.35 }, []).title, "Lit");
    const dim = explanation({ closed: true, volts: 3 }, { ...quiet, brightness: 0.06, amps: 0.125 }, []);
    assert.equal(dim.title, "Dim");
    assert.match(dim.body, /That is 6 % of its 6 W rating/);
  });

  // A closed loop at 0 V carries no current.
  it("explains zero volts as no current", () => {
    const e = explanation({ closed: true, volts: 0 }, { ...quiet, amps: 0 }, []);
    assert.equal(e.title, "No current");
  });

  // Engine messages pass through verbatim, in order.
  it("passes engine messages through", () => {
    const e = explanation(closed, quiet, [{ message: "first" }, { message: "<b>second</b>" }]);
    assert.deepEqual(e.engine, ["first", "<b>second</b>"]);
  });

  // The filament resistance follows from the 12 V / 6 W rating.
  it("derives the filament resistance from the rating", () => {
    assert.equal(BULB_OHMS, 24);
  });
});
