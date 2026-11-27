export function stateFromHash(hash) {
    const tokens = new Set(hash.replace(/^#/, "").split("&").filter(Boolean));
    return {
        preset: tokens.has("lc") ? "lc" : "rc",
        integrator: tokens.has("trap") ? 1 : 0,
        closed: true
    };
}

export function hashFromState({ preset, integrator }) {
    const parts = [];
    if (preset === "lc") parts.push("lc");
    if (integrator === 1) parts.push("trap");
    return parts.length ? `#${parts.join("&")}` : "";
}