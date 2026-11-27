export function renderLabels(container, labels) {
    let overlay = container.querySelector ? container.querySelector(".plot-labels-overlay") : container.children.find(c => c.className === "plot-labels-overlay");
    if (!overlay) {
        overlay = document.createElement("div");
        overlay.className = "plot-labels-overlay";
        overlay.style.position = "absolute";
        overlay.style.top = "0";
        overlay.style.left = "0";
        overlay.style.width = "100%";
        overlay.style.height = "100%";
        overlay.style.pointerEvents = "none";
        container.appendChild(overlay);
    }
    overlay.innerHTML = "";
    for (const [text, x, y] of labels) {
        const span = document.createElement("span");
        span.textContent = text;
        span.style.position = "absolute";
        span.style.left = `${x}px`;
        span.style.top = `${y}px`;
        overlay.appendChild(span);
    }
}
