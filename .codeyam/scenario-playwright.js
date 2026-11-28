// codeyam-generated — DO NOT EDIT.
// codeyam-editor: 0.1.7  protocol: 1  build: 3912480c4b1fdb8b834c1391b27732422f84cedc  source-sha256: ba206b4b165cc1f2d49978d1e949b5c8913bc0767dde96ad70f10661426fc745
const {
  hasLoadingMarkers,
  shouldStopWaitingForImages,
} = require("./scenario-metrics");
const {
  collectContentState,
  collectVisibleTextLength,
  probeCanvasDominated,
  probeCanvasTextMiss,
} = require("./scenario-content-probe");
const {
  candidateLabelSummary,
  canvasTextWaitGuidance,
  pageOf,
  targetBox,
  pointIn,
  devicePixelRatioOf,
  outsideTargetMessage,
  outsideTargetError,
  pointerGeometry,
  locatorPointGeometry,
  stepTimeoutMs,
  pointerOptionsFor,
  locateInteractionTarget,
} = require("./scenario-targeting");
const {
  scrollThroughDocument,
  forceFinalVisualState,
  centerCaptureWrapper,
} = require("./scenario-visual-state");
const fs = require("fs");

// PROTOTYPE (improve35 capture diagnostics): append a per-phase timing line to
// a file so we can see WHERE a slow/timed-out capture spends its budget — even
// when the editor kills this script on timeout (stderr is lost then, but the
// file survives because each line is flushed synchronously). The cwd is the
// project dir (scenario_check.rs sets `.current_dir(project_dir)`), so this
// lands at `<project>/.codeyam/logs/capture-timing.log`. Diagnostics only —
// never throws, never affects the capture result.
function logCaptureTiming(phase, data) {
  try {
    const line = `[${new Date().toISOString()}] [capture-timing] phase=${phase} ${JSON.stringify(
      data,
    )}\n`;
    fs.appendFileSync(".codeyam/logs/capture-timing.log", line);
  } catch (_) {
    /* diagnostics must never break a capture */
  }
}

const net = require("net");

