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
pub use simbiguity_circuits;
use simbiguity_circuits::{Circuit, CircuitInput, CircuitSim, Component, ComponentId};
use simbiguity_core::{FixedClock, Seconds, Simulation};
use simbiguity_math::Vec3;
use simbiguity_render::{encode_frame, CommandBuffer, Material, OrbitCamera, Scene};

use board::{build_scene, Parts};
use layout::{knob_transform, lever_transform, particle_offsets, CurrentLoop};
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
    wire_paths: alloc::vec::Vec<CurrentLoop>,
    phases: alloc::vec::Vec<f64>,
    glows: alloc::vec::Vec<Spring>,
    levers: alloc::vec::Vec<Spring>,
    flashes: alloc::vec::Vec<Spring>,
    time: f64,
    clock: FixedClock,
    commands: CommandBuffer,
    frame_buf: simbiguity_render::FrameBuf,
}

impl CircuitView {
    #[allow(clippy::unwrap_used)]
    pub fn new(volts: f64, closed: bool) -> CircuitView {
        let mut c = Circuit::new();
        let (n1, n2, n3, n4, n5, n6) = (c.node(), c.node(), c.node(), c.node(), c.node(), c.node());
        let battery = c.add_battery(n1, n6, volts);
        let switch = c.add_switch(n2, n3, closed);
        let bulb = c.add_bulb(n4, n5, BULB_VOLTS, BULB_WATTS);
        let w1 = c.connect(n1, n2);
        let w2 = c.connect(n3, n4);
        let w3 = c.connect(n5, n6);
        let layout = BoardLayout::hello_layout(battery, switch, bulb, [w1, w2, w3]);
        Self::from_layout(c, layout).unwrap()
    }

