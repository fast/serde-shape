// Copyright 2026 FastLabs Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(dead_code)]

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::de::IntoDeserializer;
use serde_shape::DeserializeDefinitionKind;
use serde_shape::DeserializeEnumShape;
use serde_shape::DeserializeShape;
use serde_shape::DeserializeShapeGraph;
use serde_shape::DeserializeStructShape;
use serde_shape::DeserializeVariantContent;
use serde_shape::FieldWireShape;
use serde_shape::FieldsStyle;
use serde_shape::ShapeId;
use serde_shape::ShapeRef;
use serde_shape::Tagging;
use serde_shape::UnionShape;
use toml_edit::DocumentMut;

#[derive(Clone, Debug, Eq, PartialEq)]
struct EnvOption {
    env_name: String,
    path: Vec<String>,
    value_kind: String,
    optional: bool,
    condition: Option<String>,
}

#[derive(Debug, Deserialize, DeserializeShape, PartialEq)]
#[serde(deny_unknown_fields)]
struct ClientConfig {
    transport: Transport,
}

#[derive(Debug, Deserialize, DeserializeShape, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Transport {
    Tcp(TcpTransport),
}

#[derive(Debug, Deserialize, DeserializeShape, PartialEq)]
#[serde(deny_unknown_fields)]
struct TcpTransport {
    host: String,
    port: u16,
    #[serde(rename = "tls.version")]
    tls_version: String,
}

#[derive(Debug, Deserialize, DeserializeShape, PartialEq)]
#[serde(deny_unknown_fields)]
struct ModeConfig {
    mode: ExecutionMode,
}

#[derive(Debug, Deserialize, DeserializeShape, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ExecutionMode {
    Fast,
    Safe,
}

#[test]
fn edits_an_internally_tagged_newtype_variant_through_generated_paths() {
    let options = env_options::<ClientConfig>("APP_CONFIG");
    let mut document = r#"
        [transport]
        kind = "tcp"
        host = "localhost"
        port = 8080
        "tls.version" = "1.2"
    "#
    .parse::<DocumentMut>()
    .expect("config should be valid TOML");

    let overrides = [
        (
            "APP_CONFIG_TRANSPORT_KIND",
            toml_edit::value("tcp"),
            ["transport", "kind"],
        ),
        (
            "APP_CONFIG_TRANSPORT_HOST",
            toml_edit::value("example.com"),
            ["transport", "host"],
        ),
        (
            "APP_CONFIG_TRANSPORT_PORT",
            toml_edit::value(443),
            ["transport", "port"],
        ),
        (
            "APP_CONFIG_TRANSPORT_TLS_VERSION",
            toml_edit::value("1.3"),
            ["transport", "tls.version"],
        ),
    ];

    for (env_name, value, expected_path) in overrides {
        let option = options
            .iter()
            .find(|option| option.env_name == env_name)
            .expect("generated environment option should exist");
        assert_eq!(option.path, expected_path);
        set_toml_path(&mut document, &option.path, value);
    }

    let config = ClientConfig::deserialize(document.into_deserializer())
        .expect("edited TOML should deserialize");
    assert_eq!(
        config,
        ClientConfig {
            transport: Transport::Tcp(TcpTransport {
                host: "example.com".to_owned(),
                port: 443,
                tls_version: "1.3".to_owned(),
            }),
        }
    );
}

#[test]
fn edits_an_internally_tagged_unit_enum_through_its_tag_path() {
    let options = env_options::<ModeConfig>("APP_CONFIG");
    let option = options
        .iter()
        .find(|option| option.env_name == "APP_CONFIG_MODE_KIND")
        .expect("generated tag option should exist");
    assert_eq!(option.path, ["mode", "kind"]);
    assert_eq!(option.value_kind, "enum[fast|safe]");

    let mut document = r#"
        [mode]
        kind = "fast"
    "#
    .parse::<DocumentMut>()
    .expect("config should be valid TOML");
    set_toml_path(&mut document, &option.path, toml_edit::value("safe"));

    let config = ModeConfig::deserialize(document.into_deserializer())
        .expect("edited TOML should deserialize");
    assert_eq!(
        config,
        ModeConfig {
            mode: ExecutionMode::Safe,
        }
    );
}

