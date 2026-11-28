import { boot } from "../../js/boot.js";
import { BoardView } from "./components/board-view.js";
import { ExperimentControls } from "./components/experiment-controls.js";
import { ScopeView } from "./components/scope-view.js";
import { EXPERIMENTS } from "./experiments.js";

async function main() {
  const boardView = new BoardView("board-canvas");
  await boardView.init();
  
  const scopeView = new ScopeView(document.getElementById("scope-canvas"));
  
  let loopId = null;
  
  const controls = new ExperimentControls(
    "experiment-select",
    "error-text",
    "readouts-container",
    (expName, params) => {
      loadExperiment(expName, params);
    }
  );
  
  function loadExperiment(name, params) {
    if (loopId) cancelAnimationFrame(loopId);
    
    const exp = EXPERIMENTS[name];
    if (!exp) return;
    
    try {
      boardView.loadLayout(exp.setup(params));
      controls.clearError();
    } catch (e) {
      controls.showError(e.message);
      return;
    }
    
    let last = performance.now();
    const loop = (now) => {
      const dt = Math.min(0.25, (now - last) / 1000);
      last = now;
      
      const readouts = boardView.renderFrame(dt);
      
      scopeView.draw([0, Math.sin(now / 200), 1, Math.sin((now+100) / 200)]);
      
      loopId = requestAnimationFrame(loop);
    };
    loopId = requestAnimationFrame(loop);
  }
  
  const initialState = controls.getInitialState();
  loadExperiment(initialState.experiment, initialState.params);
}

boot(main);