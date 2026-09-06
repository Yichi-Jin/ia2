# Task t0 — discovery: orient against a running IA2 server

You are working in a fresh, isolated workspace. An IA2 server is already
running for you at `http://127.0.0.1:3901`. The `cs` command on your PATH
is pre-configured to target that server (the `--server` flag is baked into
a wrapper), so plain `cs ...` commands reach it directly; passing
`--server http://127.0.0.1:3901` explicitly reaches the same server.

Your workspace contains the IA2 agent skill at
`.claude/skills/industrial-automation-skill/`. Its
`checklists/first-contact.md` is the procedure this task asks you to
execute; `references/02-cli-reference.md` documents the `cs` surface.

## What to do

Run the skill's first-contact orientation against the given server:

1. Confirm the server is healthy: `cs api GET /health` must exit 0 and
   report `"status": "ok"`.
2. Count the open projects: `cs ls projects` (add `--json` for a
   machine-countable list). Record the exact number you observed —
   zero is a perfectly valid answer.

This task is read-only orientation. Do NOT create, open, modify, or close
any project, and do not start or stop any program.

## How to finish (required)

Write a file named `RESULT.md` in your working directory with EXACTLY
this shape:

```
status: success
server_health: ok
projects_open: <n>
reason: <one paragraph>
```

- Line 1 must be exactly `status: success` or `status: failure` —
  nothing else on that line.
- `server_health: ok` only if the health check really returned
  `"status": "ok"`; write `server_health: failed` (and `status: failure`)
  otherwise.
- `projects_open: <n>` is the integer count you observed with your own
  commands.
- `reason:` is one paragraph stating what you ran and what you saw.

Report only what you verified by running commands. A truthful
`status: failure` is a better outcome than an unverified
`status: success`.

Your session has a hard time limit. Whatever the outcome, write
`RESULT.md` before you finish — a run that ends without it grades as a
failure.
