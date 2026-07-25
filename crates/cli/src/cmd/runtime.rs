//! Runtime lifecycle (`cs run` / `cs stop`) and the online debug verbs
//! (`cs runtime pause/resume/step/status/snapshot/force/unforce/write`),
//! plus the value-encoding helpers the force/write paths use. These
//! stay porcelain — force/write carry type-aware bit-packing and the
//! whole family has safety semantics a generic verb shouldn't blur.

use std::path::Path;

use anyhow::{Context, Result};

use crate::http::{print_json, url_encode, Client};
use crate::RuntimeCmd;

pub(crate) fn cmd_run(client: &Client, program: Option<&str>, file: Option<&Path>) -> Result<i32> {
    let body = match (program, file) {
        (None, None) => serde_json::json!({ "kind": "project" }),
        (Some(name), None) => serde_json::json!({
            "kind": "isolated",
            "program": name,
        }),
        (Some(name), Some(path)) => {
            let abs = path
                .canonicalize()
                .with_context(|| format!("resolving {}", path.display()))?;
            serde_json::json!({
                "kind": "isolated",
                "program": name,
                "file_path": abs.display().to_string(),
            })
        }
        (None, Some(_)) => {
            anyhow::bail!("--file requires --program to name the PROGRAM inside it")
        }
    };
    let resp = client.post("/api/run", &body)?;
    print_json(&resp)
}

pub(crate) fn cmd_stop(client: &Client) -> Result<i32> {
    let resp = client.post("/api/stop", &serde_json::json!({}))?;
    print_json(&resp)
}

pub(crate) fn cmd_runtime(client: &Client, cmd: RuntimeCmd, json: bool) -> Result<i32> {
    match cmd {
        RuntimeCmd::Pause { edge } => {
            let resp = post_op(client, edge.as_deref(), "pause", &serde_json::json!({}))?;
            print_json(&resp)
        }
        RuntimeCmd::Resume { edge } => {
            let resp = post_op(client, edge.as_deref(), "resume", &serde_json::json!({}))?;
            print_json(&resp)
        }
        RuntimeCmd::Step { cycles, edge } => {
            let body = serde_json::json!({ "cycles": cycles });
            let resp = post_op(client, edge.as_deref(), "step", &body)?;
            print_json(&resp)
        }
        RuntimeCmd::Status { edge } => {
            let status = match &edge {
                Some(e) => client.get(&format!("/api/edges/{}/status", url_encode(e)))?,
                None => client.get("/api/runtime/status")?,
            };
            if json {
                return print_json(&status);
            }
            let mode = status
                .get("mode")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let forces = status
                .get("forces")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            // Edge /status has no `running` bool — derive from mode.
            let running = status
                .get("running")
                .and_then(|v| v.as_bool())
                .unwrap_or_else(|| mode.get("kind").and_then(|k| k.as_str()) == Some("running"));
            println!(
                "running: {running}  mode: {}  forces: {}",
                serde_json::to_string(&mode)?,
                forces.len(),
            );
            for f in &forces {
                if let (Some(n), Some(v)) = (f.get("name").and_then(|v| v.as_str()), f.get("value"))
                {
                    println!("  {n} := {v}");
                }
            }
            Ok(0)
        }
        RuntimeCmd::Snapshot { vars, edge } => {
            // The one runtime READ agents need most: current values.
            // Local: /api/runtime/snapshot. Edge: last_snapshot off the
            // proxied /status (the edge monitor keeps it fresh).
            let snap = match &edge {
                Some(e) => {
                    let status = client.get(&format!("/api/edges/{}/status", url_encode(e)))?;
                    status
                        .get("last_snapshot")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null)
                }
                None => client.get("/api/runtime/snapshot")?,
            };
            let filtered = match &vars {
                Some(list) => filter_snapshot(&snap, list),
                None => snap,
            };
            // Snapshot output is inherently machine data — always JSON.
            print_json(&filtered)
        }
        RuntimeCmd::Force { name, value, edge } => {
            let resp = match &edge {
                Some(e) => {
                    let encoded =
                        pack_value(&name, edge_var_type(client, e, &name).as_deref(), &value)?;
                    client.post(
                        &format!("/api/edges/{}/runtime/force", url_encode(e)),
                        &serde_json::json!({ "name": name, "value": encoded }),
                    )?
                }
                None => {
                    let encoded = parse_value(client, &name, &value)?;
                    client.post(
                        &format!("/api/runtime/forces/{}", url_encode(&name)),
                        &serde_json::json!({ "value": encoded }),
                    )?
                }
            };
            print_json(&resp)
        }
        RuntimeCmd::Unforce { name, edge } => {
            let resp = match &edge {
                Some(e) => client.post(
                    &format!("/api/edges/{}/runtime/unforce", url_encode(e)),
                    &serde_json::json!({ "name": name }),
                )?,
                None => client.delete(&format!("/api/runtime/forces/{}", url_encode(&name)))?,
            };
            print_json(&resp)
        }
        RuntimeCmd::Ack { id } => {
            let resp = client.post(
                &format!("/api/runtime/alarms/{}/ack", url_encode(&id)),
                &serde_json::json!({}),
            )?;
            print_json(&resp)
        }
        RuntimeCmd::Write { name, value, edge } => {
            let resp = match &edge {
                Some(e) => {
                    let encoded =
                        pack_value(&name, edge_var_type(client, e, &name).as_deref(), &value)?;
                    client.post(
                        &format!("/api/edges/{}/runtime/write", url_encode(e)),
                        &serde_json::json!({ "name": name, "value": encoded }),
                    )?
                }
                None => {
                    let encoded = parse_value(client, &name, &value)?;
                    client.post(
                        &format!("/api/runtime/variables/{}", url_encode(&name)),
                        &serde_json::json!({ "value": encoded }),
                    )?
                }
            };
            print_json(&resp)
        }
    }
}

