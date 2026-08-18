# Simulation, alarms, and history

These three close the loop that matters most for an agent: after you
WRITE control logic, you can PROVE it behaves (sim), declare what
counts as abnormal (alarms), and answer "what happened at 03:00"
(history) — all before and after hardware, all through the same API.

## `cs sim run` — prove behaviour before hardware

The program runs on the server against the simulated device layer (the
demo Modbus slave, `_sim` EtherCAT/CANopen NICs, the fake OPC UA
server); a scenario file plays the plant. This is the step between
"`cs project check` compiles" and "deploy": generate → check → **sim**
→ deploy.

```bash
cs sim run scenarios/fill.toml                 # runs tasks.toml, plays the scenario, stops
cs sim run s.toml --program main               # one PROGRAM instead of the schedule
cs sim run s.toml --trace /tmp/trace.jsonl     # record every polled snapshot
cs sim run s.toml --no-run                     # attach to a run you already started
```

Scenario TOML — steps execute top to bottom, one key per step:

```toml
description = "inlet fills the tank; high alarm raises; no overflow"

[[steps]]
wait_ms = 300                                   # settle time

[[steps]]                                       # play the plant: write an input
set = { var = "inlet_cmd", value = true }       # bool/number/string; encoded by IEC type

[[steps]]                                       # assertion with a deadline (poll ~100ms)
expect = { var = "level", op = "gt", value = 20.0, within_ms = 6000 }

[[steps]]                                       # safety property over a window
expect_never = { var = "overflow", op = "is_true", during_ms = 1500 }

[[steps]]                                       # alarms are first-class assertables
expect_alarm = { id = "level_high", active = true, within_ms = 5000 }

[[steps]]                                       # fault injection: stall N scans so the
inject = { scan_stall_ms = 25 }                 # watchdog trips via its REAL overrun path

[[steps]]                                       # assert on the watchdog latch itself
expect_watchdog = { tripped = true, within_ms = 3000 }
```

Ops: `gt ge lt le eq ne is_true is_false` (`is_*` take no `value`).
Defaults: `within_ms`/`during_ms` 5000, `expect_alarm.active` true,
`inject.scans` threshold+1 (enough to trip), `expect_watchdog.tripped`
true. `expect_watchdog` takes `within_ms` (state REACHED by deadline) or
`during_ms` (state HOLDS for the whole window) — same split as expect vs
expect_never. A negative control before inject must use `during_ms`:
`{ tripped = false, during_ms = 800 }` — with within-semantics it would
pass on the first poll and guard nothing. `inject` is the only
vocabulary that can drive a TIMING fault deterministically — a CPU-burn
program depends on host speed and proves nothing on a fast machine
(reference scenario: `examples/watchdog_latch/scenarios/latch.toml`).
After a trip the VM keeps computing and scan_count keeps climbing —
that is exactly why a health gate must read `watchdog_tripped`, not
liveness.
Exit 0 = all held · 1 = a step failed, and the report names the step,
the deadline, and the LAST OBSERVED value ("expected level > 20 within
6000ms — last observed 3.5") — fix the logic, re-run. Execution stops
at the first failed step (later steps depend on earlier state).

The reference example lives at `examples/sim_smoke/` (tank integrator +
alarm + scenario); it also runs as a cargo e2e test.

Pattern for closed-loop plants: the scenario IS the plant for simple
cases (set inputs, watch outputs). For richer dynamics, add a second
PROGRAM that computes sensor values from actuator state and schedule it
in tasks.toml alongside the program under test.

## Alarms — declared in the project, evaluated by the runtime

Definitions are project config (`alarms.toml`), authored like any other
doc — and YOU should author them alongside the control logic; a program
that can misbehave silently is only half-delivered:

```bash
cs get alarms                       # current definitions
cs set alarms --from -              # replace (applies on the NEXT run)
```

```json
{ "alarms": [ {
    "id": "level_high",              // unique — the ack/journal key
    "variable": "tank_level",        // snapshot variable
    "condition": "gt",               // gt|ge|lt|le|eq|ne|is_true|is_false
    "limit": 90.0,                   // numeric conditions only
    "deadband": 2.0,                 // hysteresis on CLEARING (stops chatter)
    "delay_ms": 500,                 // must hold this long before raising
    "severity": "high",              // info|warn|high|critical
    "message": "Tank level high — check outlet valve"   // actionable, not a restatement
} ] }
```

The server rejects duplicate ids and numeric conditions without a
`limit` (422 — the reason prints on stderr).

State machine (ISA-18.2 shaped): raise → `active,unacked`; `cs runtime
ack` → `active,acked`; condition clears (through the deadband) →
`returned`. **Cleared-but-unacked stays standing** — an alarm that
fired at 03:00 and self-cleared remains visible until a human (or you,
deliberately) acks it.

```bash
cs get runtime/alarms               # live states, standing-first
cs runtime ack level_high           # acknowledge
cs get runtime/alarms-journal       # raised/acked/returned, most recent first
```

On the edge the same surface is `/alarms`, `/alarms/{id}/ack`,
`/alarms-journal`, and `/status` carries `alarms_standing`. The HMI has
an `alarmlist` node (summary table + per-row ACK) — see 08-hmi.md.

## History — "what happened at 03:00" is one GET

A zero-config historian samples every numeric variable at 1 Hz
(~2 h window in memory; the EDGE runtime also persists JSONL segments
under its state dir, so history survives restarts AND deploys).

```bash
cs get runtime/history --query vars=level,flow --query step_ms=1000
cs get runtime/history --query vars=level --query from_us=0 --query to_us=0
```

Response: per-variable buckets `{t_us, min, max, v}` (v = last in
bucket) + `oldest_us` so you know where coverage starts. Time base is
the snapshot clock (scan-relative); alarm journal timestamps are
wall-clock — correlate via "now".

Post-event drill: `cs get runtime/alarms-journal` for WHEN and WHAT →
`cs get runtime/history --query vars=<alarm variable>` around it for
WHY → read the POU → fix → `cs sim run` to prove the fix.
