#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
use simbiguity_math::vec::Vec3;
use simbiguity_physics::{
    constraints::Ground, error::PhysicsError, Integrator, PhysicsInput, World,
};
use simbiguity_physics_view::PhysicsView;
use simbiguity_render::{encode_frame, FrameBuf};
use simbiguity_render_wasm::status::{fail, ERR_HANDLE, ERR_INDEX, OK};

use std::cell::RefCell;
use std::vec::Vec;

pub struct Session {
    world: World,
    view: PhysicsView,
    commands: simbiguity_render::CommandBuffer,
    frame_buf: FrameBuf,
    time: f64,
    results: [f64; 4],
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
            fail(ERR_HANDLE, "invalid handle".into())
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
    let e = world.compute_energy();
    Ok(Session {
        world,
        view: PhysicsView::new(),
        commands: simbiguity_render::CommandBuffer::new(),
        frame_buf: FrameBuf::new(),
        time: 0.0,
        results: [
            e.kinetic,
            e.potential_gravity,
            e.potential_spring,
            e.total(),
        ],
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
    let e = world.compute_energy();
    Ok(Session {
        world,
        view: PhysicsView::new(),
        commands: simbiguity_render::CommandBuffer::new(),
        frame_buf: FrameBuf::new(),
        time: 0.0,
        results: [
            e.kinetic,
            e.potential_gravity,
            e.potential_spring,
            e.total(),
        ],
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
        friction: 0.0,
    });
    let p = world.add_particle(1.0, Vec3::new(0.0, 0.05, 0.0), false)?;
    world.particles[p].velocity = Vec3::new(10.0, 10.0, 0.0);
    let e = world.compute_energy();
    Ok(Session {
        world,
        view: PhysicsView::new(),
        commands: simbiguity_render::CommandBuffer::new(),
        frame_buf: FrameBuf::new(),
        time: 0.0,
        results: [
            e.kinetic,
            e.potential_gravity,
            e.potential_spring,
            e.total(),
        ],
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
    let e = world.compute_energy();
    Ok(Session {
        world,
        view: PhysicsView::new(),
        commands: simbiguity_render::CommandBuffer::new(),
        frame_buf: FrameBuf::new(),
        time: 0.0,
        results: [
            e.kinetic,
            e.potential_gravity,
            e.potential_spring,
            e.total(),
        ],
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
pub extern "C" fn phys_new_cradle() -> u32 {
    let mut world = World::new();
    world.gravity.acceleration = Vec3::ZERO;
    for i in 0..5 {
        let _ = world.add_ball(1.0, Vec3::new(i as f64 * 0.2, 0.0, 0.0), 0.1, 1.0);
    }
    world.particles[0].position.x -= 1.0;
    world.particles[0].velocity.x = 1.0;
    let e = world.compute_energy();
    insert(Session {
        world,
        view: PhysicsView::new(),
        commands: simbiguity_render::CommandBuffer::new(),
        frame_buf: FrameBuf::new(),
        time: 0.0,
        results: [
            e.kinetic,
            e.potential_gravity,
            e.potential_spring,
            e.total(),
        ],
    })
}

#[no_mangle]
pub extern "C" fn phys_new_ball_pit(count: u32) -> u32 {
    let mut world = World::new();
    world.setup_deterministic_ball_pit(count);

    let e = world.compute_energy();
    insert(Session {
        world,
        view: PhysicsView::new(),
        commands: simbiguity_render::CommandBuffer::new(),
        frame_buf: FrameBuf::new(),
        time: 0.0,
        results: [
            e.kinetic,
            e.potential_gravity,
            e.potential_spring,
            e.total(),
        ],
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
    with_session(h, |s| {
        if let Err(e) = s.world.step(dt) {
            return map_error(e);
        }
        s.time += dt;
        let e = s.world.compute_energy();
        s.results = [
            e.kinetic,
            e.potential_gravity,
            e.potential_spring,
            e.total(),
        ];
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
pub extern "C" fn phys_results_ptr(h: u32) -> *const f64 {
    let mut ptr = core::ptr::null();
    with_session(h, |s| {
        ptr = s.results.as_ptr();
        OK
    });
    ptr
}

#[no_mangle]
pub extern "C" fn phys_free(h: u32) -> i32 {
    SLOTS.with(|slots| {
        let mut slots = slots.borrow_mut();
        match (h as usize).checked_sub(1).and_then(|i| slots.get_mut(i)) {
            Some(slot @ Some(_)) => {
                *slot = None;
                OK
            }
            _ => fail(ERR_HANDLE, "invalid handle".into()),
        }
    })
}

fn create_orbit() -> Result<Session, PhysicsError> {
    let mut world = World::new();
    world.setup_orbit()?;
    let e = world.compute_energy();
    Ok(Session {
        world,
        view: PhysicsView::new(),
        commands: simbiguity_render::CommandBuffer::new(),
        frame_buf: FrameBuf::new(),
        time: 0.0,
        results: [
            e.kinetic,
            e.potential_gravity,
            e.potential_spring,
            e.total(),
        ],
    })
}

#[no_mangle]
pub extern "C" fn phys_new_orbit() -> u32 {
    match create_orbit() {
        Ok(s) => insert(s),
        Err(e) => {
            map_error(e);
            0
        }
    }
}

fn create_binary() -> Result<Session, PhysicsError> {
    let mut world = World::new();
    world.setup_binary()?;
    let e = world.compute_energy();
    Ok(Session {
        world,
        view: PhysicsView::new(),
        commands: simbiguity_render::CommandBuffer::new(),
        frame_buf: FrameBuf::new(),
        time: 0.0,
        results: [
            e.kinetic,
            e.potential_gravity,
            e.potential_spring,
            e.total(),
        ],
    })
}

#[no_mangle]
pub extern "C" fn phys_new_binary() -> u32 {
    match create_binary() {
        Ok(s) => insert(s),
        Err(e) => {
            map_error(e);
            0
        }
    }
}

fn create_figure_eight() -> Result<Session, PhysicsError> {
    let mut world = World::new();
    world.setup_figure_eight()?;
    let e = world.compute_energy();
    Ok(Session {
        world,
        view: PhysicsView::new(),
        commands: simbiguity_render::CommandBuffer::new(),
        frame_buf: FrameBuf::new(),
        time: 0.0,
        results: [
            e.kinetic,
            e.potential_gravity,
            e.potential_spring,
            e.total(),
        ],
    })
}

#[no_mangle]
pub extern "C" fn phys_new_figure_eight() -> u32 {
    match create_figure_eight() {
        Ok(s) => insert(s),
        Err(e) => {
            map_error(e);
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pendulum_preset_steps_without_panicking() {
        let h = phys_new_pendulum();
        assert_ne!(h, 0);
        for _ in 0..600 {
            assert!(phys_advance(h, 1.0 / 60.0) >= 0);
        }
        phys_free(h);
    }

    /// Verifies the bridge stress conditions by ensuring heavy physics loads do not cause WASM panics.
    #[test]
    fn bridge_stress_test() {
        struct Lcg(u32);
        impl Lcg {
            fn next(&mut self) -> u32 {
                self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
                self.0
            }
            fn chance(&mut self, p: f64) -> bool {
                (self.next() as f64 / u32::MAX as f64) < p
            }
        }
        let mut rng = Lcg(0x1337);
        for _ in 0..10000 {
            let op = rng.next() % 6;
            let h = if rng.chance(0.8) {
                phys_new_pendulum()
            } else {
                0
            };
            match op {
                0 => {
                    phys_advance(h, 0.016);
                }
                1 => {
                    phys_free(h);
                }
                2 => {
                    phys_push(h, 0, 1.0, 0.0, 0.0);
                }
                3 => {
                    phys_encode(h, 800, 600);
                }
                4 => {
                    phys_results_ptr(h);
                }
                5 => {
                    phys_mesh_count(h);
                }
                _ => unreachable!(),
            }
        }
    }

    /// Verifies orbit returns to start
    #[test]
    fn orbit_returns_to_start() {
        let h = phys_new_orbit();
        assert_ne!(h, 0);
        let dt = 1e-3;
        let steps = (core::f64::consts::TAU / dt) as usize;
        for _ in 0..steps {
            phys_advance(h, dt);
        }
        with_session(h, |s| {
            let pos = s.world.particles[0].position;
            assert!((pos.x - 1.0).abs() < 1e-3);
            assert!((pos.y - 0.0).abs() < 1e-3);
            OK
        });
        phys_free(h);
    }

    /// Verifies binary system returns to start
    #[test]
    fn binary_returns_to_start() {
        let h = phys_new_binary();
        assert_ne!(h, 0);
        let dt = 1e-3;
        // Separation = 2, G=1, mass=1. Orbit velocity is 0.5. Period = 2*pi*r / v = 2*pi*1 / 0.5 = 4*pi.
        let steps = (4.0 * core::f64::consts::PI / dt) as usize;
        for _ in 0..steps {
            phys_advance(h, dt);
        }
        with_session(h, |s| {
            let pos = s.world.particles[0].position;
            assert!((pos.x - 1.0).abs() < 1e-2);
            assert!((pos.y - 0.0).abs() < 1e-2);
            OK
        });
        phys_free(h);
    }

    /// Verifies figure eight returns to start
    #[test]
    fn figure_eight_returns_to_start() {
        let h = phys_new_figure_eight();
        assert_ne!(h, 0);
        let dt = 1e-3;
        let t = 6.32591; // from plan
        let steps = (t / dt) as usize;
        for _ in 0..steps {
            phys_advance(h, dt);
        }
        with_session(h, |s| {
            let pos = s.world.particles[0].position;
            assert!((pos.x - 0.97000436).abs() < 1e-2);
            assert!((pos.y - -0.24308753).abs() < 1e-2);
            OK
        });
        phys_free(h);
    }

    /// Verifies that Verlet integrator preserves energy better than Explicit Euler over time
    #[test]
    fn orbit_preset_energy_drift_euler_vs_verlet() {
        let euler = phys_new_orbit();
        phys_set_integrator(euler, 2); // 2 = explicit euler

        let verlet = phys_new_orbit();
        phys_set_integrator(verlet, 0); // 0 = verlet (default)

        let dt = 0.01;
        let steps = 1000;

        let mut euler_drift = 0.0;
        let mut verlet_drift = 0.0;

        let mut e0_euler = 0.0;
        with_session(euler, |s| {
            e0_euler = s.world.compute_energy().total();
            OK
        });
        for _ in 0..steps {
            phys_advance(euler, dt);
        }
        with_session(euler, |s| {
            let e1 = s.world.compute_energy().total();
            euler_drift = (e1 - e0_euler).abs();
            OK
        });

        let mut e0_verlet = 0.0;
        with_session(verlet, |s| {
            e0_verlet = s.world.compute_energy().total();
            OK
        });
        for _ in 0..steps {
            phys_advance(verlet, dt);
        }
        with_session(verlet, |s| {
            let e1 = s.world.compute_energy().total();
            verlet_drift = (e1 - e0_verlet).abs();
            OK
        });

        phys_free(euler);
        phys_free(verlet);

        assert!(
            verlet_drift < euler_drift,
            "Verlet drift {} should be less than Euler drift {}",
            verlet_drift,
            euler_drift
        );
    }
}
