//! Runtime-monitor routes — the IDE server's half of the shared
//! online-debug surface. The SEMANTICS (pulse reset, force
//! bookkeeping, mode ops, wire shapes) live once in
//! `ironplc_bridge::monitor`; the edge runtime mounts the same ops on
//! its own router. This file is only the IDE-side axum glue.

use axum::extract::{Path as AxumPath, State};
use axum::Json;
use ironplc_bridge::{DeviceHealth, ProgramHandle, RuntimeMode, VarSnapshot};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::ApiError;
use crate::routes::ProjectName;
use crate::state::{AppState, RunningInfo};

// ============================================================
//  Runtime — synchronous queries + variable writes
// ============================================================

/// Most recent VarSnapshot from the running bridge, or `null` when
/// nothing has been snapshotted in the current session (no run, or
/// project was just closed). Lets agents poll one-shot without
/// subscribing to /api/events SSE.
pub async fn runtime_snapshot(State(state): State<AppState>) -> Json<Option<VarSnapshot>> {
    Json(state.last_snapshot.lock().clone())
}

// ============================================================
//  History + alarms — served by the shared monitor layer
// ============================================================

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    /// Comma-separated variable names; empty/missing = all recorded.
    #[serde(default)]
    pub vars: String,
    #[serde(default)]
    pub from_us: u64,
    /// 0 = no upper bound.
    #[serde(default)]
    pub to_us: u64,
    /// Bucket width; defaults to 1000 ms.
    #[serde(default = "default_step_ms")]
    pub step_ms: u64,
}

fn default_step_ms() -> u64 {
    1000
}

/// Downsampled history for the named variables (min/max/last buckets).
/// Backed by the in-memory historian on the IDE server; the edge
/// runtime serves the same shape from its persisted rings at /history.
pub async fn runtime_history(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<HistoryQuery>,
) -> Json<ironplc_bridge::monitor::HistoryResponse> {
    let vars: Vec<String> = q
        .vars
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    Json(state.historian.query(&vars, q.from_us, q.to_us, q.step_ms))
}

/// Live alarm states (severity-major, standing first). Defined by the
/// project's alarms.toml; evaluated while a program runs.
pub async fn runtime_alarms(
    State(state): State<AppState>,
) -> Json<Vec<ironplc_bridge::monitor::AlarmState>> {
    Json(state.alarms.lock().states())
}

