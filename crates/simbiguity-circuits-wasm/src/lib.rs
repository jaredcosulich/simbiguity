#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
//! # simbiguity-circuits-wasm
//!
//! A hand-written bridge from JavaScript to [`simbiguity_circuits`]. No
//! `wasm-bindgen`, no generated glue: just `extern "C"` functions over
//! numbers, so you can read the whole contract here and in
//! `site/js/simbiguity.js`.
//!
//! **The contract.**
//! - A circuit lives in a table inside WASM; JS holds a `u32` **handle**
//!   (`0` is never a valid handle).
//! - Nodes and components are `u32` ids in creation order.
//! - Calls that can fail return `i32`: `>= 0` is the answer, `< 0` a status
//!   code from [`status`], with a UTF-8 message at
//!   `sb_last_error_ptr()/sb_last_error_len()`.
//! - Bulk results are a `Vec<f64>` in linear memory; JS views it with
//!   `new Float64Array(memory.buffer, ptr, len)`.
//!
//! Every export is an ordinary Rust function, so the bridge is unit-tested
//! natively (`cargo test`) — nothing here is WASM-only.

pub mod alloc;
pub mod golden;
pub mod issues;
mod slot;
pub mod status;
#[cfg(test)]
mod tests;

use std::string::ToString;

use simbiguity_circuits::{CircuitInput, Component};
use simbiguity_core::Simulation;

use issues::{issue_kind_code, issue_part_index};
use slot::{with_slot, Slot};
use status::{fail, output, ERR_KIND, OK};

/// Numbers per component in [`circuit_results_ptr`]: voltage across,
/// current, power, brightness. `NaN` marks "no value" (a floating part, a
/// component that is not a bulb, or no solution at all).
pub const RESULT_STRIDE: usize = 4;

/// Creates an empty circuit and returns its handle (never `0`).
#[no_mangle]
pub extern "C" fn circuit_new() -> u32 {
    slot::insert_empty()
}

/// Frees a circuit. Its handle is dead afterwards; freeing it again is
/// [`status::ERR_HANDLE`], not undefined behaviour.
#[no_mangle]
pub extern "C" fn circuit_free(h: u32) -> i32 {
    slot::remove(h)
}

/// Adds a node (junction); returns its id.
#[no_mangle]
pub extern "C" fn circuit_node(h: u32) -> i32 {
    with_slot(h, Slot::add_node)
}

/// An ideal battery with `+` at node `plus`; returns its component id.
#[no_mangle]
pub extern "C" fn circuit_add_battery(h: u32, plus: u32, minus: u32, emf: f64) -> i32 {
    with_slot(h, |s| s.place(Component::battery(emf), plus, minus))
}

/// A resistor of `ohms`; returns its component id.
#[no_mangle]
pub extern "C" fn circuit_add_resistor(h: u32, a: u32, b: u32, ohms: f64) -> i32 {
    with_slot(h, |s| s.place(Component::Resistor { r: ohms }, a, b))
}

/// A bulb rated `watts` at `volts`; returns its component id.
#[no_mangle]
pub extern "C" fn circuit_add_bulb(h: u32, a: u32, b: u32, volts: f64, watts: f64) -> i32 {
    with_slot(h, |s| s.place(Component::bulb(volts, watts), a, b))
}

#[no_mangle]
pub extern "C" fn circuit_add_bjt(
    h: u32,
    c: u32,
    b: u32,
    e: u32,
    is_pnp: u32,
    saturation_amps: f64,
    beta_f: f64,
    beta_r: f64,
) -> i32 {
    let polarity = if is_pnp != 0 {
        simbiguity_circuits::component::Polarity::Pnp
    } else {
        simbiguity_circuits::component::Polarity::Npn
    };
    with_slot(h, |s| {
        s.place_3(
            Component::Bjt {
                polarity,
                saturation_amps,
                beta_f,
                beta_r,
            },
            c,
            b,
            e,
        )
    })
}

#[no_mangle]
pub extern "C" fn circuit_add_signal(
    h: u32,
    a: u32,
    b: u32,
    waveform: u32,
    amplitude: f64,
    frequency: f64,
    phase: f64,
    offset: f64,
) -> i32 {
    with_slot(h, |s| {
        s.place(
            Component::SignalSource {
                waveform: waveform as u8,
                amplitude,
                frequency,
                phase,
                offset,
            },
            a,
            b,
        )
    })
}

