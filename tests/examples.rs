//! Integration tests that validate the schemas under `examples/` against
//! their associated instance files, using the actual `ys` binary.

use assert_cmd::Command;
use std::fs;
use std::path::Path;

/// Validates every `*.yaml`/`*.yml` file directly under `dir` against `schema_path`
/// by invoking the actual `ys` binary, asserting each one succeeds.
fn validate_all(schema_path: &str, dir: &Path) {
    let mut fixtures: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {}", dir.display(), e))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("yaml" | "yml")))
        .collect();
    fixtures.sort();
    assert!(
        !fixtures.is_empty(),
        "no *.yaml/*.yml files found in {}",
        dir.display()
    );

    for fixture in fixtures {
        let output = Command::cargo_bin("ys")
            .expect("ys binary")
            .args(["-f", schema_path, fixture.to_str().expect("utf8 path")])
            .output()
            .expect("run ys");
        assert!(
            output.status.success(),
            "{} failed to validate against {}:\n{}",
            fixture.display(),
            schema_path,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn kustomization_fixtures_validate() {
    validate_all(
        "examples/kustomization/schema.yaml",
        Path::new("examples/kustomization/fixtures"),
    );
}

#[test]
fn github_workflow_files_validate() {
    validate_all(
        "examples/github-workflow/schema.yaml",
        Path::new(".github/workflows"),
    );
}
