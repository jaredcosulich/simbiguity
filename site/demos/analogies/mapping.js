export const PARAM_LIMITS = {
  m: { min: 0.1, max: 10, step: 0.1 },
  c: { min: 0, max: 10, step: 0.1 },
  k: { min: 1, max: 500, step: 1 },
  f0: { min: 0, max: 10, step: 0.1 },
  omega: { min: 0.1, max: 50, step: 0.1 }
};

export function sliderToState(key, val) {
  return parseFloat(val);
}

export function stateToSlider(key, stateVal) {
  return stateVal.toString();
}

export function getDerived(state) {
  const w0 = Math.sqrt(state.k / state.m);
  const q = state.c === 0 ? Infinity : Math.sqrt(state.m * state.k) / state.c;
  return { w0, q };
}

export function getPhysicsParams(state) {
  return {
    m: state.m,
    k: state.k,
    c: state.c,
    f0: state.f0,
    omega: state.omega
  };
}

export function getCircuitParams(state) {
  return {
    l: state.m,
    r: state.c,
    c: 1.0 / state.k,
    v0: state.f0,
    omega: state.omega
  };
}
