// Canvas geometry and what a pointer over the 3D view means. Pure: no DOM.

import { Pick } from "../../js/simbiguity-render.js";

/** Pointer travel, in CSS pixels, below which a press-and-release is a click. */
export const CLICK_SLOP = 5;

/**
 * The canvas's backing-store size for its CSS size at a device pixel ratio
 * (at least 1 × 1).
 * @returns {[number, number]}
 */
export function canvasSize(clientWidth, clientHeight, dpr = 1) {
  const r = dpr || 1;
  return [Math.max(1, Math.round(clientWidth * r)), Math.max(1, Math.round(clientHeight * r))];
}

/**
 * A pointer position in canvas pixels, from its client position, the
 * canvas's on-screen rectangle and its backing-store size.
 * @param {{ left: number, top: number, width: number, height: number }} rect
 * @returns {[number, number]}
 */
export function canvasPixel(rect, canvasWidth, canvasHeight, clientX, clientY) {
  return [
    (clientX - rect.left) * (canvasWidth / rect.width),
    (clientY - rect.top) * (canvasHeight / rect.height),
  ];
}

/**
 * What clicking `hit` does: the switch flips, a blown bulb is replaced,
 * anything else does nothing.
 * @returns {"toggle" | "replace" | null}
 */
export function clickAction(hit, blown) {
  if (hit === Pick.SWITCH) return "toggle";
  if (hit === Pick.BULB && blown) return "replace";
  return null;
}

/** The cursor over `hit`: a pointer where a click does something, else the orbit grab. */
export function cursorFor(hit, blown) {
  return clickAction(hit, blown) ? "pointer" : "grab";
}
