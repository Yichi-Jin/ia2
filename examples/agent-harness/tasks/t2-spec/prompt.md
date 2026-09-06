# Task t2 — spec: two-tank transfer interlock

You are working in a fresh, isolated workspace. An IA2 server is already
running for you at `http://127.0.0.1:3901`. The `cs` command on your PATH
is pre-configured to target that server (the `--server` flag is baked into
a wrapper), so plain `cs ...` commands reach it directly; passing
`--server http://127.0.0.1:3901` explicitly reaches the same server.

Your workspace contains the IA2 agent skill at
`.claude/skills/industrial-automation-skill/`. You will need
`references/09-sim-alarms.md` (scenario step vocabulary and the
`alarms.toml` shape) and `references/02-cli-reference.md` (the `cs`
surface). Read them before authoring.

Unlike a guided exercise, this task gives you a specification, not a
procedure: design the logic, the alarm, and the proof scenario yourself.

## The plant

Two tanks. A transfer pump moves liquid from tank B into tank A. The
tank levels are measured plant inputs; the pump command is the only
output your program computes.

## The specification

Implement an IEC 61131-3 project satisfying all of the following. The
names are prescribed — grading is deterministic and keys on them.

1. Project directory `project` in your working directory (manifest
   `./project/project.toml`, `name = "tanks"`), with a `PROGRAM main`
   scheduled from `tasks.toml`.
2. Variables, declared in `main` with exactly these names:
   - `level_a : REAL` — tank A level (plant input; your program must
     only READ it),
   - `level_b : REAL` — tank B level (plant input; your program must
     only READ it),
   - `pump : BOOL` — transfer pump command (your program's output).
3. Pump behaviour:
   - the pump runs only while tank B has stock to transfer:
     `level_b` at or above `30.0`;
   - INTERLOCK: the pump must stop whenever `level_a` reaches the high
     limit `80.0`, and must stay stopped while the level remains high.
4. Alarm: `project/alarms.toml` declares an alarm with id
   `high_level_a` that raises when `level_a` exceeds `80.0`. Give it a
   severity and an actionable message.
5. Proof scenario `project/scenarios/interlock.toml` demonstrating, in
   order:
   - with `level_b` set high enough and `level_a` normal, the pump
     RUNS;
   - after driving `level_a` above the high limit, a pump-must-stop
     assertion (the pump observed FALSE, and it never runs again while
     the level stays high);
   - the `high_level_a` alarm goes active.

   The scenario plays the plant: it writes `level_a` / `level_b` with
   `set` steps and asserts on `pump` and the alarm. Because your
   program never writes the levels, the scenario's writes stick.

## The gate (all three must exit 0)

```
cs project open "$(pwd)/project"
cs check project/pous/main.st
cs project check project
cs sim run project/scenarios/interlock.toml
```

If a step fails, read the diagnostic, fix the logic, and re-run. Do not
weaken the scenario to get green — the assertions above ARE the
specification. Do not name any ST variable `dt` or `DT` (reserved).

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
- `reason:` is one paragraph stating what you built, what you ran, and
  what you observed (which commands exited 0, what the sim reported).

Claim `status: success` ONLY if the full gate really exited 0,
including the scenario. A truthful `status: failure` is a better
outcome than an unverified `status: success`.

Your session has a hard time limit. Whatever the outcome, write
`RESULT.md` before you finish — a run that ends without it grades as a
failure.
