//! The parts a learner can place, and how each one enters the equations.

/// How much more than its rated power a bulb survives. Above
/// `BLOW_RATIO × rated_power` the filament burns out. (Power grows with the
/// square of voltage, so this is about 1.41 × the rated voltage.)
pub const BLOW_RATIO: f64 = 2.0;

/// Integration method for transient analysis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Integrator {
    /// Backward Euler integration: stable but adds artificial damping.
    BackwardEuler,
    /// Trapezoidal integration: more accurate, preserves energy, but can ring.
    Trapezoidal,
}

/// A circuit element.
/// Transistor polarity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// NPN transistor.
    Npn,
    /// PNP transistor.
    Pnp,
}

/// A circuit element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Component {
    /// A bipolar junction transistor.
    Bjt {
        /// NPN or PNP.
        polarity: Polarity,
        /// Saturation current (Is) in amps.
        saturation_amps: f64,
        /// Forward current gain.
        beta_f: f64,
        /// Reverse current gain.
        beta_r: f64,
    },
    /// A battery. Terminal `a` is `+`, terminal `b` is `−`.
    Battery {
        /// Electromotive force in volts: the voltage with no current flowing.
        emf: f64,
        /// Internal resistance in ohms. `0.0` is an ideal battery.
        internal_r: f64,
    },
    /// A resistor of `r` ohms.
    Resistor {
        /// Resistance in ohms (must be positive).
        r: f64,
    },
    /// An incandescent bulb: a resistor whose resistance comes from its
    /// rating, `R = V² / P`, and whose brightness is `P_actual / P_rated`.
    Bulb {
        /// The voltage at which it glows at full brightness.
        rated_voltage: f64,
        /// The power it draws at its rated voltage, in watts.
        rated_power: f64,
        /// A blown bulb is a broken filament: it no longer conducts.
        blown: bool,
    },
    /// A switch: a wire when closed, a gap when open.
    Switch {
        /// Whether the switch is closed (conducting).
        closed: bool,
    },
    /// An ideal wire: its two ends are the same node.
    Wire,
    /// A diode (Shockley model).
    Diode {
        /// Saturation current in amps (Is).
        saturation_amps: f64,
        /// Emission coefficient (n).
        emission: f64,
    },
    /// An LED.
    Led {
        /// Color for the 3D view.
        color: &'static str,
        /// Rated current in amps.
        rated_amps: f64,
        /// Saturation current in amps (Is).
        saturation_amps: f64,
        /// Emission coefficient (n).
        emission: f64,
        /// A blown LED is an open circuit.
        blown: bool,
    },
    /// A capacitor.
    Capacitor {
        /// Capacitance in farads.
        farads: f64,
        /// Voltage across the capacitor from the previous step.
        volts: f64,
        /// Current through the capacitor from the previous step.
        amps: f64,
    },
    /// An inductor.
    Inductor {
        /// Inductance in henries.
        henries: f64,
        /// Current through the inductor from the previous step.
        amps: f64,
        /// Voltage across the inductor from the previous step.
        volts: f64,
    },
    /// A signal source (sine, square, or triangle).
    SignalSource {
        /// 0: sine, 1: square, 2: triangle.
        waveform: u8,
        /// Amplitude in volts (zero-to-peak).
        amplitude: f64,
        /// Frequency in Hz.
        frequency: f64,
        /// Phase in radians.
        phase: f64,
        /// DC offset in volts.
        offset: f64,
    },
    /// A linear voltage-controlled voltage source.
    Vcvs {
        /// Voltage gain.
        gain: f64,
    },
    /// An operational amplifier with tanh saturation.
    OpAmp {
        /// Linear voltage gain near zero.
        gain: f64,
        /// Output voltage saturation limit (V_sat).
        v_sat: f64,
    },
}

