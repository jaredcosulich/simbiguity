// Hello, circuit: the smallest complete embedding of simbiguity-circuits.
//
// 1. Load the WASM bridge.  2. Build a circuit once.  3. On every change,
// solve it and draw what the engine says. The page holds no physics of its
// own: every number on screen comes back from `circuit.solve()`.

import { loadCircuits } from "../../js/simbiguity.js";
import { explanation } from "./explain.js";
import { readouts } from "./readouts.js";
import { BULB_VOLTS, BULB_WATTS, hashFromState, stateFromHash } from "./state.js";

const $ = (id) => document.getElementById(id);

async function main() {
  const { Circuit } = await loadCircuits(new URL("../../pkg/simbiguity_circuits_wasm.wasm", import.meta.url));
  const state = stateFromHash(location.hash);

  // Battery + → main switch → bulb → battery −, plus a bypass switch that
  // puts a bare wire across the bulb.
  const c = new Circuit();
  const plus = c.node();
  const minus = c.node();
  const bulbTop = c.node();
  const battery = c.battery(plus, minus, state.volts);
  const mainSwitch = c.switch(plus, bulbTop, state.closed);
  const bulb = c.bulb(bulbTop, minus, BULB_VOLTS, BULB_WATTS);
  const bypass = c.switch(bulbTop, minus, state.short);

  function update() {
    const { solved, parts, issues } = c.solve();
    draw(state, solved ? parts : null, issues, { battery, bulb });
    history.replaceState(null, "", location.pathname + location.search + hashFromState(state));
  }

  $("toggle").onclick = $("switch").onclick = () => {
    state.closed = c.toggle(mainSwitch);
    update();
  };
  $("bypass").onclick = () => {
    state.short = c.toggle(bypass);
    update();
  };
  $("volts").value = String(state.volts);
  $("volts").oninput = (e) => {
    state.volts = Number(e.target.value);
    c.setValue(battery, state.volts);
    update();
  };
  $("replace").onclick = () => {
    c.replaceBulb(bulb);
    update();
  };
  for (const id of ["switch", "bypass"]) {
    $(id).onkeydown = (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        $(id).onclick();
      }
    };
  }
  update();
}

/** Puts the engine's answer on screen. `parts` is null when there is no solution. */
function draw(state, parts, issues, ids) {
  const view = readouts(state, parts, issues, ids);
  const { blown, brightness, flowing } = view;

  // Circuit drawing
  $("switch-lever").setAttribute("y2", state.closed ? "60" : "26");
  $("switch-lever").setAttribute("x2", state.closed ? "290" : "284");
  $("switch-label").textContent = `switch: ${state.closed ? "closed" : "open"}`;
  $("bypass-wire").classList.toggle("closed", state.short);
  $("bypass-label").textContent = `bypass: ${state.short ? "on" : "off"}`;
  $("battery-label").textContent = view.batteryLabel;
  $("bulb-glow").setAttribute("opacity", String(brightness));
  $("bulb").classList.toggle("blown", blown);
  $("filament").setAttribute("d", blown
    ? "M 384 200 L 384 165 Q 390 146 396 158 M 404 158 Q 410 146 416 165 L 416 200"
    : "M 384 200 L 384 165 Q 392 140 400 165 Q 408 140 416 165 L 416 200");
  for (const w of ["w-top-left", "w-top-right", "w-bottom"]) {
    $(w).classList.toggle("live", flowing);
  }

  // Readouts
  $("volts-value").textContent = view.batteryLabel;
  $("r-volts").textContent = view.volts;
  $("r-amps").textContent = view.current;
  $("r-watts").textContent = view.power;
  $("r-bright").textContent = view.bright;
  $("toggle").textContent = view.toggleLabel;
  $("replace").hidden = !blown;

  showExplanation(explanation(state, view, issues));
}

/** Fills the explanation panel. Engine text goes in as text, never as HTML. */
function showExplanation({ title, body, issue, engine }) {
  const box = $("explain");
  box.className = `explain panel${issue ? " issue" : ""}`;
  const h2 = document.createElement("h2");
  h2.textContent = title;
  const p = document.createElement("p");
  p.textContent = body;
  const lines = engine.map((message) => {
    const line = document.createElement("p");
    line.className = "engine";
    line.textContent = `engine: ${message}`;
    return line;
  });
  box.replaceChildren(h2, p, ...lines);
}

main().catch((err) => {
  const h2 = document.createElement("h2");
  h2.className = "error";
  h2.textContent = "The engine did not load";
  const p = document.createElement("p");
  p.textContent = err.message;
  $("explain").replaceChildren(h2, p);
  throw err;
});
