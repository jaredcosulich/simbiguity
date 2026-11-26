// What the 3D demo says about the circuit, from one set of readouts. Pure: no DOM.

import { BULB_VOLTS } from "./state.js";

/**
 * The explanation panel's heading and paragraph, and whether it reports a problem.
 * @param {{ blown: boolean, closed: boolean, brightness: number, batteryVolts: number, current: number }} r
 *   what `Circuit3D#readouts()` returns
 * @returns {{ title: string, body: string, issue: boolean }}
 */
export function explanation(r) {
  const amps = `${r.current.toFixed(2)} A`;
  if (r.blown) {
    return {
      title: "The bulb blew",
      body: `At ${r.batteryVolts.toFixed(1)} V the filament took more than twice its rated 6 W and burned through. `
        + "A broken filament is a gap, so no current can flow and the charges stop. Turn the voltage down and replace the bulb.",
      issue: true,
    };
  }
  if (!r.closed) {
    return {
      title: "The switch is open",
      body: "The loop has a gap, so no current flows: the charges in the wires stay where they are and the bulb is dark. "
        + "Click the switch to close it.",
      issue: false,
    };
  }
  if (r.brightness >= 0.999 && r.batteryVolts > BULB_VOLTS) {
    return {
      title: "Overdriven",
      body: `${amps} is more than the bulb is rated for, so the charges race around the loop and the bulb is at full glare. `
        + "Much more and the filament will burn out.",
      issue: false,
    };
  }
  return {
    title: "Current is flowing",
    body: `The battery pushes ${amps} around the loop. Each glowing dot is a bit of charge; `
      + "they move faster when more current flows, and the bulb glows as bright as the power it absorbs.",
    issue: false,
  };
}
