#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
//! # simbiguity-render-wasm
//!
//! A hand-written bridge from JavaScript to simbiguity's 3D views, in the
//! same style as `simbiguity-circuits-wasm`: plain `extern "C"` functions
//! over numbers, opaque `u32` handles, status codes, and shared linear
//! memory for bulk data.
//!
//! **The contract.**
//! - `c3d_new` builds a 3D hello circuit and returns its handle (never `0`).
//! - Meshes are static: read them once with `c3d_mesh_*` and upload them.
//! - Each frame: `c3d_advance(dt)` (or `c3d_seek(t)` for a fixed moment),
//!   then `c3d_encode(w, h)` returns the word count of a draw-command
//!   buffer at `c3d_commands_ptr` (layout: `simbiguity_render::commands`).
//!   `site/js/webgl2-executor.js` replays it.
//! - `c3d_pick(x, y, w, h)` returns what is under a pixel (`0` nothing,
//!   `1` battery, `2` switch, `3` bulb).
//! - Calls that can fail return `i32 < 0` with a message at
//!   `sb_last_error_ptr()/sb_last_error_len()`.

pub mod readouts;
mod slot;
pub mod status;
#[cfg(test)]
mod tests;

use simbiguity_circuits_view::CircuitView;
use simbiguity_render::mesh::VERTEX_STRIDE;

