# ADR-0002: external agent-orchestration interfaces — hold the bindings, build the substrate

Status: Proposed (2026-08-28) — draft for review

## Context

Two 2026 developments bracket IA2's "agents + control" territory from
above and from the side:

- **An agent-hardware operate standard** ("the operate standard"
  below; research preview, 2026-08, from a major AI vendor): a
  standardized driver layer for AI agents to *operate* physical
  devices — read/write primitives over a "states + procedures"
  manifest, natural-language device tags compiled into auto-generated
  agent reference files, network discovery, driver-enforced safety
  limits, reachable via MCP / CLI / code files. It works at task/recipe
  granularity; every launch device carries its own internal real-time
  controller (the agent supervises from above the servo loop, not
  within it). The spec is **unpublished** — no schema, wire format,
  transport, versioning, or license — and the preview is waitlisted.
- **A captive engineering agent** (GA 2026-04, from a major
  incumbent automation vendor): agent-as-*engineer* inside the
  vendor's own engineering suite (code / HMI / drive config / full
  project generation in a plan-execute-validate loop).
  Engineering-time only, cloud-session, no public API or SDK,
  output bound to the vendor stack, validation loop not visible to the
  customer.

Neither occupies IA2's slot — an open substrate where **any** coding
agent both engineers (author → `cs check` → `cs sim run` → deploy) and
supervises (HTTP / MQTT) deterministic IEC 61131-3 control on
commodity hardware. But both set expectations IA2 will be measured
against: driver-enforced write limits, per-device agent reference
files, and a discoverable "operate" surface.

Constraints that shape the decisions below:

- `MEMORY/principles.md` default-rejects "industry standard alignment"
  trades absent real demand, and reserves MCP wrapping for "if some
  agent platform demands it".
- There is no operate-standard spec text to build against; anything
  written today
  is designed twice.
- The edge runtime's security doctrine is deliberate: bind
  `127.0.0.1:13001` (`crates/runtime/src/main.rs`), "Auth: none
  (localhost-only). Remote access via SSH port-forward"
  (`docs/api.md`), no firewall holes (`docs/edge-deploy.md`).
- The scan-loop I/O contract is scalar:
  `ChannelValue = Bool | U16 | I32 | Real(f32) | F64`
  (`crates/iocore/src/lib.rs`).

## Decision 1: no external-orchestration binding before spec + demand

An operate-standard binding (southbound adapter or northbound driver) is
gated on **both**:

1. the spec is published under a real open-source license with schema,
   transport, and versioning; and
2. a named demander exists in IA2's territory — a customer, machine
   builder, integrator, or agent platform asking to address an IA2
   edge, or such devices actually appearing in target plants.

Spec publication alone re-opens *design review* (re-run the audit
behind this ADR against real schema), not implementation. Nothing in
this ADR is a commitment to any one scheme; the same gates apply to
any successor or domestic-scheme equivalent.

## Decision 2: substrate first — the scheme-agnostic prerequisites

The pieces every external operate-surface needs are the same pieces
IA2 should have on its own terms. They are separate changes (tracked
as their own work), but this ADR records why they precede any binding:

| Piece | Today | Why it is a prerequisite |
|---|---|---|
| Per-tag write governance | `/write` and `/force` accept any value on any variable; alarms annunciate, never clamp | An exported write surface without allowlist + min/max clamps is unsafe by construction. Also closes IA2's one concrete gap vs driver-enforced-limit designs. Requires an explicit decision on `/force` vs clamps (bypass = hole in the safety story; no bypass = force is no longer a pure debug override). |
| Per-variable metadata (units, range, NL description) | `Mapping` is `application/variable/direction/device/channel` only (`crates/project/src/types.rs`) | Any agent-facing reference file needs it. Extend `iomap.toml` / `alarms.toml` — no new config file (anti "multi-config syndrome"). |
| Agent attribution on the runtime write path | Takeover overlay fires only for `cs`-driven mutations | Any non-`cs` operator would drive the plant with no banner — a truthfulness-doctrine breach independent of any standard. |
| `cs device describe` — deterministic manifest from project truth (variables + alarms + mappings) | Precedent: `POST /api/hmi/{path}/generate` derives screens from project truth | This *is* the "auto-generated agent reference file", produced scheme-agnostically; a future binding only reformats it. |

## Decision 3: northbound shape — sidecar first; in-runtime facade later, maybe never

