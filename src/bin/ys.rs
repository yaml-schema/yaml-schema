use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use clap::Parser;
use clap::Subcommand;
use eyre::Context;
use eyre::Result;
use url::Url;

use yaml_schema::Engine;
use yaml_schema::loader;
use yaml_schema::version;

#[derive(Parser, Debug, Default)]
#[command(name = "ys")]
#[command(author = "Alistair Israel <aisrael@gmail.com>")]
#[command(version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("YS_TARGET"), ")"))]
#[command(about = "A tool for validating YAML against a schema")]
#[command(arg_required_else_help = true)]
pub struct Opts {
    /// The command to run
    #[command(subcommand)]
    pub command: Option<Commands>,
    /// Schema file(s) to load. The first is the root schema; additional schemas are
    /// pre-loaded for $ref resolution. May be specified multiple times (-f a.yaml -f b.yaml).
    #[arg(short = 'f', long = "schema")]
    pub schemas: Vec<String>,
    /// Specify this flag to exit (1) as soon as any error is encountered
    #[arg(long = "fail-fast", default_value = "false")]
    pub fail_fast: bool,
    /// The YAML file to validate
    pub file: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Display the ys version")]
    Version,
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
        match command_validate(opts) {
            Ok(return_code) => {
                std::process::exit(return_code);
            }
            Err(e) => {
                eprintln!("Validation failed: {e}");
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

/// The `ys validate` command
fn command_validate(opts: Opts) -> Result<i32> {
    if opts.schemas.is_empty() {
        return Err(eyre::eyre!("No schema file(s) specified"));
    }
    if opts.file.is_none() {
        return Err(eyre::eyre!("No YAML file specified"));
    }

    let root_path = opts.schemas.first().expect("No schema file(s) specified");
    let root_schema = match loader::load_file(root_path) {
        Ok(schema) => schema,
        Err(e) => {
            eprintln!("Failed to read YAML schema file: {root_path}");
            log::error!("{e}");
            return Ok(1);
        }
    };

    let mut preloaded = HashMap::new();
    for path in &opts.schemas {
        let uri = match schema_uri(path) {
            Ok(u) => u,
            Err(e) => {
                eprintln!("Failed to resolve schema path: {path}: {e}");
                return Ok(1);
            }
        };
        let schema = match loader::load_file(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to load schema file: {path}");
                log::error!("{e}");
                return Ok(1);
            }
        };
        let schema_rc = Rc::new(schema);
        let key = schema_rc.cache_key(&uri);
        // We need to insert the schema under both `uri` and `key` if they differ, because `cache_key` may normalize or canonicalize
        // the schema reference (for example, following $id or resolving symlinks). This ensures both the original URI and any internal
        // references will consistently resolve to the same Rc<Schema> instance during validation and $ref resolution.
        if key != uri {
            preloaded.insert(uri, Rc::clone(&schema_rc));
        }
        preloaded.insert(key, schema_rc);
    }

    let yaml_filename = opts.file.as_ref().expect("No YAML file specified");
    let yaml_contents = std::fs::read_to_string(yaml_filename)
        .wrap_err_with(|| format!("Failed to read YAML file: {yaml_filename}"))?;

    match Engine::evaluate_with_schemas(&root_schema, &yaml_contents, opts.fail_fast, preloaded) {
        Ok(context) => {
            if context.has_errors() {
                for error in context.errors.borrow().iter() {
                    eprintln!("{error}");
                }
                return Ok(1);
            }
            Ok(0)
        }
        Err(e) => {
            eprintln!("Validation failed: {e}");
            Ok(1)
        }
    }
}
