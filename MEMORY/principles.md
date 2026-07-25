# Design principles — read this first

These are the **non-negotiable** axes every design decision in this
codebase is judged on. If a proposal trades one of these for cleverness,
features, or "industry standard alignment", default to **reject**.

## 1. Simplicity is the headline feature

This is **the primary product positioning**, not a footnote. The whole
reason `ia2` exists in a field already crowded with Codesys,
TwinCAT, Step 7, Studio 5000, IEC 61131-3 plugins, etc. is that all of
those have decades of accreted complexity. Engineers, students, agents,
and SDE/SRE crossovers shouldn't have to learn a 5000-page reference
manual to do anything useful. Every screen, file format, button, and
endpoint should answer "what is this for?" within 5 seconds.

**Concretely**:
- One concept per screen. Don't combine "edit POU" + "schedule POU" + "deploy" into one mega-panel.
- One canonical text representation per artefact. No proprietary binary blobs.
- Defaults that work without configuration. Empty config = sensible behaviour.
- No "advanced settings" submenus. If a setting is too advanced for the main UI, it probably isn't worth keeping.

## 2. Cognitive load — keep it low

The user (or agent) should be able to load the whole product into working
memory. If you can't draw the architecture on a single whiteboard in 60
seconds, the architecture is wrong.

**Concretely**:
- Vocabulary stays small. We have POU / Device / Edge / Task. Don't introduce "Workgroup", "Project Variant", "Compilation Profile" etc.
- One name per thing. Don't call it `app` here and `pou` there.
- Compose, don't subclass. Two simple FBs > one polymorphic super-FB with config flags.
- File on disk = file in tree = file in editor. No virtual abstraction layers.

## 3. Smooth learning curve

A first-time user should be running a "blinking variable in Monitor"
within 60 seconds of opening the IDE. A first-time agent should be able
to author a working PROGRAM by reading the API catalogue (no extra
human-oriented onboarding doc required). At every step the next action
should be obvious.

**Concretely**:
- Inline hints over modal tutorials. ("Bind a PROGRAM to a task" → the "Schedule" button next to the editor.)
- Discoverable affordances. Selected POU should make the Run button mean "run this".
- Same gesture, same effect. Run button always says what it'll do.
- Examples ship with the product and STAY TRUE: `examples/` holds real, runnable projects (`sim_smoke` is the sim-scenario reference; `eg_gear_incycle`, `nx6_modbus`, `supervisory-demo` cover motion / RTU / DCS-supervision). A doc that names an example that doesn't exist is a bug.

## 4. Agent-friendly is co-equal with human-friendly

**Agents are first-class users**. We expect Claude Code, Codex, Cursor,
and future agents to drive this IDE without ever opening the GUI. This
isn't a future ambition — it's the design pivot that distinguishes us
from every existing PLC vendor's tooling, which is GUI-only.

**Concretely**:
- **API-first**: every feature reachable via REST. GUI is a thin client over the same endpoints. If a feature works in the GUI but isn't in `/api/*`, that's a bug.
- **Text-first storage**: POU sources (and future graphical languages) live in human-readable text/JSON on disk. No PLC binary project files. Grep / git diff / `cat` must work.
- **Self-describing types**: `ts-rs` exports every wire type so agents (and the IDE) can type-check requests. There is exactly one schema source of truth (the Rust struct).
- **Deterministic state**: same inputs → same outputs. No hidden mutable state in tooling that an agent can't observe.
- **Stable identifiers**: an agent that learned "POU `polymer_cstr` is in `pous/polymer_cstr.st`" yesterday must find it in the same place today.

### 4a. CLI is the headline agent interface

API-first is necessary but not sufficient. **The primary agent surface
is the `cs` command-line binary**, not the HTTP API.

Agents work like developers: read file, write file, run command, look
at stdout. Every workflow that goes through an HTTP endpoint is one
the agent must specifically learn (which URL? which Content-Type? is
the server running? which project is "open"?). Every workflow that
goes through a CLI feels native — `cs check pou.ld.json` slots into
the same mental model as `tsc --noEmit` or `cargo build`. CLI is also
the only path that works offline, in CI, in pre-commit, and inside
batch refactoring scripts.

