// What the demo shows, derived from the engine's answer. Pure: no DOM.

/**
 * Formats a reading, or an em dash when there is none.
 * @param {number | null} x
 * @param {string} unit
 * @param {number} [digits]
 * @returns {string}
 */
export function formatReading(x, unit, digits = 2) {
  return x === null || x === undefined ? "—" : `${x.toFixed(digits)} ${unit}`;
}

/**
 * Everything the page draws, from one `circuit.solve()`.
 * @param {{ closed: boolean, short: boolean, volts: number }} state
 * @param {Array<{across:number|null,current:number|null,power:number|null,brightness:number|null}> | null} parts
 *   the solved parts, or null when the engine found no solution
 * @param {Array<{kind:string, part:number, message:string}>} issues
 * @param {{ battery: number, bulb: number }} ids
 */
export function readouts(state, parts, issues, ids) {
  const blown = issues.some((i) => i.kind === "bulb-blown" && i.part === ids.bulb);
  const shorted = issues.some((i) => i.kind === "short-circuit");
  const bulb = parts?.[ids.bulb];
  const battery = parts?.[ids.battery];
  const brightness = bulb?.brightness ?? 0;
  const amps = bulb?.current ?? null;
  return {
    blown,
    shorted,
    brightness,
    amps,
    flowing: !!battery && Math.abs(battery.current ?? 0) > 1e-9,
    batteryLabel: `${state.volts.toFixed(1)} V`,
    volts: formatReading(battery?.across ?? null, "V", 1),
    current: formatReading(amps, "A"),
    power: formatReading(bulb?.power ?? null, "W"),
    bright: parts ? `${Math.round(brightness * 100)} %` : "—",
    toggleLabel: state.closed ? "Open the switch" : "Close the switch",
  };
}
