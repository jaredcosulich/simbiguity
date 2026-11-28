export function SimulationView() {
    const container = document.createElement('div');
    container.id = 'canvas-container';
    const canvas = document.createElement('canvas');
    canvas.id = 'view';
    canvas.width = 640;
    canvas.height = 400;
    container.appendChild(canvas);
    return { container, canvas };
}
