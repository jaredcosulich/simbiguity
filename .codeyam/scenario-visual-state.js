// codeyam-generated — DO NOT EDIT.
// codeyam-editor: 0.1.7  protocol: 1  build: 24a38f53b90c16cc5a19c1b851a4367b43c819a4  source-sha256: bf9558653497e68ffd0cc9c3239297f4728f182ffae531ac03a80e48dc7a721b
// npm/scenario-visual-state.js — the pre-shot passes that put a frame into its
// final visual state: tripping scroll-gated reveals, snapping entrance
// animations to their end, and centering an isolated component's wrapper.
// Split out of scenario-playwright.js, which owns page loading, settling and
// interactions and re-exports these names.

// Trip scroll-gated entrance animations before capture. Many web/visual
// stacks reveal sections with an IntersectionObserver that flips an
// `opacity:0` element to its final state once it scrolls into view. Captured
// in isolation — where the app shell that wires those observers is absent —
// or on a tall page captured at a fixed viewport, the observers never fire and
// the content stays invisible, so the screenshot is blank. Scroll the document
// end-to-end in steps to trip every observer, then return to the top so the
// captured frame starts where the app does. Stack-agnostic: it drives the same
// scroll a real user would, touching no framework API. A frame whose document
// fits in (or is shorter than) the viewport still gets a nudge-and-restore so a
// single-screen page's scroll-triggered reveal fires too. Returns the measured
// document height (0 when there is nothing to scroll).
async function scrollThroughDocument(target) {
  return target.evaluate(() => {
    const doc = document.scrollingElement || document.documentElement;
    const total = Math.max(
      (doc && doc.scrollHeight) || 0,
      document.body ? document.body.scrollHeight : 0,
    );
    if (typeof window.scrollTo !== "function") return total;
    const viewport = window.innerHeight || 0;
    if (total <= viewport || total === 0) {
      // Even a single-viewport page may gate a reveal on the first scroll
      // event; a nudge-and-restore fires those observers without moving the
      // captured frame off the top.
      window.scrollTo(0, 1);
      window.scrollTo(0, 0);
      return total;
    }
    const stride = Math.max(1, Math.floor((total - viewport) / 8));
    for (let y = 0; y <= total; y += stride) {
      window.scrollTo(0, y);
    }
    window.scrollTo(0, 0);
    return total;
  });
}

