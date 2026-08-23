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
use core::time::Duration;

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
use crate::Tagging;

impl SerializeShape for Duration {
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        context.define_named_type(
            SerializeTypeName {
                rust_name: type_name::<Self>(),
                name: "Duration",
            },
            |_| {
                SerializeDefinitionKind::Struct(SerializeStructShape {
                    style: FieldsStyle::Struct,
                    fields: vec![
                        SerializeFieldShape {
                            member: FieldMember::Named("secs"),
                            name: "secs",
                            wire_shape: FieldWireShape::Value(ShapeRef::U64),
                            skip_if: None,
                        },
                        SerializeFieldShape {
                            member: FieldMember::Named("nanos"),
                            name: "nanos",
                            wire_shape: FieldWireShape::Value(ShapeRef::U32),
                            skip_if: None,
                        },
                    ],
                    attributes: SerializeContainerAttributes {
                        tagging: Tagging::External,
                        has_flatten: false,
                        transparent: false,
                        non_exhaustive: false,
                    },
                })
            },
        )
    }
}

impl DeserializeShape for Duration {
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        context.define_named_type(
            DeserializeTypeName {
                rust_name: type_name::<Self>(),
                name: "Duration",
            },
            |_| {
                DeserializeDefinitionKind::Struct(DeserializeStructShape {
                    style: FieldsStyle::Struct,
                    fields: vec![
                        DeserializeFieldShape {
                            member: FieldMember::Named("secs"),
                            name: "secs",
                            aliases: vec!["secs"],
                            wire_shape: FieldWireShape::Value(ShapeRef::U64),
                            default: DefaultShape::None,
                        },
                        DeserializeFieldShape {
                            member: FieldMember::Named("nanos"),
                            name: "nanos",
                            aliases: vec!["nanos"],
                            wire_shape: FieldWireShape::Value(ShapeRef::U32),
                            default: DefaultShape::None,
                        },
                    ],
                    attributes: DeserializeContainerAttributes {
                        tagging: Tagging::External,
                        deny_unknown_fields: true,
                        default: DefaultShape::None,
                        has_flatten: false,
                        transparent: false,
                        expecting: None,
                        non_exhaustive: false,
                    },
                })
            },
        )
    }
}