// Resolve a URL to the TCP {host, port} a pre-flight connect should target, or
// null when there is nothing to pre-check — an unparseable URL, or a non-http(s)
// target like `data:`/blank that the capture renders with no network origin.
// Pure (no socket) so the parse, protocol gate, and default-port rules are
// unit-tested without opening a connection.
function resolveTcpTarget(url) {
  let parsed;
  try {
    parsed = new URL(url);
  } catch (_) {
    return null;
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") return null;
  return {
    host: parsed.hostname,
    port: Number(parsed.port) || (parsed.protocol === "https:" ? 443 : 80),
  };
}

// Fast pre-flight (improve35): is anything accepting TCP on the app port? A
// refused connection (the editor's reverse proxy is down) fails in milliseconds;
// without this, the iframe's `waitForLoadState("load")` hangs the FULL 30s on a
// dead origin (the observed capture failure — capture-timing showed
// navigate-iframe elapsedMs=30004, status=null). A connection that ACCEPTS is
// good enough to proceed — a slow HTTP response *after* connect is a cold
// compile, handled by the normal load wait + the editor's retry. So we only bail
// on a hard refusal/timeout, never on slowness.
async function assertAppPortReachable(url, { timeoutMs = 2500 } = {}) {
  const target = resolveTcpTarget(url);
  if (!target) return; // non-http target (data:, blank) — nothing to pre-check
  const { host, port } = target;
  const started = Date.now();
  await new Promise((resolve, reject) => {
    const socket = net.connect({ host, port });
    const finish = (err) => {
      socket.destroy();
      const elapsedMs = Date.now() - started;
      logCaptureTiming("app-port-reachable", {
        host,
        port,
        reachable: !err,
        elapsedMs,
        error: err ? err.message : null,
      });
      if (err) reject(err);
      else resolve();
    };
    socket.setTimeout(timeoutMs, () =>
      finish(
        new Error(
          `app port unreachable: TCP connect to ${host}:${port} timed out after ${timeoutMs}ms — the editor's reverse proxy is not accepting connections (proxy down?)`,
        ),
      ),
    );
    socket.once("connect", () => finish());
    socket.once("error", (e) =>
      finish(
        new Error(
          `app port unreachable: ${host}:${port} ${e.code || e.message} — is the editor's reverse proxy up? (a capture cannot render a dead app port)`,
        ),
      ),
    );
  });
}

function escapeHtmlAttribute(value) {
  return String(value).replaceAll("&", "&amp;").replaceAll('"', "&quot;");
}

// The harness background defaults to transparent so the iframe's own
// <body> background paints through — matching what users see in the Live
// Preview. Callers (via scenario-check.js) pass a concrete color when the
// UI has detected a background it wants the capture to paint behind the
// iframe, e.g. `var(--bg-deep)` from the editor shell.
// The iframe is named `codeyam-preview` (CODEYAM_PREVIEW_FRAME_NAME in
// ui/src/helpers/previewMockPolicy.ts) so a framed editor page knows it is
// inside codeyam's own capture, not a hosting IDE's pane.
function buildIframeHarness(url, { background = "transparent" } = {}) {
  const escapedUrl = escapeHtmlAttribute(url);
  const bg = String(background);
  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <style>
      html, body {
        margin: 0;
        width: 100%;
        height: 100%;
        overflow: hidden;
        background: ${bg};
      }

      iframe {
        display: block;
        width: 100%;
        height: 100%;
        border: 0;
        background: ${bg};
      }
    </style>
  </head>
  <body>
    <iframe id="scenario-frame" name="codeyam-preview" title="Scenario Preview" src="${escapedUrl}"></iframe>
  </body>
</html>`;
}

const path = require("path");

// The reserved route the editor serves the secure-context iframe harness from.
// Mirrors `HARNESS_PATH` in crates/control-api/src/preview_proxy_route.rs — the
// canonical source of the harness markup now lives in that Rust handler; the
// `buildIframeHarness` template above survives only as the degraded fallback
// below for when the harness origin can't be resolved.
const HARNESS_PATH = "/__codeyam_harness";

// Read `.codeyam/server-state.json` (cwd is the project dir — see the timing
// log note above) and return its parsed object, or null when it is absent.
function defaultReadServerState() {
  const statePath = path.join(process.cwd(), ".codeyam", "server-state.json");
  if (!fs.existsSync(statePath)) return null;
  return JSON.parse(fs.readFileSync(statePath, "utf8"));
}

// Resolve the loopback origin that serves the iframe-harness route. The harness
// is mounted on the editor's control-api listener, whose port is recorded in
// `.codeyam/server-state.json` as `controlPort` (the same file the top-level
// loader already reads for `appPort`). Returns `http://127.0.0.1:<controlPort>`
// — a secure context the nested iframe inherits, exactly as `localhost` is — or
// null when the state file is missing/unreadable, in which case the caller falls
// back to the legacy in-page `setContent` harness. `readStateFile` is injectable
// so the resolver is unit-testable without disk.
//
// The HOST is taken from `targetUrl` — the URL the iframe will load — and only
// the port comes from server-state. That mirroring is load-bearing, because the
// two capture modes address the editor by different loopback spellings:
// `/__codeyam_preview` captures are pinned to `127.0.0.1` (PROXY_CAPTURE_LOOPBACK
// in handlers.rs, matching the forwarder's own pinning) while direct app-port
// captures use `localhost`. `localhost` and `127.0.0.1` are DIFFERENT sites to
// the cookie jar, so whenever the harness host and the iframe host disagree the
// nested load is cross-site — and the `cy_session` cookie is `SameSite=Lax`,
// which rides top-level navigations only. It is therefore withheld from the
// iframe request (and from the `/api/*` calls the framed page makes), and the
// token-gated routes answer 401 on a non-loopback bind.
//
// Hardcoding EITHER spelling only moves the failure between the two modes:
// `localhost` 401s every `/__codeyam_preview` capture, `127.0.0.1` 401s the
// framed page's own `/api/scenarios` + `/api/render-environment` calls. Raising
// the cookie to `SameSite=None` fixes neither — Chromium rejects `None` without
// `Secure`, and these origins are plain http. Mirroring the target's host is what
// keeps the harness same-site in both modes, which is also the same-origin model
// the `/__codeyam_preview` subpath proxy exists to provide.
//
// `targetUrl` omitted or unparseable falls back to `127.0.0.1`, the spelling the
// proxy-route capture (the token-gated one) uses.
function resolveHarnessOrigin({
  readStateFile = defaultReadServerState,
  targetUrl = null,
} = {}) {
  try {
    const state = readStateFile();
    const port = state && state.controlPort;
    if (typeof port === "number" && port > 0) {
      let host = "127.0.0.1";
      if (targetUrl) {
        try {
          host = new URL(targetUrl).hostname || host;
        } catch (_) {
          /* unparseable target — keep the proxy-route default */
        }
      }
      return `http://${host}:${port}`;
    }
  } catch (_) {
    /* missing/unreadable state — fall back to setContent */
  }
  return null;
}

// Build the top-level harness URL for a scenario `url` and background. The
// editor-served harness document embeds `src` as the iframe URL, so the nested
// scenario inherits the harness's secure context. The background rides along as
// `bg`. Pure — `URLSearchParams` does the percent-encoding so a scenario URL
// with its own query string survives intact. Pure.
function buildHarnessUrl(harnessOrigin, url, background) {
  const params = new URLSearchParams({ src: url });
  if (background != null && background !== "") {
    params.set("bg", String(background));
  }
  return `${harnessOrigin}${HARNESS_PATH}?${params.toString()}`;
}

async function collectImageStates(target) {
  return target.evaluate(() =>
    Array.from(document.images || []).map((img) => ({
      complete: img.complete,
      naturalWidth: img.naturalWidth,
      src: img.currentSrc || img.src || "",
    })),
  );
}

async function waitForImagesSettled(
  target,
  { overallTimeoutMs = 5000, pollIntervalMs = 100 } = {},
) {
  const started = Date.now();
  let images = await collectImageStates(target);
  while (
    !shouldStopWaitingForImages(images, {
      elapsedMs: Date.now() - started,
      overallTimeoutMs,
    })
  ) {
    await new Promise((r) => setTimeout(r, pollIntervalMs));
    images = await collectImageStates(target);
  }
  const elapsedMs = Date.now() - started;
  const allComplete = images.every((img) => img && img.complete === true);
  const incompleteSrcs = images
    .filter((img) => !img || img.complete !== true || !(img.naturalWidth > 0))
    .map((img) => (img && img.src) || "")
    .slice(0, 6);
  logCaptureTiming("images-settled", {
    elapsedMs,
    settled: allComplete,
    total: images.length,
    incompleteCount: incompleteSrcs.length,
    incompleteSrcs,
    overallTimeoutMs,
  });
  return { settled: allComplete, images, elapsedMs };
}

async function waitForAnimationsSettled(
  target,
  { timeoutMs = 2000, pollIntervalMs = 100 } = {},
) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    const runningCount = await target.evaluate(() =>
      document
        .getAnimations()
        .filter((a) => a.playState === "running").length,
    );
    if (runningCount === 0) {
      return { settled: true, elapsedMs: Date.now() - started };
    }
    await new Promise((r) => setTimeout(r, pollIntervalMs));
  }
  return { settled: false, elapsedMs: Date.now() - started };
}

