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

mod common;

use common::deserialize_root_definition;
use common::serialize_root_definition;
use renamed_shape::DeserializeDefinitionKind;
use renamed_shape::DeserializeFieldShape;
use renamed_shape::DeserializeShape;
use renamed_shape::DeserializeVariantContent;
use renamed_shape::DeserializeVariantShape;
use renamed_shape::FieldWireShape;
use renamed_shape::OpaqueReason;
use renamed_shape::SerializeDefinitionKind;
use renamed_shape::SerializeFieldShape;
use renamed_shape::SerializeShape;
use renamed_shape::SerializeVariantContent;
use renamed_shape::SerializeVariantShape;
use renamed_shape::ShapeRef;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, PartialEq)]
struct FlatValue(u64);

#[derive(Debug, PartialEq, Serialize, Deserialize, SerializeShape, DeserializeShape)]
struct FlattenedCustom {
    #[serde(flatten, with = "flat_value")]
    value: FlatValue,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, SerializeShape, DeserializeShape)]
#[serde(transparent)]
struct TransparentValue {
    value: u64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, SerializeShape, DeserializeShape)]
#[serde(transparent)]
struct TransparentCustom {
    #[serde(with = "stringified")]
    value: u64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, SerializeShape, DeserializeShape)]
enum CustomVariant {
    #[serde(with = "stringified")]
    Value(u64),
}

#[derive(Debug, PartialEq, Serialize, Deserialize, SerializeShape, DeserializeShape)]
enum DeclaredCustomVariant {
    #[serde(with = "flat_value")]
    #[serde_shape(
        serialize_with = "serialize_flat_value_shape",
        deserialize_with = "deserialize_flat_value_shape"
    )]
    Value(FlatValue),
}

fn serialize_flat_value_shape(_context: &mut renamed_shape::SerializeShapeContext) -> ShapeRef {
    flat_value_shape()
}

fn deserialize_flat_value_shape(_context: &mut renamed_shape::DeserializeShapeContext) -> ShapeRef {
    flat_value_shape()
}

fn flat_value_shape() -> ShapeRef {
    ShapeRef::Map {
        key: Box::new(ShapeRef::String),
        value: Box::new(ShapeRef::U64),
    }
}

fn assert_path(actual: Option<&str>, expected_segments: &[&str]) {
    let path = syn::parse_str::<syn::ExprPath>(actual.expect("path metadata should be present"))
        .expect("metadata path should remain valid Rust");
    assert_eq!(
        path.path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>(),
        expected_segments
    );
}

#[test]
fn composes_flatten_with_custom_field_boundaries() {
    let value = FlattenedCustom {
        value: FlatValue(7),
    };
    let json = serde_json::to_value(&value).expect("value should serialize");
    assert_eq!(json, serde_json::json!({ "custom": 7 }));
    assert_eq!(
        serde_json::from_value::<FlattenedCustom>(json).expect("value should deserialize"),
        value
    );

    let serialize_field = first_serialize_field::<FlattenedCustom>();
    let FieldWireShape::Flatten(ShapeRef::Opaque(opaque)) = serialize_field.wire_shape else {
        panic!("custom serialized flatten field should be flattened and opaque");
    };
    assert_eq!(opaque.reason, OpaqueReason::CustomSerializer);
    assert_path(opaque.detail, &["flat_value", "serialize"]);

    let deserialize_field = first_deserialize_field::<FlattenedCustom>();
    let FieldWireShape::Flatten(ShapeRef::Opaque(opaque)) = deserialize_field.wire_shape else {
        panic!("custom deserialized flatten field should be flattened and opaque");
    };
    assert_eq!(opaque.reason, OpaqueReason::CustomDeserializer);
    assert_path(opaque.detail, &["flat_value", "deserialize"]);
}

#[test]
fn marks_named_transparent_fields_as_inline() {
    let value = TransparentValue { value: 11 };
    let json = serde_json::to_value(&value).expect("value should serialize");
    assert_eq!(json, serde_json::json!(11));
    assert_eq!(
        serde_json::from_value::<TransparentValue>(json).expect("value should deserialize"),
        value
    );

    assert_eq!(
        first_serialize_field::<TransparentValue>().wire_shape,
        FieldWireShape::Inline(ShapeRef::U64)
    );
    assert_eq!(
        first_deserialize_field::<TransparentValue>().wire_shape,
        FieldWireShape::Inline(ShapeRef::U64)
    );
}

