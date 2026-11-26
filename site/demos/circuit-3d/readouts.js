// The 3D demo's readout labels, from one set of readouts. Pure: no DOM.

/**
 * Every label in the readouts panel, and what the buttons say and show.
 * @param {{ closed: boolean, blown: boolean, batteryVolts: number, current: number, bulbPower: number,
 *   brightness: number, particleSpeed: number }} r what `Circuit3D#readouts()` returns
 */
export function formatReadouts(r) {
  const fmt = (x, unit, d = 2) => `${x.toFixed(d)} ${unit}`;
  return {
    volts: fmt(r.batteryVolts, "V", 1),
    current: fmt(r.current, "A"),
    power: fmt(r.bulbPower, "W"),
    bright: `${Math.round(r.brightness * 100)} %`,
    speed: `${r.particleSpeed.toFixed(2)} u/s`,
    toggleLabel: r.closed ? "Open the switch" : "Close the switch",
    showReplace: r.blown,
  };
}
