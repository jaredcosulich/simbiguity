//! The draw-command buffer: one frame as a flat stream of `u32` words.
//!
//! ## Byte layout (version 1)
//!
//! Little-endian `u32` words (`f32` values are stored as their bit pattern):
//!
//! ```text
//! word 0      MAGIC   0x4352_4253  ("SBRC" read as bytes)
//! word 1      VERSION 1
//! then, per command:
//!   header    opcode | (payload_words << 8)
//!   payload   payload_words words, layout per opcode below
//! ```
//!
//! | op | command          | payload                                   |
//! |----|------------------|-------------------------------------------|
//! | 1  | `Clear`          | r g b a (`f32`)                           |
//! | 2  | `Viewport`       | width height (`u32`)                      |
//! | 3  | `SetProgram`     | program (`u32`, see [`Program`])          |
//! | 4  | `SetUniformMat4` | slot (`u32`), 16 × `f32` column-major      |
//! | 5  | `SetUniformVec4` | slot (`u32`), 4 × `f32`                    |
//! | 6  | `SetUniformF32`  | slot (`u32`), `f32`                        |
//! | 7  | `BindMesh`       | mesh id (`u32`)                           |
//! | 8  | `DrawIndexed`    | index count, first index (`u32`)          |
//! | 9  | `DrawInstanced`  | index count (`u32`), then 3 × `f32` per instance offset |
//! | 10 | `SetBlend`       | mode (`u32`, see [`Blend`])               |
//! | 11 | `SetDepth`       | test (`0`/`1`), write (`0`/`1`)           |
//!
//! A decoder must reject a wrong magic, an unknown version and an unknown
//! opcode rather than guess; the payload length lets it skip nothing
//! silently. `site/js/webgl2-executor.js` is the reference consumer.

use alloc::vec::Vec;

/// First word of every buffer.
pub const MAGIC: u32 = 0x4352_4253;
/// The layout version this crate writes and reads.
pub const VERSION: u32 = 1;

/// The built-in shader programs, by id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Program {
    /// Flat colour.
    Unlit = 0,
    /// Lambert + Blinn-Phong + emissive.
    Lit = 1,
    /// Additive view-facing halo.
    Glow = 2,
    /// Instanced self-lit particles.
    Instanced = 3,
}

/// Uniform slots. A backend maps each slot to a named uniform
/// (`u_viewProj`, `u_model`, …) in whatever program is bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Uniform {
    /// `mat4 u_viewProj`: world → clip.
    ViewProj = 0,
    /// `mat4 u_model`: model → world.
    Model = 1,
    /// `vec4 u_color`: RGB + alpha.
    Color = 2,
    /// `vec4 u_lightDir`: direction toward the light (w unused).
    LightDir = 3,
    /// `vec4 u_lightColor`: RGB + ambient fraction in w.
    LightColor = 4,
    /// `vec4 u_eye`: camera position (w unused).
    Eye = 5,
    /// `float u_emissive`.
    Emissive = 6,
    /// `float u_scale`: instance mesh scale.
    Scale = 7,
}

impl Uniform {
    /// Number of slots.
    pub const COUNT: u32 = 8;
}

/// How fragments combine with what is already drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Blend {
    /// Overwrite.
    Off = 0,
    /// `src·α + dst·(1 − α)`: glass.
    Alpha = 1,
    /// `src·α + dst`: light adding up.
    Additive = 2,
}

/// One decoded command.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Clear colour and depth.
    Clear([f32; 4]),
    /// Set the viewport, in pixels.
    Viewport(u32, u32),
    /// Bind a built-in program.
    SetProgram(Program),
    /// Set a `mat4` uniform.
    SetUniformMat4(Uniform, [f32; 16]),
    /// Set a `vec4` uniform.
    SetUniformVec4(Uniform, [f32; 4]),
    /// Set a `float` uniform.
    SetUniformF32(Uniform, f32),
    /// Bind a mesh's vertex and index buffers.
    BindMesh(u32),
    /// Draw `count` indices starting at `first`.
    DrawIndexed {
        /// Indices to draw.
        count: u32,
        /// First index.
        first: u32,
    },
    /// Draw the bound mesh once per offset.
    DrawInstanced {
        /// Indices per instance.
        count: u32,
        /// Per-instance world offsets.
        offsets: Vec<[f32; 3]>,
    },
    /// Set the blend mode.
    SetBlend(Blend),
    /// Enable/disable depth testing and depth writes.
    SetDepth {
        /// Depth test on.
        test: bool,
        /// Depth writes on.
        write: bool,
    },
}

