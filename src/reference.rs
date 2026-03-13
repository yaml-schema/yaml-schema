use log::debug;
use saphyr::AnnotatedMapping;
use saphyr::MarkedYaml;
use saphyr::YamlData;
use url::Url;

use crate::Result;
use crate::utils::format_annotated_mapping;

/// Parsed representation of a `$ref` URI for resolution.
/// Supports same-document (#/...), relative (./other.yaml), and absolute (https://..., file:///...) references.
#[derive(Clone, Debug, PartialEq)]
pub struct RefUri {
    /// The reference as given (for display/errors).
    raw: String,
    /// Part before # (empty for same-document).
    base_ref: String,
    /// JSON Pointer fragment after # (e.g. "/$defs/foo"), or None if no fragment.
    fragment: Option<String>,
}

impl RefUri {
    /// Parse a `$ref` string into base and fragment components.
    pub fn parse(ref_str: &str) -> RefUri {
        let raw = ref_str.to_string();
        let (base_ref, fragment) = match ref_str.split_once('#') {
            Some((base, frag)) => (base.to_string(), Some(frag.to_string())),
            None => (ref_str.to_string(), None),
        };
        RefUri {
            raw,
            base_ref,
            fragment,
        }
    }

    /// Returns true if this is a same-document reference (starts with #).
    pub fn is_same_document(&self) -> bool {
        self.raw.starts_with('#')
    }

    /// Returns true if the base part of the reference is an absolute URI (has a scheme).
    pub fn is_absolute(&self) -> bool {
        Url::parse(&self.base_ref).is_ok()
    }

    /// Returns the JSON Pointer fragment (e.g. "/$defs/foo") if present.
    pub fn fragment(&self) -> Option<&str> {
        self.fragment.as_deref()
    }

    /// Returns the part before # (base URI or path).
    pub fn base_ref(&self) -> &str {
        &self.base_ref
    }

    /// Returns the raw reference string.
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// Resolve this reference against a base URL (RFC 3986).
    /// For same-document refs, returns an error (use fragment resolution instead).
    /// For relative refs, joins with base. For absolute refs, parses and optionally appends fragment.
    pub fn resolve_against(&self, base: &Url) -> Result<Url> {
        if self.is_same_document() {
            return Err(crate::generic_error!(
                "Cannot resolve same-document ref against base URI: {}",
                self.raw
            ));
        }
        let mut resolved = base.join(&self.base_ref).map_err(|e| {
            crate::generic_error!(
                "Failed to resolve $ref {} against base {}: {}",
                self.raw,
                base,
                e
            )
        })?;
        if let Some(frag) = &self.fragment {
            resolved.set_fragment(Some(frag));
        }
        Ok(resolved)
    }
}

/// A Reference is a reference to another schema, usually one that is
/// declared in the `$defs` section of the root schema.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reference {
    pub ref_name: String,
}

impl std::fmt::Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "$ref: {}", self.ref_name)
    }
}

impl Reference {
    pub fn new(ref_name: impl Into<String>) -> Self {
        Self {
            ref_name: ref_name.into(),
        }
    }
}

impl<'r> TryFrom<&MarkedYaml<'r>> for Reference {
    type Error = crate::Error;

    fn try_from(value: &MarkedYaml<'r>) -> std::result::Result<Self, Self::Error> {
        if let YamlData::Mapping(mapping) = &value.data {
            Self::try_from(mapping)
        } else {
            Err(expected_mapping!(value))
        }
    }
}

