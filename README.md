ys - yaml-schema
====

[![CI Tests](https://github.com/aisrael/yaml-schema/actions/workflows/ci-tests.yaml/badge.svg)](https://github.com/aisrael/yaml-schema/actions/workflows/ci-tests.yaml)

**yaml-schema** is a tool to validate YAML files against a YAML schema.

The YAML Schema specification is based on JSON Schema ([https://json-schema.org/](https://json-schema.org/)), but
expressed as YAML.

**yaml-schema** is both a Rust library _and_ an executable.

## Example Usage

Given a `schema.yaml` file containing:

```
type: object
properties:
  foo:
    type: string
  bar:
    type: number
```

And a `valid.yaml` file containing:

```
foo: "I'm a string"
bar: 42
```

Then when you issue the command

```
ys -f schema.yaml valid.yaml
```

Then the command should succeed with exit code 0

On the other hand, when given an `invalid.yaml` file containing:

```
foo: 42
bar: "I'm a string"
```

Then the command

```
ys -f schema.yaml invalid.yaml
```

Should fail with exit code 1

## Features

**yaml-schema** uses [Cucumber](https://cucumber-rs.github.io/cucumber/main/) to specify and test features:

- [CLI usage](features/cli.feature)
- [Basic features](features/basics.feature)
- [String validation](features/validation/strings.feature)
- [Numeric types](features/validation/numbers.feature)
- [Const](features/validation/const.feature)
- [Enums](features/validation/enums.feature)
- [Object types](features/validation/objects.feature)
- [Arrays](features/validation/arrays.feature)
- [Composition](features/composition.feature)

See the [features](features/) folder for all examples.

## Installation

Currently, **yaml-schema** requires Git, Rust and Cargo to build and
install: [https://doc.rust-lang.org/cargo/](https://doc.rust-lang.org/cargo/)

To install the stable release from [crates.io](https://crates.io/crates/yaml-schema):

```
cargo install yaml-schema
```

That should build and install the executable at `$HOME/.cargo/bin/ys` (which should be in your PATH)

Alternatively, one can install from latest source:

```
cargo install --git https://github.com/yaml-schema/yaml-schema
```

## Usage

Running `ys` without any options or arguments should display the help:

```
A tool for validating YAML against a schema

Usage: ys [OPTIONS] [FILE] [COMMAND]

Commands:
  version  Display the ys version
  help     Print this message or the help of the given subcommand(s)

Arguments:
  [FILE]  The YAML file to validate

Options:
  -f, --schema <SCHEMAS>  The schema to validate against
      --fail-fast         Specify this flag to exit (1) as soon as any error is encountered
  -h, --help              Print help
  -V, --version           Print version
  ```

## Self-Validation

**yaml-schema** is _self-validating_. That is, running

```
cargo run -- -f yaml-schema.yaml yaml-schema.yaml
```

_should_ always succeed.