// Track in-flight network requests on a Playwright page so a capture can wait
// for client-side data fetches to settle before screenshotting. The
// resource-timing API (`performance.getEntriesByType("resource")`) only records
// COMPLETED requests, so it cannot see a fetch that is still in flight — the
// exact window where a client-fetch page shows a loading skeleton. We count
// request starts against finishes/failures instead. Returns a live view:
// `inFlight()` is the current outstanding count and `lastActivityMs()` is the
// timestamp of the most recent request start OR completion (0 when the page has
// made no requests since the tracker attached). Attach BEFORE navigation so
// every request is counted. Stack-agnostic — it observes raw HTTP activity, not
// any framework's fetch wrapper.
function createNetworkTracker(page) {
  let inFlight = 0;
  let lastActivityMs = 0;
  const bump = () => {
    lastActivityMs = Date.now();
  };
  if (page && typeof page.on === "function") {
    page.on("request", () => {
      inFlight += 1;
      bump();
    });
    const settle = () => {
      inFlight = Math.max(0, inFlight - 1);
      bump();
    };
    page.on("requestfinished", settle);
    page.on("requestfailed", settle);
  }
  return {
    inFlight: () => inFlight,
    lastActivityMs: () => lastActivityMs,
  };
}

// Bounded network-quiet wait: after the DOM is stable a client-side data fetch
// can still be in flight — the loading skeleton is gone but the fetched rows
// haven't replaced it yet, so a screenshot here catches the in-between frame.
// Wait until no request has been outstanding for `quietWindowMs`, hard-capped at
// `overallTimeoutMs`. Two properties matter:
//   - A page that made NO requests (lastActivityMs stays 0) is already quiet and
//     returns on the first poll, so server-rendered captures incur no extra wait.
//   - A streaming / long-poll endpoint that never goes idle hits the cap and the
//     caller captures anyway — the wait can never hang the capture.
async function waitForNetworkQuiet(
  tracker,
  { quietWindowMs = 500, overallTimeoutMs = 5000, pollIntervalMs = 100 } = {},
) {
  const started = Date.now();
  while (Date.now() - started < overallTimeoutMs) {
    const idleForMs = Date.now() - tracker.lastActivityMs();
    if (tracker.inFlight() === 0 && idleForMs >= quietWindowMs) {
      const elapsedMs = Date.now() - started;
      logCaptureTiming("network-quiet", { outcome: "quiet", elapsedMs });
      return { quiet: true, elapsedMs };
    }
    await new Promise((r) => setTimeout(r, pollIntervalMs));
  }
  const elapsedMs = Date.now() - started;
  logCaptureTiming("network-quiet", {
    outcome: "capped",
    elapsedMs,
    inFlight: tracker.inFlight(),
  });
  return { quiet: false, elapsedMs };
}

// `loadingMarkers` are the project's app-specific loading strings (from
// stack.json `capture.loadingMarkers`); they extend the codeyam-harness
// defaults so a stable-but-still-loading app screen counts as "not ready"
// and the loop keeps waiting instead of capturing the loading flash.
//
// `confirmOnly` is for a page whose app already signalled ready (see
// `waitForReadySignal`). The signal answers "has it painted?", so a single
// sample, taken 100 ms in rather than 500, only confirms that no loading marker
// is up and the root is painted. It does not wait for two identical polls,
// which put a 1.5 s floor under every capture.
async function waitForStablePage(
  page,
  target,
  timeoutMs = 10000,
  loadingMarkers = [],
  { confirmOnly = false } = {},
) {
  const started = Date.now();
  const pollIntervalMs = confirmOnly ? 100 : 500;
  const requiredStableSamples = confirmOnly ? 1 : 2;
  let lastHtml = "";
  let stableCount = 0;
  let lastHadLoadingMarkers = false;
  let lastHtmlChanged = false;

  while (Date.now() - started < timeoutMs) {
    await page.waitForTimeout(pollIntervalMs);

    const pageState = await target.evaluate(() => {
      const getById = document.getElementById;
      const root =
        typeof getById === "function" ? document.getElementById("root") : null;
      return {
        bodyText: document.body?.innerText ?? "",
        html: document.body?.innerHTML ?? "",
        // Whether the SPA mount point exists AND has painted anything. An
        // existing-but-empty `<div id="root">` is the pre-paint window of a
        // slow-first-paint scenario (e.g. a live-session app that spends a few
        // seconds connecting before the gate UI mounts). `rootExists` lets us
        // distinguish that from a mid-redirect/teardown `null` body, where
        // there is no root to wait on and a stable-empty page is legitimately
        // settled.
        rootExists: !!root,
        rootChildCount: root ? root.childElementCount : 0,
      };
    });

    lastHadLoadingMarkers = hasLoadingMarkers(pageState.bodyText, loadingMarkers);
    lastHtmlChanged = pageState.html !== lastHtml;

    // A mounted-but-unpainted root is "still loading", not "settled": the HTML
    // can sit byte-stable for a second or two while the SPA boots, which would
    // otherwise satisfy the stability check and capture a blank frame before
    // first paint. Treat an existing root with zero children AND no body text
    // as not-ready so the loop keeps polling until the app actually paints (or
    // the overall timeout fires, by which point real content is present). A
    // `null` body (rootExists=false) is unaffected — it stays trivially stable.
    const rootUnpainted =
      pageState.rootExists &&
      pageState.rootChildCount === 0 &&
      (pageState.bodyText ?? "").trim().length === 0;

    if (!lastHadLoadingMarkers && (confirmOnly || !lastHtmlChanged) && !rootUnpainted) {
      stableCount += 1;
      if (stableCount >= requiredStableSamples) {
        const remaining = () => Math.max(0, timeoutMs - (Date.now() - started));
        await waitForAnimationsSettled(target, {
          timeoutMs: Math.min(2000, remaining()),
        });
        await waitForImagesSettled(target, { overallTimeoutMs: remaining() });
        logCaptureTiming("stable-page", {
          outcome: "stabilized",
          elapsedMs: Date.now() - started,
        });
        return { stabilized: true, hadLoadingMarkers: false };
      }
    } else {
      stableCount = 0;
    }

    lastHtml = pageState.html;
  }
  // Hit the cap without stabilizing — record WHY: a persistent loading marker
  // (app stuck) vs HTML still mutating each poll (HMR / animation / re-render).
  // The returned `hadLoadingMarkers` is the signal the capture advisory keys
  // off: a marker still on screen at the cap means the page never finished its
  // (likely client-side) load, so the screenshot caught its loading state.
  logCaptureTiming("stable-page", {
    outcome: "timed-out",
    elapsedMs: Date.now() - started,
    timeoutMs,
    lastHadLoadingMarkers,
    lastHtmlStillChanging: lastHtmlChanged,
  });
  return { stabilized: false, hadLoadingMarkers: lastHadLoadingMarkers };
}

