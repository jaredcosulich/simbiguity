#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use simbiguity_core::simulation::{Seconds, Simulation};
use simbiguity_math::vec::Vec3;
use simbiguity_physics::World;
use simbiguity_physics_view::PhysicsView;
use simbiguity_render::{encode_frame, FrameBuf};
use simbiguity_render_wasm::status::{ERR_HANDLE, OK};

// Minimal slot mapping for handles
static mut SLOTS: Option<Vec<Option<Session>>> = None;

struct Session {
    world: World,
    view: PhysicsView,
    commands: simbiguity_render::CommandBuffer,
    frame_buf: FrameBuf,
    time: f64,
}

#[allow(static_mut_refs)]
unsafe fn slots() -> &'static mut Vec<Option<Session>> {
    if SLOTS.is_none() {
        SLOTS = Some(Vec::new());
    }
    SLOTS.as_mut().unwrap()
}

fn insert(session: Session) -> u32 {
    let s = unsafe { slots() };
    for (i, slot) in s.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(session);
            return (i as u32) + 1;
        }
    }
    s.push(Some(session));
    s.len() as u32
}

fn with_session<F>(h: u32, f: F) -> i32
where
    F: FnOnce(&mut Session) -> i32,
{
    let idx = (h as usize).wrapping_sub(1);
    let s = unsafe { slots() };
    if let Some(Some(session)) = s.get_mut(idx) {
        return f(session);
    }
    ERR_HANDLE
}

#[no_mangle]
pub extern "C" fn phys_new_pendulum() -> u32 {
    let mut world = World::new();
    let _ = world.add_particle(0.0, Vec3::new(0.0, 5.0, 0.0), true);
    let _ = world.add_particle(1.0, Vec3::new(4.0, 5.0, 0.0), false);
    let _ = world.add_rod(0, 1, 4.0);
    insert(Session {
        world,
        view: PhysicsView::new(),
        commands: simbiguity_render::CommandBuffer::new(),
        frame_buf: FrameBuf::new(),
        time: 0.0,
    })
}

#[no_mangle]
pub extern "C" fn phys_new_spring_mass() -> u32 {
    let mut world = World::new();
    let _ = world.add_particle(0.0, Vec3::new(0.0, 5.0, 0.0), true);
    let _ = world.add_particle(1.0, Vec3::new(0.0, 2.0, 0.0), false);
    let _ = world.add_spring(0, 1, 2.0, 10.0, 0.5);
    insert(Session {
        world,
        view: PhysicsView::new(),
        commands: simbiguity_render::CommandBuffer::new(),
        frame_buf: FrameBuf::new(),
        time: 0.0,
    })
}

#[no_mangle]
pub extern "C" fn phys_advance(h: u32, dt: f64) -> i32 {
    with_session(h, |s| {
        s.world.step(Seconds(dt));
        s.time += dt;
        OK
    })
}

#[no_mangle]
pub extern "C" fn phys_encode(h: u32, width: u32, height: u32) -> i32 {
    with_session(h, |s| {
        s.view.update(&s.world);
        s.commands.clear();
        encode_frame(
            &s.view.scene,
            &s.view.camera,
            width,
            height,
            &mut s.frame_buf,
            &mut s.commands,
        );
        s.commands.words().len() as i32
    })
}

#[no_mangle]
pub extern "C" fn phys_commands_ptr(h: u32) -> *const u32 {
    let mut ptr = core::ptr::null();
    with_session(h, |s| {
        ptr = s.commands.words().as_ptr();
        OK
    });
    ptr
}

#[no_mangle]
pub extern "C" fn phys_mesh_count(h: u32) -> i32 {
    with_session(h, |s| s.view.scene.meshes.len() as i32)
}

#[no_mangle]
pub extern "C" fn phys_mesh_vertices_ptr(h: u32, i: u32) -> *const f32 {
    let mut ptr = core::ptr::null();
    with_session(h, |s| {
        if let Some(m) = s.view.scene.meshes.get(i as usize) {
            ptr = m.vertices.as_ptr();
            OK
        } else {
            -1
        }
    });
    ptr
}

#[no_mangle]
pub extern "C" fn phys_mesh_vertices_len(h: u32, i: u32) -> i32 {
    let mut len = 0;
    with_session(h, |s| {
        if let Some(m) = s.view.scene.meshes.get(i as usize) {
            len = m.vertices.len() as i32;
            OK
        } else {
            -1
        }
    });
    len
}

#[no_mangle]
pub extern "C" fn phys_mesh_indices_ptr(h: u32, i: u32) -> *const u32 {
    let mut ptr = core::ptr::null();
    with_session(h, |s| {
        if let Some(m) = s.view.scene.meshes.get(i as usize) {
            ptr = m.indices.as_ptr();
            OK
        } else {
            -1
        }
    });
    ptr
}

#[no_mangle]
pub extern "C" fn phys_mesh_indices_len(h: u32, i: u32) -> i32 {
    let mut len = 0;
    with_session(h, |s| {
        if let Some(m) = s.view.scene.meshes.get(i as usize) {
            len = m.indices.len() as i32;
            OK
        } else {
            -1
        }
    });
    len
}

#[no_mangle]
pub extern "C" fn phys_set_camera(h: u32, yaw_deg: f64, pitch_deg: f64, distance: f64) -> i32 {
    with_session(h, |s| {
        let c = &mut s.view.camera;
        c.yaw = yaw_deg.to_radians();
        c.pitch = pitch_deg.to_radians();
        c.distance = distance;
        c.clamp();
        OK
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    // Verifies physics wasm init.
    #[test]
    fn test_wasm() {
        let handle = phys_new_pendulum();
        assert!(handle > 0);
    }
}
