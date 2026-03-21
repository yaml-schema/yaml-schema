use assert_cmd::Command;
use cucumber::World;
use cucumber::gherkin::Scenario;
use cucumber::gherkin::Step;
use cucumber::given;
use cucumber::then;
use cucumber::when;
use log::debug;
use log::error;
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;
use yaml_schema::Engine;
use yaml_schema::Result;
use yaml_schema::RootSchema;
use yaml_schema::loader;
use yaml_schema::validation::ValidationError;

#[derive(Debug, Default)]
struct CommandOutput {
    exit_code: i32,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Default, World)]
pub struct FeaturesWorld {
    root_schema: Option<RootSchema>,
    yaml_schema_error: Option<yaml_schema::Error>,
    errors: Option<Rc<RefCell<Vec<ValidationError>>>>,
    command_output: Option<CommandOutput>,
}

#[given(regex = "a YAML schema:")]
async fn a_yaml_schema(world: &mut FeaturesWorld, step: &Step) {
    let schema = step.docstring().expect("Expected a docstring");
    debug!("schema: {schema:?}");
    match loader::load_from_str(schema) {
        Ok(root_schema) => world.root_schema = Some(root_schema),
        Err(e) => {
            error!("Error: {e:?}");
            world.yaml_schema_error = Some(e);
        }
    }
}

fn evaluate(world: &mut FeaturesWorld, s: &str) -> Result<bool> {
    let context = Engine::evaluate(
        world.root_schema.as_ref().expect("No root schema"),
        s,
        false,
    )?;
    world.errors = Some(context.errors.clone());
    for error in context.errors.borrow().iter() {
        println!("{error}");
    }
    Ok(!context.has_errors())
}

#[then(regex = "it should accept:")]
async fn it_should_accept(world: &mut FeaturesWorld, step: &Step) {
    let raw_input = step.docstring().expect("Expected a docstring");
    let input_without_beginning_newline =
        raw_input.strip_prefix('\n').expect("Expected a docstring");
    let result = evaluate(world, input_without_beginning_newline).expect("Evaluation failed");
    assert!(result);
}

#[then(regex = "it should NOT accept:")]
async fn it_should_not_accept(world: &mut FeaturesWorld, step: &Step) {
    let raw_input = step.docstring().expect("Expected a docstring");
    let input_without_beginning_newline =
        raw_input.strip_prefix('\n').expect("Expected a docstring");
    let result = evaluate(world, input_without_beginning_newline).expect("Evaluation failed");
    assert!(!result);
}

#[then(expr = "the error message should be {string}")]
fn the_error_message_should_be(world: &mut FeaturesWorld, expected_error_message: String) {
    let errors = world
        .errors
        .as_ref()
        .expect("Unable to borrow errors")
        .borrow();
    match errors.first() {
        Some(error) => {
            let actual_error_message = error.to_string();
            assert_eq!(actual_error_message, expected_error_message);
        }
        None => {
            panic!("Expected an error message, but there was no error!");
        }
    }
}

#[then(expr = "it should fail with {string}")]
async fn it_should_fail_with(world: &mut FeaturesWorld, expected_error_message: String) {
    if let Some(yaml_schema_error) = world.yaml_schema_error.as_ref() {
        assert_eq!(expected_error_message, yaml_schema_error.to_string());
    } else {
        panic!("Expected an error message, but there was no error!");
    }
}

#[when(regex = "the following command is run:")]
async fn the_following_command_is_run(world: &mut FeaturesWorld, step: &Step) {
    let docstring = step.docstring().expect("Expected a docstring");
    let command_line = docstring.trim();
    let mut parts = command_line.split_whitespace();
    let program = parts.next().expect("Expected a command name");
    let args: Vec<&str> = parts.collect();

    let output = Command::cargo_bin(program)
        .unwrap_or_else(|_| panic!("Binary '{program}' not found"))
        .args(&args)
        .output()
        .unwrap_or_else(|e| panic!("Failed to execute command: {e}"));

    world.command_output = Some(CommandOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    });
}

