#!/usr/bin/env python3
"""
Gemini CLI hooks for editor mode step tracking.

Reads .codeyam/editor-step.json and emits Gemini-shaped JSON output on
AfterTool / BeforeAgent / AfterAgent events; SessionStart / PreCompress
only reset the step-context dedupe marker. Step labels, restrictions,
and totals are loaded from .codeyam/cache/step-metadata.json — the same
cache the canonical Claude hook uses — so this hook never embeds the
step tables. If the cache is missing or unreadable, the hook degrades
to a label-less reminder rather than serving stale literals.
"""

import hashlib
import json
import os
import sys
from datetime import datetime, timezone


CACHE_REL_PATH = os.path.join(".codeyam", "cache", "step-metadata.json")

SUPPORTED_SCHEMA_VERSIONS = (1, 2, 3)


def _empty_mode_table():
    return {
        "total": 0,
        "labels": {},
        "descriptions": {},
        "restrictions": {},
        "slugsByStep": {},
        "codeChangeSlugs": [],
        "commitSlugs": [],
        "pushSlugs": [],
        "previewRequiredSlugs": [],
        "testEditSlugs": [],
        "testRunSlugs": [],
        "noTestSlugs": {},
    }


def _kv_pairs_to_int_dict(pairs):
    """Cache mode-table fields are JSON arrays of [step, value] pairs.
    Convert to {int(step): value} for O(1) lookup."""
    out = {}
    for entry in pairs or []:
        try:
            step_num, value = entry
            out[int(step_num)] = value
        except (TypeError, ValueError):
            continue
    return out


def _string_list(raw):
    """Normalize a JSON value to a list[str]. Non-list inputs and
    non-string members are dropped — the hook treats malformed entries
    as if they were absent rather than blocking on parse error."""
    if not isinstance(raw, list):
        return []
    return [s for s in raw if isinstance(s, str)]


def _no_test_slug_map(raw):
    """Normalize the `noTestSlugs` projection to {slug: {kind, nextTestRunSlug}}.

    The cache carries it as a list of objects; a dict keyed by slug is what
    the hook actually looks up. Malformed members are dropped rather than
    raised on — a slug with no entry reads as "word the block the old way",
    which is the same status-quo degrade an absent projection gets."""
    out = {}
    if not isinstance(raw, list):
        return out
    for entry in raw:
        if not isinstance(entry, dict):
            continue
        slug = entry.get("slug")
        kind = entry.get("kind")
        if not isinstance(slug, str) or not isinstance(kind, str):
            continue
        next_slug = entry.get("nextTestRunSlug")
        out[slug] = {
            "kind": kind,
            "nextTestRunSlug": next_slug if isinstance(next_slug, str) else None,
        }
    return out


