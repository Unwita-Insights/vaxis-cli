use serde_json::Value;
use std::process::{Command, Output};

fn vaxis(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vaxis"))
        .args(args)
        .output()
        .expect("vaxis command should run")
}

fn assert_invalid_arguments(output: Output, expected_message: &str) {
    assert_eq!(output.status.code(), Some(1));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout should be JSON: {error}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(value["error"], "invalid_arguments");
    assert!(value["message"]
        .as_str()
        .unwrap()
        .contains(expected_message));
}

#[test]
fn delete_json_requires_id_without_prompting() {
    assert_invalid_arguments(
        vaxis(&[
            "diagrams",
            "delete",
            "--app-id",
            "app-id",
            "--force",
            "--json",
        ]),
        "diagram ID",
    );
}

#[test]
fn delete_json_requires_force_without_prompting() {
    assert_invalid_arguments(
        vaxis(&["diagrams", "delete", "diagram-id", "--json"]),
        "--force",
    );
}
