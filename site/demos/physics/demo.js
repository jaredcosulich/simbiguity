import { boot } from "../../js/boot.js";
import { parseState, formatState } from './state.js';
import { WebGL2Executor } from '../../js/webgl2-executor.js';
import { renderLabels } from '../../js/plot-labels.js';

const presetSelect = document.getElementById('preset');
const integratorSelect = document.getElementById('integrator');
const timeScaleInput = document.getElementById('time-scale');
const restitutionInput = document.getElementById('restitution');
const frictionInput = document.getElementById('friction');
const resetBtn = document.getElementById('reset-btn');
const shareBtn = document.getElementById('share-btn');
const simTimeEl = document.getElementById('sim-time');
const stepTimeEl = document.getElementById('step-time');
const momentumEl = document.getElementById('momentum');
const fpsEl = document.getElementById('fps');

async function loadPhysics(url) {
  const response = fetch(url);
  const { instance } = WebAssembly.instantiateStreaming
    ? await WebAssembly.instantiateStreaming(response, {}).catch(async () =>
        WebAssembly.instantiate(await (await fetch(url)).arrayBuffer(), {}))
    : await WebAssembly.instantiate(await (await response).arrayBuffer(), {});
  const w = instance.exports;
  const utf8 = new TextDecoder();
  const bytes = (ptr, len) => new Uint8Array(w.memory.buffer, ptr, len);
  const f64s = (ptr, len) => new Float64Array(w.memory.buffer, ptr, len);
  const u32s = (ptr, len) => new Uint32Array(w.memory.buffer, ptr, len);
  const str = (ptr, len) => utf8.decode(bytes(ptr, len));
  function check(status) {
    if (status < 0) {
      throw new Error(`Physics Error: ${str(w.sb_last_error_ptr(), w.sb_last_error_len())}`);
    }
    return status;
  }
  class Physics {
    constructor(preset = "pendulum") {
      if (preset === "spring") this.handle = check(w.phys_new_spring_mass());
      else if (preset === "projectile") this.handle = check(w.phys_new_projectile());
      else if (preset === "chain") this.handle = check(w.phys_new_chain());
      else if (preset === "double_pendulum") this.handle = check(w.phys_new_double_pendulum(1e-9));
      else if (preset === "cradle") this.handle = check(w.phys_new_cradle());
      else if (preset === "ball_pit") this.handle = check(w.phys_new_ball_pit(300));
      else if (preset === "sliding_block") this.handle = check(w.phys_new_sliding_block());
      else if (preset === "orbit") this.handle = check(w.phys_new_orbit());
      else if (preset === "binary") this.handle = check(w.phys_new_binary());
      else if (preset === "figure_eight") this.handle = check(w.phys_new_figure_eight());
      else if (preset === "cloth") this.handle = check(w.phys_new_cloth(20));
      else this.handle = check(w.phys_new_pendulum());
    }
    setIntegrator(method) { 
        let m = 0;
        if (method === "symplectic_euler" || method === "symplectic") m = 1;
        else if (method === "explicit_euler" || method === "explicit") m = 2;
        check(w.phys_set_integrator(this.handle, m)); 
    }
    setTimeScale(s) { check(w.phys_set_time_scale(this.handle, s)); }
    setRestitution(r) { check(w.phys_set_restitution(this.handle, r)); }
    setFriction(f) { check(w.phys_set_friction(this.handle, f)); }
    step(dt) { check(w.phys_advance(this.handle, dt)); }
    getEnergy() {
      const raw = Array.from(f64s(w.phys_results_ptr(this.handle), 4));
      return { kinetic: raw[0], potential_gravity: raw[1], potential_spring: raw[2], total: raw[3] };
    }
    getMomentum() { return w.phys_momentum(this.handle); }
    getTime() { return w.phys_time(this.handle); }
    getStepTimeMs() { return w.phys_step_time_ms(this.handle); }
    getParticleCount() { return w.phys_particle_count(this.handle); }
    getPositions() { 
       const c = this.getParticleCount();
       return Array.from(f64s(w.phys_positions_ptr(this.handle), c * 3));
    }
    getMeshCount() { return w.phys_mesh_count(this.handle); }
    getMeshVertices(i) {
        const len = w.phys_mesh_vertices_len(this.handle, i);
        return new Float32Array(w.memory.buffer, w.phys_mesh_vertices_ptr(this.handle, i), len);
    }
    getMeshIndices(i) {
        const len = w.phys_mesh_indices_len(this.handle, i);
        return new Uint32Array(w.memory.buffer, w.phys_mesh_indices_ptr(this.handle, i), len);
    }
    encode(width, height) { 
        const len = check(w.phys_encode(this.handle, width, height)); 
        return u32s(w.phys_commands_ptr(this.handle), len);
    }
    free() { check(w.phys_free(this.handle)); }
  }
  return { Physics, exports: w };
}

