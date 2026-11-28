export function bindBoardInteraction(canvas, state, w, getSelectedKind, getRot, setRot, exec, getView, updateUICallback, drawCallback) {
    canvas.addEventListener("contextmenu", (e) => {
        e.preventDefault();
        setRot((getRot() + 90) % 360);
    });

    canvas.addEventListener("pointerdown", (e) => {
        if (e.button !== 0) return;
        const rect = canvas.getBoundingClientRect();
        const px = (e.clientX - rect.left) / rect.width;
        const py = (e.clientY - rect.top) / rect.height;
        const tag = w.c3d_pick(state.levelHandle, px, py, rect.width, rect.height);
        
        const ptrX = w.sb_malloc(4);
        const ptrZ = w.sb_malloc(4);
        
        if (tag > 0) {
            // Part was clicked, remove it
            if (w.puzzle_part_cell(state.levelHandle, tag, ptrX, ptrZ) === 1) {
                const viewX = new Int32Array(w.memory.buffer, ptrX, 1)[0];
                const viewZ = new Int32Array(w.memory.buffer, ptrZ, 1)[0];
                w.puzzle_remove(state.levelHandle, viewX, viewZ);
                state.check();
                updateUICallback();
            }
        } else {
            // Empty space clicked, place part
            if (w.puzzle_pick_cell(state.levelHandle, px, py, rect.width, rect.height, ptrX, ptrZ) === 1) {
                const viewX = new Int32Array(w.memory.buffer, ptrX, 1)[0];
                const viewZ = new Int32Array(w.memory.buffer, ptrZ, 1)[0];
                w.puzzle_place(state.levelHandle, getSelectedKind(), viewX, viewZ, getRot());
                state.check();
                updateUICallback();
            }
        }
        
        w.sb_free(ptrX, 4);
        w.sb_free(ptrZ, 4);
        
        const view = getView();
        if (view) {
            for (const m of view.meshes()) exec.uploadMesh(m.id, m.vertices, m.indices);
            drawCallback();
        }
    });
}