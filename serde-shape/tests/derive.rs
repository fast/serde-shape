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

use serde_shape::DefinitionKind;
use serde_shape::FieldMember;
use serde_shape::SerdeShape;

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

#[derive(SerdeShape)]
#[serde(rename(serialize = "wire-output", deserialize = "wire-input"))]
struct SplitIo {
    #[serde(
        rename(serialize = "out-id", deserialize = "in-id"),
        alias = "legacy-id"
    )]
    id: u64,
    #[serde(skip_serializing_if = "is_missing")]
    maybe: Option<String>,
    #[serde(skip_serializing, rename(deserialize = "secret-in"))]
    secret: String,
    #[serde(
        skip_deserializing,
        rename(serialize = "only-out", deserialize = "only-in"),
        serialize_with = "serialize_not_shape"
    )]
    output_only: NotShape,
}

#[derive(SerdeShape)]
#[serde(
    rename_all(serialize = "SCREAMING_SNAKE_CASE", deserialize = "kebab-case"),
    rename_all_fields(serialize = "camelCase", deserialize = "snake_case")
)]
enum SplitEnum {
    StructVariant {
        field_name: String,
    },
    #[serde(
        rename(serialize = "SERIALIZED", deserialize = "deserialized"),
        alias = "legacy"
    )]
    Renamed,
    #[serde(skip_serializing)]
    InputOnly,
    #[serde(skip_deserializing, serialize_with = "serialize_variant")]
    OutputOnly(NotShape),
}

struct NotShape;

fn default_retries() -> u8 {
    3
}

#[test]
fn snapshots_struct_shape_from_container_and_field_attrs() {
    insta::assert_debug_snapshot!(Config::shape());
}

#[test]
fn snapshots_internally_tagged_enum_shape_from_variant_attrs() {
    insta::assert_debug_snapshot!(Storage::shape());
}

#[test]
fn snapshots_transparent_struct_shape() {
    insta::assert_debug_snapshot!(UserId::shape());
}

#[test]
fn snapshots_conversion_based_opaque_shape() {
    insta::assert_debug_snapshot!(FromString::shape());
}

#[test]
fn snapshots_skipped_generic_field_without_shape_bound() {
    insta::assert_debug_snapshot!(SkipsGeneric::<NotShape>::shape());
}

#[test]
fn snapshots_phantom_data_generic_field_without_shape_bound() {
    insta::assert_debug_snapshot!(Marker::<NotShape>::shape());
}

#[test]
fn snapshots_recursive_type_reusing_the_same_definition() {
    insta::assert_debug_snapshot!(Recursive::shape());
}

#[test]
fn exposes_serialize_side_field_metadata() {
    let shape = SplitIo::shape();
    let serde_shape::ShapeRef::Definition(id) = shape.root else {
        panic!("root shape should be a definition");
    };
    let definition = shape.definition(id).expect("definition exists");
    let DefinitionKind::Struct(struct_shape) = &definition.kind else {
        panic!("definition should be a struct");
    };

    assert_eq!(definition.type_name.serde_name, "wire-input");
    assert_eq!(definition.type_name.serialize_name, "wire-output");

    let [id, maybe, secret, output_only] = struct_shape.fields.as_slice() else {
        panic!("struct should expose all fields");
    };

    assert_eq!(id.member, FieldMember::Named("id"));
    assert_eq!(id.serialize_name, "out-id");
    assert_eq!(id.deserialize_name, "in-id");
    assert_eq!(id.deserialize_aliases, vec!["in-id", "legacy-id"]);
    assert!(!id.skip_serializing);
    assert_eq!(id.skip_serializing_if, None);
    assert!(!id.custom_serializer);

    assert_eq!(maybe.serialize_name, "maybe");
    assert_eq!(maybe.deserialize_name, "maybe");
    assert_eq!(maybe.skip_serializing_if, Some("is_missing"));

    assert_eq!(secret.serialize_name, "secret");
    assert_eq!(secret.deserialize_name, "secret-in");
    assert!(secret.skip_serializing);
    assert!(!secret.skip_deserializing);

    assert_eq!(output_only.serialize_name, "only-out");
    assert_eq!(output_only.deserialize_name, "only-in");
    assert!(output_only.skip_deserializing);
    assert!(output_only.custom_serializer);
    assert!(!output_only.custom_deserializer);
    assert_eq!(output_only.shape, None);
}

#[test]
fn exposes_serialize_side_variant_metadata() {
    let shape = SplitEnum::shape();
    let serde_shape::ShapeRef::Definition(id) = shape.root else {
        panic!("root shape should be a definition");
    };
    let definition = shape.definition(id).expect("definition exists");
    let DefinitionKind::Enum(enum_shape) = &definition.kind else {
        panic!("definition should be an enum");
    };

    let [struct_variant, renamed, input_only, output_only] = enum_shape.variants.as_slice() else {
        panic!("enum should expose all variants");
    };

    assert_eq!(struct_variant.rust_name, "StructVariant");
    assert_eq!(struct_variant.serialize_name, "STRUCT_VARIANT");
    assert_eq!(struct_variant.deserialize_name, "struct-variant");
    assert!(!struct_variant.skip_serializing);

    let [field] = struct_variant.fields.as_slice() else {
        panic!("struct variant should expose its field");
    };
    assert_eq!(field.serialize_name, "fieldName");
    assert_eq!(field.deserialize_name, "field_name");

    assert_eq!(renamed.serialize_name, "SERIALIZED");
    assert_eq!(renamed.deserialize_name, "deserialized");
    assert_eq!(renamed.deserialize_aliases, vec!["deserialized", "legacy"]);

    assert_eq!(input_only.serialize_name, "INPUT_ONLY");
    assert_eq!(input_only.deserialize_name, "input-only");
    assert!(input_only.skip_serializing);
    assert!(!input_only.skip_deserializing);

    assert_eq!(output_only.serialize_name, "OUTPUT_ONLY");
    assert_eq!(output_only.deserialize_name, "output-only");
    assert!(!output_only.skip_serializing);
    assert!(output_only.skip_deserializing);
    assert!(output_only.custom_serializer);
    assert!(!output_only.custom_deserializer);
    assert!(output_only.fields.is_empty());
}