    pub fn from_layout(
        circuit: Circuit,
        layout: BoardLayout,
    ) -> Result<CircuitView, crate::board_layout::LayoutError> {
        layout.validate(&circuit)?;
        let (scene, parts, _) = build_scene(&circuit, &layout);
        let num_switches = parts.levers.len();
        let num_bulbs = parts.glasses.len();
        let wire_paths = layout
            .wires
            .iter()
            .map(crate::layout::wire_path)
            .collect::<alloc::vec::Vec<_>>();
        let phases = alloc::vec![0.0; layout.wires.len()];
        let mut view = CircuitView {
            sim: CircuitSim::new(circuit),
            layout,
            scene,
            parts,
            camera: default_camera(),
            wire_paths,
            phases,
            glows: alloc::vec![Spring::critical(0.0, 5.0); num_bulbs],
            levers: alloc::vec![Spring::critical(0.0, 14.0); num_switches],
            flashes: alloc::vec![Spring::critical(0.0, 3.0); num_bulbs],
            time: 0.0,
            clock: FixedClock::new(Seconds(STEP)).with_max_steps(30),
            commands: CommandBuffer::new(),
            frame_buf: simbiguity_render::FrameBuf::new(),
        };
        view.seek(0.0);
        Ok(view)
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
    pub fn is_closed(&self, id: ComponentId) -> bool {
        matches!(
            self.sim.circuit().component(id),
            Component::Switch { closed: true }
        )
    }

    pub fn is_blown(&self, id: ComponentId) -> bool {
        matches!(
            self.sim.circuit().component(id),
            Component::Bulb { blown: true, .. }
        )
    }

    pub fn glow_of(&self, id: ComponentId) -> f64 {
        let i = self
            .layout
            .parts
            .iter()
            .filter(|p| {
                matches!(
                    self.sim.circuit().component(p.component),
                    Component::Bulb { .. }
                )
            })
            .position(|p| p.component == id)
            .unwrap_or(0);
        self.glows[i].value()
    }

    /// Applies an input and re-solves; a bulb that blows now flashes.
    fn input(&mut self, input: CircuitInput) {
        let mut was_blown = alloc::vec::Vec::new();
        for p in &self.layout.parts {
            if matches!(
                self.sim.circuit().component(p.component),
                Component::Bulb { .. }
            ) {
                was_blown.push(self.is_blown(p.component));
            }
        }
        self.sim.apply(input);
        self.sim.step(Seconds(0.0));

        let blown_now: alloc::vec::Vec<bool> = self
            .layout
            .parts
            .iter()
            .filter(|p| {
                matches!(
                    self.sim.circuit().component(p.component),
                    Component::Bulb { .. }
                )
            })
            .map(|p| self.is_blown(p.component))
            .collect();

        for (i, flash) in self.flashes.iter_mut().enumerate() {
            if blown_now[i] && !was_blown[i] {
                flash.snap(1.0);
            }
        }
        let targets: alloc::vec::Vec<_> = (0..self.levers.len())
            .map(|i| self.lever_target(i))
            .collect();
        for (i, lever) in self.levers.iter_mut().enumerate() {
            lever.target = targets[i];
        }

        let bulb_targets: alloc::vec::Vec<_> = self
            .layout
            .parts
            .iter()
            .filter(|p| {
                matches!(
                    self.sim.circuit().component(p.component),
                    Component::Bulb { .. }
                )
            })
            .map(|p| {
                self.sim
                    .state()
                    .solution()
                    .and_then(|s| s.brightness(p.component))
                    .unwrap_or(0.0)
            })
            .collect();

        for (i, glow) in self.glows.iter_mut().enumerate() {
            glow.target = bulb_targets[i];
        }
        self.sync_scene();
    }

    pub fn circuit(&self) -> &Circuit {
        self.sim.circuit()
    }

    pub fn sim(&self) -> &CircuitSim {
        &self.sim
    }

    pub fn sim_mut(&mut self) -> &mut CircuitSim {
        &mut self.sim
    }

    pub fn toggle_part_by_index(&mut self, id: usize) -> bool {
        let cid = self
            .layout
            .parts
            .iter()
            .map(|p| p.component)
            .chain(self.layout.wires.iter().map(|w| w.component))
            .find(|c| c.index() == id);
        if let Some(cid) = cid {
            self.toggle_part(cid)
        } else {
            false
        }
    }

    pub fn toggle_part(&mut self, id: ComponentId) -> bool {
        if matches!(self.sim.circuit().component(id), Component::Switch { .. }) {
            self.input(CircuitInput::ToggleSwitch(id));
            if let simbiguity_circuits::Component::Switch { closed } =
                self.sim.circuit().component(id)
            {
                *closed
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Flips the switch; returns `true` if it is now closed.
    pub fn toggle_switch(&mut self) -> bool {
        let id = self
            .layout
            .parts
            .iter()
            .find(|p| {
                matches!(
                    self.sim.circuit().component(p.component),
                    Component::Switch { .. }
                )
            })
            .map(|p| p.component);
        if let Some(s) = id {
            self.input(CircuitInput::ToggleSwitch(s));
            self.is_closed(s)
        } else {
            false
        }
    }

    /// Sets the battery voltage.
    pub fn set_volts(&mut self, volts: f64) {
        let id = self
            .layout
            .parts
            .iter()
            .find(|p| {
                matches!(
                    self.sim.circuit().component(p.component),
                    Component::Battery { .. }
                )
            })
            .map(|p| p.component);
        if let Some(b) = id {
            self.input(CircuitInput::SetValue(b, volts));
        }
    }

    /// Screws in a fresh bulb.
    pub fn replace_bulb(&mut self) {
        let id = self
            .layout
            .parts
            .iter()
            .find(|p| {
                matches!(
                    self.sim.circuit().component(p.component),
                    Component::Bulb { .. }
                )
            })
            .map(|p| p.component);
        if let Some(b) = id {
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

        let battery = self
            .layout
            .parts
            .iter()
            .find(|p| {
                matches!(
                    self.sim.circuit().component(p.component),
                    Component::Battery { .. }
                )
            })
            .map(|p| p.component);
        let switch = self
            .layout
            .parts
            .iter()
            .find(|p| {
                matches!(
                    self.sim.circuit().component(p.component),
                    Component::Switch { .. }
                )
            })
            .map(|p| p.component);
        let bulb = self
            .layout
            .parts
            .iter()
            .find(|p| {
                matches!(
                    self.sim.circuit().component(p.component),
                    Component::Bulb { .. }
                )
            })
            .map(|p| p.component);

        let current = val(battery, &|s, c| s.current(c));
        let battery_volts = battery
            .and_then(|b| match self.sim.circuit().component(b) {
                Component::Battery { emf, .. } => Some(*emf),
                _ => None,
            })
            .unwrap_or(0.0);

        Readouts {
            closed: switch.map(|s| self.is_closed(s)).unwrap_or(false),
            battery_volts,
            current,
            bulb_volts: val(bulb, &|s, c| s.voltage_across(c).unwrap_or(0.0)),
            bulb_power: val(bulb, &|s, c| s.power(c)),
            brightness: val(bulb, &|s, c| s.brightness(c).unwrap_or(0.0)),
            glow: bulb.map(|b| self.glow_of(b).clamp(0.0, 1.0)).unwrap_or(0.0),
            particle_speed: particle_speed(current),
            blown: bulb.map(|b| self.is_blown(b)).unwrap_or(false),
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
        let solution = self.sim.state().solution();
        let currents: alloc::vec::Vec<f64> = self
            .layout
            .wires
            .iter()
            .map(|w| solution.map(|s| s.current(w.component)).unwrap_or(0.0))
            .collect();
        for (i, p) in self.phases.iter_mut().enumerate() {
            *p += particle_speed(currents[i]) * dt;
        }
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
        let bulb_targets: alloc::vec::Vec<_> = self
            .layout
            .parts
            .iter()
            .filter(|p| {
                matches!(
                    self.sim.circuit().component(p.component),
                    Component::Bulb { .. }
                )
            })
            .map(|p| {
                self.sim
                    .state()
                    .solution()
                    .and_then(|s| s.brightness(p.component))
                    .unwrap_or(0.0)
            })
            .collect();

        for (i, glow) in self.glows.iter_mut().enumerate() {
            glow.snap(0.0);
            glow.target = bulb_targets[i];
        }
        let targets: alloc::vec::Vec<_> = (0..self.levers.len())
            .map(|i| self.lever_target(i))
            .collect();
        for (i, lever) in self.levers.iter_mut().enumerate() {
            lever.target = targets[i];
            lever.snap(lever.target);
        }

        let blown_now: alloc::vec::Vec<bool> = self
            .layout
            .parts
            .iter()
            .filter(|p| {
                matches!(
                    self.sim.circuit().component(p.component),
                    Component::Bulb { .. }
                )
            })
            .map(|p| self.is_blown(p.component))
            .collect();

        for (i, flash) in self.flashes.iter_mut().enumerate() {
            flash.snap(if blown_now[i] { 1.0 } else { 0.0 });
            flash.target = 0.0;
        }
        self.phases.fill(0.0);
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
        let blown_now: alloc::vec::Vec<bool> = self
            .layout
            .parts
            .iter()
            .filter(|p| {
                matches!(
                    self.sim.circuit().component(p.component),
                    Component::Bulb { .. }
                )
            })
            .map(|p| self.is_blown(p.component))
            .collect();

        let p = &self.parts;
        let s = &mut self.scene;

        for (i, &lever_id) in p.levers.iter().enumerate() {
            let angle = self.levers[i].value();
            s.node_mut(lever_id).transform = lever_transform(p.switch_bases[i], angle);
        }
        for (i, &knob_id) in p.knobs.iter().enumerate() {
            let angle = self.levers[i].value();
            s.node_mut(knob_id).transform = knob_transform(p.switch_bases[i], angle);
        }

        for (i, &glass_id) in p.glasses.iter().enumerate() {
            let is_blown = blown_now[i];
            let look = bulb_look(self.glows[i].value(), self.flashes[i].value(), is_blown);
            paint(&mut s.node_mut(glass_id).material, look.glass);
            paint(&mut s.node_mut(p.filaments[i]).material, look.filament);
            paint(&mut s.node_mut(p.halos[i]).material, look.halo);
            s.node_mut(p.filaments[i]).visible = !look.blown;
            s.node_mut(p.filaments_broken[i]).visible = look.blown;
        }

        for &(cid, glass, halo) in &p.leds {
            let brightness = self
                .sim
                .state()
                .solution()
                .and_then(|sol| sol.brightness(cid))
                .unwrap_or(0.0);
            let rgb = s.node(glass).material.color;
            s.node_mut(glass).material = simbiguity_render::Material::lit(rgb)
                .with_alpha(0.8 + 0.2 * (brightness as f32).min(1.0));
            s.node_mut(halo).material = simbiguity_render::Material::glow(rgb, brightness as f32);
        }

        for (i, path) in self.wire_paths.iter().enumerate() {
            let wire_id = self.layout.wires[i].component;
            let current = self
                .sim
                .state()
                .solution()
                .map(|sol| sol.current(wire_id))
                .unwrap_or(0.0);
            let charge = charge_look(current);
            let len = path.length();
            let count = if len > 0.0 {
                (len * 2.5 + 0.5) as usize
            } else {
                0
            };
            let inst = &mut s.instances[i];
            inst.offsets.clear();
            inst.offsets
                .extend(particle_offsets(path, self.phases[i], count));
            inst.material.color = charge.color;
            inst.material.emissive = charge.emissive;
        }
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
