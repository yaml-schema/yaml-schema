use std::fs::File;
use std::fs::read_to_string;
use std::path::Path;
use std::time::Duration;

use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("ys_vs_boon");
    group.sample_size(10000);
    group.measurement_time(Duration::from_secs(10));
    group.bench_function("boon", |b| b.iter(boon));
    group.bench_function("ys", |b| b.iter(ys));
    group.finish();

    let mut group = c.benchmark_group("kustomization");
    group.sample_size(1000);
    group.measurement_time(Duration::from_secs(5));
    group.bench_function("boon", |b| b.iter(boon_kustomization));
    group.bench_function("ys", |b| b.iter(ys_kustomization));
    group.finish();

    let mut group = c.benchmark_group("kubernetes_pod");
    group.sample_size(1000);
    group.measurement_time(Duration::from_secs(5));
    group.bench_function("boon", |b| b.iter(boon_pod));
    group.bench_function("ys", |b| b.iter(ys_pod));
    group.finish();

    let mut group = c.benchmark_group("kubernetes_deployment");
    group.sample_size(1000);
    group.measurement_time(Duration::from_secs(5));
    group.bench_function("boon", |b| b.iter(boon_deployment));
    group.bench_function("ys", |b| b.iter(ys_deployment));
    group.finish();

    let mut group = c.benchmark_group("github_workflow");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(10));
    group.bench_function("boon", |b| b.iter(boon_github_workflow));
    group.bench_function("ys", |b| b.iter(ys_github_workflow));
    group.finish();
}

fn ys() {
    let schema_filename = "yaml-schema.yaml";
    let root_schema =
        yaml_schema::loader::load_file(schema_filename).expect("Failed to load schema");
    let yaml_contents = read_to_string(schema_filename).expect("Failed to read YAML file");

    let context = yaml_schema::Engine::evaluate(&root_schema, &yaml_contents, false)
        .expect("Failed to validate YAML");
    if context.has_errors() {
        for error in context.errors.borrow().iter() {
            eprintln!("{error}");
        }
    }
    assert!(!context.has_errors());
}

fn boon() {
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    let mut loader = boon::SchemeUrlLoader::new();
    loader.register("file", Box::new(FileUrlLoader));
    compiler.use_loader(Box::new(loader));

    let schema_value = boon_schema_value(Path::new("yaml-schema.yaml"));
    compiler
        .add_resource("yaml-schema.yaml", schema_value)
        .expect("Failed to add resource");
    let sch_index = compiler
        .compile("yaml-schema.yaml", &mut schemas)
        .expect("Failed to compile schema");
    let instance = boon_instance_value(Path::new("yaml-schema.yaml"));
    let valid = schemas.validate(&instance, sch_index).is_ok();
    assert!(valid);
}

/// Validates a `ys` schema+instance pair by loading the schema via
/// [`yaml_schema::loader::load_file`] (which sets `base_uri`, so relative
/// `$ref`s resolve against the schema's own directory) and evaluating the
/// instance against it.
fn ys_validate(schema_path: &str, instance_path: &str) {
    let root_schema = yaml_schema::loader::load_file(schema_path).expect("Failed to load schema");
    let yaml_contents = read_to_string(instance_path).expect("Failed to read YAML file");

    let context = yaml_schema::Engine::evaluate(&root_schema, &yaml_contents, false)
        .expect("Failed to validate YAML");
    if context.has_errors() {
        for error in context.errors.borrow().iter() {
            eprintln!("{error}");
        }
    }
    assert!(!context.has_errors());
}

fn ys_kustomization() {
    ys_validate(
        "examples/kustomization/schema.yaml",
        "examples/kustomization/fixtures/kustomization-overlay.yaml",
    );
}

fn ys_pod() {
    ys_validate(
        "examples/kubernetes/pod/schema.yaml",
        "examples/kubernetes/pod/fixtures/multi-container-pod.yaml",
    );
}

fn ys_deployment() {
    ys_validate(
        "examples/kubernetes/deployment/schema.yaml",
        "examples/kubernetes/deployment/fixtures/deployment-with-resources.yaml",
    );
}

fn ys_github_workflow() {
    ys_validate(
        "examples/github-workflow/schema.yaml",
        ".github/workflows/build.yaml",
    );
}

fn boon_kustomization() {
    boon_validate(
        "examples/kustomization/schema.yaml",
        "examples/kustomization/fixtures/kustomization-overlay.yaml",
    );
}

