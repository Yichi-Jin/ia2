# Examples

Real, runnable IA2 projects — open any of them with
`cs project open examples/<name>` (or the IDE's Open dialog) and go.

| Project | What it teaches |
|---|---|
| `sim_smoke/` | The **`cs sim` reference**: a tiny tank integrator, an `alarms.toml` high-level alarm, and `scenarios/fill.toml` proving fill → alarm → no-overflow. Run it: `cs project open examples/sim_smoke && cs sim run examples/sim_smoke/scenarios/fill.toml`. Also exercised by the cargo e2e test (`crates/cli/tests/sim_e2e.rs`). |
| `eg_gear_incycle/` | EtherCAT electronic gearing — on-the-fly, phase-continuous gear-ratio change against the `_sim` NIC. |
| `write_governance/` | Device-free scenario proving writes above/below the allowlist range are clamped and in-range values reach the program unchanged. |
| `nx6_modbus/` | Modbus TCP wiring against a real remote-IO block: device config, iomap, task schedule. |
| `watchdog_latch/` | The **scan-watchdog latch** fixture: proves the overrun watchdog holds outputs off instead of quietly resuming. `burn_n` ships as 0 so a healthy host is the negative control; `scenarios/latch.toml` injects the stall (`inject` step) and asserts the latch (`expect_watchdog`). Run it: `cs project open examples/watchdog_latch && cs sim run examples/watchdog_latch/scenarios/latch.toml`. Bench procedure: `docs/bench/watchdog-latch-runbook.md`. |
| `supervisory-demo/` | The DCS-supervision shape: OPC UA southbound to a fake DCS (`cargo run -p iomap-opcua --example fake_dcs`), MQTT northbound config, an edge entry. |

Every example is plain text (TOML + ST + JSON) — read them like code,
copy them as starting points. If a doc references an example that
isn't in this table, that's a bug (see `MEMORY/principles.md` § 3).