def load_step_metadata(project_dir):
    """Load the per-mode step metadata cache. Returns
    {<mode name>: <mode_table>} — one entry per mode the cache
    actually carries — where each mode_table carries label/
    description/restriction dicts plus slug-keyed capability arrays.

    The mode set is ENUMERATED FROM THE DOCUMENT, not hard-coded. The
    Rust emitter ships `assist` and `design` tables alongside `ui` and
    `backend`, and a hard-coded `("ui", "backend")` pair dropped them on
    the floor — which is what made an assist session load the UI table,
    be labeled `UI Flow Step 1/28`, and have its commit refused at
    `assist-wrap` because the UI table's `commitSlugs` is `["commit"]`.
    Reading the keys off the document means the next mode the emitter
    ships is picked up with no edit here.

    Falls back to empty tables on any read/parse error so the hook
    keeps firing (just without labels and with no slug allowlists)
    when the cache is missing or a different binary version wrote a
    shape we don't understand. Empty capability arrays mean the
    pretool hook treats every slug as ungated — degrade to "allow"
    rather than brick the session on cache skew."""
    cache_path = os.path.join(project_dir, CACHE_REL_PATH)
    fallback = {"ui": _empty_mode_table(), "backend": _empty_mode_table()}
    try:
        with open(cache_path, "r") as f:
            raw = json.load(f)
    except (IOError, json.JSONDecodeError):
        return fallback
    if not isinstance(raw, dict) or raw.get("schemaVersion") not in SUPPORTED_SCHEMA_VERSIONS:
        return fallback
    out = {}
    for mode_key, mode_raw in raw.items():
        # `schemaVersion` is the one non-mode top-level key. Guarding on
        # the VALUE's type rather than a name denylist means any future
        # scalar sibling is skipped too, instead of being normalized into
        # a bogus mode table.
        if mode_key == "schemaVersion" or not isinstance(mode_raw, dict):
            continue
        out[mode_key] = {
            "total": int(mode_raw.get("total", 0) or 0),
            "labels": _kv_pairs_to_int_dict(mode_raw.get("labels", [])),
            "descriptions": _kv_pairs_to_int_dict(mode_raw.get("descriptions", [])),
            "restrictions": _kv_pairs_to_int_dict(mode_raw.get("restrictions", [])),
            "slugsByStep": _kv_pairs_to_int_dict(mode_raw.get("slugsByStep", [])),
            "codeChangeSlugs": _string_list(mode_raw.get("codeChangeSlugs")),
            "commitSlugs": _string_list(mode_raw.get("commitSlugs")),
            "pushSlugs": _string_list(mode_raw.get("pushSlugs")),
            "previewRequiredSlugs": _string_list(mode_raw.get("previewRequiredSlugs")),
            # Additive within v3, like `noTestSlugs`. Absent → empty list →
            # no slug admits test-only edits, which is the pre-existing
            # refusal, never a production edit let through.
            "testEditSlugs": _string_list(mode_raw.get("testEditSlugs")),
            # v3 projection. Absent in a v1/v2 cache → empty list → the
            # pretool hook treats test runs as ungated (degrade to "allow"),
            # never "block every test run".
            "testRunSlugs": _string_list(mode_raw.get("testRunSlugs")),
            # Companion to `testRunSlugs`, added additively WITHIN v3 (a
            # version bump would make this closed `SUPPORTED_SCHEMA_VERSIONS`
            # tuple reject a newer cache outright and drop every gate).
            # Absent → empty map → the pretool hook falls back to its
            # original pre-Demo wording.
            "noTestSlugs": _no_test_slug_map(mode_raw.get("noTestSlugs")),
        }
    return out


# How each shipped mode names its flow in the banner the user reads every
# turn. `ui` and `backend` are builds and say "Flow"; `assist` and `design`
# are not, and saying "Flow" for them would re-assert the very thing the
# tracks exist to avoid.
_MODE_DISPLAY_PREFIXES = {
    "ui": "UI Flow",
    "backend": "Backend Flow",
    "assist": "Assist",
    "design": "Design Round",
}


# The modes that are NOT builds. `assist` triages a non-build request and
# `design` explores mockups; neither is walking a build to completion, so
# build-flow framing ("do not skip ahead") is a false statement about the
# session rather than a useful guardrail.
#
# Enumerated as the NON-build set, not the build set, so the default for an
# unrecognized mode is "build" — see `mode_is_build`.
_NON_BUILD_MODES = frozenset(("assist", "design"))


def mode_is_build(mode):
    """True when `mode` walks a build flow to completion.

    Unknown modes count as builds. That is the conservative direction: the
    guardrail copy is kept rather than dropped for a mode this hook copy
    has never heard of, so a newer emitter can never silently relax a real
    build flow's framing against an older shipped hook."""
    if not isinstance(mode, str) or not mode.strip():
        return True
    return mode not in _NON_BUILD_MODES


