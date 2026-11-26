// webgl2-executor.js — replays a simbiguity draw-command buffer with WebGL2.
//
// Rust (crates/simbiguity-render) decides everything about a frame: draw
// order, matrices, colours, blend state. This file only turns those words
// into GL calls. It has no scene, no camera and no logic of its own.
//
//   const exec = await WebGL2Executor.create(canvas.getContext("webgl2"));
//   exec.uploadMesh(0, vertices /* Float32Array */, indices /* Uint32Array */);
//   exec.execute(words /* Uint32Array from c3d_encode */);
//
// The byte layout is documented in crates/simbiguity-render/src/commands.rs.

export const MAGIC = 0x43524253; // "SBRC"
export const VERSION = 1;

/** Uniform names, indexed by the slot numbers Rust writes. */
export const UNIFORMS = Object.freeze([
  "u_viewProj", "u_model", "u_color", "u_lightDir", "u_lightColor", "u_eye", "u_emissive", "u_scale",
]);

/** Shader sources per program id, relative to this module. */
const PROGRAMS = Object.freeze([
  ["mesh.vert", "unlit.frag"],
  ["mesh.vert", "lit.frag"],
  ["mesh.vert", "glow.frag"],
  ["instanced.vert", "instanced.frag"],
]);

/** Payload sizes in words for fixed-size opcodes (9 is variable). */
const FIXED = Object.freeze({ 1: 4, 2: 2, 3: 1, 4: 17, 5: 5, 6: 2, 7: 1, 8: 2, 10: 1, 11: 2 });

/** Thrown for a buffer this executor cannot replay. */
export class CommandBufferError extends Error {
  constructor(message) {
    super(message);
    this.name = "CommandBufferError";
  }
}

/**
 * Decodes a command buffer into plain objects. Pure: no GL, so it can be
 * tested in Node. Rejects a bad header, an unknown opcode or a wrong
 * payload length instead of guessing.
 * @param {Uint32Array} words
 * @returns {Array<object>}
 */
export function decodeCommands(words) {
  if (words.length < 2) throw new CommandBufferError("missing header");
  if (words[0] !== MAGIC) throw new CommandBufferError(`bad magic 0x${words[0].toString(16)}`);
  if (words[1] !== VERSION) throw new CommandBufferError(`unsupported version ${words[1]}`);
  const f32 = new Float32Array(words.buffer, words.byteOffset, words.length);
  const out = [];
  let i = 2;
  while (i < words.length) {
    const op = words[i] & 0xff;
    const len = words[i] >>> 8;
    const p = i + 1;
    if (p + len > words.length) throw new CommandBufferError(`truncated command at word ${i}`);
    if (op === 9 ? (len < 1 || (len - 1) % 3 !== 0) : FIXED[op] !== len) {
      if (FIXED[op] === undefined && op !== 9) throw new CommandBufferError(`unknown opcode ${op} at word ${i}`);
      throw new CommandBufferError(`bad length ${len} for opcode ${op} at word ${i}`);
    }
    switch (op) {
      case 1: out.push({ op: "clear", rgba: Array.from(f32.subarray(p, p + 4)) }); break;
      case 2: out.push({ op: "viewport", width: words[p], height: words[p + 1] }); break;
      case 3: out.push({ op: "program", id: words[p] }); break;
      case 4: out.push({ op: "mat4", slot: words[p], value: f32.slice(p + 1, p + 17) }); break;
      case 5: out.push({ op: "vec4", slot: words[p], value: f32.slice(p + 1, p + 5) }); break;
      case 6: out.push({ op: "f32", slot: words[p], value: f32[p + 1] }); break;
      case 7: out.push({ op: "mesh", id: words[p] }); break;
      case 8: out.push({ op: "draw", count: words[p], first: words[p + 1] }); break;
      case 9: out.push({ op: "instanced", count: words[p], offsets: f32.slice(p + 1, p + len) }); break;
      case 10: out.push({ op: "blend", mode: words[p] }); break;
      case 11: out.push({ op: "depth", test: words[p] !== 0, write: words[p + 1] !== 0 }); break;
    }
    i = p + len;
  }
  return out;
}

async function fetchText(name) {
  const res = await fetch(new URL(`./shaders/${name}`, import.meta.url));
  if (!res.ok) throw new Error(`could not load shader ${name}: HTTP ${res.status}`);
  return res.text();
}

