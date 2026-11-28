import { parseState, formatState } from './state.js';
import { WebGL2Executor } from '../../js/webgl2-executor.js';
import { loadPhysics } from './physics.js';
import { PhysicsHeader } from './components/header.js';
import { SimulationView } from './components/simulation-view.js';
import { PhysicsControls } from './components/controls.js';
import { PhysicsReadouts } from './components/readouts.js';
import { EnergyChart } from './components/energy-chart.js';
import { startPhysicsLoop } from './loop.js';

export async function setupUI() {
    document.body.appendChild(PhysicsHeader());
    const mainEl = document.createElement('main');
    document.body.appendChild(mainEl);

    const { container: canvasContainer, canvas } = SimulationView();
    mainEl.appendChild(canvasContainer);

    let state = parseState(window.location.hash);
    const { container: controlsContainer, sync: syncControls } = PhysicsControls(
        state,
        (newState) => {
            window.location.hash = formatState(newState);
        },
        () => {
            syncUI(); 
        },
        () => {
            navigator.clipboard.writeText(window.location.href);
            alert('URL copied to clipboard');
        }
    );
    mainEl.appendChild(controlsContainer);

    const readouts = PhysicsReadouts();
    controlsContainer.appendChild(readouts.container);

    mainEl.appendChild(EnergyChart());

    const { Physics } = await loadPhysics(new URL("../../pkg/simbiguity_physics_wasm.wasm", import.meta.url));
    const executor = await WebGL2Executor.create(canvas.getContext('webgl2'));

    let physics = null;

    function syncUI() {
        state = parseState(window.location.hash);
        syncControls(state);
        
        if (physics) {
            physics.free();
        }
        physics = new Physics(state.preset);
        physics.setIntegrator(state.integrator);
        physics.setTimeScale(state.timeScale);
        physics.setRestitution(state.restitution);
        physics.setFriction(state.friction);
    }

    window.addEventListener('hashchange', syncUI);
    syncUI();

    startPhysicsLoop(
        () => physics,
        executor,
        canvas,
        () => state,
        readouts
    );
}
