export function parseState(hash) {
    const params = new URLSearchParams(hash.replace('#', '?'));
    return {
        preset: params.get('preset') || 'pendulum',
        integrator: params.get('integrator') || 'verlet',
        timeScale: parseFloat(params.get('timeScale')) || 1.0,
        restitution: parseFloat(params.get('restitution')) || 0.8,
        friction: parseFloat(params.get('friction')) || 0.5,
    };
}

export function formatState(state) {
    return `preset=${state.preset}&integrator=${state.integrator}&timeScale=${state.timeScale}&restitution=${state.restitution}&friction=${state.friction}`;
}
