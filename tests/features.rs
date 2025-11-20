use cucumber::World;
use cucumber::gherkin::Scenario;
use cucumber::gherkin::Step;
use cucumber::given;
use cucumber::then;
use log::{debug, error};
use std::cell::RefCell;
use std::rc::Rc;
use yaml_schema::validation::ValidationError;
use yaml_schema::{Engine, Result, RootSchema};

#[derive(Debug, Default, World)]
pub struct FeaturesWorld {
    root_schema: RootSchema,
    yaml_schema_error: Option<yaml_schema::Error>,
    errors: Option<Rc<RefCell<Vec<ValidationError>>>>,
}

#[given(regex = "a YAML schema:")]
async fn a_yaml_schema(world: &mut FeaturesWorld, step: &Step) {
    let schema = step.docstring().unwrap();
    debug!("schema: {schema:?}");
    match RootSchema::load_from_str(schema) {
        Ok(root_schema) => world.root_schema = root_schema,
        Err(e) => {
            error!("Error: {e:?}");
            world.yaml_schema_error = Some(e);
        }
    }
}

fn evaluate(world: &mut FeaturesWorld, s: &str) -> Result<bool> {
    let context = Engine::evaluate(&world.root_schema, s, false)?;
    world.errors = Some(context.errors.clone());
    for error in context.errors.borrow().iter() {
        println!("{error}");
    }
    Ok(!context.has_errors())
}

#[then(regex = "it should accept:")]
async fn it_should_accept(world: &mut FeaturesWorld, step: &Step) {
    let raw_input = step.docstring().unwrap();
    let input_without_beginning_newline = raw_input.strip_prefix('\n').unwrap();
    let result = evaluate(world, input_without_beginning_newline).unwrap();
    assert!(result);
}

#[then(regex = "it should NOT accept:")]
async fn it_should_not_accept(world: &mut FeaturesWorld, step: &Step) {
    let raw_input = step.docstring().unwrap();
    let input_without_beginning_newline = raw_input.strip_prefix('\n').unwrap();
    let result = evaluate(world, input_without_beginning_newline).unwrap();
    assert!(!result);
}

#[then(expr = "the error message should be {string}")]
fn the_error_message_should_be(world: &mut FeaturesWorld, expected_error_message: String) {
    let errors = world.errors.as_ref().unwrap().borrow();
    if !errors.is_empty() {
        let first_error = errors.first().unwrap();
        let actual_error_message = first_error.to_string();
        assert_eq!(actual_error_message, expected_error_message);
    } else {
        panic!("Expected an error message, but there was no error!");
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
                        .unwrap()
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

    for feature_file in list_feature_files("features").unwrap() {
        FeaturesWorld::filter_run(format!("features/{feature_file}"), filter_ignored_scenarios)
            .await;
    }
    for feature_file in list_feature_files("features/validation").unwrap() {
        FeaturesWorld::filter_run(
            format!("features/validation/{feature_file}"),
            filter_ignored_scenarios,
        )
        .await;
    }
}
