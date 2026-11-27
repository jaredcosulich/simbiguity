import { parseState } from './state.js';

const presetSelect = document.getElementById('preset');
const integratorSelect = document.getElementById('integrator');

function syncUI() {
    const state = parseState(window.location.hash);
    presetSelect.value = state.preset;
    integratorSelect.value = state.integrator;
}

window.addEventListener('hashchange', syncUI);
syncUI();

// Basic wasm hookup would go here...
// The plan asks for: preset picker, sliders, integrator toggle, energy chart, drag-to-push