When Decision 1's gates open for an operate surface, the **first
artifact is an out-of-process sidecar driver** wrapping the edge
runtime's existing HTTP monitor:

```
agent harness ── operate protocol ─── sidecar (own process, on the box
                                        or tunnel-side) ── 127.0.0.1:13001
                                        /status /events /write /alarms
                                        /alarms/{id}/ack /pause /resume
                                        /stop /history
```

- Zero runtime changes, zero doctrine cost: the runtime keeps
  localhost-only + SSH; the sidecar terminates the external protocol
  and is itself cheap to write (external driver-authoring is the
  explicit design goal of operate-standard specs — a driver can even be
  agent-generated against `docs/api.md`).
- Safe by construction once Decision 2's clamps exist.
- An **in-runtime facade** is considered only if the sidecar
  demonstrably can't meet a real consumer's needs. If built, it is a
  sibling of `crates/runtime/src/northbound.rs` and copies
  `NorthboundConfig`'s governance shape — write capability is opt-in
  (`NorthboundConfig::allow_write`, `crates/project/src/types.rs`),
  off by default.

Rules for any northbound shape:

- Exported surface is exactly: read everything; write the clamped
  allowlist; ack alarms. **`/force` / `/unforce` are never exported**
  — they are debug overrides, not setpoint sources.
- Network-discoverable presence and an auth layer are a separate,
  later, opt-in milestone; the auth design must exist before any
  non-localhost bind ships. The localhost doctrine is not reversed
  for an unpublished spec.
- Any external writer must surface through the attribution mechanism
  (Decision 2), or the surface does not ship.

## Decision 4: task-handshake convention (designed, not built)

External orchestration layers speak in long-running parameterized
procedures ("run task X"); the scan loop speaks in scalar channels
sampled every cycle. The mapping — in **both** directions (consuming
an intelligent device southbound; exporting ST sequences as named
procedures northbound) — is the classic PLC handshake:

- Per pre-registered task: `start` (BOOL, rising edge) · `busy` ·
  `done` · `error_code` · optional `progress`; numeric parameter
  channels are **latched at the start edge**.
- Tasks and their parameter channels are declared in device config;
  no dynamic/rich payloads at runtime.
- **`ChannelValue` stays scalar.** Adding `Str` is a contract change
  rippling through `to_vm_bits`, the i32-based VM write API (see
  ADR-0001 patch registry), snapshots, and `iomap_check` — rejected
  until a demonstrated need survives review.
- Any adapter implementing the handshake ships a `_sim` mode (like
  the `_sim` EtherCAT nic) so `cs sim run` scenarios assert on
  **status transitions, not timing** — external task durations are
  nondeterministic ("proof before hardware" applies to bindings too).
- Failsafe: for a device mid-task, `enter_failsafe` must invoke the
  device's **own safe-stop primitive** and report honestly if it
  couldn't. Zeroing command channels is not "stop" for a moving
  device.

## Decision 5: southbound seam notes (external-scheme `iomap-*` adapter, when gated in)

Recorded so the knowledge survives until Decision 1's gates open:

- One crate implementing the `IoDevice` trait
  (`crates/iocore/src/lib.rs`). Template is **`iomap-opcua`**, not
  the field-bus adapters: background task owns the connection and a
  tag mirror (bulk refresh per `poll_interval_ms`; `read_channel`
  serves the mirror; writes go direct), bounded connect + reconnect
  forever + `HealthTracker`, per-channel opt-in failsafe. The OPC UA
  crate's header comment ("IA2 is the *supervisory* layer") is this
  adapter's mirror image.
- Registration points: variants on the `Protocol` and
  `ProtocolConfig` enums (`crates/project/src/types.rs`; config shaped
  like `OpcuaConfig`: endpoint + `poll_interval_ms` + channels of
  `name / tag / data_type / access / failsafe`), one `connect_one()`
  arm (`crates/ironplc-bridge/src/runtime.rs`);
  `iomap_check` picks channel names up via
  `ProtocolConfig::channel_names()`.
- Free by construction: server device routes, the `cs` quartet,
  ts-rs types. Real work: web device-editor pane, a `docs/api.md`
  row if a browse/discovery route is added (test-enforced), skill
  `references/02` / `06`, and SKILL.md's protocol scope line.
