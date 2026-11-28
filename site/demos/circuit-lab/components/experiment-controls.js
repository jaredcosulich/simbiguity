import { readState, writeState } from "../state.js";

export class ExperimentControls {
  constructor(selectId, errorId, readoutsId, onExperimentChange) {
    this.select = document.getElementById(selectId);
    this.errorText = document.getElementById(errorId);
    this.readouts = document.getElementById(readoutsId);
    this.onExperimentChange = onExperimentChange;
    
    this.select.addEventListener("change", (e) => {
      const exp = e.target.value;
      writeState(exp, {});
      this.onExperimentChange(exp, {});
    });
    
    const state = readState();
    this.select.value = state.experiment;
  }
  
  showError(msg) {
    this.errorText.innerText = msg;
  }
  
  clearError() {
    this.errorText.innerText = "";
  }
  
  updateReadouts(readoutsText) {
    this.readouts.innerText = readoutsText || "";
  }
  
  getInitialState() {
    return readState();
  }
}