// Poll an app-declared ready-signal expression (stack.json
// `capture.readySignal`) in `target` until it is truthy or `timeoutMs` elapses.
// This is how a canvas app says "first frame painted": its pixels never move
// `innerHTML`, so `waitForStablePage` reads it as settled before the wasm has
// drawn anything. An expression that throws (the app global not yet defined)
// counts as not-ready rather than failing the wait. Never throws; the caller
// turns `ready: false` into a capture issue.
async function waitForReadySignal(target, signal, timeoutMs) {
  if (!signal) return { awaited: false, ready: true, elapsedMs: 0 };
  const started = Date.now();
  const guarded = `(() => { try { return !!(${signal}); } catch (_) { return false; } })()`;
  try {
    await target.waitForFunction(guarded, undefined, {
      timeout: timeoutMs,
      polling: 100,
    });
    const elapsedMs = Date.now() - started;
    logCaptureTiming("ready-signal", { outcome: "ready", elapsedMs });
    return { awaited: true, ready: true, elapsedMs };
  } catch (_) {
    const elapsedMs = Date.now() - started;
    logCaptureTiming("ready-signal", { outcome: "timed-out", elapsedMs, timeoutMs });
    return { awaited: true, ready: false, elapsedMs };
  }
}

// Start watching for `frame` to navigate, from the moment its ready signal
// arrived. A navigation after that point means something reloaded the page
// under the capture (a dev server's live reload): the frame about to be shot is
// no longer the one that signalled ready. Returns `{ navigated() }`, read just
// before the screenshot. A page that cannot report navigations (a stubbed test
// target) reads as never navigated.
function watchNavigationAfterReady(page, frame) {
  let navigated = false;
  if (page && typeof page.on === "function") {
    page.on("framenavigated", (navigatedFrame) => {
      if (navigatedFrame === frame) navigated = true;
    });
  }
  return { navigated: () => navigated };
}

// `preflight` is injectable so unit tests that drive a mock page can stay
// network-free; production callers use the default real reachability check.
async function loadScenarioInIframe(
  page,
  url,
  { background, preflight = assertAppPortReachable, harnessOrigin } = {},
) {
  await preflight(url);
  // `undefined` (the default) means "resolve from server-state"; an explicit
  // value (including `null`) is honored as-is so tests can force either path.
  const resolvedHarnessOrigin =
    harnessOrigin !== undefined ? harnessOrigin : resolveHarnessOrigin();
  const navStarted = Date.now();
  const responsePromise = page
    .waitForResponse(
      (response) =>
        response.request().resourceType() === "document" &&
        response.url() === url,
      { timeout: 30000 },
    )
    .catch(() => null);

  if (resolvedHarnessOrigin) {
    // Navigate the page TOP-LEVEL to the harness document served from the
    // editor's `localhost` origin, so the ancestor document is a secure context
    // and the nested scenario iframe inherits it (the Sveltia-class fix). The
    // inner iframe `src` is still `url`, so the document-response probe above is
    // unchanged.
    const harnessUrl = buildHarnessUrl(resolvedHarnessOrigin, url, background);
    const harnessResponse = await page.goto(harnessUrl, {
      waitUntil: "domcontentloaded",
    });
    // Fail loudly on a fail-closed harness-auth rejection BEFORE the blind 30s
    // `#scenario-frame` wait below. On a non-loopback (0.0.0.0 / cloud) bind the
    // harness control-API route is session-token-gated; a generated
    // `.codeyam/capture.js` (or an editor binary) that predates the `cy_session`
    // cookie injection navigates without a token -> the server refuses the
    // request with 401 -> the harness document never renders -> every component
    // scenario times out on `#scenario-frame` with a generic Playwright error
    // that hides the real cause. Surfacing the 401 here turns that blind timeout
    // into a one-line root cause on the very first failing capture, naming both
    // fixes. (Route/page scenarios navigate top-level to the un-gated app proxy,
    // so they never hit this path.)
    if (harnessResponse && harnessResponse.status() === 401) {
      throw new Error(
        `Capture harness route ${harnessUrl} returned 401 (fail-closed ` +
          `session-token auth on a non-loopback bind). The capture script did not ` +
          `send the \`cy_session\` token — most often a generated ` +
          `\`.codeyam/capture.js\` (or an editor binary) that predates session-token ` +
          `injection. Fixes: refresh the editor binary on this VM, or set ` +
          `\`CODEYAM_INSECURE_BIND=1\` and restart the editor.`,
      );
    }
  } else {
    // Degraded fallback: no resolvable harness origin (server-state missing), so
    // use the legacy in-page harness. The top-level document is then
    // `about:blank` — NOT a secure context — so a secure-context-gated app may
    // refuse to mount, but every non-secure-context scenario captures exactly as
    // before.
    await page.setContent(buildIframeHarness(url, { background }), {
      waitUntil: "domcontentloaded",
    });
  }

  const frameHandle = await page.waitForSelector("#scenario-frame", {
    state: "attached",
    timeout: 30000,
  });
  const frame = await frameHandle.contentFrame();
  if (!frame) {
    throw new Error("Scenario iframe did not attach");
  }

  await frame.waitForLoadState("load", { timeout: 30000 });
  const response = await responsePromise;
  logCaptureTiming("navigate-iframe", {
    elapsedMs: Date.now() - navStarted,
    status: response ? response.status() : null,
    url,
  });
  return { frame, response };
}

