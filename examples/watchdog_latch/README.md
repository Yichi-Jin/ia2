# watchdog_latch — bench fixture for the scan-watchdog latch

Proves on real hardware what
`runtime::tests::watchdog_trip_latches_outputs_off_until_restart` proves in
a unit test: once the scan watchdog trips, **no non-zero output may reach
the bus until the program is restarted**.

## Why a fixture is needed

The watchdog fires on a real timing condition — a scan body that cannot
finish inside its interval. Nothing in the `cs sim` scenario DSL
(`wait_ms / set / expect / expect_never / expect_alarm`) can inject that,
so the behaviour cannot be driven from a scenario. This project creates
the condition structurally instead: `pous/main.st` burns CPU in a `FOR`
loop that cannot complete within the 2 ms task interval, so **every** scan
overruns and the 5-in-a-row threshold is reached in ~10 ms.

## Safety — this fixture cannot move an axis

`control_word` is only ever driven to `16#0006` (Shutdown: Enable voltage +
Quick stop, Switch on cleared). That parks a CiA402 drive at
**Ready to switch on** — energised, not enabled. Operation Enabled is never
commanded and `target_position` is deliberately left unmapped.

The observable is the **drive's statusword**, not motion. Do not edit the
program to command `16#000F`.

## What you are watching

| Phase | controlword on the wire | Expected statusword |
|---|---|---|
| Before the trip | `0x0006` | `0x1631` — Ready to switch on |
| Watchdog trips, failsafe zeroes the PDI | `0x0000` | bit 6 set — **Switch on disabled** |
| **Fixed build** | stays `0x0000` | **stays** in Switch on disabled |
| **Pre-fix build** | back to `0x0006` next scan | **returns** to `0x1631` |

The whole verdict is whether the statusword *stays* or *bounces back*.

## Running it

Offline first (no hardware, proves the project loads):

```bash
cargo run -p server &
cs api POST /api/projects/open -d '{"path":"'$PWD'"}'
cs api POST /api/project/validate     # [] == clean
```

On the bench, point the device at the real NIC first:

```bash
# devices/servo.toml
nic = "enp2s0"        # was "_sim"
```

Then follow `docs/bench/watchdog-latch-runbook.md`.

## Tuning `burn_n`

`burn_n` (default 200000) sets the burn length. It is a plain `DINT` in the
program, so it can be forced at runtime. Raise it until scans actually
overrun — the check is that `scan_count` stops tracking wall clock:

```bash
cs get runtime/status     # scan_count should grow far slower than uptime_secs × 500
```

On a host fast enough to finish 200000 iterations inside 2 ms the watchdog
will never trip and the test is vacuous — always confirm the overrun before
trusting the result.
