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
use core::any::type_name;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use std::net::SocketAddrV6;

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

macro_rules! union_shape {
    ($ty:ty => $binary:expr) => {
        impl SerializeShape for $ty {
            fn serialize_shape_in(_context: &mut SerializeShapeContext) -> ShapeRef {
                ShapeRef::union([ShapeRef::String, $binary])
            }
        }

        impl DeserializeShape for $ty {
            fn deserialize_shape_in(_context: &mut DeserializeShapeContext) -> ShapeRef {
                ShapeRef::union([ShapeRef::String, $binary])
            }
        }
    };
}

union_shape!(Ipv4Addr => ipv4_binary_shape());
union_shape!(Ipv6Addr => ipv6_binary_shape());
union_shape!(SocketAddrV4 => socket_v4_binary_shape());
union_shape!(SocketAddrV6 => socket_v6_binary_shape());

impl SerializeShape for IpAddr {
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        let binary = context.define_named_type(
            SerializeTypeName {
                rust_name: type_name::<Self>(),
                name: "IpAddr",
            },
            |_| {
                SerializeDefinitionKind::Enum(SerializeEnumShape {
                    repr: Tagging::External,
                    variants: vec![
                        serialize_newtype_variant("V4", ipv4_binary_shape()),
                        serialize_newtype_variant("V6", ipv6_binary_shape()),
                    ],
                    attributes: serialize_enum_attributes(),
                })
            },
        );
        ShapeRef::union([ShapeRef::String, binary])
    }
}

impl DeserializeShape for IpAddr {
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        let binary = context.define_named_type(
            DeserializeTypeName {
                rust_name: type_name::<Self>(),
                name: "IpAddr",
            },
            |_| {
                DeserializeDefinitionKind::Enum(DeserializeEnumShape {
                    repr: Tagging::External,
                    variants: vec![
                        deserialize_newtype_variant("V4", ipv4_binary_shape()),
                        deserialize_newtype_variant("V6", ipv6_binary_shape()),
                    ],
                    attributes: deserialize_enum_attributes(),
                })
            },
        );
        ShapeRef::union([ShapeRef::String, binary])
    }
}

impl SerializeShape for SocketAddr {
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        let binary = context.define_named_type(
            SerializeTypeName {
                rust_name: type_name::<Self>(),
                name: "SocketAddr",
            },
            |_| {
                SerializeDefinitionKind::Enum(SerializeEnumShape {
                    repr: Tagging::External,
                    variants: vec![
                        serialize_newtype_variant("V4", socket_v4_binary_shape()),
                        serialize_newtype_variant("V6", socket_v6_binary_shape()),
                    ],
                    attributes: serialize_enum_attributes(),
                })
            },
        );
        ShapeRef::union([ShapeRef::String, binary])
    }
}

impl DeserializeShape for SocketAddr {
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        let binary = context.define_named_type(
            DeserializeTypeName {
                rust_name: type_name::<Self>(),
                name: "SocketAddr",
            },
            |_| {
                DeserializeDefinitionKind::Enum(DeserializeEnumShape {
                    repr: Tagging::External,
                    variants: vec![
                        deserialize_newtype_variant("V4", socket_v4_binary_shape()),
                        deserialize_newtype_variant("V6", socket_v6_binary_shape()),
                    ],
                    attributes: deserialize_enum_attributes(),
                })
            },
        );
        ShapeRef::union([ShapeRef::String, binary])
    }
}

fn ipv4_binary_shape() -> ShapeRef {
    ShapeRef::Array {
        item: Box::new(ShapeRef::U8),
        len: 4,
    }
}

fn ipv6_binary_shape() -> ShapeRef {
    ShapeRef::Array {
        item: Box::new(ShapeRef::U8),
        len: 16,
    }
}

fn socket_v4_binary_shape() -> ShapeRef {
    ShapeRef::Tuple(vec![ipv4_binary_shape(), ShapeRef::U16])
}

fn socket_v6_binary_shape() -> ShapeRef {
    ShapeRef::Tuple(vec![ipv6_binary_shape(), ShapeRef::U16])
}

fn serialize_newtype_variant(name: &'static str, shape: ShapeRef) -> SerializeVariantShape {
    SerializeVariantShape {
        rust_name: name,
        name,
        style: FieldsStyle::Newtype,
        content: SerializeVariantContent::Fields(vec![SerializeFieldShape {
            member: FieldMember::Unnamed(0),
            name: "0",
            wire_shape: FieldWireShape::Value(shape),
            skip_if: None,
        }]),
        untagged: false,
    }
}

fn deserialize_newtype_variant(name: &'static str, shape: ShapeRef) -> DeserializeVariantShape {
    DeserializeVariantShape {
        rust_name: name,
        name,
        aliases: vec![name],
        style: FieldsStyle::Newtype,
        content: DeserializeVariantContent::Fields(vec![DeserializeFieldShape {
            member: FieldMember::Unnamed(0),
            name: "0",
            aliases: vec!["0"],
            wire_shape: FieldWireShape::Value(shape),
            default: DefaultShape::None,
        }]),
        other: false,
        untagged: false,
    }
}

fn serialize_enum_attributes() -> SerializeContainerAttributes {
    SerializeContainerAttributes {
        tagging: Tagging::External,
        has_flatten: false,
        transparent: false,
        non_exhaustive: false,
    }
}

fn deserialize_enum_attributes() -> DeserializeContainerAttributes {
    DeserializeContainerAttributes {
        tagging: Tagging::External,
        deny_unknown_fields: false,
        default: DefaultShape::None,
        has_flatten: false,
        transparent: false,
        expecting: None,
        non_exhaustive: false,
    }
}