/// How one component enters the MNA system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Stamp {
    /// Joins its two nodes into one (wires, closed switches).
    Merge,
    /// A conductance `g = 1/R` between its nodes (resistors, intact bulbs).
    Conductance(f64),
    /// A battery with internal resistance, as its Norton equivalent: a
    /// current source of `emf / r` into `+` in parallel with `1 / r`.
    Norton {
        /// `1 / internal_r`.
        g: f64,
        /// Short-circuit current `emf / internal_r`.
        current: f64,
    },
    /// An ideal voltage source: adds an unknown current and the equation
    /// `V_+ − V_− = emf`.
    VoltageSource(f64),
    /// Conducts nothing (open switches, blown bulbs).
    Open,
    /// A 3-terminal linearized BJT companion.
    Bjt {
        g_b_be: f64,
        g_b_bc: f64,
        g_c_be: f64,
        g_c_bc: f64,
        i_eq_b: f64,
        i_eq_c: f64,
    },
    /// A voltage-controlled voltage source, possibly with offset.
    Vcvs { gain: f64, offset: f64 },
    /// An operational amplifier (3 terminals: plus, minus, out).
    OpAmp { eq_gain: f64, offset: f64 },
}

impl Component {
    /// A capacitor with no initial charge.
    pub fn capacitor(farads: f64) -> Component {
        Component::Capacitor {
            farads,
            volts: 0.0,
            amps: 0.0,
        }
    }

    /// An inductor with no initial current.
    pub fn inductor(henries: f64) -> Component {
        Component::Inductor {
            henries,
            amps: 0.0,
            volts: 0.0,
        }
    }

    /// An ideal battery (no internal resistance).
    pub fn battery(emf: f64) -> Component {
        Component::Battery {
            emf,
            internal_r: 0.0,
        }
    }

    /// A new (not blown) bulb with the given rating.
    pub fn bulb(rated_voltage: f64, rated_power: f64) -> Component {
        Component::Bulb {
            rated_voltage,
            rated_power,
            blown: false,
        }
    }

    /// Resistance of a bulb from its rating, `R = V² / P`.
    pub fn bulb_resistance(rated_voltage: f64, rated_power: f64) -> f64 {
        rated_voltage * rated_voltage / rated_power
    }

    /// `true` when every value is physically meaningful: finite, resistances
    /// and ratings positive, internal resistance not negative.
    pub fn is_valid(&self) -> bool {
        match *self {
            Component::Battery { emf, internal_r } => {
                emf.is_finite() && internal_r.is_finite() && internal_r >= 0.0
            }
            Component::Resistor { r } => r.is_finite() && r > 0.0,
            Component::Bulb {
                rated_voltage,
                rated_power,
                ..
            } => {
                rated_voltage.is_finite()
                    && rated_power.is_finite()
                    && rated_voltage > 0.0
                    && rated_power > 0.0
            }
            Component::Diode {
                saturation_amps,
                emission,
            } => {
                saturation_amps.is_finite()
                    && saturation_amps > 0.0
                    && emission.is_finite()
                    && emission > 0.0
            }
            Component::Led {
                rated_amps,
                saturation_amps,
                emission,
                ..
            } => {
                rated_amps.is_finite()
                    && rated_amps > 0.0
                    && saturation_amps.is_finite()
                    && saturation_amps > 0.0
                    && emission.is_finite()
                    && emission > 0.0
            }
            Component::Switch { .. } | Component::Wire => true,
            Component::Capacitor { farads, .. } => farads.is_finite() && farads > 0.0,
            Component::Inductor { henries, .. } => henries.is_finite() && henries > 0.0,
            Component::SignalSource {
                waveform,
                amplitude,
                frequency,
                phase,
                offset,
            } => {
                waveform <= 2
                    && amplitude.is_finite()
                    && frequency.is_finite()
                    && frequency > 0.0
                    && phase.is_finite()
                    && offset.is_finite()
            }
            Component::Bjt {
                saturation_amps,
                beta_f,
                beta_r,
                ..
            } => {
                saturation_amps.is_finite()
                    && saturation_amps > 0.0
                    && beta_f.is_finite()
                    && beta_f > 0.0
                    && beta_r.is_finite()
                    && beta_r > 0.0
            }
            Component::Vcvs { gain } => gain.is_finite(),
            Component::OpAmp { gain, v_sat } => {
                gain.is_finite() && v_sat.is_finite() && v_sat > 0.0
            }
        }
    }

