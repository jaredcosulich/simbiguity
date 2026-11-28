import { loadCircuit3D } from "../../../js/simbiguity-render.js";
import { WebGL2Executor } from "../../../js/webgl2-executor.js";
import { buildLayout } from "../experiments.js";

export class BoardView {
  constructor(canvasId) {
    this.canvas = document.getElementById(canvasId);
    this.gl = this.canvas.getContext("webgl2", { antialias: true, preserveDrawingBuffer: true });
    this.exec = null;
    this.w = null;
    this.Circuit3D = null;
    this.view = null;
  }
  
  async init() {
    const [{ Circuit3D, exports: w }, exec] = await Promise.all([
      loadCircuit3D(new URL("../../../pkg/simbiguity_render_wasm.wasm", import.meta.url)),
      WebGL2Executor.create(this.gl)
    ]);
    this.w = w;
    this.Circuit3D = Circuit3D;
    this.exec = exec;
  }
  
  loadLayout(layout) {
    const buf = buildLayout(layout.nodeCount, layout.parts, layout.wires);
    const ptr = this.w.sb_malloc(buf.length * 4);
    const mem = new Uint32Array(this.w.memory.buffer, ptr, buf.length);
    mem.set(buf);
    
    const handle = this.w.c3d_new_from_layout(ptr, buf.length);
    this.w.sb_free(ptr, buf.length * 4);
    
    if (handle === 0) {
      const err = new TextDecoder().decode(new Uint8Array(this.w.memory.buffer, this.w.sb_last_error_ptr(), this.w.sb_last_error_len()));
      throw new Error(err);
    }
    
    this.view = new this.Circuit3D({ handle });
    this.view.setCamera(25, 38, 15);
    for (const m of this.view.meshes()) {
      this.exec.uploadMesh(m.id, m.vertices, m.indices);
    }
  }
  
  renderFrame(dt) {
    if (!this.view) return null;
    this.view.advance(dt);
    this.canvas.width = this.canvas.clientWidth * window.devicePixelRatio;
    this.canvas.height = this.canvas.clientHeight * window.devicePixelRatio;
    this.exec.execute(this.view.frame(this.canvas.width, this.canvas.height));
    return this.view.readouts(); // or whatever readouts the view supports
  }
}