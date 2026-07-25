//! `cs api` — the raw escape hatch. Any endpoint in `docs/api.md`,
//! one command, zero per-endpoint CLI code:
//!
//!   cs api GET  /api/edges/pi/probe
//!   cs api POST /api/edges/pi/attach
//!   cs api POST /api/devices/rio/esi-assemble --from idents.json
//!   cs api POST /api/project/migrate-tasks
//!
//! This is what guarantees "everything the GUI can do, the CLI can do"
//! stays true by construction. Prefer the porcelain (`ls/get/set/rm`,
//! `run`, `deploy`, …) when one exists — it has better output and exit
//! semantics — and drop to `cs api` for the rest.

use anyhow::{anyhow, Result};

use crate::http::{print_json, read_blob, Body, Client, UsageError};

pub(crate) fn cmd_api(
    client: &Client,
    method: &str,
    path: &str,
    from: Option<&str>,
    query: &[(String, String)],
) -> Result<i32> {
    let method = method.to_ascii_uppercase();
    if !path.starts_with('/') {
        return Err(UsageError::wrap(anyhow!(
            "path must start with `/` (e.g. /api/runtime/status)"
        )));
    }

    let body_text = match from {
        Some(f) => Some(read_blob(f)?),
        None => None,
    };
    let body_json = match &body_text {
        Some(t) if !t.trim().is_empty() => {
            Some(serde_json::from_str::<serde_json::Value>(t).map_err(|e| {
                UsageError::wrap(anyhow!(
                    "request body isn't valid JSON (the IA2 API speaks JSON): {e}"
                ))
            })?)
        }
        _ => None,
    };

    let full = if query.is_empty() {
        path.to_string()
    } else {
        let qs: Vec<String> = query.iter().map(|(k, v)| format!("{k}={v}")).collect();
        format!("{path}?{}", qs.join("&"))
    };

    let body = match &body_json {
        Some(v) => Body::Json(v),
        None => Body::None,
    };
    let resp = client.request(&method, &full, body, None)?;
    print_json(&resp)
}
