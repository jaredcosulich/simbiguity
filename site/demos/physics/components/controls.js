export function PhysicsControls(state, onStateChange, onReset, onShare) {
    const container = document.createElement('div');
    container.id = 'controls';
    container.innerHTML = `
      <label>Preset:
        <select id="preset">
          <option value="pendulum">Pendulum</option>
          <option value="spring">Spring-mass</option>
          <option value="projectile">Projectile</option>
          <option value="chain">Chain</option>
          <option value="double_pendulum">Double pendulum</option>
          <option value="cradle">Newton's cradle</option>
          <option value="ball_pit">Ball pit</option>
          <option value="sliding_block">Sliding block</option>
          <option value="orbit">Orbit</option>
          <option value="binary">Binary star</option>
          <option value="figure_eight">Figure-eight three-body</option>
          <option value="cloth">Cloth</option>
        </select>
      </label>
      <label>Integrator:
        <select id="integrator">
          <option value="verlet">Velocity Verlet</option>
          <option value="symplectic">Symplectic Euler</option>
          <option value="explicit">Explicit Euler</option>
        </select>
      </label>
      <label>Time Scale:
        <input type="range" id="time-scale" min="0.1" max="4.0" step="0.1" value="1.0">
      </label>
      <label>Restitution:
        <input type="range" id="restitution" min="0.0" max="1.0" step="0.05" value="0.8">
      </label>
      <label>Friction:
        <input type="range" id="friction" min="0.0" max="1.0" step="0.05" value="0.5">
      </label>
      <button id="reset-btn">Reset</button>
      <button id="share-btn">Share</button>
    `;

    const presetSelect = container.querySelector('#preset');
    const integratorSelect = container.querySelector('#integrator');
    const timeScaleInput = container.querySelector('#time-scale');
    const restitutionInput = container.querySelector('#restitution');
    const frictionInput = container.querySelector('#friction');
    
    presetSelect.value = state.preset || 'pendulum';
    integratorSelect.value = state.integrator || 'verlet';
    timeScaleInput.value = state.timeScale || 1.0;
    restitutionInput.value = state.restitution || 0.8;
    frictionInput.value = state.friction || 0.5;

    const dispatchChange = () => {
        onStateChange({
            preset: presetSelect.value,
            integrator: integratorSelect.value,
            timeScale: parseFloat(timeScaleInput.value),
            restitution: parseFloat(restitutionInput.value),
            friction: parseFloat(frictionInput.value)
        });
    };

    presetSelect.onchange = dispatchChange;
    integratorSelect.onchange = dispatchChange;
    timeScaleInput.onchange = dispatchChange;
    restitutionInput.onchange = dispatchChange;
    frictionInput.onchange = dispatchChange;
    
    container.querySelector('#reset-btn').onclick = onReset;
    container.querySelector('#share-btn').onclick = onShare;

    return { 
      container, 
      sync: (newState) => {
          presetSelect.value = newState.preset;
          integratorSelect.value = newState.integrator;
          timeScaleInput.value = newState.timeScale;
          restitutionInput.value = newState.restitution;
          frictionInput.value = newState.friction;
      }
    };
}