    /// How this component enters the system for transient analysis.
    pub(crate) fn transient_stamp(&self, dt: f64, integrator: Integrator, t_eval: f64) -> Stamp {
        match *self {
            Component::Capacitor {
                farads,
                volts,
                amps,
            } => match integrator {
                Integrator::BackwardEuler => {
                    let g = farads / dt;
                    Stamp::Norton {
                        g,
                        current: g * volts,
                    }
                }
                // Trapezoidal derivation: i_{n+1} + i_n = (2C/dt) * (v_{n+1} - v_n).
                // Rearranged for Norton equivalent (where i_passive = g * v_{n+1} - I_source):
                // i_{n+1} = (2C/dt) * v_{n+1} - (2C/dt * v_n + i_n).
                // Thus g = 2C/dt, and current = g * volts + amps.
                Integrator::Trapezoidal => {
                    let g = 2.0 * farads / dt;
                    Stamp::Norton {
                        g,
                        current: g * volts + amps,
                    }
                }
            },
            Component::Inductor {
                henries,
                amps,
                volts,
            } => match integrator {
                Integrator::BackwardEuler => {
                    let g = dt / henries;
                    Stamp::Norton { g, current: -amps }
                }
                // Trapezoidal derivation: v_{n+1} + v_n = (2L/dt) * (i_{n+1} - i_n).
                // Rearranged for i_{n+1}: i_{n+1} = (dt/2L) * v_{n+1} + (dt/2L * v_n + i_n).
                // Since Norton passive readback is i_{passive} = g * v_{n+1} - current,
                // we need -current = dt/2L * v_n + i_n, so current = -(amps + g * volts).
                Integrator::Trapezoidal => {
                    let g = dt / (2.0 * henries);
                    Stamp::Norton {
                        g,
                        current: -(amps + g * volts),
                    }
                }
            },
            _ => self.stamp(t_eval),
        }
    }

