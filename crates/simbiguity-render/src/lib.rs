#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
//! # simbiguity-render
//!
//! Renderer-agnostic 3D: Rust decides *what* to draw, a backend only issues
//! the GPU calls.
//!
//! - [`mesh`] — box, cylinder, sphere, capsule and tube-along-a-path
//!   generators, producing interleaved position + normal vertices.
//! - [`scene`] — meshes, nodes with transforms and materials, instanced
//!   particles, one directional light.
//! - [`camera`] — a perspective orbit camera: project points to pixels,
//!   cast rays from pixels.
//! - [`commands`] — the **draw-command buffer**: a versioned, documented
//!   `u32` word stream (`SetProgram`, `SetUniformMat4`, `BindMesh`,
//!   `DrawIndexed` …). The web executor (`site/js/webgl2-executor.js`)
//!   replays it with WebGL2; a native or software backend could replay the
//!   same words.
//! - [`pick`] — ray vs. bounding box, nearest pickable node wins.
//! - [`frame`] — compiles a scene + camera into one frame's commands.
//!
//! Geometry and transforms are `f64` (from `simbiguity-math`) until the
//! command buffer, which is `f32` — the GPU's native width.
//!
//! `#![no_std]` with `alloc`.
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;
#[cfg(any(test, feature = "std"))]
extern crate std;

pub mod camera;
pub mod commands;
pub mod decode;
pub mod frame;
pub mod mesh;
pub mod path;
pub mod pick;
pub mod scene;

pub use camera::OrbitCamera;
pub use commands::{Blend, Command, CommandBuffer, Program, Uniform};
pub use decode::DecodeError;
pub use frame::{encode_frame, FrameBuf};
pub use mesh::Mesh;
pub use pick::{Aabb, Hit, Ray};
pub use scene::{Instances, Light, Material, MeshId, Node, NodeId, Scene, Shading};