def mode_display_prefix(mode):
    """The banner prefix for `mode` — `Assist` for an assist session,
    `Backend Flow` for a backend build, and so on.

    An unrecognized mode renders as its own title-cased name rather than
    defaulting to `UI Flow`. The default was a lie the user read on every
    prompt: an assist session announced itself as `UI Flow Step 1/28
    (Plan)` and then told the agent "Do NOT skip ahead" on a track that
    has four steps and no plan. A mode the emitter ships tomorrow should
    misspell its own name at worst, never claim to be a different
    workflow."""
    if not isinstance(mode, str) or not mode.strip():
        return _MODE_DISPLAY_PREFIXES["ui"]
    known = _MODE_DISPLAY_PREFIXES.get(mode)
    if known:
        return known
    return mode.replace("-", " ").replace("_", " ").title()


def resolve_mode(state, metadata):
    """Return the workflow mode name and the matching mode table.

    Looks the mode up BY NAME. A mode that is named but absent from the
    cache returns an empty table, not the UI one — substituting UI does
    not degrade to "no gating", it silently applies the UI build's 28
    steps, restrictions, and `commitSlugs` to a session that is not a UI
    build. A missing `mode` still resolves to `ui`, the legacy shape."""
    mode = state.get("mode", "ui") or "ui"
    table = metadata.get(mode)
    if isinstance(table, dict):
        return mode, table
    return mode, _empty_mode_table()


def label_signals_user_feedback_phase(label):
    """Return True when the current step is one where late user
    feedback ("can you change X?") commonly arrives — and the agent
    should re-register/re-capture rather than refusing.

    Match keys on the label substring so a manifest reorder doesn't
    drift this gate. Today the slugs that produce these labels are
    `present-live` / `ui-present` / `backend-confirm` / `backend-present`
    (presentation phases), `reconcile` (glossary reconcile), and
    `finalize` (last gate before journal/commit)."""
    if not label:
        return False
    needle = label.strip().lower()
    return any(
        marker in needle
        for marker in ("present", "reconcile", "finalize")
    )


def label_signals_pre_commit_warning(label):
    """Return True when an uncommitted-changes warning is appropriate
    at this step. Fires on labels emitted by the presentation slugs
    (`present-live` / `ui-present` / `backend-confirm` / `backend-present`,
    where the user is seeing the work) and `finalize` (last gate before
    journal/commit)."""
    if not label:
        return False
    needle = label.strip().lower()
    return needle in ("present", "demo", "finalize")


def log_event(project_dir, event, data=None):
    """Append a JSONL entry to .codeyam/logs/editor-log.jsonl."""
    try:
        logs_dir = os.path.join(project_dir, ".codeyam", "logs")
        os.makedirs(logs_dir, exist_ok=True)
        log_path = os.path.join(logs_dir, "editor-log.jsonl")
        entry = {"ts": datetime.now(timezone.utc).isoformat(), "event": event}
        if data:
            entry.update(data)
        with open(log_path, "a") as f:
            f.write(json.dumps(entry) + "\n")
    except Exception:
        pass


# Marker recording what the step context last went out for, so the unchanged
# copies stop. Ported from the Claude hook, which keys the same file the same
# way — the two hooks never run in one session, so one shape serves both.
# BeforeAgent used to inline the whole editor-mode-context.md on EVERY prompt,
# and AfterTool re-sent the step restriction after EVERY shell/write/replace
# call, all identical until the step changed. The step directives and
# restriction are keyed on (session, feature, step, label) and re-fire only
# when that key moves. The editor-mode-context.md block has its own key,
# (session, digest of the file's content): nothing in it depends on the step,
# so it goes out once per session and again only when the file is genuinely
# rewritten. SessionStart and PreCompress delete the marker, so a new session,
# a resume, a /clear and a compression (which summarizes the earlier copies
# away) re-emit both on the next event.
#
# A payload without a session_id is still deduped: the key's session part is
# simply empty, so it falls back to (feature, step, label).
_STEP_CONTEXT_MARKER_REL = os.path.join(".codeyam", "state", "step-context-injected.json")

