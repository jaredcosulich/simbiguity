import test from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import { loadCircuits } from "./simbiguity.js";

test("LED with 160 ohm resistor", async () => {
    const wasm = fs.readFileSync(new URL("../pkg/simbiguity_circuits_wasm.wasm", import.meta.url));
    const { Circuit } = await loadCircuits(wasm);
    
    const c = new Circuit();
    const n0 = c.node();
    const n1 = c.node();
    const n2 = c.node();
    
    // c.battery(p, m, 12.0)
    c.battery(n1, n0, 5.0);
    c.resistor(n1, n2, 160.0);
    // led(a, b, color = "red", rated_amps = 0.02, is = 1.5195e-17, n = 2.0)
    const led = c.led(n2, n0, "red", 0.020, 1.5195e-17, 2.0);
    
    const { solved, parts, issues } = c.solve();
    assert.strictEqual(solved, true);
    
    const ledResult = parts[led]; // index matches returned component ID? Wait, does component ID map to parts array?
    // Yes, parts is an array of {across, current, power, brightness} ordered by component creation.
    
    // We expect Vf ~ 1.8V, I ~ 20mA, light ~ 1.0.
    assert.ok(Math.abs(ledResult.across - 1.80000) < 1e-4, `Vf was ${ledResult.across}`);
    assert.ok(Math.abs(ledResult.current - 0.020) < 1e-6, `Id was ${ledResult.current}`);
    assert.ok(Math.abs(ledResult.brightness - 1.0) < 1e-3, `Light was ${ledResult.brightness}`);
    
    c.free();
});