pub use readouts::{pack_readouts, READOUT_COUNT};
use slot::{with_mesh, with_view};
pub use status::{last_error, ERR_HANDLE, ERR_INDEX, OK};

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn sb_malloc(size: u32) -> *mut u8 {
    let mut vec = std::vec::Vec::with_capacity(size as usize);
    let ptr = vec.as_mut_ptr();
    core::mem::forget(vec);
    ptr
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn sb_free(ptr: *mut u8, size: u32) {
    unsafe {
        let _ = std::vec::Vec::from_raw_parts(ptr, 0, size as usize);
    }
}

/// Builds a 3D hello circuit at `volts` with the switch closed when
/// `closed != 0`; returns its handle.
#[no_mangle]
pub extern "C" fn c3d_new(volts: f64, closed: i32) -> u32 {
    slot::insert(CircuitView::new(volts, closed != 0))
}

/// Maximum length of a layout buffer, in words.
pub const MAX_LAYOUT_WORDS: usize = 65536;

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn c3d_new_from_layout(ptr: *const u32, len: u32) -> u32 {
    if ptr.is_null() || len as usize > MAX_LAYOUT_WORDS {
        let _ = status::fail(-2, "invalid layout buffer".into());
        return 0;
    }
    // SAFETY: The caller guarantees `ptr` points to `len` consecutive valid `u32`s
    // allocated by `sb_malloc`. We enforce a non-null pointer and a maximum length
    // to bound the slice within reasonable memory limits.
    let words = unsafe { core::slice::from_raw_parts(ptr, len as usize) };
    match simbiguity_circuits_view::board_layout::BoardLayout::unpack(words) {
        Ok((circuit, layout)) => match CircuitView::from_layout(circuit, layout) {
            Ok(view) => slot::insert(view),
            Err(e) => {
                let _ = status::fail(-2, e.as_str().into());
                0
            }
        },
        Err(e) => {
            let _ = status::fail(-2, e.as_str().into());
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn c3d_toggle_part(h: u32, id: u32) -> i32 {
    with_view(h, |s| i32::from(s.view.toggle_part_by_index(id as usize)))
}

#[no_mangle]
pub extern "C" fn c3d_part_count(h: u32) -> i32 {
    with_view(h, |s| s.view.circuit().parts().len() as i32)
}

#[no_mangle]
pub extern "C" fn c3d_part_kind(h: u32, id: u32) -> i32 {
    with_view(h, |s| {
        let c = s.view.circuit();
        if id as usize >= c.parts().len() {
            return 0;
        }
        match c.parts()[id as usize].component {
            simbiguity_circuits_view::simbiguity_circuits::Component::Battery { .. } => 1,
            simbiguity_circuits_view::simbiguity_circuits::Component::Switch { .. } => 2,
            simbiguity_circuits_view::simbiguity_circuits::Component::Bulb { .. } => 3,
            simbiguity_circuits_view::simbiguity_circuits::Component::Wire => 4,
            _ => 0,
        }
    })
}

/// Frees a view. Freeing it again is [`ERR_HANDLE`].
#[no_mangle]
pub extern "C" fn c3d_free(h: u32) -> i32 {
    slot::remove(h)
}

/// Flips the switch; returns `1` if now closed, `0` if open.
#[no_mangle]
pub extern "C" fn c3d_toggle(h: u32) -> i32 {
    with_view(h, |s| i32::from(s.view.toggle_switch()))
}

/// Sets the battery voltage.
#[no_mangle]
pub extern "C" fn c3d_set_volts(h: u32, volts: f64) -> i32 {
    with_view(h, |s| {
        s.view.set_volts(volts);
        OK
    })
}

/// Screws in a fresh bulb.
#[no_mangle]
pub extern "C" fn c3d_replace_bulb(h: u32) -> i32 {
    with_view(h, |s| {
        s.view.replace_bulb();
        OK
    })
}

/// Advances the animation by `dt` seconds of wall-clock time.
#[no_mangle]
pub extern "C" fn c3d_advance(h: u32, dt: f64) -> i32 {
    with_view(h, |s| {
        s.view.advance(dt);
        OK
    })
}

/// Restarts the animation and runs it to exactly `t` seconds.
#[no_mangle]
pub extern "C" fn c3d_seek(h: u32, t: f64) -> i32 {
    with_view(h, |s| {
        s.view.seek(t);
        OK
    })
}

/// Places the camera: yaw and pitch in degrees, distance in board units
/// (clamped to the camera's limits).
#[no_mangle]
pub extern "C" fn c3d_set_camera(h: u32, yaw_deg: f64, pitch_deg: f64, distance: f64) -> i32 {
    with_view(h, |s| {
        let c = &mut s.view.camera;
        c.yaw = yaw_deg.to_radians();
        c.pitch = pitch_deg.to_radians();
        c.distance = distance;
        c.clamp();
        OK
    })
}

/// Orbits the camera by degrees (pitch is clamped).
#[no_mangle]
pub extern "C" fn c3d_orbit(h: u32, d_yaw_deg: f64, d_pitch_deg: f64) -> i32 {
    with_view(h, |s| {
        s.view
            .camera
            .orbit(d_yaw_deg.to_radians(), d_pitch_deg.to_radians());
        OK
    })
}

/// Multiplies the camera distance by `factor` (clamped).
#[no_mangle]
pub extern "C" fn c3d_zoom(h: u32, factor: f64) -> i32 {
    with_view(h, |s| {
        s.view.camera.zoom(factor);
        OK
    })
}

/// Encodes the current frame; returns its length in `u32` words.
#[no_mangle]
pub extern "C" fn c3d_encode(h: u32, width: u32, height: u32) -> i32 {
    with_view(h, |s| s.view.encode(width, height).len() as i32)
}

/// Address of the frame [`c3d_encode`] wrote. Valid until the next call
/// into this view.
#[no_mangle]
pub extern "C" fn c3d_commands_ptr(h: u32) -> *const u32 {
    let mut ptr = core::ptr::null();
    with_view(h, |s| {
        ptr = s.view.commands().as_ptr();
        OK
    });
    ptr
}

/// What is under pixel `(x, y)` of a `width × height` view: `0` nothing,
/// otherwise a pick tag (`1` battery, `2` switch, `3` bulb).
#[no_mangle]
pub extern "C" fn c3d_pick(h: u32, x: f64, y: f64, width: f64, height: f64) -> i32 {
    with_view(h, |s| {
        s.view.pick(x, y, width, height).map_or(0, |t| t as i32)
    })
}

/// Refreshes the readouts buffer; returns its length ([`READOUT_COUNT`]).
#[no_mangle]
pub extern "C" fn c3d_readouts(h: u32) -> i32 {
    with_view(h, |s| {
        s.readouts = pack_readouts(&s.view.readouts(), &s.view.camera, s.view.time());
        READOUT_COUNT as i32
    })
}

/// Address of the readouts [`c3d_readouts`] filled.
#[no_mangle]
pub extern "C" fn c3d_readouts_ptr(h: u32) -> *const f64 {
    let mut ptr = core::ptr::null();
    with_view(h, |s| {
        ptr = s.readouts.as_ptr();
        OK
    });
    ptr
}

/// Number of meshes in the view's scene.
#[no_mangle]
pub extern "C" fn c3d_mesh_count(h: u32) -> i32 {
    with_view(h, |s| s.view.scene().meshes.len() as i32)
}

/// Address of mesh `i`'s interleaved vertices (`f32`: position, normal).
#[no_mangle]
pub extern "C" fn c3d_mesh_vertices_ptr(h: u32, i: u32) -> *const f32 {
    let mut ptr = core::ptr::null();
    with_mesh(h, i, |m| {
        ptr = m.vertices.as_ptr();
        OK
    });
    ptr
}

/// Number of `f32`s at [`c3d_mesh_vertices_ptr`] (6 per vertex).
#[no_mangle]
pub extern "C" fn c3d_mesh_vertices_len(h: u32, i: u32) -> i32 {
    with_mesh(h, i, |m| m.vertices.len() as i32)
}

/// Address of mesh `i`'s `u32` triangle indices.
#[no_mangle]
pub extern "C" fn c3d_mesh_indices_ptr(h: u32, i: u32) -> *const u32 {
    let mut ptr = core::ptr::null();
    with_mesh(h, i, |m| {
        ptr = m.indices.as_ptr();
        OK
    });
    ptr
}

/// Number of indices at [`c3d_mesh_indices_ptr`].
#[no_mangle]
pub extern "C" fn c3d_mesh_indices_len(h: u32, i: u32) -> i32 {
    with_mesh(h, i, |m| m.indices.len() as i32)
}

/// `f32`s per vertex in the mesh buffers.
#[no_mangle]
pub extern "C" fn c3d_vertex_stride() -> u32 {
    VERTEX_STRIDE as u32
}
