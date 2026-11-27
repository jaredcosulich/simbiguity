import { bindLevelSelect } from './components/level-select.js';

console.log("Game initialized");

bindLevelSelect((levelId) => {
    // Rust checking happens here. For prototype, we will just advance to the next step.
    console.log("Selected level: ", levelId);
});