fn boon_pod() {
    boon_validate(
        "examples/kubernetes/pod/schema.yaml",
        "examples/kubernetes/pod/fixtures/multi-container-pod.yaml",
    );
}

fn boon_github_workflow() {
    boon_validate(
        "examples/github-workflow/schema.yaml",
        ".github/workflows/build.yaml",
    );
}

/// Validates a `boon` schema+instance pair that has no cross-file `$ref`s,
/// using the schema's relative path as its resource id (matching the
/// existing `boon()` pattern for `yaml-schema.yaml`).
fn boon_validate(schema_path: &str, instance_path: &str) {
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    let mut loader = boon::SchemeUrlLoader::new();
    loader.register("file", Box::new(FileUrlLoader));
    compiler.use_loader(Box::new(loader));

    let schema_value = boon_schema_value(Path::new(schema_path));
    compiler
        .add_resource(schema_path, schema_value)
        .expect("Failed to add resource");
    let sch_index = compiler
        .compile(schema_path, &mut schemas)
        .expect("Failed to compile schema");
    let instance = boon_instance_value(Path::new(instance_path));
    let valid = schemas.validate(&instance, sch_index).is_ok();
    assert!(valid);
}

/// Validates the `kubernetes_deployment` schema+instance pair with `boon`.
///
/// Unlike [`boon_validate`], this schema has a cross-file `$ref` into
/// `../pod/schema.yaml`, so the deployment schema resource must be added
/// under its canonical `file://` URL: relative `$ref`s are resolved against
/// that URL, producing another `file://` URL that `FileUrlLoader` then loads.
fn boon_deployment() {
    let schema_path = "examples/kubernetes/deployment/schema.yaml";
    let instance_path = "examples/kubernetes/deployment/fixtures/deployment-with-resources.yaml";

    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    let mut loader = boon::SchemeUrlLoader::new();
    loader.register("file", Box::new(FileUrlLoader));
    compiler.use_loader(Box::new(loader));

    let canonical = Path::new(schema_path)
        .canonicalize()
        .expect("Failed to canonicalize schema path");
    let schema_url = url::Url::from_file_path(&canonical).expect("Failed to convert path to URL");

    let schema_value = boon_schema_value(Path::new(schema_path));
    compiler
        .add_resource(schema_url.as_str(), schema_value)
        .expect("Failed to add resource");
    let sch_index = compiler
        .compile(schema_url.as_str(), &mut schemas)
        .expect("Failed to compile schema");
    let instance = boon_instance_value(Path::new(instance_path));
    let valid = schemas.validate(&instance, sch_index).is_ok();
    assert!(valid);
}

/// Reads a YAML schema file and strips its top-level `$schema` key.
///
/// yaml-schema's example/meta schemas declare `$schema:
/// https://yaml-schema.net/yaml-schema.yaml`, a self-referential,
/// non-standard meta-schema URL that boon can't resolve to a known JSON
/// Schema draft (boon explicitly rejects a schema whose `$schema` points
/// back at itself). Dropping it lets boon fall back to its default (latest)
/// draft instead.
fn boon_schema_value(path: &Path) -> serde_json::Value {
    let mut value: serde_json::Value =
        serde_yaml::from_reader(File::open(path).expect("Failed to open YAML file"))
            .expect("Failed to read YAML file");
    strip_dollar_schema(&mut value);
    value
}

fn boon_instance_value(path: &Path) -> serde_json::Value {
    serde_yaml::from_reader(File::open(path).expect("Failed to open YAML file"))
        .expect("Failed to read YAML file")
}

fn strip_dollar_schema(value: &mut serde_json::Value) {
    if let Some(obj) = value.as_object_mut() {
        obj.remove("$schema");
    }
}

struct FileUrlLoader;
impl boon::UrlLoader for FileUrlLoader {
    fn load(&self, url: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let url = url::Url::parse(url)?;
        let path = url.to_file_path().map_err(|_| "invalid file path")?;
        let file = File::open(&path)?;
        let mut value = if path
            .extension()
            .filter(|&ext| ext == "yaml" || ext == "yml")
            .is_some()
        {
            serde_yaml::from_reader(file)?
        } else {
            serde_json::from_reader(file)?
        };
        // Cross-file $refs (e.g. the kubernetes_deployment example referencing
        // ../pod/schema.yaml) can pull in another yaml-schema example/meta
        // schema, which also declares a self-referential `$schema` key. Strip
        // it here too, for the same reason as `boon_schema_value`.
        strip_dollar_schema(&mut value);
        Ok(value)
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
