// The controls panel: readouts, the voltage label and the two buttons.

const $ = (id) => document.getElementById(id);

/**
 * Writes formatted readouts into the panel.
 * @param {ReturnType<typeof import("./readouts.js").formatReadouts>} d
 */
export function showReadouts(d) {
  $("volts-value").textContent = d.volts;
  $("r-volts").textContent = d.volts;
  $("r-amps").textContent = d.current;
  $("r-watts").textContent = d.power;
  $("r-bright").textContent = d.bright;
  $("r-speed").textContent = d.speed;
  $("toggle").textContent = d.toggleLabel;
  $("replace").hidden = !d.showReplace;
}
