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

use crate::DefaultShape;
use crate::DeserializeContainerAttributes;
use crate::DeserializeDefinitionKind;
use crate::DeserializeEnumShape;
use crate::DeserializeFieldShape;
use crate::DeserializeShape;
use crate::DeserializeShapeContext;
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
use crate::SerializeVariantContent;
use crate::SerializeVariantShape;
use crate::ShapeRef;
use crate::Tagging;
use crate::TypeName;

impl<T, E> SerializeShape for Result<T, E>
where
    T: SerializeShape,
    E: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        context.define_named_type(TypeName::of::<Self>("Result"), |context| {
            SerializeDefinitionKind::Enum(SerializeEnumShape {
                repr: Tagging::External,
                variants: vec![
                    serialize_result_variant("Ok", T::serialize_shape_in(context)),
                    serialize_result_variant("Err", E::serialize_shape_in(context)),
                ],
                attributes: SerializeContainerAttributes::default(),
            })
        })
    }
}

impl<T, E> DeserializeShape for Result<T, E>
where
    T: DeserializeShape,
    E: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        context.define_named_type(TypeName::of::<Self>("Result"), |context| {
            DeserializeDefinitionKind::Enum(DeserializeEnumShape {
                repr: Tagging::External,
                variants: vec![
                    deserialize_result_variant("Ok", T::deserialize_shape_in(context)),
                    deserialize_result_variant("Err", E::deserialize_shape_in(context)),
                ],
                attributes: DeserializeContainerAttributes::default(),
            })
        })
    }
}

fn serialize_result_variant(name: &'static str, shape: ShapeRef) -> SerializeVariantShape {
    SerializeVariantShape {
        rust_name: name,
        name,
        description: None,
        style: FieldsStyle::Newtype,
        content: SerializeVariantContent::Fields(vec![SerializeFieldShape {
            member: FieldMember::Unnamed(0),
            name: "0",
            description: None,
            wire_shape: FieldWireShape::Value(shape),
            skip_if: None,
        }]),
        untagged: false,
    }
}

fn deserialize_result_variant(name: &'static str, shape: ShapeRef) -> DeserializeVariantShape {
    DeserializeVariantShape {
        rust_name: name,
        name,
        aliases: vec![name],
        description: None,
        style: FieldsStyle::Newtype,
        content: DeserializeVariantContent::Fields(vec![DeserializeFieldShape {
            member: FieldMember::Unnamed(0),
            name: "0",
            aliases: vec!["0"],
            description: None,
            wire_shape: FieldWireShape::Value(shape),
            default: DefaultShape::None,
        }]),
        other: false,
        untagged: false,
    }
}