// Inject a capture-only stylesheet that snaps entrance animations to their
// FINAL frame: remove animation/transition timing, then reveal the elements an
// entrance animation left INVISIBLE. This is the belt to
// `scrollThroughDocument`'s suspenders — it covers pure-CSS keyframe entrances
// that are mid-flight or stuck at an `opacity:0` start state even after the
// observers fired. It targets the generic CSS symptom, not any framework's
// reveal class, so it works for any stack. The caller gates this OFF when a
// scenario declares an interactive state, so an intentionally animated /
// collapsed interactive frame is never clobbered. Idempotent (a single injected
// style id) and best-effort. Returns true when the style is present after the
// call.
//
// The reveal is conditional on ANIMATION EVIDENCE, not on transparency alone:
// an element is revealed only when, before the reset lands, an animation or a
// transition was actually holding it back. An element resting at `opacity: 0`
// with neither is hidden BY INTENT — a closed dropdown, a hover-only action, an
// empty toast slot, an off-screen drawer — and is left exactly as the app
// rendered it. Elements carrying an explicit hidden-intent signal
// (`aria-hidden="true"`, `inert`, or `pointer-events: none`), on themselves or
// on an ancestor, are skipped even when a transition is declared — that is the
// resting shape of a closed popover which animates on open.
//
// It is never a blanket `opacity: 1 !important` / `transform: none !important`
// over `*`. A blanket rule cannot tell an entrance animation's `opacity: 0` from
// DELIBERATE, resting state — a disabled control's dim, a muted row, a
// collapsed chevron's rotation — so it silently flattened every one of them out
// of every screenshot the capture pipeline produced. Two scenarios differing
// only in such a state then captured byte-identically and collided in the
// distinct-capture gate, and the collision was unfixable in the component: the
// state rendered correctly in a real browser (verified: computed opacity 0.4 vs
// 1) and was erased only at capture time. Anything already visible is now left
// exactly as the app rendered it.
//
// The reveal's TRANSFORM half additionally stops at the SVG boundary, because a
// CSS `transform` overrides the SVG `transform` presentation attribute: inside
// an `<svg>` it erases the static geometry that CONSTRUCTS the drawing instead
// of neutralizing an entrance animation, so a revealed SVG node lands on the
// origin. Invisible SVG nodes are therefore revealed in place — opacity forced,
// transform left alone. See the loop below for why the boundary is
// `ownerSVGElement` rather than `closest("svg")`.
async function forceFinalVisualState(target) {
  return target.evaluate(() => {
    const STYLE_ID = "__codeyam_force_final_state";
    if (
      typeof document.getElementById === "function" &&
      document.getElementById(STYLE_ID)
    ) {
      return true;
    }
    if (typeof document.createElement !== "function") return false;
    const style = document.createElement("style");
    style.id = STYLE_ID;
    // Temporal properties only — these carry no resting state, so removing
    // them cannot erase anything the app meant to show.
    style.textContent =
      "*, *::before, *::after {" +
      "animation: none !important;" +
      "transition: none !important;" +
      "}";
    const head = document.head || document.documentElement;
    if (!head || typeof head.appendChild !== "function") return false;

    const styleOf = (el) => {
      try {
        return getComputedStyle(el);
      } catch (_) {
        return null;
      }
    };

    // Which elements is an animation or a transition ACTUALLY holding back?
    // Snapshot that BEFORE the stylesheet lands, because injecting it destroys
    // the evidence: `animation: none` / `transition: none` clears
    // `animationName`, empties the running-animation list, and zeroes every
    // duration. Only elements in this set are eligible for the reveal below.
    const isAnimationHeld = (el) => {
      let animations = null;
      if (typeof el.getAnimations === "function") {
        try {
          animations = el.getAnimations();
        } catch (_) {
          animations = null;
        }
      }
      if (animations && animations.length > 0) return true;
      // No `getAnimations` (older engines) falls through to the computed-style
      // evidence below — never to "reveal every invisible element".
      const computed = styleOf(el);
      if (!computed) return false;
      const animationName = computed.animationName;
      if (
        typeof animationName === "string" &&
        animationName !== "" &&
        animationName !== "none"
      ) {
        return true;
      }
      const properties = computed.transitionProperty;
      if (
        typeof properties !== "string" ||
        properties === "" ||
        properties === "none"
      ) {
        return false;
      }
      const coversVisibility = properties
        .split(",")
        .map((property) => property.trim())
        .some(
          (property) =>
            property === "opacity" ||
            property === "transform" ||
            property === "all",
        );
      if (!coversVisibility) return false;
      const durations = computed.transitionDuration;
      if (typeof durations !== "string") return false;
      // A declared transition with a zero duration animates nothing, so it is
      // not holding anything back.
      return durations.split(",").some((duration) => parseFloat(duration) > 0);
    };

    // Explicit "this is hidden on purpose" signals, which outrank animation
    // evidence. A closed popover carries its open-transition while resting
    // closed, so a transition alone cannot distinguish it from an entrance the
    // capture should finish. An ancestor counts: hiding a whole subtree is the
    // common spelling.
    const hasHiddenIntent = (el, computed) => {
      if (computed && computed.pointerEvents === "none") return true;
      if (typeof el.closest === "function") {
        try {
          if (el.closest('[aria-hidden="true"], [inert]')) return true;
        } catch (_) {
          // Fall through to the self-only checks below.
        }
      }
      if (el.inert === true) return true;
      return (
        typeof el.getAttribute === "function" &&
        el.getAttribute("aria-hidden") === "true"
      );
    };

    const heldByAnimation = new Set();
    if (typeof document.querySelectorAll === "function") {
      for (const el of document.querySelectorAll("*")) {
        if (isAnimationHeld(el)) heldByAnimation.add(el);
      }
    }

    head.appendChild(style);

    // With animations disabled above, an element held back by an entrance
    // animation now computes to its pre-animation resting state — typically
    // `opacity: 0`, often paired with a translate/scale that parks it offscreen.
    // Reveal exactly those, and only those: membership in the pre-reset
    // `heldByAnimation` snapshot is what separates them from UI the app is
    // deliberately holding at `opacity: 0`, which stays hidden. An element at
    // any visible opacity is left untouched either way.
    //
    // The transform half of the reveal STOPS at the SVG boundary. A CSS
    // `transform` overrides the SVG `transform` presentation attribute, so
    // inside an `<svg>` a forced `transform: none` does not neutralize an
    // entrance animation — it erases the static rotate/translate/scale that
    // CONSTRUCTS the drawing, revealing the node collapsed on the origin. The
    // opacity half is still right there (a node at ~0 shows nothing either
    // way), so an invisible SVG node is revealed IN PLACE. The boundary test is
    // `ownerSVGElement` and NOT `closest("svg")`: HTML inside a
    // `<foreignObject>` is a real HTMLElement with no `ownerSVGElement`, so it
    // keeps the full reveal — `closest("svg")` would silently strand its
    // genuine CSS entrance transform.
    if (typeof document.querySelectorAll !== "function") return true;
    const INVISIBLE_EPSILON = 0.01;
    for (const el of document.querySelectorAll("*")) {
      const computed = styleOf(el);
      if (!computed) continue;
      const opacity = parseFloat(computed.opacity);
      if (!Number.isFinite(opacity) || opacity > INVISIBLE_EPSILON) continue;
      if (!heldByAnimation.has(el)) continue;
      if (hasHiddenIntent(el, computed)) continue;
      if (!el.style || typeof el.style.setProperty !== "function") continue;
      const inSvg =
        el.ownerSVGElement != null ||
        (typeof el.tagName === "string" && el.tagName.toLowerCase() === "svg");
      el.style.setProperty("opacity", "1", "important");
      if (!inSvg) el.style.setProperty("transform", "none", "important");
    }
    return true;
  });
}

