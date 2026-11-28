// The 3D hello circuit. Rust owns the circuit, the animation and every
// draw decision; this page only forwards input, replays the frame's draw
// commands with WebGL2 and prints the readouts.

import { loadCircuit3D } from "../../js/simbiguity-render.js";
import { WebGL2Executor } from "../../js/webgl2-executor.js";
import { bindCanvasInput } from "./canvas-input.js";
import { showExplanation, showLoadError } from "./explain-panel.js";
import { explanation } from "./explain.js";
import { canvasSize } from "./pointer.js";
import { formatReadouts } from "./readouts.js";
import { showReadouts } from "./readouts-panel.js";
import { hashFromState, stateFromHash } from "./state.js";

const $ = (id) => document.getElementById(id);

async function main() {
  const canvas = $("view");
  const gl = canvas.getContext("webgl2", { antialias: true, preserveDrawingBuffer: true });
  const [{ Circuit3D }, exec] = await Promise.all([
    loadCircuit3D(new URL("../../pkg/simbiguity_render_wasm.wasm", import.meta.url)),
    WebGL2Executor.create(gl),
  ]);

  const state = stateFromHash(location.hash);
  const view = new Circuit3D({ volts: state.volts, closed: state.closed });
  if (state.camera) view.setCamera(state.camera.yaw, state.camera.pitch, state.camera.distance);
  for (const m of view.meshes()) exec.uploadMesh(m.id, m.vertices, m.indices);

  const frozen = state.fixedTime !== null;
  if (frozen) view.seek(state.fixedTime);

  function size() {
    const [w, h] = canvasSize(canvas.clientWidth, canvas.clientHeight, window.devicePixelRatio);
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w;
      canvas.height = h;
    }
    return [w, h];
  }

  function draw() {
    const [w, h] = size();
    exec.execute(view.frame(w, h));
    show(view.readouts());
  }

  /** After any input: frozen pages re-run to the same moment. */
  function changed() {
    if (frozen) view.seek(state.fixedTime);
    saveHash(view.readouts(), state);
    draw();
  }

  // Controls
  $("volts").value = String(state.volts);
  $("volts").oninput = (e) => {
    view.setVolts(Number(e.target.value));
    changed();
  };
  $("toggle").onclick = () => {
    view.toggle();
    changed();
  };
  $("replace").onclick = () => {
    view.replaceBulb();
    changed();
  };

  // Orbit by dragging, zoom with the wheel, click the switch to flip it.
  const orbited = () => {
    state.camera = view.readouts().camera;
    if (frozen) draw();
  };
  bindCanvasInput(canvas, view, {
    onInput: changed,
    onOrbit: orbited,
    onCamera: () => {
      orbited();
      saveHash(view.readouts(), state);
    },
    onKeyToggle: () => $("toggle").onclick(),
  });

  if (frozen) {
    new ResizeObserver(draw).observe(canvas);
    draw();
  } else {
    let last = performance.now();
    const loop = (now) => {
      view.advance(Math.min(0.25, (now - last) / 1000));
      last = now;
      draw();
      requestAnimationFrame(loop);
    };
    draw();
    requestAnimationFrame(loop);
  }
  document.body.dataset.ready = "true";
}

/** Updates the panels, skipping frames where nothing they show changed. */
let lastShown = "";
function show(r) {
  const d = formatReadouts(r);
  const e = explanation(r);
  const key = JSON.stringify([d, e]);
  if (key === lastShown) return;
  lastShown = key;
  showReadouts(d);
  showExplanation(e);
}

export function saveHash(r, state) {
  const camera = r.camera;
  const custom = state.camera !== null;
  history.replaceState(null, "", location.pathname + location.search + hashFromState({
    closed: r.closed,
    volts: r.batteryVolts,
    fixedTime: state.fixedTime,
    camera: custom ? camera : null,
  }));
}

main().catch((err) => {
  showLoadError(err);
  throw err;
});
