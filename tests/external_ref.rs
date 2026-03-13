//! Integration tests for external $ref resolution.

use yaml_schema::Engine;
use yaml_schema::loader;

#[test]
fn test_external_ref_resolves_and_validates() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let dir = temp.path();

    let common_yaml = r##"
$defs:
  Id:
    type: string
"##;
    let common_path = dir.join("common.yaml");
    std::fs::write(&common_path, common_yaml).expect("write common.yaml");

    let schema_yaml = r##"
type: object
properties:
  id:
    $ref: "./common.yaml#/$defs/Id"
"##;
    let schema_path = dir.join("schema.yaml");
    std::fs::write(&schema_path, schema_yaml).expect("write schema.yaml");

    let schema_str = schema_path.to_str().expect("path to str");
    let root_schema = loader::load_file(schema_str).expect("load schema");

    let context = Engine::evaluate(&root_schema, r##"id: "abc-123""##, false).expect("evaluate");
    assert!(
        !context.has_errors(),
        "Expected no errors: {:?}",
        context.errors.borrow()
    );

    let context = Engine::evaluate(&root_schema, "id: 42", false).expect("evaluate");
    assert!(context.has_errors(), "Expected validation error for id: 42");
}

#[test]
fn test_absolute_file_uri_ref_resolves_and_validates() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let dir = temp.path();

    let common_yaml = r##"
$defs:
  Name:
    type: string
"##;
    let common_path = dir.join("common.yaml");
    std::fs::write(&common_path, common_yaml).expect("write common.yaml");

    let common_url = url::Url::from_file_path(common_path.canonicalize().expect("canonicalize"))
        .expect("file url");

    let schema_yaml = format!(
        r##"
type: object
properties:
  name:
    $ref: "{}#/$defs/Name"
"##,
        common_url
    );

    let root_schema = loader::load_from_str(&schema_yaml).expect("load schema");

    let context = Engine::evaluate(&root_schema, r##"name: "Alice""##, false).expect("evaluate");
    assert!(
        !context.has_errors(),
        "Expected no errors: {:?}",
        context.errors.borrow()
    );

    let context = Engine::evaluate(&root_schema, "name: 123", false).expect("evaluate");
    assert!(
        context.has_errors(),
        "Expected validation error for name: 123"
    );
}

#[test]
fn test_absolute_file_uri_ref_without_fragment() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let dir = temp.path();

    let target_yaml = r##"
type: integer
"##;
    let target_path = dir.join("int_schema.yaml");
    std::fs::write(&target_path, target_yaml).expect("write int_schema.yaml");

    let target_url = url::Url::from_file_path(target_path.canonicalize().expect("canonicalize"))
        .expect("file url");

    let schema_yaml = format!(
        r##"
type: object
properties:
  count:
    $ref: "{}"
"##,
        target_url
    );

    let root_schema = loader::load_from_str(&schema_yaml).expect("load schema");

    let context = Engine::evaluate(&root_schema, "count: 42", false).expect("evaluate");
    assert!(
        !context.has_errors(),
        "Expected no errors: {:?}",
        context.errors.borrow()
    );

    let context =
        Engine::evaluate(&root_schema, r##"count: "not a number""##, false).expect("evaluate");
    assert!(
        context.has_errors(),
        "Expected validation error for non-integer"
    );
}

#[test]
fn test_absolute_uri_ref_caches_by_id() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let dir = temp.path();

    let common_yaml = r##"
$id: "https://example.com/common"
$defs:
  Color:
    type: string
    enum:
      - red
      - green
      - blue
"##;
    let common_path = dir.join("common.yaml");
    std::fs::write(&common_path, common_yaml).expect("write common.yaml");

    let common_url = url::Url::from_file_path(common_path.canonicalize().expect("canonicalize"))
        .expect("file url");

    let schema_yaml = format!(
        r##"
type: object
properties:
  primary:
    $ref: "{}#/$defs/Color"
  secondary:
    $ref: "{}#/$defs/Color"
"##,
        common_url, common_url
    );

    let root_schema = loader::load_from_str(&schema_yaml).expect("load schema");

    let context =
        Engine::evaluate(&root_schema, "primary: red\nsecondary: blue", false).expect("evaluate");
    assert!(
        !context.has_errors(),
        "Expected no errors: {:?}",
        context.errors.borrow()
    );

    let context = Engine::evaluate(&root_schema, "primary: purple\nsecondary: blue", false)
        .expect("evaluate");
    assert!(
        context.has_errors(),
        "Expected validation error for invalid color"
    );
}
