import { boot } from "../../js/boot.js";
import { loadCircuit3D, Pick } from "../../js/simbiguity-render.js";
import { WebGL2Executor } from "../../js/webgl2-executor.js";
import { bindLevelSelect, updateLevelSelectUI } from './components/level-select.js';
import { bindControls } from './components/controls.js';
import { GameState } from './game-state.js';
import { $, stringFromWasm } from './components/utils.js';
import { updateGoalsUI } from './components/goals.js';
import { updateExplanationUI, showExplanation } from './components/explanation-ui.js';
import { handleHash } from './components/url-state.js';
import { bindPartsTray } from './components/parts-tray.js';
import { bindBoardInteraction } from './components/board-interaction.js';

async function main() {
  const canvas = $("view");
  const gl = canvas.getContext("webgl2", { antialias: true, preserveDrawingBuffer: true });
  const [{ Circuit3D, exports: w }, exec] = await Promise.all([
    loadCircuit3D(new URL("../../pkg/simbiguity_render_wasm.wasm", import.meta.url)),
    WebGL2Executor.create(gl),
  ]);

  const state = new GameState(w);
  let view = null;
  let currentLevel = "1";
  let activeShareCode = null;
  let selectedKind = 4; // Wire default
  let currentRot = 0; // Default 0

  function loadLevel(id, shareCode) {
    if (view) view.free();
    currentLevel = id;
    const handle = state.loadLevel(id);
    view = new Circuit3D({ handle });
    
    if (shareCode) {
        const encoder = new TextEncoder();
        const codeEncoded = encoder.encode(shareCode + '\0');
        const codePtr = w.sb_malloc(codeEncoded.length);
        new Uint8Array(w.memory.buffer, codePtr, codeEncoded.length).set(codeEncoded);
        
        const res = w.puzzle_load_share(handle, codePtr);
        w.sb_free(codePtr, codeEncoded.length);
        
        if (res < 0) {
            const ptr = w.sb_last_error_ptr();
            const len = w.sb_last_error_len();
            const errMsg = new TextDecoder().decode(new Uint8Array(w.memory.buffer, ptr, len));
            showExplanation("Error", errMsg, true);
        } else {
            // Also evaluate check after loading share
            state.check();
        }
    }

    for (const m of view.meshes()) exec.uploadMesh(m.id, m.vertices, m.indices);
    
    // Set level title
    if (w.puzzle_level_title(parseInt(id) - 1) === 0) {
       $("level-desc").textContent = stringFromWasm(w);
    }

    updateUI();
    draw();
  }
  
  function updateUI() {
    const handle = state.levelHandle;
    if (!handle) return;

    const { allPassed, goalCount } = updateGoalsUI(handle, w);
    
    const solvedStr = localStorage.getItem('simbiguity.puzzles.solved') || "[]";
    const solved = JSON.parse(solvedStr);
    
    updateLevelSelectUI(w, currentLevel, solved);
    
    activeShareCode = updateExplanationUI(
        handle, 
        w, 
        allPassed, 
        goalCount, 
        currentLevel, 
        activeShareCode, 
        () => {
            if (!solved.includes(currentLevel)) {
                solved.push(currentLevel);
                localStorage.setItem('simbiguity.puzzles.solved', JSON.stringify(solved));
                setTimeout(updateUI, 0); 
            }
        }
    );
  }


  const loadLevelCallback = (lvl, s) => {
      if (!view || lvl !== currentLevel || s) {
          loadLevel(lvl, s);
      }
  };
  window.addEventListener("hashchange", () => handleHash(currentLevel, loadLevelCallback));

  bindPartsTray((kind) => {
      selectedKind = kind;
  });

  bindBoardInteraction(
      canvas, 
      state, 
      w, 
      () => selectedKind, 
      () => currentRot, 
      (rot) => { currentRot = rot; }, 
      exec, 
      () => view, 
      updateUI, 
      draw
  );

  function size() {
    const dpr = window.devicePixelRatio || 1;
    const w = Math.floor(canvas.clientWidth * dpr);
    const h = Math.floor(canvas.clientHeight * dpr);
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w;
      canvas.height = h;
    }
    return [w, h];
  }

  function draw() {
    if (!view) return;
    const [wid, hgt] = size();
    exec.execute(view.frame(wid, hgt));
  }

  bindLevelSelect((levelId) => {
    window.location.hash = `#level=${levelId}`;
  });

  bindControls(state, () => {
    if (view) {
        for (const m of view.meshes()) exec.uploadMesh(m.id, m.vertices, m.indices);
        updateUI();
        draw();
    }
  });

  handleHash(currentLevel, loadLevelCallback);

  let last = performance.now();
  const loop = (now) => {
    if (view) {
      view.advance(Math.min(0.25, (now - last) / 1000));
      draw();
    }
    last = now;
    requestAnimationFrame(loop);
  };
  requestAnimationFrame(loop);
}

boot(main);