/// pause/resume/step against the local runtime or an edge proxy.
fn post_op(
    client: &Client,
    edge: Option<&str>,
    op: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value> {
    match edge {
        Some(e) => client.post(&format!("/api/edges/{}/runtime/{op}", url_encode(e)), body),
        None => client.post(&format!("/api/runtime/{op}"), body),
    }
}

/// Keep only the named vars (comma-separated) in a snapshot payload.
fn filter_snapshot(snap: &serde_json::Value, vars: &str) -> serde_json::Value {
    let wanted: Vec<&str> = vars
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let Some(arr) = snap.get("vars").and_then(|v| v.as_array()) else {
        return snap.clone();
    };
    let filtered: Vec<serde_json::Value> = arr
        .iter()
        .filter(|v| {
            v.get("name")
                .and_then(|n| n.as_str())
                .map(|n| wanted.contains(&n))
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    let mut out = snap.clone();
    if let Some(obj) = out.as_object_mut() {
        obj.insert("vars".into(), serde_json::Value::Array(filtered));
    }
    out
}

/// Convert a human-typed value into the i32 the runtime wire protocol
/// expects, type-aware via the runtime's snapshot.
///
/// Why: the bridge stores all variables — BOOL, INT, REAL, … — in
/// 32-bit slots and the force/write endpoint takes a raw `i32`. For
/// REAL the i32 is the IEEE-754 bit pattern of the float, NOT the
/// integer value. This helper does the conversion so humans (and
/// agents) can use natural notation.
pub(crate) fn parse_value(client: &Client, name: &str, raw: &str) -> Result<i32> {
    let var_type = snapshot_var_type(client, name).unwrap_or_default();
    pack_value(name, var_type.as_deref(), raw)
}

/// Resolve an edge variable's type from the edge runtime's `/status`
/// (last snapshot, which carries per-variable `type_name`).
fn edge_var_type(client: &Client, edge: &str, name: &str) -> Option<String> {
    let status = client
        .get(&format!("/api/edges/{}/status", url_encode(edge)))
        .ok()?;
    let vars = status.get("last_snapshot")?.get("vars")?.as_array()?;
    for v in vars {
        if v.get("name").and_then(|n| n.as_str()) == Some(name) {
            return v
                .get("type_name")
                .and_then(|t| t.as_str())
                .map(String::from);
        }
    }
    None
}

/// Bit-pack a human value string into the i32 force/write wire, given the
/// variable's IEC `var_type` (None = unknown → guess from value format).
fn pack_value(name: &str, var_type: Option<&str>, raw: &str) -> Result<i32> {
    // BOOL shortcuts. Case-insensitive because TRUE/FALSE are the IEC
    // canonical form but agents type either.
    match raw.to_ascii_lowercase().as_str() {
        "true" => return Ok(1),
        "false" => return Ok(0),
        _ => {}
    }

    match var_type {
        Some("BOOL") => {
            let n: i32 = raw.parse().with_context(|| {
                format!("value `{raw}` doesn't fit BOOL (expected TRUE/FALSE/1/0)")
            })?;
            Ok(if n != 0 { 1 } else { 0 })
        }
        Some("REAL") => {
            let f: f32 = raw
                .parse()
                .with_context(|| format!("value `{raw}` doesn't parse as REAL (32-bit float)"))?;
            Ok(f.to_bits() as i32)
        }
        Some("LREAL") => {
            anyhow::bail!(
                "LREAL (64-bit float) doesn't fit the 32-bit force wire — \
                 use a REAL variable, or write the low 32 bits manually"
            )
        }
        Some(int_type)
            if matches!(
                int_type,
                "INT" | "DINT" | "SINT" | "UINT" | "UDINT" | "USINT" | "BYTE" | "WORD" | "DWORD"
            ) =>
        {
            let n: i64 = raw.parse().with_context(|| {
                format!("value `{raw}` doesn't parse as integer for {int_type}")
            })?;
            Ok(n as i32)
        }
        Some(other) => {
            anyhow::bail!("don't know how to encode value `{raw}` for type {other} (yet)")
        }
        None => {
            // No type info — guess from format and warn loudly.
            if raw.contains('.') || raw.contains('e') || raw.contains('E') {
                let f: f32 = raw.parse().with_context(|| {
                    format!("value `{raw}` looks like a float but doesn't parse as f32")
                })?;
                eprintln!(
                    "note: runtime didn't expose `{name}`'s type — guessed REAL from value format"
                );
                Ok(f.to_bits() as i32)
            } else {
                let n: i32 = raw.parse().with_context(|| {
                    format!("value `{raw}` doesn't parse as i32; if you meant REAL, use `{raw}.0`")
                })?;
                eprintln!("note: runtime didn't expose `{name}`'s type — assumed INT family");
                Ok(n)
            }
        }
    }
}

/// Best-effort variable type lookup via `/api/runtime/snapshot`.
fn snapshot_var_type(client: &Client, name: &str) -> Result<Option<String>> {
    let snap = match client.get("/api/runtime/snapshot") {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let vars = match snap.get("vars").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return Ok(None),
    };
    for v in vars {
        if v.get("name").and_then(|n| n.as_str()) == Some(name) {
            return Ok(v
                .get("type_name")
                .and_then(|t| t.as_str())
                .map(String::from));
        }
    }
    Ok(None)
}