fn env_options<T: DeserializeShape>(env_prefix: &str) -> Vec<EnvOption> {
    let shape = DeserializeShapeGraph::for_type::<T>();
    let mut collector = EnvCollector {
        shape: &shape,
        env_prefix,
        options: BTreeMap::new(),
    };
    collector.visit_shape_ref(shape.root(), &mut Vec::new(), false, None);
    collector.options.into_values().collect()
}

struct EnvCollector<'a> {
    shape: &'a DeserializeShapeGraph,
    env_prefix: &'a str,
    options: BTreeMap<Vec<String>, EnvOption>,
}

impl EnvCollector<'_> {
    fn visit_shape_ref(
        &mut self,
        shape_ref: &ShapeRef,
        path: &mut Vec<String>,
        optional: bool,
        condition: Option<String>,
    ) {
        match shape_ref {
            ShapeRef::Option(inner) => {
                self.visit_shape_ref(inner, path, true, condition);
            }
            ShapeRef::Union(union) => {
                let value_kind = self.union_kind(union);
                self.push_leaf(path, &value_kind, optional, condition);
            }
            ShapeRef::Definition(id) => {
                self.visit_definition(*id, path, optional, condition);
            }
            ShapeRef::Seq(_) | ShapeRef::Array { .. } => {
                self.push_leaf(path, "array", optional, condition);
            }
            ShapeRef::Map { .. } => {
                self.push_leaf(path, "object", optional, condition);
            }
            ShapeRef::Tuple(_) => {
                self.push_leaf(path, "array", optional, condition);
            }
            ShapeRef::Opaque(opaque) => {
                self.push_leaf(
                    path,
                    &format!("opaque({:?})", opaque.reason),
                    optional,
                    condition,
                );
            }
            shape_ref => {
                self.push_leaf(path, primitive_kind(shape_ref), optional, condition);
            }
        }
    }

    fn visit_definition(
        &mut self,
        id: ShapeId,
        path: &mut Vec<String>,
        optional: bool,
        condition: Option<String>,
    ) {
        let definition = self.shape.definition(id).expect("shape definition exists");
        match &definition.kind {
            DeserializeDefinitionKind::Struct(shape) => {
                self.visit_struct(shape, path, optional, condition);
            }
            DeserializeDefinitionKind::Enum(shape) => {
                self.visit_enum(shape, path, optional, condition);
            }
            DeserializeDefinitionKind::Opaque(opaque) => {
                self.push_leaf(
                    path,
                    &format!("opaque({:?})", opaque.reason),
                    optional,
                    condition,
                );
            }
        }
    }

    fn visit_struct(
        &mut self,
        shape: &DeserializeStructShape,
        path: &mut Vec<String>,
        optional: bool,
        condition: Option<String>,
    ) {
        match shape.style {
            FieldsStyle::Struct => {
                for field in &shape.fields {
                    let field_optional = optional || !field.default.is_none();
                    self.visit_field_wire_shape(
                        field.name,
                        &field.wire_shape,
                        path,
                        field_optional,
                        condition.clone(),
                    );
                }
            }
            FieldsStyle::Newtype if shape.fields.len() == 1 => {
                self.visit_newtype_wire_shape(
                    &shape.fields[0].wire_shape,
                    path,
                    optional,
                    condition,
                );
            }
            FieldsStyle::Tuple | FieldsStyle::Newtype | FieldsStyle::Unit => {
                self.push_leaf(path, "object", optional, condition);
            }
        }
    }

    fn visit_enum(
        &mut self,
        shape: &DeserializeEnumShape,
        path: &mut Vec<String>,
        optional: bool,
        condition: Option<String>,
    ) {
        let variants = shape
            .variants
            .iter()
            .filter(|variant| !matches!(&variant.content, DeserializeVariantContent::Omitted))
            .map(|variant| variant.name)
            .collect::<Vec<_>>();

        let all_variants_are_unit = shape
            .variants
            .iter()
            .filter(|variant| !matches!(&variant.content, DeserializeVariantContent::Omitted))
            .all(|variant| variant.style == FieldsStyle::Unit)
            && !variants.is_empty();

        if matches!(&shape.repr, Tagging::External) && all_variants_are_unit {
            self.push_leaf(
                path,
                &format!("enum[{}]", variants.join("|")),
                optional,
                condition,
            );
            return;
        }

        if let Tagging::Internal { tag } = shape.repr {
            let tag_path = appended_path(path, tag);
            self.push_leaf(
                &tag_path,
                &format!("enum[{}]", variants.join("|")),
                optional,
                condition.clone(),
            );

            for variant in &shape.variants {
                let variant_condition = format!("{}={}", tag_path.join("."), variant.name);
                let variant_condition = Some(merge_conditions(
                    condition.as_deref(),
                    variant_condition.as_str(),
                ));

                match &variant.content {
                    DeserializeVariantContent::Omitted => {}
                    DeserializeVariantContent::Fields(fields)
                        if variant.style == FieldsStyle::Newtype && fields.len() == 1 =>
                    {
                        self.visit_newtype_wire_shape(
                            &fields[0].wire_shape,
                            path,
                            optional,
                            variant_condition,
                        );
                    }
                    DeserializeVariantContent::Fields(fields) => {
                        for field in fields {
                            self.visit_field_wire_shape(
                                field.name,
                                &field.wire_shape,
                                path,
                                optional,
                                variant_condition.clone(),
                            );
                        }
                    }
                    DeserializeVariantContent::Custom(opaque) => {
                        self.push_leaf(
                            path,
                            &format!("opaque({:?})", opaque.reason),
                            optional,
                            variant_condition,
                        );
                    }
                    _ => {
                        self.push_leaf(path, "unsupported", optional, variant_condition);
                    }
                }
            }
            return;
        }

        self.push_leaf(
            path,
            &format!("enum[{}]", variants.join("|")),
            optional,
            condition,
        );
    }

    fn visit_field_wire_shape(
        &mut self,
        field_name: &str,
        wire_shape: &FieldWireShape,
        path: &mut Vec<String>,
        optional: bool,
        condition: Option<String>,
    ) {
        match wire_shape {
            FieldWireShape::Omitted => {}
            FieldWireShape::Value(shape_ref) => {
                path.push(field_name.to_owned());
                self.visit_shape_ref(shape_ref, path, optional, condition);
                path.pop();
            }
            FieldWireShape::Flatten(shape_ref) => {
                self.visit_shape_ref(shape_ref, path, optional, condition);
            }
            FieldWireShape::Inline(shape_ref) => {
                self.visit_shape_ref(shape_ref, path, optional, condition);
            }
            _ => {
                path.push(field_name.to_owned());
                self.push_leaf(path, "unsupported", optional, condition);
                path.pop();
            }
        }
    }

    fn visit_newtype_wire_shape(
        &mut self,
        wire_shape: &FieldWireShape,
        path: &mut Vec<String>,
        optional: bool,
        condition: Option<String>,
    ) {
        match wire_shape {
            FieldWireShape::Omitted => {}
            FieldWireShape::Value(shape_ref)
            | FieldWireShape::Flatten(shape_ref)
            | FieldWireShape::Inline(shape_ref) => {
                self.visit_shape_ref(shape_ref, path, optional, condition);
            }
            _ => {
                self.push_leaf(path, "unsupported", optional, condition);
            }
        }
    }

    fn union_kind(&self, union: &UnionShape) -> String {
        let alternatives = union.alternatives();
        if alternatives.iter().all(ShapeRef::is_integer) {
            return "integer".to_owned();
        }
        if alternatives.iter().all(ShapeRef::is_float) {
            return "float".to_owned();
        }
        if alternatives.iter().all(ShapeRef::is_number) {
            return "number".to_owned();
        }

        alternatives
            .iter()
            .fold(Vec::<String>::new(), |mut kinds, alternative| {
                let kind = self.union_alternative_kind(alternative);
                if !kinds.contains(&kind) {
                    kinds.push(kind);
                }
                kinds
            })
            .join("|")
    }

    fn union_alternative_kind(&self, shape_ref: &ShapeRef) -> String {
        match shape_ref {
            ShapeRef::Option(inner) => self.union_alternative_kind(inner),
            ShapeRef::Seq(_) | ShapeRef::Array { .. } | ShapeRef::Tuple(_) => "array".to_owned(),
            ShapeRef::Map { .. } => "object".to_owned(),
            ShapeRef::Union(union) => self.union_kind(union),
            ShapeRef::Definition(id) => {
                let definition = self.shape.definition(*id).expect("shape definition exists");
                match &definition.kind {
                    DeserializeDefinitionKind::Struct(shape) if shape.attributes.transparent => {
                        shape
                            .fields
                            .iter()
                            .find_map(|field| match &field.wire_shape {
                                FieldWireShape::Inline(inner) => {
                                    Some(self.union_alternative_kind(inner))
                                }
                                FieldWireShape::Omitted
                                | FieldWireShape::Value(_)
                                | FieldWireShape::Flatten(_) => None,
                                _ => None,
                            })
                            .unwrap_or_else(|| "unit".to_owned())
                    }
                    DeserializeDefinitionKind::Struct(shape)
                        if shape.style == FieldsStyle::Newtype && shape.fields.len() == 1 =>
                    {
                        match &shape.fields[0].wire_shape {
                            FieldWireShape::Omitted => "unit".to_owned(),
                            FieldWireShape::Value(inner)
                            | FieldWireShape::Flatten(inner)
                            | FieldWireShape::Inline(inner) => self.union_alternative_kind(inner),
                            _ => "unknown".to_owned(),
                        }
                    }
                    DeserializeDefinitionKind::Struct(_) => "object".to_owned(),
                    DeserializeDefinitionKind::Enum(_) => "enum".to_owned(),
                    DeserializeDefinitionKind::Opaque(opaque) => {
                        format!("opaque({:?})", opaque.reason)
                    }
                }
            }
            ShapeRef::Opaque(opaque) => format!("opaque({:?})", opaque.reason),
            shape_ref => primitive_kind(shape_ref).to_owned(),
        }
    }

    fn push_leaf(
        &mut self,
        path: &[String],
        value_kind: &str,
        optional: bool,
        condition: Option<String>,
    ) {
        if path.is_empty() {
            return;
        }

        let path = path.to_vec();
        self.options
            .entry(path.clone())
            .or_insert_with(|| EnvOption {
                env_name: env_name(self.env_prefix, &path),
                path,
                value_kind: value_kind.to_owned(),
                optional,
                condition,
            });
    }
}

