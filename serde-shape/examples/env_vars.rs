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
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::path::PathBuf;

use serde_shape::DefinitionKind;
use serde_shape::EnumShape;
use serde_shape::FieldsStyle;
use serde_shape::SerdeShape;
use serde_shape::Shape;
use serde_shape::ShapeId;
use serde_shape::ShapeRef;
use serde_shape::StructShape;
use serde_shape::Tagging;

#[derive(Clone, Debug, Eq, PartialEq)]
struct EnvOption {
    env_name: String,
    config_path: String,
    value_kind: String,
    optional: bool,
    condition: Option<String>,
}

#[derive(SerdeShape)]
#[serde(deny_unknown_fields)]
struct Config {
    server: ServerConfig,
    storage: StorageConfig,
    telemetry: TelemetryConfig,
}

#[derive(SerdeShape)]
#[serde(deny_unknown_fields)]
struct ServerConfig {
    #[serde(default = "default_dir")]
    dir: PathBuf,
    #[serde(default = "default_listen_data_addr")]
    listen_data_addr: SocketAddr,
    #[serde(skip_serializing_if = "Option::is_none")]
    advertise_data_addr: Option<SocketAddr>,
    #[serde(default)]
    initial_peers: Vec<String>,
    #[serde(default = "default_cluster_id")]
    cluster_id: String,
}

#[derive(SerdeShape)]
#[serde(deny_unknown_fields)]
struct StorageConfig {
    #[serde(default)]
    backend: StorageBackend,
    #[serde(default = "default_disk_capacity")]
    disk_capacity: ByteSize,
    #[serde(default = "default_memory_capacity")]
    memory_capacity: ByteSize,
    #[serde(skip_serializing_if = "Option::is_none")]
    disk_throttle: Option<DiskThrottle>,
}

#[derive(SerdeShape)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "snake_case"
)]
enum StorageBackend {
    Local { data_dir: PathBuf },
    S3 { bucket: String, region: String },
}

#[derive(SerdeShape)]
#[serde(deny_unknown_fields)]
struct DiskThrottle {
    read_iops: u64,
    write_iops: u64,
    iops_counter: CounterConfig,
}

#[derive(SerdeShape)]
#[serde(deny_unknown_fields)]
struct CounterConfig {
    mode: CounterMode,
    size: NonZeroUsize,
}

#[derive(SerdeShape)]
#[serde(rename_all = "snake_case")]
enum CounterMode {
    Window,
    LeakyBucket,
}

#[derive(SerdeShape)]
#[serde(deny_unknown_fields)]
struct TelemetryConfig {
    #[serde(default)]
    logs: LogsConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    traces: Option<TracesConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metrics: Option<MetricsConfig>,
}

#[derive(SerdeShape)]
#[serde(deny_unknown_fields)]
struct LogsConfig {
    #[serde(flatten)]
    sink: LogSink,
    filter: String,
}

#[derive(SerdeShape)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "snake_case"
)]
enum LogSink {
    File {
        dir: PathBuf,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_files: Option<NonZeroUsize>,
    },
    Stderr,
    Opentelemetry {
        otlp_endpoint: String,
    },
}

#[derive(SerdeShape)]
#[serde(deny_unknown_fields)]
struct TracesConfig {
    capture_log_filter: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    opentelemetry: Option<OpentelemetryTracesConfig>,
}

#[derive(SerdeShape)]
#[serde(deny_unknown_fields)]
struct OpentelemetryTracesConfig {
    otlp_endpoint: String,
}

#[derive(SerdeShape)]
#[serde(deny_unknown_fields)]
struct MetricsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    opentelemetry: Option<OpentelemetryMetricsConfig>,
}

#[derive(SerdeShape)]
#[serde(deny_unknown_fields)]
struct OpentelemetryMetricsConfig {
    otlp_endpoint: String,
    #[serde(default = "default_metrics_push_interval")]
    push_interval: HumanDuration,
}

#[derive(Clone, Copy, Debug)]
struct ByteSize(u64);

impl SerdeShape for ByteSize {
    fn shape_in(_context: &mut serde_shape::ShapeContext) -> ShapeRef {
        ShapeRef::String
    }
}

#[derive(Clone, Copy, Debug)]
struct HumanDuration(u64);

impl SerdeShape for HumanDuration {
    fn shape_in(_context: &mut serde_shape::ShapeContext) -> ShapeRef {
        ShapeRef::String
    }
}