#[no_mangle]
pub extern "C" fn sim_time(h: u32) -> f64 {
    let mut out = 0.0;
    with_slot(h, |s| {
        out = s.sim.time().0;
        OK
    });
    out
}

#[no_mangle]
pub extern "C" fn sim_add_probe(h: u32, is_current: u32, target_id: u32, capacity: u32) -> i32 {
    with_slot(h, |s| {
        let probe = if is_current != 0 {
            match s.part(target_id) {
                Ok(id) => simbiguity_circuits::sim::Probe::Current(id),
                Err(e) => return e,
            }
        } else {
            match s.node(target_id) {
                Ok(id) => simbiguity_circuits::sim::Probe::NodeVoltage(id),
                Err(e) => return e,
            }
        };
        let id = s.sim.add_probe(probe, capacity as usize);
        id.0 as i32
    })
}

static mut LAST_PROBE_SAMPLES: std::vec::Vec<f64> = std::vec::Vec::new();

#[allow(static_mut_refs)]
#[no_mangle]
pub extern "C" fn sim_probe_samples_ptr(h: u32, probe_id: u32) -> *const f64 {
    let mut ptr = core::ptr::null();
    with_slot(h, |s| {
        let samples = s
            .sim
            .probe_samples(simbiguity_circuits::sim::ProbeId(probe_id as usize));
        let mut flat = std::vec::Vec::with_capacity(samples.len() * 2);
        for (t, v) in samples {
            flat.push(t);
            flat.push(v);
        }
        unsafe {
            LAST_PROBE_SAMPLES = flat;
            ptr = LAST_PROBE_SAMPLES.as_ptr();
        }
        OK
    });
    ptr
}

#[allow(static_mut_refs)]
#[no_mangle]
pub extern "C" fn sim_probe_samples_len() -> i32 {
    unsafe { LAST_PROBE_SAMPLES.len() as i32 }
}

#[no_mangle]
pub extern "C" fn circuit_add_diode(h: u32, a: u32, b: u32, is: f64, n: f64) -> i32 {
    with_slot(h, |s| {
        s.place(
            Component::Diode {
                saturation_amps: is,
                emission: n,
            },
            a,
            b,
        )
    })
}

#[no_mangle]
pub extern "C" fn circuit_add_led(h: u32, a: u32, b: u32, rated_amps: f64, is: f64, n: f64) -> i32 {
    with_slot(h, |s| {
        s.place(
            Component::Led {
                color: "red",
                rated_amps,
                saturation_amps: is,
                emission: n,
                blown: false,
            },
            a,
            b,
        )
    })
}

/// A capacitor of `farads`; returns its component id.
#[no_mangle]
pub extern "C" fn circuit_add_capacitor(h: u32, a: u32, b: u32, farads: f64) -> i32 {
    with_slot(h, |s| s.place(Component::capacitor(farads), a, b))
}

/// An inductor of `henries`; returns its component id.
#[no_mangle]
pub extern "C" fn circuit_add_inductor(h: u32, a: u32, b: u32, henries: f64) -> i32 {
    with_slot(h, |s| s.place(Component::inductor(henries), a, b))
}

/// A switch, closed when `closed != 0`; returns its component id.
#[no_mangle]
pub extern "C" fn circuit_add_switch(h: u32, a: u32, b: u32, closed: i32) -> i32 {
    with_slot(h, |s| {
        s.place(
            Component::Switch {
                closed: closed != 0,
            },
            a,
            b,
        )
    })
}

/// An ideal wire between `a` and `b`; returns its component id.
#[no_mangle]
pub extern "C" fn circuit_add_wire(h: u32, a: u32, b: u32) -> i32 {
    with_slot(h, |s| s.place(Component::Wire, a, b))
}

/// Flips a switch; returns its new state (`1` closed, `0` open). Takes
/// effect at the next [`circuit_solve`].
#[no_mangle]
pub extern "C" fn circuit_toggle(h: u32, id: u32) -> i32 {
    with_slot(h, |s| {
        let id = match s.part(id) {
            Ok(id) => id,
            Err(e) => return e,
        };
        match *s.sim.circuit().component(id) {
            Component::Switch { closed } => {
                s.sim.apply(CircuitInput::ToggleSwitch(id));
                i32::from(!closed)
            }
            _ => fail(
                ERR_KIND,
                std::format!("component {} is not a switch", id.index()),
            ),
        }
    })
}