#[then(expr = "it should exit with status code {int}")]
async fn it_should_exit_with_status_code(world: &mut FeaturesWorld, expected_code: i32) {
    let output = world
        .command_output
        .as_ref()
        .expect("No command has been run");
    assert_eq!(
        output.exit_code, expected_code,
        "Expected exit code {expected_code}, got {}.\nstdout: {}\nstderr: {}",
        output.exit_code, output.stdout, output.stderr
    );
}

#[then(regex = "it should output:")]
async fn it_should_output(world: &mut FeaturesWorld, step: &Step) {
    let expected = step.docstring().expect("Expected a docstring");
    let expected = expected.trim();
    let output = world
        .command_output
        .as_ref()
        .expect("No command has been run");
    let actual = output.stdout.trim();
    assert_eq!(actual, expected, "stdout mismatch");
}

#[then(regex = "stderr output should end with:")]
async fn stderr_output_should_end_with(world: &mut FeaturesWorld, step: &Step) {
    let expected = step.docstring().expect("Expected a docstring");
    let expected = expected.trim();
    let output = world
        .command_output
        .as_ref()
        .expect("No command has been run");
    let actual = output.stderr.trim();
    assert!(
        actual.ends_with(expected),
        "Expected stderr to end with:\n{expected}\nBut got:\n{actual}"
    );
}

#[then(regex = "stdout should be a JSON array with two validation errors for paths foo and bar")]
async fn stdout_json_two_validation_errors(world: &mut FeaturesWorld) {
    let output = world
        .command_output
        .as_ref()
        .expect("No command has been run");
    let v: Value = serde_json::from_str(output.stdout.trim()).expect("stdout should be JSON");
    let arr = v.as_array().expect("stdout should be a JSON array");
    assert_eq!(
        arr.len(),
        2,
        "expected two validation errors, got {}",
        arr.len()
    );
    for entry in arr {
        let obj = entry.as_object().expect("each error should be an object");
        for key in ["index", "line", "col", "path", "error"] {
            assert!(obj.contains_key(key), "missing key {key} in {obj:?}");
        }
    }
    let paths: Vec<&str> = arr
        .iter()
        .map(|e| {
            e.get("path")
                .and_then(|p| p.as_str())
                .expect("path should be a string")
        })
        .collect();
    assert!(
        paths.contains(&"foo") && paths.contains(&"bar"),
        "expected paths foo and bar, got {paths:?}"
    );
}

fn list_feature_files(dir: &str) -> std::result::Result<Vec<String>, std::io::Error> {
    let entries = std::fs::read_dir(dir)?;
    let feature_files = entries
        .collect::<std::result::Result<Vec<std::fs::DirEntry>, std::io::Error>>()?
        .iter()
        .filter_map(|entry| {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("feature") {
                Some(
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .expect("Unable to get file name")
                        .to_string(),
                )
            } else {
                None
            }
        })
        .collect::<Vec<String>>();
    Ok(feature_files)
}

fn filter_ignored_scenarios(
    _feature: &cucumber::gherkin::Feature,
    _rule: Option<&cucumber::gherkin::Rule>,
    scenario: &Scenario,
) -> bool {
    !scenario.tags.iter().any(|tag| tag == "ignore")
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .format_target(false)
        .format_timestamp_secs()
        .target(env_logger::Target::Stdout)
        .init();

    for feature_file in list_feature_files("features").expect("Unable to list feature files") {
        FeaturesWorld::filter_run(format!("features/{feature_file}"), filter_ignored_scenarios)
            .await;
    }
    for feature_file in
        list_feature_files("features/validation").expect("Unable to list feature files")
    {
        FeaturesWorld::filter_run(
            format!("features/validation/{feature_file}"),
            filter_ignored_scenarios,
        )
        .await;
    }
}
