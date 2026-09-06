# Task t3 — debug the pumphouse project until its scenario passes

You are working against a live IA2 server at `http://127.0.0.1:3901`.
The `cs` command on your PATH already carries that server URL, so plain
`cs ...` calls are enough; the explicit form is
`cs --server http://127.0.0.1:3901 ...`.

Your working directory contains `project/` — an IA2 project named
`pumphouse`: a transfer pump that runs on operator request and must be
cut out when the receiving tank level reaches its high limit. The
project's simulation scenario currently FAILS.

Consult the industrial-automation skill in
`.claude/skills/industrial-automation-skill/` for the `cs` workflow
(orientation, checking, simulation, debugging). Then:

1. Open the project on the server: `cs project open "$(pwd)/project"`.
2. Reproduce the failure: `cs sim run project/scenarios/pump_cutout.toml`.
3. Diagnose the root cause, fix the control logic, and re-run the
   scenario until it passes (`cs sim run` exit code 0).
4. Confirm the project still compiles: `cs project check project`.

Simulation only — do not add devices or any hardware-facing
configuration.

## Immutability rule

Everything in `project/` EXCEPT `pous/pump_control.st` is the fixed
specification for this exercise: do not edit, rename, delete, or ADD
any other file under it. The graded run verifies the exact content and
the exact file set — the only file you may change is the POU itself.

## Result contract

When you are done — whichever way it went — finish by writing
`RESULT.md` in your working directory. Its first line must be exactly
`status: success` or `status: failure` (claim success only if you have
a green `cs sim run` of the required scenario to show). From the second
line on, write `reason: <one paragraph>` honestly reporting what you
did and what the outcome was.

For this task, the `reason:` paragraph must also name the file that
contained the defect and state the root cause in one sentence.

Your session has a hard time limit. Whatever the outcome, write
`RESULT.md` before you finish — a run that ends without it grades as a
failure.
