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

#![cfg(feature = "derive")]
#![allow(dead_code)]

use serde_shape::DefaultShape;
use serde_shape::DefinitionKind;
use serde_shape::FieldsStyle;
use serde_shape::IntegerWidth;
use serde_shape::OpaqueReason;
use serde_shape::SerdeShape;
use serde_shape::Shape;
use serde_shape::ShapeRef;
use serde_shape::Tagging;

#[derive(SerdeShape)]
#[serde(
    rename_all = "kebab-case",
    deny_unknown_fields,
    default,
    expecting = "config object"
)]
struct Config {
    http_port: u16,
    #[serde(alias = "endpoint")]
    api_url: Option<String>,
    #[serde(default = "default_retries")]
    retries: u8,
    #[serde(flatten)]
    storage: Storage,
    #[serde(skip)]
    skipped: NotShape,
    #[serde(deserialize_with = "custom_secret")]
    secret: NotShape,
}

#[derive(SerdeShape)]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case"
)]
#[non_exhaustive]
enum Storage {
    #[serde(alias = "s3-compatible")]
    S3 {
        bucket_name: String,
    },
    AzBlob {
        container_name: String,
    },
    #[serde(other)]
    Other,
}

#[derive(SerdeShape)]
#[serde(transparent)]
struct UserId(u64);

#[derive(SerdeShape)]
#[serde(from = "String")]
struct FromString(String);

#[derive(SerdeShape)]
struct SkipsGeneric<T> {
    #[serde(skip)]
    value: T,
}

#[derive(SerdeShape)]
struct Marker<T> {
    marker: std::marker::PhantomData<T>,
}

#[derive(SerdeShape)]
struct Recursive {
    child: Option<Box<Recursive>>,
}

struct NotShape;

fn default_retries() -> u8 {
    3
}

fn root_definition(shape: &Shape) -> &serde_shape::DefinitionShape {
    let ShapeRef::Definition(id) = shape.root else {
        panic!("unexpected root shape: {:?}", shape.root);
    };
    shape.definition(id).unwrap()
}

#[test]
fn derives_struct_shape_from_container_and_field_attrs() {
    let shape = Config::shape();
    let config = root_definition(&shape);
    assert_eq!(config.type_name.serde_name, "Config");

    let DefinitionKind::Struct(config) = &config.kind else {
        panic!("expected struct shape");
    };
    assert_eq!(config.style, FieldsStyle::Struct);
    assert!(config.attributes.deny_unknown_fields);
    assert!(config.attributes.has_flatten);
    assert_eq!(config.attributes.default, DefaultShape::Default);
    assert_eq!(config.attributes.expecting, Some("config object"));
    assert_eq!(config.fields.len(), 6);

    let port = &config.fields[0];
    assert_eq!(port.member, serde_shape::FieldMember::Named("http_port"));
    assert_eq!(port.deserialize_name, "http-port");
    assert_eq!(port.shape, Some(ShapeRef::Unsigned(IntegerWidth::W16)));

    let api_url = &config.fields[1];
    assert_eq!(api_url.deserialize_name, "api-url");
    assert!(api_url.deserialize_aliases.contains(&"endpoint"));
    assert!(matches!(api_url.shape, Some(ShapeRef::Option(_))));

    let retries = &config.fields[2];
    assert_eq!(retries.deserialize_name, "retries");
    assert_eq!(retries.default, DefaultShape::Path("default_retries"));

    let storage = &config.fields[3];
    assert_eq!(storage.deserialize_name, "storage");
    assert!(storage.flatten);
    assert!(matches!(storage.shape, Some(ShapeRef::Definition(_))));

    let skipped = &config.fields[4];
    assert!(skipped.skip_deserializing);
    assert_eq!(skipped.shape, None);

    let secret = &config.fields[5];
    assert!(secret.custom_deserializer);
    assert_eq!(secret.shape, None);
}

#[test]
fn derives_internally_tagged_enum_shape_from_variant_attrs() {
    let shape = Storage::shape();
    let storage = root_definition(&shape);
    let DefinitionKind::Enum(storage) = &storage.kind else {
        panic!("expected enum shape");
    };

    assert_eq!(storage.repr, Tagging::Internal { tag: "type" });
    assert_eq!(
        storage.attributes.tagging,
        Tagging::Internal { tag: "type" }
    );
    assert!(storage.attributes.non_exhaustive);
    assert_eq!(storage.variants.len(), 3);
    assert_eq!(storage.variants[0].rust_name, "S3");
    assert_eq!(storage.variants[0].deserialize_name, "s3");
    assert!(
        storage.variants[0]
            .deserialize_aliases
            .contains(&"s3-compatible")
    );
    assert_eq!(storage.variants[1].deserialize_name, "az-blob");
    assert_eq!(
        storage.variants[0].fields[0].deserialize_name,
        "bucket-name"
    );
    assert!(storage.variants[2].other);
}

#[test]
fn derives_transparent_struct_shape() {
    let shape = UserId::shape();
    let user_id = root_definition(&shape);
    let DefinitionKind::Struct(user_id) = &user_id.kind else {
        panic!("expected struct shape");
    };

    assert_eq!(user_id.style, FieldsStyle::Newtype);
    assert!(user_id.attributes.transparent);
    assert!(user_id.fields[0].transparent);
    assert_eq!(
        user_id.fields[0].shape,
        Some(ShapeRef::Unsigned(IntegerWidth::W64))
    );
}

#[test]
fn marks_conversion_based_shape_as_opaque() {
    let shape = FromString::shape();
    let from_string = root_definition(&shape);
    let DefinitionKind::Opaque(opaque) = &from_string.kind else {
        panic!("expected opaque shape");
    };

    assert_eq!(opaque.reason, OpaqueReason::FromType);
    assert_eq!(opaque.detail, Some("String"));
}

#[test]
fn skipped_generic_field_does_not_require_shape_bound() {
    let shape = SkipsGeneric::<NotShape>::shape();
    let definition = root_definition(&shape);
    let DefinitionKind::Struct(skips) = &definition.kind else {
        panic!("expected struct shape");
    };

    assert_eq!(skips.fields.len(), 1);
    assert!(skips.fields[0].skip_deserializing);
    assert_eq!(skips.fields[0].shape, None);
}

#[test]
fn phantom_data_generic_field_does_not_require_shape_bound() {
    let shape = Marker::<NotShape>::shape();
    let definition = root_definition(&shape);
    let DefinitionKind::Struct(marker) = &definition.kind else {
        panic!("expected struct shape");
    };

    assert_eq!(marker.fields.len(), 1);
    assert_eq!(marker.fields[0].shape, Some(ShapeRef::Unit));
}

#[test]
fn recursive_type_reuses_the_same_definition() {
    let shape = Recursive::shape();
    let recursive = root_definition(&shape);
    let DefinitionKind::Struct(recursive) = &recursive.kind else {
        panic!("expected struct shape");
    };

    assert_eq!(shape.definitions.len(), 1);
    let child = &recursive.fields[0];
    let Some(ShapeRef::Option(child)) = &child.shape else {
        panic!("expected optional child shape");
    };
    let ShapeRef::Definition(id) = child.as_ref() else {
        panic!("expected recursive definition reference");
    };
    assert_eq!(id.0, 0);
}
