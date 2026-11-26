//! Reading a draw-command buffer back into [`Command`]s.
//!
//! The layout is documented in [`crate::commands`]. A decoder must reject a
//! wrong magic, an unknown version and an unknown opcode rather than guess.

use alloc::vec::Vec;

use crate::commands::{Blend, Command, CommandBuffer, Program, Uniform, MAGIC, VERSION};

/// Why a buffer could not be decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// Shorter than the two header words.
    MissingHeader,
    /// Word 0 is not [`MAGIC`].
    BadMagic(u32),
    /// Word 1 is a version this decoder does not know.
    UnsupportedVersion(u32),
    /// An opcode this version does not define, at word `at`.
    UnknownOp {
        /// The opcode.
        op: u32,
        /// Word index of its header.
        at: usize,
    },
    /// A known opcode with the wrong payload length, at word `at`.
    BadLength {
        /// The opcode.
        op: u32,
        /// Word index of its header.
        at: usize,
    },
    /// A command's payload runs past the end of the buffer.
    Truncated,
    /// An enum value (program, uniform, blend) out of range, at word `at`.
    BadValue {
        /// Word index of the command header.
        at: usize,
    },
}

impl CommandBuffer {
    /// Decodes a buffer back into commands.
    pub fn decode(words: &[u32]) -> Result<Vec<Command>, DecodeError> {
        if words.len() < 2 {
            return Err(DecodeError::MissingHeader);
        }
        if words[0] != MAGIC {
            return Err(DecodeError::BadMagic(words[0]));
        }
        if words[1] != VERSION {
            return Err(DecodeError::UnsupportedVersion(words[1]));
        }
        let mut out = Vec::new();
        let mut i = 2;
        while i < words.len() {
            let (op, len) = (words[i] & 0xff, (words[i] >> 8) as usize);
            let p = words
                .get(i + 1..i + 1 + len)
                .ok_or(DecodeError::Truncated)?;
            out.push(decode_one(op, p, i)?);
            i += 1 + len;
        }
        Ok(out)
    }
}

fn decode_one(op: u32, p: &[u32], at: usize) -> Result<Command, DecodeError> {
    let f = |k: usize| f32::from_bits(p[k]);
    let need = |n: usize| {
        if p.len() == n {
            Ok(())
        } else {
            Err(DecodeError::BadLength { op, at })
        }
    };
    let bad = DecodeError::BadValue { at };
    Ok(match op {
        1 => {
            need(4)?;
            Command::Clear([f(0), f(1), f(2), f(3)])
        }
        2 => {
            need(2)?;
            Command::Viewport(p[0], p[1])
        }
        3 => {
            need(1)?;
            Command::SetProgram(program(p[0]).ok_or(bad)?)
        }
        4 => {
            need(17)?;
            let mut m = [0.0; 16];
            for (k, v) in m.iter_mut().enumerate() {
                *v = f(k + 1);
            }
            Command::SetUniformMat4(uniform(p[0]).ok_or(bad)?, m)
        }
        5 => {
            need(5)?;
            Command::SetUniformVec4(uniform(p[0]).ok_or(bad)?, [f(1), f(2), f(3), f(4)])
        }
        6 => {
            need(2)?;
            Command::SetUniformF32(uniform(p[0]).ok_or(bad)?, f(1))
        }
        7 => {
            need(1)?;
            Command::BindMesh(p[0])
        }
        8 => {
            need(2)?;
            Command::DrawIndexed {
                count: p[0],
                first: p[1],
            }
        }
        9 => {
            if p.is_empty() || (p.len() - 1) % 3 != 0 {
                return Err(DecodeError::BadLength { op, at });
            }
            let offsets = p[1..]
                .chunks(3)
                .map(|c| {
                    [
                        f32::from_bits(c[0]),
                        f32::from_bits(c[1]),
                        f32::from_bits(c[2]),
                    ]
                })
                .collect();
            Command::DrawInstanced {
                count: p[0],
                offsets,
            }
        }
        10 => {
            need(1)?;
            Command::SetBlend(match p[0] {
                0 => Blend::Off,
                1 => Blend::Alpha,
                2 => Blend::Additive,
                _ => return Err(bad),
            })
        }
        11 => {
            need(2)?;
            Command::SetDepth {
                test: p[0] != 0,
                write: p[1] != 0,
            }
        }
        _ => return Err(DecodeError::UnknownOp { op, at }),
    })
}

fn program(v: u32) -> Option<Program> {
    Some(match v {
        0 => Program::Unlit,
        1 => Program::Lit,
        2 => Program::Glow,
        3 => Program::Instanced,
        _ => return None,
    })
}

