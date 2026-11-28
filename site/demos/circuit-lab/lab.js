import { boot } from "../../js/boot.js";
import { loadCircuits } from "../../js/simbiguity.js";
import { BoardView } from "./components/board-view.js";
import { ExperimentControls } from "./components/experiment-controls.js";
import { ScopeView } from "./components/scope-view.js";
import { EXPERIMENTS } from "./experiments.js";
import { readState, writeState } from "./state.js";

async function main() {
  const { Circuit } = await loadCircuits("../../pkg/simbiguity_circuits_wasm.wasm");
  const boardView = new BoardView("board-canvas");
  await boardView.init();
  
  const scopeView = new ScopeView(document.getElementById("scope-canvas"));
  
  let loopId = null;
  let currentCircuit = null;
  
  const controls = new ExperimentControls(
    "experiment-select",
    "error-text",
    "readouts-container",
    (expName, params) => {
      writeState(expName, params);
      loadExperiment(expName, params);
    }
  );
  
  function loadExperiment(name, params) {
    if (loopId) cancelAnimationFrame(loopId);
    if (currentCircuit) currentCircuit.free();
    
    const exp = EXPERIMENTS[name];
    if (!exp) return;
    
    currentCircuit = new Circuit();
    let setupData;
    
    try {
      setupData = exp.setup(currentCircuit, params);
      boardView.loadLayout(setupData.layout);
      controls.clearError();
    } catch (e) {
      controls.showError(e.message);
      return;
    }
    
    let last = performance.now();
    const loop = (now) => {
      const dt = Math.min(0.25, (now - last) / 1000);
      last = now;
      
      const simDt = dt * (params.timeScale || 1.0);
      let remaining = simDt;
      const stepSize = setupData.sampleTime || 1e-4;
      let iters = 0;
      while(remaining > 0 && iters++ < 2000) {
        currentCircuit.step(stepSize);
        remaining -= stepSize;
      }
      
      boardView.renderFrame(dt);
      
      const s1 = setupData.probes[0] ? setupData.probes[0].read() : null;
      const s2 = setupData.probes[1] ? setupData.probes[1].read() : null;
      
      scopeView.draw(s1, s2, { 
        timeDiv: params.timeDiv || 0.001, 
        voltsDiv: params.voltsDiv || 1, 
        triggerLevel: params.triggerLevel || 0,
        sampleTime: stepSize
      });
      
      controls.updateReadouts(setupData.readouts());
      
      loopId = requestAnimationFrame(loop);
    };
    loopId = requestAnimationFrame(loop);
  }
  
  const initialState = readState();
  if (controls.setExperiment) controls.setExperiment(initialState.experiment, initialState.params);
  loadExperiment(initialState.experiment, initialState.params);
}

boot(main);