// Load the scenario as a top-level navigation instead of embedding it in
// the iframe harness. A top-level document is a first-party context, so a
// `SameSite=Lax` session cookie is sent on the navigation — which is what
// auth-gated application routes need to render the authenticated page
// rather than redirecting to /login. The returned `frame` is the page's
// main frame so callers can treat it uniformly with the iframe path
// (`frame.url()`, `frame.evaluate(...)`, `waitForStablePage(page, frame)`).
async function loadScenarioTopLevel(
  page,
  url,
  { preflight = assertAppPortReachable } = {},
) {
  await preflight(url);
  const navStarted = Date.now();
  try {
    const response = await page.goto(url, {
      waitUntil: "load",
      timeout: 30000,
    });
    logCaptureTiming("navigate-toplevel", {
      elapsedMs: Date.now() - navStarted,
      status: response ? response.status() : null,
      url,
    });
    return { frame: page.mainFrame(), response };
  } catch (error) {
    if (error.message && error.message.toLowerCase().includes("timeout")) {
      let parsed = null;
      try {
        parsed = new URL(url);
      } catch (_) {}
      if (parsed) {
        let appPort = null;
        try {
          const path = require("path");
          const statePath = path.join(process.cwd(), ".codeyam", "server-state.json");
          if (fs.existsSync(statePath)) {
            const state = JSON.parse(fs.readFileSync(statePath, "utf8"));
            appPort = state.appPort;
          }
        } catch (_) {}

        if (appPort) {
          let appHealthy = false;
          try {
            await assertAppPortReachable(`http://127.0.0.1:${appPort}/`, { timeoutMs: 500 });
            appHealthy = true;
          } catch (_) {}

          if (appHealthy) {
            throw new Error(
              `proxy navigation timed out: the proxy at 127.0.0.1:${parsed.port} did not respond within 30000ms. app healthy on :${appPort}, proxy dead on :${parsed.port}.`
            );
          } else {
            throw new Error(
              `proxy navigation timed out: the proxy at 127.0.0.1:${parsed.port} did not respond within 30000ms. app also unresponsive on :${appPort}.`
            );
          }
        } else {
          throw new Error(
            `proxy navigation timed out: the proxy at 127.0.0.1:${parsed.port} did not respond within 30000ms. proxy dead/hung?`
          );
        }
      }
    }
    throw error;
  }
}

// Text matching prefers an EXACT, INTERACTIVE element over a looser substring /
// plain-text hit (see `resolveTextTarget`), so the first attempt lands on the
// chip or button the agent meant. When a genuine ambiguity survives that
// resolution (>1 candidate in the winning tier), we still act — on the best
// visible candidate — but push an actionable warning naming every candidate
// into the optional `warnings` array so the agent can switch to an exact
// selector; a zero-match throws an error that spells out the substring caveat
// and the exact-selector / URL-query-param alternatives.
// Verbs that drive the page's mouse / keyboard directly rather than a located
// element's own handlers — the canvas verbs. A `<canvas>` paints every control
// into one element, so these aim at a position inside a DOM target (or, for
// `key` / `type`, at whatever already has focus).
const PAGE_KEYBOARD_ACTIONS = ["key", "type"];
const DEFAULT_DRAG_STEPS = 10;

// True for a step whose effect is painted rather than written to the DOM: any
// positional click/hover, any drag or wheel, and page-level keyboard input.
// Callers use it to settle on pixels instead of `innerHTML`, and to skip the
// DOM-only hydration-race retry, which would turn one canvas click into two.
function isCanvasInteraction(interaction) {
  const { action, position } = interaction || {};
  return (
    action === "drag" ||
    action === "wheel" ||
    PAGE_KEYBOARD_ACTIONS.includes(action) ||
    (position != null && (action === "click" || action === "hover"))
  );
}

// Page-level keyboard input. Deliberately not `locator.press`: a `<canvas>`
// without `tabindex` never receives focus, so a locator-scoped press is
// delivered nowhere. A preceding click on the canvas gives the app focus.
async function performKeyboardInteraction(frame, { action, value }) {
  const keyboard = pageOf(frame).keyboard;
  if (action === "key") {
    await keyboard.press(value || "Enter");
  } else {
    await keyboard.type(value ?? "");
  }
}

// Resolve a step's target to the single element to act on, with this script's
// canvas probe as the default (see `locateInteractionTarget`). Shared by
// `performInteraction` and by callers that need the target's box BEFORE
// acting (the pixel-effect signal clips its before/after shots to it).
function resolveInteractionTarget(frame, interaction, opts = {}) {
  const canvasProbe = opts.canvasProbe ?? probeCanvasTextMiss;
  return locateInteractionTarget(frame, interaction, { ...opts, canvasProbe });
}

