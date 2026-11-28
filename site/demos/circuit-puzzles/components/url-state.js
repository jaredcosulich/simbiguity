export function handleHash(currentLevel, loadLevelCallback) {
    const hash = window.location.hash.slice(1);
    const params = new URLSearchParams(hash);
    const lvl = params.get("level") || "1";
    const s = params.get("s");
    
    if (lvl !== currentLevel || s) {
        loadLevelCallback(lvl, s);
    } else {
        loadLevelCallback(lvl, null);
    }
}