    /// How this component enters the system. Assumes [`Component::is_valid`].
    pub(crate) fn stamp(&self, t_eval: f64) -> Stamp {
        match *self {
            Component::Battery { emf, internal_r } if internal_r > 0.0 => Stamp::Norton {
                g: 1.0 / internal_r,
                current: emf / internal_r,
            },
            Component::Battery { emf, .. } => Stamp::VoltageSource(emf),
            Component::Resistor { r } => Stamp::Conductance(1.0 / r),
            Component::Bulb { blown: true, .. } => Stamp::Open,
            Component::Bulb {
                rated_voltage,
                rated_power,
                ..
            } => Stamp::Conductance(rated_power / (rated_voltage * rated_voltage)),
            Component::Switch { closed: true } | Component::Wire => Stamp::Merge,
            Component::Switch { closed: false } => Stamp::Open,
            Component::Diode { .. } => Stamp::Open,
            Component::Led { blown: true, .. } => Stamp::Open,
            Component::Led { .. } => Stamp::Open,
            Component::Vcvs { gain } => Stamp::Vcvs { gain, offset: 0.0 },
            Component::OpAmp { .. } => Stamp::Vcvs {
                gain: 0.0,
                offset: 0.0,
            },
            Component::Bjt { .. } => Stamp::Bjt {
                g_b_be: 0.0,
                g_b_bc: 0.0,
                g_c_be: 0.0,
                g_c_bc: 0.0,
                i_eq_b: 0.0,
                i_eq_c: 0.0,
            },
            Component::Capacitor { .. } => Stamp::Open,
            Component::Inductor { .. } => Stamp::Merge,
            Component::SignalSource {
                waveform,
                amplitude,
                frequency,
                phase,
                offset,
            } => {
                let wt_phi = 2.0 * core::f64::consts::PI * frequency * t_eval + phase;
                let val = match waveform {
                    0 => simbiguity_math::float::sin(wt_phi),
                    1 => {
                        if simbiguity_math::float::sin(wt_phi) >= 0.0 {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                    2 => {
                        let mut phase_mod =
                            (frequency * t_eval + phase / (2.0 * core::f64::consts::PI)) % 1.0;
                        if phase_mod < 0.0 {
                            phase_mod += 1.0;
                        }
                        if phase_mod < 0.25 {
                            phase_mod * 4.0
                        } else if phase_mod < 0.75 {
                            1.0 - (phase_mod - 0.25) * 4.0
                        } else {
                            -1.0 + (phase_mod - 0.75) * 4.0
                        }
                    }
                    _ => 0.0,
                };
                Stamp::VoltageSource(offset + amplitude * val)
            }
        }
    }

    /// A bulb's brightness when it absorbs `power` watts: `power / rated`,
    /// clamped to `0..=1`, and `0` once blown. `None` for anything else.
    pub fn brightness(&self, power: f64, current: f64) -> Option<f64> {
        match *self {
            Component::Bulb { blown: true, .. } => Some(0.0),
            Component::Bulb { rated_power, .. } => Some((power / rated_power).clamp(0.0, 1.0)),
            Component::Led { blown: true, .. } => Some(0.0),
            Component::Led { rated_amps, .. } => Some((current.abs() / rated_amps).clamp(0.0, 1.5)),
            _ => None,
        }
    }

    /// Burns out a bulb's filament. Anything that is not a bulb is unchanged.
    pub fn blow(&mut self) {
        match self {
            Component::Bulb { blown, .. } => *blown = true,
            Component::Led { blown, .. } => *blown = true,
            _ => {}
        }
    }

    /// Returns the saturation current and emission coefficient if this is a diode or intact LED.
    pub fn diode_params(&self) -> Option<(f64, f64)> {
        match self {
            Component::Diode {
                saturation_amps,
                emission,
            } => Some((*saturation_amps, *emission)),
            Component::Led {
                saturation_amps,
                emission,
                blown: false,
                ..
            } => Some((*saturation_amps, *emission)),
            _ => None,
        }
    }

    /// Returns whether this component requires Newton-Raphson iteration.
    pub fn is_nonlinear(&self) -> bool {
        self.diode_params().is_some()
            || matches!(self, Component::Bjt { .. } | Component::OpAmp { .. })
    }

    /// Sets the component's main value: a battery's EMF, a resistor's
    /// resistance, a bulb's rated voltage, a switch's state (`0.0` = open,
    /// anything else = closed). A wire has no value and is left unchanged.
    pub fn set_value(&mut self, v: f64) {
        match self {
            Component::Battery { emf, .. } => *emf = v,
            Component::Resistor { r } => *r = v,
            Component::Bulb { rated_voltage, .. } => *rated_voltage = v,
            Component::Switch { closed } => *closed = v != 0.0,
            Component::Capacitor { farads, .. } => *farads = v,
            Component::Inductor { henries, .. } => *henries = v,
            Component::Wire
            | Component::Diode { .. }
            | Component::Led { .. }
            | Component::Bjt { .. }
            | Component::Vcvs { .. }
            | Component::OpAmp { .. }
            | Component::SignalSource { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Brightness is power over rated power, clamped to the 0..=1 range.
    #[test]
    fn brightness_is_power_ratio_clamped() {
        let b = Component::bulb(6.0, 3.0);
        assert_eq!(b.brightness(0.75, 0.0), Some(0.25));
        assert_eq!(b.brightness(6.0, 0.0), Some(1.0));
        assert_eq!(b.brightness(-1.0, 0.0), Some(0.0));
    }

    /// A blown bulb is dark whatever the power; non-bulbs have no brightness.
    #[test]
    fn blown_bulbs_are_dark_and_others_have_none() {
        let mut b = Component::bulb(6.0, 3.0);
        b.blow();
        assert_eq!(b.brightness(3.0, 0.5), Some(0.0));
        assert_eq!(Component::Resistor { r: 1.0 }.brightness(3.0, 0.5), None);
        assert_eq!(Component::Wire.brightness(3.0, 0.5), None);
    }

    /// Blowing marks a bulb blown and keeps its rating; anything else is
    /// unchanged.
    #[test]
    fn blow_only_affects_bulbs() {
        let mut b = Component::bulb(6.0, 3.0);
        b.blow();
        let expected = Component::Bulb {
            rated_voltage: 6.0,
            rated_power: 3.0,
            blown: true,
        };
        assert_eq!(b, expected);
        let mut r = Component::Resistor { r: 2.0 };
        r.blow();
        assert_eq!(r, Component::Resistor { r: 2.0 });
    }

    /// Each component enters the system in the documented way.
    #[test]
    fn stamps_match_each_component() {
        assert_eq!(
            Component::battery(6.0).stamp(0.0),
            Stamp::VoltageSource(6.0)
        );
        assert_eq!(
            Component::Battery {
                emf: 6.0,
                internal_r: 2.0
            }
            .stamp(0.0),
            Stamp::Norton {
                g: 0.5,
                current: 3.0
            }
        );
        assert_eq!(
            Component::Resistor { r: 4.0 }.stamp(0.0),
            Stamp::Conductance(0.25)
        );
        assert_eq!(
            Component::bulb(6.0, 3.0).stamp(0.0),
            Stamp::Conductance(1.0 / 12.0)
        );
        assert_eq!(Component::Switch { closed: true }.stamp(0.0), Stamp::Merge);
        assert_eq!(Component::Switch { closed: false }.stamp(0.0), Stamp::Open);
        assert_eq!(Component::Wire.stamp(0.0), Stamp::Merge);
        let mut blown = Component::bulb(6.0, 3.0);
        blown.blow();
        assert_eq!(blown.stamp(0.0), Stamp::Open);
    }

    /// Non-physical values are rejected; physical ones accepted.
    #[test]
    fn validity_rejects_non_physical_values() {
        assert!(Component::battery(-3.0).is_valid());
        assert!(!Component::Battery {
            emf: 1.0,
            internal_r: -1.0
        }
        .is_valid());
        assert!(!Component::battery(f64::INFINITY).is_valid());
        assert!(!Component::Resistor { r: 0.0 }.is_valid());
        assert!(!Component::bulb(0.0, 1.0).is_valid());
        assert!(!Component::bulb(1.0, f64::NAN).is_valid());
        assert!(Component::Wire.is_valid());
    }

    /// set_value changes each component's main value; a wire ignores it.
    #[test]
    fn set_value_changes_the_main_value() {
        let mut b = Component::battery(1.0);
        b.set_value(9.0);
        assert_eq!(b, Component::battery(9.0));
        let mut s = Component::Switch { closed: true };
        s.set_value(0.0);
        assert_eq!(s, Component::Switch { closed: false });
        let mut w = Component::Wire;
        w.set_value(5.0);
        assert_eq!(w, Component::Wire);
    }

    /// R = V² / P for a bulb.
    /// diode_params returns correct values for non-linear components and None for others.
    #[test]
    fn diode_params_and_nonlinear_check() {
        let d = Component::Diode {
            saturation_amps: 1e-14,
            emission: 1.5,
        };
        assert_eq!(d.diode_params(), Some((1e-14, 1.5)));
        assert!(d.is_nonlinear());

        let l = Component::Led {
            color: "red",
            rated_amps: 0.02,
            saturation_amps: 1e-20,
            emission: 2.0,
            blown: false,
        };
        assert_eq!(l.diode_params(), Some((1e-20, 2.0)));
        assert!(l.is_nonlinear());

        let blown_l = Component::Led {
            color: "red",
            rated_amps: 0.02,
            saturation_amps: 1e-20,
            emission: 2.0,
            blown: true,
        };
        assert_eq!(blown_l.diode_params(), None);
        assert!(!blown_l.is_nonlinear());

        let r = Component::Resistor { r: 100.0 };
        assert_eq!(r.diode_params(), None);
        assert!(!r.is_nonlinear());
    }

    /// R = V² / P for a bulb.
    #[test]
    fn bulb_resistance_from_rating() {
        assert_eq!(Component::bulb_resistance(6.0, 3.0), 12.0);
    }
}