/// Acknowledge one alarm. 404 for an unknown id.
pub async fn runtime_alarm_ack(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<ironplc_bridge::monitor::AlarmState>, ApiError> {
    let now_us = ironplc_bridge::monitor::wall_now_us();
    state
        .alarms
        .lock()
        .ack(&id, now_us)
        .map(Json)
        .map_err(ApiError::NotFound)
}

#[derive(Debug, Deserialize)]
pub struct JournalQuery {
    #[serde(default = "default_journal_limit")]
    pub limit: usize,
}

fn default_journal_limit() -> usize {
    100
}

/// Most-recent-first alarm journal (raised / acked / returned).
pub async fn runtime_alarm_journal(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<JournalQuery>,
) -> Json<Vec<ironplc_bridge::monitor::AlarmJournalEntry>> {
    Json(state.alarms.lock().journal(q.limit))
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct RuntimeStatus {
    pub running: bool,
    pub project: Option<String>,
    /// Program instances declared in tasks.toml (what's actually scheduled).
    pub program_instances: Vec<String>,
    /// Device names declared in the project's `devices/` files — configuration,
    /// not connectivity. See `device_health` for whether they are actually up.
    pub devices: Vec<String>,
    /// Live per-device fieldbus health from the running program: `healthy:
    /// false` means that device's inputs are frozen at last-known values and
    /// its outputs are being dropped, while the scan loop keeps running and
    /// `running` stays `true`. Empty when nothing is running.
    ///
    /// Without this, a UI built on `/api/runtime/status` cannot distinguish a
    /// healthy plant from one whose bus died — the variable values alone never
    /// reveal it (see `DeviceHealth`).
    pub device_health: Vec<DeviceHealth>,
    /// `true` once the scan watchdog has tripped: the program lost real-time
    /// guarantees, `enter_failsafe` zeroed every output and the output phase
    /// is latched off until an explicit restart.
    ///
    /// Read this before trusting any variable value as plant state. After a
    /// trip the VM keeps computing and `scan_count` keeps advancing, so a UI
    /// that only watches values sees a perfectly healthy plant while the bus
    /// is actually holding zeros. `running` also stays `true` — the scan loop
    /// deliberately keeps going so operators can still inspect live state.
    pub watchdog_tripped: bool,
    /// Scan count from the most recent snapshot; 0 before the first one.
    pub scan_count: u64,
    /// Timestamp_us of the most recent snapshot, or 0.
    pub last_snapshot_us: u64,
    pub last_error: Option<String>,
    /// What kind of run is active (isolated vs scheduled) and which
    /// PROGRAM(s) it covers. Populated from `AppState.running_info`
    /// which the /api/run handler writes when it starts a program;
    /// cleared on /api/stop and on close-project. `None` here just
    /// means nothing is currently running.
    pub running_info: Option<RunningInfo>,
    /// Current scan-loop mode (`running` / `paused` / `step{remaining}`).
    /// `None` when nothing's running.
    pub mode: Option<RuntimeMode>,
    /// Currently-forced variables in `name=value` pairs, sorted by name.
    /// Empty when no force is active. The shape matches the in-memory
    /// HashMap snapshot from `ProgramHandle::forces()`.
    pub forces: Vec<ForceEntry>,
}

pub use ironplc_bridge::monitor::ForceEntry;

/// One-shot overview of the runtime — designed for agents who want
/// "what's going on right now" without composing /health + /api/project
/// + the SSE stream.
///
/// Multi-project note: status is scoped to whichever project the
/// caller named in `X-IA2-Project` (falling back to active). The
/// `running_info` field reports the globally-running program even if
/// it doesn't belong to the queried project; the IDE renders this as
/// "running: <project>/<program>" so the user can see when their
/// window is observing a sibling project's run.
pub async fn runtime_status(
    State(state): State<AppState>,
    project: ProjectName,
) -> Json<RuntimeStatus> {
    let running = state.program.lock().is_some();
    // Project-scoped fields: pulled from whichever project the
    // header (or active fallback) names. None if no project matched.
    let (project_name, programs, devices) = {
        let guard = state.projects.lock();
        let store = match project.as_deref() {
            Some(name) => guard.get(name),
            None => guard.active(),
        };
        match store {
            Some(store) => {
                let tasks = store.read_tasks().ok().flatten().unwrap_or_default();
                let programs = tasks.programs.iter().map(|p| p.instance.clone()).collect();
                let devices = store
                    .list_devices()
                    .map(|ds| ds.iter().map(|d| d.name.clone()).collect())
                    .unwrap_or_default();
                (Some(store.name().to_string()), programs, devices)
            }
            None => (None, vec![], vec![]),
        }
    };
    let snap = state.last_snapshot.lock().clone();
    let last_error = state.last_error.lock().clone();
    let running_info = state.running_info.lock().clone();
    // Mode + forces come from the live ProgramHandle, when there is
    // one. Clone the handle out of the mutex briefly to avoid holding
    // the sync lock across the calls.
    let (mode, forces, device_health, watchdog_tripped) = {
        let guard = state.program.lock();
        match guard.as_ref() {
            Some(rp) => (
                Some(rp.handle.mode()),
                ironplc_bridge::monitor::force_entries(&rp.handle),
                rp.handle.device_health(),
                rp.handle.watchdog_tripped(),
            ),
            None => (None, vec![], vec![], false),
        }
    };
    Json(RuntimeStatus {
        running,
        project: project_name,
        program_instances: programs,
        devices,
        device_health,
        watchdog_tripped,
        scan_count: snap.as_ref().map(|s| s.scan_count).unwrap_or(0),
        last_snapshot_us: snap.as_ref().map(|s| s.timestamp_us).unwrap_or(0),
        last_error,
        running_info,
        mode,
        forces,
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct WriteVariableRequest {
    /// Raw i32 value to write — the VM's variable-write primitive is
    /// `write_variable(VarIndex, i32)`, so callers map their domain type
    /// to an i32 (BOOL → 0/1, USINT/UINT → numeric, etc.).
    pub value: i32,
    /// Momentary hold: after `pulse_ms` the SERVER writes 0 back. The
    /// reset guarantee lives here — not in a page timer — so a closed
    /// tab or suspended tablet can't leave a momentary command latched
    /// (the HMI pulse action's contract). Overlapping pulses on one
    /// variable are latest-write-wins.
    #[serde(default)]
    pub pulse_ms: Option<u32>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct WriteVariableResponse {
    pub name: String,
    pub value: i32,
}

/// Poke a variable while the program is running. Applied between scan
/// rounds (so the next round's logic sees the new value). 404 if the
/// name doesn't resolve to any declared variable; 409 if no program is
/// running.
pub async fn write_runtime_variable(
    State(state): State<AppState>,
    _project: ProjectName,
    AxumPath(name): AxumPath<String>,
    Json(req): Json<WriteVariableRequest>,
) -> Result<Json<WriteVariableResponse>, ApiError> {
    // The runtime is global (one PROGRAM at a time, hardware constraint);
    // the `X-IA2-Project` header is accepted for symmetry but not used
    // to select a program — clients can poll runtime_status to see which
    // project's program is actually running. Clone the handle out of the
    // mutex so we don't hold a sync lock across the .await below — see
    // the bridge::ProgramHandle docs.
    let handle: ProgramHandle = live_program(&state)?;
    // The momentary-pulse reset guarantee lives in the shared monitor
    // core — one implementation for IDE server and edge runtime.
    let value =
        ironplc_bridge::monitor::write_with_pulse(&handle, &name, req.value, req.pulse_ms).await?;
    Ok(Json(WriteVariableResponse { name, value }))
}

// ============================================================
//  Debug control trio: pause / resume / step + force / unforce
//
//  All four endpoints share the same "look up the live handle, 409
//  if nothing running" pattern as `write_runtime_variable`. Mode
//  toggles are synchronous on the bridge side (just a mutex write);
//  force commands round-trip through the cmd channel so the scan
//  loop can validate the variable name before the ack comes back.
// ============================================================

pub use ironplc_bridge::monitor::{ModeResponse, StepRequest};

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct ForceRequest {
    pub value: i32,
}

/// Look up the live program handle or return 409. Used by every
/// debug-control endpoint below. The handle is global (one PROGRAM
/// runs at a time, hardware constraint), so this helper doesn't take
/// a project name — callers that want to know *which* project owns
/// the running program use `/api/runtime/status.running_info`.
fn live_program(state: &AppState) -> Result<ProgramHandle, ApiError> {
    state
        .program
        .lock()
        .as_ref()
        .map(|rp| rp.handle.clone())
        .ok_or(ApiError::Conflict("no program running".into()))
}

pub async fn runtime_pause(State(state): State<AppState>) -> Result<Json<ModeResponse>, ApiError> {
    let handle = live_program(&state)?;
    Ok(Json(ironplc_bridge::monitor::pause(&handle)))
}

pub async fn runtime_resume(State(state): State<AppState>) -> Result<Json<ModeResponse>, ApiError> {
    let handle = live_program(&state)?;
    Ok(Json(ironplc_bridge::monitor::resume(&handle)))
}

pub async fn runtime_step(
    State(state): State<AppState>,
    body: Option<Json<StepRequest>>,
) -> Result<Json<ModeResponse>, ApiError> {
    let handle = live_program(&state)?;
    let cycles = body.map(|Json(r)| r.cycles).unwrap_or(1);
    Ok(Json(ironplc_bridge::monitor::step(&handle, cycles)))
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ForceResponse {
    pub name: String,
    pub value: i32,
}

/// Pin a variable's value. The scan loop applies it on every cycle
/// until `unforce_runtime_variable` is called. 404 if the variable
/// isn't declared in this POU; 409 if nothing's running.
pub async fn force_runtime_variable(
    State(state): State<AppState>,
    _project: ProjectName,
    AxumPath(name): AxumPath<String>,
    Json(req): Json<ForceRequest>,
) -> Result<Json<ForceResponse>, ApiError> {
    let handle = live_program(&state)?;
    let value = handle.force_variable(&name, req.value).await?;
    Ok(Json(ForceResponse { name, value }))
}

/// Release a forced variable. No-op (200) if the variable wasn't
/// forced — convenient for idempotent agent retries.
pub async fn unforce_runtime_variable(
    State(state): State<AppState>,
    _project: ProjectName,
    AxumPath(name): AxumPath<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let handle = live_program(&state)?;
    handle.unforce_variable(&name).await?;
    Ok(Json(serde_json::json!({ "name": name, "forced": false })))
}

/// List currently-forced variables. Returns `[]` when nothing's
/// running (rather than 409) — easier for clients to render.
pub async fn list_runtime_forces(State(state): State<AppState>) -> Json<Vec<ForceEntry>> {
    let forces = state
        .program
        .lock()
        .as_ref()
        .map(|rp| ironplc_bridge::monitor::force_entries(&rp.handle))
        .unwrap_or_default();
    Json(forces)
}