// Center the isolated-component wrapper in the viewport before the shot.
//
// The capture pipeline always shoots the full viewport (`page.screenshot({
// fullPage: false })`) and never an element clip, so an isolation page that
// does not center its own component strands it in the top-left corner of every
// frame. Next.js does this in its isolation layout and the query-param stacks
// do it in `.codeyam/harness/isolate.tsx`, but the server-rendered scaffolds
// historically did not — and neither do pages hand-authored before that was
// fixed. This pass closes the gap at capture time, for any stack.
//
// It MEASURES rather than assumes: a wrapper that is already centered is left
// alone, so pages that already do the right thing are byte-identical. The
// `#codeyam-capture` marker (the cross-stack isolation convention; expo emits
// it via `nativeID`) is the gate, so an ordinary route capture is never
// touched. Centering is applied to `<body>` and to each ancestor strictly
// between body and the wrapper — never to the wrapper itself, which frequently
// carries the component's own padding/background and must keep its own box.
// Each axis independently falls back to `flex-start` when the wrapper overflows
// the viewport, so an oversized component loses its bottom/right edge rather
// than being clipped symmetrically out of its top/left one.
//
// Idempotent (a single injected style id), best-effort, and returns a small
// result object describing what it did.
async function centerCaptureWrapper(target) {
  return target.evaluate(() => {
    const STYLE_ID = "__codeyam_center_capture";
    const ANCESTOR_CLASS = "__codeyam-center-ancestor";
    // Subpixel layout and fractional scrollbar widths make exact gap equality
    // unreliable; a couple of px of slop keeps already-centered pages on the
    // no-op branch.
    const TOLERANCE_PX = 2;

    if (typeof document.getElementById !== "function") {
      return { applied: false, reason: "no-dom" };
    }
    if (document.getElementById(STYLE_ID)) {
      return { applied: false, reason: "already-applied" };
    }
    const wrapper = document.getElementById("codeyam-capture");
    if (!wrapper || typeof wrapper.getBoundingClientRect !== "function") {
      return { applied: false, reason: "no-capture-marker" };
    }

    const rect = wrapper.getBoundingClientRect();
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;

    const leftGap = rect.left;
    const rightGap = viewportWidth - rect.right;
    const topGap = rect.top;
    const bottomGap = viewportHeight - rect.bottom;

    const fitsHorizontally = rect.width <= viewportWidth;
    const fitsVertically = rect.height <= viewportHeight;
    const centeredHorizontally =
      fitsHorizontally && Math.abs(leftGap - rightGap) <= TOLERANCE_PX;
    const centeredVertically =
      fitsVertically && Math.abs(topGap - bottomGap) <= TOLERANCE_PX;

    if (centeredHorizontally && centeredVertically) {
      return { applied: false, reason: "already-centered" };
    }

    const horizontal = fitsHorizontally ? "center" : "flex-start";
    const vertical = fitsVertically ? "center" : "flex-start";

    // Stamp the ancestor chain so the injected rule targets exactly the
    // elements between <body> and the wrapper and cannot leak elsewhere.
    let node = wrapper.parentElement;
    while (node && node !== document.body) {
      if (node.classList && typeof node.classList.add === "function") {
        node.classList.add(ANCESTOR_CLASS);
      }
      node = node.parentElement;
    }

    if (typeof document.createElement !== "function") {
      return { applied: false, reason: "no-dom" };
    }
    const style = document.createElement("style");
    style.id = STYLE_ID;
    style.textContent =
      "body, ." +
      ANCESTOR_CLASS +
      " {" +
      "margin: 0;" +
      "min-height: 100vh;" +
      "display: flex;" +
      "align-items: " +
      vertical +
      ";" +
      "justify-content: " +
      horizontal +
      ";" +
      "}";
    const head = document.head || document.documentElement;
    if (!head || typeof head.appendChild !== "function") {
      return { applied: false, reason: "no-dom" };
    }
    head.appendChild(style);
    return { applied: true, horizontal, vertical };
  });
}

module.exports = {
  scrollThroughDocument,
  forceFinalVisualState,
  centerCaptureWrapper,
};

// Pixel-path script: its code and this number both feed the capture pipeline
// id. Bump the number for a change that alters pixels without changing code.
const CAPTURE_VISUAL_CONTRACT = 1;
module.exports.CAPTURE_VISUAL_CONTRACT = CAPTURE_VISUAL_CONTRACT;
