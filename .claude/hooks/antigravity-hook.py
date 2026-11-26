#!/usr/bin/env python3
"""
Antigravity CLI (`agy`) hook shim for editor mode.

This file is a SHIM, not an implementation. The editor's workflow enforcement
lives in the two scripts beside it:

  - editor-step-hook.py    — step tracking + step-context injection
  - editor-pretool-hook.py — PreToolUse enforcement (exit 2 = block)

Those stay the single source of truth across every build-agent provider, as
they already are for OpenCode's plugin. `.agents/hooks.json` registers this
file once per Antigravity event, passing the event name as argv[1]; the shim
translates Antigravity's stdin payload into the Claude-shaped one those scripts
parse, runs the right one, and translates its answer back.

## Antigravity's contract (antigravity.google/docs/hooks)

  stdin, every event: conversationId, workspacePaths, transcriptPath,
                      artifactDirectoryPath, modelName
  PreToolUse:  + toolCall {name, args}; answers {"decision": "allow"|"deny", "reason"}
  PostToolUse: + toolCall; answers {}
  PreInvocation / SessionStart: answer {"injectSteps": [...]} to add context
  Stop: answers {"decision": "continue", "reason"} to keep the agent going

A refusal is a stdout DOCUMENT, not an exit code — which is the translation
this shim exists for. Antigravity has no UserPromptSubmit; PreInvocation (once
per agent invocation) is where the step context goes. PostToolUse has no
injection slot, so the step hook's per-tool restriction reminder is not shown;
the next PreInvocation carries the step context regardless.

## Idle hook

`Stop` (when the turn really ends), the first `PreInvocation` and `PreToolUse`
also POST the matching trigger to the editor's `/api/idle-hook`, as Claude's
settings curl does, so the editor learns the turn state first-hand — and the
run schedule starts its next plan the moment a scheduled plan's turn ends.

## Session record

Every event also records `conversationId` and `transcriptPath` to
`.codeyam/state/antigravity-session.json`, which is how the editor learns the
id to resume with `agy --conversation <id>` (control-api
`antigravity_session.rs`). Recorded only under CODEYAM_EDITOR_ACTIVE, so a
developer's own `agy` in the same folder never overwrites it.

The shim never fails the agent: any error degrades to an allow / empty answer.
"""

import json
import os
import subprocess
import sys
import time
import urllib.request

SESSION_RECORD_REL = os.path.join(".codeyam", "state", "antigravity-session.json")
SESSION_TOKEN_REL = os.path.join(".codeyam", "session-token")
IDLE_HOOK_TIMEOUT_SECS = 3

# Antigravity tool name -> the Claude Code tool name the Python hooks match on.
# An unmapped name passes through verbatim: guessing would write wrong tracking
# state, and the hooks' allow-lists treat an unknown name conservatively.
TOOL_NAME_MAP = {
    "run_command": "Bash",
    "write_to_file": "Write",
    "replace_file_content": "Edit",
    "multi_replace_file_content": "Edit",
    "view_file": "Read",
    "list_dir": "Glob",
    "find_by_name": "Glob",
    "grep_search": "Grep",
    "search_web": "WebSearch",
    "read_url_content": "WebFetch",
    "ask_question": "AskUserQuestion",
    "invoke_subagent": "Agent",
}


def _decode(value):
    """Antigravity has been observed to double-encode string arguments
    (`"\\"path\\""`). Unwrap one level when a string parses as a JSON string;
    leave everything else alone."""
    if isinstance(value, str) and len(value) >= 2 and value[0] == '"' and value[-1] == '"':
        try:
            decoded = json.loads(value)
            if isinstance(decoded, str):
                return decoded
        except ValueError:
            pass
    return value


def to_claude_tool_input(args):
    """Layer Claude-shaped aliases over Antigravity's PascalCase arguments.

    The original keys are kept so nothing is lost; a hook that reads
    `file_path` off an object that only has `TargetFile` would see an empty
    string and wave the call through."""
    if isinstance(args, str):
        try:
            args = json.loads(args)
        except ValueError:
            args = {}
    if not isinstance(args, dict):
        args = {}
    a = {k: _decode(v) for k, v in args.items()}

    def first(*keys):
        for key in keys:
            value = a.get(key)
            if isinstance(value, str) and value:
                return value
        return None

    out = dict(a)
    for alias, keys in (
        ("command", ("CommandLine", "command")),
        ("file_path", ("TargetFile", "AbsolutePath", "FilePath", "file_path")),
        ("content", ("CodeContent", "content")),
        ("old_string", ("TargetContent", "old_string")),
        ("new_string", ("ReplacementContent", "new_string")),
        ("path", ("DirectoryPath", "SearchPath", "path")),
        ("pattern", ("Query", "Pattern", "pattern")),
        ("url", ("Url", "url")),
    ):
        value = first(*keys)
        if value is not None:
            out[alias] = value
    return out


