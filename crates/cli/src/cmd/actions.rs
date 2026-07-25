//! Domain actions that stay porcelain: HMI incremental authoring
//! (`cs hmi op` / `generate`) and FB-library vendoring (`cs library
//! import`). Everything CRUD-shaped moved to the `ls/get/set/rm`
//! quartet; these three carry semantics a generic verb can't express.

use anyhow::Result;

use crate::http::{print_json, read_json_blob, url_encode, Client};

/// Apply a batch of structured edits to a screen. Accepts both
/// `{"ops":[...]}` and a bare `[...]` — agents hand-writing a single
/// op shouldn't need the wrapper.
pub(crate) fn cmd_hmi_op(client: &Client, path: &str, from: &str) -> Result<i32> {
    let raw = read_json_blob(from)?;
    let body = if raw.is_array() {
        serde_json::json!({ "ops": raw })
    } else {
        raw
    };
    let resp = client.post(&format!("/api/hmi/{}/ops", url_encode(path)), &body)?;
    print_json(&resp)
}

/// Deterministic first-pass screen from project truth. 409 if the
/// screen exists — pass `--force` to regenerate.
pub(crate) fn cmd_hmi_generate(
    client: &Client,
    path: &str,
    force: bool,
    title: Option<&str>,
) -> Result<i32> {
    let body = serde_json::json!({ "force": force, "title": title });
    let resp = client.post(&format!("/api/hmi/{}/generate", url_encode(path)), &body)?;
    print_json(&resp)
}

/// Vendor library blocks into `pous/lib/<name>/`. Omit `blocks` to
/// import the whole library; re-importing overwrites (the update path).
pub(crate) fn cmd_library_import(client: &Client, library: &str, blocks: &[String]) -> Result<i32> {
    let body = serde_json::json!({ "library": library, "blocks": blocks });
    let resp = client.post("/api/library/import", &body)?;
    print_json(&resp)
}