# The restriction is still re-sent every Nth tool call within one step, so a
# long step keeps it in recent view without paying for it on every call.
STEP_REMINDER_EVERY_N_CALLS = 25


def _step_injection_key(session_id, feature, step, label):
    return f"{session_id or ''}|{feature}|{step}|{label}"


def _context_injection_key(session_id, context_text):
    digest = hashlib.sha256(context_text.encode("utf-8")).hexdigest()[:16]
    return f"{session_id or ''}|{digest}"


def _read_step_context_marker(project_dir):
    try:
        with open(os.path.join(project_dir, _STEP_CONTEXT_MARKER_REL), "r") as f:
            marker = json.load(f)
        return marker if isinstance(marker, dict) else {}
    except (IOError, ValueError):
        return {}


def _write_step_context_marker(project_dir, marker):
    marker_path = os.path.join(project_dir, _STEP_CONTEXT_MARKER_REL)
    try:
        os.makedirs(os.path.dirname(marker_path), exist_ok=True)
        with open(marker_path, "w") as f:
            json.dump(marker, f)
    except Exception:
        pass


def _clear_step_context_marker(project_dir):
    try:
        os.remove(os.path.join(project_dir, _STEP_CONTEXT_MARKER_REL))
    except OSError:
        pass


def _step_context_due(project_dir, key):
    """True when the full step context has not yet gone out for `key`, and
    records that it now has. An unwritable marker re-reads as absent, so the
    failure mode is the old every-prompt injection, never silence."""
    marker = _read_step_context_marker(project_dir)
    if marker.get("promptKey") == key:
        return False
    marker["promptKey"] = key
    _write_step_context_marker(project_dir, marker)
    return True


def _editor_context_due(project_dir, key):
    """True when the editor-mode-context block has not yet gone out for `key`
    (session + content digest), and records that it now has. Fails toward
    re-injection the same way `_step_context_due` does."""
    marker = _read_step_context_marker(project_dir)
    if marker.get("contextKey") == key:
        return False
    marker["contextKey"] = key
    _write_step_context_marker(project_dir, marker)
    return True


def _editor_context_lines(project_dir, session_id):
    """The `<editor-mode-context>` block as prompt lines when it is due for this
    session and content, else `[]`. A missing or unreadable file yields `[]`."""
    context_path = os.path.join(project_dir, ".codeyam", "editor-mode-context.md")
    try:
        with open(context_path, "r") as f:
            context_text = f.read().strip()
    except Exception:
        return []
    if not _editor_context_due(
        project_dir, _context_injection_key(session_id, context_text)
    ):
        return []
    return ["\n<editor-mode-context>", context_text, "</editor-mode-context>\n"]


def _step_reminder_due(project_dir, key):
    """True on the first reminder-bearing tool call for `key` and on every
    STEP_REMINDER_EVERY_N_CALLS-th call after it; counts this call either way."""
    marker = _read_step_context_marker(project_dir)
    calls = marker.get("toolCalls", 0) if marker.get("toolKey") == key else 0
    if not isinstance(calls, int):
        calls = 0
    marker["toolKey"] = key
    marker["toolCalls"] = calls + 1
    _write_step_context_marker(project_dir, marker)
    return calls % STEP_REMINDER_EVERY_N_CALLS == 0


def detect_event():
    """Detect Gemini hook event from stdin."""
    try:
        raw = sys.stdin.read()
        if not raw.strip():
            return "unknown", {}
        data = json.loads(raw)
        event_name = data.get("hook_event_name")
        if event_name == "AfterTool":
            return "after_tool", data
        elif event_name == "BeforeAgent":
            return "before_agent", data
        elif event_name == "AfterAgent":
            return "after_agent", data
        elif event_name == "SessionStart":
            return "session_start", data
        elif event_name == "PreCompress":
            return "pre_compress", data
        return "unknown", data
    except Exception:
        return "unknown", {}


