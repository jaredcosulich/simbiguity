// Pure helpers for the engine status page: no DOM, no fetch, so they can
// be tested under `node --test`. The report shape is written by
// `cargo run -p xtask -- site-report` (xtask/src/report.rs).

/** Test statuses in the order the filter offers them. */
export const TEST_STATUSES = ["passed", "failed", "ignored"];

/** Where the page loads its report from: `?report=<path>` or the default. */
export function reportUrl(search) {
  const params = new URLSearchParams(search);
  const custom = params.get("report");
  // Only same-site relative paths, so a link cannot point the page at
  // someone else's report.
  if (custom && !/^[a-z]+:|^\/\//i.test(custom)) return custom;
  return "report.json";
}

/** `#crate=x&status=failed&q=bridge` → filter state. */
export function parseFilters(hash) {
  const params = new URLSearchParams(String(hash || "").replace(/^#/, ""));
  const status = params.get("status");
  return {
    crate: params.get("crate") || "",
    status: TEST_STATUSES.includes(status) ? status : "",
    q: params.get("q") || "",
  };
}

/** Filter state → `#...` (empty filters are omitted; all empty → ""). */
export function formatFilters({ crate, status, q }) {
  const params = new URLSearchParams();
  if (crate) params.set("crate", crate);
  if (status) params.set("status", status);
  if (q) params.set("q", q);
  const s = params.toString();
  return s ? `#${s}` : "";
}

/**
 * The hash to write after the filters change, or null when the URL
 * already says the same thing (so history is not touched needlessly).
 */
export function nextHash(state, currentHash) {
  const hash = formatFilters(state);
  const current = currentHash === "#" ? "" : currentHash || "";
  return hash === current ? null : hash;
}

/** "All 12 tests" when nothing is filtered out, else "3 of 12 tests". */
export function countLabel(shown, total) {
  return shown === total ? `All ${total} tests` : `${shown} of ${total} tests`;
}

/** The line a capability card shows above its proving tests, or null. */
export function proofSummary(tests) {
  if (!tests.length) return null;
  const failed = tests.filter((t) => t.status === "failed").length;
  if (failed) return `${failed} of ${tests.length} proving tests failing`;
  return `Proven by ${tests.length} ${tests.length === 1 ? "test" : "tests"}`;
}

/** Tests matching every active filter; `q` matches description or name. */
export function filterTests(tests, { crate, status, q }) {
  const needle = (q || "").trim().toLowerCase();
  return tests.filter((t) => {
    if (crate && t.crate !== crate) return false;
    if (status && t.status !== status) return false;
    if (!needle) return true;
    const haystack = `${t.description || ""} ${t.module}::${t.name}`.toLowerCase();
    return haystack.includes(needle);
  });
}

/** The Rust path a developer would grep for, e.g. `float::tests::clamp`. */
export function rustPath(test) {
  return test.module ? `${test.module}::${test.name}` : test.name;
}

/** GitHub link to the test's line at the exact commit, or null. */
export function sourceUrl(report, test) {
  if (!report.repository || !report.commit || !test.file) return null;
  const line = test.line ? `#L${test.line}` : "";
  return `${report.repository}/blob/${report.commit}/${test.file}${line}`;
}

/** GitHub link to the commit the report was built from, or null. */
export function commitUrl(report) {
  if (!report.repository || !report.commit) return null;
  return `${report.repository}/commit/${report.commit}`;
}

/** `2026-11-26T18:49:08+00:00` → `26 Nov 2026`. */
export function formatDate(iso) {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  return `${d.getUTCDate()} ${months[d.getUTCMonth()]} ${d.getUTCFullYear()}`;
}

/** Headline numbers for the top of the page. */
export function summarize(report) {
  const caps = report.capabilities || [];
  const count = (s) => caps.filter((c) => c.status === s).length;
  return {
    passed: report.totals?.passed ?? 0,
    failed: report.totals?.failed ?? 0,
    ignored: report.totals?.ignored ?? 0,
    tests: report.totals?.tests ?? (report.tests || []).length,
    externalDependencies: report.totals?.externalDependencies ?? 0,
    crates: (report.crates || []).length,
    capabilities: caps.length,
    proven: count("passing"),
    failing: count("failing"),
    untested: count("untested"),
  };
}

/**
 * Capabilities grouped by crate, keeping manifest order within a crate.
 * Crates with something to see lead: most live demos first, then the
 * report's (alphabetical) order.
 */
export function capabilitiesByCrate(report) {
  const groups = new Map();
  for (const cap of report.capabilities || []) {
    if (!groups.has(cap.crate)) groups.set(cap.crate, []);
    groups.get(cap.crate).push(cap);
  }
  const demos = (caps) => caps.filter((c) => c.demo).length;
  return [...groups]
    .map(([crate, capabilities], order) => ({ crate, capabilities, order }))
    .sort((a, b) => demos(b.capabilities) - demos(a.capabilities) || a.order - b.order)
    .map(({ crate, capabilities }) => ({ crate, capabilities }));
}

/** The tests that prove a capability, failures first. */
export function provingTests(report, capability) {
  const byId = new Map((report.tests || []).map((t) => [t.id, t]));
  const rank = { failed: 0, ignored: 1, passed: 2 };
  return capability.tests
    .map((id) => byId.get(id))
    .filter(Boolean)
    .sort((a, b) => rank[a.status] - rank[b.status]);
}

/** Link to a capability's demo, relative to `site/status/`. */
export function demoUrl(capability) {
  if (!capability.demo) return null;
  return `../${capability.demo.replace(/^\//, "")}`;
}

/** The same demo in `?embed` mode: just the demo, without its page header. */
export function demoEmbedUrl(capability) {
  const url = demoUrl(capability);
  if (!url) return null;
  const [path, hash] = url.split("#");
  return `${path}${path.includes("?") ? "&" : "?"}embed${hash !== undefined ? `#${hash}` : ""}`;
}

/** Human label for a capability status. */
export function capabilityLabel(status) {
  return { passing: "Proven", failing: "Failing", untested: "Untested" }[status] || status;
}
