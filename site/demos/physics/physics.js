export async function loadPhysics(source) {
  let instance;
  if (source instanceof Uint8Array || source instanceof ArrayBuffer) {
    const res = await WebAssembly.instantiate(source, {});
    instance = res.instance;
  } else {
    const response = typeof source === "string" || source instanceof URL ? fetch(source) : source;
    if (WebAssembly.instantiateStreaming) {
      try {
        const res = await WebAssembly.instantiateStreaming(response, {});
        instance = res.instance;
      } catch (e) {
        const r = await Promise.resolve(response);
        const res = await WebAssembly.instantiate(await r.arrayBuffer(), {});
        instance = res.instance;
      }
    } else {
      const r = await Promise.resolve(response);
      const res = await WebAssembly.instantiate(await r.arrayBuffer(), {});
      instance = res.instance;
    }
  }
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
    constructor(preset = "pendulum", offset = 1e-9) {
      if (preset === "spring") this.handle = check(w.phys_new_spring_mass());
      else if (preset === "projectile") this.handle = check(w.phys_new_projectile());
      else if (preset === "chain") this.handle = check(w.phys_new_chain());
      else if (preset === "double_pendulum") this.handle = check(w.phys_new_double_pendulum(offset));
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
