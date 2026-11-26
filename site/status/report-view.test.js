import { describe, it } from "node:test";
import assert from "node:assert/strict";

import {
  TEST_STATUSES,
  capabilitiesByCrate,
  capabilityLabel,
  commitUrl,
  countLabel,
  demoEmbedUrl,
  demoUrl,
  filterTests,
  formatDate,
  formatFilters,
  nextHash,
  parseFilters,
  proofSummary,
  provingTests,
  reportUrl,
  rustPath,
  sourceUrl,
  summarize,
} from "./report-view.js";

const REPO = "https://github.com/example/simbiguity";
const SHA = "5d47b4a0123456789abcdef0123456789abcdef0";

function test(overrides = {}) {
  return {
    id: "simbiguity-math::float::tests::clamp",
    crate: "simbiguity-math",
    module: "float::tests",
    name: "clamp",
    status: "passed",
    description: "Clamp keeps a value inside its range.",
    file: "crates/simbiguity-math/src/float.rs",
    line: 42,
    ...overrides,
  };
}

const tests = [
  test(),
  test({
    id: "simbiguity-circuits::tests/analysis::balanced_wheatstone_bridge",
    crate: "simbiguity-circuits",
    module: "tests/analysis",
    name: "balanced_wheatstone_bridge",
    status: "failed",
    description: "A balanced Wheatstone bridge carries no current across its middle.",
  }),
  test({
    id: "simbiguity-core::rng::tests::seeded",
    crate: "simbiguity-core",
    module: "rng::tests",
    name: "seeded",
    status: "ignored",
    description: "",
  }),
];

describe("TEST_STATUSES", () => {
  // The filter offers exactly the three libtest outcomes, passed first.
  it("lists the three outcomes in filter order", () => {
    assert.deepEqual(TEST_STATUSES, ["passed", "failed", "ignored"]);
  });
});

describe("reportUrl", () => {
  // With no query the page loads the report built next to it.
  it("defaults to report.json", () => {
    assert.equal(reportUrl(""), "report.json");
  });

  // A scenario points the page at a committed fixture.
  it("accepts a same-site relative fixture path", () => {
    assert.equal(reportUrl("?report=fixtures/one-failing.json"), "fixtures/one-failing.json");
  });

  // An absolute URL could show someone else's report, so it is ignored.
  it("rejects an absolute URL", () => {
    assert.equal(reportUrl("?report=https://evil.example/report.json"), "report.json");
  });

  // A protocol-relative URL is just as foreign and is ignored too.
  it("rejects a protocol-relative URL", () => {
    assert.equal(reportUrl("?report=//evil.example/report.json"), "report.json");
  });

  // A javascript: URL is refused regardless of letter case.
  it("rejects any scheme regardless of case", () => {
    assert.equal(reportUrl("?report=JavaScript:alert"), "report.json");
  });

  // An empty report parameter falls back to the default.
  it("falls back when the parameter is empty", () => {
    assert.equal(reportUrl("?report="), "report.json");
  });
});

describe("parseFilters", () => {
  // Every filter is read from the hash.
  it("reads crate, status and search from the hash", () => {
    assert.deepEqual(parseFilters("#crate=simbiguity-circuits&status=failed&q=bridge"), {
      crate: "simbiguity-circuits",
      status: "failed",
      q: "bridge",
    });
  });

  // An empty or missing hash means no filters.
  it("returns empty filters for an empty hash", () => {
    const empty = { crate: "", status: "", q: "" };
    assert.deepEqual(parseFilters(""), empty);
    assert.deepEqual(parseFilters(undefined), empty);
  });

  // A status the filter does not offer is dropped rather than matching nothing.
  it("drops an unknown status", () => {
    assert.equal(parseFilters("#status=exploded").status, "");
  });

  // The leading hash mark is optional.
  it("accepts a hash without the leading mark", () => {
    assert.equal(parseFilters("q=spring").q, "spring");
  });

  // Encoded spaces in the search survive the round trip.
  it("decodes an encoded search", () => {
    assert.equal(parseFilters("#q=wheatstone+bridge").q, "wheatstone bridge");
  });
});

