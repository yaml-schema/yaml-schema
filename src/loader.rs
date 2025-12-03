//! The loader module loads the YAML schema from a file into the in-memory model

use std::time::Duration;

use reqwest::Url;
use reqwest::blocking::Client;
use saphyr::LoadableYamlNode;
use saphyr::MarkedYaml;
use saphyr::Scalar;
use saphyr::YamlData;

use crate::Error;
use crate::Number;
use crate::Result;
use crate::RootSchema;
use crate::schemas::BooleanOrSchema;
use crate::schemas::YamlSchema;
use crate::utils::format_marker;
use crate::utils::try_unwrap_saphyr_scalar;

/// Load a YAML schema from a file.
/// Delegates to the `load_from_doc` function to load the schema from the first document.
pub fn load_file<'f, S: AsRef<str>>(path: S) -> Result<RootSchema<'f>> {
    let fs_metadata = std::fs::metadata(path.as_ref())?;
    if !fs_metadata.is_file() {
        return Err(Error::FileNotFound(path.as_ref().to_string()));
    }
    let s = std::fs::read_to_string(path.as_ref())?;
    load_from_str(&s)
}

/// Load a YAML schema from a &str.
pub fn load_from_str<'f>(s: &str) -> Result<RootSchema<'f>> {
    let docs = MarkedYaml::load_from_str(s).map_err(Error::YamlParsingError)?;
    load_from_docs(docs)
}

/// Load a RootSchema from Vec of docs.
pub fn load_from_docs<'f>(docs: Vec<MarkedYaml<'f>>) -> Result<RootSchema<'f>> {
    if docs.is_empty() {
        return Ok(RootSchema::empty()); // empty schema
    }
    let first_doc = docs.first().expect("No documents found");
    load_from_doc(first_doc)
}

/// Load a YAML schema from a document. Basically just a wrapper around the TryFrom<&MarkedYaml<'_>> for RootSchema.
pub fn load_from_doc<'f>(doc: &MarkedYaml<'f>) -> Result<RootSchema<'f>> {
    RootSchema::try_from(doc)
}

/// Error type for URL loading operations
#[derive(thiserror::Error, Debug)]
pub enum UrlLoadError {
    #[error("Failed to download from URL: {0}")]
    DownloadError(#[from] reqwest::Error),

    #[error("Failed to parse URL: {0}")]
    ParseUrlError(#[from] url::ParseError),

    #[error("Failed to parse YAML: {0}")]
    ParseError(#[from] saphyr::ScanError),

    #[error("No YAML documents found in the downloaded content")]
    NoDocuments,
}

impl From<reqwest::Error> for crate::Error {
    fn from(value: reqwest::Error) -> Self {
        crate::Error::UrlLoadError(UrlLoadError::DownloadError(value))
    }
}

/// Downloads a YAML schema from a URL and parses it into a YamlSchema
///
/// # Arguments
/// * `url` - The URL to download the YAML schema from
/// * `timeout_seconds` - Optional timeout in seconds for the HTTP request (default: 30 seconds)
///
/// # Returns
/// A `Result` containing the parsed `YamlSchema` if successful, or an error if the download or parsing fails.
///
/// # Example
/// ```no_run
/// use yaml_schema::loader::download_from_url;
///
/// let schema = download_from_url("https://example.com/schema.yaml", None).unwrap();
/// ```
pub fn download_from_url(url_string: &str, timeout_seconds: Option<u64>) -> Result<RootSchema<'_>> {
    // Create a new HTTP client with a custom timeout
    let timeout = Duration::from_secs(timeout_seconds.unwrap_or(30));
    let client = Client::builder()
        .timeout(timeout)
        .use_native_tls()
        .build()?;

    let url = Url::parse(url_string).map_err(|e| Error::UrlLoadError(e.into()))?;

    // Download the YAML content
    let response = client.get(url).send()?;
    if !response.status().is_success() {
        match response.error_for_status() {
            Ok(_) => unreachable!(),
            Err(e) => return Err(e.into()),
        }
    }

    let yaml_content = response.text()?;

    // Parse the YAML content
    let docs = MarkedYaml::load_from_str(&yaml_content).map_err(UrlLoadError::ParseError)?;

    match docs.first() {
        Some(doc) => load_from_doc(doc),
        None => Err(UrlLoadError::NoDocuments.into()),
    }
}

pub fn marked_yaml_to_string<S: Into<String> + Copy>(yaml: &MarkedYaml, msg: S) -> Result<String> {
    if let YamlData::Value(Scalar::String(s)) = &yaml.data {
        Ok(s.to_string())
    } else {
        Err(Error::ExpectedScalar(msg.into()))
    }
}

pub fn load_array_of_schemas_marked<'f>(value: &MarkedYaml<'f>) -> Result<Vec<YamlSchema<'f>>> {
    if let YamlData::Sequence(values) = &value.data {
        values
            .iter()
            .map(|v| {
                if v.is_mapping() {
                    v.try_into()
                } else {
                    Err(generic_error!("Expected a mapping, but got: {:?}", v))
                }
            })
            .collect::<Result<Vec<YamlSchema>>>()
    } else {
        Err(generic_error!(
            "{} Expected a sequence, but got: {:?}",
            format_marker(&value.span.start),
            value
        ))
    }
}

