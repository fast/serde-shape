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

use serde_shape::DeserializeDefinitionKind;
use serde_shape::DeserializeShape;
use serde_shape::FieldMember;
use serde_shape::SerializeDefinitionKind;
use serde_shape::SerializeShape;

#[derive(DeserializeShape)]
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

#[derive(DeserializeShape)]
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

#[derive(DeserializeShape)]
#[serde(transparent)]
struct UserId(u64);

#[derive(DeserializeShape)]
#[serde(from = "String")]
struct FromString(String);

#[derive(DeserializeShape)]
struct SkipsGeneric<T> {
    #[serde(skip)]
    value: T,
}

#[derive(DeserializeShape)]
struct Marker<T> {
    marker: core::marker::PhantomData<T>,
}

#[derive(DeserializeShape)]
struct Recursive {
    child: Option<Box<Recursive>>,
}

#[derive(SerializeShape, DeserializeShape)]
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

#[derive(SerializeShape, DeserializeShape)]
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

#[derive(SerializeShape)]
struct SerializeOnly<T> {
    #[serde(skip_serializing)]
    skipped: T,
    visible: u8,
}

#[derive(DeserializeShape)]
struct DeserializeOnly<T> {
    #[serde(skip_deserializing)]
    skipped: T,
    visible: u8,
}

struct NotShape;

fn default_retries() -> u8 {
    3
}

#[test]
fn snapshots_struct_shape_from_container_and_field_attrs() {
    insta::assert_debug_snapshot!(Config::deserialize_shape());
}

#[test]
fn snapshots_internally_tagged_enum_shape_from_variant_attrs() {
    insta::assert_debug_snapshot!(Storage::deserialize_shape());
}

#[test]
fn snapshots_transparent_struct_shape() {
    insta::assert_debug_snapshot!(UserId::deserialize_shape());
}

#[test]
fn snapshots_conversion_based_opaque_shape() {
    insta::assert_debug_snapshot!(FromString::deserialize_shape());
}

#[test]
fn snapshots_skipped_generic_field_without_shape_bound() {
    insta::assert_debug_snapshot!(SkipsGeneric::<NotShape>::deserialize_shape());
}

#[test]
fn snapshots_phantom_data_generic_field_without_shape_bound() {
    insta::assert_debug_snapshot!(Marker::<NotShape>::deserialize_shape());
}

#[test]
fn snapshots_recursive_type_reusing_the_same_definition() {
    insta::assert_debug_snapshot!(Recursive::deserialize_shape());
}

#[test]
fn exposes_deserialize_field_metadata() {
    let shape = SplitIo::deserialize_shape();
    let serde_shape::ShapeRef::Definition(id) = shape.root else {
        panic!("root shape should be a definition");
    };
    let definition = shape.definition(id).expect("definition exists");
    let DeserializeDefinitionKind::Struct(struct_shape) = &definition.kind else {
        panic!("definition should be a struct");
    };

    assert_eq!(definition.type_name.name, "wire-input");

    let [id, maybe, secret, output_only] = struct_shape.fields.as_slice() else {
        panic!("struct should expose all fields");
    };

    assert_eq!(id.member, FieldMember::Named("id"));
    assert_eq!(id.name, "in-id");
    assert_eq!(id.aliases, vec!["in-id", "legacy-id"]);
    assert!(!id.skip);
    assert!(!id.custom_deserializer);

    assert_eq!(maybe.name, "maybe");
    assert!(!maybe.skip);

    assert_eq!(secret.name, "secret-in");
    assert!(!secret.skip);

    assert_eq!(output_only.name, "only-in");
    assert!(output_only.skip);
    assert!(!output_only.custom_deserializer);
    assert_eq!(output_only.value_shape, None);
}