describe("formatFilters", () => {
  // No active filter means no hash at all.
  it("returns an empty string when nothing is filtered", () => {
    assert.equal(formatFilters({ crate: "", status: "", q: "" }), "");
  });

  // Only the active filters are written.
  it("omits empty filters", () => {
    assert.equal(formatFilters({ crate: "", status: "failed", q: "" }), "#status=failed");
  });

  // Every active filter is written in a stable order.
  it("writes all active filters", () => {
    assert.equal(
      formatFilters({ crate: "simbiguity-math", status: "passed", q: "clamp" }),
      "#crate=simbiguity-math&status=passed&q=clamp",
    );
  });

  // What formatFilters writes, parseFilters reads back unchanged.
  it("round-trips through parseFilters", () => {
    const state = { crate: "simbiguity-core", status: "ignored", q: "a b&c" };
    assert.deepEqual(parseFilters(formatFilters(state)), state);
  });
});

describe("nextHash", () => {
  // A changed filter produces the new hash to write.
  it("returns the new hash when the filters changed", () => {
    assert.equal(nextHash({ crate: "", status: "failed", q: "" }, ""), "#status=failed");
  });

  // An unchanged filter leaves history alone.
  it("returns null when the URL already matches", () => {
    assert.equal(nextHash({ crate: "", status: "failed", q: "" }, "#status=failed"), null);
  });

  // Clearing every filter removes the hash.
  it("returns an empty hash when the filters are cleared", () => {
    assert.equal(nextHash({ crate: "", status: "", q: "" }, "#q=bridge"), "");
  });

  // A bare hash mark already means no filters.
  it("treats a bare hash mark as no filters", () => {
    assert.equal(nextHash({ crate: "", status: "", q: "" }, "#"), null);
  });

  // Nothing to write when there is no hash and no filters.
  it("returns null with no hash and no filters", () => {
    assert.equal(nextHash({ crate: "", status: "", q: "" }, ""), null);
  });
});

describe("filterTests", () => {
  const none = { crate: "", status: "", q: "" };

  // No filters shows every test.
  it("keeps every test when nothing is filtered", () => {
    assert.equal(filterTests(tests, none).length, 3);
  });

  // The crate filter keeps one crate's tests.
  it("filters by crate", () => {
    assert.deepEqual(
      filterTests(tests, { ...none, crate: "simbiguity-math" }).map((t) => t.name),
      ["clamp"],
    );
  });

  // The status filter keeps one outcome.
  it("filters by status", () => {
    assert.deepEqual(
      filterTests(tests, { ...none, status: "failed" }).map((t) => t.name),
      ["balanced_wheatstone_bridge"],
    );
  });

  // Search matches the plain-English description, ignoring case.
  it("searches descriptions case-insensitively", () => {
    assert.equal(filterTests(tests, { ...none, q: "WHEATSTONE" }).length, 1);
  });

  // Search also matches the Rust path, so a developer can paste a module.
  it("searches the module path", () => {
    assert.deepEqual(
      filterTests(tests, { ...none, q: "rng::tests" }).map((t) => t.name),
      ["seeded"],
    );
  });

  // Surrounding whitespace in the search is ignored.
  it("trims the search", () => {
    assert.equal(filterTests(tests, { ...none, q: "  clamp  " }).length, 1);
  });

  // Filters combine: every one must match.
  it("requires every active filter to match", () => {
    assert.equal(filterTests(tests, { crate: "simbiguity-math", status: "failed", q: "" }).length, 0);
  });
});

describe("countLabel", () => {
  // Nothing filtered out reads as the full count.
  it("says All when every test is shown", () => {
    assert.equal(countLabel(12, 12), "All 12 tests");
  });

  // A filtered table says how many of the total remain.
  it("says how many of the total are shown", () => {
    assert.equal(countLabel(3, 12), "3 of 12 tests");
  });

  // A filter that matches nothing still reports the total.
  it("reports zero shown", () => {
    assert.equal(countLabel(0, 12), "0 of 12 tests");
  });
});

