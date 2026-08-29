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
use core::ops::Range;
use core::ops::RangeFrom;
use core::ops::RangeInclusive;
use core::ops::RangeTo;

use crate::DefaultShape;
use crate::DeserializeContainerAttributes;
use crate::DeserializeDefinitionKind;
use crate::DeserializeFieldShape;
use crate::DeserializeShape;
use crate::DeserializeShapeContext;
use crate::DeserializeStructShape;
use crate::FieldMember;
use crate::FieldWireShape;
use crate::FieldsStyle;
use crate::SerializeContainerAttributes;
use crate::SerializeDefinitionKind;
use crate::SerializeFieldShape;
use crate::SerializeShape;
use crate::SerializeShapeContext;
use crate::SerializeStructShape;
use crate::ShapeRef;
use crate::TypeName;

macro_rules! range_shape {
    ($($range:ident { $($field:ident),+ $(,)? })+) => {
        $(
            impl<T> SerializeShape for $range<T>
            where
                T: SerializeShape,
            {
                fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
                    context.define_named_type(
                        TypeName::of::<Self>(stringify!($range)),
                        |context| {
                            SerializeDefinitionKind::Struct(SerializeStructShape {
                                style: FieldsStyle::Struct,
                                fields: vec![
                                    $(SerializeFieldShape {
                                        member: FieldMember::Named(stringify!($field)),
                                        name: stringify!($field),
                                        description: None,
                                        wire_shape: FieldWireShape::Value(
                                            T::serialize_shape_in(context),
                                        ),
                                        skip_if: None,
                                    }),+
                                ],
                                attributes: SerializeContainerAttributes::default(),
                            })
                        },
                    )
                }
            }

            impl<T> DeserializeShape for $range<T>
            where
                T: DeserializeShape,
            {
                fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
                    context.define_named_type(
                        TypeName::of::<Self>(stringify!($range)),
                        |context| {
                            DeserializeDefinitionKind::Struct(DeserializeStructShape {
                                style: FieldsStyle::Struct,
                                fields: vec![
                                    $(DeserializeFieldShape {
                                        member: FieldMember::Named(stringify!($field)),
                                        name: stringify!($field),
                                        aliases: vec![stringify!($field)],
                                        description: None,
                                        wire_shape: FieldWireShape::Value(
                                            T::deserialize_shape_in(context),
                                        ),
                                        default: DefaultShape::None,
                                    }),+
                                ],
                                attributes: DeserializeContainerAttributes {
                                    deny_unknown_fields: true,
                                    ..DeserializeContainerAttributes::default()
                                },
                            })
                        },
                    )
                }
            }
        )+
    };
}

range_shape! {
    Range { start, end }
    RangeFrom { start }
    RangeInclusive { start, end }
    RangeTo { end }
}