fn appended_path(path: &[String], segment: &str) -> Vec<String> {
    let mut path = path.to_owned();
    path.push(segment.to_owned());
    path
}

fn set_toml_path(document: &mut DocumentMut, path: &[String], value: toml_edit::Item) {
    let (key, parents) = path.split_last().expect("config path should not be empty");
    let mut current = document.as_item_mut();
    for parent in parents {
        current = &mut current[parent.as_str()];
    }
    current[key.as_str()] = value;
}

fn merge_conditions(existing: Option<&str>, new: &str) -> String {
    existing.map_or_else(|| new.to_owned(), |existing| format!("{existing}; {new}"))
}

fn primitive_kind(shape_ref: &ShapeRef) -> &'static str {
    if shape_ref.is_integer() {
        "integer"
    } else if shape_ref.is_float() {
        "float"
    } else if shape_ref.is_number() {
        "number"
    } else {
        match shape_ref {
            ShapeRef::Unit => "unit",
            ShapeRef::Bool => "boolean",
            ShapeRef::Char | ShapeRef::String | ShapeRef::Bytes => "string",
            ShapeRef::Option(_)
            | ShapeRef::Seq(_)
            | ShapeRef::Array { .. }
            | ShapeRef::Map { .. }
            | ShapeRef::Tuple(_)
            | ShapeRef::Union(_)
            | ShapeRef::Definition(_)
            | ShapeRef::Opaque(_) => {
                unreachable!("compound shapes are handled before leaf mapping")
            }
            _ => unreachable!("numeric shapes are handled before leaf mapping"),
        }
    }
}

fn env_name(prefix: &str, path: &[String]) -> String {
    let path = path
        .iter()
        .flat_map(|segment| segment.chars().chain(['_']))
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("{prefix}_{}", path.trim_end_matches('_'))
}
