use std::fs::File;
use std::fs::read_to_string;
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

    let mut schema_value: serde_json::Value =
        serde_yaml::from_reader(File::open("yaml-schema.yaml").expect("Failed to open YAML file"))
            .expect("Failed to read YAML file");
    // yaml-schema.yaml's `$schema` is a self-referential, non-standard meta-schema URL
    // that boon can't resolve to a known JSON Schema draft (boon explicitly rejects a
    // schema whose `$schema` points back at itself). Drop it so boon falls back to its
    // default (latest) draft instead.
    if let Some(obj) = schema_value.as_object_mut() {
        obj.remove("$schema");
    }
    compiler
        .add_resource("yaml-schema.yaml", schema_value)
        .expect("Failed to add resource");
    let sch_index = compiler
        .compile("yaml-schema.yaml", &mut schemas)
        .expect("Failed to compile schema");
    let instance =
        serde_yaml::from_reader(File::open("yaml-schema.yaml").expect("Failed to open YAML file"))
            .expect("Failed to read YAML file");
    let valid = schemas.validate(&instance, sch_index).is_ok();
    assert!(valid);
}

struct FileUrlLoader;
impl boon::UrlLoader for FileUrlLoader {
    fn load(&self, url: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let url = url::Url::parse(url)?;
        let path = url.to_file_path().map_err(|_| "invalid file path")?;
        let file = File::open(&path)?;
        if path
            .extension()
            .filter(|&ext| ext == "yaml" || ext == "yml")
            .is_some()
        {
            Ok(serde_yaml::from_reader(file)?)
        } else {
            Ok(serde_json::from_reader(file)?)
        }
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
