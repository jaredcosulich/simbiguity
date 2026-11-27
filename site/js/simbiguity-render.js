// simbiguity-render.js — the JavaScript side of the 3D view bridge
// (crates/simbiguity-render-wasm). Same rules as simbiguity.js: no bundler,
// no generated glue, every export is a plain function over numbers.
//
//   import { loadCircuit3D } from "./simbiguity-render.js";
//   const { Circuit3D } = await loadCircuit3D("./pkg/simbiguity_render_wasm.wasm");
//   const view = new Circuit3D({ volts: 12, closed: true });
//   for (const m of view.meshes()) executor.uploadMesh(m.id, m.vertices, m.indices);
//   executor.execute(view.frame(width, height));

import { SimbiguityError } from "./simbiguity.js";

/** What `pick` can return. */
export const Pick = Object.freeze({ NOTHING: 0, BATTERY: 1, SWITCH: 2, BULB: 3 });

/** `f64`s in the readouts buffer (crates/simbiguity-render-wasm/src/readouts.rs). */
export const READOUT_COUNT = 14;

/**
 * Names the slots of the readouts buffer `c3d_readouts` fills. Pure.
 * @param {ArrayLike<number>} r at least `READOUT_COUNT` numbers
 */
export function readoutsFromArray(r) {
  if (r.length < READOUT_COUNT) {
    throw new RangeError(`readouts need ${READOUT_COUNT} numbers, got ${r.length}`);
  }
  return {
    closed: r[0] === 1,
    batteryVolts: r[1],
    current: r[2],
    bulbVolts: r[3],
    bulbPower: r[4],
    brightness: r[5],
    glow: r[6],
    particleSpeed: r[7],
    blown: r[8] === 1,
    solved: r[9] === 1,
    camera: { yaw: r[10], pitch: r[11], distance: r[12] },
    time: r[13],
  };
}

/**
 * Fetches and instantiates the 3D view bridge.
 * @param {string | URL} url where simbiguity_render_wasm.wasm is served
 */
export async function loadCircuit3D(url) {
  const response = fetch(url);
  const { instance } = WebAssembly.instantiateStreaming
    ? await WebAssembly.instantiateStreaming(response, {}).catch(async () =>
        WebAssembly.instantiate(await (await fetch(url)).arrayBuffer(), {}))
    : await WebAssembly.instantiate(await (await response).arrayBuffer(), {});
  const w = instance.exports;
  const utf8 = new TextDecoder();

  // Views of memory.buffer go stale when memory grows: build them fresh.
  function check(status) {
    if (status < 0) {
      const msg = utf8.decode(new Uint8Array(w.memory.buffer, w.sb_last_error_ptr(), w.sb_last_error_len()));
      throw new SimbiguityError(status, msg);
    }
    return status;
  }

  /** The 3D hello circuit living inside WASM. */
  class Circuit3D {
    constructor({ volts = 12, closed = false, handle = null } = {}) {
      this.handle = handle !== null ? handle : w.c3d_new(volts, closed ? 1 : 0);
    }

    /** Every mesh, copied out of linear memory, for upload. */
    meshes() {
      const h = this.handle;
      const out = [];
      const count = check(w.c3d_mesh_count(h));
      for (let i = 0; i < count; i++) {
        const vlen = check(w.c3d_mesh_vertices_len(h, i));
        const ilen = check(w.c3d_mesh_indices_len(h, i));
        out.push({
          id: i,
          vertices: new Float32Array(w.memory.buffer, w.c3d_mesh_vertices_ptr(h, i), vlen).slice(),
          indices: new Uint32Array(w.memory.buffer, w.c3d_mesh_indices_ptr(h, i), ilen).slice(),
        });
      }
      return out;
    }

    toggle() { return check(w.c3d_toggle(this.handle)) === 1; }
    setVolts(v) { check(w.c3d_set_volts(this.handle, v)); }
    replaceBulb() { check(w.c3d_replace_bulb(this.handle)); }
    /** Advances by wall-clock seconds (fixed internal steps). */
    advance(dt) { check(w.c3d_advance(this.handle, dt)); }
    /** Restarts the animation and runs it to exactly `t` seconds. */
    seek(t) { check(w.c3d_seek(this.handle, t)); }
    setCamera(yawDeg, pitchDeg, distance) { check(w.c3d_set_camera(this.handle, yawDeg, pitchDeg, distance)); }
    orbit(dYawDeg, dPitchDeg) { check(w.c3d_orbit(this.handle, dYawDeg, dPitchDeg)); }
    zoom(factor) { check(w.c3d_zoom(this.handle, factor)); }

    /** One frame's draw commands (a copy, safe to keep). */
    frame(width, height) {
      const len = check(w.c3d_encode(this.handle, width, height));
      return new Uint32Array(w.memory.buffer, w.c3d_commands_ptr(this.handle), len).slice();
    }

    /** What is under canvas pixel (x, y): one of `Pick`. */
    pick(x, y, width, height) { return check(w.c3d_pick(this.handle, x, y, width, height)); }

    /** The engine's numbers and the animation/camera state. */
    readouts() {
      const len = check(w.c3d_readouts(this.handle));
      return readoutsFromArray(new Float64Array(w.memory.buffer, w.c3d_readouts_ptr(this.handle), len));
    }

    free() { check(w.c3d_free(this.handle)); }
  }

  return { Circuit3D, exports: w };
}
