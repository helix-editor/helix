use std::collections::HashMap;

use schemars::schema_for;
use serde;

#[derive(Debug)]
pub struct Options {
    options: HashMap<String, Option>,
}

#[derive(Debug)]
pub struct Documentation {
    pub default: std::option::Option<String>,
    pub description: std::option::Option<String>,
}

#[derive(Debug)]
struct Option {
    documentation: Documentation,
    validation: std::option::Option<Validation>,
}

#[derive(Debug)]
struct Validation {
    kind: std::option::Option<OptionKind>,
    one_of: std::option::Option<Vec<String>>,
}

#[derive(Debug)]
enum OptionKind {
    Boolean,
    Numeric,
}

#[derive(Default)]
struct OptionBuilder {
    default: std::option::Option<String>,
    description: std::option::Option<String>,
    validation: std::option::Option<Validation>,
}

impl Option {
    fn builder() -> OptionBuilder {
        OptionBuilder::default()
    }
}

impl OptionBuilder {
    fn with_description<T>(mut self, description: T) -> Self
    where
        T: Into<std::option::Option<String>>,
    {
        self.description = description.into();
        self
    }

    fn with_default<T>(mut self, default: T) -> Self
    where
        T: Into<std::option::Option<String>>,
    {
        self.default = default.into();
        self
    }

    fn must_be_boolean(mut self) -> Self {
        // TODO: Now that we derive this, this is probably overkill.
        self.validation = Some(Validation {
            kind: Some(OptionKind::Boolean),
            one_of: None,
        });
        self
    }

    fn must_be_numeric(mut self) -> Self {
        self.validation = Some(Validation {
            kind: Some(OptionKind::Numeric),
            one_of: None,
        });
        self
    }

    fn must_be_one_of(mut self, values: Vec<String>) -> Self {
        self.validation = Some(Validation {
            kind: None,
            one_of: Some(values),
        });
        self
    }

    fn build(self) -> Option {
        Option {
            documentation: Documentation {
                description: self.description,
                default: self.default,
            },
            validation: self.validation,
        }
    }
}

