import { parseState } from './state.js';
import { SimbiguityError } from '../../js/simbiguity.js';


const presetSelect = document.getElementById('preset');
const integratorSelect = document.getElementById('integrator');

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
  const str = (ptr, len) => utf8.decode(bytes(ptr, len));
  function check(status) {
    if (status < 0) {
      throw new Error(`Physics Error: ${str(w.sb_last_error_ptr(), w.sb_last_error_len())}`);
    }
    return status;
  }
  class Physics {
    constructor(preset = "pendulum") {
      if (preset === "spring_mass") this.handle = check(w.phys_new_spring_mass());
      else if (preset === "projectile") this.handle = check(w.phys_new_projectile());
      else if (preset === "chain") this.handle = check(w.phys_new_chain());
      else this.handle = check(w.phys_new_pendulum());
    }
    setIntegrator(method) { 
        let m = 0;
        if (method === "symplectic_euler") m = 1;
        else if (method === "explicit_euler") m = 2;
        check(w.phys_set_integrator(this.handle, m)); 
    }
    push(index, fx, fy, fz) { check(w.phys_push(this.handle, index, fx, fy, fz)); }
    step(dt) { check(w.phys_advance(this.handle, dt)); }
    getEnergy() {
      const raw = Array.from(f64s(w.phys_results_ptr(this.handle), 4));
      return { kinetic: raw[0], potential_gravity: raw[1], potential_spring: raw[2], total: raw[3] };
    }
    encode(width, height) { return check(w.phys_encode(this.handle, width, height)); }
    free() { check(w.phys_free(this.handle)); }
  }
  return { Physics, exports: w };
}

let physics = null;

async function main() {
    const { Physics } = await loadPhysics(new URL("../../pkg/simbiguity_physics_wasm.wasm", import.meta.url));

    function syncUI() {
        const state = parseState(window.location.hash);
        presetSelect.value = state.preset;
        integratorSelect.value = state.integrator;
        
        if (physics) {
            physics.free();
        }
        physics = new Physics(state.preset);
        physics.setIntegrator(state.integrator);
    }

    presetSelect.onchange = () => {
        const state = parseState(window.location.hash);
        state.preset = presetSelect.value;
        window.location.hash = `preset=${state.preset}&integrator=${state.integrator}`;
    };

    integratorSelect.onchange = () => {
        const state = parseState(window.location.hash);
        state.integrator = integratorSelect.value;
        window.location.hash = `preset=${state.preset}&integrator=${state.integrator}`;
    };

    window.addEventListener('hashchange', syncUI);
    syncUI();

    // animation loop
    function loop() {
        if (physics) {
            physics.step(1/60);
            const e = physics.getEnergy();
            // Just log for now if plot is hard to bind, but plan says "energy chart via simbiguity-plot"
            const energyDisplay = document.getElementById('energy') || document.body.appendChild(document.createElement('div'));
            energyDisplay.id = 'energy';
            energyDisplay.innerText = `KE: ${e.kinetic.toFixed(2)}, PE(g): ${e.potential_gravity.toFixed(2)}, PE(s): ${e.potential_spring.toFixed(2)}, Total: ${e.total.toFixed(2)}`;
        }
        requestAnimationFrame(loop);
    }
    loop();
}

main().catch(console.error);
