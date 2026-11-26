//! The readouts buffer: a view's numbers as a flat `f64` array JS reads
//! out of linear memory (`site/js/simbiguity-render.js` names each slot).

use simbiguity_circuits_view::Readouts;
use simbiguity_render::OrbitCamera;

/// `f64`s in the buffer: closed, battery volts, current, bulb volts, bulb
/// power, brightness, glow, particle speed, blown, solved, camera yaw
/// (degrees), camera pitch (degrees), camera distance, time.
pub const READOUT_COUNT: usize = 14;

/// Packs readouts, the camera and the animation time in buffer order.
/// Booleans are `1.0` or `0.0`; camera angles are in degrees.
pub fn pack_readouts(r: &Readouts, camera: &OrbitCamera, time: f64) -> [f64; READOUT_COUNT] {
    let b = |v: bool| f64::from(u8::from(v));
    [
        b(r.closed),
        r.battery_volts,
        r.current,
        r.bulb_volts,
        r.bulb_power,
        r.brightness,
        r.glow,
        r.particle_speed,
        b(r.blown),
        b(r.solved),
        camera.yaw.to_degrees(),
        camera.pitch.to_degrees(),
        camera.distance,
        time,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn readouts() -> Readouts {
        Readouts {
            closed: true,
            battery_volts: 12.0,
            current: 0.5,
            bulb_volts: 12.0,
            bulb_power: 6.0,
            brightness: 1.0,
            glow: 0.75,
            particle_speed: 1.2,
            blown: false,
            solved: true,
        }
    }

    /// Every field lands in its documented slot, booleans as one or zero.
    #[test]
    fn fields_land_in_their_slots() {
        let cam = OrbitCamera {
            yaw: core::f64::consts::PI,
            pitch: core::f64::consts::FRAC_PI_4,
            distance: 9.5,
            ..OrbitCamera::default()
        };
        let p = pack_readouts(&readouts(), &cam, 2.5);
        assert_eq!(
            &p[..10],
            &[1.0, 12.0, 0.5, 12.0, 6.0, 1.0, 0.75, 1.2, 0.0, 1.0]
        );
        assert!((p[10] - 180.0).abs() < 1e-9);
        assert!((p[11] - 45.0).abs() < 1e-9);
        assert_eq!((p[12], p[13]), (9.5, 2.5));
    }

    /// An open, blown, unsolved circuit packs its flags the other way round.
    #[test]
    fn flags_flip() {
        let r = Readouts {
            closed: false,
            blown: true,
            solved: false,
            ..readouts()
        };
        let p = pack_readouts(&r, &OrbitCamera::default(), 0.0);
        assert_eq!((p[0], p[8], p[9]), (0.0, 1.0, 0.0));
    }
}
