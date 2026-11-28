import { stringFromWasm, $ } from './utils.js';

export function bindLevelSelect(onSelect) {
    $('level-select').addEventListener('click', (e) => {
        if (e.target.tagName === 'LI') {
            document.querySelectorAll('#level-select li').forEach(li => li.classList.remove('active'));
            e.target.classList.add('active');
            onSelect(e.target.dataset.level);
        }
    });
}

export function updateLevelSelectUI(w, currentLevel, solved) {
    const levelCount = w.puzzle_level_count();
    const ul = $("level-select");
    ul.innerHTML = "";
    for (let i = 1; i <= levelCount; i++) {
        const li = document.createElement("li");
        li.dataset.level = i.toString();
        if (i.toString() === currentLevel) li.classList.add("active");
        if (solved.includes(i.toString())) li.classList.add("solved");
        
        if (w.puzzle_level_title(i - 1) === 0) {
           li.textContent = `${i}. ${stringFromWasm(w)}`;
        }
        ul.appendChild(li);
    }
    
    const h2 = document.querySelector(".panel h2");
    if (h2.textContent.startsWith("Levels")) {
        h2.textContent = `Levels (${solved.length} of ${levelCount} solved)`;
    }
}
