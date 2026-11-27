//! Native tests of the bridge: every export is called exactly as JS calls it.

use std::vec::Vec;

use super::*;
use crate::status::{sb_last_error_len, sb_last_error_ptr};
use simbiguity_circuits_view::default_camera;
use simbiguity_math::Vec3;
use simbiguity_render::{Command, CommandBuffer};

const W: u32 = 960;
const H: u32 = 600;

fn readouts(h: u32) -> Vec<f64> {
    let n = c3d_readouts(h);
    assert_eq!(n, READOUT_COUNT as i32);
    // SAFETY: the bridge just filled `n` f64s at this address.
    unsafe { std::slice::from_raw_parts(c3d_readouts_ptr(h), n as usize) }.to_vec()
}

fn frame(h: u32) -> Vec<u32> {
    let n = c3d_encode(h, W, H);
    assert!(n > 2);
    // SAFETY: the bridge just wrote `n` words at this address.
    unsafe { std::slice::from_raw_parts(c3d_commands_ptr(h), n as usize) }.to_vec()
}

fn error_message() -> std::string::String {
    // SAFETY: ptr/len describe the bridge's current error string.
    let bytes =
        unsafe { std::slice::from_raw_parts(sb_last_error_ptr(), sb_last_error_len() as usize) };
    std::string::String::from_utf8(bytes.to_vec()).unwrap()
}

/// A new view hands out a nonzero handle and reads back the state it was built with.
#[test]
fn new_view_reads_back_its_state() {
    let h = c3d_new(12.0, 1);
    assert_ne!(h, 0);
    let r = readouts(h);
    assert_eq!((r[0], r[1], r[9]), (1.0, 12.0, 1.0));
    assert!(
        (r[2] - 0.5).abs() < 1e-9,
        "12 V across a 12 V 6 W bulb is 0.5 A"
    );
    assert_eq!(c3d_free(h), OK);
}

/// A freed or never-issued handle is an error with a readable message.
#[test]
fn bad_handles_are_rejected() {
    let h = c3d_new(12.0, 0);
    assert_eq!(c3d_free(h), OK);
    assert_eq!(c3d_free(h), ERR_HANDLE);
    assert_eq!(c3d_toggle(h), ERR_HANDLE);
    assert_eq!(c3d_encode(0, W, H), ERR_HANDLE);
    assert!(c3d_commands_ptr(9999).is_null());
    assert!(c3d_readouts_ptr(9999).is_null());
    assert!(error_message().contains("no 3D view"));
    assert_eq!(last_error(), error_message());
}

/// Toggle reports the new switch state and volts, replace and seek reach the view.
#[test]
fn inputs_reach_the_view() {
    let h = c3d_new(12.0, 0);
    assert_eq!(c3d_toggle(h), 1);
    assert_eq!(c3d_set_volts(h, 24.0), OK);
    assert_eq!(readouts(h)[8], 1.0, "24 V blows the bulb");
    assert_eq!(c3d_set_volts(h, 12.0), OK);
    assert_eq!(c3d_replace_bulb(h), OK);
    assert_eq!(readouts(h)[8], 0.0);
    assert_eq!(c3d_seek(h, 2.5), OK);
    assert!((readouts(h)[13] - 2.5).abs() < 1e-9);
    // 0.1 s is 12 fixed steps (well under the 30-step catch-up cap).
    assert_eq!(c3d_advance(h, 0.1), OK);
    assert!((readouts(h)[13] - 2.6).abs() < 0.01);
    assert_eq!(c3d_toggle(h), 0);
    c3d_free(h);
}

/// The camera exports work in degrees and clamp to the camera's limits.
#[test]
fn camera_exports_use_degrees_and_clamp() {
    let h = c3d_new(12.0, 0);
    assert_eq!(c3d_set_camera(h, 90.0, 30.0, 8.0), OK);
    let r = readouts(h);
    assert!((r[10] - 90.0).abs() < 1e-9 && (r[11] - 30.0).abs() < 1e-9);
    assert_eq!(r[12], 8.0);
    assert_eq!(c3d_orbit(h, 10.0, 0.0), OK);
    assert!((readouts(h)[10] - 100.0).abs() < 1e-9);
    assert_eq!(c3d_zoom(h, 100.0), OK);
    assert_eq!(readouts(h)[12], 20.0, "max distance");
    assert_eq!(c3d_set_camera(h, 0.0, 0.0, 1.0), OK);
    assert_eq!(readouts(h)[12], 4.0, "min distance");
    c3d_free(h);
}