// `from_config` translates the embedded JSON Schema into a minimal representation
// of the configuration options available to be set at runtime.
pub fn from_config() -> anyhow::Result<Options> {
    use schemars::schema::*;

    let mut options = HashMap::new();
    let root_schema: RootSchema = schema_for!(helix_view::editor::Config);

    fn bool_to_option(metadata: Metadata) -> Option {
        Option::builder()
            .with_default(metadata.default.unwrap().as_bool().map(|b| b.to_string()))
            .with_description(metadata.description)
            .must_be_boolean()
            .build()
    }

    fn number_to_option(metadata: Metadata) -> Option {
        Option::builder()
            .with_default(metadata.default.unwrap().as_u64().map(|n| n.to_string()))
            .with_description(metadata.description)
            .must_be_numeric()
            .build()
    }

    fn enum_to_option(schemas: &Vec<Schema>, metadata: Metadata) -> Option {
        Option::builder()
            .with_default(
                metadata
                    .default
                    // We add _or_default() here because enums with manual Default implementations
                    // are not picked up by JsonSchema derive. Example: CursorKind.
                    .unwrap_or_default()
                    .as_str()
                    .map(|s| s.to_owned()),
            )
            .with_description(metadata.description)
            .must_be_one_of(
                schemas
                    .iter()
                    .map(|s| s.clone().into_object())
                    .flat_map(|s| s.enum_values.unwrap_or_default())
                    .map(|v| v.as_str().unwrap().to_owned())
                    .collect(),
            )
            .build()
    }

    // For options where we don't have runtime validation on :set-option, still extract
    // the default and doc comment for use in the editor and during documentation generation.
    fn no_validation_option(metadata: Metadata) -> Option {
        Option::builder()
            // REVIEW: This formats the default as JSON. This is probably correct or close most of the time.
            .with_default(metadata.default.map(|v| v.to_string()))
            .with_description(metadata.description)
            .build()
    }

    fn qualify_prefix(prefix: &Vec<String>, node: String) -> String {
        let mut prefix = prefix.clone();
        prefix.push(node);
        prefix.join(".")
    }

    fn traverse_schema(
        prefix: Vec<String>,
        root_schema: &RootSchema,
        mut schema: SchemaObject,
        options: &mut HashMap<String, Option>,
    ) {
        use schemars::schema::SingleOrVec::{Single, Vec};

        for (key, value) in &schema.object().properties {
            let qualified_option_name = qualify_prefix(&prefix, key.to_owned());
            log::debug!("inspecting schema for {}", qualified_option_name);

            match value {
                Schema::Object(object) => {
                    let mut object = object.clone();
                    let metadata = object.clone().metadata().clone();

                    match &object.instance_type {
                        Some(Single(t)) if **t == InstanceType::Boolean => {
                            options.insert(qualified_option_name, bool_to_option(metadata));
                        }

                        Some(Single(t)) if **t == InstanceType::Integer => {
                            options.insert(qualified_option_name, number_to_option(metadata));
                        }

                        Some(Single(t)) if **t == InstanceType::Array => {
                            options.insert(qualified_option_name, no_validation_option(metadata));
                        }

                        Some(Single(s)) if **s == InstanceType::String => {
                            // TODO: We could adapt the string validation from JsonSchema for min/max
                            //       length and patterns. For `char` types it correctly sets length 1,
                            //       for example.
                            options.insert(qualified_option_name, no_validation_option(metadata));
                        }

                        // TODO: Other optional types could be supported.
                        Some(Vec(v)) if **v == vec![InstanceType::Integer, InstanceType::Null] => {
                            // TODO: This isn't quite right as "null" is currently accepted.
                            options.insert(qualified_option_name, number_to_option(metadata));
                        }

                        // This is what happens when another enum or struct is referenced.
                        None if object.subschemas.is_some() => {
                            // If we have an allOf schema, we might be an enum.
                            if let Some(schemas) = &object.subschemas().all_of {
                                let reference =
                                    schemas.first().map(|s| s.clone().into_object().reference);

                                if let Some(Some(reference_id)) = reference {
                                    let reference_id = reference_id.split("/").last().unwrap();

                                    let mut definition = root_schema
                                        .definitions
                                        .get(reference_id)
                                        .unwrap()
                                        .clone()
                                        .into_object();

                                    // If the definition has a subschema oneOf, it's likely an enum.
                                    if let Some(schemas) = &definition.subschemas().one_of {
                                        options.insert(
                                            key.to_owned(),
                                            enum_to_option(schemas, metadata),
                                        );
                                    } else if definition.object.is_some() {
                                        let mut prefix = prefix.clone();
                                        prefix.push(key.to_owned());

                                        log::debug!("recursing: {:?}", prefix);

                                        // If the definition has object validations, it's a sub-structure we
                                        // should recurse into.
                                        traverse_schema(prefix, &root_schema, definition, options);
                                    } else {
                                        log::debug!(
                                            "unhandled referenced subschema: {}:\n{:#?}",
                                            qualified_option_name,
                                            definition.subschemas()
                                        );
                                    }
                                } else {
                                    log::debug!(
                                        "invalid reference_id for {}: {:?}",
                                        qualified_option_name,
                                        reference
                                    );
                                }
                            } else {
                                log::debug!(
                                    "unhandled subschema type: {}:\n{:#?}",
                                    qualified_option_name,
                                    object.subschemas()
                                );
                            }
                        }
                        _ => {
                            log::debug!("option cannot be parsed: {}", qualified_option_name);
                            log::debug!("\n{:#?}", object);
                        }
                    }
                }
                _ => {
                    log::debug!(
                        "option did not have Object schema: {}",
                        qualified_option_name
                    );
                    log::debug!("\n{:#?}", value);
                }
            }
        }
    }

    traverse_schema(
        vec![],
        &root_schema,
        root_schema.schema.clone(),
        &mut options,
    );

    // log::debug!("options: {:?}", options);

    Ok(Options { options })
}

