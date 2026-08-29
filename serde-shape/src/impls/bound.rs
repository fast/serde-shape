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

use alloc::vec;
use core::any::type_name;
use core::ops::Bound;

use crate::DefaultShape;
use crate::DeserializeContainerAttributes;
use crate::DeserializeDefinitionKind;
use crate::DeserializeEnumShape;
use crate::DeserializeFieldShape;
use crate::DeserializeShape;
use crate::DeserializeShapeContext;
use crate::DeserializeTypeName;
use crate::DeserializeVariantContent;
use crate::DeserializeVariantShape;
use crate::FieldMember;
use crate::FieldWireShape;
use crate::FieldsStyle;
use crate::SerializeContainerAttributes;
use crate::SerializeDefinitionKind;
use crate::SerializeEnumShape;
use crate::SerializeFieldShape;
use crate::SerializeShape;
use crate::SerializeShapeContext;
use crate::SerializeTypeName;
use crate::SerializeVariantContent;
use crate::SerializeVariantShape;
use crate::ShapeRef;
use crate::Tagging;

impl<T> SerializeShape for Bound<T>
where
    T: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        context.define_named_type(
            SerializeTypeName {
                rust_name: type_name::<Self>(),
                name: "Bound",
            },
            |context| {
                SerializeDefinitionKind::Enum(SerializeEnumShape {
                    repr: Tagging::External,
                    variants: vec![
                        serialize_bound_variant("Unbounded", None),
                        serialize_bound_variant("Included", Some(T::serialize_shape_in(context))),
                        serialize_bound_variant("Excluded", Some(T::serialize_shape_in(context))),
                    ],
                    attributes: SerializeContainerAttributes::default(),
                })
            },
        )
    }
}

impl<T> DeserializeShape for Bound<T>
where
    T: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        context.define_named_type(
            DeserializeTypeName {
                rust_name: type_name::<Self>(),
                name: "Bound",
            },
            |context| {
                DeserializeDefinitionKind::Enum(DeserializeEnumShape {
                    repr: Tagging::External,
                    variants: vec![
                        deserialize_bound_variant("Unbounded", None),
                        deserialize_bound_variant(
                            "Included",
                            Some(T::deserialize_shape_in(context)),
                        ),
                        deserialize_bound_variant(
                            "Excluded",
                            Some(T::deserialize_shape_in(context)),
                        ),
                    ],
                    attributes: DeserializeContainerAttributes::default(),
                })
            },
        )
    }
}

fn serialize_bound_variant(
    name: &'static str,
    value_shape: Option<ShapeRef>,
) -> SerializeVariantShape {
    let (style, fields) = match value_shape {
        Some(value_shape) => (
            FieldsStyle::Newtype,
            vec![SerializeFieldShape {
                member: FieldMember::Unnamed(0),
                name: "0",
                description: None,
                wire_shape: FieldWireShape::Value(value_shape),
                skip_if: None,
            }],
        ),
        None => (FieldsStyle::Unit, vec![]),
    };

    SerializeVariantShape {
        rust_name: name,
        name,
        description: None,
        style,
        content: SerializeVariantContent::Fields(fields),
        untagged: false,
    }
}

fn deserialize_bound_variant(
    name: &'static str,
    value_shape: Option<ShapeRef>,
) -> DeserializeVariantShape {
    let (style, fields) = match value_shape {
        Some(value_shape) => (
            FieldsStyle::Newtype,
            vec![DeserializeFieldShape {
                member: FieldMember::Unnamed(0),
                name: "0",
                aliases: vec!["0"],
                description: None,
                wire_shape: FieldWireShape::Value(value_shape),
                default: DefaultShape::None,
            }],
        ),
        None => (FieldsStyle::Unit, vec![]),
    };

    DeserializeVariantShape {
        rust_name: name,
        name,
        aliases: vec![name],
        description: None,
        style,
        content: DeserializeVariantContent::Fields(fields),
        other: false,
        untagged: false,
    }
}
