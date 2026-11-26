import { bindLevelSelect } from './components/level-select.js';

console.log("Game initialized");

bindLevelSelect((levelId) => {
    // Mock solved state for level 1
    if (levelId === '1') {
        document.getElementById('explanation').classList.add('visible');
        document.getElementById('explanation-text').innerText = "By connecting the battery to the bulb, charge can flow in a complete loop. You built a working circuit!";
        document.querySelector('#goal-list li').classList.add('done');
        document.querySelector(`[data-level="${levelId}"]`).classList.add('solved');
    } else {
        document.getElementById('explanation').classList.remove('visible');
        document.querySelector('#goal-list li').classList.remove('done');
    }
});