impl Options {
    pub fn get_help(&self, name: &str) -> std::option::Option<&Documentation> {
        self.options.get(name).map(|o| &o.documentation)
    }

    pub fn validate(&self, name: &str, value: &str) -> anyhow::Result<()> {
        match self.options.get(name).map(|o| &o.validation) {
            Some(Some(validation)) => validation.validate(value),

            // If we don't have any knowledge of this option, assume it validates. If it's truly invalid,
            // it will fail to apply to the actual config objects.
            _ => Ok(()),
        }
    }
}

impl Validation {
    fn validate(&self, value: &str) -> anyhow::Result<()> {
        log::debug!("validate: {:?}", self);
        match self.kind {
            Some(OptionKind::Numeric) => {
                if !value.parse::<usize>().is_ok() {
                    anyhow::bail!("value must be numeric");
                }
            }
            Some(OptionKind::Boolean) => {
                if !value.parse::<bool>().is_ok() {
                    anyhow::bail!("value must be one of: true, false");
                }
            }
            None => (),
        }

        if let Some(one_of) = &self.one_of {
            if !one_of.iter().any(|v| v == value) {
                anyhow::bail!("value must be one of: {}", one_of.join(", "));
            }
        }

        Ok(())
    }
}

#[test]
fn test_actual_from_config() {
    from_config().expect("loads ok");
}

// #[test]
// fn test_get_help() {
//     let options = from_str(
//         r#"
//             [my-option]
//             description = "my excellent option"
//             default = "`true`"

//             ["nested.my-option"]
//             description = "into the depths"
//             default = "`false`"
//         "#,
//     )
//     .expect("should parse");

//     assert_eq!(Some("my excellent option"), options.get_help("my-option"));
//     assert_eq!(
//         Some("into the depths"),
//         options.get_help("nested.my-option")
//     );
//     assert_eq!(None, options.get_help("not.a-command"));
// }

// #[test]
// fn test_validations() {
//     let options = from_str(
//         r#"
//             [boolean-only]
//             description = "should be a boolean"
//             default = "`true`"
//             validation = { kind = "boolean" }

//             [numeric-only]
//             description = "should be a number"
//             default = "`4`"
//             validation = { kind = "numeric" }

//             [one-of-a-set]
//             description = "favorite color"
//             default = "`red`"
//             validation = { one-of = ["red", "blue", "green"] }
//         "#,
//     )
//     .expect("should parse");

//     #[rustfmt::skip]
//     let ok_examples = [
//         ("boolean-only", "true"),
//         ("boolean-only", "false"),

//         ("numeric-only", "0"),
//         ("numeric-only", "12319872372"),

//         ("one-of-a-set", "red"),
//         ("one-of-a-set", "blue"),
//         ("one-of-a-set", "green"),
//     ];

//     #[rustfmt::skip]
//     let fail_examples = [
//         ("boolean-only", "purple", "value must be one of: true, false"),

//         ("numeric-only", "-1000", "value must be numeric"),
//         ("numeric-only", "purple", "value must be numeric"),

//         ("one-of-a-set", "purple", "value must be one of: red, blue, green"),
//     ];

//     for (name, value) in ok_examples {
//         options
//             .validate(name, value)
//             .map_err(|_| format!("{} should allow value {}", name, value))
//             .unwrap()
//     }

//     for (name, value, err_msg) in fail_examples {
//         match options.validate(name, value) {
//             Ok(_) => assert!(false, "{} should be invalid for {}", value, name),
//             Err(e) => assert_eq!(err_msg, e.to_string()),
//         }
//     }
// }
