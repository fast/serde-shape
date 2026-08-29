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

mod common;

use common::deserialize_root_definition;
use common::serialize_root_definition;
use renamed_shape::DefaultShape;
use renamed_shape::DeserializeDefinitionKind;
use renamed_shape::DeserializeShape;
use renamed_shape::DeserializeShapeContext;
use renamed_shape::DeserializeVariantContent;
use renamed_shape::FieldMember;
use renamed_shape::FieldWireShape;
use renamed_shape::OpaqueReason;
use renamed_shape::SerializeDefinitionKind;
use renamed_shape::SerializeShape;
use renamed_shape::SerializeShapeContext;
use renamed_shape::SerializeVariantContent;
use renamed_shape::ShapeRef;
use renamed_shape::Tagging;

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
#[serde(try_from = "u16")]
struct TryFromU16(u16);

#[derive(SerializeShape)]
#[serde(into = "String")]
struct IntoString(String);

#[derive(DeserializeShape)]
#[serde(from = "T")]
struct FromGeneric<T>(T);

#[derive(SerializeShape, DeserializeShape)]
#[serde_shape(
    serialize_with = "serialize_number_or_string_shape",
    deserialize_with = "deserialize_string_shape"
)]
struct ContainerShapeOverride(NotShape);

#[derive(SerializeShape, DeserializeShape)]
struct FieldShapeOverrides {
    #[serde(with = "custom_representation")]
    #[serde_shape(
        serialize_with = "serialize_string_shape",
        deserialize_with = "deserialize_string_shape"
    )]
    custom: NotShape,
    #[serde_shape(
        serialize_with = "serialize_u8_shape",
        deserialize_with = "deserialize_bool_shape"
    )]
    directional: NotShape,
}

#[derive(SerializeShape, DeserializeShape)]
#[serde_shape(bound(serialize = "T: SerializeShape", deserialize = "T: DeserializeShape"))]
struct GenericFieldShape<T> {
    #[serde_shape(
        serialize_with = "serialize_type_shape::<T>",
        deserialize_with = "deserialize_type_shape::<T>"
    )]
    custom: NotShape,
    #[serde(skip)]
    marker: core::marker::PhantomData<T>,
}

/// Selects the retry policy.
///
/// This text is available to configuration tooling.
#[derive(SerializeShape, DeserializeShape)]
enum DocumentedSetting {
    /// Uses the built-in retry policy.
    Default,
    /// Uses a fixed retry limit.
    Fixed {
        /// Maximum number of retry attempts.
        retries: u8,
    },
}

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
struct RecursiveGeneric<T> {
    value: T,
    child: Option<Box<RecursiveGeneric<T>>>,
}

trait HasValue {
    type Value;
}

struct ValueProvider;

impl HasValue for ValueProvider {
    type Value = u16;
}

