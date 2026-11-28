const $ = (id) => document.getElementById(id);

export function bindControls(state, draw) {
    $("btn-undo").onclick = () => { state.undo(); draw(); };
    $("btn-redo").onclick = () => { state.redo(); draw(); };
    $("btn-reset").onclick = () => { state.reset(); draw(); };
    $("btn-share").onclick = () => { state.share(); };
}
