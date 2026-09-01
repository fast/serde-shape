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

//! Inspects a Serde configuration model and prints an environment-variable reference.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::Deserialize;
use serde_shape::DeserializeDefinitionKind;
use serde_shape::DeserializeFieldShape;
use serde_shape::DeserializeShape;
use serde_shape::DeserializeShapeGraph;
use serde_shape::DeserializeVariantContent;
use serde_shape::FieldWireShape;
use serde_shape::ShapeId;
use serde_shape::ShapeRef;
use serde_shape::Tagging;

#[derive(Deserialize, DeserializeShape)]
struct ApplicationConfig {
    #[serde(flatten)]
    runtime: RuntimeConfig,
    storage: StorageConfig,
    #[serde(default)]
    telemetry: Option<TelemetryConfig>,
}

#[derive(Deserialize, DeserializeShape)]
#[serde(rename_all = "kebab-case")]
struct RuntimeConfig {
    /// Address on which the application accepts requests.
    listen_address: String,
    /// Number of worker threads used by the application.
    worker_threads: usize,
}

#[derive(Deserialize, DeserializeShape)]
#[serde(
    tag = "backend",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case"
)]
enum StorageConfig {
    Memory,
    Local {
        /// Directory used to store application data.
        directory: String,
        /// Prevents the application from modifying stored data.
        #[serde(default)]
        read_only: bool,
    },
}

#[derive(Deserialize, DeserializeShape)]
struct TelemetryConfig {
    /// Destination for exported telemetry.
    endpoint: String,
}

fn main() {
    let graph = ApplicationConfig::deserialize_shape();
    let mut entries = BTreeMap::new();
    let mut visiting = BTreeSet::new();
    collect_shape("", graph.root(), &graph, &mut entries, &mut visiting);

    println!("{:<32} {:<24} TYPE", "ENVIRONMENT VARIABLE", "CONFIG PATH");
    for (path, kind) in entries {
        println!("{:<32} {:<24} {kind}", environment_name(&path), path);
    }
}

fn collect_shape(
    path: &str,
    shape: &ShapeRef,
    graph: &DeserializeShapeGraph,
    entries: &mut BTreeMap<String, &'static str>,
    visiting: &mut BTreeSet<ShapeId>,
) {
    let kind = match shape {
        ShapeRef::Bool => Some("boolean"),
        shape if shape.is_integer() => Some("integer"),
        shape if shape.is_float() => Some("number"),
        ShapeRef::Char | ShapeRef::String => Some("string"),
        ShapeRef::Option(inner) => return collect_shape(path, inner, graph, entries, visiting),
        ShapeRef::Definition(id) => {
            assert!(
                visiting.insert(*id),
                "recursive configuration shapes are not supported by this example"
            );
            let definition = graph
                .definition(*id)
                .expect("definition references should resolve inside their graph");
            match &definition.kind {
                DeserializeDefinitionKind::Struct(shape) => {
                    collect_fields(path, &shape.fields, graph, entries, visiting);
                }
                DeserializeDefinitionKind::Enum(shape) => {
                    let Tagging::Internal { tag } = shape.repr else {
                        panic!("this example supports only internally tagged enums");
                    };
                    insert_entry(join_path(path, tag), "string", entries);
                    for variant in &shape.variants {
                        match &variant.content {
                            DeserializeVariantContent::Omitted => {}
                            DeserializeVariantContent::Fields(fields) => {
                                collect_fields(path, fields, graph, entries, visiting);
                            }
                            DeserializeVariantContent::Shape(shape) => {
                                collect_shape(path, shape, graph, entries, visiting);
                            }
                            _ => panic!("custom enum variants are not supported by this example"),
                        }
                    }
                }
                DeserializeDefinitionKind::Opaque(_) => {
                    panic!("opaque configuration shapes are not supported by this example");
                }
            }
            visiting.remove(id);
            return;
        }
        _ => panic!("unsupported configuration shape at {path:?}: {shape:?}"),
    };

    insert_entry(
        path.to_owned(),
        kind.expect("leaf shapes should have a kind"),
        entries,
    );
}

fn collect_fields(
    path: &str,
    fields: &[DeserializeFieldShape],
    graph: &DeserializeShapeGraph,
    entries: &mut BTreeMap<String, &'static str>,
    visiting: &mut BTreeSet<ShapeId>,
) {
    for field in fields {
        match &field.wire_shape {
            FieldWireShape::Omitted => {}
            FieldWireShape::Value(shape) => {
                collect_shape(
                    &join_path(path, field.name),
                    shape,
                    graph,
                    entries,
                    visiting,
                );
            }
            FieldWireShape::Flatten(shape) | FieldWireShape::Inline(shape) => {
                collect_shape(path, shape, graph, entries, visiting);
            }
            _ => panic!("unsupported field shape at {path:?}"),
        }
    }
}

fn insert_entry(path: String, kind: &'static str, entries: &mut BTreeMap<String, &'static str>) {
    if let Some(previous) = entries.insert(path.clone(), kind) {
        assert_eq!(previous, kind, "conflicting shapes at {path:?}");
    }
}

fn join_path(prefix: &str, field: &str) -> String {
    if prefix.is_empty() {
        field.to_owned()
    } else {
        format!("{prefix}.{field}")
    }
}

fn environment_name(path: &str) -> String {
    format!("APP_{}", path.to_uppercase().replace(['.', '-'], "_"))
}
