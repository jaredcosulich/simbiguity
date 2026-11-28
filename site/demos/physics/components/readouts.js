export function PhysicsReadouts() {
    const container = document.createElement('div');
    container.id = 'readouts';
    container.style.display = 'flex';
    container.style.gap = '20px';
    container.style.fontFamily = 'monospace';
    container.style.background = '#ddd';
    container.style.padding = '10px';
    container.style.marginTop = '10px';

    container.innerHTML = `
         <div>Sim Time: <span id="sim-time">0</span>s</div>
         <div>Step Time: <span id="step-time">0</span>ms</div>
         <div>Momentum: <span id="momentum">0</span></div>
         <div>FPS: <span id="fps">0</span></div>
    `;

    const simTimeEl = container.querySelector('#sim-time');
    const stepTimeEl = container.querySelector('#step-time');
    const momentumEl = container.querySelector('#momentum');
    const fpsEl = container.querySelector('#fps');

    return {
        container,
        update: (simTime, stepTime, momentum) => {
            simTimeEl.textContent = simTime.toFixed(2);
            stepTimeEl.textContent = stepTime.toFixed(2);
            momentumEl.textContent = momentum.toFixed(2);
        },
        updateFps: (fps) => {
            fpsEl.textContent = fps;
        },
        updateEnergy: (e) => {
            let energyDisplay = container.querySelector('#energy');
            if (!energyDisplay) {
                energyDisplay = document.createElement('div');
                energyDisplay.id = 'energy';
                container.appendChild(energyDisplay);
            }
            energyDisplay.innerText = `KE: ${e.kinetic.toFixed(2)}, PE(g): ${e.potential_gravity.toFixed(2)}, PE(s): ${e.potential_spring.toFixed(2)}, Total: ${e.total.toFixed(2)}`;
        },
        updateChaosAngle: (logDiff) => {
            let diffDisplay = container.querySelector('#angle-diff');
            if (logDiff === null) {
                if (diffDisplay) diffDisplay.remove();
                return;
            }
            if (!diffDisplay) {
                diffDisplay = document.createElement('div');
                diffDisplay.id = 'angle-diff';
                container.appendChild(diffDisplay);
            }
            diffDisplay.textContent = `Chaos Angle Diff (log10 rad): ${logDiff}`;
        }
    };
}
