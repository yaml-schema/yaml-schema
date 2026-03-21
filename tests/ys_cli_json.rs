use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

#[test]
fn validation_errors_stdout_is_json_array_with_multiple_entries() {
    let dir = tempdir().expect("tempdir");
    let schema_path = dir.path().join("schema.yaml");
    let instance_path = dir.path().join("instance.yaml");
    fs::write(
        &schema_path,
        r"type: object
properties:
  a:
    type: string
  b:
    type: string
",
    )
    .expect("write schema");
    fs::write(
        &instance_path,
        r"a: 1
b: 2
",
    )
    .expect("write instance");

    let output = Command::cargo_bin("ys")
        .expect("ys binary")
        .args([
            "--json",
            "-f",
            schema_path.to_str().expect("utf8 path"),
            instance_path.to_str().expect("utf8 path"),
        ])
        .output()
        .expect("run ys");

    assert!(!output.status.success(), "ys should fail validation");
    let v: Value = serde_json::from_slice(&output.stdout).expect("stdout is JSON");
    let arr = v.as_array().expect("stdout is JSON array");
    assert!(
        arr.len() > 1,
        "expected multiple validation errors, got {}",
        arr.len()
    );
    for entry in arr {
        assert!(entry.get("path").is_some());
        assert!(entry.get("error").is_some());
    }
}

#[test]
fn validation_without_f_uses_dollar_schema_and_json_output() {
    let dir = tempdir().expect("tempdir");
    let schema_path = dir.path().join("schema.yaml");
    let instance_path = dir.path().join("instance.yaml");
    fs::write(
        &schema_path,
        r"type: object
properties:
  a:
    type: string
  b:
    type: string
",
    )
    .expect("write schema");
    fs::write(
        &instance_path,
        r"$schema: schema.yaml
a: 1
b: 2
",
    )
    .expect("write instance");

    let output = Command::cargo_bin("ys")
        .expect("ys binary")
        .args(["--json", instance_path.to_str().expect("utf8 path")])
        .output()
        .expect("run ys");

    assert!(!output.status.success(), "ys should fail validation");
    let v: Value = serde_json::from_slice(&output.stdout).expect("stdout is JSON");
    let arr = v.as_array().expect("stdout is JSON array");
    assert!(
        arr.len() > 1,
        "expected multiple validation errors, got {}",
        arr.len()
    );
    for entry in arr {
        assert!(entry.get("path").is_some());
        assert!(entry.get("error").is_some());
    }
}
