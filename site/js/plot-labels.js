export function renderLabels(container, labels) {
    container.innerHTML = "";
    for (const [text, x, y] of labels) {
        const span = document.createElement("span");
        span.textContent = text;
        span.style.position = "absolute";
        span.style.left = `${x}px`;
        span.style.top = `${y}px`;
        container.appendChild(span);
    }
}