def main():
    # Only run in editor Build sessions
    if not os.environ.get("CODEYAM_EDITOR_ACTIVE"):
        print(json.dumps({}))
        return

    # Gemini provides 'cwd' in the payload
    event_type, event_data = detect_event()
    project_dir = event_data.get("cwd", os.getcwd())
    state_path = os.path.join(project_dir, ".codeyam", "editor-step.json")

    # A new session, a resume, a /clear and a compression all leave the agent
    # without the step context it was last given, so the next prompt and the
    # next tool call re-emit it. Cleared whether or not a cycle is in flight:
    # one may start before the next prompt.
    if event_type in ("session_start", "pre_compress"):
        _clear_step_context_marker(project_dir)
        print(json.dumps({}))
        return

    if not os.path.exists(state_path):
        # No editor-step.json — a stateless session entry. Mirrors the Claude
        # hook's entry nudge: name triage as the entry rather than leaving the
        # routing to a self-judged build-vs-not call. `before_agent` is this
        # hook's analog of UserPromptSubmit; unlike the Claude payload it
        # carries no prompt text, so there is no slash-command to guard on.
        if event_type == "before_agent":
            print(json.dumps({
                "additionalContext": (
                    "Editor Mode — no cycle in flight. Enter this request at triage, not "
                    "at the build flow: run "
                    "`codeyam-editor editor step --slug assist-triage --mode assist`. "
                    "Triage classifies the request against a written rubric (and defaults "
                    "to `build` when ambiguous), then hands off to the build flow itself "
                    "when it really is one — so do not judge build-vs-not yourself here."
                )
            }))
            return
        print(json.dumps({}))
        return

    try:
        with open(state_path, "r") as f:
            state = json.load(f)
    except (json.JSONDecodeError, IOError):
        print(json.dumps({}))
        return

    metadata = load_step_metadata(project_dir)
    step = state.get("step")
    feature = state.get("feature", "")
    mode, mode_table = resolve_mode(state, metadata)
    total_steps = mode_table["total"]
    step_labels = mode_table["labels"]
    step_restrictions = mode_table["restrictions"]
    label = step_labels.get(step, "Unknown")
    mode_prefix = mode_display_prefix(mode)

    if not step:
        print(json.dumps({}))
        return

    # ── Task tracking ──────────────────────────────────────────────────
    task_tracking_path = os.path.join(project_dir, ".codeyam", "editor-task-tracking.json")

    output_json = {
        "decision": "allow"
    }

    if event_type == "after_tool":
        tool_name = event_data.get("tool_name", "")
        tool_input = event_data.get("tool_input", {}) or {}

        if tool_name == "write_todos":
            todos = tool_input.get("todos", [])
            expected_prefix = f"Complete codeyam editor step {step}"

            tracking_updated = False
            task_created = False
            task_completed = False

            for t in todos:
                desc = t.get("description", "")
                status = t.get("status", "")
                if desc.startswith(expected_prefix):
                    if status == "in_progress":
                        task_created = True
                        tracking_updated = True
                    elif status == "completed":
                        task_created = True
                        task_completed = True
                        tracking_updated = True

            if tracking_updated:
                try:
                    tracking = {}
                    if os.path.exists(task_tracking_path):
                        with open(task_tracking_path, "r") as f:
                            tracking = json.load(f)

                    tracking["step"] = step
                    if task_created:
                        tracking["taskCreated"] = True
                    if task_completed:
                        tracking["taskCompleted"] = True
                    else:
                        tracking.setdefault("taskCompleted", False)

                    with open(task_tracking_path, "w") as f:
                        json.dump(tracking, f)
                except Exception as e:
                    sys.stderr.write(f"Error updating task tracking: {e}\n")

        # Reminder display for AfterTool (only for significant tools)
        if tool_name in ("run_shell_command", "write_file", "replace"):
            restriction = step_restrictions.get(step, "")
            if restriction and _step_reminder_due(
                project_dir,
                _step_injection_key(event_data.get("session_id"), feature, step, label),
            ):
                output_json["systemMessage"] = f"[Step {step}: {label}] {restriction}"

    # Log the hook firing
    log_data = {"step": step, "label": label, "feature": feature, "hook": event_type}
    if event_type == "after_tool":
        log_data["tool"] = event_data.get("tool_name", "")
    log_event(project_dir, "hook", log_data)

    restriction = step_restrictions.get(step, "")
    # In-cycle, the next step is simply the next step. At the END of a cycle
    # the next USER REQUEST is not necessarily a build, so route it through
    # triage rather than booting the build flow: "deploy it and give me the
    # live URL" is a config/hosting request that a `step 1` fallback drags
    # through the whole build workflow. `assist-triage` classifies it and
    # hands off to the build flow itself when it really is one.
    next_cmd = (
        f"codeyam-editor editor step {step + 1}"
        if step < total_steps
        else "codeyam-editor editor step --slug assist-triage --mode assist"
    )

    # ── BeforeAgent: inject step context ──────────────────────────
    if event_type == "before_agent":
        lines = [
            f'Editor Mode — {mode_prefix} Step {step}/{total_steps} ({label}): "{feature}"',
        ]

        # The step directives only change when the step does, so a later
        # prompt in the same step gets the header line alone.
        directive_due = _step_context_due(
            project_dir,
            _step_injection_key(event_data.get("session_id"), feature, step, label),
        )

        # Inject editor-mode-context.md. Its content never depends on the
        # step, so it is keyed on the content itself and goes out once per
        # session, not once per step.
        lines.extend(_editor_context_lines(project_dir, event_data.get("session_id")))

        # Include active scenario context
        active_scenario_path = os.path.join(project_dir, ".codeyam", "active-scenario.json")
        try:
            with open(active_scenario_path, "r") as f:
                active = json.load(f)
            scenario_name = (
                active.get("scenarioName")
                or active.get("scenarioSlug", "").replace("_", " ")
            )
            if scenario_name:
                lines.append(
                    f'The user is currently viewing scenario: "{scenario_name}". '
                    "Assume any feedback refers to this scenario unless they say otherwise."
                )
        except (IOError, json.JSONDecodeError):
            pass

        if label_signals_user_feedback_phase(label):
            lines.append(
                "If the user is requesting changes (even indirectly), "
                "make the changes, re-register affected scenarios, and update the journal. "
                "Then continue from the current step."
            )
        elif not directive_due:
            pass
        elif mode_is_build(mode):
            lines.append(
                f"You are on step {step}. Follow the `codeyam-editor editor` workflow. "
                f"Do NOT skip ahead or make changes outside the current step."
            )
        else:
            # A non-build track (assist, design) is not walking a build to
            # completion, so "do not skip ahead" asserts something untrue
            # about the session on every single prompt — the exact framing
            # these tracks exist to avoid. Point at the current step without
            # claiming the session is mid-build.
            lines.append(
                f"You are on step {step} of the `{mode}` track. "
                f"Follow the `codeyam-editor editor` steps for this track."
            )
        if restriction and directive_due:
            lines.append(restriction)

        output_json["hookSpecificOutput"] = {
            "additionalContext": "\n".join(lines)
        }

    # ── AfterAgent: step banner, restriction, next command ───────────────────
    # The numbered per-step progress tracker this used to render was dropped
    # in 31fb63e24 — the UI is the sole presentation layer for progress.
    if event_type == "after_agent":
        lines = []
        if feature and feature != "No feature":
            lines.append(f'Editor Mode — {mode_prefix} Step {step}/{total_steps} ({label}): "{feature}"')
        
        if restriction:
            lines.append(restriction)

        if next_cmd:
            lines.append(f"When this step is complete, run: {next_cmd}")

        if lines:
            output_json["systemMessage"] = "\n".join(lines)

    # Output final JSON
    print(json.dumps(output_json))

if __name__ == "__main__":
    main()