/// Sets a component's main value (a battery's EMF, a resistor's ohms, a
/// bulb's rated volts, a switch's state). Takes effect at the next solve.
#[no_mangle]
pub extern "C" fn circuit_set_value(h: u32, id: u32, value: f64) -> i32 {
    with_slot(h, |s| match s.part(id) {
        Ok(id) => {
            s.sim.apply(CircuitInput::SetValue(id, value));
            OK
        }
        Err(e) => e,
    })
}

/// Screws a fresh bulb in place of a blown one. Takes effect at the next solve.
#[no_mangle]
pub extern "C" fn circuit_replace_bulb(h: u32, id: u32) -> i32 {
    with_slot(h, |s| {
        let id = match s.part(id) {
            Ok(id) => id,
            Err(e) => return e,
        };
        match s.sim.circuit().component(id) {
            Component::Bulb { .. } => {
                s.sim.apply(CircuitInput::ReplaceBulb(id));
                OK
            }
            _ => fail(
                ERR_KIND,
                std::format!("component {} is not a bulb", id.index()),
            ),
        }
    })
}

/// Solves the circuit; returns how many issues were found (`0` = all good).
/// Blown bulbs stay blown until replaced.
#[no_mangle]
pub extern "C" fn circuit_solve(h: u32) -> i32 {
    with_slot(h, Slot::solve)
}

/// Steps the circuit forward in time by `dt` seconds, solving the transient response.
#[no_mangle]
pub extern "C" fn circuit_step(h: u32, dt: f64) -> i32 {
    with_slot(h, |s| s.step(dt))
}

/// `1` if the last solve produced a solution, `0` if a fatal issue stopped it.
#[no_mangle]
pub extern "C" fn circuit_has_solution(h: u32) -> i32 {
    with_slot(h, |s| i32::from(s.sim.state().solution().is_some()))
}

/// A bulb's brightness `0..=1` from the last solve; `NaN` (with a message
/// in `sb_last_error_*`) for a bad handle, a non-bulb, or no solution.
#[no_mangle]
pub extern "C" fn circuit_bulb_brightness(h: u32, id: u32) -> f64 {
    let mut out = f64::NAN;
    with_slot(h, |s| {
        let id = match s.part(id) {
            Ok(id) => id,
            Err(e) => return e,
        };
        match s.sim.state().solution().map(|sol| sol.brightness(id)) {
            Some(Some(b)) => {
                out = b;
                OK
            }
            Some(None) => fail(
                ERR_KIND,
                std::format!("component {} is not a bulb", id.index()),
            ),
            None => fail(ERR_KIND, "the circuit has no solution"),
        }
    });
    out
}

/// Address of the last solve's results: [`RESULT_STRIDE`] `f64`s per
/// component, in id order. Valid until the next call into this circuit.
#[no_mangle]
pub extern "C" fn circuit_results_ptr(h: u32) -> *const f64 {
    let mut ptr = core::ptr::null();
    with_slot(h, |s| {
        ptr = s.results.as_ptr();
        OK
    });
    ptr
}

/// Number of `f64`s at [`circuit_results_ptr`] (or a negative status).
#[no_mangle]
pub extern "C" fn circuit_results_len(h: u32) -> i32 {
    with_slot(h, |s| s.results.len() as i32)
}

/// The kind of issue `i`: 1 invalid value, 2 short circuit, 3 battery loop,
/// 4 floating node, 5 open circuit, 6 blown bulb.
#[no_mangle]
pub extern "C" fn circuit_issue_kind(h: u32, i: u32) -> i32 {
    with_slot(h, |s| match s.issue(i) {
        Ok(issue) => issue_kind_code(&issue),
        Err(e) => e,
    })
}

/// The part issue `i` points at: a component id, or a node id for a
/// floating node.
#[no_mangle]
pub extern "C" fn circuit_issue_part(h: u32, i: u32) -> i32 {
    with_slot(h, |s| match s.issue(i) {
        Ok(issue) => issue_part_index(&issue) as i32,
        Err(e) => e,
    })
}

/// Writes issue `i`'s explanation to `sb_out_ptr()`; returns its length.
#[no_mangle]
pub extern "C" fn circuit_issue_message(h: u32, i: u32) -> i32 {
    with_slot(h, |s| match s.issue(i) {
        Ok(issue) => output(issue.to_string()),
        Err(e) => e,
    })
}