#[test]
fn exposes_serialize_field_metadata() {
    let shape = SplitIo::serialize_shape();
    let serde_shape::ShapeRef::Definition(id) = shape.root else {
        panic!("root shape should be a definition");
    };
    let definition = shape.definition(id).expect("definition exists");
    let SerializeDefinitionKind::Struct(struct_shape) = &definition.kind else {
        panic!("definition should be a struct");
    };

    assert_eq!(definition.type_name.name, "wire-output");

    let [id, maybe, secret, output_only] = struct_shape.fields.as_slice() else {
        panic!("struct should expose all fields");
    };

    assert_eq!(id.member, FieldMember::Named("id"));
    assert_eq!(id.name, "out-id");
    assert!(!id.skip);
    assert_eq!(id.skip_if, None);
    assert!(!id.custom_serializer);

    assert_eq!(maybe.name, "maybe");
    assert_eq!(maybe.skip_if, Some("is_missing"));

    assert_eq!(secret.name, "secret");
    assert!(secret.skip);
    assert_eq!(secret.value_shape, None);

    assert_eq!(output_only.name, "only-out");
    assert!(!output_only.skip);
    assert!(output_only.custom_serializer);
    assert_eq!(output_only.value_shape, None);
}

#[test]
fn exposes_deserialize_variant_metadata() {
    let shape = SplitEnum::deserialize_shape();
    let serde_shape::ShapeRef::Definition(id) = shape.root else {
        panic!("root shape should be a definition");
    };
    let definition = shape.definition(id).expect("definition exists");
    let DeserializeDefinitionKind::Enum(enum_shape) = &definition.kind else {
        panic!("definition should be an enum");
    };

    let [struct_variant, renamed, input_only, output_only] = enum_shape.variants.as_slice() else {
        panic!("enum should expose all variants");
    };

    assert_eq!(struct_variant.rust_name, "StructVariant");
    assert_eq!(struct_variant.name, "struct-variant");
    assert!(!struct_variant.skip);

    let [field] = struct_variant.fields.as_slice() else {
        panic!("struct variant should expose its field");
    };
    assert_eq!(field.name, "field_name");

    assert_eq!(renamed.name, "deserialized");
    assert_eq!(renamed.aliases, vec!["deserialized", "legacy"]);

    assert_eq!(input_only.name, "input-only");
    assert!(!input_only.skip);

    assert_eq!(output_only.name, "output-only");
    assert!(output_only.skip);
    assert!(!output_only.custom_deserializer);
    assert!(output_only.fields.is_empty());
}

#[test]
fn exposes_serialize_variant_metadata() {
    let shape = SplitEnum::serialize_shape();
    let serde_shape::ShapeRef::Definition(id) = shape.root else {
        panic!("root shape should be a definition");
    };
    let definition = shape.definition(id).expect("definition exists");
    let SerializeDefinitionKind::Enum(enum_shape) = &definition.kind else {
        panic!("definition should be an enum");
    };

    let [struct_variant, renamed, input_only, output_only] = enum_shape.variants.as_slice() else {
        panic!("enum should expose all variants");
    };

    assert_eq!(struct_variant.rust_name, "StructVariant");
    assert_eq!(struct_variant.name, "STRUCT_VARIANT");
    assert!(!struct_variant.skip);

    let [field] = struct_variant.fields.as_slice() else {
        panic!("struct variant should expose its field");
    };
    assert_eq!(field.name, "fieldName");

    assert_eq!(renamed.name, "SERIALIZED");

    assert_eq!(input_only.name, "INPUT_ONLY");
    assert!(input_only.skip);

    assert_eq!(output_only.name, "OUTPUT_ONLY");
    assert!(!output_only.skip);
    assert!(output_only.custom_serializer);
    assert!(output_only.fields.is_empty());
}

#[test]
fn derives_one_direction_without_requiring_the_other_direction() {
    let serialize_shape = SerializeOnly::<NotShape>::serialize_shape();
    let deserialize_shape = DeserializeOnly::<NotShape>::deserialize_shape();

    assert!(matches!(
        serialize_shape.definition(match serialize_shape.root {
            serde_shape::ShapeRef::Definition(id) => id,
            _ => panic!("serialize root shape should be a definition"),
        }),
        Some(serde_shape::SerializeDefinitionShape { .. })
    ));
    assert!(matches!(
        deserialize_shape.definition(match deserialize_shape.root {
            serde_shape::ShapeRef::Definition(id) => id,
            _ => panic!("deserialize root shape should be a definition"),
        }),
        Some(serde_shape::DeserializeDefinitionShape { .. })
    ));
}
