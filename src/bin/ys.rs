use clap::Parser;
use clap::Subcommand;
use eyre::Context;
use eyre::Result;

use yaml_schema::version;
use yaml_schema::Engine;
use yaml_schema::RootSchema;

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
    /// The schema to validate against
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
    if let Some(comand) = opts.command {
        match comand {
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
                eprintln!("Validation failed: {}", e);
                std::process::exit(1);
            }
        }
    }
}

/// The `ys validate` command
fn command_validate(opts: Opts) -> Result<i32> {
    // Currently, we only support a single schema file
    // TODO: Support multiple schema files
    if opts.schemas.is_empty() {
        return Err(eyre::eyre!("No schema file(s) specified"));
    }
    if opts.file.is_none() {
        return Err(eyre::eyre!("No YAML file specified"));
    }

    let schema_filename = opts.schemas.first().unwrap();
    let root_schema = match RootSchema::load_file(schema_filename) {
        Ok(schema) => schema,
        Err(e) => {
            eprintln!("Failed to read YAML schema file: {}", schema_filename);
            log::error!("{}", e);
            return Ok(1);
        }
    };

    let yaml_filename = opts.file.as_ref().unwrap();
    let yaml_contents = std::fs::read_to_string(yaml_filename)
        .wrap_err_with(|| format!("Failed to read YAML file: {}", yaml_filename))?;

    match Engine::evaluate(&root_schema, &yaml_contents, opts.fail_fast) {
        Ok(context) => {
            if context.has_errors() {
                for error in context.errors.borrow().iter() {
                    eprintln!("{}", error);
                }
                return Ok(1);
            }
            Ok(0)
        }
        Err(e) => {
            eprintln!("Validation failed: {}", e);
            Ok(1)
        }
    }
}
