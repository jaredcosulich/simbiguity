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

/// A two-terminal circuit element. Terminal `a` and terminal `b` are the
/// nodes it is placed between (see [`crate::Placed`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Component {
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
            Component::Switch { .. } | Component::Wire => true,
            Component::Capacitor { farads, .. } => farads.is_finite() && farads > 0.0,
            Component::Inductor { henries, .. } => henries.is_finite() && henries > 0.0,
        }
    }

    /// How this component enters the system for transient analysis.
    pub(crate) fn transient_stamp(&self, dt: f64, integrator: Integrator) -> Stamp {
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
                    Stamp::Norton { g, current: amps }
                }
                Integrator::Trapezoidal => {
                    let g = dt / (2.0 * henries);
                    Stamp::Norton {
                        g,
                        current: amps + g * volts,
                    }
                }
            },
            _ => self.stamp(),
        }
    }

    /// How this component enters the system. Assumes [`Component::is_valid`].
    pub(crate) fn stamp(&self) -> Stamp {
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
            Component::Capacitor { .. } => Stamp::Open,
            Component::Inductor { .. } => Stamp::Merge,
        }
    }

    /// A bulb's brightness when it absorbs `power` watts: `power / rated`,
    /// clamped to `0..=1`, and `0` once blown. `None` for anything else.
    pub fn brightness(&self, power: f64) -> Option<f64> {
        match *self {
            Component::Bulb { blown: true, .. } => Some(0.0),
            Component::Bulb { rated_power, .. } => Some((power / rated_power).clamp(0.0, 1.0)),
            _ => None,
        }
    }

    /// Burns out a bulb's filament. Anything that is not a bulb is unchanged.
    pub fn blow(&mut self) {
        if let Component::Bulb { blown, .. } = self {
            *blown = true;
        }
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
            Component::Wire => {}
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
        assert_eq!(b.brightness(0.75), Some(0.25));
        assert_eq!(b.brightness(6.0), Some(1.0));
        assert_eq!(b.brightness(-1.0), Some(0.0));
    }

    /// A blown bulb is dark whatever the power; non-bulbs have no brightness.
    #[test]
    fn blown_bulbs_are_dark_and_others_have_none() {
        let mut b = Component::bulb(6.0, 3.0);
        b.blow();
        assert_eq!(b.brightness(3.0), Some(0.0));
        assert_eq!(Component::Resistor { r: 1.0 }.brightness(3.0), None);
        assert_eq!(Component::Wire.brightness(3.0), None);
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
        assert_eq!(Component::battery(6.0).stamp(), Stamp::VoltageSource(6.0));
        assert_eq!(
            Component::Battery {
                emf: 6.0,
                internal_r: 2.0
            }
            .stamp(),
            Stamp::Norton {
                g: 0.5,
                current: 3.0
            }
        );
        assert_eq!(
            Component::Resistor { r: 4.0 }.stamp(),
            Stamp::Conductance(0.25)
        );
        assert_eq!(
            Component::bulb(6.0, 3.0).stamp(),
            Stamp::Conductance(1.0 / 12.0)
        );
        assert_eq!(Component::Switch { closed: true }.stamp(), Stamp::Merge);
        assert_eq!(Component::Switch { closed: false }.stamp(), Stamp::Open);
        assert_eq!(Component::Wire.stamp(), Stamp::Merge);
        let mut blown = Component::bulb(6.0, 3.0);
        blown.blow();
        assert_eq!(blown.stamp(), Stamp::Open);
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
    #[test]
    fn bulb_resistance_from_rating() {
        assert_eq!(Component::bulb_resistance(6.0, 3.0), 12.0);
    }
}
