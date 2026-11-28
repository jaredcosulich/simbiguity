export function readState() {
  const hash = window.location.hash.slice(1);
  if (!hash) return { experiment: "led", params: {} };
  const params = new URLSearchParams(hash);
  const exp = params.get("exp") || "led";
  params.delete("exp");
  const p = {};
  for (const [k, v] of params.entries()) {
    p[k] = parseFloat(v);
  }
  return { experiment: exp, params: p };
}

export function writeState(experiment, params) {
  const p = new URLSearchParams();
  p.set("exp", experiment);
  for (const [k, v] of Object.entries(params)) {
    p.set(k, v);
  }
  window.location.hash = p.toString();
}