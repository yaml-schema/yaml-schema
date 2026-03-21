use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use clap::Parser;
use clap::Subcommand;
use eyre::Context;
use eyre::Result;
use serde_json::json;
use url::Url;

use yaml_schema::Engine;
use yaml_schema::RootSchema;
use yaml_schema::loader;
use yaml_schema::validation::ValidationError;
use yaml_schema::version;

#[derive(Parser, Debug, Default)]
#[command(name = "ys")]
#[command(author = "Alistair Israel <aisrael@gmail.com>")]
#[command(version = clap::crate_version!())]
#[command(about = "A tool for validating YAML against a schema")]
#[command(arg_required_else_help = true)]
pub struct Opts {
    /// The command to run
    #[command(subcommand)]
    pub command: Option<Commands>,
    /// Schema file(s) to load. The first is the root schema; additional schemas are
    /// pre-loaded for $ref resolution. May be specified multiple times (-f a.yaml -f b.yaml).
    /// Omit when the instance YAML has a top-level string `$schema` (URL or path).
    #[arg(short = 'f', long = "schema")]
    pub schemas: Vec<String>,
    /// Specify this flag to exit (1) as soon as any error is encountered
    #[arg(long = "fail-fast", default_value = "false")]
    pub fail_fast: bool,
    /// Emit errors as JSON: validation failures as a JSON array on stdout; other failures as
    /// {"error":"..."} on stderr.
    #[arg(long = "json")]
    pub json: bool,
    /// The YAML file to validate
    pub file: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Display the ys version")]
    Version,
}

fn emit_json_error(message: &str) {
    eprintln!("{}", json!({ "error": message }));
}

fn emit_validation_errors_json(errors: &[ValidationError]) {
    let entries: Vec<serde_json::Value> = errors
        .iter()
        .map(|e| {
            json!({
                "index": e.marker.map(|m| m.index()),
                "line": e.marker.map(|m| m.line()),
                "col": e.marker.map(|m| m.col()),
                "path": e.path,
                "error": e.error,
            })
        })
        .collect();
    println!("{}", serde_json::Value::Array(entries));
}

/// The main entrypoint function of the ys executable
fn main() {
    env_logger::init();
    let opts = Opts::parse();
    if let Some(command) = opts.command {
        match command {
            Commands::Version => {
                println!("ys {}", version());
            }
        }
    } else {
        let json = opts.json;
        match command_validate(opts) {
            Ok(return_code) => {
                std::process::exit(return_code);
            }
            Err(e) => {
                if json {
                    emit_json_error(&e.to_string());
                } else {
                    eprintln!("Validation failed: {e}");
                }
                std::process::exit(1);
            }
        }
    }
}

fn schema_uri(path: &str) -> Result<String> {
    let canonical = Path::new(path)
        .canonicalize()
        .wrap_err_with(|| format!("Failed to resolve schema path: {path}"))?;
    let url = Url::from_file_path(canonical)
        .map_err(|_| eyre::eyre!("Failed to convert path to URL: {path}"))?;
    Ok(url.to_string())
}

fn insert_preloaded_entry(
    preloaded: &mut HashMap<String, Rc<RootSchema>>,
    schema: RootSchema,
    uri: String,
) -> Rc<RootSchema> {
    let schema_rc = Rc::new(schema);
    let key = schema_rc.cache_key(&uri);
    // Insert under both `uri` and `key` when they differ so $ref resolution matches the CLI preload map.
    if key != uri {
        preloaded.insert(uri, Rc::clone(&schema_rc));
    }
    preloaded.insert(key, Rc::clone(&schema_rc));
    schema_rc
}

/// The `ys validate` command
fn command_validate(opts: Opts) -> Result<i32> {
    let json = opts.json;
    let yaml_filename = match &opts.file {
        Some(f) => f.as_str(),
        None => return Err(eyre::eyre!("No YAML file specified")),
    };

    let yaml_contents = std::fs::read_to_string(yaml_filename)
        .wrap_err_with(|| format!("Failed to read YAML file: {yaml_filename}"))?;

    let (root_for_eval, preloaded) = if !opts.schemas.is_empty() {
        let root_path = opts.schemas.first().expect("non-empty schemas");
        let root_schema = match loader::load_file(root_path) {
            Ok(schema) => schema,
            Err(e) => {
                if json {
                    emit_json_error(&format!("Failed to read YAML schema file {root_path}: {e}"));
                } else {
                    eprintln!("Failed to read YAML schema file: {root_path}");
                    log::error!("{e}");
                }
                return Ok(1);
            }
        };

        let mut preloaded = HashMap::new();
        for path in &opts.schemas {
            let uri = match schema_uri(path) {
                Ok(u) => u,
                Err(e) => {
                    if json {
                        emit_json_error(&format!("Failed to resolve schema path {path}: {e}"));
                    } else {
                        eprintln!("Failed to resolve schema path: {path}: {e}");
                    }
                    return Ok(1);
                }
            };
            let schema = match loader::load_file(path) {
                Ok(s) => s,
                Err(e) => {
                    if json {
                        emit_json_error(&format!("Failed to load schema file {path}: {e}"));
                    } else {
                        eprintln!("Failed to load schema file: {path}");
                        log::error!("{e}");
                    }
                    return Ok(1);
                }
            };
            let _ = insert_preloaded_entry(&mut preloaded, schema, uri);
        }

        let root_rc = Rc::new(root_schema);
        (root_rc, preloaded)
    } else {
        let instance_parent = Path::new(yaml_filename).parent().unwrap_or(Path::new("."));
        let schema_ref = match loader::extract_dollar_schema_from_yaml(&yaml_contents) {
            Ok(Some(s)) => s,
            Ok(None) => {
                return Err(eyre::eyre!(
                    "No schema: pass -f/--schema or add a string `$schema` key to the YAML root mapping"
                ));
            }
            Err(e) => {
                return Err(eyre::eyre!(
                    "Could not read `$schema` from instance YAML: {e}"
                ));
            }
        };

        let (root, uri) = match loader::load_root_schema_from_ref(&schema_ref, instance_parent) {
            Ok(pair) => pair,
            Err(e) => {
                if json {
                    emit_json_error(&format!(
                        "Failed to load schema from $schema {schema_ref:?}: {e}"
                    ));
                } else {
                    eprintln!("Failed to load schema from $schema: {schema_ref}");
                    log::error!("{e}");
                }
                return Ok(1);
            }
        };

        let mut preloaded = HashMap::new();
        let root_rc = insert_preloaded_entry(&mut preloaded, root, uri);

        (root_rc, preloaded)
    };

    match Engine::evaluate_with_schemas(
        root_for_eval.as_ref(),
        &yaml_contents,
        opts.fail_fast,
        preloaded,
    ) {
        Ok(context) => {
            if context.has_errors() {
                let errors = context.errors.borrow();
                if json {
                    emit_validation_errors_json(errors.as_slice());
                } else {
                    for error in errors.iter() {
                        eprintln!("{error}");
                    }
                }
                return Ok(1);
            }
            Ok(0)
        }
        Err(e) => {
            if json {
                emit_json_error(&format!("Validation failed: {e}"));
            } else {
                eprintln!("Validation failed: {e}");
            }
            Ok(1)
        }
    }
}
