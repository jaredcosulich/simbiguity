// codeyam-generated — DO NOT EDIT.
// codeyam-editor: 0.1.7  protocol: 1  build: 24a38f53b90c16cc5a19c1b851a4367b43c819a4  source-sha256: 2fdb25f7374f7269cc22f59ad0b728f8e395034b250303bd5918051d1da4b6f4
// npm/scenario-content-probe.js — read-only measurements of what a frame is
// showing: its content state, the length of its painted text, and whether its
// UI is painted into a canvas. Split out of scenario-playwright.js, which owns
// page loading, settling and interactions and re-exports these names.
const { isCanvasDominated, canvasCoversViewport } = require("./scenario-metrics");

async function collectContentState(target) {
  return target.evaluate(() => {
    const root = document.getElementById("root");
    const imgs = Array.from(document.images || []);
    const loadedImageCount = imgs.filter(
      (img) => img.complete && img.naturalWidth > 0,
    ).length;
    // Media is counted inside open shadow roots too. `querySelectorAll` stops
    // at a shadow boundary, and an app that paints inside a web component —
    // Flutter web draws its canvas under <flutter-view>'s shadow root — would
    // otherwise read as zero media and zero text, failing the blank gate on a
    // fully rendered frame.
    const scopes = [document];
    for (let i = 0; i < scopes.length; i++) {
      for (const el of scopes[i].querySelectorAll("*")) {
        if (el.shadowRoot) scopes.push(el.shadowRoot);
      }
    }
    const mediaSelectors = ["svg", "canvas", "video"];
    let mediaBboxCount = 0;
    for (const selector of mediaSelectors) {
      const nodes = scopes.flatMap((scope) =>
        Array.from(scope.querySelectorAll(selector)),
      );
      for (const node of nodes) {
        const rect = node.getBoundingClientRect();
        if (rect.width > 0 && rect.height > 0) {
          mediaBboxCount += 1;
        }
      }
    }
    return {
      bodyTextLength: document.body ? document.body.innerText.trim().length : 0,
      rootChildCount: root ? root.childElementCount : 0,
      rootTextLength: root ? (root.textContent || "").trim().length : 0,
      imageCount: imgs.length,
      loadedImageCount,
      mediaBboxCount,
    };
  });
}

// Sum the length of text that is actually PAINTED to the frame — text whose
// element ancestor chain is not hidden by `display:none` / `visibility:hidden`
// and is not collapsed to `opacity:0`, and which occupies a non-zero box. This
// is the signal the blank-frame gate uses to catch a reveal-suppressed capture:
// a frame whose DOM rendered text (so `bodyTextLength > 0`) but whose every
// section is still sitting at the `opacity:0` start of an entrance animation
// that never fired, leaving the screenshot blank. Counts text mid-transition
// (any opacity above exactly 0) as visible, so a reveal that has begun is not
// flagged. Returns `undefined` when the DOM-walking / computed-style APIs are
// unavailable (e.g. a stubbed test target) so callers fall back to the legacy
// DOM-presence behavior rather than treating an un-measurable frame as blank.
async function collectVisibleTextLength(target) {
  return target.evaluate(() => {
    if (
      typeof document.createTreeWalker !== "function" ||
      typeof window.getComputedStyle !== "function" ||
      typeof NodeFilter === "undefined"
    ) {
      return undefined;
    }
    const root = document.body || document.documentElement;
    if (!root) return 0;
    const isHidden = (el) => {
      for (
        let node = el;
        node && node.nodeType === 1;
        node = node.parentElement
      ) {
        const style = window.getComputedStyle(node);
        if (!style) continue;
        if (style.display === "none" || style.visibility === "hidden")
          return true;
        if (parseFloat(style.opacity) === 0) return true;
      }
      return false;
    };
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    let total = 0;
    let node;
    while ((node = walker.nextNode())) {
      const text = (node.nodeValue || "").trim();
      if (!text) continue;
      const el = node.parentElement;
      if (!el || isHidden(el)) continue;
      const rect = el.getBoundingClientRect();
      if (rect.width <= 0 || rect.height <= 0) continue;
      total += text.length;
    }
    return total;
  });
}

// Is the frame canvas-dominated right now — visible canvas/svg/video and no
// visible text? Reads the same content state the blank gate classifies on.
// Best-effort: a frame that cannot be measured reads as NOT canvas, so the
// caller keeps its ordinary DOM behavior instead of refusing on a guess.
async function probeCanvasDominated(frame) {
  try {
    const state = await collectContentState(frame);
    const visible = await collectVisibleTextLength(frame);
    if (typeof visible === "number") state.visibleTextLength = visible;
    return isCanvasDominated(state);
  } catch (_) {
    return false;
  }
}

// The largest visible canvas's area beside the viewport's, in CSS px², for
// `canvasCoversViewport`. `null` when the frame cannot be measured.
async function collectCanvasGeometry(frame) {
  try {
    return await frame.evaluate(() => {
      let largestCanvasArea = 0;
      const canvases = document.querySelectorAll("canvas");
      for (let i = 0; i < canvases.length; i++) {
        const r = canvases[i].getBoundingClientRect();
        const cs = getComputedStyle(canvases[i]);
        if (cs.visibility === "hidden" || cs.display === "none") continue;
        const w = Math.max(0, Math.min(r.right, window.innerWidth) - Math.max(r.left, 0));
        const h = Math.max(0, Math.min(r.bottom, window.innerHeight) - Math.max(r.top, 0));
        largestCanvasArea = Math.max(largestCanvasArea, w * h);
      }
      return { largestCanvasArea, viewportArea: window.innerWidth * window.innerHeight };
    });
  } catch (_) {
    return null;
  }
}

// The canvas probe for a TEXT MISS, deliberately wider than
// `probeCanvasDominated`: a painted UI often keeps a little DOM text (a title
// bar, a hidden IME input's label, a version badge), which makes the page not
// canvas-dominated and sent every painted-UI miss to the "check your spelling"
// branch. A miss is already a miss, so naming the canvas when one covers most
// of the viewport cannot turn a working step into a failing one. The text
// WAIT keeps the strict probe, because it refuses up front.
async function probeCanvasTextMiss(frame) {
  if (await probeCanvasDominated(frame)) return true;
  return canvasCoversViewport(await collectCanvasGeometry(frame));
}

module.exports = {
  collectContentState,
  collectVisibleTextLength,
  probeCanvasDominated,
  collectCanvasGeometry,
  probeCanvasTextMiss,
};

// Pixel-path script: collectContentState feeds the blank gate and settling, so
// its code and this number both feed the capture pipeline id.
const CAPTURE_VISUAL_CONTRACT = 1;
module.exports.CAPTURE_VISUAL_CONTRACT = CAPTURE_VISUAL_CONTRACT;