export class WebGL2Executor {
  /**
   * Compiles the built-in programs and returns a ready executor.
   * @param {WebGL2RenderingContext} gl
   */
  static async create(gl) {
    if (!gl) throw new Error("WebGL2 is not available in this browser");
    const names = [...new Set(PROGRAMS.flat())];
    const sources = Object.fromEntries(await Promise.all(names.map(async (n) => [n, await fetchText(n)])));
    return new WebGL2Executor(gl, sources);
  }

  constructor(gl, sources) {
    this.gl = gl;
    this.programs = PROGRAMS.map(([vs, fs]) => this.#link(sources[vs], sources[fs], `${vs}+${fs}`));
    this.meshes = new Map();
    this.instanceBuffer = gl.createBuffer();
    this.current = null;
    gl.enable(gl.CULL_FACE);
    gl.cullFace(gl.BACK);
  }

  #compile(type, src, label) {
    const gl = this.gl;
    const s = gl.createShader(type);
    gl.shaderSource(s, src);
    gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
      throw new Error(`shader ${label} failed to compile: ${gl.getShaderInfoLog(s)}`);
    }
    return s;
  }

  #link(vs, fs, label) {
    const gl = this.gl;
    const prog = gl.createProgram();
    gl.attachShader(prog, this.#compile(gl.VERTEX_SHADER, vs, label));
    gl.attachShader(prog, this.#compile(gl.FRAGMENT_SHADER, fs, label));
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
      throw new Error(`program ${label} failed to link: ${gl.getProgramInfoLog(prog)}`);
    }
    return { prog, loc: UNIFORMS.map((u) => gl.getUniformLocation(prog, u)) };
  }

  /**
   * Uploads (or replaces) mesh `id`: interleaved position+normal floats and
   * u32 triangle indices.
   */
  uploadMesh(id, vertices, indices) {
    const gl = this.gl;
    const vao = gl.createVertexArray();
    gl.bindVertexArray(vao);
    const vbo = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
    gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 24, 0);
    gl.enableVertexAttribArray(1);
    gl.vertexAttribPointer(1, 3, gl.FLOAT, false, 24, 12);
    const ibo = gl.createBuffer();
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, ibo);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, indices, gl.STATIC_DRAW);
    gl.bindVertexArray(null);
    this.meshes.set(id, { vao });
  }

  /** Replays one frame. */
  execute(words) {
    const gl = this.gl;
    for (const c of decodeCommands(words)) {
      switch (c.op) {
        case "clear":
          gl.clearColor(...c.rgba);
          gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
          break;
        case "viewport": gl.viewport(0, 0, c.width, c.height); break;
        case "program":
          this.current = this.programs[c.id];
          if (!this.current) throw new CommandBufferError(`unknown program ${c.id}`);
          gl.useProgram(this.current.prog);
          break;
        case "mat4": gl.uniformMatrix4fv(this.#loc(c.slot), false, c.value); break;
        case "vec4": gl.uniform4fv(this.#loc(c.slot), c.value); break;
        case "f32": gl.uniform1f(this.#loc(c.slot), c.value); break;
        case "mesh": {
          const m = this.meshes.get(c.id);
          if (!m) throw new CommandBufferError(`mesh ${c.id} was never uploaded`);
          gl.bindVertexArray(m.vao);
          break;
        }
        case "draw": gl.drawElements(gl.TRIANGLES, c.count, gl.UNSIGNED_INT, c.first * 4); break;
        case "instanced": {
          gl.bindBuffer(gl.ARRAY_BUFFER, this.instanceBuffer);
          gl.bufferData(gl.ARRAY_BUFFER, c.offsets, gl.DYNAMIC_DRAW);
          gl.enableVertexAttribArray(2);
          gl.vertexAttribPointer(2, 3, gl.FLOAT, false, 12, 0);
          gl.vertexAttribDivisor(2, 1);
          gl.drawElementsInstanced(gl.TRIANGLES, c.count, gl.UNSIGNED_INT, 0, c.offsets.length / 3);
          gl.disableVertexAttribArray(2);
          break;
        }
        case "blend":
          if (c.mode === 0) gl.disable(gl.BLEND);
          else {
            gl.enable(gl.BLEND);
            if (c.mode === 1) gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
            else gl.blendFunc(gl.ONE, gl.ONE);
          }
          break;
        case "depth":
          if (c.test) gl.enable(gl.DEPTH_TEST); else gl.disable(gl.DEPTH_TEST);
          gl.depthMask(c.write);
          break;
      }
    }
    gl.bindVertexArray(null);
  }

  #loc(slot) {
    if (!this.current) throw new CommandBufferError("uniform set before any program");
    return this.current.loc[slot] ?? null;
  }
}
