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
use alloc::vec::Vec;
use core::any::type_name;
use core::time::Duration;
#[cfg(feature = "std")]
use std::time::SystemTime;

use crate::DefaultShape;
use crate::DeserializeContainerAttributes;
use crate::DeserializeDefinitionKind;
use crate::DeserializeFieldShape;
use crate::DeserializeShape;
use crate::DeserializeShapeContext;
use crate::DeserializeStructShape;
use crate::DeserializeTypeName;
use crate::FieldMember;
use crate::FieldWireShape;
use crate::FieldsStyle;
use crate::SerializeContainerAttributes;
use crate::SerializeDefinitionKind;
use crate::SerializeFieldShape;
use crate::SerializeShape;
use crate::SerializeShapeContext;
use crate::SerializeStructShape;
use crate::SerializeTypeName;
use crate::ShapeRef;

macro_rules! time_shape {
    ($ty:ty, $name:literal, $($field:literal => $shape:expr),+ $(,)?) => {
        impl SerializeShape for $ty {
            fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
                serialize_time_shape(
                    context,
                    type_name::<Self>(),
                    $name,
                    [$(($field, $shape)),+],
                )
            }
        }

        impl DeserializeShape for $ty {
            fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
                deserialize_time_shape(
                    context,
                    type_name::<Self>(),
                    $name,
                    [$(($field, $shape)),+],
                )
            }
        }
    };
}

time_shape!(Duration, "Duration", "secs" => ShapeRef::U64, "nanos" => ShapeRef::U32);

#[cfg(feature = "std")]
time_shape!(
    SystemTime,
    "SystemTime",
    "secs_since_epoch" => ShapeRef::U64,
    "nanos_since_epoch" => ShapeRef::U32,
);

fn serialize_time_shape<const N: usize>(
    context: &mut SerializeShapeContext,
    rust_name: &'static str,
    name: &'static str,
    fields: [(&'static str, ShapeRef); N],
) -> ShapeRef {
    let fields: Vec<_> = fields
        .into_iter()
        .map(|(name, shape)| SerializeFieldShape {
            member: FieldMember::Named(name),
            name,
            description: None,
            wire_shape: FieldWireShape::Value(shape),
            skip_if: None,
        })
        .collect();
    context.define_named_type(SerializeTypeName { rust_name, name }, move |_| {
        SerializeDefinitionKind::Struct(SerializeStructShape {
            style: FieldsStyle::Struct,
            fields,
            attributes: SerializeContainerAttributes::default(),
        })
    })
}

fn deserialize_time_shape<const N: usize>(
    context: &mut DeserializeShapeContext,
    rust_name: &'static str,
    name: &'static str,
    fields: [(&'static str, ShapeRef); N],
) -> ShapeRef {
    let fields: Vec<_> = fields
        .into_iter()
        .map(|(name, shape)| DeserializeFieldShape {
            member: FieldMember::Named(name),
            name,
            aliases: vec![name],
            description: None,
            wire_shape: FieldWireShape::Value(shape),
            default: DefaultShape::None,
        })
        .collect();
    context.define_named_type(DeserializeTypeName { rust_name, name }, move |_| {
        DeserializeDefinitionKind::Struct(DeserializeStructShape {
            style: FieldsStyle::Struct,
            fields,
            attributes: DeserializeContainerAttributes {
                deny_unknown_fields: true,
                ..DeserializeContainerAttributes::default()
            },
        })
    })
}
