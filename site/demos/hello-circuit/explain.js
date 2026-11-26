// Says, in plain words, why the bulb looks the way it does. Pure: no DOM.

import { BULB_VOLTS, BULB_WATTS } from "./state.js";

/** The bulb filament's resistance, from its rating: R = V² / P. */
export const BULB_OHMS = (BULB_VOLTS * BULB_VOLTS) / BULB_WATTS;

/**
 * The explanation panel's text.
 * @param {{ closed: boolean, volts: number }} state
 * @param {{ blown: boolean, shorted: boolean, brightness: number, amps: number | null }} view
 *   from `readouts()`
 * @param {Array<{ message: string }>} issues the engine's issues, quoted verbatim
 * @returns {{ title: string, body: string, issue: boolean, engine: string[] }}
 */
export function explanation(state, { blown, shorted, brightness, amps }, issues) {
  const v = state.volts.toFixed(1);
  let title;
  let body;
  if (shorted) {
    title = "Short circuit";
    body = "The bypass wire joins the battery's terminals with nothing in between. An ideal battery " +
      "would push an unlimited current through it, so there is no answer to give: the engine " +
      "reports the short instead of a number. (A real battery would get hot and drain fast.) " +
      "Every bit of current takes the bypass, so the bulb gets none.";
  } else if (blown) {
    const watts = (state.volts * state.volts) / BULB_OHMS;
    title = "The bulb blew";
    body = `At ${v} V the bulb would draw ${watts.toFixed(1)} W, more than twice its ` +
      `${BULB_WATTS} W rating, so the filament burned out. Power grows with the square of the voltage ` +
      `(P = V² / R), so a bulb survives only about 1.4× its rated voltage. A broken filament is a gap, ` +
      `so no current flows now. Turn the voltage down and replace the bulb.`;
  } else if (!state.closed) {
    title = "Open circuit";
    body = "The switch is open, so there is a gap in the loop. Current needs a complete path from the " +
      "battery's + back to its −, so none flows and the bulb stays dark. Click the switch.";
  } else if (amps !== null && amps > 0) {
    title = brightness >= 1 ? "Full brightness" : brightness < 0.25 ? "Dim" : "Lit";
    body = `The loop is complete. The bulb's filament is ${BULB_OHMS} Ω, so Ohm's law gives ` +
      `I = V / R = ${v} / ${BULB_OHMS} = ${amps.toFixed(2)} A, and it turns ` +
      `P = V × I = ${(state.volts * amps).toFixed(2)} W into light and heat. ` +
      (brightness >= 1
        ? `That is at or above its ${BULB_WATTS} W rating. Push much further and it will blow.`
        : `That is ${Math.round(brightness * 100)} % of its ${BULB_WATTS} W rating.`);
  } else {
    title = "No current";
    body = "The loop is complete but the battery is at 0 V, so nothing pushes current around it.";
  }
  return { title, body, issue: shorted || blown, engine: issues.map((i) => i.message) };
}