impl<'r> TryFrom<&AnnotatedMapping<'r, MarkedYaml<'r>>> for Reference {
    type Error = crate::Error;

    fn try_from(mapping: &AnnotatedMapping<'r, MarkedYaml<'r>>) -> crate::Result<Self> {
        debug!("[Reference#try_from] {}", format_annotated_mapping(mapping));
        let ref_key = MarkedYaml::value_from_str("$ref");
        if let Some(ref_value) = mapping.get(&ref_key) {
            match &ref_value.data {
                YamlData::Value(saphyr::Scalar::String(s)) => {
                    Ok(Reference::new(s.as_ref().to_string()))
                }
                _ => Err(generic_error!(
                    "Expected a string value for $ref, but got: {:?}",
                    ref_value
                )),
            }
        } else {
            Err(generic_error!(
                "No $ref key found in mapping: {}",
                format_annotated_mapping(mapping)
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use saphyr::LoadableYamlNode;

    use crate::Validator as _;
    use crate::YamlSchema;
    use crate::loader;

    use super::RefUri;

    #[test]
    fn test_reference() {
        let schema = r##"
            $defs:
                name:
                    type: string
            type: object
            properties:
                name:
                    $ref: "#/$defs/name"
        "##;
        let root_schema = loader::load_from_str(schema).expect("Failed to load schema");
        let YamlSchema::Subschema(subschema) = &root_schema.schema else {
            panic!("Expected a subschema");
        };

        // Assert that the subschema has the expected structure
        // 1. Verify it's an object type
        assert!(
            subschema.r#type.is_or_contains("object"),
            "Expected object type"
        );

        // 2. Verify $defs contains "name" definition
        let defs = subschema
            .defs
            .as_ref()
            .expect("Expected $defs to be present");
        let name_def = defs.get("name").expect("Expected 'name' in $defs");

        // 3. Verify the "name" definition is a string schema
        let YamlSchema::Subschema(name_subschema) = name_def else {
            panic!("Expected name definition to be a subschema");
        };
        assert!(
            name_subschema.r#type.is_or_contains("string"),
            "Expected name definition to be a string type"
        );

        // 4. Verify object_schema exists with properties
        let object_schema = subschema
            .object_schema
            .as_ref()
            .expect("Expected object_schema");
        let properties = object_schema
            .properties
            .as_ref()
            .expect("Expected properties");

        // 5. Verify properties contains "name" with a reference
        let name_property = properties.get("name").expect("Expected 'name' property");
        let YamlSchema::Subschema(name_prop_subschema) = name_property else {
            panic!("Expected name property to be a subschema");
        };

        // 6. Verify the name property is a reference to "#/$defs/name"
        let ref_value = name_prop_subschema
            .r#ref
            .as_ref()
            .expect("Expected $ref in name property");
        assert_eq!(
            ref_value.ref_name, "#/$defs/name",
            "Expected reference to '#/$defs/name'"
        );

        let context = crate::Context::with_root_schema(&root_schema, true);
        let value = r##"
            name: "John Doe"
        "##;
        let docs = saphyr::MarkedYaml::load_from_str(value).unwrap();
        let value = docs.first().unwrap();
        let result = root_schema.validate(&context, value);
        assert!(result.is_ok());
        assert!(!context.has_errors());
    }

    #[test]
    fn test_json_ptr() {
        let ptr = jsonptr::Pointer::parse("/$defs/schema").expect("Failed to parse JSON pointer");
        let components: Vec<_> = ptr.components().collect();
        assert!(!components.is_empty());
    }

    #[test]
    fn test_circular_reference_direct() {
        let schema = r##"
            $defs:
                a:
                    $ref: "#/$defs/a"
            $ref: "#/$defs/a"
        "##;
        let root_schema = loader::load_from_str(schema).expect("Failed to load schema");
        let context = crate::Context::with_root_schema(&root_schema, false);
        let docs = saphyr::MarkedYaml::load_from_str("test").unwrap();
        let value = docs.first().unwrap();
        let result = root_schema.validate(&context, value);
        assert!(result.is_ok());
        assert!(context.has_errors());
        let errors = context.errors.borrow();
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0].error.contains("Circular $ref detected"),
            "Expected circular ref error, got: {}",
            errors[0].error
        );
    }

    #[test]
    fn test_circular_reference_indirect() {
        let schema = r##"
            $defs:
                a:
                    $ref: "#/$defs/b"
                b:
                    $ref: "#/$defs/a"
            $ref: "#/$defs/a"
        "##;
        let root_schema = loader::load_from_str(schema).expect("Failed to load schema");
        let context = crate::Context::with_root_schema(&root_schema, false);
        let docs = saphyr::MarkedYaml::load_from_str("test").unwrap();
        let value = docs.first().unwrap();
        let result = root_schema.validate(&context, value);
        assert!(result.is_ok());
        assert!(context.has_errors());
        let errors = context.errors.borrow();
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0].error.contains("Circular $ref detected"),
            "Expected circular ref error, got: {}",
            errors[0].error
        );
    }

    #[test]
    fn test_non_circular_ref_still_works() {
        let schema = r##"
            $defs:
                name:
                    type: string
            type: object
            properties:
                first_name:
                    $ref: "#/$defs/name"
                last_name:
                    $ref: "#/$defs/name"
        "##;
        let root_schema = loader::load_from_str(schema).expect("Failed to load schema");
        let context = crate::Context::with_root_schema(&root_schema, false);
        let value = r#"
            first_name: "Alice"
            last_name: "Smith"
        "#;
        let docs = saphyr::MarkedYaml::load_from_str(value).unwrap();
        let value = docs.first().unwrap();
        let result = root_schema.validate(&context, value);
        assert!(result.is_ok());
        assert!(
            !context.has_errors(),
            "Expected no errors, got: {:?}",
            context.errors.borrow()
        );
    }

    #[test]
    fn test_ref_uri_same_document() {
        let r = RefUri::parse("#/$defs/name");
        assert!(r.is_same_document());
        assert_eq!(r.fragment(), Some("/$defs/name"));
        assert_eq!(r.base_ref(), "");
    }

    #[test]
    fn test_ref_uri_relative_with_fragment() {
        let r = RefUri::parse("./common.yaml#/$defs/Id");
        assert!(!r.is_same_document());
        assert_eq!(r.fragment(), Some("/$defs/Id"));
        assert_eq!(r.base_ref(), "./common.yaml");
    }

    #[test]
    fn test_ref_uri_absolute() {
        let r = RefUri::parse("https://example.com/schema.yaml#/$defs/User");
        assert!(!r.is_same_document());
        assert!(r.is_absolute());
        assert_eq!(r.fragment(), Some("/$defs/User"));
        assert_eq!(r.base_ref(), "https://example.com/schema.yaml");
    }

    #[test]
    fn test_ref_uri_is_absolute() {
        assert!(RefUri::parse("https://example.com/schema.yaml").is_absolute());
        assert!(RefUri::parse("http://example.com/s.yaml#/$defs/X").is_absolute());
        assert!(RefUri::parse("file:///tmp/schema.yaml").is_absolute());
        assert!(!RefUri::parse("./common.yaml#/$defs/Id").is_absolute());
        assert!(!RefUri::parse("#/$defs/name").is_absolute());
        assert!(!RefUri::parse("common.yaml").is_absolute());
    }

    #[test]
    fn test_ref_uri_resolve_relative() {
        let r = RefUri::parse("./other.yaml#/$defs/foo");
        let base = url::Url::parse("file:///dir/schema.yaml").unwrap();
        let resolved = r.resolve_against(&base).unwrap();
        assert!(resolved.as_str().contains("other.yaml"));
        assert_eq!(resolved.fragment(), Some("/$defs/foo"));
    }

    #[test]
    fn test_ref_accepts_external_ref() {
        let schema = r##"
            type: object
            properties:
                id:
                    $ref: "./common.yaml#/$defs/Id"
        "##;
        let root_schema =
            loader::load_from_str(schema).expect("Should load schema with external $ref");
        let YamlSchema::Subschema(subschema) = &root_schema.schema else {
            panic!("Expected Subschema");
        };
        let object_schema = subschema.object_schema.as_ref().unwrap();
        let name_property = object_schema
            .properties
            .as_ref()
            .unwrap()
            .get("id")
            .unwrap();
        let YamlSchema::Subschema(prop_schema) = name_property else {
            panic!("Expected Subschema for id property");
        };
        let ref_val = prop_schema.r#ref.as_ref().unwrap();
        assert_eq!(ref_val.ref_name, "./common.yaml#/$defs/Id");
    }
}
