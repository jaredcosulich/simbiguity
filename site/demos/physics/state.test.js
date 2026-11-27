import test from "node:test";
import assert from "node:assert";
import { parseState } from "./state.js";

test("parseState defaults", () => {
    const s = parseState("");
    assert.strictEqual(s.preset, "pendulum");
    assert.strictEqual(s.integrator, "verlet");
});

test("parseState with values", () => {
    const s = parseState("#preset=chain&integrator=explicit_euler");
    assert.strictEqual(s.preset, "chain");
    assert.strictEqual(s.integrator, "explicit_euler");
});
