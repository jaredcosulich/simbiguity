import test from "node:test";
import assert from "node:assert";
import { stateFromHash, hashFromState } from "./state.js";

test("stateFromHash parses empty string to defaults", () => {
    assert.deepStrictEqual(stateFromHash(""), { preset: "rc", integrator: 0, closed: true });
});

test("stateFromHash parses #lc and #trap", () => {
    assert.deepStrictEqual(stateFromHash("#lc"), { preset: "lc", integrator: 0, closed: true });
    assert.deepStrictEqual(stateFromHash("#trap"), { preset: "rc", integrator: 1, closed: true });
    assert.deepStrictEqual(stateFromHash("#lc&trap"), { preset: "lc", integrator: 1, closed: true });
});

test("hashFromState produces expected hash", () => {
    assert.strictEqual(hashFromState({ preset: "rc", integrator: 0 }), "");
    assert.strictEqual(hashFromState({ preset: "lc", integrator: 0 }), "#lc");
    assert.strictEqual(hashFromState({ preset: "rc", integrator: 1 }), "#trap");
    assert.strictEqual(hashFromState({ preset: "lc", integrator: 1 }), "#lc&trap");
});