- Size anchor: `iomap-opcua` ≈ 1,050 lines; expect ~1–1.5× for the
  mirror lane plus ~300–500 lines of handshake. The handshake design
  (Decision 4) is the risk; the mirror code is the bulk.

## Revisit triggers

Any of these re-opens this ADR out of cycle (they are watch
conditions, not commitments):

| # | Trigger | Action |
|---|---|---|
| T1 | An operate-standard spec published under a real open-source license | Re-run the seam audit against real schema within two weeks; finalize Decision 4; southbound go/no-go |
| T2 | A fieldbus/PLC gateway driver appears (someone wraps OPC UA / Modbus / a PLC as an operate-standard device) | Start the northbound sidecar |
| T3 | A competing IEC 61131-3 runtime ships support for such a standard | Northbound sidecar sprint |
| T4 | The standard's material covers discrete manufacturing / machine state models | Escalate from tracking to prototyping |
| T5 | A domestic equivalent scheme emerges (possibly product-first, without a formal "publication") | Evaluate alignment with it first; re-scope this ADR's target |
| T6 | A customer / platform asks to address an IA2 edge through such a surface | Decision 1's demand gate is met — sidecar path |
| E1 | The captive engineering-agent vendor publishes an API/SDK or adopts MCP | "Any agent drives it" differentiation compresses — revisit positioning assumptions in docs |
| E2 | On-prem/air-gapped variants of captive agents ship | Deployment differentiation compresses — same |
| E3 | A captive agent gains a non-vendor export/target path | Hardware-agnosticism differentiation compresses — same |
| E4 | A captive agent moves downstream into commissioning / online PLC writes | The engineering-time boundary assumed above no longer holds — revisit Decisions 2–3 urgency |

## Consequences

- IA2 spends nothing on speculative bindings, yet a binding, when
  gated in, is a bounded job: the substrate exists (Decision 2), the
  shape is chosen (Decision 3), the hard design question is settled
  (Decision 4), and the seam is mapped (Decision 5).
- The runtime's security doctrine and the `ChannelValue` contract are
  explicitly protected from "the standard needs it" pressure — changes
  there require re-opening this ADR, not an adapter PR.
- The substrate items become the visible, customer-verifiable answer
  to the risks analysts attach to captive agents (unbounded write
  access, rubber-stamp review): limits are enforced config, proofs are
  re-runnable text artifacts the customer owns.

## Implementation note (2026-09-03, still Proposed)

Decision 2's substrate rows landed as working code (uncommitted, under
review with this ADR):

- **Per-tag write governance** — `project.toml` `[governance]`:
  `write_mode = "open" | "allowlist"` (default `open` = exact legacy
  behaviour) plus `[[governance.rules]]` entries naming a monitor
  variable with optional `min`/`max`. Enforced in the scan thread's
  write command, so all four write paths (IDE server, edge runtime
  HTTP, IDE→edge proxy, MQTT northbound) share one implementation.
  Unlisted writes are denied — 403 on the HTTP paths; the MQTT path
  has no reply channel, so its denials land in the log and the audit
  ring instead — and out-of-range writes are clamped, with the
  response/log saying so.
- **The `/force` question is decided: force bypasses governance.**
  Rationale: clamping force would destroy its only purpose (driving a
  variable somewhere logic never takes it, e.g. commissioning travel
  tests), and exporting an ungoverned force northbound would be a hole
  — so force stays a pure local debug override and is simply never
  exported (Decision 3's rule already says exactly that). The risk the
  ADR flagged ("bypass = hole") is closed by the export ban, not by
  clamping.
- **Per-variable metadata** — `unit` / `min` / `max` / `description`
  on `iomap.toml` `[[mappings]]` entries (optional; old files
  round-trip byte-identical). Consumed by `hmi generate` and by
  `cs get devices/<n>/describe`.
- **`cs device describe`** — `GET /api/devices/{name}/describe` +
  `cs get devices/<n>/describe`, the deterministic per-device agent
  reference file (config with secrets redacted, bound variables with
  metadata, related alarms, applicable governance rules).
- **Attribution** — `X-IA2-Origin` convention (a self-declared label,
  not authentication), a bounded write-audit ring on the edge runtime
  (`GET /audit`), and server-side auto-attribution that surfaces every
  mutating runtime call on the takeover overlay unless it claims to be
  the IDE (`gui`) or is `cs` traffic belonging to the open agent
  session. This is the mechanism Decision 3 requires before any
  external write surface ships.