def project_dir_from(payload):
    paths = payload.get("workspacePaths")
    if isinstance(paths, list) and paths and isinstance(paths[0], str) and paths[0]:
        return paths[0]
    return os.getcwd()


def record_session(project_dir, payload):
    if not os.environ.get("CODEYAM_EDITOR_ACTIVE"):
        return
    conversation_id = payload.get("conversationId")
    if not isinstance(conversation_id, str) or not conversation_id.strip():
        return
    record = {
        "conversationId": conversation_id,
        "transcriptPath": payload.get("transcriptPath") or None,
        "recordedAt": int(time.time()),
    }
    path = os.path.join(project_dir, SESSION_RECORD_REL)
    try:
        os.makedirs(os.path.dirname(path), exist_ok=True)
        tmp = path + ".tmp"
        with open(tmp, "w") as f:
            json.dump(record, f)
        os.replace(tmp, path)
    except OSError:
        pass


def _post_idle_hook_once(port, body, token):
    request = urllib.request.Request(
        f"http://127.0.0.1:{port}/api/idle-hook",
        data=body,
        method="POST",
        headers={"content-type": "application/json"},
    )
    if token:
        request.add_header("Authorization", f"Bearer {token}")
    try:
        with urllib.request.urlopen(request, timeout=IDLE_HOOK_TIMEOUT_SECS) as response:
            return 200 <= response.status < 300
    except (OSError, ValueError):
        return False


def _live_control_port():
    try:
        result = subprocess.run(
            ["codeyam-editor", "editor", "control-port"],
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.SubprocessError):
        return ""
    return result.stdout.strip() if result.returncode == 0 else ""


def _hook_session_id():
    """The id the editor registered this PTY's idle-hook channel under: the
    hook id, falling back to the session id. Empty when neither is set."""
    return os.environ.get("CODEYAM_EDITOR_HOOK_ID") or os.environ.get(
        "CODEYAM_EDITOR_SESSION_ID", ""
    )


def _read_session_token(project_dir):
    """The editor's bearer token, read fresh each call (a server restart
    rotates it). Empty when the file is absent or unreadable."""
    try:
        with open(os.path.join(project_dir, SESSION_TOKEN_REL)) as f:
            return f.read().strip()
    except OSError:
        return ""


def post_idle_hook(project_dir, trigger):
    """Report a turn transition to the editor's `/api/idle-hook` — the same
    request Claude's settings curl sends (`install.rs::idle_hook_command`), so
    the run schedule's next plan starts the moment this turn ends.

    Same contract as that curl: only inside an editor PTY, the hook id falling
    back to the session id, the token read fresh from `.codeyam/session-token`,
    and one retry on the live port when `CODEYAM_EDITOR_PORT` has gone stale.
    Silent on every failure — the server's TTY detector is the backstop."""
    if not os.environ.get("CODEYAM_EDITOR_ACTIVE"):
        return
    session_id = _hook_session_id()
    if not session_id:
        return
    body = json.dumps({"trigger": trigger, "sessionId": session_id}).encode()
    token = _read_session_token(project_dir)
    port = os.environ.get("CODEYAM_EDITOR_PORT", "")
    if port and _post_idle_hook_once(port, body, token):
        return
    live = _live_control_port()
    if live and live != port:
        _post_idle_hook_once(live, body, token)