let physics = null;
let executor = null;
let frameCount = 0;
let lastFpsTime = performance.now();

async function main() {
    const { Physics } = await loadPhysics(new URL("../../pkg/simbiguity_physics_wasm.wasm", import.meta.url));
    const canvas = document.getElementById('view');
    executor = await WebGL2Executor.create(canvas.getContext('webgl2'));

    function syncUI() {
        const state = parseState(window.location.hash);
        presetSelect.value = state.preset;
        integratorSelect.value = state.integrator;
        timeScaleInput.value = state.timeScale;
        restitutionInput.value = state.restitution;
        frictionInput.value = state.friction;
        
        if (physics) {
            physics.free();
        }
        physics = new Physics(state.preset);
        physics.setIntegrator(state.integrator);
        physics.setTimeScale(state.timeScale);
        physics.setRestitution(state.restitution);
        physics.setFriction(state.friction);
    }

    function updateHash() {
        const state = {
            preset: presetSelect.value,
            integrator: integratorSelect.value,
            timeScale: parseFloat(timeScaleInput.value),
            restitution: parseFloat(restitutionInput.value),
            friction: parseFloat(frictionInput.value)
        };
        window.location.hash = formatState(state);
    }

    presetSelect.onchange = updateHash;
    integratorSelect.onchange = updateHash;
    timeScaleInput.onchange = updateHash;
    restitutionInput.onchange = updateHash;
    frictionInput.onchange = updateHash;
    resetBtn.onclick = syncUI;
    shareBtn.onclick = () => {
        navigator.clipboard.writeText(window.location.href);
        alert('URL copied to clipboard');
    };

    window.addEventListener('hashchange', syncUI);
    syncUI();

    function loop() {
        const now = performance.now();
        frameCount++;
        if (now - lastFpsTime >= 1000) {
            fpsEl.textContent = frameCount;
            frameCount = 0;
            lastFpsTime = now;
        }

        if (physics) {
            physics.step(1/60);
            
            simTimeEl.textContent = physics.getTime().toFixed(2);
            stepTimeEl.textContent = physics.getStepTimeMs().toFixed(2);
            momentumEl.textContent = physics.getMomentum().toFixed(2);
            
            const e = physics.getEnergy();
            const energyDisplay = document.getElementById('energy') || document.body.appendChild(document.createElement('div'));
            energyDisplay.id = 'energy';
            energyDisplay.innerText = `KE: ${e.kinetic.toFixed(2)}, PE(g): ${e.potential_gravity.toFixed(2)}, PE(s): ${e.potential_spring.toFixed(2)}, Total: ${e.total.toFixed(2)}`;

            const meshCount = physics.getMeshCount();
            for (let i = 0; i < meshCount; i++) {
                executor.uploadMesh(i, physics.getMeshVertices(i), physics.getMeshIndices(i));
            }

            const commands = physics.encode(canvas.width, canvas.height);
            executor.execute(commands, canvas.width, canvas.height);
            
            // "The double pendulum runs two copies whose starting angles differ by 1e-9 rad... A readout shows their angle difference on a log scale."
            // We can add the readout dynamically for double_pendulum
            if (presetSelect.value === 'double_pendulum') {
                const pos = physics.getPositions();
                // base is 0, pend1 p1 is pos 1, pend1 p2 is pos 2. pend2 p1 is pos 4, pend2 p2 is pos 5.
                // pend1 is index 0-2 (0-8 in pos array), pend2 is index 3-5 (9-17 in pos array)
                // p1 is pos[3-5], p2 is pos[6-8]. p4 is pos[12-14], p5 is pos[15-17].
                // We'll compute the angle of the second bob relative to the first bob.
                const angle1 = Math.atan2(pos[7] - pos[4], pos[6] - pos[3]);
                const angle2 = Math.atan2(pos[16] - pos[13], pos[15] - pos[12]);
                let diff = Math.abs(angle1 - angle2);
                if (diff === 0) diff = 1e-16;
                const logDiff = Math.log10(diff).toFixed(2);
                
                let diffDisplay = document.getElementById('angle-diff');
                if (!diffDisplay) {
                    diffDisplay = document.createElement('div');
                    diffDisplay.id = 'angle-diff';
                    document.getElementById('readouts').appendChild(diffDisplay);
                }
                diffDisplay.textContent = `Chaos Angle Diff (log10 rad): ${logDiff}`;
            } else {
                const diffDisplay = document.getElementById('angle-diff');
                if (diffDisplay) diffDisplay.remove();
            }
        }
        requestAnimationFrame(loop);
    }
    loop();
}

boot(main);