// Drive a single user-style interaction against the settled frame before the
// screenshot, so an interactive state (expanded accordion, open modal, filled
// field) can be captured without editing app source.
//
// The target is matched by visible `text` (preferred — the agent reliably
// knows the label it rendered) or a CSS `selector`. `action` is click / fill /
// press; `value` carries the text for `fill` or the key for `press` (e.g.
// `Enter`). On a no-match target this THROWS with the list of candidate
// interactive labels — the capture script's outer catch turns that into a
// failed capture with an actionable message, never a silent blank screenshot.
async function performInteraction(
  frame,
  interaction,
  { timeoutMs = 5000, appearTimeoutMs = timeoutMs, warnings } = {},
) {
  const { action, value, position, from, to, steps, button, deltaX, deltaY } =
    interaction || {};

  if (PAGE_KEYBOARD_ACTIONS.includes(action)) {
    try {
      await performKeyboardInteraction(frame, { action, value });
    } catch (error) {
      throw new Error(
        `preview-interact: action "${action}" failed: ${error.message || String(error)}`,
      );
    }
    return;
  }

  // Before target resolution: `hidden` must pass on an absent target.
  if (action === "waitFor") {
    const bound = { defaultTimeoutMs: Math.max(timeoutMs, 8000) };
    return waitForPredicate(frame, interaction, bound).catch(async (error) => {
      const labels = await candidateLabelSummary(frame);
      throw new Error(`preview-interact: ${error.message}. Candidate interactive labels: ${labels}`);
    });
  }

  // A step's own `timeoutMs` replaces both defaults: a heavy page is slow to
  // render the target and slow to make it actionable.
  const actionTimeoutMs = stepTimeoutMs(interaction, timeoutMs);
  const { locator, targetDesc } = await resolveInteractionTarget(frame, interaction, {
    warnings,
    appearTimeoutMs: stepTimeoutMs(interaction, appearTimeoutMs),
  });
  const pointerOpts = pointerOptionsFor(interaction, actionTimeoutMs);

  // Pointer verbs report where they landed; every other verb returns undefined.
  let geometry;
  try {
    switch (action) {
      case "click":
        geometry = await locatorPointGeometry(frame, locator, targetDesc, action, position);
        await locator.click(pointerOpts);
        break;
      case "fill":
        await locator.fill(value ?? "", { timeout: actionTimeoutMs });
        break;
      case "press":
        await locator.press(value || "Enter", { timeout: actionTimeoutMs });
        break;
      case "hover":
        // Reveals hover-only affordances (an action bar, a tooltip) — one of the
        // most common ephemeral states a resting-render screenshot misses.
        geometry = await locatorPointGeometry(frame, locator, targetDesc, action, position);
        await locator.hover(pointerOpts);
        break;
      case "drag": {
        // Pan / select-box / move: built from raw mouse events because a canvas
        // has no drop target for `locator.dragTo`.
        const box = await targetBox(locator, targetDesc);
        const mouse = pageOf(frame).mouse;
        const start = pointIn(box, from);
        if (!start.inside) {
          throw await outsideTargetError(frame, action, "from", from, targetDesc, box);
        }
        const end = pointIn(box, to);
        geometry = pointerGeometry(box, start, end);
        // Ending off the target can be deliberate (dragging something off the
        // canvas), so it warns rather than refuses.
        if (!end.inside) {
          geometry.warning =
            `drag ends outside the target: ` +
            outsideTargetMessage("to", to, targetDesc, box, await devicePixelRatioOf(frame));
          if (Array.isArray(warnings)) warnings.push(`preview-interact: ${geometry.warning}`);
        }
        const mouseButton = { button: button || "left" };
        await mouse.move(start.x, start.y);
        await mouse.down(mouseButton);
        await mouse.move(end.x, end.y, {
          steps: typeof steps === "number" && steps > 0 ? steps : DEFAULT_DRAG_STEPS,
        });
        await mouse.up(mouseButton);
        break;
      }
      case "wheel": {
        // Zoom / scroll: the wheel event lands wherever the pointer is, so move
        // there first.
        const box = await targetBox(locator, targetDesc);
        const mouse = pageOf(frame).mouse;
        const at = pointIn(box, position);
        if (!at.inside) {
          throw await outsideTargetError(frame, action, "position", position, targetDesc, box);
        }
        geometry = pointerGeometry(box, at);
        await mouse.move(at.x, at.y);
        await mouse.wheel(deltaX || 0, deltaY || 0);
        break;
      }
      default:
        throw new Error(
          `preview-interact: unknown action "${action}" (expected click | fill | press | hover | drag | wheel | key | type | waitFor)`,
        );
    }
  } catch (error) {
    if (error && error.outsideTarget) throw error;
    throw new Error(
      `preview-interact: action "${action}" failed against ${targetDesc}: ${error.message || String(error)}. ` +
        `Candidate interactive labels: ${await candidateLabelSummary(frame)}`,
    );
  }
  return geometry;
}

// One screenshot of `page`, clipped to `clip` when given, as a Buffer. The
// pixel half of the interaction-effect signal and of the canvas settle. A
// failed shot returns null, which compares unequal to everything — so a flaky
// screenshot reads as "changed / not yet settled", never as a false "inert".
async function pixelSnapshot(page, clip) {
  try {
    const opts = { fullPage: false };
    if (clip && clip.width > 0 && clip.height > 0) opts.clip = clip;
    return await page.screenshot(opts);
  } catch (_) {
    return null;
  }
}

function samePixels(a, b) {
  return Buffer.isBuffer(a) && Buffer.isBuffer(b) && a.equals(b);
}

// Settle a canvas-painted page: `waitForStablePage` compares `innerHTML`, which
// a canvas never changes, so after a canvas verb it reads "settled" while the
// app is still animating a pan or zoom. Wait instead for two consecutive equal
// screenshots, bounded by `timeoutMs` — the same "real signal, safety bound"
// rule the DOM settle follows.
async function waitForPixelStable(
  page,
  { clip, timeoutMs = 5000, intervalMs = 150, snapshot = pixelSnapshot } = {},
) {
  const deadline = Date.now() + timeoutMs;
  let previous = await snapshot(page, clip);
  while (Date.now() < deadline) {
    await new Promise((resolve) => setTimeout(resolve, intervalMs));
    const current = await snapshot(page, clip);
    if (samePixels(previous, current)) return true;
    previous = current;
  }
  return false;
}

