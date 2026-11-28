import { stringFromWasm } from './utils.js';

export function updateGoalsUI(handle, w) {
    const goalCount = w.puzzle_goal_count(handle);
    const goalList = document.getElementById("goal-list");
    goalList.innerHTML = "";
    let allPassed = true;
    for (let i = 0; i < goalCount; i++) {
        const li = document.createElement("li");
        w.puzzle_goal_text(handle, i);
        li.textContent = stringFromWasm(w);
        const status = w.puzzle_goal_status(handle, i);
        if (status === 1) {
            li.classList.add("done");
        } else {
            allPassed = false;
        }
        goalList.appendChild(li);
    }
    return { allPassed, goalCount };
}