**Rules**:
- **Meta-primitives over noun×verb sprawl.** The CLI stays bash-sized:
  ONE generic resource quartet (`cs ls/get/set/rm <path>`, where the
  path mirrors the on-disk/API layout) plus ONE raw escape hatch
  (`cs api METHOD /api/...`) cover every resource — including ones
  that don't exist yet. Before adding a `cs` subcommand, prove the
  quartet and `cs api` can't express it; only actions with real domain
  semantics (safety, exit codes, type-aware encoding: `check`, `run`,
  `runtime force/write`, `deploy`, `sim run`, `hmi op/generate`,
  `agent run`) earn a verb. In review, count new leaf subcommands as
  a cost. This is what makes "everything via CLI" a THEOREM instead
  of a treadmill: a new server resource is CLI-covered by construction.
- **One generic proxy IS a primitive; N specialized wrappers are not.**
  Online operations (live values, debug control, attach) are fine to
  reach through the CLI because they all flow through the same two
  code paths (quartet + `api`), not a hand-rolled wrapper each.
- **Error truthfulness is the contract.** The server writes actionable
  error bodies; the CLI surfaces them VERBATIM on stderr and maps
  status onto exit codes: `0` success / `1` problems in the user's
  content (diagnostics, failed probe/deploy/sim) / `2` bad request —
  usage errors and HTTP 4xx / `≥3` infrastructure. An agent must never
  need to guess why a call failed.
- **`--help` text is written FOR THE AGENT**. Say when to use the tool,
  when NOT to, what to call next. Style reference:
  `vendor/ironplc/compiler/mcp/src/server.rs`.
- **Global flags, global behaviour**: `--json`, `--server`, `--project`
  are top-level flags honored by every command — no per-command
  re-implementation, no command that silently drops one.
- **No MCP server (yet)**. MCP is a wire format with a specific
  protocol; CLI is universal (and cheaper in agent context tokens).
  Future work can wrap our CLI as MCP if some agent platform demands
  it — doing it the other way around would be awkward.

## 5. Truthfulness everywhere; proof before hardware

Two doctrines that started in the HMI ("Stop unconfirmed", "no e-stop
cosplay") now apply to the whole system:

- **Every layer tells the truth about outcomes.** Deploy fails when the
  service didn't restart, when the tar stream broke, or when the
  version stamp is missing — never "success with a footnote". The CLI
  surfaces server error bodies verbatim and exit-codes honestly. Docs
  that describe unimplemented behaviour (or deny implemented behaviour)
  are release blockers, same as failing tests: `docs/api.md` coverage
  is test-enforced, and command examples in the skill must exist.
- **Behaviour ships with proof.** The loop is generate → `cs check` →
  `cs project check` → **`cs sim run <scenario>`** → deploy. Logic
  with dynamics (fills, sequences, interlocks, alarm conditions)
  carries a scenario in `scenarios/`; alarms are declared in
  `alarms.toml` next to the logic that can trip them. "It compiles"
  is not "it works" — an agent that can't demonstrate behaviour in sim
  has not finished the task.

## What this rules out

Anti-patterns to refuse — refer to these by name in code review:

- **Codesys-clone-itis**: implementing a feature because Codesys has it, when nobody asked for it and it adds three new concepts.
- **Multi-config syndrome**: every feature getting its own `.toml` / `.yaml` / "profile" — config sprawl by accumulation.
- **GUI-only features**: anything authored by mouse drag that has no REST equivalent.
- **Magic project files**: opaque binary blobs only the IDE itself can read.
- **Hidden state**: caches, locks, daemons that a user (or agent) can't observe via the API.
- **Tutorial dependency**: a feature that needs a "getting started" doc to be usable at all.

## When in doubt

Ask: "would a curious engineer who has never used a PLC understand this
in 30 seconds?" and "would an agent reading the OpenAPI schema know how
to drive this without any extra explanation?" If either answer is no,
simplify.

These principles override individual feature requests when they
conflict. They were added on 2026-05-15 after several conversations
where we caught ourselves drifting toward feature-by-feature parity with
Codesys; treat any drift in that direction as a regression on the
project's central proposition.