def run_hook(project_dir, script, payload):
    """Run a sibling hook with `payload` on stdin. None when it or a python
    interpreter is unavailable — absence is not an error."""
    hook = os.path.join(os.path.dirname(os.path.abspath(__file__)), script)
    if not os.path.isfile(hook):
        return None
    env = dict(os.environ)
    env["CLAUDE_PROJECT_DIR"] = project_dir
    try:
        result = subprocess.run(
            [sys.executable, hook],
            input=json.dumps(payload),
            capture_output=True,
            text=True,
            cwd=project_dir,
            env=env,
            timeout=25,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if result.stderr:
        sys.stderr.write(result.stderr)
    return result


def injected_text(stdout):
    """The context a Claude hook asked to inject: `additionalContext` from a
    JSON answer, or the raw text Claude would add to context."""
    text = (stdout or "").strip()
    if not text:
        return ""
    try:
        doc = json.loads(text)
    except ValueError:
        return text
    if isinstance(doc, dict):
        specific = doc.get("hookSpecificOutput")
        if isinstance(specific, dict) and isinstance(specific.get("additionalContext"), str):
            return specific["additionalContext"].strip()
        if isinstance(doc.get("additionalContext"), str):
            return doc["additionalContext"].strip()
        return ""
    return text


def inject(text):
    if not text:
        return {}
    return {
        "injectSteps": [
            {"systemMessage": {"systemMessage": text, "metadata": {"source": "codeyam-editor"}}}
        ]
    }


def base_payload(event, payload):
    return {
        "hook_event_name": event,
        "session_id": payload.get("conversationId") or "",
        "transcript_path": payload.get("transcriptPath") or "",
        "cwd": project_dir_from(payload),
    }


def pre_tool_use(project_dir, payload):
    call = payload.get("toolCall") or {}
    name = call.get("name") or ""
    claude = base_payload("PreToolUse", payload)
    claude["tool_name"] = TOOL_NAME_MAP.get(name, name)
    claude["tool_input"] = to_claude_tool_input(call.get("args"))
    result = run_hook(project_dir, "editor-pretool-hook.py", claude)
    if result is None:
        return {"decision": "allow"}
    if result.returncode == 2:
        reason = (result.stderr or "").strip() or (
            "Blocked by the CodeYam editor workflow at the current step."
        )
        return {"decision": "deny", "reason": reason}
    try:
        doc = json.loads(result.stdout or "")
    except ValueError:
        doc = None
    specific = doc.get("hookSpecificOutput") if isinstance(doc, dict) else None
    if isinstance(specific, dict) and specific.get("permissionDecision") == "deny":
        reason = specific.get("permissionDecisionReason") or "Blocked by the CodeYam editor workflow."
        return {"decision": "deny", "reason": reason}
    return {"decision": "allow"}


def post_tool_use(project_dir, payload):
    call = payload.get("toolCall") or {}
    name = call.get("name") or ""
    claude = base_payload("PostToolUse", payload)
    claude["tool_name"] = TOOL_NAME_MAP.get(name, name)
    claude["tool_input"] = to_claude_tool_input(call.get("args"))
    run_hook(project_dir, "editor-step-hook.py", claude)
    return {}


def context_event(project_dir, payload, claude_event):
    claude = base_payload(claude_event, payload)
    if claude_event == "UserPromptSubmit":
        # Antigravity's PreInvocation carries no prompt text.
        claude["prompt"] = ""
    result = run_hook(project_dir, "editor-step-hook.py", claude)
    return inject(injected_text(result.stdout if result else ""))


def stop(project_dir, payload):
    claude = base_payload("Stop", payload)
    result = run_hook(project_dir, "editor-step-hook.py", claude)
    try:
        doc = json.loads((result.stdout if result else "") or "")
    except ValueError:
        doc = None
    if isinstance(doc, dict) and doc.get("decision") == "block" and doc.get("reason"):
        return {"decision": "continue", "reason": doc["reason"]}
    # Only a turn that really ends is reported: a `continue` keeps the agent
    # working, and reporting it would start the next scheduled plan on top.
    post_idle_hook(project_dir, "stop")
    return {}


def main():
    event = sys.argv[1] if len(sys.argv) > 1 else ""
    try:
        raw = sys.stdin.read()
        payload = json.loads(raw) if raw.strip() else {}
        if not isinstance(payload, dict):
            payload = {}
    except ValueError:
        payload = {}
    project_dir = project_dir_from(payload)
    try:
        record_session(project_dir, payload)
        if event == "PreToolUse":
            post_idle_hook(project_dir, "pre-tool-use")
            answer = pre_tool_use(project_dir, payload)
        elif event == "PostToolUse":
            answer = post_tool_use(project_dir, payload)
        elif event == "PreInvocation":
            # `invocationNum` counts model invocations within one execution;
            # only the first stands in for UserPromptSubmit, or the step
            # context would be re-injected on every step of the turn.
            if payload.get("invocationNum", 0) in (0, None):
                post_idle_hook(project_dir, "user-prompt-submit")
                answer = context_event(project_dir, payload, "UserPromptSubmit")
            else:
                answer = {}
        elif event == "SessionStart":
            answer = context_event(project_dir, payload, "SessionStart")
        elif event == "Stop":
            answer = stop(project_dir, payload)
        else:
            answer = {}
    except Exception as e:  # never fail the agent over a shim bug
        sys.stderr.write(f"antigravity-hook: {e}\n")
        answer = {"decision": "allow"} if event == "PreToolUse" else {}
    print(json.dumps(answer))


if __name__ == "__main__":
    main()
