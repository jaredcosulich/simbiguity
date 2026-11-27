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
  // Nodes: 0, 1, 2, 3, 4, 5
  // Components:
  // Battery: 1, a=1, b=0, x=0, z=0, r=0, emf=12.0
  // Bulb 1:  3, a=2, b=0, x=3, z=-1, r=0, volts=12.0, watts=6.0
  // Bulb 2:  3, a=2, b=0, x=3, z=1, r=0, volts=12.0, watts=6.0
  // Wire 1:  4, a=1, b=2, pts=2, [(0,0), (3,0)] // Wait, battery cell is (0,0)?
  const buf = new Uint32Array(100);
  let pos = 0;
  buf[pos++] = 4; // nodes: 0(minus), 1(plus), 2(switch top), 3(split)
  buf[pos++] = 6; // 3 parts, 3 wires
  
  const putF64 = (v) => {
    const arr = new Float64Array([v]);
    const u32 = new Uint32Array(arr.buffer);
    buf[pos++] = u32[0];
    buf[pos++] = u32[1];
  };

  // Battery: kind=1, nodes 1, 0, cell 0, 0, rot 0
  buf[pos++] = 1; buf[pos++] = 1; buf[pos++] = 0; buf[pos++] = 0; buf[pos++] = 0; buf[pos++] = 0; putF64(12.0);
  
  // Switch: kind=2, nodes 1, 2, cell 1, 0, rot 0
  buf[pos++] = 2; buf[pos++] = 1; buf[pos++] = 2; buf[pos++] = 1; buf[pos++] = 0; buf[pos++] = 0; buf[pos++] = 1; // closed=1
  
  // Bulb 1 (top branch): kind=3, nodes 2, 0, cell 3, -2, rot 0
  buf[pos++] = 3; buf[pos++] = 2; buf[pos++] = 0; buf[pos++] = 3; buf[pos++] = -2 >>> 0; buf[pos++] = 0; putF64(12.0); putF64(6.0);
  
  // Bulb 2 (bottom branch): kind=3, nodes 2, 0, cell 3, 2, rot 0
  buf[pos++] = 3; buf[pos++] = 2; buf[pos++] = 0; buf[pos++] = 3; buf[pos++] = 2; buf[pos++] = 0; putF64(12.0); putF64(6.0);

  // Wire 1 (split): kind=4, nodes 2, 3(wait actually it connects switch to bulbs), cell 2,0
  buf[pos++] = 4; buf[pos++] = 2; buf[pos++] = 0; buf[pos++] = 3; // 3 points
  buf[pos++] = 1; buf[pos++] = 0; buf[pos++] = 2; buf[pos++] = 0; buf[pos++] = 3; buf[pos++] = -2 >>> 0;
  
  // Wire 2: bottom branch
  buf[pos++] = 4; buf[pos++] = 2; buf[pos++] = 0; buf[pos++] = 3; // 3 points
  buf[pos++] = 1; buf[pos++] = 0; buf[pos++] = 2; buf[pos++] = 0; buf[pos++] = 3; buf[pos++] = 2;
  
  const ptr = w.sb_malloc(pos * 4);
  const mem = new Uint32Array(w.memory.buffer, ptr, pos);
  mem.set(buf.subarray(0, pos));
  
  const handle = w.c3d_new_from_layout(ptr, pos);
  if (handle === 0) {
    const err = new TextDecoder().decode(new Uint8Array(w.memory.buffer, w.sb_last_error_ptr(), w.sb_last_error_len()));
    console.error("Layout error:", err);
    return;
  }
  w.sb_free(ptr, pos * 4);
  
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

main().catch(console.error);
