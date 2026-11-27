use simbiguity_core::simulation::{Seconds, Simulation};
use simbiguity_math::vec::Vec3;
use simbiguity_physics::{
    constraints::Ground, error::PhysicsError, Integrator, PhysicsInput, World,
};
use simbiguity_physics_view::PhysicsView;
use simbiguity_render::{encode_frame, FrameBuf};
use simbiguity_render_wasm::status::{ERR_HANDLE, ERR_INDEX, OK};

fn fail(code: i32, _message: String) -> i32 {
    code
}

use std::cell::RefCell;
use std::vec::Vec;

pub struct Session {
    world: World,
    view: PhysicsView,
    commands: simbiguity_render::CommandBuffer,
    frame_buf: FrameBuf,
    time: f64,
}

thread_local! {
    static SLOTS: RefCell<Vec<Option<Session>>> = const { RefCell::new(Vec::new()) };
}

fn insert(session: Session) -> u32 {
    SLOTS.with(|slots| {
        let mut slots = slots.borrow_mut();
        for (i, slot) in slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(session);
                return (i as u32) + 1;
            }
        }
        slots.push(Some(session));
        slots.len() as u32
    })
}

fn with_session<F>(h: u32, f: F) -> i32
where
    F: FnOnce(&mut Session) -> i32,
{
    SLOTS.with(|slots| {
        let mut slots = slots.borrow_mut();
        let idx = (h as usize).wrapping_sub(1);
        if let Some(Some(session)) = slots.get_mut(idx) {
            f(session)
        } else {
            fail(ERR_HANDLE, std::format!("no session with handle {}", h))
        }
    })
}

fn map_error(e: PhysicsError) -> i32 {
    match e {
        PhysicsError::IndexOutOfBounds => fail(ERR_INDEX, "index out of bounds".into()),
        PhysicsError::InvalidParameter => fail(-2, "invalid parameter".into()),
        PhysicsError::InvalidTimeStep => fail(-2, "invalid time step".into()),
        _ => fail(-3, "physics error".into()),
    }
}

fn create_pendulum() -> Result<Session, PhysicsError> {
    let mut world = World::new();
    world.add_particle(1.0, Vec3::new(0.0, 5.0, 0.0), true)?;
    world.add_particle(1.0, Vec3::new(4.0, 5.0, 0.0), false)?;
    world.add_rod(0, 1, 4.0)?;
    Ok(Session {
        world,
        view: PhysicsView::new(),
        commands: simbiguity_render::CommandBuffer::new(),
        frame_buf: FrameBuf::new(),
        time: 0.0,
    })
}

#[no_mangle]
pub extern "C" fn phys_new_pendulum() -> u32 {
    match create_pendulum() {
        Ok(s) => insert(s),
        Err(e) => {
            map_error(e);
            0
        }
    }
}

fn create_spring_mass() -> Result<Session, PhysicsError> {
    let mut world = World::new();
    world.add_particle(1.0, Vec3::new(0.0, 5.0, 0.0), true)?;
    world.add_particle(1.0, Vec3::new(0.0, 2.0, 0.0), false)?;
    world.add_spring(0, 1, 2.0, 10.0, 0.5)?;
    Ok(Session {
        world,
        view: PhysicsView::new(),
        commands: simbiguity_render::CommandBuffer::new(),
        frame_buf: FrameBuf::new(),
        time: 0.0,
    })
}

#[no_mangle]
pub extern "C" fn phys_new_spring_mass() -> u32 {
    match create_spring_mass() {
        Ok(s) => insert(s),
        Err(e) => {
            map_error(e);
            0
        }
    }
}

fn create_projectile() -> Result<Session, PhysicsError> {
    let mut world = World::new();
    world.ground = Some(Ground {
        y: 0.0,
        restitution: 0.25,
    });
    let p = world.add_particle(1.0, Vec3::new(0.0, 0.05, 0.0), false)?;
    world.particles[p].velocity = Vec3::new(10.0, 10.0, 0.0);
    Ok(Session {
        world,
        view: PhysicsView::new(),
        commands: simbiguity_render::CommandBuffer::new(),
        frame_buf: FrameBuf::new(),
        time: 0.0,
    })
}

#[no_mangle]
pub extern "C" fn phys_new_projectile() -> u32 {
    match create_projectile() {
        Ok(s) => insert(s),
        Err(e) => {
            map_error(e);
            0
        }
    }
}

fn create_chain() -> Result<Session, PhysicsError> {
    let mut world = World::new();
    world.add_particle(1.0, Vec3::new(0.0, 5.0, 0.0), true)?;
    for i in 1..5 {
        world.add_particle(1.0, Vec3::new(i as f64, 5.0, 0.0), false)?;
        world.add_rod(i - 1, i, 1.0)?;
    }
    Ok(Session {
        world,
        view: PhysicsView::new(),
        commands: simbiguity_render::CommandBuffer::new(),
        frame_buf: FrameBuf::new(),
        time: 0.0,
    })
}

#[no_mangle]
pub extern "C" fn phys_new_chain() -> u32 {
    match create_chain() {
        Ok(s) => insert(s),
        Err(e) => {
            map_error(e);
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn phys_set_integrator(h: u32, kind: u32) -> i32 {
    with_session(h, |s| {
        s.world.integrator = match kind {
            0 => Integrator::VelocityVerlet,
            1 => Integrator::SymplecticEuler,
            2 => Integrator::ExplicitEuler,
            _ => return fail(-2, "invalid integrator".into()),
        };
        OK
    })
}

#[no_mangle]
pub extern "C" fn phys_push(h: u32, index: u32, fx: f64, fy: f64, fz: f64) -> i32 {
    with_session(h, |s| {
        s.world.apply(PhysicsInput::Push {
            index: index as usize,
            force: Vec3::new(fx, fy, fz),
        });
        OK
    })
}

#[no_mangle]
pub extern "C" fn phys_advance(h: u32, dt: f64) -> i32 {
    if dt.is_nan() || dt <= 0.0 {
        return fail(-2, "invalid time step".into());
    }
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

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn phys_get_energy(h: u32, out_ptr: *mut f64) -> i32 {
    with_session(h, |s| {
        let e = s.world.compute_energy();
        unsafe {
            *out_ptr.add(0) = e.kinetic;
            *out_ptr.add(1) = e.potential_gravity;
            *out_ptr.add(2) = e.potential_spring;
            *out_ptr.add(3) = e.total();
        }
        OK
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pendulum preset is what a learner sees first. It must build two
    /// particles (a pinned anchor and a bob) and keep swinging for 10 seconds
    /// of frames without crashing the page.
    #[test]
    fn pendulum_preset_steps_without_panicking() {
        let h = phys_new_pendulum();
        assert_ne!(h, 0);
        for _ in 0..600 {
            assert!(phys_advance(h, 1.0 / 60.0) >= 0);
        }
    }
}
