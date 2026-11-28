import { loadCircuit3D } from "../../js/simbiguity-render.js";
import { WebGL2Executor } from "../../js/webgl2-executor.js";
import { bindLevelSelect } from './components/level-select.js';
import { bindControls } from './components/controls.js';
import { GameState } from './game-state.js';

const $ = (id) => document.getElementById(id);

async function main() {
  const canvas = $("view");
  const gl = canvas.getContext("webgl2", { antialias: true, preserveDrawingBuffer: true });
  const [{ Circuit3D, exports: w }, exec] = await Promise.all([
    loadCircuit3D(new URL("../../pkg/simbiguity_render_wasm.wasm", import.meta.url)),
    WebGL2Executor.create(gl),
  ]);

  const state = new GameState(w);
  let view = null;

  function loadLevel(id) {
    if (view) view.free();
    const handle = state.loadLevel(id);
    view = new Circuit3D({ handle });
    for (const m of view.meshes()) exec.uploadMesh(m.id, m.vertices, m.indices);
    draw();
  }

  function size() {
    const dpr = window.devicePixelRatio || 1;
    const w = Math.floor(canvas.clientWidth * dpr);
    const h = Math.floor(canvas.clientHeight * dpr);
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w;
      canvas.height = h;
    }
    return [w, h];
  }

  function draw() {
    if (!view) return;
    const [w, h] = size();
    exec.execute(view.frame(w, h));
  }

  bindLevelSelect((levelId) => {
    loadLevel(levelId);
  });

  bindControls(state, draw);

  // initial load
  loadLevel("1");
  
  if (localStorage.getItem('prototype') === 'short-circuit') {
      state.prototypeSimulate('short-circuit');
  } else if (localStorage.getItem('prototype') === 'corrupted') {
      state.prototypeSimulate('corrupted');
  } else if (localStorage.getItem('prototype') === 'solved') {
      state.share();
  }

  let last = performance.now();
  const loop = (now) => {
    if (view) {
      view.advance(Math.min(0.25, (now - last) / 1000));
      draw();
    }
    last = now;
    requestAnimationFrame(loop);
  };
  requestAnimationFrame(loop);
}

main().catch(console.error);