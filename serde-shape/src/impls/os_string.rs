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

use alloc::boxed::Box;
use alloc::vec;
use std::ffi::OsStr;
use std::ffi::OsString;

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

impl SerializeShape for OsStr {
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        serialize_os_string(context, TypeName::of::<Self>("OsString"))
    }
}

impl SerializeShape for OsString {
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        serialize_os_string(context, TypeName::of::<Self>("OsString"))
    }
}

impl DeserializeShape for OsString {
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        deserialize_os_string(context, TypeName::of::<Self>("OsString"))
    }
}

impl DeserializeShape for Box<OsStr> {
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        deserialize_os_string(context, TypeName::of::<Self>("OsString"))
    }
}

fn serialize_os_string(context: &mut SerializeShapeContext, type_name: TypeName) -> ShapeRef {
    context.define_named_type(type_name, |_| {
        SerializeDefinitionKind::Enum(SerializeEnumShape {
            repr: Tagging::External,
            variants: vec![SerializeVariantShape {
                rust_name: platform_variant_name(),
                name: platform_variant_name(),
                description: None,
                style: FieldsStyle::Newtype,
                content: SerializeVariantContent::Fields(vec![SerializeFieldShape {
                    member: FieldMember::Unnamed(0),
                    name: "0",
                    description: None,
                    wire_shape: FieldWireShape::Value(platform_value_shape()),
                    skip_if: None,
                }]),
                untagged: false,
            }],
            attributes: SerializeContainerAttributes::default(),
        })
    })
}

fn deserialize_os_string(context: &mut DeserializeShapeContext, type_name: TypeName) -> ShapeRef {
    context.define_named_type(type_name, |_| {
        DeserializeDefinitionKind::Enum(DeserializeEnumShape {
            repr: Tagging::External,
            variants: vec![DeserializeVariantShape {
                rust_name: platform_variant_name(),
                name: platform_variant_name(),
                aliases: vec![platform_variant_name()],
                description: None,
                style: FieldsStyle::Newtype,
                content: DeserializeVariantContent::Fields(vec![DeserializeFieldShape {
                    member: FieldMember::Unnamed(0),
                    name: "0",
                    aliases: vec!["0"],
                    description: None,
                    wire_shape: FieldWireShape::Value(platform_value_shape()),
                    default: DefaultShape::None,
                }]),
                other: false,
                untagged: false,
            }],
            attributes: DeserializeContainerAttributes::default(),
        })
    })
}

#[cfg(unix)]
fn platform_variant_name() -> &'static str {
    "Unix"
}

#[cfg(windows)]
fn platform_variant_name() -> &'static str {
    "Windows"
}

#[cfg(unix)]
fn platform_value_shape() -> ShapeRef {
    ShapeRef::Seq(Box::new(ShapeRef::U8))
}

#[cfg(windows)]
fn platform_value_shape() -> ShapeRef {
    ShapeRef::Seq(Box::new(ShapeRef::U16))
}
