// simbiguity.js — the whole JavaScript side of the simbiguity WASM bridge.
//
// No bundler, no npm, no generated glue. Copy this file next to the .wasm
// and import it:
//
//   import { loadCircuits } from "./simbiguity.js";
//   const { Circuit } = await loadCircuits("./pkg/simbiguity_circuits_wasm.wasm");
//   const c = new Circuit();
//
// The Rust side (crates/simbiguity-circuits-wasm) exports plain functions
// over numbers. This file turns them into a small class.

/** Numbers per component in the results buffer: across, current, power, brightness. */
const RESULT_STRIDE = 4;

/** Issue kinds, as numbered by `circuit_issue_kind`. */
export const IssueKind = Object.freeze({
  1: "invalid-value",
  2: "short-circuit",
  3: "battery-loop",
  4: "floating-node",
  5: "open-circuit",
  6: "bulb-blown",
  7: "solver-failure",
  8: "inrush",
  9: "invalid-time-step",
});

/** Thrown when a bridge call returns a negative status code. */
export class SimbiguityError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "SimbiguityError";
    this.code = code;
  }
}

/**
 * Fetches and instantiates the circuits bridge.
 * @param {string | URL} url where simbiguity_circuits_wasm.wasm is served
 * @returns {Promise<{ Circuit: typeof Circuit, exports: WebAssembly.Exports }>}
 */
