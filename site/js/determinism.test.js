import test from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import { loadCircuits } from "./simbiguity.js";

function hashF64s(vals) {
    let hash = 0xcbf29ce484222325n;
    const buf = new ArrayBuffer(8);
    const f64 = new Float64Array(buf);
    const u8 = new Uint8Array(buf);

    for (const v of vals) {
        f64[0] = v;
        for (let i = 0; i < 8; i++) {
            hash ^= BigInt(u8[i]);
            hash = BigInt.asUintN(64, hash * 0x100000001b3n);
        }
    }
    return hash.toString();
}

function buildHelloFixture(c, bulbWatts) {
    const p = c.node();
    const m = c.node();
    const t = c.node();
    
    c.battery(p, m, 12.0);
    c.switch(p, t, true);
    c.bulb(t, m, 12.0, bulbWatts);
}

function solveAndHash(w, h) {
    w.circuit_solve(h);
    const len = w.circuit_results_len(h);
    const raw = Array.from(new Float64Array(w.memory.buffer, w.circuit_results_ptr(h), len));
    return hashF64s(raw);
}

test("WASM determinism hash", async () => {
    const json = JSON.parse(fs.readFileSync(new URL("../../crates/simbiguity-circuits-wasm/golden-hashes.json", import.meta.url), "utf8"));
    const wasmBytes = fs.readFileSync(new URL("../../site/pkg/simbiguity_circuits_wasm.wasm", import.meta.url));
    const { Circuit, exports: w } = await loadCircuits(wasmBytes);

    // hello fixture
    const c1 = new Circuit();
    buildHelloFixture(c1, 6.0);
    let hash1 = solveAndHash(w, c1.handle);
    assert.strictEqual(hash1, json.hello, "hello fixture hash mismatch");

    // hello fixture modified
    const c2 = new Circuit();
    buildHelloFixture(c2, 6.0 + 1e-12);
    let hash2 = solveAndHash(w, c2.handle);
    assert.notStrictEqual(hash2, json.hello, "hello modified fixture should not match");
    
    c1.free();
    c2.free();
});
