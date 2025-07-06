use regex::Regex;

/// A string schema
#[derive(Debug, Default)]
pub struct StringSchema {
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<Regex>,
    pub r#enum: Option<Vec<String>>,
}

impl StringSchema {
    pub fn builder() -> StringSchemaBuilder {
        StringSchemaBuilder::new()
    }
}

impl PartialEq for StringSchema {
    fn eq(&self, other: &Self) -> bool {
        self.min_length == other.min_length
            && self.max_length == other.max_length
            && are_patterns_equivalent(&self.pattern, &other.pattern)
    }
}

/// 'Naive' check to see if two regexes are equal, by comparing their string representations
/// We do it this way because we can't `impl PartialEq for Regex` and don't want to have to
/// alias or wrap the `regex::Regex` type
fn are_patterns_equivalent(a: &Option<Regex>, b: &Option<Regex>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.as_str() == b.as_str(),
        (None, None) => true,
        _ => false,
    }
}

impl std::fmt::Display for StringSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "StringSchema {{ min_length: {:?}, max_length: {:?}, pattern: {:?} }}",
            self.min_length, self.max_length, self.pattern
        )
    }
}

pub struct StringSchemaBuilder(StringSchema);

impl Default for StringSchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl StringSchemaBuilder {
    pub fn new() -> Self {
        Self(StringSchema::default())
    }

    pub fn build(&mut self) -> StringSchema {
        std::mem::take(&mut self.0)
    }

    pub fn min_length(&mut self, min_length: usize) -> &mut Self {
        self.0.min_length = Some(min_length);
        self
    }

    pub fn max_length(&mut self, max_length: usize) -> &mut Self {
        self.0.max_length = Some(max_length);
        self
    }

    pub fn pattern(&mut self, pattern: Regex) -> &mut Self {
        self.0.pattern = Some(pattern);
        self
    }

    pub fn r#enum(&mut self, r#enum: Vec<String>) -> &mut Self {
        self.0.r#enum = Some(r#enum);
        self
    }

    pub fn add_enum<S>(&mut self, s: S) -> &mut Self
    where
        S: Into<String>,
    {
        if let Some(r#enum) = self.0.r#enum.as_mut() {
            r#enum.push(s.into());
            self
        } else {
            self.r#enum(vec![s.into()])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_schema_builder() {
        let schema = StringSchema::builder()
            .add_enum("foo")
            .add_enum("bar")
            .build();
        assert_eq!(
            StringSchema {
                min_length: None,
                max_length: None,
                pattern: None,
                r#enum: Some(vec!["foo".into(), "bar".into()]),
            },
            schema
        );
    }
}
