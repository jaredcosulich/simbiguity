//! How the animated state looks: pure functions from glow, flash and
//! current to colours and strengths. [`crate::CircuitView`] writes them
//! into the scene each frame.

use simbiguity_anim::Lerp;

/// A material's colour, opacity and self-light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Surface {
    /// Linear RGB.
    pub color: [f32; 3],
    /// Opacity (only the glass uses it).
    pub alpha: f32,
    /// Self-illumination.
    pub emissive: f32,
}

/// Everything about the bulb that changes as it warms, cools or blows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BulbLook {
    /// The glass envelope.
    pub glass: Surface,
    /// The intact filament (hidden once blown).
    pub filament: Surface,
    /// The halo around the bulb.
    pub halo: Surface,
    /// `true` shows the broken filament instead of the intact one.
    pub blown: bool,
}

const CLEAR_GLASS: [f32; 3] = [0.95, 0.96, 1.0];
const WARM_GLASS: [f32; 3] = [1.0, 0.86, 0.55];
const SMOKY_GLASS: [f32; 3] = [0.3, 0.27, 0.25];
const COLD_FILAMENT: [f32; 3] = [0.42, 0.36, 0.3];
const HOT_FILAMENT: [f32; 3] = [1.0, 0.82, 0.4];
const HALO: [f32; 3] = [1.0, 0.78, 0.36];
const FLASH_HALO: [f32; 3] = [1.0, 0.95, 0.85];
const RESTING_CHARGE: [f32; 3] = [0.35, 0.45, 0.6];
const FLOWING_CHARGE: [f32; 3] = [1.0, 0.82, 0.32];

/// The bulb at animated `glow` (brightness through its spring, may
/// overshoot past 1) and `flash` (the burst when it blows, decaying from 1).
/// Inputs are clamped to `glow ∈ [0, 1.5]` and `flash ∈ [0, 1]`.
pub fn bulb_look(glow: f64, flash: f64, blown: bool) -> BulbLook {
    let glow = glow.clamp(0.0, 1.5) as f32;
    let flash = flash.clamp(0.0, 1.0) as f32;
    let glass = if blown {
        Surface {
            color: SMOKY_GLASS,
            alpha: 0.62,
            emissive: flash * 0.8,
        }
    } else {
        Surface {
            color: lerp3(CLEAR_GLASS, WARM_GLASS, glow),
            alpha: 0.28 + 0.3 * glow.min(1.0),
            emissive: glow * 0.9,
        }
    };
    BulbLook {
        glass,
        filament: Surface {
            color: lerp3(COLD_FILAMENT, HOT_FILAMENT, glow.min(1.0)),
            alpha: 1.0,
            emissive: glow * 2.2,
        },
        halo: Surface {
            color: lerp3(HALO, FLASH_HALO, flash),
            alpha: 1.0,
            emissive: glow * 0.85 + flash * 1.6,
        },
        blown,
    }
}

/// The charge particles for a loop current of `amps`: dim blue-grey at
/// rest, amber when flowing, brighter as more flows (either direction).
pub fn charge_look(amps: f64) -> Surface {
    let flow = (amps.abs() / 0.5).min(1.6) as f32;
    Surface {
        color: lerp3(RESTING_CHARGE, FLOWING_CHARGE, flow.min(1.0)),
        alpha: 1.0,
        emissive: 0.35 + 0.65 * flow,
    }
}

/// Per-channel blend of two colours.
pub fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = f64::from(t);
    [
        f64::from(a[0]).lerp(f64::from(b[0]), t) as f32,
        f64::from(a[1]).lerp(f64::from(b[1]), t) as f32,
        f64::from(a[2]).lerp(f64::from(b[2]), t) as f32,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cold bulb has clear, mostly transparent glass, a dark filament and no halo.
    #[test]
    fn cold_bulb_is_clear_and_dark() {
        let b = bulb_look(0.0, 0.0, false);
        assert_eq!(b.glass.color, CLEAR_GLASS);
        assert_eq!(b.glass.alpha, 0.28);
        assert_eq!(b.filament.color, COLD_FILAMENT);
        assert_eq!((b.filament.emissive, b.halo.emissive), (0.0, 0.0));
    }

    /// A fully lit bulb has warm glass, a hot filament and a halo.
    #[test]
    fn lit_bulb_is_warm_and_glowing() {
        let b = bulb_look(1.0, 0.0, false);
        assert_eq!(b.glass.color, WARM_GLASS);
        assert_eq!(b.filament.color, HOT_FILAMENT);
        assert!((b.halo.emissive - 0.85).abs() < 1e-6);
        assert!(!b.blown);
    }

    /// More glow means brighter glass, filament and halo.
    #[test]
    fn glow_brightens_monotonically() {
        let (dim, bright) = (bulb_look(0.3, 0.0, false), bulb_look(0.7, 0.0, false));
        assert!(bright.glass.emissive > dim.glass.emissive);
        assert!(bright.filament.emissive > dim.filament.emissive);
        assert!(bright.halo.emissive > dim.halo.emissive);
    }

    /// A blown bulb has smoky glass lit only by the decaying flash.
    #[test]
    fn blown_bulb_is_smoky_and_flashes() {
        let b = bulb_look(0.0, 1.0, true);
        assert!(b.blown);
        assert_eq!(b.glass.color, SMOKY_GLASS);
        assert!((b.glass.emissive - 0.8).abs() < 1e-6);
        assert_eq!(b.halo.color, FLASH_HALO);
        assert_eq!(bulb_look(0.0, 0.0, true).glass.emissive, 0.0);
    }

    /// Out-of-range glow and flash are clamped.
    #[test]
    fn inputs_are_clamped() {
        assert_eq!(bulb_look(9.0, 9.0, false), bulb_look(1.5, 1.0, false));
        assert_eq!(bulb_look(-1.0, -1.0, false), bulb_look(0.0, 0.0, false));
    }

    /// Charges at rest are dim blue-grey; flowing ones are amber, whichever way they flow.
    #[test]
    fn charges_warm_with_current() {
        let rest = charge_look(0.0);
        assert_eq!(rest.color, RESTING_CHARGE);
        assert!((rest.emissive - 0.35).abs() < 1e-6);
        assert_eq!(charge_look(0.5).color, FLOWING_CHARGE);
        assert_eq!(charge_look(-0.5), charge_look(0.5));
        assert_eq!(charge_look(5.0), charge_look(0.8), "brightness saturates");
    }

    /// lerp3 blends each channel and hits both ends.
    #[test]
    fn lerp3_blends_channels() {
        let (a, b) = ([0.0, 0.5, 1.0], [1.0, 0.5, 0.0]);
        assert_eq!(lerp3(a, b, 0.0), a);
        assert_eq!(lerp3(a, b, 1.0), b);
        assert_eq!(lerp3(a, b, 0.5), [0.5, 0.5, 0.5]);
    }
}
