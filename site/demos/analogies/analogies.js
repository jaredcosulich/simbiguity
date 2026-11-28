import { boot } from "../../js/boot.js";
import { loadCircuits } from "../../js/simbiguity.js";
import { loadCircuit3D } from "../../js/simbiguity-render.js";
import { loadPhysics } from "../physics/physics.js";
import { WebGL2Executor } from "../../js/webgl2-executor.js";
import { parseState, formatState, DEFAULT_STATE } from "./state.js";
import { PARAM_LIMITS, getPhysicsParams, getCircuitParams, getDerived } from "./mapping.js";
import { buildLayout } from "../circuit-lab/experiments.js";

const $ = id => document.getElementById(id);

async function main() {
  const physCanvas = $("view-phys");
  const circCanvas = $("view-circ");
  const plotCanvas = $("view-plot");

  const physGl = physCanvas.getContext("webgl2", { antialias: true, preserveDrawingBuffer: true });
  const circGl = circCanvas.getContext("webgl2", { antialias: true, preserveDrawingBuffer: true });
  const plotGl = plotCanvas.getContext("webgl2", { antialias: true, preserveDrawingBuffer: true });

  const physExecutor = await WebGL2Executor.create(physGl);
  const circExecutor = await WebGL2Executor.create(circGl);
  const plotExecutor = await WebGL2Executor.create(plotGl);

  const { Circuit, exports: circWasmCore } = await loadCircuits(new URL("../../pkg/simbiguity_circuits_wasm.wasm", import.meta.url));
  const { Physics } = await loadPhysics(new URL("../../pkg/simbiguity_physics_wasm.wasm", import.meta.url));

  const { Circuit3D, exports: circWasm } = await loadCircuit3D(new URL("../../pkg/simbiguity_render_wasm.wasm", import.meta.url));

  let currentState = parseState(window.location.hash);

  // Setup UI controls
  const controlsDiv = $("controls");
  const tbody = $("mapping-tbody");

  const sliders = {};
  const labels = {
    m: ["m", "L", "mass / inductance"],
    c: ["c", "R", "damping / resistance"],
    k: ["k", "1/C", "stiffness / inverse capacitance"],
    f0: ["F₀", "V₀", "drive amplitude"],
    omega: ["ω", "ω", "drive frequency"]
  };

  for (const [key, limit] of Object.entries(PARAM_LIMITS)) {
    const tr = document.createElement("tr");
    tr.innerHTML = `<td>${labels[key][0]}</td><td>${labels[key][1]}</td><td id="val-${key}">${currentState[key]}</td>`;
    tbody.appendChild(tr);

    const label = document.createElement("label");
    label.innerText = labels[key][2];
    const input = document.createElement("input");
    input.type = "range";
    input.min = limit.min;
    input.max = limit.max;
    input.step = limit.step;
    input.value = currentState[key];
    
    input.addEventListener("input", e => {
      const v = parseFloat(e.target.value);
      currentState[key] = v;
      $(`val-${key}`).innerText = v.toString();
      updateDerived();
      syncSimulationParams();
      window.history.replaceState(null, "", formatState(currentState));
    });
    sliders[key] = input;
    
    label.appendChild(input);
    controlsDiv.appendChild(label);
  }

  function updateDerived() {
    const { w0, q } = getDerived(currentState);
    $("val-w0").innerText = w0.toFixed(3);
    $("val-q").innerText = q.toFixed(3);
  }
  updateDerived();

  // Create engines
  let physics = null;
  let circuit = null;
  let circuit3D = null;
  let chargeProbe = null;

  // Track components for updating
  let srcId, rId, lId, cId;

  function initEngines() {
    if (physics) physics.free();
    if (circuit) circuit.free();
    if (circuit3D) circuit3D.free();

    const p = getPhysicsParams(currentState);
    physics = new Physics("driven_oscillator", 0, p.m, p.k, p.c, p.f0, p.omega);
    physics.setIntegrator("verlet");

    for (let i = 0; i < physics.getMeshCount(); i++) {
        physExecutor.uploadMesh(i, physics.getMeshVertices(i), physics.getMeshIndices(i));
    }

    const cp = getCircuitParams(currentState);
    circuit = new Circuit();
    const n0 = circuit.node();
    const n1 = circuit.node();
    const n2 = circuit.node();
    const n3 = circuit.node();
    
    // Series RLC for math and plot
    srcId = circuit.signal(n0, n1, 0, cp.v0, cp.omega / (2 * Math.PI), 0, 0);
    lId = circuit.inductor(n1, n2, cp.l);
    rId = circuit.resistor(n2, n3, cp.r);
    cId = circuit.capacitor(n3, n0, cp.c);

    circuit.setIntegrator(1); // trapezoidal
    chargeProbe = circuit.addProbe(2, cId, 1000);

    // 3D RLC board
    const layout = {
      nodeCount: 4,
      parts: [
        { kind: 7, a: 0, b: 1, x: 0, y: 0, waveform: 0, amplitude: cp.v0, frequency: cp.omega / (2 * Math.PI), phase: 0, offset: 0 },
        { kind: 10, a: 1, b: 2, x: 2, y: 0, h: cp.l },
        { kind: 8, a: 2, b: 3, x: 4, y: 0, r: cp.r },
        { kind: 9, a: 3, b: 0, x: 6, y: 0, f: cp.c }
      ],
      wires: []
    };
    const buf = buildLayout(layout.nodeCount, layout.parts, layout.wires);
    const ptr = circWasm.sb_malloc(buf.length * 4);
    const mem = new Uint32Array(circWasm.memory.buffer, ptr, buf.length);
    mem.set(buf);
    const handle = circWasm.c3d_new_from_layout(ptr, buf.length);
    circWasm.sb_free(ptr, buf.length * 4);
    if (handle === 0) {
        const utf8 = new TextDecoder();
        const errLen = circWasm.sb_last_error_len();
        const errPtr = circWasm.sb_last_error_ptr();
        const msg = utf8.decode(new Uint8Array(circWasm.memory.buffer, errPtr, errLen));
        throw new Error("c3d_new_from_layout failed: " + msg);
    }
    // wrap handle manually
    circuit3D = new Circuit3D({ volts: 0, closed: false }); 
    circuit3D.free(); // free the default hello circuit
    circuit3D.handle = handle;
    // Set nice camera
    circuit3D.setCamera(-45, 30, 15);
    
    for (const m of circuit3D.meshes()) circExecutor.uploadMesh(m.id, m.vertices, m.indices);
  }

  function syncSimulationParams() {
    const p = getPhysicsParams(currentState);
    physics.setDrivenOscillatorOmega(p.omega);
    physics.setDrivenOscillatorC(p.c);
    // mass and stiffness can't be easily mutated in standard physics wrapper, 
    // but the plan says "one set of sliders drives both". If they change M or K, 
    // we need to re-init. Let's just re-init for simplicity when they change M, K, or anything.
    initEngines();
  }

  initEngines();

  // Loop
  const dt = 1e-4;
  const frameDt = 1/60;
  const stepsPerFrame = Math.floor(frameDt / dt);

  function frame() {
    for (let i = 0; i < stepsPerFrame; i++) {
      physics.step(dt);
      circuit.step(dt);
      circuit3D.advance(dt);

      const x = physics.getPositions()[3]; 
      
      const results = circuit.step(dt).parts;
      const vC = results[3 * 4]; 
      const cp = getCircuitParams(currentState);
      const q = cp.c * vC;

      circuit.plotPush(x);
      circuit.plotPush2(q);
    }

    physExecutor.execute(physics.encode(physCanvas.width, physCanvas.height), physics.getMeshCount(), physics);

    const cmds = circuit3D.frame(circCanvas.width, circCanvas.height);
    circExecutor.execute(cmds);

    circuit.plotDraw(plotCanvas.width, plotCanvas.height, -1.0, 1.0);
    const count = circWasmCore.circuit_plot_mesh_count(circuit.handle);
    for (let i = 0; i < count; i++) {
        const vLen = circWasmCore.circuit_plot_mesh_vertices_len(circuit.handle, i);
        const vPtr = circWasmCore.circuit_plot_mesh_vertices_ptr(circuit.handle, i);
        const iLen = circWasmCore.circuit_plot_mesh_indices_len(circuit.handle, i);
        const iPtr = circWasmCore.circuit_plot_mesh_indices_ptr(circuit.handle, i);
        plotExecutor.uploadMesh(i, new Float32Array(circWasmCore.memory.buffer, vPtr, vLen), new Uint32Array(circWasmCore.memory.buffer, iPtr, iLen));
    }
    const plotCmds = new Uint32Array(circWasmCore.memory.buffer, circWasmCore.circuit_plot_commands_ptr(circuit.handle), circWasmCore.circuit_plot_commands_len(circuit.handle));
    
    // The plot executor needs a program active before setting uniforms
    if (!plotExecutor.current) {
        plotExecutor.current = plotExecutor.programs[0];
        plotExecutor.gl.useProgram(plotExecutor.current.prog);
    }
    plotExecutor.execute(plotCmds);

    requestAnimationFrame(frame);
  }

  requestAnimationFrame(frame);
}

boot(main);
