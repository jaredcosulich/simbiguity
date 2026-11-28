import { describe, it } from "node:test";
import assert from "node:assert/strict";

import { CommandBufferError, MAGIC, UNIFORMS, VERSION, decodeCommands, WebGL2Executor } from "./webgl2-executor.js";

const f = (x) => new Uint32Array(Float32Array.of(x).buffer)[0];
const head = (op, len) => op | (len << 8);

/** A buffer of header + the given command words. */
const words = (...cmds) => Uint32Array.from([MAGIC, VERSION, ...cmds.flat()]);

describe("decodeCommands", () => {
  // A header-only buffer decodes to no commands.
  it("decodes an empty frame", () => {
    assert.deepEqual(decodeCommands(words()), []);
  });

  // Every opcode decodes to its named command with its payload.
  it("decodes every opcode", () => {
    const mat = Array.from({ length: 16 }, (_, i) => f(i));
    const cmds = decodeCommands(words(
      [head(1, 4), f(0.5), f(0.25), f(0), f(1)],
      [head(2, 2), 640, 400],
      [head(3, 1), 1],
      [head(4, 17), 0, ...mat],
      [head(5, 5), 2, f(1), f(0.5), f(0.25), f(1)],
      [head(6, 2), 6, f(1.5)],
      [head(7, 1), 9],
      [head(8, 2), 36, 0],
      [head(9, 4), 12, f(1), f(2), f(3)],
      [head(10, 1), 2],
      [head(11, 2), 1, 0],
    ));
    assert.deepEqual(cmds.map((c) => c.op), [
      "clear", "viewport", "program", "mat4", "vec4", "f32", "mesh", "draw", "instanced", "blend", "depth",
    ]);
    assert.deepEqual(cmds[0].rgba, [0.5, 0.25, 0, 1]);
    assert.deepEqual(cmds[1], { op: "viewport", width: 640, height: 400 });
    assert.deepEqual(Array.from(cmds[3].value), Array.from({ length: 16 }, (_, i) => i));
    assert.equal(cmds[5].value, 1.5);
    assert.deepEqual(cmds[7], { op: "draw", count: 36, first: 0 });
    assert.deepEqual(Array.from(cmds[8].offsets), [1, 2, 3]);
    assert.deepEqual(cmds[10], { op: "depth", test: true, write: false });
  });

  // A missing header, wrong magic or unknown version is refused.
  it("rejects bad headers", () => {
    assert.throws(() => decodeCommands(Uint32Array.of(MAGIC)), /missing header/);
    assert.throws(() => decodeCommands(Uint32Array.of(1, VERSION)), /bad magic 0x1/);
    assert.throws(() => decodeCommands(Uint32Array.of(MAGIC, 9)), /unsupported version 9/);
  });

  // An opcode the executor does not know is refused, naming the word.
  it("rejects an unknown opcode", () => {
    assert.throws(() => decodeCommands(words([head(42, 0)])), /unknown opcode 42 at word 2/);
  });

  // A payload length that does not fit the opcode is refused.
  it("rejects a wrong payload length", () => {
    assert.throws(() => decodeCommands(words([head(7, 2), 1, 2])), /bad length 2 for opcode 7/);
    assert.throws(() => decodeCommands(words([head(9, 3), 1, 2, 3])), /bad length 3 for opcode 9/);
    assert.throws(() => decodeCommands(words([head(9, 0)])), /bad length 0 for opcode 9/);
  });

  // A payload that runs past the end is truncated.
  it("rejects a truncated command", () => {
    assert.throws(() => decodeCommands(words([head(1, 4), 0, 0])), /truncated command at word 2/);
  });

  // Every decoding failure is a CommandBufferError.
  it("throws CommandBufferError", () => {
    assert.throws(() => decodeCommands(Uint32Array.of(0, 0)), (e) => e instanceof CommandBufferError && e.name === "CommandBufferError");
  });
});

describe("UNIFORMS", () => {
  // Uniform names are in the slot order Rust writes (crates/simbiguity-render/src/commands.rs).
  it("lists the eight slots in Rust order", () => {
    assert.equal(UNIFORMS.length, 8);
    assert.equal(UNIFORMS[0], "u_viewProj");
    assert.equal(UNIFORMS[7], "u_scale");
  });
});

describe("WebGL2Executor", () => {
  it("executes GL calls from decoded commands", async () => {
    const originalFetch = global.fetch;
    let fetched = false;
    global.fetch = async () => {
      fetched = true;
      return new Response("fake shader text");
    };

    try {
      let clearArgs = null;
      let clearCalled = false;
      const gl = {
        CULL_FACE: 1, BACK: 2, VERTEX_SHADER: 3, FRAGMENT_SHADER: 4, LINK_STATUS: 5, COMPILE_STATUS: 6,
        ARRAY_BUFFER: 7, STATIC_DRAW: 8, FLOAT: 9, ELEMENT_ARRAY_BUFFER: 10, TRIANGLES: 11, UNSIGNED_INT: 12,
        COLOR_BUFFER_BIT: 13, DEPTH_BUFFER_BIT: 14, BLEND: 15, SRC_ALPHA: 16, ONE_MINUS_SRC_ALPHA: 17, ONE: 18,
        DEPTH_TEST: 19,
        enable: () => {}, cullFace: () => {}, createShader: () => ({}),
        shaderSource: () => {}, compileShader: () => {},
        getShaderParameter: () => true, createProgram: () => ({}),
        attachShader: () => {}, linkProgram: () => {},
        getProgramParameter: () => true, getUniformLocation: () => ({}),
        createVertexArray: () => {}, bindVertexArray: () => {},
        createBuffer: () => {}, bindBuffer: () => {}, bufferData: () => {},
        enableVertexAttribArray: () => {}, vertexAttribPointer: () => {},
        disableVertexAttribArray: () => {}, vertexAttribDivisor: () => {},
        drawElementsInstanced: () => {}, drawElements: () => {},
        clearColor: (r, g, b, a) => { clearArgs = [r, g, b, a]; },
        clear: () => { clearCalled = true; }, viewport: () => {},
        useProgram: () => {}, uniformMatrix4fv: () => {},
        uniform4fv: () => {}, uniform1f: () => {},
        disable: () => {}, blendFunc: () => {}, depthMask: () => {}
      };
      
      const exec = await WebGL2Executor.create(gl);
      
      const words = new Uint32Array(8);
      words[0] = MAGIC;
      words[1] = VERSION;
      words[2] = 0x00000401; // op=1 (clear), len=4
      const f32 = new Float32Array(words.buffer);
      f32[3] = 0.1; f32[4] = 0.2; f32[5] = 0.3; f32[6] = 1.0;
      
      exec.execute(words.slice(0, 7));
      
      assert.ok(fetched, "should fetch shaders");
      assert.ok(clearCalled, "should call gl.clear");
      assert.ok(Math.abs(clearArgs[0] - 0.1) < 0.0001);
      assert.ok(Math.abs(clearArgs[1] - 0.2) < 0.0001);
      assert.ok(Math.abs(clearArgs[2] - 0.3) < 0.0001);
      assert.equal(clearArgs[3], 1.0);
    } finally {
      global.fetch = originalFetch;
    }
  });
});