describe("proofSummary", () => {
  // A capability with no proving tests has no summary line.
  it("returns null with no tests", () => {
    assert.equal(proofSummary([]), null);
  });

  // One passing test reads in the singular.
  it("uses the singular for one test", () => {
    assert.equal(proofSummary([test()]), "Proven by 1 test");
  });

  // Several passing tests read in the plural.
  it("uses the plural for several tests", () => {
    assert.equal(proofSummary([test(), test()]), "Proven by 2 tests");
  });

  // Any failure leads the summary with the failing count.
  it("counts failures when any test failed", () => {
    assert.equal(
      proofSummary([test(), test({ status: "failed" })]),
      "1 of 2 proving tests failing",
    );
  });

  // An ignored test does not count as a failure.
  it("does not count ignored tests as failing", () => {
    assert.equal(proofSummary([test({ status: "ignored" })]), "Proven by 1 test");
  });
});

describe("rustPath", () => {
  // The module and name join the way Rust paths do.
  it("joins module and name", () => {
    assert.equal(rustPath(test()), "float::tests::clamp");
  });

  // A test at a binary's root has no module.
  it("returns the bare name without a module", () => {
    assert.equal(rustPath(test({ module: "" })), "clamp");
  });
});

describe("sourceUrl", () => {
  const report = { repository: REPO, commit: SHA };

  // The link points at the test's line at the exact commit.
  it("links to the file and line at the commit", () => {
    assert.equal(
      sourceUrl(report, test()),
      `${REPO}/blob/${SHA}/crates/simbiguity-math/src/float.rs#L42`,
    );
  });

  // Without a line the link points at the file.
  it("omits the line anchor when the line is unknown", () => {
    assert.equal(
      sourceUrl(report, test({ line: null })),
      `${REPO}/blob/${SHA}/crates/simbiguity-math/src/float.rs`,
    );
  });

  // No repository, commit or file means no link.
  it("returns null when anything is missing", () => {
    assert.equal(sourceUrl({ commit: SHA }, test()), null);
    assert.equal(sourceUrl({ repository: REPO }, test()), null);
    assert.equal(sourceUrl(report, test({ file: "" })), null);
  });
});

describe("commitUrl", () => {
  // The build line links to the commit on GitHub.
  it("links to the commit", () => {
    assert.equal(commitUrl({ repository: REPO, commit: SHA }), `${REPO}/commit/${SHA}`);
  });

  // A local build without a repository has no link.
  it("returns null without a repository or commit", () => {
    assert.equal(commitUrl({ commit: SHA }), null);
    assert.equal(commitUrl({ repository: REPO }), null);
  });
});

describe("formatDate", () => {
  // An ISO timestamp becomes a short UTC date.
  it("formats an ISO timestamp as a short date", () => {
    assert.equal(formatDate("2026-11-26T18:49:08+00:00"), "26 Nov 2026");
  });

  // The date is taken in UTC, not the reader's timezone.
  it("uses the UTC date", () => {
    assert.equal(formatDate("2026-01-01T02:00:00+05:00"), "31 Dec 2025");
  });

  // A malformed timestamp renders as nothing rather than Invalid Date.
  it("returns an empty string for a bad timestamp", () => {
    assert.equal(formatDate("not a date"), "");
  });
});

describe("summarize", () => {
  // The headline numbers come from the totals and the capability statuses.
  it("counts totals, crates and capability statuses", () => {
    const s = summarize({
      totals: { passed: 10, failed: 1, ignored: 2, tests: 13, externalDependencies: 0 },
      crates: [{}, {}],
      capabilities: [{ status: "passing" }, { status: "failing" }, { status: "untested" }, { status: "passing" }],
    });
    assert.deepEqual(s, {
      passed: 10,
      failed: 1,
      ignored: 2,
      tests: 13,
      externalDependencies: 0,
      crates: 2,
      capabilities: 4,
      proven: 2,
      failing: 1,
      untested: 1,
    });
  });

  // An empty report yields zeros rather than throwing.
  it("returns zeros for an empty report", () => {
    const s = summarize({});
    assert.equal(s.passed, 0);
    assert.equal(s.tests, 0);
    assert.equal(s.crates, 0);
    assert.equal(s.capabilities, 0);
  });

  // Without totals the test count falls back to the test list.
  it("counts the test list when totals are missing", () => {
    assert.equal(summarize({ tests }).tests, 3);
  });
});