#[test]
fn composes_transparent_with_custom_field_boundaries() {
    let value = TransparentCustom { value: 13 };
    let json = serde_json::to_value(&value).expect("value should serialize");
    assert_eq!(json, serde_json::json!("13"));
    assert_eq!(
        serde_json::from_value::<TransparentCustom>(json).expect("value should deserialize"),
        value
    );

    let serialize_field = first_serialize_field::<TransparentCustom>();
    let FieldWireShape::Inline(ShapeRef::Opaque(opaque)) = serialize_field.wire_shape else {
        panic!("custom serialized transparent field should be inline and opaque");
    };
    assert_eq!(opaque.reason, OpaqueReason::CustomSerializer);
    assert_path(opaque.detail, &["stringified", "serialize"]);

    let deserialize_field = first_deserialize_field::<TransparentCustom>();
    let FieldWireShape::Inline(ShapeRef::Opaque(opaque)) = deserialize_field.wire_shape else {
        panic!("custom deserialized transparent field should be inline and opaque");
    };
    assert_eq!(opaque.reason, OpaqueReason::CustomDeserializer);
    assert_path(opaque.detail, &["stringified", "deserialize"]);
}

#[test]
fn retains_custom_variant_boundary_details() {
    let value = CustomVariant::Value(17);
    let json = serde_json::to_value(&value).expect("value should serialize");
    assert_eq!(json, serde_json::json!({ "Value": "17" }));
    assert_eq!(
        serde_json::from_value::<CustomVariant>(json).expect("value should deserialize"),
        value
    );

    let serialize_variant = first_serialize_variant::<CustomVariant>();
    let SerializeVariantContent::Custom(opaque) = serialize_variant.content else {
        panic!("custom serialized variant should expose opaque content");
    };
    assert_eq!(opaque.reason, OpaqueReason::CustomSerializer);
    assert_path(opaque.detail, &["stringified", "serialize"]);

    let deserialize_variant = first_deserialize_variant::<CustomVariant>();
    let DeserializeVariantContent::Custom(opaque) = deserialize_variant.content else {
        panic!("custom deserialized variant should expose opaque content");
    };
    assert_eq!(opaque.reason, OpaqueReason::CustomDeserializer);
    assert_path(opaque.detail, &["stringified", "deserialize"]);
}

#[test]
fn declares_known_custom_variant_content() {
    let value = DeclaredCustomVariant::Value(FlatValue(19));
    let json = serde_json::to_value(&value).expect("value should serialize");
    assert_eq!(json, serde_json::json!({ "Value": { "custom": 19 } }));
    assert_eq!(
        serde_json::from_value::<DeclaredCustomVariant>(json).expect("value should deserialize"),
        value
    );

    let expected = flat_value_shape();
    let serialize_variant = first_serialize_variant::<DeclaredCustomVariant>();
    assert_eq!(
        serialize_variant.content,
        SerializeVariantContent::Shape(expected.clone())
    );
    let deserialize_variant = first_deserialize_variant::<DeclaredCustomVariant>();
    assert_eq!(
        deserialize_variant.content,
        DeserializeVariantContent::Shape(expected)
    );
}

fn first_serialize_field<T>() -> SerializeFieldShape
where
    T: SerializeShape,
{
    let definition = serialize_root_definition::<T>();
    let SerializeDefinitionKind::Struct(shape) = definition.kind else {
        panic!("definition should be a struct");
    };
    shape.fields.into_iter().next().expect("field should exist")
}

fn first_deserialize_field<T>() -> DeserializeFieldShape
where
    T: DeserializeShape,
{
    let definition = deserialize_root_definition::<T>();
    let DeserializeDefinitionKind::Struct(shape) = definition.kind else {
        panic!("definition should be a struct");
    };
    shape.fields.into_iter().next().expect("field should exist")
}

fn first_serialize_variant<T>() -> SerializeVariantShape
where
    T: SerializeShape,
{
    let definition = serialize_root_definition::<T>();
    let SerializeDefinitionKind::Enum(shape) = definition.kind else {
        panic!("definition should be an enum");
    };
    shape
        .variants
        .into_iter()
        .next()
        .expect("variant should exist")
}

fn first_deserialize_variant<T>() -> DeserializeVariantShape
where
    T: DeserializeShape,
{
    let definition = deserialize_root_definition::<T>();
    let DeserializeDefinitionKind::Enum(shape) = definition.kind else {
        panic!("definition should be an enum");
    };
    shape
        .variants
        .into_iter()
        .next()
        .expect("variant should exist")
}

mod flat_value {
    use std::collections::BTreeMap;

    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serializer;
    use serde::de::Error as _;
    use serde::ser::SerializeMap;

    use super::FlatValue;

    pub fn serialize<S>(value: &FlatValue, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("custom", &value.0)?;
        map.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<FlatValue, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut values = BTreeMap::<String, u64>::deserialize(deserializer)?;
        values
            .remove("custom")
            .map(FlatValue)
            .ok_or_else(|| D::Error::missing_field("custom"))
    }
}

mod stringified {
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serializer;
    use serde::de::Error as _;

    pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(D::Error::custom)
    }
}
