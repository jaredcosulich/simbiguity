export function parseState(hash) {
    const params = new URLSearchParams(hash.replace('#', '?'));
    return {
        preset: params.get('preset') || 'pendulum',
        integrator: params.get('integrator') || 'verlet',
    };
}
