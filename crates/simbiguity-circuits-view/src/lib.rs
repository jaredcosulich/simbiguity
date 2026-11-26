#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
//! # simbiguity-circuits-view
//!
//! The hello circuit in 3D: a battery, a switch, a bulb and three copper
//! wires on a wooden board, with current drawn as particles flowing along
//! the wires and the bulb glowing as bright as the engine says it is.
//!
//! **Animation follows simulation, never the reverse.** Every visual is a
//! pure function of the solved circuit:
//! - particle speed is [`particle_speed`] of the battery current (zero
//!   current → the particles stand still);
//! - the bulb's glow follows its solved brightness through a critically
//!   damped [`Spring`], so it warms up and cools down instead of snapping;
//! - the switch lever swings on a spring toward open or closed.
//!
//! Time advances through a [`FixedClock`] at [`STEP`], so the same inputs
//! give the same frame on every machine, and [`CircuitView::seek`] can jump
//! to an exact moment for pixel-stable captures.
#![allow(missing_docs, dead_code, unused_imports, unused_variables)]
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;
#[cfg(any(test, feature = "std"))]
extern crate std;

pub mod board;
pub mod board_layout;
pub mod layout;
pub mod look;
#[cfg(test)]
mod view_tests;

use board_layout::BoardLayout;
use simbiguity_anim::Spring;
use simbiguity_circuits::{Circuit, CircuitInput, CircuitSim, Component, ComponentId};
use simbiguity_core::{FixedClock, Seconds, Simulation};
use simbiguity_math::Vec3;
use simbiguity_render::{encode_frame, CommandBuffer, Material, OrbitCamera, Scene};

use board::{build_scene, Parts};
use layout::{current_loop, knob_transform, lever_transform, particle_offsets, CurrentLoop};
use look::{bulb_look, charge_look, Surface};

/// The bulb's rating: 12 V, 6 W (so 0.5 A at full brightness).
pub const BULB_VOLTS: f64 = 12.0;
/// The bulb's rated power.
pub const BULB_WATTS: f64 = 6.0;
/// Particle speed per amp, in board units per second.
pub const SPEED_PER_AMP: f64 = 2.4;
/// The fixed animation step.
pub const STEP: f64 = 1.0 / 120.0;
/// How many charge particles ride the loop.
pub const PARTICLES: usize = 36;

/// How fast charge particles move for a current of `amps`. Proportional,
/// signed (negative flows backward), and exactly zero for zero current.
pub fn particle_speed(amps: f64) -> f64 {
    amps * SPEED_PER_AMP
}

/// The numbers a page shows next to the 3D view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Readouts {
    /// Switch closed?
    pub closed: bool,
    /// Battery EMF, volts.
    pub battery_volts: f64,
    /// Loop current, amps.
    pub current: f64,
    /// Voltage across the bulb.
    pub bulb_volts: f64,
    /// Power the bulb absorbs, watts.
    pub bulb_power: f64,
    /// Solved brightness `0..=1`.
    pub brightness: f64,
    /// Displayed glow `0..=1` (brightness through the spring).
    pub glow: f64,
    /// Particle speed, units per second.
    pub particle_speed: f64,
    /// Is the bulb blown?
    pub blown: bool,
    /// Did the circuit have a solution?
    pub solved: bool,
}

/// The 3D hello circuit: simulation, animation state, scene and camera.
#[derive(Debug, Clone)]
pub struct CircuitView {
    sim: CircuitSim,
    layout: BoardLayout,
    scene: Scene,
    parts: Parts,
    /// The camera the frame is drawn through.
    pub camera: OrbitCamera,
    path: CurrentLoop,
    glows: alloc::vec::Vec<Spring>,
    levers: alloc::vec::Vec<Spring>,
    flashes: alloc::vec::Vec<Spring>,
    phase: f64,
    time: f64,
    clock: FixedClock,
    commands: CommandBuffer,
    frame_buf: simbiguity_render::FrameBuf,
}

impl CircuitView {
    pub fn new(volts: f64, closed: bool) -> CircuitView {
        let mut c = Circuit::new();
        let (plus, minus, top) = (c.node(), c.node(), c.node());
        let battery = c.add_battery(plus, minus, volts);
        let switch = c.add_switch(plus, top, closed);
        let bulb = c.add_bulb(top, minus, BULB_VOLTS, BULB_WATTS);
        let (w1, w2, w3) = (
            c.connect(plus, plus),
            c.connect(top, top),
            c.connect(minus, minus),
        ); // Fake components
        let layout = BoardLayout::hello_layout(battery, switch, bulb, [w1, w2, w3]);
        Self::from_layout(c, layout)
    }

