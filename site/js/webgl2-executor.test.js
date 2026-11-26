import { describe, it } from "node:test";
import assert from "node:assert/strict";

import { CommandBufferError, MAGIC, UNIFORMS, VERSION, decodeCommands } from "./webgl2-executor.js";

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
