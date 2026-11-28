import test from "node:test";
import assert from "node:assert";
import fs from "node:fs";

test("WASM determinism hash", async () => {
    const json = JSON.parse(fs.readFileSync(new URL("../../crates/simbiguity-circuits-wasm/golden-hashes.json", import.meta.url), "utf8"));
    
    // We will test the hash logic once the TDD phase starts and we can actually load the WASM and check the outputs.
    assert.strictEqual(typeof json.hello, "string");
});
