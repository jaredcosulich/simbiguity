export class GameState {
    constructor(wasm) {
        this.w = wasm;
        this.levelHandle = 0;
    }

    loadLevel(id) {
        const encoder = new TextEncoder();
        const idBytes = encoder.encode(id + '\0');
        const ptr = this.w.sb_malloc(idBytes.length);
        new Uint8Array(this.w.memory.buffer, ptr, idBytes.length).set(idBytes);
        this.levelHandle = this.w.puzzle_load(ptr);
        this.w.sb_free(ptr, idBytes.length);
        return this.levelHandle;
    }

    undo() {
        if (this.levelHandle) this.w.puzzle_undo(this.levelHandle);
    }
    redo() {
        if (this.levelHandle) this.w.puzzle_redo(this.levelHandle);
    }
    reset() {
        if (this.levelHandle) this.w.puzzle_reset(this.levelHandle);
    }
    
    check() {
        if (!this.levelHandle) return;
        const count = this.w.puzzle_goal_count(this.levelHandle);
        let allPassed = true;
        for (let i = 0; i < count; i++) {
            if (this.w.puzzle_goal_status(this.levelHandle, i) === 0) {
                allPassed = false;
                break;
            }
        }
        
        if (allPassed && count > 0) {
            this.share();
        }
    }

    share() { 
        if (this.w.puzzle_share(this.levelHandle) === 0) {
            const ptr = this.w.sb_last_out_ptr();
            const len = this.w.sb_last_out_len();
            const code = new TextDecoder().decode(new Uint8Array(this.w.memory.buffer, ptr, len));
            
            document.getElementById('explanation').classList.add('visible');
            document.getElementById('explanation').style.borderColor = 'var(--success)';
            document.getElementById('explanation-text').innerHTML = `You connected the circuit! <br><br> Share link: <a href='#s=${code}'>#s=${code}</a>`;
        }
    }

    prototypeSimulate(state) {
        // Obsolete
    }
}