fn default_dir() -> PathBuf {
    PathBuf::from("/var/lib/percas")
}

fn default_listen_data_addr() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], 7654))
}

fn default_cluster_id() -> String {
    "percas-cluster".to_string()
}

fn default_disk_capacity() -> ByteSize {
    ByteSize(512 * 1024 * 1024)
}

fn default_memory_capacity() -> ByteSize {
    ByteSize(1024 * 1024 * 1024)
}

fn default_metrics_push_interval() -> HumanDuration {
    HumanDuration(30)
}

fn main() {
    for option in env_options::<Config>("PERCAS_CONFIG") {
        let optional = if option.optional {
            "optional"
        } else {
            "required"
        };
        let condition = option.condition.as_deref().unwrap_or("-");
        println!(
            "{:<72} {:<56} {:<18} {:<8} {}",
            option.env_name, option.config_path, option.value_kind, optional, condition
        );
    }
}

fn env_options<T: SerdeShape>(env_prefix: &str) -> Vec<EnvOption> {
    let shape = Shape::for_type::<T>();
    let mut collector = EnvCollector {
        shape: &shape,
        env_prefix,
        options: BTreeMap::new(),
    };
    collector.visit_shape_ref(&shape.root, &mut Vec::new(), false, None);
    collector.options.into_values().collect()
}

struct EnvCollector<'a> {
    shape: &'a Shape,
    env_prefix: &'a str,
    options: BTreeMap<String, EnvOption>,
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
            DefinitionKind::Struct(shape) => {
                self.visit_struct(shape, path, optional, condition);
            }
            DefinitionKind::Enum(shape) => {
                self.visit_enum(shape, path, optional, condition);
            }
            DefinitionKind::Opaque(opaque) => {
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
        shape: &StructShape,
        path: &mut Vec<String>,
        optional: bool,
        condition: Option<String>,
    ) {
        match shape.style {
            FieldsStyle::Struct => {
                for field in &shape.fields {
                    let Some(field_shape) = &field.shape else {
                        continue;
                    };
                    let field_optional = optional || !field.default.is_none();
                    if field.flatten {
                        self.visit_shape_ref(field_shape, path, field_optional, condition.clone());
                    } else {
                        path.push(field.deserialize_name.to_owned());
                        self.visit_shape_ref(field_shape, path, field_optional, condition.clone());
                        path.pop();
                    }
                }
            }
            FieldsStyle::Newtype if shape.fields.len() == 1 => {
                if let Some(field_shape) = &shape.fields[0].shape {
                    self.visit_shape_ref(field_shape, path, optional, condition);
                }
            }
            FieldsStyle::Tuple | FieldsStyle::Newtype | FieldsStyle::Unit => {
                self.push_leaf(path, "object", optional, condition);
            }
        }
    }

    fn visit_enum(
        &mut self,
        shape: &EnumShape,
        path: &mut Vec<String>,
        optional: bool,
        condition: Option<String>,
    ) {
        let variants = shape
            .variants
            .iter()
            .filter(|variant| !variant.skip_deserializing)
            .map(|variant| variant.deserialize_name)
            .collect::<Vec<_>>();

        if shape
            .variants
            .iter()
            .all(|variant| variant.style == FieldsStyle::Unit)
        {
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
                if variant.skip_deserializing {
                    continue;
                }

                let variant_condition =
                    format!("{}={}", tag_path.join("."), variant.deserialize_name);
                let variant_condition = Some(merge_conditions(
                    condition.as_deref(),
                    variant_condition.as_str(),
                ));

                for field in &variant.fields {
                    let Some(field_shape) = &field.shape else {
                        continue;
                    };
                    path.push(field.deserialize_name.to_owned());
                    self.visit_shape_ref(field_shape, path, optional, variant_condition.clone());
                    path.pop();
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

        let config_path = path.join(".");
        self.options
            .entry(config_path.clone())
            .or_insert_with(|| EnvOption {
                env_name: env_name(self.env_prefix, path),
                config_path,
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

fn merge_conditions(existing: Option<&str>, new: &str) -> String {
    existing.map_or_else(|| new.to_owned(), |existing| format!("{existing}; {new}"))
}

fn primitive_kind(shape_ref: &ShapeRef) -> &'static str {
    if shape_ref.is_integer() {
        "integer"
    } else if shape_ref.is_float() {
        "float"
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