fn uniform(v: u32) -> Option<Uniform> {
    use Uniform::*;
    [
        ViewProj, Model, Color, LightDir, LightColor, Eye, Emissive, Scale,
    ]
    .get(v as usize)
    .copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn every_command() -> Vec<Command> {
        let mut m = [0.0f32; 16];
        for (i, v) in m.iter_mut().enumerate() {
            *v = i as f32 * 0.5;
        }
        vec![
            Command::Clear([0.1, 0.2, 0.3, 1.0]),
            Command::Viewport(640, 400),
            Command::SetProgram(Program::Unlit),
            Command::SetProgram(Program::Lit),
            Command::SetProgram(Program::Glow),
            Command::SetProgram(Program::Instanced),
            Command::SetUniformMat4(Uniform::ViewProj, m),
            Command::SetUniformVec4(Uniform::Color, [1.0, 0.5, 0.25, 0.75]),
            Command::SetUniformF32(Uniform::Scale, 0.085),
            Command::BindMesh(7),
            Command::DrawIndexed {
                count: 36,
                first: 0,
            },
            Command::DrawInstanced {
                count: 12,
                offsets: vec![[1.0, 2.0, 3.0], [-1.0, 0.0, 4.5]],
            },
            Command::DrawInstanced {
                count: 3,
                offsets: vec![],
            },
            Command::SetBlend(Blend::Off),
            Command::SetBlend(Blend::Alpha),
            Command::SetBlend(Blend::Additive),
            Command::SetDepth {
                test: true,
                write: false,
            },
        ]
    }

    fn encode(cs: &[Command]) -> CommandBuffer {
        let mut b = CommandBuffer::new();
        for c in cs {
            b.push(c);
        }
        b
    }

    /// Every command survives an encode and decode round trip unchanged.
    #[test]
    fn every_command_round_trips() {
        let cs = every_command();
        assert_eq!(CommandBuffer::decode(encode(&cs).words()), Ok(cs));
    }

    /// The header is the magic word then the version, and an empty buffer decodes to nothing.
    #[test]
    fn header_is_magic_then_version() {
        let b = CommandBuffer::new();
        assert_eq!(b.words(), &[MAGIC, VERSION]);
        assert_eq!(CommandBuffer::decode(b.words()), Ok(vec![]));
    }

    /// A short buffer, wrong magic or unknown version is rejected.
    #[test]
    fn bad_headers_are_rejected() {
        assert_eq!(
            CommandBuffer::decode(&[MAGIC]),
            Err(DecodeError::MissingHeader)
        );
        assert_eq!(
            CommandBuffer::decode(&[1, VERSION]),
            Err(DecodeError::BadMagic(1))
        );
        assert_eq!(
            CommandBuffer::decode(&[MAGIC, 9]),
            Err(DecodeError::UnsupportedVersion(9))
        );
    }

    /// An opcode this version does not define is an error naming the word.
    #[test]
    fn unknown_opcode_is_rejected() {
        assert_eq!(
            CommandBuffer::decode(&[MAGIC, VERSION, 42]),
            Err(DecodeError::UnknownOp { op: 42, at: 2 })
        );
    }

    /// A payload length that does not fit the opcode is an error.
    #[test]
    fn wrong_payload_length_is_rejected() {
        let bind_two = 7 | (2 << 8);
        assert_eq!(
            CommandBuffer::decode(&[MAGIC, VERSION, bind_two, 1, 2]),
            Err(DecodeError::BadLength { op: 7, at: 2 })
        );
        let instanced_bad = 9 | (3 << 8);
        assert_eq!(
            CommandBuffer::decode(&[MAGIC, VERSION, instanced_bad, 1, 2, 3]),
            Err(DecodeError::BadLength { op: 9, at: 2 })
        );
        assert_eq!(
            CommandBuffer::decode(&[MAGIC, VERSION, 9]),
            Err(DecodeError::BadLength { op: 9, at: 2 })
        );
    }

    /// A payload that runs past the end of the buffer is truncated.
    #[test]
    fn truncated_payload_is_rejected() {
        let clear = 1 | (4 << 8);
        assert_eq!(
            CommandBuffer::decode(&[MAGIC, VERSION, clear, 0, 0]),
            Err(DecodeError::Truncated)
        );
    }

    /// Out-of-range program, uniform and blend values are rejected.
    #[test]
    fn out_of_range_enum_values_are_rejected() {
        let bad = Err(DecodeError::BadValue { at: 2 });
        assert_eq!(
            CommandBuffer::decode(&[MAGIC, VERSION, 3 | (1 << 8), 4]),
            bad
        );
        assert_eq!(
            CommandBuffer::decode(&[MAGIC, VERSION, 6 | (2 << 8), 8, 0]),
            bad
        );
        assert_eq!(
            CommandBuffer::decode(&[MAGIC, VERSION, 10 | (1 << 8), 3]),
            bad
        );
    }

    /// Any nonzero depth word means true.
    #[test]
    fn depth_flags_treat_nonzero_as_true() {
        assert_eq!(
            CommandBuffer::decode(&[MAGIC, VERSION, 11 | (2 << 8), 5, 0]),
            Ok(vec![Command::SetDepth {
                test: true,
                write: false
            }])
        );
    }
}
