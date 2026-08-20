use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Output};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::tempdir;

fn normalized_hash(value: &str) -> String {
    let normalized = format!("{}\n", value.replace("\r\n", "\n").trim_end());
    format!("{:x}", Sha256::digest(normalized.as_bytes()))
}

fn config_home(token: &str) -> tempfile::TempDir {
    let home = tempdir().unwrap();
    let config_dir = home.path().join("vaxis");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("config.toml"), format!(
        "auth_url = 'http://unused'\n[user]\nname = 'Test'\nemail = 'test@example.com'\ntoken = '{token}'\n",
    )).unwrap();
    home
}

fn run(args: &[&str], cwd: &std::path::Path, config: &std::path::Path, url: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vaxis"));
    command.args(args).current_dir(cwd)
        .env("APPDATA", config)
        .env("XDG_CONFIG_HOME", config);
    if let Some(url) = url { command.env("VAXIS_AUTH_URL", url); }
    command.output().unwrap()
}

fn one_response(body: String) -> (String, thread::JoinHandle<()>) {
    response_with_status("200 OK", body)
}

fn response_with_status(status: &'static str, body: String) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let started = Instant::now();
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock && started.elapsed() < Duration::from_secs(5) => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("test server did not receive a request: {error}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut request = Vec::new();
        let mut buffer = [0u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let count = stream.read(&mut buffer).unwrap();
            if count == 0 { break; }
            request.extend_from_slice(&buffer[..count]);
        }
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(), body,
        );
        stream.write_all(response.as_bytes()).unwrap();
    });
    (url, handle)
}

#[test]
fn status_reports_remote_missing_and_check_exits_two() {
    for check in [false, true] {
        let repo = tempdir().unwrap();
        Command::new("git").args(["init", "--quiet"]).current_dir(repo.path()).status().unwrap();
        let architecture_dir = repo.path().join("architecture");
        fs::create_dir_all(&architecture_dir).unwrap();
        fs::write(architecture_dir.join("architecture.vaxis.mmd"), "flowchart TB\n  a[A]\n").unwrap();
        fs::write(architecture_dir.join("vaxis.yaml"),
            "schema_version: 2\nroot_diagram_id: deleted\nfile: architecture.vaxis.mmd\nsynced_hash: baseline\n",
        ).unwrap();
        let (url, server) = response_with_status("404 Not Found", "{}".into());
        let config = config_home("token");
        let mut args = vec!["diagrams", "sync", "status", "--json"];
        if check { args.push("--check"); }

        let output = run(&args, repo.path(), config.path(), Some(&url));
        server.join().unwrap();
        assert_eq!(output.status.code(), Some(if check { 2 } else { 0 }));
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["state"], "remote_missing");
    }
}

#[test]
fn sync_preserves_session_expired_for_unauthorized_requests() {
    let repo = tempdir().unwrap();
    Command::new("git").args(["init", "--quiet"]).current_dir(repo.path()).status().unwrap();
    let (url, server) = response_with_status("401 Unauthorized", "{}".into());
    let config = config_home("expired");

    let output = run(
        &["diagrams", "sync", "init", "root", "--json"],
        repo.path(), config.path(), Some(&url),
    );
    server.join().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "session_expired");
}

#[test]
fn json_sync_error_is_stdout_only() {
    let repo = tempdir().unwrap();
    Command::new("git").args(["init", "--quiet"]).current_dir(repo.path()).status().unwrap();
    let config = config_home("token");
    let output = run(
        &["diagrams", "sync", "init", "root", "--dir", "../escape", "--json"],
        repo.path(), config.path(), None,
    );
    assert_eq!(output.status.code(), Some(1));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "invalid_sync_directory");
    assert!(output.stderr.is_empty());
}