/// Sets the integration method (0 = BackwardEuler, 1 = Trapezoidal).
#[no_mangle]
pub extern "C" fn circuit_set_integrator(h: u32, method: i32) -> i32 {
    with_slot(h, |s| {
        let integrator = if method == 1 {
            simbiguity_circuits::component::Integrator::Trapezoidal
        } else {
            simbiguity_circuits::component::Integrator::BackwardEuler
        };
        s.sim = s.sim.clone().with_integrator(integrator);
        OK
    })
}

#[no_mangle]
pub extern "C" fn circuit_plot_push(h: u32, v: f64) -> i32 {
    with_slot(h, |s| {
        s.chart_series.push(s.time, v);
        OK
    })
}

#[no_mangle]
pub extern "C" fn circuit_plot_draw(h: u32, width: f64, height: f64) -> i32 {
    with_slot(h, |s| {
        let t_max = s.time;
        let t_min = (t_max - 5.0).max(0.0);
        let axes = simbiguity_plot::Axes {
            x_min: t_min,
            x_max: t_min + 5.0,
            y_min: 0.0,
            y_max: 12.0,
            px_x: 0.0,
            px_y: 0.0,
            px_w: width,
            px_h: height,
        };
        let series_list = vec![(&s.chart_series, [1.0, 0.0, 0.0, 1.0])];
        let chart = simbiguity_plot::Chart {
            axes,
            series: series_list,
        };

        let mut cmd = simbiguity_render::CommandBuffer::new();
        let (meshes, labels) = chart.draw(&mut cmd, 0);

        s.chart_meshes = meshes;
        s.chart_commands = cmd.words().to_vec();

        // pack labels into JSON or custom string for plot-labels.js
        // plot-labels.js expects an array of [text, x, y].
        let mut js_labels = String::from("[");
        let mut first = true;
        for (text, x, y) in labels.iter() {
            if !x.is_finite() || !y.is_finite() {
                continue;
            }
            if !first {
                js_labels.push(',');
            }
            first = false;
            let escaped_text = text.replace('\\', "\\\\").replace('"', "\\\"");
            js_labels.push_str(&std::format!("[\"{}\",{},{}]", escaped_text, x, y));
        }
        js_labels.push(']');
        s.chart_labels = js_labels;

        OK
    })
}

#[no_mangle]
pub extern "C" fn circuit_plot_commands_ptr(h: u32) -> *const u32 {
    let mut ptr = core::ptr::null();
    with_slot(h, |s| {
        ptr = s.chart_commands.as_ptr();
        OK
    });
    ptr
}

#[no_mangle]
pub extern "C" fn circuit_plot_commands_len(h: u32) -> i32 {
    with_slot(h, |s| s.chart_commands.len() as i32)
}

#[no_mangle]
pub extern "C" fn circuit_plot_mesh_count(h: u32) -> i32 {
    with_slot(h, |s| s.chart_meshes.len() as i32)
}

#[no_mangle]
pub extern "C" fn circuit_plot_mesh_vertices_ptr(h: u32, i: u32) -> *const f32 {
    let mut ptr = core::ptr::null();
    with_slot(h, |s| {
        if let Some(m) = s.chart_meshes.get(i as usize) {
            ptr = m.vertices.as_ptr();
        }
        OK
    });
    ptr
}

#[no_mangle]
pub extern "C" fn circuit_plot_mesh_vertices_len(h: u32, i: u32) -> i32 {
    with_slot(h, |s| {
        if let Some(m) = s.chart_meshes.get(i as usize) {
            m.vertices.len() as i32
        } else {
            0
        }
    })
}

#[no_mangle]
pub extern "C" fn circuit_plot_mesh_indices_ptr(h: u32, i: u32) -> *const u32 {
    let mut ptr = core::ptr::null();
    with_slot(h, |s| {
        if let Some(m) = s.chart_meshes.get(i as usize) {
            ptr = m.indices.as_ptr();
        }
        OK
    });
    ptr
}

#[no_mangle]
pub extern "C" fn circuit_plot_mesh_indices_len(h: u32, i: u32) -> i32 {
    with_slot(h, |s| {
        if let Some(m) = s.chart_meshes.get(i as usize) {
            m.indices.len() as i32
        } else {
            0
        }
    })
}

#[no_mangle]
pub extern "C" fn circuit_plot_labels_ptr(h: u32) -> *const u8 {
    let mut ptr = core::ptr::null();
    with_slot(h, |s| {
        ptr = s.chart_labels.as_ptr();
        OK
    });
    ptr
}

#[no_mangle]
pub extern "C" fn circuit_plot_labels_len(h: u32) -> i32 {
    with_slot(h, |s| s.chart_labels.len() as i32)
}