    pub fn from_layout(circuit: Circuit, layout: BoardLayout) -> CircuitView {
        let (scene, parts, _) = build_scene(&circuit, &layout);
        let num_switches = parts.levers.len();
        let num_bulbs = parts.glasses.len();
        let mut view = CircuitView {
            sim: CircuitSim::new(circuit),
            layout,
            scene,
            parts,
            camera: default_camera(),
            path: current_loop(),
            glows: alloc::vec![Spring::critical(0.0, 5.0); num_bulbs],
            levers: alloc::vec![Spring::critical(0.0, 14.0); num_switches],
            flashes: alloc::vec![Spring::critical(0.0, 3.0); num_bulbs],
            phase: 0.0,
            time: 0.0,
            clock: FixedClock::new(Seconds(STEP)).with_max_steps(30),
            commands: CommandBuffer::new(),
            frame_buf: simbiguity_render::FrameBuf::new(),
        };
        // Initial setup omitted for brevity in minimal prototype
        view.seek(0.0);
        view
    }

    fn lever_target(&self, i: usize) -> f64 {
        let switch_id = self
            .layout
            .parts
            .iter()
            .filter(|p| {
                matches!(
                    self.sim.circuit().component(p.component),
                    Component::Switch { .. }
                )
            })
            .nth(i)
            .map(|p| p.component);
        let closed = switch_id
            .map(|s| {
                matches!(
                    self.sim.circuit().component(s),
                    Component::Switch { closed: true }
                )
            })
            .unwrap_or(false);
        if closed {
            0.0
        } else {
            crate::layout::LEVER_OPEN_ANGLE
        }
    }

    /// Is the switch closed?
    fn first_battery(&self) -> Option<simbiguity_circuits::ComponentId> {
        self.layout
            .parts
            .iter()
            .find(|p| {
                matches!(
                    self.sim.circuit().component(p.component),
                    Component::Battery { .. }
                )
            })
            .map(|p| p.component)
    }

    fn first_switch(&self) -> Option<simbiguity_circuits::ComponentId> {
        self.layout
            .parts
            .iter()
            .find(|p| {
                matches!(
                    self.sim.circuit().component(p.component),
                    Component::Switch { .. }
                )
            })
            .map(|p| p.component)
    }

    fn first_bulb(&self) -> Option<simbiguity_circuits::ComponentId> {
        self.layout
            .parts
            .iter()
            .find(|p| {
                matches!(
                    self.sim.circuit().component(p.component),
                    Component::Bulb { .. }
                )
            })
            .map(|p| p.component)
    }

    pub fn is_closed(&self) -> bool {
        self.first_switch()
            .map(|s| {
                matches!(
                    self.sim.circuit().component(s),
                    Component::Switch { closed: true }
                )
            })
            .unwrap_or(false)
    }

    /// Is the bulb blown?
    pub fn is_blown(&self) -> bool {
        self.first_bulb()
            .map(|b| {
                matches!(
                    self.sim.circuit().component(b),
                    Component::Bulb { blown: true, .. }
                )
            })
            .unwrap_or(false)
    }

    /// Applies an input and re-solves; a bulb that blows now flashes.
    fn input(&mut self, input: CircuitInput) {
        let was_blown = self.is_blown();
        self.sim.apply(input);
        self.sim.step(Seconds(0.0));
        let is_blown = self.is_blown();
        if is_blown && !was_blown {
            for f in &mut self.flashes {
                f.snap(1.0);
            }
        }
        let targets: alloc::vec::Vec<_> = (0..self.levers.len())
            .map(|i| self.lever_target(i))
            .collect();
        for (i, lever) in self.levers.iter_mut().enumerate() {
            lever.target = targets[i];
        }
        let brightness = self.readouts().brightness;
        for glow in &mut self.glows {
            glow.target = brightness;
        }
        self.sync_scene();
    }

    /// Flips the switch; returns `true` if it is now closed.
    pub fn toggle_switch(&mut self) -> bool {
        if let Some(s) = self.first_switch() {
            self.input(CircuitInput::ToggleSwitch(s));
        }
        self.is_closed()
    }

    /// Sets the battery voltage.
    pub fn set_volts(&mut self, volts: f64) {
        if let Some(b) = self.first_battery() {
            self.input(CircuitInput::SetValue(b, volts));
        }
    }

    /// Screws in a fresh bulb.
    pub fn replace_bulb(&mut self) {
        if let Some(b) = self.first_bulb() {
            self.input(CircuitInput::ReplaceBulb(b));
        }
    }

    /// The engine's answer, plus the animated glow.
    pub fn readouts(&self) -> Readouts {
        let sol = self.sim.state().solution();
        let val = |comp: Option<simbiguity_circuits::ComponentId>,
                   f: &dyn Fn(
            &simbiguity_circuits::Solution,
            simbiguity_circuits::ComponentId,
        ) -> f64|
         -> f64 { comp.and_then(|c| sol.map(|s| f(s, c))).unwrap_or(0.0) };

        let battery = self.first_battery();
        let switch = self.first_switch();
        let bulb = self.first_bulb();

        let current = val(battery, &|s, c| s.current(c));
        let battery_volts = battery
            .and_then(|b| match self.sim.circuit().component(b) {
                Component::Battery { emf, .. } => Some(*emf),
                _ => None,
            })
            .unwrap_or(0.0);

        Readouts {
            closed: self.is_closed(),
            battery_volts,
            current,
            bulb_volts: val(bulb, &|s, c| s.voltage_across(c).unwrap_or(0.0)),
            bulb_power: val(bulb, &|s, c| s.power(c)),
            brightness: val(bulb, &|s, c| s.brightness(c).unwrap_or(0.0)),
            glow: self
                .glows
                .first()
                .map(|g| g.value().clamp(0.0, 1.0))
                .unwrap_or(0.0),
            particle_speed: particle_speed(current),
            blown: self.is_blown(),
            solved: sol.is_some(),
        }
    }

