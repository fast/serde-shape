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
use serde_shape::DeserializeFieldShape;
use serde_shape::DeserializeShape;
use serde_shape::DeserializeShapeGraph;
use serde_shape::DeserializeStructShape;
use serde_shape::DeserializeVariantContent;
use serde_shape::FieldWireShape;
use serde_shape::FieldsStyle;
use serde_shape::ShapeId;
use serde_shape::ShapeRef;
use serde_shape::Tagging;
use toml_edit::DocumentMut;

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
    let paths = env_paths::<ClientConfig>("APP_CONFIG");
    assert_eq!(paths.len(), 4);
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
        let path = paths
            .get(env_name)
            .expect("generated environment option should exist");
        assert_eq!(path, &expected_path);
        set_toml_path(&mut document, path, value);
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
    let paths = env_paths::<ModeConfig>("APP_CONFIG");
    let path = paths
        .get("APP_CONFIG_MODE_KIND")
        .expect("generated tag option should exist");
    assert_eq!(paths.len(), 1);
    assert_eq!(path, &["mode", "kind"]);

    let mut document = r#"
        [mode]
        kind = "fast"
    "#
    .parse::<DocumentMut>()
    .expect("config should be valid TOML");
    set_toml_path(&mut document, path, toml_edit::value("safe"));

    let config = ModeConfig::deserialize(document.into_deserializer())
        .expect("edited TOML should deserialize");
    assert_eq!(
        config,
        ModeConfig {
            mode: ExecutionMode::Safe,
        }
    );
}

fn env_paths<T: DeserializeShape>(prefix: &str) -> BTreeMap<String, Vec<String>> {
    let graph = T::deserialize_shape();
    let mut collector = EnvPathCollector {
        graph: &graph,
        prefix,
        paths: BTreeMap::new(),
    };
    collector.visit(graph.root(), &mut Vec::new(), &mut Vec::new());
    collector.paths
}

struct EnvPathCollector<'a> {
    graph: &'a DeserializeShapeGraph,
    prefix: &'a str,
    paths: BTreeMap<String, Vec<String>>,
}

impl EnvPathCollector<'_> {
    fn visit(
        &mut self,
        shape_ref: &ShapeRef,
        path: &mut Vec<String>,
        definition_stack: &mut Vec<ShapeId>,
    ) {
        match shape_ref {
            ShapeRef::Option(inner) => self.visit(inner, path, definition_stack),
            ShapeRef::Definition(id) => self.visit_definition(*id, path, definition_stack),
            _ => self.record(path),
        }
    }

    fn visit_definition(
        &mut self,
        id: ShapeId,
        path: &mut Vec<String>,
        definition_stack: &mut Vec<ShapeId>,
    ) {
        if definition_stack.contains(&id) {
            return;
        }
        definition_stack.push(id);

        let definition = self.graph.definition(id).expect("shape definition exists");
        match &definition.kind {
            DeserializeDefinitionKind::Struct(shape) => {
                self.visit_struct(shape, path, definition_stack);
            }
            DeserializeDefinitionKind::Enum(shape) => {
                self.visit_enum(shape, path, definition_stack);
            }
            DeserializeDefinitionKind::Opaque(_) => self.record(path),
        }

        definition_stack.pop();
    }

    fn visit_struct(
        &mut self,
        shape: &DeserializeStructShape,
        path: &mut Vec<String>,
        definition_stack: &mut Vec<ShapeId>,
    ) {
        match shape.style {
            FieldsStyle::Struct => {
                for field in &shape.fields {
                    self.visit_field(field, path, definition_stack);
                }
            }
            FieldsStyle::Newtype if shape.fields.len() == 1 => {
                self.visit_newtype(&shape.fields[0].wire_shape, path, definition_stack);
            }
            FieldsStyle::Tuple | FieldsStyle::Newtype | FieldsStyle::Unit => self.record(path),
        }
    }

    fn visit_enum(
        &mut self,
        shape: &DeserializeEnumShape,
        path: &mut Vec<String>,
        definition_stack: &mut Vec<ShapeId>,
    ) {
        let Tagging::Internal { tag } = &shape.repr else {
            self.record(path);
            return;
        };

        path.push((*tag).to_owned());
        self.record(path);
        path.pop();

        for variant in &shape.variants {
            match &variant.content {
                DeserializeVariantContent::Omitted => {}
                DeserializeVariantContent::Fields(fields)
                    if variant.style == FieldsStyle::Newtype && fields.len() == 1 =>
                {
                    self.visit_newtype(&fields[0].wire_shape, path, definition_stack);
                }
                DeserializeVariantContent::Fields(fields) => {
                    for field in fields {
                        self.visit_field(field, path, definition_stack);
                    }
                }
                DeserializeVariantContent::Custom(_) => self.record(path),
                _ => self.record(path),
            }
        }
    }

    fn visit_field(
        &mut self,
        field: &DeserializeFieldShape,
        path: &mut Vec<String>,
        definition_stack: &mut Vec<ShapeId>,
    ) {
        match &field.wire_shape {
            FieldWireShape::Omitted => {}
            FieldWireShape::Value(shape_ref) => {
                path.push(field.name.to_owned());
                self.visit(shape_ref, path, definition_stack);
                path.pop();
            }
            FieldWireShape::Flatten(shape_ref) | FieldWireShape::Inline(shape_ref) => {
                self.visit(shape_ref, path, definition_stack);
            }
            _ => {
                path.push(field.name.to_owned());
                self.record(path);
                path.pop();
            }
        }
    }

    fn visit_newtype(
        &mut self,
        wire_shape: &FieldWireShape,
        path: &mut Vec<String>,
        definition_stack: &mut Vec<ShapeId>,
    ) {
        match wire_shape {
            FieldWireShape::Omitted => {}
            FieldWireShape::Value(shape_ref)
            | FieldWireShape::Flatten(shape_ref)
            | FieldWireShape::Inline(shape_ref) => {
                self.visit(shape_ref, path, definition_stack);
            }
            _ => self.record(path),
        }
    }

    fn record(&mut self, path: &[String]) {
        if path.is_empty() {
            return;
        }
        self.paths
            .entry(env_name(self.prefix, path))
            .or_insert_with(|| path.to_vec());
    }
}

fn set_toml_path(document: &mut DocumentMut, path: &[String], value: toml_edit::Item) {
    let (key, parents) = path.split_last().expect("config path should not be empty");
    let mut current = document.as_item_mut();
    for parent in parents {
        current = &mut current[parent.as_str()];
    }
    current[key.as_str()] = value;
}

fn env_name(prefix: &str, path: &[String]) -> String {
    let suffix = path
        .join("_")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("{prefix}_{suffix}")
}