export async function loadCircuits(source) {
  // The module imports nothing: the engine needs no host functions.
  const isUrl = typeof source === "string" || source instanceof URL;
  
  let instance;
  if (isUrl) {
    const response = fetch(source);
    if (WebAssembly.instantiateStreaming) {
      instance = (await WebAssembly.instantiateStreaming(response, {}).catch(async () =>
        // Some servers send .wasm without `application/wasm`; fall back.
        WebAssembly.instantiate(await (await fetch(source)).arrayBuffer(), {})
      )).instance;
    } else {
      instance = (await WebAssembly.instantiate(await (await response).arrayBuffer(), {})).instance;
    }
  } else if (source instanceof WebAssembly.Module) {
    instance = await WebAssembly.instantiate(source, {});
  } else {
    // ArrayBuffer, Uint8Array
    instance = (await WebAssembly.instantiate(source, {})).instance;
  }
  const w = instance.exports;
  const utf8 = new TextDecoder();

  // Linear memory can grow during any call that allocates, which detaches
  // old views of `memory.buffer`. So build views fresh, right before reading.
  const bytes = (ptr, len) => new Uint8Array(w.memory.buffer, ptr, len);
  const f64s = (ptr, len) => new Float64Array(w.memory.buffer, ptr, len);
  const str = (ptr, len) => utf8.decode(bytes(ptr, len));

  /** Turns a negative status into a SimbiguityError carrying Rust's message. */
  function check(status) {
    if (status < 0) {
      throw new SimbiguityError(status, str(w.sb_last_error_ptr(), w.sb_last_error_len()));
    }
    return status;
  }

  /** A DC circuit living inside WASM. Call `free()` when done with it. */
  class Circuit {
    constructor() {
      this.handle = w.circuit_new();
    }

    /** Adds a junction; returns its node id. */
    node() { return check(w.circuit_node(this.handle)); }

    /** An ideal battery, `+` at `plus`; returns its component id. */
    battery(plus, minus, volts) { return check(w.circuit_add_battery(this.handle, plus, minus, volts)); }

    resistor(a, b, ohms) { return check(w.circuit_add_resistor(this.handle, a, b, ohms)); }

    capacitor(a, b, farads) { return check(w.circuit_add_capacitor(this.handle, a, b, farads)); }
    
    inductor(a, b, henries) { return check(w.circuit_add_inductor(this.handle, a, b, henries)); }

    /** A bulb rated `watts` at `volts`. */
    bulb(a, b, volts, watts) { return check(w.circuit_add_bulb(this.handle, a, b, volts, watts)); }

    addBjt(c, b, e, is_pnp, saturation_amps = 1e-14, beta_f = 100, beta_r = 1) {
        return check(w.circuit_add_bjt(this.handle, c, b, e, is_pnp ? 1 : 0, saturation_amps, beta_f, beta_r));
    }

    addVcvs(out_plus, out_minus, ctrl_plus, ctrl_minus, gain) {
        return check(w.circuit_add_vcvs(this.handle, out_plus, out_minus, ctrl_plus, ctrl_minus, gain));
    }

    addOpAmp(ctrl_plus, ctrl_minus, out, gain = 1e5, v_sat = 12.0) {
        return check(w.circuit_add_opamp(this.handle, ctrl_plus, ctrl_minus, out, gain, v_sat));
    }

    diode(a, b, is = 1e-14, n = 1.0) { return check(w.circuit_add_diode(this.handle, a, b, is, n)); }

    signal(a, b, waveform, amplitude, frequency, phase = 0.0, offset = 0.0) {
      return check(w.circuit_add_signal(this.handle, a, b, waveform, amplitude, frequency, phase, offset));
    }

    time() { return w.sim_time(this.handle); }

    addProbe(probeType, targetId, capacity) {
      // probeType: 0 = node voltage, 1 = current, 2 = charge
      let typeNum = typeof probeType === "boolean" ? (probeType ? 1 : 0) : probeType;
      const probeId = check(w.sim_add_probe(this.handle, typeNum, targetId, capacity));
      return {
        read: () => {
          const ptr = w.sim_probe_samples_ptr(this.handle, probeId);
          const len = w.sim_probe_samples_len();
          return new Float64Array(w.memory.buffer, ptr, len);
        }
      };
    }

    led(a, b, color = "red", rated_amps = 0.02, is = 1.5195e-17, n = 2.0) {
      return check(w.circuit_add_led(this.handle, a, b, rated_amps, is, n));
    }

    switch(a, b, closed = false) { return check(w.circuit_add_switch(this.handle, a, b, closed ? 1 : 0)); }

    wire(a, b) { return check(w.circuit_add_wire(this.handle, a, b)); }

    /** Flips a switch; returns `true` when it is now closed. */
    toggle(id) { return check(w.circuit_toggle(this.handle, id)) === 1; }

    /** Sets a battery's volts, a resistor's ohms, a bulb's rated volts. */
    setValue(id, value) { check(w.circuit_set_value(this.handle, id, value)); }


    setIntegrator(method) { check(w.circuit_set_integrator(this.handle, method)); }
    plotPush(v) { check(w.circuit_plot_push(this.handle, v)); }
    plotPush2(v) { check(w.circuit_plot_push2(this.handle, v)); }
    plotDraw(width, height, yMin = 0.0, yMax = 12.0) { check(w.circuit_plot_draw(this.handle, width, height, yMin, yMax)); }
    
    replaceBulb(id) { check(w.circuit_replace_bulb(this.handle, id)); }


    /**
     * Steps the circuit by dt.
     */
    step(dt) {
      const h = this.handle;
      const issueCount = check(w.circuit_step(h, dt));
      const solved = check(w.circuit_has_solution(h)) === 1;

      const len = check(w.circuit_results_len(h));
      const raw = Array.from(f64s(w.circuit_results_ptr(h), len));
      const val = (x) => (Number.isNaN(x) ? null : x);
      const parts = raw; // We return the flat array to demo.js directly
      return { solved, parts };
    }

    /**
     * Solves the circuit and reads everything back in one go.
     * @returns {{ solved: boolean, parts: Array<{across:number|null,current:number|null,power:number|null,brightness:number|null}>, issues: Array<{kind:string, part:number, message:string}> }}
     */
    solve() {
      const h = this.handle;
      const issueCount = check(w.circuit_solve(h));
      const solved = check(w.circuit_has_solution(h)) === 1;

      // Copy out of linear memory before any further call can move it.
      const len = check(w.circuit_results_len(h));
      const raw = Array.from(f64s(w.circuit_results_ptr(h), len));
      const val = (x) => (Number.isNaN(x) ? null : x);
      const parts = [];
      for (let i = 0; i < len; i += RESULT_STRIDE) {
        parts.push({
          across: val(raw[i]),
          current: val(raw[i + 1]),
          power: val(raw[i + 2]),
          brightness: val(raw[i + 3]),
        });
      }

      const issues = [];
      for (let i = 0; i < issueCount; i++) {
        const msgLen = check(w.circuit_issue_message(h, i));
        issues.push({
          kind: IssueKind[check(w.circuit_issue_kind(h, i))],
          part: check(w.circuit_issue_part(h, i)),
          message: str(w.sb_out_ptr(), msgLen),
        });
      }
      return { solved, parts, issues };
    }

    /** Releases the circuit. Using it afterwards throws. */
    free() { check(w.circuit_free(this.handle)); }
  }

  return { Circuit, exports: w };
}
