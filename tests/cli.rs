use core::panic;
use cucumber::{World, gherkin::Step, given, then, when};
use log::{debug, info};
use std::process::Command;

#[derive(Debug, Default, World)]
pub struct CliWorld {
    command_output: Option<String>,
    command_stderr: Option<String>,
}

#[when(regex = "the following command is run:")]
async fn run_command(world: &mut CliWorld, step: &Step) {
    let raw_command = step.docstring().unwrap();
    debug!("raw_command {raw_command}");
    let parts = raw_command.split_whitespace().collect::<Vec<&str>>();
    assert!(!parts.is_empty(), "No command provided");
    let mut args: Vec<&str> = parts[1..].to_vec();
    let executable = if parts[0] == "ys" {
        args.insert(0, "--");
        args.insert(0, "run");
        "cargo"
    } else {
        parts[0]
    };
    debug!("Executable: {executable}");

    match Command::new(executable).args(args).output() {
        Ok(output) => {
            let output_str = String::from_utf8(output.stdout).unwrap();
            debug!("Output: {output_str}");
            world.command_output = Some(output_str);
            let stderr_output = String::from_utf8(output.stderr).unwrap();
            debug!("stderr: {stderr_output}");
            world.command_stderr = Some(stderr_output);
        }
        Err(e) => {
            panic!("Failed to run command: {}", e);
        }
    }
}

#[given(expr = r#"a file named `{word}` containing:"#)]
async fn a_file_containing(_world: &mut CliWorld, filename: String, step: &Step) {
    debug!("filename: {filename:?}");
    let file_content = step.docstring().unwrap();
    debug!("file_content: {file_content:?}");
}

#[then(expr = "it should exit with status code {int}")]
async fn it_should_exit_with_status(_world: &mut CliWorld, status: i32) {
    debug!("status: {status:?}");
}

#[then(expr = "it should output:")]
async fn it_should_output(world: &mut CliWorld, step: &Step) {
    assert!(world.command_output.is_some());
    // For some reason, the output docstring has a leading newline
    let expected_output = step.docstring().unwrap().strip_prefix('\n').unwrap();
    let actual_output = world.command_output.as_ref().unwrap();
    assert_eq!(expected_output, actual_output);
}

#[then(expr = "stderr output should end with:")]
async fn stderr_output_should_end_with(world: &mut CliWorld, step: &Step) {
    assert!(world.command_stderr.is_some());
    // For some reason, the output docstring has a leading newline
    let expected_output = step.docstring().unwrap().strip_prefix('\n').unwrap();
    let actual_output = world.command_stderr.as_ref().unwrap();
    assert!(actual_output.ends_with(expected_output));
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .format_target(false)
        .format_timestamp_secs()
        .target(env_logger::Target::Stdout)
        .init();
    info!("Running CLI tests");

    CliWorld::run("features/cli.feature").await;
}