    /// Seconds of animation so far.
    pub fn time(&self) -> f64 {
        self.time
    }

    /// One fixed step of animation.
    fn tick(&mut self, dt: f64) {
        for glow in &mut self.glows {
            glow.step(dt);
        }
        for lever in &mut self.levers {
            lever.step(dt);
        }
        for flash in &mut self.flashes {
            flash.step(dt);
        }
        self.phase += particle_speed(self.readouts().current) * dt;
        self.time += dt;
    }

    /// Banks `real_dt` seconds of wall-clock time and runs the whole fixed
    /// steps it pays for.
    pub fn advance(&mut self, real_dt: f64) {
        let n = self.clock.advance(Seconds(real_dt)).steps;
        for _ in 0..n {
            self.tick(STEP);
        }
        self.sync_scene();
    }

    /// Restarts the animation from its resting state and runs it to time
    /// `t` exactly: the bulb starts cold, the lever starts where the switch
    /// is, a blown bulb starts its flash. Same inputs, same frame.
    pub fn seek(&mut self, t: f64) {
        let brightness = self.readouts().brightness;
        for glow in &mut self.glows {
            glow.snap(0.0);
            glow.target = brightness;
        }
        let targets: alloc::vec::Vec<_> = (0..self.levers.len())
            .map(|i| self.lever_target(i))
            .collect();
        for (i, lever) in self.levers.iter_mut().enumerate() {
            lever.target = targets[i];
            lever.snap(lever.target);
        }
        let is_blown = self.is_blown();
        for flash in &mut self.flashes {
            flash.snap(if is_blown { 1.0 } else { 0.0 });
            flash.target = 0.0;
        }
        self.phase = 0.0;
        self.time = 0.0;
        self.clock.reset();
        let steps = (t.max(0.0) / STEP + 0.5) as u64;
        for _ in 0..steps {
            self.tick(STEP);
        }
        self.sync_scene();
    }

    /// Writes the animated state into the scene.
    fn sync_scene(&mut self) {
        let current = self.readouts().current;
        let is_blown = self.is_blown();

        let p = &self.parts;
        let s = &mut self.scene;

        for (i, &lever_id) in p.levers.iter().enumerate() {
            let angle = self.levers[i].value();
            s.node_mut(lever_id).transform = lever_transform(angle);
        }
        for (i, &knob_id) in p.knobs.iter().enumerate() {
            let angle = self.levers[i].value();
            s.node_mut(knob_id).transform = knob_transform(angle);
        }

        for (i, &glass_id) in p.glasses.iter().enumerate() {
            let look = bulb_look(self.glows[i].value(), self.flashes[i].value(), is_blown);
            paint(&mut s.node_mut(glass_id).material, look.glass);
            paint(&mut s.node_mut(p.filaments[i]).material, look.filament);
            paint(&mut s.node_mut(p.halos[i]).material, look.halo);
            s.node_mut(p.filaments[i]).visible = !look.blown;
            s.node_mut(p.filaments_broken[i]).visible = look.blown;
        }

        let charge = charge_look(current);
        let inst = &mut self.scene.instances[0];
        inst.offsets = particle_offsets(&self.path, self.phase, PARTICLES);
        inst.material.color = charge.color;
        inst.material.emissive = charge.emissive;
    }

    /// The scene as it stands now.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Encodes the current frame for a `width × height` viewport.
    pub fn encode(&mut self, width: u32, height: u32) -> &[u32] {
        encode_frame(
            &self.scene,
            &self.camera,
            width,
            height,
            &mut self.frame_buf,
            &mut self.commands,
        );
        self.commands.words()
    }

    /// The frame the last [`CircuitView::encode`] wrote.
    pub fn commands(&self) -> &[u32] {
        self.commands.words()
    }

    /// Which pick tag (see [`layout::PICK_SWITCH`] etc.) is under pixel
    /// `(px, py)` of a `width × height` view, if any.
    pub fn pick(&self, px: f64, py: f64, width: f64, height: f64) -> Option<u32> {
        let ray = self.camera.ray(px, py, width, height);
        self.scene.pick(&ray).map(|h| h.tag)
    }
}

/// The opening camera: a three-quarter view from the front right.
pub fn default_camera() -> OrbitCamera {
    OrbitCamera {
        target: Vec3::new(0.0, 0.35, 0.0),
        yaw: 25f64.to_radians(),
        pitch: 38f64.to_radians(),
        distance: 9.5,
        min_distance: 4.0,
        max_distance: 20.0,
        ..OrbitCamera::default()
    }
}

/// Writes a look's colour and strength into a material; the glass also
/// takes its opacity (every other surface keeps alpha 1).
fn paint(m: &mut Material, look: Surface) {
    m.color = look.color;
    m.alpha = look.alpha;
    m.emissive = look.emissive;
}