// Hold until a visible-text or selector predicate becomes true, bounded by a
// wall-clock timeout. Behavioral demos hinge on transient states (an overlay
// appears, then a new round renders); a flow step waits for that real signal
// instead of a fixed sleep — the same "hold to a real signal with a safety
// bound" rule the rest of the capture pipeline follows. THROWS on timeout
// naming the predicate and the bound, so the capture script's outer catch
// turns a never-appearing predicate into an actionable failure rather than an
// infinite hang. `text` is matched case-insensitively as a substring (the
// agent reliably knows the copy it rendered); `selector` is a CSS selector.
//
// A text predicate on a canvas-dominated page is refused at once rather than
// after the full bound: canvas-painted words are never DOM text, so the wait
// could only time out. A hybrid page (DOM text beside a canvas) is not
// canvas-dominated and keeps the ordinary wait; a canvas that mounts late is
// caught on the timeout path, which re-checks and appends the same guidance.
// A selector predicate is never refused — naming the canvas element itself is
// a legitimate wait for that element. `canvasProbe` is injectable so unit
// tests classify the page without a browser.
async function waitForPredicate(
  frame,
  predicate,
  { defaultTimeoutMs = 8000, canvasProbe = probeCanvasDominated } = {},
) {
  const { text, selector, timeoutMs, state = "visible" } = predicate || {};
  const bound =
    typeof timeoutMs === "number" && timeoutMs > 0 ? timeoutMs : defaultTimeoutMs;

  let locator;
  let desc;
  const byText = typeof text === "string" && text.length > 0;
  if (byText) {
    if (await canvasProbe(frame)) {
      throw new Error(`waitFor: predicate text "${text}" ${canvasTextWaitGuidance()}`);
    }
    locator = frame.getByText(text, { exact: false }).first();
    desc = `text "${text}"`;
  } else if (typeof selector === "string" && selector.length > 0) {
    locator = frame.locator(selector).first();
    desc = `selector "${selector}"`;
  } else {
    throw new Error("waitFor: predicate requires a `text` or `selector` target");
  }

  try {
    // `hidden` also resolves when nothing matches, so a spinner that is
    // removed (not merely hidden) still ends the wait.
    await locator.waitFor({ state, timeout: bound });
  } catch (_) {
    const lateCanvas = byText && (await canvasProbe(frame));
    throw new Error(
      `waitFor: predicate ${desc} did not become ${state} within ${bound}ms` +
        (lateCanvas ? `. The page is now canvas-dominated: ${canvasTextWaitGuidance()}` : ""),
    );
  }
}

// Drive an ordered sequence of interactions against the settled frame, settling
// the page between each so a later step sees the DOM the earlier one produced.
// This is the persisted-scenario path (`scenario.interactions`): unlike the
// single fire-and-forget `preview-interact`, the whole sequence is replayed on
// every capture and recapture. Any step that matches nothing throws (with the
// candidate-labels hint from `performInteraction`), and the caller turns that
// into a failed capture — never a silent resting-state screenshot for a
// sequence that didn't fully run.
// A step that matches SOMETHING but changes nothing is the other half: after a
// redesign a text target can still resolve to an inert label, and a canvas
// click at a stale position lands on empty paint. So the viewport is
// snapshotted before the first step and after each step until one changes it,
// and a sequence that was meant to change the page but in which no step did
// fails too (see `deadInteractionSequenceMessage`). A sequence that changes the
// page and then undoes it is not dead, even though it ends where it began. A pure `waitFor` sequence is exempt — it
// asserts state, it does not change it. A snapshot that could not be taken
// (null) never fails the capture.
// `settle`, `pixelSettle` and `snapshot` are injectable so unit tests that
// drive a mock frame stay network-free and fast; production callers use the
// real `waitForStablePage` re-settle and a full-viewport `pixelSnapshot`.
async function performInteractionSequence(
  page,
  frame,
  interactions,
  {
    timeoutMs = 5000,
    appearTimeoutMs = timeoutMs,
    settleMs = 5000,
    loadingMarkers,
    settle = waitForStablePage,
    pixelSettle = waitForPixelStable,
    snapshot = pixelSnapshot,
    warnings,
  } = {},
) {
  const expectsEffect = interactions.some((step) => (step || {}).action !== "waitFor");
  // The frame the next step is judged against, and whether any step so far
  // changed it. Judging each step against its predecessor (not the last frame
  // against the first) lets a sequence that undoes itself — select then clear,
  // open then close — pass, while one where no step did anything still fails.
  let previous = expectsEffect ? await snapshot(page, null) : null;
  let changed = false;
  for (let i = 0; i < interactions.length; i += 1) {
    try {
      await performInteraction(frame, interactions[i], {
        timeoutMs,
        appearTimeoutMs,
        warnings,
      });
    } catch (err) {
      // Prefix the failing step's index so a miss in a multi-step sequence is
      // locatable, matching the model-side `interactions[i]` validator.
      throw new Error(`interactions[${i}]: ${err.message}`);
    }
    await settle(page, frame, settleMs, loadingMarkers);
    if (isCanvasInteraction(interactions[i])) {
      await pixelSettle(page, { timeoutMs: settleMs });
    }
    // Once any step changed the frame there is nothing left to prove, so the
    // remaining steps take no snapshot. A null snapshot is unknown, and
    // `samePixels` reads it as "changed" so it never fails a capture.
    if (expectsEffect && !changed) {
      const current = await snapshot(page, null);
      changed = !samePixels(previous, current);
      previous = current;
    }
  }
  if (expectsEffect && !changed) {
    throw new Error(deadInteractionSequenceMessage(interactions));
  }
}

// The stable head of the dead-sequence failure. `recapture-stale` matches it
// (with the no-match wording) to report a sweep's dead targets as one cause.
const DEAD_INTERACTION_MARKER = "interactions had no visible effect";

// One step as the operator wrote it: `click text "Save"`, `click selector
// "canvas" at (120, 40)`, `key "Enter"`.
function describeInteractionStep(step) {
  const { action, text, selector, position, value } = step || {};
  let target = "";
  if (typeof text === "string" && text) target = ` text "${text}"`;
  else if (typeof selector === "string" && selector) target = ` selector "${selector}"`;
  else if (typeof value === "string" && value) target = ` "${value}"`;
  const at =
    position && typeof position.x === "number" && typeof position.y === "number"
      ? ` at (${position.x}, ${position.y})`
      : "";
  return `${action || "?"}${target}${at}`;
}

