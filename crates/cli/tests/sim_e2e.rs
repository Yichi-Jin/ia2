//! True end-to-end test of `cs sim run`: a REAL `server` binary, the
//! bundled `examples/sim_smoke` project, and the cs binary as a
//! subprocess — the exact loop an agent runs to prove generated logic.
//!
//! Skips (with a loud note) when `target/debug/server` hasn't been
//! built yet: `cargo build -p server` first, or run the workspace
//! quality gate which builds everything.

use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command as StdCommand, Stdio};

use assert_cmd::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn server_binary() -> Option<PathBuf> {
    let p = repo_root().join("target/debug/server");
    p.exists().then_some(p)
}

struct ServerGuard(Child);

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn cs(server: &str) -> Command {
    let mut c = Command::cargo_bin("cs").expect("cs binary");
    c.arg("--server").arg(server);
    c
}

#[test]
fn sim_run_proves_and_refutes_against_a_real_server() {
    let Some(server_bin) = server_binary() else {
        eprintln!("SKIP sim_e2e: target/debug/server not built (cargo build -p server)");
        return;
    };

    // Free port: bind-then-drop; the server grabs it a moment later.
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    drop(l);
    let base = format!("http://127.0.0.1:{port}");

    // Copy the example project to a tempdir so runs never dirty the repo.
    let tmp = tempfile::tempdir().unwrap();
    let proj = tmp.path().join("sim_smoke");
    copy_dir(&repo_root().join("examples/sim_smoke"), &proj);

    let child = StdCommand::new(&server_bin)
        .arg("--bind")
        .arg(format!("127.0.0.1:{port}"))
        // No demo slave — keeps the test from fighting over port 5502
        // with a dev server on the same machine.
        .arg("--demo-modbus-addr")
        .arg("")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn server");
    let _guard = ServerGuard(child);

    // Wait for /health (up to ~5 s).
    let mut up = false;
    for _ in 0..50 {
        let ok = cs(&base)
            .arg("api")
            .arg("GET")
            .arg("/health")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            up = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(up, "server did not come up on {base}");

    cs(&base)
        .arg("api")
        .arg("POST")
        .arg("/api/projects/open")
        .arg("--from")
        .arg("-")
        .write_stdin(format!("{{\"path\":\"{}\"}}", proj.display()))
        .assert()
        .success();

    // The bundled scenario must pass end to end (fills tank, alarm
    // raises, no overflow) — this is the agent's self-verification loop.
    cs(&base)
        .arg("sim")
        .arg("run")
        .arg(proj.join("scenarios/fill.toml"))
        .assert()
        .success()
        .stderr(predicates::str::contains("scenario passed"));

    // And a wrong expectation must FAIL with exit 1 and name the step.
    let bad = tmp.path().join("bad.toml");
    std::fs::write(
        &bad,
        "[[steps]]\nexpect = { var = \"level\", op = \"lt\", value = -1.0, within_ms = 600 }\n",
    )
    .unwrap();
    cs(&base)
        .arg("sim")
        .arg("run")
        .arg(&bad)
        .assert()
        .code(1)
        .stderr(predicates::str::contains("scenario FAILED"));
}

fn copy_dir(src: &PathBuf, dst: &PathBuf) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let to = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to);
        } else {
            std::fs::copy(entry.path(), &to).unwrap();
        }
    }
}