describe("capabilitiesByCrate", () => {
  // Capabilities stay in manifest order within their crate.
  it("groups by crate in manifest order", () => {
    const groups = capabilitiesByCrate({
      capabilities: [
        { crate: "a", name: "a1" },
        { crate: "b", name: "b1" },
        { crate: "a", name: "a2" },
      ],
    });
    assert.deepEqual(
      groups.map((g) => [g.crate, g.capabilities.map((c) => c.name)]),
      [
        ["a", ["a1", "a2"]],
        ["b", ["b1"]],
      ],
    );
  });

  // The crate with the most live demos leads the page.
  it("puts crates with more demos first", () => {
    const groups = capabilitiesByCrate({
      capabilities: [
        { crate: "core", name: "rng" },
        { crate: "circuits", name: "ohm", demo: "demos/hello-circuit/" },
        { crate: "circuits", name: "bulb", demo: "demos/circuit-3d/" },
        { crate: "render", name: "mesh", demo: "demos/circuit-3d/" },
      ],
    });
    assert.deepEqual(groups.map((g) => g.crate), ["circuits", "render", "core"]);
  });

  // A report with no capabilities has no groups.
  it("returns no groups for no capabilities", () => {
    assert.deepEqual(capabilitiesByCrate({}), []);
  });
});

describe("provingTests", () => {
  const report = { tests };

  // A failing proof is listed before passing ones.
  it("orders failures first, then ignored, then passed", () => {
    const capability = { tests: tests.map((t) => t.id) };
    assert.deepEqual(
      provingTests(report, capability).map((t) => t.status),
      ["failed", "ignored", "passed"],
    );
  });

  // An id the report does not contain is skipped.
  it("skips ids the report does not contain", () => {
    const capability = { tests: ["nope", tests[0].id] };
    assert.deepEqual(provingTests(report, capability).map((t) => t.name), ["clamp"]);
  });

  // A capability proven by nothing has no proving tests.
  it("returns nothing for an untested capability", () => {
    assert.deepEqual(provingTests(report, { tests: [] }), []);
  });
});

describe("demoUrl", () => {
  // The demo path is made relative to the status page.
  it("makes the demo relative to the status page", () => {
    assert.equal(demoUrl({ demo: "demos/hello-circuit/" }), "../demos/hello-circuit/");
  });

  // A leading slash does not escape the site root.
  it("strips a leading slash", () => {
    assert.equal(demoUrl({ demo: "/demos/circuit-3d/" }), "../demos/circuit-3d/");
  });

  // A capability with no demo has no link.
  it("returns null without a demo", () => {
    assert.equal(demoUrl({}), null);
  });
});

describe("demoEmbedUrl", () => {
  // The embed flag is added as a query parameter.
  it("adds the embed flag", () => {
    assert.equal(demoEmbedUrl({ demo: "demos/hello-circuit/" }), "../demos/hello-circuit/?embed");
  });

  // An existing query keeps its parameters.
  it("appends to an existing query", () => {
    assert.equal(demoEmbedUrl({ demo: "demos/circuit-3d/?bulb=blown" }), "../demos/circuit-3d/?bulb=blown&embed");
  });

  // A hash stays at the end of the URL.
  it("keeps the hash after the flag", () => {
    assert.equal(demoEmbedUrl({ demo: "demos/circuit-3d/#switch" }), "../demos/circuit-3d/?embed#switch");
  });

  // A capability with no demo has no embed.
  it("returns null without a demo", () => {
    assert.equal(demoEmbedUrl({}), null);
  });
});

describe("capabilityLabel", () => {
  // Each status reads as a plain word on its pill.
  it("labels each status", () => {
    assert.equal(capabilityLabel("passing"), "Proven");
    assert.equal(capabilityLabel("failing"), "Failing");
    assert.equal(capabilityLabel("untested"), "Untested");
  });

  // An unknown status shows as itself.
  it("passes an unknown status through", () => {
    assert.equal(capabilityLabel("pending"), "pending");
  });
});