// Why a sequence whose frame did not change fails, naming every step so the
// scenario author can see which targets the current UI no longer honors.
function deadInteractionSequenceMessage(interactions) {
  const steps = interactions
    .map((step, i) => `[${i}] ${describeInteractionStep(step)}`)
    .join("; ");
  return (
    `${DEAD_INTERACTION_MARKER}: the viewport is pixel-identical before and ` +
    `after each of all ${interactions.length} step(s) (${steps}). Each step resolved a ` +
    `target, but nothing it did changed the page, so this frame would be the ` +
    `scenario's resting screen, not the state it declares. The usual cause is a ` +
    `UI change that moved or renamed the control a step aims at (a text label ` +
    `that now matches inert copy, or a canvas position that is now empty). ` +
    `Update the scenario's interactions to target the current control.`
  );
}

// The query-param isolation harness renders a placeholder instead of the
// component when the component manifest has no entry for it (most often a fresh
// checkout, where the gitignored manifest is absent). That placeholder is a
// perfectly stable page, so nothing else in the capture path notices it: an
// interaction against it was reported as a selector miss, and a recapture
// saved it over the scenario's good committed screenshot. The shipped harness
// marks it with this attribute; harnesses scaffolded before the marker existed
// are matched by codeyam's own placeholder wording inside `#codeyam-capture`
// (a compatibility shim for text codeyam wrote, never app text).
const ISOLATE_MISSING_ATTR = "data-codeyam-isolate-missing";
const LEGACY_ISOLATE_MISSING_PATTERN = /^\s*No such component "([^"]*)"/;

// The component name the harness placeholder reports as missing, or null when
// the frame rendered something else.
async function isolateMissingComponent(frame) {
  return frame
    .evaluate(
      ({ attr, pattern }) => {
        const marked = document.querySelector(`[${attr}]`);
        if (marked) return marked.getAttribute(attr) || "";
        const wrapper = document.getElementById("codeyam-capture");
        if (!wrapper) return null;
        const match = (wrapper.textContent || "").match(new RegExp(pattern));
        return match ? match[1] : null;
      },
      {
        attr: ISOLATE_MISSING_ATTR,
        pattern: LEGACY_ISOLATE_MISSING_PATTERN.source,
      },
    )
    .catch(() => null);
}

// `routing.componentManifest` from `.codeyam/stack.json`, for naming the file
// in the error; a generic phrase when the stack declares none.
function componentManifestPath() {
  try {
    const stack = JSON.parse(
      fs.readFileSync(require("path").join(".codeyam", "stack.json"), "utf8"),
    );
    const manifest = stack && stack.routing && stack.routing.componentManifest;
    if (typeof manifest === "string" && manifest) return manifest;
  } catch (_) {}
  return "the component manifest";
}

function isolateMissingComponentMessage(name, manifestPath) {
  return (
    `isolation harness has no component "${name}" — the component manifest ` +
    `(${manifestPath}) is missing it. Run codeyam-editor editor isolate --all, ` +
    `or restart the dev server to regenerate it.`
  );
}

// A canvas app (egui) paints its "No such component" panel where no DOM check
// can read it, so its dispatcher publishes the component the served build
// cannot draw as this global instead. Null when unset.
const SERVED_BUILD_MISSING_GLOBAL = "__codeyamIsolateMissing";

async function servedBuildMissingComponent(frame) {
  return frame
    .evaluate((name) => {
      const value = globalThis[name];
      return typeof value === "string" && value ? value : null;
    }, SERVED_BUILD_MISSING_GLOBAL)
    .catch(() => null);
}

function servedBuildMissingComponentMessage(name) {
  return (
    `the served build cannot draw component "${name}" — it was built before ` +
    `that component was isolated. Wait for the dev server to finish ` +
    `rebuilding (or run codeyam-editor editor restart-dev-server), then retry.`
  );
}

// Fail the capture when the frame is the harness placeholder. Called after the
// scenario loads and before any interaction or screenshot, so an interaction
// never runs against the placeholder and the committed PNG is never replaced.
async function assertIsolatedComponentResolved(
  frame,
  { manifestPath = componentManifestPath } = {},
) {
  const unbuilt = await servedBuildMissingComponent(frame);
  if (unbuilt !== null) {
    throw new Error(servedBuildMissingComponentMessage(unbuilt));
  }
  const missing = await isolateMissingComponent(frame);
  if (missing !== null) {
    throw new Error(isolateMissingComponentMessage(missing, manifestPath()));
  }
}

module.exports = {
  logCaptureTiming,
  defaultReadServerState,
  resolveTcpTarget,
  assertAppPortReachable,
  escapeHtmlAttribute,
  buildIframeHarness,
  HARNESS_PATH,
  resolveHarnessOrigin,
  buildHarnessUrl,
  collectContentState,
  scrollThroughDocument,
  collectVisibleTextLength,
  forceFinalVisualState,
  centerCaptureWrapper,
  collectImageStates,
  waitForImagesSettled,
  waitForAnimationsSettled,
  createNetworkTracker,
  waitForNetworkQuiet,
  waitForStablePage,
  waitForReadySignal,
  watchNavigationAfterReady,
  loadScenarioInIframe,
  loadScenarioTopLevel,
  isCanvasInteraction,
  resolveInteractionTarget,
  performInteraction,
  pixelSnapshot,
  samePixels,
  waitForPixelStable,
  waitForPredicate,
  probeCanvasDominated,
  performInteractionSequence,
  DEAD_INTERACTION_MARKER,
  describeInteractionStep,
  ISOLATE_MISSING_ATTR,
  isolateMissingComponent,
  isolateMissingComponentMessage,
  servedBuildMissingComponent,
  servedBuildMissingComponentMessage,
  assertIsolatedComponentResolved,
};

// Pixel-path script: its code and this number both feed the capture pipeline
// id. Bump the number for a change that alters pixels without changing code.
const CAPTURE_VISUAL_CONTRACT = 1;
module.exports.CAPTURE_VISUAL_CONTRACT = CAPTURE_VISUAL_CONTRACT;
