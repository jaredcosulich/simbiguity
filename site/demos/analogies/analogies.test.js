import { test } from "node:test";
import * as assert from "node:assert";
import { loadPhysics } from "../physics/physics.js";
import { loadCircuits } from "../../js/simbiguity.js";
import { getPhysicsParams, getCircuitParams } from "./mapping.js";
import * as fs from "node:fs/promises";
import * as path from "node:path";
import * as url from "node:url";

const __dirname = url.fileURLToPath(new URL(".", import.meta.url));

const physWasm = await fs.readFile(path.join(__dirname, "../../pkg/simbiguity_physics_wasm.wasm"));
const circWasm = await fs.readFile(path.join(__dirname, "../../pkg/simbiguity_circuits_wasm.wasm"));

const { Physics } = await loadPhysics(physWasm);
const { Circuit } = await loadCircuits(circWasm);

function simulateSteadyState(omega, dt = 1e-4, settleTime = 150, measureTime = 2) {
    const state = { m: 1, k: 100, c: 0.2, f0: 1, omega };
    const pp = getPhysicsParams(state);
    const cp = getCircuitParams(state);

    const physics = new Physics("driven_oscillator", 0, pp.m, pp.k, pp.c, pp.f0, pp.omega);
    physics.setIntegrator("verlet");

    const circuit = new Circuit();
    const n0 = circuit.node(), n1 = circuit.node(), n2 = circuit.node(), n3 = circuit.node();
    circuit.signal(n0, n1, 0, cp.v0, cp.omega / (2 * Math.PI), 0, 0);
    circuit.inductor(n1, n2, cp.l);
    circuit.resistor(n2, n3, cp.r);
    const cId = circuit.capacitor(n3, n0, cp.c);
    circuit.setIntegrator(1);

    const settleSteps = Math.floor(settleTime / dt);
    for (let i = 0; i < settleSteps; i++) {
        physics.step(dt);
        circuit.step(dt);
    }

    let minX = Infinity, maxX = -Infinity;
    let minQ = Infinity, maxQ = -Infinity;

    const measureSteps = Math.floor(measureTime / dt);
    for (let i = 0; i < measureSteps; i++) {
        physics.step(dt);
        const results = circuit.step(dt);

        const x = physics.getPositions()[3];
        if (x < minX) minX = x;
        if (x > maxX) maxX = x;

        const vC = results.parts[3 * 4];
        const q = cp.c * vC;
        if (q < minQ) minQ = q;
        if (q > maxQ) maxQ = q;
    }

    physics.free();
    circuit.free();

    return { xAmp: (maxX - minX) / 2, qAmp: (maxQ - minQ) / 2 };
}

test("rlc_and_mass_spring_resonate_identically", () => {
    const { xAmp, qAmp } = simulateSteadyState(10, 1e-4, 150, 2);
    assert.ok(Math.abs(xAmp - 0.500) < 0.005, `xAmp ${xAmp} not 0.500`);
    assert.ok(Math.abs(qAmp - 0.500) < 0.005, `qAmp ${qAmp} not 0.500`);
    assert.ok(Math.abs(xAmp - qAmp) < 0.005, `xAmp and qAmp differ`);
});

test("off_resonance_omega_20", () => {
    const { xAmp, qAmp } = simulateSteadyState(20, 1e-4, 150, 2);
    assert.ok(Math.abs(xAmp - 0.003333) < 0.0001, `xAmp ${xAmp} not 0.003333`);
    assert.ok(Math.abs(qAmp - 0.003333) < 0.0001, `qAmp ${qAmp} not 0.003333`);
});

test("sweep_peaks_at_10", () => {
    let peakX = -1, peakW = -1;
    for (let w = 9; w <= 11; w += 0.2) {
        const { xAmp } = simulateSteadyState(w, 1e-4, 80, 1);
        if (xAmp > peakX) {
            peakX = xAmp;
            peakW = w;
        }
    }
    assert.ok(Math.abs(peakW - 10) < 0.001, `Peak at ${peakW} not 10`);
});

test("mapping.js is exact and invertible", () => {
    const st = { m: 2, c: 0.5, k: 50, f0: 5, omega: 15 };
    const pp = getPhysicsParams(st);
    assert.equal(pp.m, 2);
    assert.equal(pp.k, 50);
    assert.equal(pp.c, 0.5);
    const cp = getCircuitParams(st);
    assert.equal(cp.l, 2);
    assert.equal(cp.r, 0.5);
    assert.equal(cp.c, 1/50);
});
