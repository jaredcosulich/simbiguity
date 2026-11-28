export class GameState {
    constructor(wasm) {
        this.w = wasm;
        this.levelHandle = 0;
    }

    loadLevel(id) {
        const encoder = new TextEncoder();
        const idBytes = encoder.encode(id + '\0');
        this.levelHandle = this.w.puzzle_load(idBytes);
        return this.levelHandle;
    }

    undo() { console.log("Undo"); }
    redo() { console.log("Redo"); }
    reset() { console.log("Reset"); }
    
    check() {
        if (this.levelHandle && this.w.puzzle_check(this.levelHandle)) {
            this.share();
        }
    }

    share() { 
        document.getElementById('explanation').classList.add('visible');
        document.getElementById('explanation').style.borderColor = 'var(--success)';
        document.getElementById('explanation-text').innerHTML = "You connected the circuit! <br><br> Share link: <a href='#s=123'>#s=123</a>";
    }

    prototypeSimulate(state) {
        if (state === 'short-circuit') {
            document.getElementById('explanation').classList.add('visible');
            document.getElementById('explanation').style.borderColor = 'var(--warn)';
            document.getElementById('explanation-text').innerHTML = "<span class='issue'>Short Circuit!</span> The battery is connected directly to itself.";
        } else if (state === 'corrupted') {
            document.getElementById('explanation').classList.add('visible');
            document.getElementById('explanation').style.borderColor = 'var(--warn)';
            document.getElementById('explanation-text').innerHTML = "<span class='issue'>Error</span> This link is damaged.";
        }
    }
}
