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

use serde_shape::SerdeShape;
use serde_shape_derive::SerdeShape as DeriveSerdeShape;

#[derive(DeriveSerdeShape)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct Config {
    http_port: u16,
    #[serde(alias = "endpoint")]
    api_url: Option<String>,
    #[serde(flatten)]
    storage: Storage,
}

#[derive(DeriveSerdeShape)]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case"
)]
enum Storage {
    S3 { bucket_name: String },
    AzBlob { container_name: String },
}

#[derive(DeriveSerdeShape)]
struct SkipsGeneric<T> {
    #[serde(skip)]
    value: T,
}

struct NotShape;

#[test]
fn derives_struct_shape_from_serde_attrs() {
    let shape = Config::shape();
    let root_id = match shape.root {
        serde_shape::ShapeRef::Definition(id) => id,
        ref shape => panic!("unexpected root shape: {shape:?}"),
    };

    let config = shape.definition(root_id).unwrap();
    assert_eq!(config.type_name.serde_name, "Config");

    let serde_shape::DefinitionKind::Struct(config) = &config.kind else {
        panic!("expected struct shape");
    };
    assert_eq!(config.style, serde_shape::FieldsStyle::Struct);
    assert!(config.attributes.deny_unknown_fields);
    assert!(config.attributes.has_flatten);
    assert_eq!(config.fields.len(), 3);

    let port = &config.fields[0];
    assert_eq!(port.deserialize_name, "http-port");
    assert_eq!(
        port.shape,
        Some(serde_shape::ShapeRef::Unsigned(
            serde_shape::IntegerWidth::W16
        ))
    );

    let api_url = &config.fields[1];
    assert_eq!(api_url.deserialize_name, "api-url");
    assert!(api_url.deserialize_aliases.contains(&"endpoint"));

    let storage = &config.fields[2];
    assert_eq!(storage.deserialize_name, "storage");
    assert!(storage.flatten);
    assert!(matches!(
        storage.shape,
        Some(serde_shape::ShapeRef::Definition(_))
    ));
}

#[test]
fn derives_internally_tagged_enum_shape() {
    let shape = Storage::shape();
    let root_id = match shape.root {
        serde_shape::ShapeRef::Definition(id) => id,
        ref shape => panic!("unexpected root shape: {shape:?}"),
    };

    let storage = shape.definition(root_id).unwrap();
    let serde_shape::DefinitionKind::Enum(storage) = &storage.kind else {
        panic!("expected enum shape");
    };

    assert_eq!(storage.repr, serde_shape::Tagging::Internal { tag: "type" });
    assert_eq!(storage.variants.len(), 2);
    assert_eq!(storage.variants[0].deserialize_name, "s3");
    assert_eq!(storage.variants[1].deserialize_name, "az-blob");
    assert_eq!(
        storage.variants[0].fields[0].deserialize_name,
        "bucket-name"
    );
}

#[test]
fn skipped_generic_field_does_not_require_shape_bound() {
    let shape = SkipsGeneric::<NotShape>::shape();
    let root_id = match shape.root {
        serde_shape::ShapeRef::Definition(id) => id,
        ref shape => panic!("unexpected root shape: {shape:?}"),
    };

    let definition = shape.definition(root_id).unwrap();
    let serde_shape::DefinitionKind::Struct(skips) = &definition.kind else {
        panic!("expected struct shape");
    };

    assert_eq!(skips.fields.len(), 1);
    assert!(skips.fields[0].skip_deserializing);
    assert_eq!(skips.fields[0].shape, None);
}
