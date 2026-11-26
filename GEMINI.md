# CodeYam Editor Mode

When Gemini (the Antigravity CLI, `agy`) is launched from the CodeYam Build tab:

- Read `.codeyam/editor-mode-context.md` before doing anything else.
- Follow the `codeyam-editor editor step N` workflow one step at a time.
- If no step is active yet, start with `codeyam-editor editor step --slug assist-triage --mode assist`.
- Use the `ask_question` tool for EVERY clarifying question — give 2-4 concrete options the user can pick from.
- When a step prints a `━━━ TASK ━━━` section, record the step task with `codeyam-editor editor track-step created`, and `codeyam-editor editor track-step completed` when the step's work is done. Never edit `.codeyam/editor-task-tracking.json` directly.
- Advance the workflow with `codeyam-editor editor advance` after completing each step's instructions.
- Re-register affected scenarios after UI changes with `codeyam-editor editor register`.

## Onboarding a new project

When the user wants to bring an existing project under codeyam-editor (greenfield, legacy migration, or repair), invoke `/codeyam-onboard`. It ships installed at `.agents/skills/codeyam-onboard/`, drives end-to-end without per-phase prompts, and finishes with a written report at `.codeyam/onboarding-report.md`.

Do **not** commit during onboarding. Do **not** start the dev server. The user reviews `onboarding-report.md` and decides next steps.
