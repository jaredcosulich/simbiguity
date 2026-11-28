import { test, describe } from 'node:test';
import assert from 'node:assert';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadPhysics } from './physics.js';
import { parseState } from './state.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const wasmPath = path.join(__dirname, '../../../site/pkg/simbiguity_physics_wasm.wasm');
const wasmBytes = fs.readFileSync(wasmPath);

async function getPhysics() {
    return await loadPhysics(wasmBytes);
}

describe('Physics WASM behaviour', () => {
    test('orbit_explicit_euler_gains_energy_verlet_does_not', async () => {
        const { Physics } = await getPhysics();
        const dt = 1e-3;
        const steps = Math.floor(10 * 2 * Math.PI / dt);
        
        const simE = new Physics("orbit");
        simE.setIntegrator("explicit");
        const e0E = simE.getEnergy().total;
        for (let i = 0; i < steps; i++) simE.step(dt);
        const e1E = simE.getEnergy().total;
        assert(e1E > e0E * 1.01, `Expected Euler energy to rise by >1%. Initial: ${e0E}, Final: ${e1E}`);
        simE.free();

        const simV = new Physics("orbit");
        simV.setIntegrator("verlet");
        const e0V = simV.getEnergy().total;
        for (let i = 0; i < steps; i++) simV.step(dt);
        const e1V = simV.getEnergy().total;
        const drift = Math.abs((e1V - e0V) / e0V);
        assert(drift < 1e-6, `Expected Verlet relative drift < 1e-6, got ${drift}`);
        simV.free();
    });

    test.skip('Double pendulum, offset 0 vs 1e-9', async () => {
        const { Physics } = await getPhysics();
        const dt = 1/240;
        const sim0 = new Physics("double_pendulum", 0.0);
        const sim1 = new Physics("double_pendulum", 1e-9);
        
        let diff_1s = 0;
        let diff_20s = 0;
        
        for (let i = 0; i < 240 * 20; i++) {
            sim0.step(dt);
            sim1.step(dt);
            if (i === 240 - 1) { // 1 second
                const pos0 = sim0.getPositions();
                const pos1 = sim1.getPositions();
                const dx0 = pos0[6] - pos0[3];
                const dy0 = pos0[7] - pos0[4];
                const dx1 = pos1[6] - pos1[3];
                const dy1 = pos1[7] - pos1[4];
                const a0 = Math.atan2(dy0, dx0);
                const a1 = Math.atan2(dy1, dx1);
                diff_1s = Math.abs(a1 - a0);
            }
        }
        const pos0 = sim0.getPositions();
        const pos1 = sim1.getPositions();
        const dx0 = pos0[6] - pos0[3];
        const dy0 = pos0[7] - pos0[4];
        const dx1 = pos1[6] - pos1[3];
        const dy1 = pos1[7] - pos1[4];
        const a0 = Math.atan2(dy0, dx0);
        const a1 = Math.atan2(dy1, dx1);
        diff_20s = Math.abs(a1 - a0);
        
        assert(diff_1s < 1e-6, `Expected < 1e-6 at 1s, got ${diff_1s}`);
        assert(diff_20s > 0.1, `Expected > 0.1 at 20s, got ${diff_20s}`);
        sim0.free();
        sim1.free();
    });

    test('Cradle', async () => {
        const { Physics } = await getPhysics();
        const sim = new Physics("cradle");
        const dt = 1e-3;
        // Wait for first impact. Ball 0 starts at x = -1, v = 1. Impact happens near x = 0.
        // It takes around 1 second.
        let last_speed = 0;
        for (let i = 0; i < 1500; i++) {
            const pos = sim.getPositions(); // last ball is index 4 (12, 13, 14)
            sim.step(dt);
            const pos2 = sim.getPositions();
            const vx = (pos2[12] - pos[12]) / dt;
            if (Math.abs(vx) > 0.5) {
                last_speed = Math.abs(vx);
                break;
            }
        }
        assert(Math.abs(last_speed - 1.0) < 1e-3, `Expected speed 1.0, got ${last_speed}`);
        sim.free();
    });

    test('Cloth 20x20', async () => {
        const { Physics } = await getPhysics();
        const sim = new Physics("cloth");
        const p0 = sim.getPositions();
        for (let i = 0; i < 500; i++) sim.step(0.01);
        const p1 = sim.getPositions();
        // Corners are 0 and 19. They should not move at all.
        assert.strictEqual(p0[0], p1[0]);
        assert.strictEqual(p0[1], p1[1]);
        assert.strictEqual(p0[2], p1[2]);
        assert.strictEqual(p0[19*3], p1[19*3]);
        assert.strictEqual(p0[19*3+1], p1[19*3+1]);
        assert.strictEqual(p0[19*3+2], p1[19*3+2]);
        sim.free();
    });

    test('Replay', async () => {
        const { Physics } = await getPhysics();
        const simA = new Physics("cradle");
        const simB = new Physics("cradle");
        for (let i = 0; i < 1000; i++) {
            simA.step(0.01);
            simB.step(0.01);
        }
        const pA = simA.getPositions();
        const pB = simB.getPositions();
        assert.strictEqual(pA.length, pB.length);
        for (let i = 0; i < pA.length; i++) {
            assert.ok(Object.is(pA[i], pB[i]));
        }
        simA.free();
        simB.free();
    });

    test('Every preset', async () => {
        const { Physics } = await getPhysics();
        const presets = ["pendulum", "spring", "projectile", "chain", "double_pendulum", "cradle", "ball_pit", "sliding_block", "orbit", "binary", "figure_eight", "cloth"];
        for (const p of presets) {
            const sim = new Physics(p);
            for (let i = 0; i < 100; i++) sim.step(0.016);
            sim.free();
        }
    });
});
