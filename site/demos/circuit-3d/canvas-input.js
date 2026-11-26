// Pointer, wheel and keyboard input on the 3D canvas: drag to orbit, scroll
// to zoom, click (or Enter / Space) to flip the switch or replace a blown bulb.

import { canvasPixel, clickAction, cursorFor, CLICK_SLOP } from "./pointer.js";

/**
 * Wires the canvas to the view.
 * @param {HTMLCanvasElement} canvas
 * @param {object} view a `Circuit3D`
 * @param {{ onInput: () => void, onOrbit: () => void, onCamera: () => void, onKeyToggle: () => void }} hooks
 *   `onInput` after a click changed the circuit, `onOrbit` on every drag move,
 *   `onCamera` once the camera has settled (drag released, wheel turned),
 *   `onKeyToggle` for Enter / Space
 */
export function bindCanvasInput(canvas, view, { onInput, onOrbit, onCamera, onKeyToggle }) {
  const pickAt = (e) => {
    const [x, y] = canvasPixel(canvas.getBoundingClientRect(), canvas.width, canvas.height, e.clientX, e.clientY);
    return view.pick(x, y, canvas.width, canvas.height);
  };

  let drag = null;
  canvas.addEventListener("pointerdown", (e) => {
    drag = { x: e.clientX, y: e.clientY, moved: 0 };
    canvas.setPointerCapture(e.pointerId);
  });
  canvas.addEventListener("pointermove", (e) => {
    if (!drag) {
      canvas.style.cursor = cursorFor(pickAt(e), view.readouts().blown);
      return;
    }
    const dx = e.clientX - drag.x;
    const dy = e.clientY - drag.y;
    drag.moved += Math.abs(dx) + Math.abs(dy);
    drag.x = e.clientX;
    drag.y = e.clientY;
    view.orbit(-dx * 0.4, dy * 0.3);
    onOrbit();
  });
  canvas.addEventListener("pointerup", (e) => {
    const wasClick = drag && drag.moved < CLICK_SLOP;
    drag = null;
    const action = wasClick ? clickAction(pickAt(e), view.readouts().blown) : null;
    if (action === "toggle") view.toggle();
    else if (action === "replace") view.replaceBulb();
    if (action) onInput();
    else onCamera();
  });
  canvas.addEventListener("wheel", (e) => {
    e.preventDefault();
    view.zoom(Math.pow(1.0015, e.deltaY));
    onCamera();
  }, { passive: false });
  canvas.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onKeyToggle();
    }
  });
}
