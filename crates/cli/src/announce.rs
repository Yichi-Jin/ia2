//! Best-effort heartbeat / takeover-session announcement to the IDE, so
//! the "agent in control" overlay renders whenever the CLI CHANGES
//! state. Read-only commands (`ls`, `get`, `check`, `probe`, `runtime
//! status/snapshot`, …) deliberately skip the heartbeat — querying
//! state isn't "operating" and shouldn't trigger the overlay. That rule
//! lives here and nowhere else.

use crate::http::http_agent;
use crate::{Command, HmiCmd, LibraryCmd, ProjectCmd, RuntimeCmd};

/// Env var holding the active session id between `cs agent enter` and
/// `cs agent leave`. Set by the user or by `cs agent run` for its
/// wrapped command.
pub(crate) const SESSION_ENV: &str = "IA2_AGENT_SESSION";

/// Overlay label for commands that mutate server-visible state; `None`
/// for reads and offline analysis. One match arm per verb — the
/// quartet collapses what used to be ~40 arms into four.
pub(crate) fn announce_label(cmd: &Command) -> Option<String> {
    match cmd {
        // Meta-primitives: mutation is decided by the HTTP verb.
        Command::Api { method, path, .. } => {
            if method.eq_ignore_ascii_case("get") {
                None
            } else {
                Some(format!("api {} {}", method.to_uppercase(), path))
            }
        }
        Command::Set { path, .. } => Some(format!("set {path}")),
        Command::Rm { path } => Some(format!("rm {path}")),
        Command::Ls { .. } | Command::Get { .. } => None,

        // Offline / read-only analysis.
        Command::Check { .. }
        | Command::Transpile { .. }
        | Command::Symbols { .. }
        | Command::Probe { .. } => None,

        // Runtime control mutates; status/snapshot read.
        Command::Runtime(r) => match r {
            RuntimeCmd::Status { .. } | RuntimeCmd::Snapshot { .. } => None,
            RuntimeCmd::Pause { .. } => Some("runtime pause".into()),
            RuntimeCmd::Resume { .. } => Some("runtime resume".into()),
            RuntimeCmd::Step { .. } => Some("runtime step".into()),
            RuntimeCmd::Force { .. } => Some("runtime force".into()),
            RuntimeCmd::Unforce { .. } => Some("runtime unforce".into()),
            RuntimeCmd::Write { .. } => Some("runtime write".into()),
            RuntimeCmd::Ack { .. } => Some("runtime ack".into()),
        },

        Command::Run { .. } => Some("run".into()),
        Command::Stop => Some("stop".into()),
        Command::Sim(crate::SimCmd::Run { scenario, .. }) => Some(format!("sim run {scenario}")),
        Command::Deploy { name } => Some(format!("deploy {name}")),

        Command::Hmi(h) => match h {
            HmiCmd::Op { path, .. } => Some(format!("hmi op {path}")),
            HmiCmd::Generate { path, .. } => Some(format!("hmi generate {path}")),
        },
        Command::Library(LibraryCmd::Import { library, .. }) => {
            Some(format!("library import {library}"))
        }

        Command::Project(p) => match p {
            ProjectCmd::Check { .. } | ProjectCmd::Info { .. } => None,
            ProjectCmd::Create { name } => Some(format!("project create {name}")),
            ProjectCmd::Open { .. } => Some("project open".into()),
            ProjectCmd::Close => Some("project close".into()),
        },

        // `cs agent run` manages its own explicit session.
        Command::Agent(_) => None,
    }
}

/// Per-process session id, comparable in logs to a heartbeat session
/// hint. Format: `cs-<pid>-<nanos>`.
pub(crate) fn session_id() -> &'static str {
    use std::sync::OnceLock;
    static SESSION: OnceLock<String> = OnceLock::new();
    SESSION.get_or_init(|| {
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("cs-{pid}-{nanos:x}")
    })
}

/// Fire-and-forget heartbeat. Short timeout because we'd rather miss
/// the visual cue than hold up a command's actual work. Inside a
/// `cs agent run` session the forwarded IA2_AGENT_SESSION keeps these
/// on the steady session banner instead of flashing.
pub(crate) fn announce_agent(server: &str, command_label: &str) {
    let session = std::env::var(SESSION_ENV)
        .ok()
        .unwrap_or_else(|| session_id().to_string());
    let _ = http_agent()
        .post(&format!("{server}/api/agent/heartbeat"))
        .timeout(std::time::Duration::from_millis(300))
        .send_json(serde_json::json!({
            "command": command_label,
            "session": session,
        }));
}
