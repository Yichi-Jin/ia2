# Task t1 — guided: build and prove the `blinker` project

You are working in a fresh, isolated workspace. An IA2 server is already
running for you at `http://127.0.0.1:3901`. The `cs` command on your PATH
is pre-configured to target that server (the `--server` flag is baked into
a wrapper), so plain `cs ...` commands reach it directly; passing
`--server http://127.0.0.1:3901` explicitly reaches the same server.

Your workspace contains the IA2 agent skill at
`.claude/skills/industrial-automation-skill/`. Consult
`references/02-cli-reference.md` for the `cs` surface and
`references/09-sim-alarms.md` for the scenario step vocabulary. The
repository reference example for project shape is described there; this
prompt is self-contained.

Follow this procedure exactly. The names below are prescribed — grading
is deterministic and keys on them.

## Step 1 — create the project files

Create a project directory named `project` in your working directory
(so the manifest is `./project/project.toml`), containing:

- `project/project.toml` — manifest with `name = "blinker"` (and a
  `version`).
- `project/tasks.toml` — one task with `interval_ms = 50` scheduling a
  PROGRAM instance `main` running program `main` (same `[[tasks]]` /
  `[[programs]]` shape as any IA2 project).
- `project/pous/main.st` — `PROGRAM main` declaring exactly:
  - `lamp : BOOL` — the output this task is about,
  - an INT scan counter of your choosing (do NOT name any variable
    `dt` or `DT` — it is reserved and fails compilation).

  Behaviour: increment the counter every scan; every 10 scans reset it
  and toggle the lamp (`lamp := NOT lamp;`). At 50 ms per scan the lamp
  then holds each state for 500 ms — comfortably longer than the
  scenario poll interval (~100 ms), so assertions observe every state.
- `project/scenarios/toggle.toml` — a scenario asserting BOTH
  transitions of `lamp`. The lamp starts FALSE, so use three `expect`
  steps in order: `lamp` `is_true` (rising edge happened), then
  `is_false` (falling edge happened), then `is_true` again (it keeps
  toggling). Give each a `within_ms` of a few seconds. One key per
  `[[steps]]` entry; ops `is_true`/`is_false` take no `value`.

## Step 2 — open the project on the server

```
cs project open "$(pwd)/project"
```

## Step 3 — run the full gate (all three must exit 0)

```
cs check project/pous/main.st
cs project check project
cs sim run project/scenarios/toggle.toml
```

If any step fails, read the diagnostic (it names the step, the deadline,
and the last observed value), fix your files, and re-run. Do not weaken
an assertion just to get green — the scenario above is the specification.

This task is simulation-only: create no `devices/` and no `edges/`
entries, and do not deploy anywhere.

## How to finish (required)

Write a file named `RESULT.md` in your working directory with EXACTLY
this shape:

```
status: success
reason: <one paragraph>
```

- Line 1 must be exactly `status: success` or `status: failure` —
  nothing else on that line.
- `reason:` is one paragraph stating what you ran and what you observed
  (which commands exited 0, what the sim reported).

Claim `status: success` ONLY if all three gate commands really exited 0.
A truthful `status: failure` is a better outcome than an unverified
`status: success`.

Your session has a hard time limit. Whatever the outcome, write
`RESULT.md` before you finish — a run that ends without it grades as a
failure.
