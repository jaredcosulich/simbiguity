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
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;
#[cfg(any(test, feature = "std"))]
extern crate std;

pub mod board;
pub mod layout;
pub mod look;
#[cfg(test)]
mod view_tests;

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
    battery: ComponentId,
    switch: ComponentId,
    bulb: ComponentId,
    scene: Scene,
    parts: Parts,
    /// The camera the frame is drawn through.
    pub camera: OrbitCamera,
    path: CurrentLoop,
    glow: Spring,
    lever: Spring,
    flash: Spring,
    phase: f64,
    time: f64,
    clock: FixedClock,
    commands: CommandBuffer,
    frame_buf: simbiguity_render::FrameBuf,
}

impl CircuitView {
    /// The hello circuit at `volts`, switch `closed` or open.
    pub fn new(volts: f64, closed: bool) -> CircuitView {
        let mut c = Circuit::new();
        let (plus, minus, top) = (c.node(), c.node(), c.node());
        let battery = c.add_battery(plus, minus, volts);
        let switch = c.add_switch(plus, top, closed);
        let bulb = c.add_bulb(top, minus, BULB_VOLTS, BULB_WATTS);
        let (scene, parts, _) = build_scene();
        let mut view = CircuitView {
            sim: CircuitSim::new(c),
            battery,
            switch,
            bulb,
            scene,
            parts,
            camera: default_camera(),
            path: current_loop(),
            glow: Spring::critical(0.0, 5.0),
            lever: Spring::critical(0.0, 14.0),
            flash: Spring::critical(0.0, 3.0),
            phase: 0.0,
            time: 0.0,
            clock: FixedClock::new(Seconds(STEP)).with_max_steps(30),
            commands: CommandBuffer::new(),
            frame_buf: simbiguity_render::FrameBuf::new(),
        };
        view.lever.target = view.lever_target();
        view.seek(0.0);
        view
    }

    fn lever_target(&self) -> f64 {
        if self.is_closed() {
            0.0
        } else {
            layout::LEVER_OPEN_ANGLE
        }
    }

    /// Is the switch closed?
    pub fn is_closed(&self) -> bool {
        matches!(
            self.sim.circuit().component(self.switch),
            Component::Switch { closed: true }
        )
    }

    /// Is the bulb blown?
    pub fn is_blown(&self) -> bool {
        matches!(
            self.sim.circuit().component(self.bulb),
            Component::Bulb { blown: true, .. }
        )
    }

    /// Applies an input and re-solves; a bulb that blows now flashes.
    fn input(&mut self, input: CircuitInput) {
        let was_blown = self.is_blown();
        self.sim.apply(input);
        self.sim.step(Seconds(0.0));
        if self.is_blown() && !was_blown {
            self.flash.snap(1.0);
        }
        self.lever.target = self.lever_target();
        self.glow.target = self.readouts().brightness;
        self.sync_scene();
    }

    /// Flips the switch; returns `true` if it is now closed.
    pub fn toggle_switch(&mut self) -> bool {
        self.input(CircuitInput::ToggleSwitch(self.switch));
        self.is_closed()
    }

    /// Sets the battery voltage.
    pub fn set_volts(&mut self, volts: f64) {
        self.input(CircuitInput::SetValue(self.battery, volts));
    }

    /// Screws in a fresh bulb.
    pub fn replace_bulb(&mut self) {
        self.input(CircuitInput::ReplaceBulb(self.bulb));
    }

    /// The engine's answer, plus the animated glow.
    pub fn readouts(&self) -> Readouts {
        let sol = self.sim.state().solution();
        let val = |f: &dyn Fn(&simbiguity_circuits::Solution) -> f64| sol.map_or(0.0, f);
        let current = val(&|s| s.current(self.battery));
        let battery_volts = match self.sim.circuit().component(self.battery) {
            Component::Battery { emf, .. } => *emf,
            _ => 0.0,
        };
        Readouts {
            closed: self.is_closed(),
            battery_volts,
            current,
            bulb_volts: val(&|s| s.voltage_across(self.bulb).unwrap_or(0.0)),
            bulb_power: val(&|s| s.power(self.bulb)),
            brightness: val(&|s| s.brightness(self.bulb).unwrap_or(0.0)),
            glow: self.glow.value().clamp(0.0, 1.0),
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
        self.glow.step(dt);
        self.lever.step(dt);
        self.flash.step(dt);
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
        self.glow.snap(0.0);
        self.glow.target = brightness;
        self.lever.target = self.lever_target();
        self.lever.snap(self.lever.target);
        self.flash.snap(if self.is_blown() { 1.0 } else { 0.0 });
        self.flash.target = 0.0;
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
        let look = bulb_look(self.glow.value(), self.flash.value(), self.is_blown());
        let angle = self.lever.value();
        let p = self.parts;
        let s = &mut self.scene;
        s.node_mut(p.lever).transform = lever_transform(angle);
        s.node_mut(p.knob).transform = knob_transform(angle);
        paint(&mut s.node_mut(p.glass).material, look.glass);
        paint(&mut s.node_mut(p.filament).material, look.filament);
        paint(&mut s.node_mut(p.halo).material, look.halo);
        s.node_mut(p.filament).visible = !look.blown;
        s.node_mut(p.filament_broken).visible = look.blown;

        let charge = charge_look(self.readouts().current);
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
