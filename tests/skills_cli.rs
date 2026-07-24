use serde_json::Value;
use std::fs;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn vaxis(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vaxis"))
        .args(args)
        .output()
        .expect("vaxis command should run")
}

fn json_stdout(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout should be JSON: {error}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn temporary_project() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "vaxis-skills-cli-{}-{nonce}",
        std::process::id()
    ))
}

#[test]
fn list_and_path_have_stable_json_output() {
    let list = vaxis(&["skills", "list", "--json"]);
    assert!(list.status.success());
    let list_json = json_stdout(&list);
    assert_eq!(list_json[0]["name"], "core");
    let expected_source = format!("embedded:v{}/core", env!("CARGO_PKG_VERSION"));
    assert_eq!(list_json[0]["source"], expected_source);

    let path = vaxis(&["skills", "path", "core", "--json"]);
    assert!(path.status.success());
    let path_json = json_stdout(&path);
    assert_eq!(path_json["name"], "core");
    assert_eq!(path_json["source"], expected_source);
}

#[test]
fn get_and_preview_have_stable_json_output() {
    let expected_source = format!("embedded:v{}/core", env!("CARGO_PKG_VERSION"));
    let expected_content = include_str!("../skill-data/core/SKILL.md");

    for action in ["get", "preview"] {
        let output = vaxis(&["skills", action, "core", "--json"]);
        assert!(output.status.success());
        let value = json_stdout(&output);
        assert_eq!(value["name"], "core");
        assert_eq!(value["source"], expected_source);
        assert_eq!(value["content"], expected_content);
    }
}

#[test]
fn get_without_json_prints_exact_core_skill() {
    let output = vaxis(&["skills", "get", "core"]);
    assert!(output.status.success());
    assert_eq!(output.stdout, include_bytes!("../skill-data/core/SKILL.md"));
}

#[test]
fn json_errors_are_structured_and_never_prompt() {
    let unknown = vaxis(&["skills", "get", "missing", "--json"]);
    assert_eq!(unknown.status.code(), Some(1));
    let unknown_json = json_stdout(&unknown);
    assert_eq!(unknown_json["error"], "unknown_skill");

    let missing_selection = vaxis(&["install", "--skills", "--json"]);
    assert_eq!(missing_selection.status.code(), Some(1));
    let selection_json = json_stdout(&missing_selection);
    assert_eq!(selection_json["error"], "invalid_arguments");
    assert!(selection_json["message"]
        .as_str()
        .unwrap()
        .contains("--agent"));

    let missing_target = vaxis(&["install", "--json"]);
    assert_eq!(missing_target.status.code(), Some(1));
    assert_eq!(json_stdout(&missing_target)["error"], "invalid_arguments");
}

#[test]
fn project_install_reports_json_result() {
    let project = temporary_project();
    fs::create_dir_all(&project).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_vaxis"))
        .current_dir(&project)
        .args([
            "install",
            "--skills",
            "--agent",
            "codex",
            "--project",
            "--yes",
            "--json",
        ])
        .output()
        .expect("vaxis install should run");

    assert!(output.status.success());
    let result = json_stdout(&output);
    assert_eq!(result[0]["agent"], "codex");
    assert_eq!(result[0]["status"], "installed");
    assert!(project
        .join(".agents/skills/vaxis/SKILL.md")
        .is_file());
    fs::remove_dir_all(project).unwrap();
}