/// A frame's commands, encoded. Decoding lives in [`crate::decode`].
#[derive(Debug, Clone, PartialEq)]
pub struct CommandBuffer {
    words: Vec<u32>,
}

impl Default for CommandBuffer {
    fn default() -> CommandBuffer {
        CommandBuffer::new()
    }
}

impl CommandBuffer {
    /// A buffer holding only the header.
    pub fn new() -> CommandBuffer {
        CommandBuffer {
            words: alloc::vec![MAGIC, VERSION],
        }
    }

    /// Empties back to the header, keeping the allocation.
    pub fn clear(&mut self) {
        self.words.truncate(2);
    }

    /// The encoded words.
    pub fn words(&self) -> &[u32] {
        &self.words
    }

    /// Appends one command.
    pub fn push(&mut self, c: &Command) {
        let w = &mut self.words;
        let header = w.len();
        w.push(0);
        let op = match c {
            Command::Clear(rgba) => {
                w.extend(rgba.iter().map(|f| f.to_bits()));
                1
            }
            Command::Viewport(x, y) => {
                w.extend([*x, *y]);
                2
            }
            Command::SetProgram(p) => {
                w.push(*p as u32);
                3
            }
            Command::SetUniformMat4(u, m) => {
                w.push(*u as u32);
                w.extend(m.iter().map(|f| f.to_bits()));
                4
            }
            Command::SetUniformVec4(u, v) => {
                w.push(*u as u32);
                w.extend(v.iter().map(|f| f.to_bits()));
                5
            }
            Command::SetUniformF32(u, f) => {
                w.extend([*u as u32, f.to_bits()]);
                6
            }
            Command::BindMesh(id) => {
                w.push(*id);
                7
            }
            Command::DrawIndexed { count, first } => {
                w.extend([*count, *first]);
                8
            }
            Command::DrawInstanced { count, offsets } => {
                w.push(*count);
                for o in offsets {
                    w.extend(o.iter().map(|f| f.to_bits()));
                }
                9
            }
            Command::SetBlend(b) => {
                w.push(*b as u32);
                10
            }
            Command::SetDepth { test, write } => {
                w.extend([u32::from(*test), u32::from(*write)]);
                11
            }
        };
        let payload = (w.len() - header - 1) as u32;
        w[header] = op | (payload << 8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each header packs the opcode in the low byte and the payload word count above it.
    #[test]
    fn headers_pack_opcode_and_length() {
        let mut b = CommandBuffer::new();
        b.push(&Command::Viewport(3, 4));
        b.push(&Command::SetDepth {
            test: true,
            write: false,
        });
        assert_eq!(&b.words()[2..], &[2 | (2 << 8), 3, 4, 11 | (2 << 8), 1, 0]);
    }

    /// f32 payloads are stored as their bit patterns.
    #[test]
    fn floats_are_stored_as_bits() {
        let mut b = CommandBuffer::new();
        b.push(&Command::SetUniformF32(Uniform::Emissive, 1.5));
        assert_eq!(&b.words()[2..], &[6 | (2 << 8), 6, 1.5f32.to_bits()]);
    }

    /// clear drops every command but keeps the header, and Default equals new.
    #[test]
    fn clear_keeps_only_the_header() {
        let mut b = CommandBuffer::default();
        b.push(&Command::BindMesh(1));
        b.clear();
        assert_eq!(b, CommandBuffer::new());
        assert_eq!(Uniform::COUNT, Uniform::Scale as u32 + 1);
    }
}
