# Offline readiness checklist

Use this lane when the target hardware is unavailable or the user explicitly asks to prepare now and test on a later bench day. Offline evidence is valuable, but it is not hardware acceptance.

## Keep the boundary explicit

- Work only in the local checkout, local project files, a loopback IA2 server, and simulated adapters.
- Do not discover a bench host, probe an edge, deploy, scan a physical bus, or call runtime `write` / `force` against hardware.
- Do not invent device addresses, interface names, identities, mappings, firmware versions, or network routes. Record each unknown for bench day.
- Treat `cs check`, `cs project check`, unit tests, and `cs sim run` as offline evidence only. They do not prove real timing, wiring, PDO/SDO layout, scaling, polarity, failsafe behavior, or device identity.

## Evidence to close offline

- [ ] Record the repository path, branch, commit, and pre-existing working-tree changes.
- [ ] Run the relevant formatting, unit, build, and static checks for the files changed.
- [ ] Run `cs project check` for each prepared project.
- [ ] Run every applicable scenario with `cs sim run`; preserve the scenario and failure output.
- [ ] Confirm simulations use explicit simulated devices or loopback services, never an unresolved production endpoint.
- [ ] List every hardware-dependent claim under **BENCH PENDING**, with the exact observation that will close it.

## Prepare the bench handoff

Before the first connection, identify the site-specific operating handoff and safety owner. The first live contact is read-only: establish the target identity, service state, deployed runtime or binary hash, and `/status` baseline before any restart, deployment, force, write, or configuration change.

Carry these items to the bench:

- the exact commit and binary/configuration hashes tested offline;
- the project and scenario names that passed offline;
- the expected device identities and mappings, clearly marked as hypotheses until read back;
- the staged hardware tests, safe state, abort condition, and rollback owner;
- a results table that keeps **OFFLINE PASS**, **BENCH PASS**, **FAIL**, and **NOT RUN** distinct.

If the site-specific safety handoff is missing, authorization is unclear, or the read-only baseline disagrees with the prepared assumptions, stop at **BENCH HOLD** and report the mismatch.
