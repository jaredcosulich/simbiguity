export const DEFAULT_STATE = {
  m: 1,
  c: 0.2,
  k: 100,
  f0: 1,
  omega: 10
};

export function parseState(hash) {
  if (!hash || hash.length <= 1) return { ...DEFAULT_STATE };
  const params = new URLSearchParams(hash.slice(1));
  const st = {};
  for (const k of Object.keys(DEFAULT_STATE)) {
    const val = params.get(k);
    if (val !== null) st[k] = parseFloat(val);
    else st[k] = DEFAULT_STATE[k];
  }
  return st;
}

export function formatState(state) {
  const params = new URLSearchParams();
  for (const [k, v] of Object.entries(state)) {
    params.set(k, v.toString());
  }
  return "#" + params.toString();
}