/// An encoded frame is a valid command buffer that sets the requested viewport.
#[test]
fn encode_writes_a_decodable_frame() {
    let h = c3d_new(12.0, 1);
    let cmds = CommandBuffer::decode(&frame(h)).unwrap();
    assert_eq!(cmds[0], Command::Viewport(W, H));
    assert!(cmds
        .iter()
        .any(|c| matches!(c, Command::DrawInstanced { .. })));
    c3d_free(h);
}

/// Picking at the pixel the switch base projects to hits the switch; a corner of the sky hits nothing.
#[test]
fn pick_hits_the_switch_and_misses_the_sky() {
    let h = c3d_new(12.0, 0);
    let (x, y) = default_camera()
        .project(Vec3::new(0.3, 0.1, -1.8), f64::from(W), f64::from(H))
        .unwrap();
    assert_eq!(c3d_pick(h, x, y, f64::from(W), f64::from(H)), 2);
    assert_eq!(c3d_pick(h, 2.0, 2.0, f64::from(W), f64::from(H)), 0);
    c3d_free(h);
}

/// Mesh exports describe every mesh, and an index past the end is an error.
#[test]
fn mesh_exports_cover_every_mesh() {
    let h = c3d_new(12.0, 0);
    let n = c3d_mesh_count(h);
    assert!(n > 10);
    for i in 0..n as u32 {
        let v = c3d_mesh_vertices_len(h, i);
        let ix = c3d_mesh_indices_len(h, i);
        assert_eq!(v % c3d_vertex_stride() as i32, 0);
        assert!(ix > 0 && ix % 3 == 0);
        assert!(!c3d_mesh_vertices_ptr(h, i).is_null());
        assert!(!c3d_mesh_indices_ptr(h, i).is_null());
    }
    assert_eq!(c3d_mesh_vertices_len(h, n as u32), ERR_INDEX);
    assert_eq!(c3d_mesh_indices_len(h, n as u32), ERR_INDEX);
    assert!(c3d_mesh_vertices_ptr(h, n as u32).is_null());
    assert!(c3d_mesh_indices_ptr(h, n as u32).is_null());
    assert!(error_message().contains("no mesh"));
    c3d_free(h);
}

/// Verifies that layout buffer unpacking works and bridge methods can instantiate components.
#[test]
fn layout_bridge_exports() {
    let buf = std::vec![
        2, // node_count
        1, // num_parts
        2, // kind=2 (switch)
        0, // a
        1, // b
        0, // x
        0, // z
        0, // rot
        0, // open
    ];

    let h = c3d_new_from_layout(buf.as_ptr(), buf.len() as u32);
    assert_ne!(h, 0);
    assert_eq!(c3d_part_count(h), 1);
    assert_eq!(c3d_part_kind(h, 0), 2); // switch
    assert_eq!(c3d_toggle_part(h, 0), 1); // toggles to closed
    c3d_free(h);
}

/// Verifies that the bridge safely rejects structurally invalid layout buffers.
#[test]
fn layout_bridge_rejects_malformed_buffer() {
    let buf = std::vec![2, 1, 99]; // Unknown component kind
    let h = c3d_new_from_layout(buf.as_ptr(), buf.len() as u32);
    assert_eq!(h, 0);
    assert!(error_message().contains("buffer too short") || error_message().contains("unknown"));

    let h2 = c3d_new_from_layout(std::ptr::null(), 100);
    assert_eq!(h2, 0);
    assert!(error_message().contains("invalid layout"));

    let h3 = c3d_new_from_layout(buf.as_ptr(), 999999);
    assert_eq!(h3, 0);
    assert!(error_message().contains("invalid layout"));
}