#[derive(SerializeShape, DeserializeShape)]
struct AssociatedValue<T: HasValue> {
    value: T::Value,
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

fn serialize_number_or_string_shape(_context: &mut SerializeShapeContext) -> ShapeRef {
    ShapeRef::union([ShapeRef::U16, ShapeRef::String])
}

fn serialize_string_shape(context: &mut SerializeShapeContext) -> ShapeRef {
    String::serialize_shape_in(context)
}

fn deserialize_string_shape(context: &mut DeserializeShapeContext) -> ShapeRef {
    String::deserialize_shape_in(context)
}

fn serialize_type_shape<T: SerializeShape>(context: &mut SerializeShapeContext) -> ShapeRef {
    T::serialize_shape_in(context)
}

fn deserialize_type_shape<T: DeserializeShape>(context: &mut DeserializeShapeContext) -> ShapeRef {
    T::deserialize_shape_in(context)
}

fn serialize_u8_shape(_context: &mut SerializeShapeContext) -> ShapeRef {
    ShapeRef::U8
}

fn deserialize_bool_shape(_context: &mut DeserializeShapeContext) -> ShapeRef {
    ShapeRef::Bool
}

fn default_retries() -> u8 {
    3
}

#[test]
fn exposes_deserialize_container_attributes() {
    let definition = deserialize_root_definition::<Config>();
    let DeserializeDefinitionKind::Struct(shape) = &definition.kind else {
        panic!("definition should be a struct");
    };

    assert_eq!(shape.attributes.default, DefaultShape::Default);
    assert!(shape.attributes.deny_unknown_fields);
    assert_eq!(shape.attributes.expecting, Some("config object"));

    let [http_port, api_url, retries, storage, skipped, secret] = shape.fields.as_slice() else {
        panic!("config should expose all fields");
    };
    assert_eq!(http_port.name, "http-port");
    assert_eq!(api_url.aliases, ["api-url", "endpoint"]);
    assert_eq!(retries.default, DefaultShape::Path("default_retries"));
    assert!(matches!(storage.wire_shape, FieldWireShape::Flatten(_)));
    assert_eq!(skipped.wire_shape, FieldWireShape::Omitted);
    let FieldWireShape::Value(ShapeRef::Opaque(opaque)) = &secret.wire_shape else {
        panic!("custom deserializer should be opaque");
    };
    assert_eq!(opaque.reason, OpaqueReason::CustomDeserializer);
}

#[test]
fn exposes_deserialize_enum_attributes() {
    let definition = deserialize_root_definition::<Storage>();
    let DeserializeDefinitionKind::Enum(shape) = &definition.kind else {
        panic!("definition should be an enum");
    };

    assert_eq!(shape.repr, Tagging::Internal { tag: "type" });
    assert!(shape.attributes.non_exhaustive);
    assert_eq!(shape.variants[0].aliases, ["s3", "s3-compatible"]);
    assert!(shape.variants[2].other);
}

#[test]
fn exposes_transparent_shape() {
    let definition = deserialize_root_definition::<UserId>();
    let DeserializeDefinitionKind::Struct(shape) = &definition.kind else {
        panic!("transparent definition should be a struct");
    };
    assert!(shape.attributes.transparent);
    assert_eq!(
        shape.fields[0].wire_shape,
        FieldWireShape::Inline(ShapeRef::U64)
    );
}

#[test]
fn follows_serde_conversion_shapes() {
    assert_eq!(FromString::deserialize_shape().root(), &ShapeRef::String);
    assert_eq!(TryFromU16::deserialize_shape().root(), &ShapeRef::U16);
    assert_eq!(IntoString::serialize_shape().root(), &ShapeRef::String);
    assert_eq!(FromGeneric::<u8>::deserialize_shape().root(), &ShapeRef::U8);
}

#[test]
fn applies_container_and_field_custom_shape_functions() {
    assert_eq!(
        ContainerShapeOverride::serialize_shape().root(),
        &ShapeRef::union([ShapeRef::U16, ShapeRef::String])
    );
    assert_eq!(
        ContainerShapeOverride::deserialize_shape().root(),
        &ShapeRef::String
    );

    let definition = serialize_root_definition::<FieldShapeOverrides>();
    let SerializeDefinitionKind::Struct(shape) = &definition.kind else {
        panic!("serialize definition should be a struct");
    };
    assert_eq!(
        shape.fields[0].wire_shape,
        FieldWireShape::Value(ShapeRef::String)
    );
    assert_eq!(
        shape.fields[1].wire_shape,
        FieldWireShape::Value(ShapeRef::U8)
    );

    let definition = deserialize_root_definition::<FieldShapeOverrides>();
    let DeserializeDefinitionKind::Struct(shape) = &definition.kind else {
        panic!("deserialize definition should be a struct");
    };
    assert_eq!(
        shape.fields[0].wire_shape,
        FieldWireShape::Value(ShapeRef::String)
    );
    assert_eq!(
        shape.fields[1].wire_shape,
        FieldWireShape::Value(ShapeRef::Bool)
    );
}

#[test]
fn applies_explicit_bounds_to_generic_shape_hooks() {
    let definition = serialize_root_definition::<GenericFieldShape<u32>>();
    let SerializeDefinitionKind::Struct(shape) = &definition.kind else {
        panic!("serialize definition should be a struct");
    };
    assert_eq!(
        shape.fields[0].wire_shape,
        FieldWireShape::Value(ShapeRef::U32)
    );

    let definition = deserialize_root_definition::<GenericFieldShape<u32>>();
    let DeserializeDefinitionKind::Struct(shape) = &definition.kind else {
        panic!("deserialize definition should be a struct");
    };
    assert_eq!(
        shape.fields[0].wire_shape,
        FieldWireShape::Value(ShapeRef::U32)
    );
}

#[test]
fn preserves_rust_documentation() {
    let definition = serialize_root_definition::<DocumentedSetting>();
    assert_eq!(
        definition.description,
        Some("Selects the retry policy.\n\nThis text is available to configuration tooling.")
    );
    let SerializeDefinitionKind::Enum(shape) = &definition.kind else {
        panic!("serialize definition should be an enum");
    };
    assert_eq!(
        shape.variants[1].description,
        Some("Uses a fixed retry limit.")
    );
    let SerializeVariantContent::Fields(fields) = &shape.variants[1].content else {
        panic!("fixed variant should expose fields");
    };
    assert_eq!(
        fields[0].description,
        Some("Maximum number of retry attempts.")
    );

    let definition = deserialize_root_definition::<DocumentedSetting>();
    assert_eq!(
        definition.description,
        Some("Selects the retry policy.\n\nThis text is available to configuration tooling.")
    );
    let DeserializeDefinitionKind::Enum(shape) = &definition.kind else {
        panic!("deserialize definition should be an enum");
    };
    assert_eq!(
        shape.variants[1].description,
        Some("Uses a fixed retry limit.")
    );
    let DeserializeVariantContent::Fields(fields) = &shape.variants[1].content else {
        panic!("fixed variant should expose fields");
    };
    assert_eq!(
        fields[0].description,
        Some("Maximum number of retry attempts.")
    );
}

#[test]
fn omits_shape_bounds_for_skipped_and_marker_fields() {
    assert_eq!(
        SkipsGeneric::<NotShape>::deserialize_shape()
            .definitions()
            .len(),
        1
    );
    assert_eq!(
        Marker::<NotShape>::deserialize_shape().definitions().len(),
        1
    );
}

#[test]
fn reuses_recursive_definition() {
    let graph = Recursive::deserialize_shape();
    let ShapeRef::Definition(id) = graph.root() else {
        panic!("recursive root should be a definition");
    };
    let id = *id;
    let DeserializeDefinitionKind::Struct(shape) = &graph.definition(id).unwrap().kind else {
        panic!("recursive definition should be a struct");
    };
    assert_eq!(graph.definitions().len(), 1);
    assert_eq!(
        shape.fields[0].wire_shape,
        FieldWireShape::Value(ShapeRef::Option(Box::new(ShapeRef::Definition(id))))
    );
}

#[test]
fn derives_recursive_generic_shapes_without_cyclic_bounds() {
    let serialize = RecursiveGeneric::<u8>::serialize_shape();
    let deserialize = RecursiveGeneric::<u8>::deserialize_shape();

    assert_eq!(serialize.definitions().len(), 1);
    assert_eq!(deserialize.definitions().len(), 1);
}

#[test]
fn derives_shape_bounds_for_associated_values() {
    let serialize = AssociatedValue::<ValueProvider>::serialize_shape();
    let deserialize = AssociatedValue::<ValueProvider>::deserialize_shape();

    assert_eq!(serialize.definitions().len(), 1);
    assert_eq!(deserialize.definitions().len(), 1);
}

#[test]
fn exposes_deserialize_field_metadata() {
    let definition = deserialize_root_definition::<SplitIo>();
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
    assert!(matches!(&id.wire_shape, FieldWireShape::Value(_)));

    assert_eq!(maybe.name, "maybe");
    assert!(matches!(&maybe.wire_shape, FieldWireShape::Value(_)));

    assert_eq!(secret.name, "secret-in");
    assert!(matches!(&secret.wire_shape, FieldWireShape::Value(_)));

    assert_eq!(output_only.name, "only-in");
    assert_eq!(output_only.wire_shape, FieldWireShape::Omitted);
}

#[test]
fn exposes_serialize_field_metadata() {
    let definition = serialize_root_definition::<SplitIo>();
    let SerializeDefinitionKind::Struct(struct_shape) = &definition.kind else {
        panic!("definition should be a struct");
    };

    assert_eq!(definition.type_name.name, "wire-output");

    let [id, maybe, secret, output_only] = struct_shape.fields.as_slice() else {
        panic!("struct should expose all fields");
    };

    assert_eq!(id.member, FieldMember::Named("id"));
    assert_eq!(id.name, "out-id");
    assert_eq!(id.skip_if, None);
    assert!(matches!(&id.wire_shape, FieldWireShape::Value(_)));

    assert_eq!(maybe.name, "maybe");
    assert_eq!(maybe.skip_if, Some("is_missing"));

    assert_eq!(secret.name, "secret");
    assert_eq!(secret.wire_shape, FieldWireShape::Omitted);

    assert_eq!(output_only.name, "only-out");
    let FieldWireShape::Value(ShapeRef::Opaque(opaque)) = &output_only.wire_shape else {
        panic!("custom serialized field should expose an opaque wire shape");
    };
    assert_eq!(opaque.reason, OpaqueReason::CustomSerializer);
    assert_eq!(opaque.detail, Some("serialize_not_shape"));
}

#[test]
fn exposes_deserialize_variant_metadata() {
    let definition = deserialize_root_definition::<SplitEnum>();
    let DeserializeDefinitionKind::Enum(enum_shape) = &definition.kind else {
        panic!("definition should be an enum");
    };

    let [struct_variant, renamed, input_only, output_only] = enum_shape.variants.as_slice() else {
        panic!("enum should expose all variants");
    };

    assert_eq!(struct_variant.rust_name, "StructVariant");
    assert_eq!(struct_variant.name, "struct-variant");

    let DeserializeVariantContent::Fields(fields) = &struct_variant.content else {
        panic!("struct variant should expose derived fields");
    };
    let [field] = fields.as_slice() else {
        panic!("struct variant should expose its field");
    };
    assert_eq!(field.name, "field_name");

    assert_eq!(renamed.name, "deserialized");
    assert_eq!(renamed.aliases, vec!["deserialized", "legacy"]);

    assert_eq!(input_only.name, "input-only");
    assert!(matches!(
        &input_only.content,
        DeserializeVariantContent::Fields(_)
    ));

    assert_eq!(output_only.name, "output-only");
    assert_eq!(output_only.content, DeserializeVariantContent::Omitted);
}

#[test]
fn exposes_serialize_variant_metadata() {
    let definition = serialize_root_definition::<SplitEnum>();
    let SerializeDefinitionKind::Enum(enum_shape) = &definition.kind else {
        panic!("definition should be an enum");
    };

    let [struct_variant, renamed, input_only, output_only] = enum_shape.variants.as_slice() else {
        panic!("enum should expose all variants");
    };

    assert_eq!(struct_variant.rust_name, "StructVariant");
    assert_eq!(struct_variant.name, "STRUCT_VARIANT");

    let SerializeVariantContent::Fields(fields) = &struct_variant.content else {
        panic!("struct variant should expose derived fields");
    };
    let [field] = fields.as_slice() else {
        panic!("struct variant should expose its field");
    };
    assert_eq!(field.name, "fieldName");

    assert_eq!(renamed.name, "SERIALIZED");

    assert_eq!(input_only.name, "INPUT_ONLY");
    assert_eq!(input_only.content, SerializeVariantContent::Omitted);

    assert_eq!(output_only.name, "OUTPUT_ONLY");
    let SerializeVariantContent::Custom(opaque) = &output_only.content else {
        panic!("custom serialized variant should expose opaque content");
    };
    assert_eq!(opaque.reason, OpaqueReason::CustomSerializer);
    assert_eq!(opaque.detail, Some("serialize_variant"));
}

#[test]
fn derives_one_direction_without_requiring_the_other_direction() {
    assert!(matches!(
        serialize_root_definition::<SerializeOnly<NotShape>>().kind,
        SerializeDefinitionKind::Struct(_)
    ));
    assert!(matches!(
        deserialize_root_definition::<DeserializeOnly<NotShape>>().kind,
        DeserializeDefinitionKind::Struct(_)
    ));
}