pub fn load_integer(value: &saphyr::Yaml) -> Result<i64> {
    let scalar = try_unwrap_saphyr_scalar(value)?;
    match scalar {
        saphyr::Scalar::Integer(i) => Ok(*i),
        _ => Err(unsupported_type!(
            "Expected type: integer, but got: {:?}",
            value
        )),
    }
}

pub fn load_integer_marked(value: &MarkedYaml) -> Result<i64> {
    if let YamlData::Value(Scalar::Integer(i)) = &value.data {
        Ok(*i)
    } else {
        Err(generic_error!(
            "{} Expected integer value, got: {:?}",
            format_marker(&value.span.start),
            value
        ))
    }
}

pub fn load_number(value: &saphyr::Yaml) -> Result<Number> {
    let scalar = try_unwrap_saphyr_scalar(value)?;
    match scalar {
        Scalar::Integer(i) => Ok(Number::integer(*i)),
        Scalar::FloatingPoint(o) => Ok(Number::float(o.into_inner())),
        _ => Err(unsupported_type!(
            "Expected type: integer or float, but got: {:?}",
            value
        )),
    }
}

pub fn load_array_items_marked<'input>(
    value: &MarkedYaml<'input>,
) -> Result<BooleanOrSchema<'input>> {
    match &value.data {
        YamlData::Value(scalar) => {
            if let Scalar::Boolean(b) = scalar {
                Ok(BooleanOrSchema::Boolean(*b))
            } else {
                Err(generic_error!(
                    "array: boolean or mapping with type or $ref, but got: {:?}",
                    value
                ))
            }
        }
        YamlData::Mapping(_mapping) => {
            let schema: YamlSchema = value.try_into()?;
            Ok(BooleanOrSchema::schema(schema))
        }
        _ => Err(generic_error!(
            "array: boolean or mapping with type or $ref, but got: {:?}",
            value
        )),
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;
    use saphyr::LoadableYamlNode;
    use saphyr::MarkedYaml;

    use crate::ConstValue;
    use crate::Engine;
    use crate::Result;
    use crate::Validator as _;
    use crate::loader;
    use crate::schemas::IntegerSchema;
    use crate::schemas::SchemaType;
    use crate::schemas::StringSchema;

    use super::*;

    #[test]
    fn test_boolean_literal_true() {
        let root_schema = load_from_doc(&MarkedYaml::value_from_str("true")).unwrap();
        assert_eq!(root_schema.schema, YamlSchema::BooleanLiteral(true));
    }

    #[test]
    fn test_boolean_literal_false() {
        let root_schema = load_from_doc(&MarkedYaml::value_from_str("false")).unwrap();
        assert_eq!(root_schema.schema, YamlSchema::BooleanLiteral(false));
    }

    #[test]
    fn test_const_string() {
        let docs = MarkedYaml::load_from_str("const: string value").unwrap();
        let root_schema = load_from_doc(docs.first().unwrap()).unwrap();
        let YamlSchema::Subschema(subschema) = &root_schema.schema else {
            panic!("Expected Subschema, but got: {:?}", &root_schema.schema);
        };
        assert_eq!(subschema.r#const, Some(ConstValue::string("string value")));
    }

    #[test]
    fn test_const_integer() {
        let docs = MarkedYaml::load_from_str("const: 42").unwrap();
        let root_schema = load_from_doc(docs.first().unwrap()).unwrap();
        let YamlSchema::Subschema(subschema) = &root_schema.schema else {
            panic!("Expected Subschema, but got: {:?}", &root_schema.schema);
        };
        assert_eq!(subschema.r#const, Some(ConstValue::integer(42)));
    }

    #[test]
    fn test_type_foo_should_error() {
        let docs = MarkedYaml::load_from_str("type: foo").unwrap();
        let root_schema = load_from_doc(docs.first().unwrap());
        assert!(root_schema.is_err());
        assert_eq!(
            root_schema.unwrap_err().to_string(),
            "Unsupported type: Expected type: string, number, integer, object, or array, but got: foo"
        );
    }

    #[test]
    fn test_type_string() {
        let docs = MarkedYaml::load_from_str("type: string").unwrap();
        let root_schema = load_from_doc(docs.first().unwrap()).unwrap();
        let YamlSchema::Subschema(subschema) = &root_schema.schema else {
            panic!("Expected Subschema, but got: {:?}", &root_schema.schema);
        };
        assert_eq!(subschema.r#type, Some(SchemaType::single("string")));
    }

    #[test]
    fn test_type_object_with_string_with_description() {
        let root_schema = loader::load_from_str(
            r#"
            type: object
            properties:
                name:
                    type: string
                    description: This is a description
        "#,
        )
        .expect("Failed to load schema");
        let YamlSchema::Subschema(subschema) = &root_schema.schema else {
            panic!("Expected Subschema, but got: {:?}", &root_schema.schema);
        };
        let Some(object_schema) = &subschema.object_schema else {
            panic!(
                "Expected ObjectSchema, but got: {:?}",
                &subschema.object_schema
            );
        };
        let name_property = object_schema
            .properties
            .as_ref()
            .expect("Expected properties")
            .get("name")
            .expect("Expected `name` property");

        let YamlSchema::Subschema(name_property_schema) = &name_property else {
            panic!(
                "Expected Subschema for `name` property, but got: {:?}",
                &name_property
            );
        };
        assert_eq!(
            name_property_schema.r#type,
            Some(SchemaType::single("string"))
        );
        assert_eq!(
            name_property_schema.string_schema,
            Some(StringSchema::default())
        );
        assert_eq!(
            name_property_schema.metadata_and_annotations.description,
            Some("This is a description".to_string())
        );
    }

    #[test]
    fn test_type_string_with_pattern() {
        let root_schema = loader::load_from_str(
            r#"
        type: string
        pattern: "^(\\([0-9]{3}\\))?[0-9]{3}-[0-9]{4}$"
        "#,
        )
        .unwrap();
        let YamlSchema::Subschema(subschema) = &root_schema.schema else {
            panic!("Expected Subschema, but got: {:?}", &root_schema.schema);
        };
        assert_eq!(subschema.r#type, Some(SchemaType::single("string")));
        let expected = StringSchema {
            pattern: Some(Regex::new("^(\\([0-9]{3}\\))?[0-9]{3}-[0-9]{4}$").unwrap()),
            ..Default::default()
        };

        assert_eq!(subschema.string_schema, Some(expected));
    }

    #[test]
    fn test_integer_schema() {
        let root_schema = loader::load_from_str("type: integer").unwrap();
        let YamlSchema::Subschema(subschema) = &root_schema.schema else {
            panic!("Expected Subschema, but got: {:?}", &root_schema.schema);
        };
        let integer_schema = IntegerSchema::default();
        assert_eq!(subschema.integer_schema, Some(integer_schema));
    }

    #[test]
    fn test_enum() {
        let root_schema = loader::load_from_str(
            r#"
        enum:
          - foo
          - bar
          - baz
        "#,
        )
        .unwrap();
        let enum_values = ["foo", "bar", "baz"]
            .iter()
            .map(|s| ConstValue::string(s.to_string()))
            .collect();
        let YamlSchema::Subschema(subschema) = &root_schema.schema else {
            panic!("Expected Subschema, but got: {:?}", &root_schema.schema);
        };
        assert_eq!(subschema.r#enum, Some(enum_values));
    }

    #[test]
    fn test_enum_without_type() {
        let root_schema = loader::load_from_str(
            r#"
            enum:
              - red
              - amber
              - green
              - null
              - 42
            "#,
        )
        .unwrap();
        let enum_values = vec![
            ConstValue::string("red".to_string()),
            ConstValue::string("amber".to_string()),
            ConstValue::string("green".to_string()),
            ConstValue::null(),
            ConstValue::integer(42),
        ];
        let YamlSchema::Subschema(subschema) = &root_schema.schema else {
            panic!("Expected Subschema, but got: {:?}", &root_schema.schema);
        };
        assert_eq!(subschema.r#enum, Some(enum_values));
    }

    #[test]
    fn test_defs() {
        let root_schema = loader::load_from_str(
            r##"
            $defs:
              foo:
                type: boolean
            "##,
        )
        .unwrap();
        let YamlSchema::Subschema(subschema) = &root_schema.schema else {
            panic!("Expected Subschema, but got: {:?}", &root_schema.schema);
        };
        assert!(subschema.defs.is_some());
        let Some(defs) = &subschema.defs else {
            panic!("Expected defs, but got: {:?}", &subschema.defs);
        };
        assert_eq!(defs.len(), 1);
        assert_eq!(defs.get("foo"), Some(&YamlSchema::typed_boolean()));
    }

    #[test]
    fn test_one_of_with_ref() {
        let root_schema = loader::load_from_str(
            r##"
            $defs:
              foo:
                type: boolean
            oneOf:
              - type: string
              - $ref: "#/$defs/foo"
            "##,
        )
        .unwrap();
        println!("root_schema: {root_schema:?}");
        let YamlSchema::Subschema(subschema) = &root_schema.schema else {
            panic!("Expected Subschema, but got: {:?}", &root_schema.schema);
        };
        assert!(subschema.one_of.is_some());
        let Some(one_of) = &subschema.one_of else {
            panic!("Expected oneOf, but got: {:?}", &subschema.one_of);
        };
        assert_eq!(one_of.one_of.len(), 2);
        assert_eq!(
            one_of.one_of[0],
            YamlSchema::typed_string(StringSchema::default()),
            "one_of[0] should be a string schema"
        );
        assert_eq!(
            one_of.one_of[1],
            YamlSchema::ref_str("#/$defs/foo"),
            "one_of[1] should be a reference to '#/$defs/foo'"
        );

        let s = r#"
        false
        "#;
        let docs = MarkedYaml::load_from_str(s).unwrap();
        let value = docs.first().unwrap();
        let context = crate::Context::with_root_schema(&root_schema, true);
        let result = root_schema.validate(&context, value);
        println!("result: {result:?}");
        assert!(result.is_ok());
        for error in context.errors.borrow().iter() {
            println!("error: {error:?}");
        }
        assert!(!context.has_errors());
    }

    #[test]
    fn test_self_validate() -> Result<()> {
        let schema_filename = "yaml-schema.yaml";
        let root_schema = match loader::load_file(schema_filename) {
            Ok(schema) => schema,
            Err(e) => {
                eprintln!("Failed to read YAML schema file: {schema_filename}");
                log::error!("{e}");
                return Err(e);
            }
        };

        let yaml_contents = std::fs::read_to_string(schema_filename)?;

        let context = Engine::evaluate(&root_schema, &yaml_contents, false)?;
        if context.has_errors() {
            for error in context.errors.borrow().iter() {
                eprintln!("{error}");
            }
        }
        assert!(!context.has_errors());

        Ok(())
    }

    #[test]
    fn test_download_from_url() {
        // This is an integration test that requires internet access
        if std::env::var("CI").is_ok() {
            // Skip in CI environments if needed
            return;
        }

        let result = std::panic::catch_unwind(|| {
            let url = "https://yaml-schema.net/yaml-schema.yaml";
            let result = download_from_url(url, Some(10));

            // Verify the download and parse was successful
            let root_schema = result.expect("Failed to download and parse YAML schema from URL");

            // Verify we got a valid schema with expected properties
            let YamlSchema::Subschema(subschema) = &root_schema.schema else {
                panic!("Expected Subschema, but got: {:?}", &root_schema.schema);
            };
            assert_eq!(subschema.r#type, Some(SchemaType::single("object")));
            assert!(subschema.object_schema.is_some());

            // Verify the local schema is valid against the downloaded schema
            if let Ok(local_schema) = std::fs::read_to_string("yaml-schema.yaml") {
                let context = Engine::evaluate(&root_schema, &local_schema, false);
                if let Ok(ctx) = context {
                    if ctx.has_errors() {
                        for error in ctx.errors.borrow().iter() {
                            eprintln!("Validation error: {}", error);
                        }
                        panic!("Downloaded schema failed validation against local schema");
                    }
                } else if let Err(e) = context {
                    panic!("Failed to validate downloaded schema: {}", e);
                }
            }
        });

        if let Err(e) = result {
            // If the test fails due to network issues, mark it as passed with a warning
            if let Some(s) = e.downcast_ref::<String>()
                && (s.contains("Network is unreachable")
                    || s.contains("failed to lookup address information"))
            {
                eprintln!("Warning: Network unreachable, skipping download test");
                return;
            }

            // Re-panic if the failure wasn't network-related
            std::panic::resume_unwind(e);
        }
    }
}
