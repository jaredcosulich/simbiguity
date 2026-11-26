import { describe, it } from "node:test";
import assert from "node:assert/strict";

import { BULB_VOLTS, hashFromState, stateFromHash } from "./state.js";

describe("stateFromHash", () => {
  // An empty hash is the starting state: switch open, no bypass, rated voltage.
  it("reads an empty hash as the default state", () => {
    assert.deepEqual(stateFromHash(""), { closed: false, short: false, volts: BULB_VOLTS });
    assert.deepEqual(stateFromHash("#"), { closed: false, short: false, volts: BULB_VOLTS });
  });

  // Each token switches on its own flag, in any order.
  it("reads closed, short and v tokens in any order", () => {
    assert.deepEqual(stateFromHash("#v=20&short&closed"), { closed: true, short: true, volts: 20 });
  });

  // The leading # is optional, so location.hash and a bare string both work.
  it("accepts a hash without the leading hash sign", () => {
    assert.deepEqual(stateFromHash("closed&v=6.5"), { closed: true, short: false, volts: 6.5 });
  });

  // Voltages outside the slider range are clamped, not rejected.
  it("clamps volts to the slider range", () => {
    assert.equal(stateFromHash("#v=99").volts, 24);
    assert.equal(stateFromHash("#v=-3").volts, 0);
  });

  // A garbled or empty v= falls back to the rated voltage instead of NaN or 0.
  it("falls back to rated volts for an unreadable v token", () => {
    assert.equal(stateFromHash("#v=abc").volts, BULB_VOLTS);
    assert.equal(stateFromHash("#v=").volts, BULB_VOLTS);
  });

  // Tokens the demo does not know are ignored.
  it("ignores unknown tokens", () => {
    assert.deepEqual(stateFromHash("#open&foo=1"), { closed: false, short: false, volts: BULB_VOLTS });
  });
});

describe("hashFromState", () => {
  // The default state needs no hash at all, so the plain URL stays clean.
  it("returns an empty string for the default state", () => {
    assert.equal(hashFromState({ closed: false, short: false, volts: BULB_VOLTS }), "");
  });

  // Tokens come out in a fixed order: closed, short, then v.
  it("writes tokens in a fixed order", () => {
    assert.equal(hashFromState({ closed: true, short: true, volts: 20 }), "#closed&short&v=20");
  });

  // The rated voltage is the default, so it is left out of the hash.
  it("omits v at the rated voltage", () => {
    assert.equal(hashFromState({ closed: true, short: false, volts: BULB_VOLTS }), "#closed");
  });

  // Every hash it writes reads back as the same state.
  it("round-trips through stateFromHash", () => {
    for (const s of [
      { closed: false, short: false, volts: 0 },
      { closed: true, short: false, volts: 17.5 },
      { closed: true, short: true, volts: BULB_VOLTS },
      { closed: false, short: true, volts: 24 },
    ]) {
      assert.deepEqual(stateFromHash(hashFromState(s)), s);
    }
  });
});