#[test]
fn status_check_exits_two_for_remote_drift() {
    let repo = tempdir().unwrap();
    Command::new("git").args(["init", "--quiet"]).current_dir(repo.path()).status().unwrap();
    let architecture_dir = repo.path().join("architecture");
    fs::create_dir_all(&architecture_dir).unwrap();
    let local = "flowchart TB\n  local[Local]\n";
    fs::write(architecture_dir.join("architecture.vaxis.mmd"), local).unwrap();
    fs::write(architecture_dir.join("vaxis.yaml"), format!(
        "schema_version: 2\nroot_diagram_id: root\nfile: architecture.vaxis.mmd\nsynced_hash: {}\n",
        normalized_hash(local),
    )).unwrap();
    let remote = "flowchart TB\n  remote[Remote]\n";
    let body = serde_json::json!({"root_id":"root","diagram_count":1,"mermaid":remote}).to_string();
    let (url, server) = one_response(body);
    let config = config_home("token");

    let output = run(
        &["diagrams", "sync", "status", "--check", "--json"],
        repo.path(), config.path(), Some(&url),
    );
    server.join().unwrap();
    assert_eq!(output.status.code(), Some(2), "stdout={} stderr={}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["state"], "remote_changed");
}

#[test]
fn status_reports_invalid_utf8_and_malformed_mermaid_as_invalid_local() {
    for bytes in [vec![0xff, 0xfe], b"this is not Mermaid\n".to_vec()] {
        let repo = tempdir().unwrap();
        Command::new("git").args(["init", "--quiet"]).current_dir(repo.path()).status().unwrap();
        let architecture_dir = repo.path().join("architecture");
        fs::create_dir_all(&architecture_dir).unwrap();
        fs::write(architecture_dir.join("architecture.vaxis.mmd"), bytes).unwrap();
        fs::write(architecture_dir.join("vaxis.yaml"),
            "schema_version: 2\nroot_diagram_id: root\nfile: architecture.vaxis.mmd\nsynced_hash: baseline\n",
        ).unwrap();
        let body = serde_json::json!({
            "root_id":"root", "diagram_count":1, "mermaid":"flowchart TB\n  remote[Remote]",
        }).to_string();
        let (url, server) = one_response(body);
        let config = config_home("token");

        let output = run(
            &["diagrams", "sync", "status", "--json"],
            repo.path(), config.path(), Some(&url),
        );
        server.join().unwrap();
        assert_eq!(output.status.code(), Some(0), "stdout={} stderr={}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["state"], "invalid_local");
    }
}

#[test]
fn pull_replaces_existing_architecture_and_manifest_together() {
    let repo = tempdir().unwrap();
    Command::new("git").args(["init", "--quiet"]).current_dir(repo.path()).status().unwrap();
    let architecture_dir = repo.path().join("architecture");
    fs::create_dir_all(&architecture_dir).unwrap();
    let local = "flowchart TB\n  local[Local]\n";
    fs::write(architecture_dir.join("architecture.vaxis.mmd"), local).unwrap();
    fs::write(architecture_dir.join("vaxis.yaml"), format!(
        "schema_version: 2\nroot_diagram_id: root\nfile: architecture.vaxis.mmd\nsynced_hash: {}\n",
        normalized_hash(local),
    )).unwrap();
    let remote = "flowchart TB\n  remote[Remote]\n";
    let body = serde_json::json!({"root_id":"root","diagram_count":1,"mermaid":remote,"revision":7}).to_string();
    let (url, server) = one_response(body);
    let config = config_home("token");

    let output = run(
        &["diagrams", "sync", "pull", "--json"],
        repo.path(), config.path(), Some(&url),
    );
    server.join().unwrap();
    assert_eq!(output.status.code(), Some(0), "stdout={} stderr={}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert_eq!(fs::read_to_string(architecture_dir.join("architecture.vaxis.mmd")).unwrap(), remote);
    let manifest = fs::read_to_string(architecture_dir.join("vaxis.yaml")).unwrap();
    assert!(manifest.contains(&normalized_hash(remote)));
    assert!(manifest.contains("remote_revision: 7"));
}

#[test]
fn pull_leaves_local_only_changes_untouched() {
    let repo = tempdir().unwrap();
    Command::new("git").args(["init", "--quiet"]).current_dir(repo.path()).status().unwrap();
    let architecture_dir = repo.path().join("architecture");
    fs::create_dir_all(&architecture_dir).unwrap();
    let baseline = "flowchart TB\n  baseline[Baseline]\n";
    let local = "flowchart TB\n  local[Local Edit]\n";
    fs::write(architecture_dir.join("architecture.vaxis.mmd"), local).unwrap();
    fs::write(architecture_dir.join("vaxis.yaml"), format!(
        "schema_version: 2\nroot_diagram_id: root\nfile: architecture.vaxis.mmd\nsynced_hash: {}\n",
        normalized_hash(baseline),
    )).unwrap();
    let body = serde_json::json!({"root_id":"root","diagram_count":1,"mermaid":baseline}).to_string();
    let (url, server) = one_response(body);
    let config = config_home("token");

    let output = run(
        &["diagrams", "sync", "pull", "--json"],
        repo.path(), config.path(), Some(&url),
    );
    server.join().unwrap();
    assert_eq!(output.status.code(), Some(0));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["updated"], false);
    assert_eq!(fs::read_to_string(architecture_dir.join("architecture.vaxis.mmd")).unwrap(), local);
}

#[test]
fn init_rejects_a_child_id_when_server_reports_the_actual_root() {
    let repo = tempdir().unwrap();
    Command::new("git").args(["init", "--quiet"]).current_dir(repo.path()).status().unwrap();
    let body = serde_json::json!({
        "root_id":"actual-root", "diagram_count":2, "mermaid":"flowchart TB\n  root[Root]",
    }).to_string();
    let (url, server) = one_response(body);
    let config = config_home("token");

    let output = run(
        &["diagrams", "sync", "init", "child", "--json"],
        repo.path(), config.path(), Some(&url),
    );
    server.join().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "root_diagram_required");
    assert!(!repo.path().join("architecture").exists());
}

#[test]
fn pull_stops_on_invalid_local_content_instead_of_treating_it_as_missing() {
    let repo = tempdir().unwrap();
    Command::new("git").args(["init", "--quiet"]).current_dir(repo.path()).status().unwrap();
    let architecture_dir = repo.path().join("architecture");
    fs::create_dir_all(&architecture_dir).unwrap();
    let file = architecture_dir.join("architecture.vaxis.mmd");
    fs::write(&file, [0xff, 0xfe]).unwrap();
    fs::write(architecture_dir.join("vaxis.yaml"),
        "schema_version: 2\nroot_diagram_id: root\nfile: architecture.vaxis.mmd\nsynced_hash: baseline\n",
    ).unwrap();
    let body = serde_json::json!({
        "root_id":"root", "diagram_count":1, "mermaid":"flowchart TB\n  remote[Remote]",
    }).to_string();
    let (url, server) = one_response(body);
    let config = config_home("token");

    let output = run(
        &["diagrams", "sync", "pull", "--json"],
        repo.path(), config.path(), Some(&url),
    );
    server.join().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "invalid_local");
    assert_eq!(fs::read(&file).unwrap(), [0xff, 0xfe]);
}

#[cfg(unix)]
#[test]
fn status_rejects_a_sync_directory_symlinked_outside_the_repository() {
    use std::os::unix::fs::symlink;
    let repo = tempdir().unwrap();
    let outside = tempdir().unwrap();
    Command::new("git").args(["init", "--quiet"]).current_dir(repo.path()).status().unwrap();
    symlink(outside.path(), repo.path().join("architecture")).unwrap();
    let config = config_home("token");

    let output = run(
        &["diagrams", "sync", "status", "--json"],
        repo.path(), config.path(), None,
    );
    assert_eq!(output.status.code(), Some(1));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "invalid_sync_directory");
}
