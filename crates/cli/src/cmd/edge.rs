//! `cs deploy` / `cs probe` — the two edge orchestration verbs. Edge
//! CRUD is `cs set/get/rm edges/<name>`; edge sub-reads are
//! `cs get edges/<name>/{probe,status,logs,scan,system}`; attach /
//! detach are `cs api POST /api/edges/<name>/attach|detach`.

use anyhow::Result;

use crate::http::{url_encode, Body, Client};

pub(crate) fn cmd_deploy(client: &Client, name: &str, json: bool) -> Result<i32> {
    // The server's /api/edges/{name}/deploy route owns the SSH+tar
    // dance. Bigger timeout than the default (30 s) because the
    // tar+ssh round-trip can take minutes on a slow link.
    let value = client.request(
        "POST",
        &format!("/api/edges/{}/deploy", url_encode(name)),
        Body::Json(&serde_json::json!({})),
        Some(std::time::Duration::from_secs(600)),
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        // Human-readable: the streamed deploy log, then the verdict.
        let version = value.get("version").and_then(|v| v.as_str()).unwrap_or("?");
        let log = value.get("log").and_then(|v| v.as_str()).unwrap_or("");
        if !log.is_empty() {
            eprintln!("{log}");
        }
        let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if ok {
            eprintln!("✓ deployed to '{name}' as version {version}");
        } else {
            eprintln!("✗ deploy to '{name}' FAILED — read the log above");
        }
        if let Some(w) = value.get("warning").and_then(|v| v.as_str()) {
            eprintln!("⚠ {w}");
        }
    }
    // ok=false means the script ran but exited non-zero (remote failure).
    let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    Ok(if ok { 0 } else { 1 })
}

pub(crate) fn cmd_probe(client: &Client, name: &str, json: bool) -> Result<i32> {
    let value = client.get(&format!("/api/edges/{}/probe", url_encode(name)))?;
    let reachable = value
        .get("reachable")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if json {
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else if reachable {
        let scans = value
            .get("scan_count")
            .and_then(|v| v.as_u64())
            .map(|n| n.to_string())
            .unwrap_or_else(|| "?".into());
        let uptime = value
            .get("uptime_secs")
            .and_then(|v| v.as_u64())
            .map(|n| format!("{n}s"))
            .unwrap_or_else(|| "?".into());
        let version = value
            .get("runtime_version")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        println!("✓ {name} reachable · v{version} · {scans} scans · up {uptime}");
    } else {
        let err = value
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unreachable");
        eprintln!("✗ {name}: {err}");
    }
    Ok(if reachable { 0 } else { 1 })
}
