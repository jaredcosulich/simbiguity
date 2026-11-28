import { boot } from "../../js/boot.js";
import { loadCircuit3D } from "../../js/simbiguity-render.js";
import { WebGL2Executor } from "../../js/webgl2-executor.js";
import { bindCanvasInput } from "../circuit-3d/canvas-input.js";
import { formatReadouts } from "../circuit-3d/readouts.js";

async function main() {
  const canvas = document.createElement("canvas");
  document.body.appendChild(canvas);
  Object.assign(canvas.style, { width: "100%", height: "80vh", display: "block" });
  
  const gl = canvas.getContext("webgl2", { antialias: true, preserveDrawingBuffer: true });
  const { Circuit3D, exports: w } = await loadCircuit3D(new URL("../../pkg/simbiguity_render_wasm.wasm", import.meta.url));
  const exec = await WebGL2Executor.create(gl);

  // Layout buffer format for a parallel circuit
  // Words: node_count, num_parts, [parts...]
  const buf = new Uint32Array(200);
  let pos = 0;
  buf[pos++] = 5; // nodes: 0(minus), 1(batt+), 2(series-), 3(split), 4(top switch-)
  buf[pos++] = 9; // 5 parts, 4 wires
  
  const putF64 = (v) => {
    const arr = new Float64Array([v]);
    const u32 = new Uint32Array(arr.buffer);
    buf[pos++] = u32[0];
    buf[pos++] = u32[1];
  };

  // Battery: kind=1, nodes 1, 0, cell -2, 0, rot 0
  buf[pos++] = 1; buf[pos++] = 1; buf[pos++] = 0; buf[pos++] = -2 >>> 0; buf[pos++] = 0; buf[pos++] = 0; putF64(24.0);
  
  // Series Bulb: kind=3, nodes 1, 2, cell 0, 0, rot 0
  buf[pos++] = 3; buf[pos++] = 1; buf[pos++] = 2; buf[pos++] = 0; buf[pos++] = 0; buf[pos++] = 0; putF64(12.0); putF64(6.0);

  // Switch (top branch): kind=2, nodes 2, 4, cell 2, -2, rot 0
  buf[pos++] = 2; buf[pos++] = 2; buf[pos++] = 4; buf[pos++] = 2; buf[pos++] = -2 >>> 0; buf[pos++] = 0; buf[pos++] = 1; // closed=1
  
  // Bulb 1 (top branch): kind=3, nodes 4, 0, cell 4, -2, rot 0
  buf[pos++] = 3; buf[pos++] = 4; buf[pos++] = 0; buf[pos++] = 4; buf[pos++] = -2 >>> 0; buf[pos++] = 0; putF64(12.0); putF64(6.0);
  
  // Bulb 2 (bottom branch): kind=3, nodes 2, 0, cell 4, 2, rot 0
  buf[pos++] = 3; buf[pos++] = 2; buf[pos++] = 0; buf[pos++] = 4; buf[pos++] = 2; buf[pos++] = 0; putF64(12.0); putF64(6.0);

  // Wire 1: batt- to bottom branch and up to top branch
  buf[pos++] = 4; buf[pos++] = 0; buf[pos++] = 0; buf[pos++] = 4; // 4 points
  buf[pos++] = -2 >>> 0; buf[pos++] = 0; buf[pos++] = 4; buf[pos++] = 2; buf[pos++] = 4; buf[pos++] = 0; buf[pos++] = 4; buf[pos++] = -2 >>> 0;
  
  // Wire 2: batt+ to series bulb
  buf[pos++] = 4; buf[pos++] = 1; buf[pos++] = 1; buf[pos++] = 2; // 2 points
  buf[pos++] = -2 >>> 0; buf[pos++] = 0; buf[pos++] = 0; buf[pos++] = 0;

  // Wire 3: series bulb to switch and bottom bulb
  buf[pos++] = 4; buf[pos++] = 2; buf[pos++] = 2; buf[pos++] = 3; // 3 points
  buf[pos++] = 0; buf[pos++] = 0; buf[pos++] = 2; buf[pos++] = -2 >>> 0; buf[pos++] = 4; buf[pos++] = 2;

  // Wire 4: switch to top bulb
  buf[pos++] = 4; buf[pos++] = 4; buf[pos++] = 4; buf[pos++] = 2; // 2 points
  buf[pos++] = 2; buf[pos++] = -2 >>> 0; buf[pos++] = 4; buf[pos++] = -2 >>> 0;

  const ptr = w.sb_malloc(pos * 4);
  const mem = new Uint32Array(w.memory.buffer, ptr, pos);
  mem.set(buf.subarray(0, pos));
  
  const handle = w.c3d_new_from_layout(ptr, pos);
  w.sb_free(ptr, pos * 4);
  if (handle === 0) {
    const err = new TextDecoder().decode(new Uint8Array(w.memory.buffer, w.sb_last_error_ptr(), w.sb_last_error_len()));
    console.error("Layout error:", err);
    return;
  }
  
  const view = new Circuit3D({ handle });
  
  view.setCamera(25, 38, 15);
  for (const m of view.meshes()) exec.uploadMesh(m.id, m.vertices, m.indices);

  function draw() {
    canvas.width = canvas.clientWidth * window.devicePixelRatio;
    canvas.height = canvas.clientHeight * window.devicePixelRatio;
    exec.execute(view.frame(canvas.width, canvas.height));
  }

  bindCanvasInput(canvas, view, {
    onInput: draw,
    onOrbit: draw,
    onCamera: draw,
    onKeyToggle: () => { view.toggle(); draw(); }
  });

  let last = performance.now();
  const loop = (now) => {
    view.advance(Math.min(0.25, (now - last) / 1000));
    last = now;
    draw();
    requestAnimationFrame(loop);
  };
  requestAnimationFrame(loop);
}

boot(main);
