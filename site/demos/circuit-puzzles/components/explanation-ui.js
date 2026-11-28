import { stringFromWasm, $ } from './utils.js';

export function showExplanation(titleText, msg, isWarn) {
    const panel = $("explanation");
    panel.classList.add("visible");
    panel.style.borderColor = isWarn ? "var(--warn)" : "var(--success)";
    const title = panel.querySelector("h3");
    title.textContent = titleText;
    title.style.color = isWarn ? "var(--warn)" : "var(--success)";
    
    const text = $("explanation-text");
    text.innerHTML = "";
    const span = document.createElement("span");
    span.className = "issue";
    span.textContent = msg;
    text.appendChild(span);
}

export function updateExplanationUI(handle, w, allPassed, goalCount, currentLevel, activeShareCode, onSolved) {
    if (allPassed && goalCount > 0) {
        onSolved();
        
        w.puzzle_share(handle);
        const code = stringFromWasm(w);
        const newShareCode = code;
        window.history.replaceState(null, "", `#level=${currentLevel}&s=${code}`);
        
        const panel = $("explanation");
        panel.classList.add("visible");
        panel.style.borderColor = "var(--success)";
        
        const title = panel.querySelector("h3");
        title.textContent = "Level Solved!";
        title.style.color = "var(--success)";
        
        const text = $("explanation-text");
        text.innerHTML = "";
        text.textContent = "You connected the circuit! \nShare link: ";
        const a = document.createElement("a");
        a.href = `#level=${currentLevel}&s=${code}`;
        a.textContent = `#s=${code}`;
        text.appendChild(a);
        
        return newShareCode;
    } else {
        const issueRes = w.puzzle_issue_text(handle);
        const panel = $("explanation");
        if (issueRes === 1) {
            const msg = stringFromWasm(w);
            showExplanation("Issue", msg, true);
            return activeShareCode;
        } else {
            panel.classList.remove("visible");
            if (activeShareCode) {
                window.history.replaceState(null, "", `#level=${currentLevel}`);
                return null;
            }
            return activeShareCode;
        }
    }
}