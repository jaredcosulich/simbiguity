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
export async function loadCircuits(url) {
  // The module imports nothing: the engine needs no host functions.
  const response = fetch(url);
  const { instance } = WebAssembly.instantiateStreaming
    ? await WebAssembly.instantiateStreaming(response, {}).catch(async () =>
        // Some servers send .wasm without `application/wasm`; fall back.
        WebAssembly.instantiate(await (await fetch(url)).arrayBuffer(), {}))
    : await WebAssembly.instantiate(await (await response).arrayBuffer(), {});
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

    switch(a, b, closed = false) { return check(w.circuit_add_switch(this.handle, a, b, closed ? 1 : 0)); }

    wire(a, b) { return check(w.circuit_add_wire(this.handle, a, b)); }

    /** Flips a switch; returns `true` when it is now closed. */
    toggle(id) { return check(w.circuit_toggle(this.handle, id)) === 1; }

    /** Sets a battery's volts, a resistor's ohms, a bulb's rated volts. */
    setValue(id, value) { check(w.circuit_set_value(this.handle, id, value)); }


    setIntegrator(method) { check(w.circuit_set_integrator(this.handle, method)); }
    plotPush(v) { check(w.circuit_plot_push(this.handle, v)); }
    plotDraw(width, height) { check(w.circuit_plot_draw(this.handle, width, height)); }
    
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
