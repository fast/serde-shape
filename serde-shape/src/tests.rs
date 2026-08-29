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

use alloc::borrow::Cow;
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::collections::BinaryHeap;
use alloc::collections::LinkedList;
use alloc::collections::VecDeque;
use alloc::ffi::CString;
use alloc::rc::Rc;
use alloc::rc::Weak as RcWeak;
use alloc::string::String;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::cell::Cell;
use core::cell::RefCell;
use core::cmp::Reverse;
use core::ffi::CStr;
use core::num::Saturating;
use core::num::Wrapping;
use core::ops::Bound;
use core::ops::Range;
use core::ops::RangeFrom;
use core::ops::RangeInclusive;
use core::ops::RangeTo;

use crate::DeserializeDefinitionKind;
use crate::DeserializeShape;
use crate::DeserializeShapeContext;
use crate::DeserializeShapeGraph;
use crate::FieldWireShape;
use crate::FieldsStyle;
use crate::OpaqueReason;
use crate::OpaqueShape;
use crate::SerializeDefinitionKind;
use crate::SerializeShape;
use crate::SerializeShapeContext;
use crate::SerializeShapeGraph;
use crate::ShapeRef;
use crate::Tagging;
use crate::TypeName;

struct BorrowedShape;

struct OwnedShape(BorrowedShape);

impl ToOwned for BorrowedShape {
    type Owned = OwnedShape;

    fn to_owned(&self) -> Self::Owned {
        OwnedShape(BorrowedShape)
    }
}

impl Borrow<BorrowedShape> for OwnedShape {
    fn borrow(&self) -> &BorrowedShape {
        &self.0
    }
}

impl SerializeShape for BorrowedShape {
    fn serialize_shape_in(_context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::U8
    }
}

impl DeserializeShape for OwnedShape {
    fn deserialize_shape_in(_context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::String
    }
}

#[test]
fn classifies_flat_numeric_shapes() {
    assert!(ShapeRef::I8.is_signed_integer());
    assert!(ShapeRef::Usize.is_unsigned_integer());
    assert!(ShapeRef::I128.is_integer());
    assert!(ShapeRef::U64.is_integer());
    assert!(ShapeRef::F32.is_float());
    assert!(ShapeRef::F64.is_number());
    assert!(!ShapeRef::String.is_number());
}

#[test]
fn classifies_union_numeric_shapes() {
    assert!(ShapeRef::union([ShapeRef::I8, ShapeRef::U64]).is_integer());
    assert!(ShapeRef::union([ShapeRef::F32, ShapeRef::F64]).is_float());
    assert!(ShapeRef::union([ShapeRef::I16, ShapeRef::F64]).is_number());
    assert!(!ShapeRef::union([ShapeRef::String, ShapeRef::U64]).is_integer());
}

#[test]
fn normalizes_union_shapes() {
    assert_eq!(ShapeRef::try_union([]), None);
    assert_eq!(ShapeRef::union([ShapeRef::String]), ShapeRef::String);
    assert_eq!(
        ShapeRef::union([ShapeRef::String, ShapeRef::I8]),
        ShapeRef::union([ShapeRef::I8, ShapeRef::String])
    );

    let union = ShapeRef::union([
        ShapeRef::String,
        ShapeRef::I8,
        ShapeRef::union([ShapeRef::U64, ShapeRef::String]),
        ShapeRef::I8,
    ]);
    let ShapeRef::Union(union) = union else {
        panic!("multiple distinct alternatives should produce a union");
    };
    assert_eq!(
        union.alternatives(),
        &[ShapeRef::I8, ShapeRef::U64, ShapeRef::String]
    );
}

#[test]
fn keeps_distinct_definition_builders_with_the_same_type_name() {
    let mut serialize = SerializeShapeContext::default();
    let first = serialize.define_named_type(
        TypeName {
            rust_name: "duplicate::Type",
            name: "First",
        },
        |_| {
            SerializeDefinitionKind::Opaque(OpaqueShape {
                type_name: "duplicate::Type",
                reason: OpaqueReason::Unsupported,
                detail: Some("first"),
            })
        },
    );
    let second = serialize.define_named_type(
        TypeName {
            rust_name: "duplicate::Type",
            name: "Second",
        },
        |_| {
            SerializeDefinitionKind::Opaque(OpaqueShape {
                type_name: "duplicate::Type",
                reason: OpaqueReason::Unsupported,
                detail: Some("second"),
            })
        },
    );

    assert_ne!(first, second);
    assert_eq!(serialize.finish().len(), 2);

    let mut deserialize = DeserializeShapeContext::default();
    let first = deserialize.define_named_type(
        TypeName {
            rust_name: "duplicate::Type",
            name: "First",
        },
        |_| {
            DeserializeDefinitionKind::Opaque(OpaqueShape {
                type_name: "duplicate::Type",
                reason: OpaqueReason::Unsupported,
                detail: Some("first"),
            })
        },
    );
    let second = deserialize.define_named_type(
        TypeName {
            rust_name: "duplicate::Type",
            name: "Second",
        },
        |_| {
            DeserializeDefinitionKind::Opaque(OpaqueShape {
                type_name: "duplicate::Type",
                reason: OpaqueReason::Unsupported,
                detail: Some("second"),
            })
        },
    );

    assert_ne!(first, second);
    assert_eq!(deserialize.finish().len(), 2);
}

#[cfg(target_has_atomic = "ptr")]
#[test]
fn maps_atomic_shapes() {
    assert_eq!(
        SerializeShapeGraph::for_type::<core::sync::atomic::AtomicUsize>().root(),
        &ShapeRef::Usize
    );
    assert_eq!(
        DeserializeShapeGraph::for_type::<core::sync::atomic::AtomicUsize>().root(),
        &ShapeRef::Usize
    );
}

#[test]
fn builds_map_shape() {
    let serialize_shape = SerializeShapeGraph::for_type::<BTreeMap<String, Option<u16>>>();
    let deserialize_shape = DeserializeShapeGraph::for_type::<BTreeMap<String, Option<u16>>>();
    let expected = ShapeRef::Map {
        key: Box::new(ShapeRef::String),
        value: Box::new(ShapeRef::Option(Box::new(ShapeRef::U16))),
    };

    assert_eq!(serialize_shape.root(), &expected);
    assert!(serialize_shape.root_definition().is_none());
    assert!(serialize_shape.definitions().is_empty());
    assert_eq!(deserialize_shape.root(), &expected);
    assert!(deserialize_shape.root_definition().is_none());
    assert!(deserialize_shape.definitions().is_empty());
}

#[test]
fn distinguishes_byte_sequences_from_borrowed_byte_input() {
    assert_eq!(
        <[u8] as SerializeShape>::serialize_shape().root(),
        &ShapeRef::Seq(Box::new(ShapeRef::U8))
    );
    assert_eq!(
        SerializeShapeGraph::for_type::<Vec<u8>>().root(),
        &ShapeRef::Seq(Box::new(ShapeRef::U8))
    );
    assert_eq!(
        <&[u8] as DeserializeShape>::deserialize_shape().root(),
        &ShapeRef::Bytes
    );
    assert_eq!(
        <&str as DeserializeShape>::deserialize_shape().root(),
        &ShapeRef::String
    );
    assert_eq!(
        <Box<[u8]> as DeserializeShape>::deserialize_shape().root(),
        &ShapeRef::Seq(Box::new(ShapeRef::U8))
    );
}

#[test]
fn maps_result_as_an_externally_tagged_enum() {
    let serialize = SerializeShapeGraph::for_type::<Result<u8, String>>();
    let ShapeRef::Definition(id) = serialize.root() else {
        panic!("result should produce a named definition");
    };
    let SerializeDefinitionKind::Enum(shape) = &serialize.definition(*id).unwrap().kind else {
        panic!("result definition should be an enum");
    };

    assert_eq!(shape.repr, Tagging::External);
    assert_eq!(shape.variants.len(), 2);
    assert_eq!(shape.variants[0].name, "Ok");
    assert_eq!(shape.variants[0].style, FieldsStyle::Newtype);
    let crate::SerializeVariantContent::Fields(fields) = &shape.variants[0].content else {
        panic!("Ok should contain one reflected field");
    };
    assert_eq!(fields[0].wire_shape, FieldWireShape::Value(ShapeRef::U8));

    let deserialize = DeserializeShapeGraph::for_type::<Result<u8, String>>();
    let ShapeRef::Definition(id) = deserialize.root() else {
        panic!("result should produce a named definition");
    };
    let DeserializeDefinitionKind::Enum(shape) = &deserialize.definition(*id).unwrap().kind else {
        panic!("result definition should be an enum");
    };
    assert_eq!(shape.variants[1].name, "Err");
    let crate::DeserializeVariantContent::Fields(fields) = &shape.variants[1].content else {
        panic!("Err should contain one reflected field");
    };
    assert_eq!(
        fields[0].wire_shape,
        FieldWireShape::Value(ShapeRef::String)
    );
}

#[test]
fn maps_duration_as_serde_struct_fields() {
    let deserialize = DeserializeShapeGraph::for_type::<core::time::Duration>();
    let ShapeRef::Definition(id) = deserialize.root() else {
        panic!("duration should produce a named definition");
    };
    let DeserializeDefinitionKind::Struct(shape) = &deserialize.definition(*id).unwrap().kind
    else {
        panic!("duration definition should be a struct");
    };

    assert!(shape.attributes.deny_unknown_fields);
    assert_eq!(shape.fields[0].name, "secs");
    assert_eq!(
        shape.fields[0].wire_shape,
        FieldWireShape::Value(ShapeRef::U64)
    );
    assert_eq!(shape.fields[1].name, "nanos");
    assert_eq!(
        shape.fields[1].wire_shape,
        FieldWireShape::Value(ShapeRef::U32)
    );
}

#[test]
fn maps_range_struct_shapes() {
    assert_range_shapes::<Range<u8>>("Range", &["start", "end"]);
    assert_range_shapes::<RangeFrom<u8>>("RangeFrom", &["start"]);
    assert_range_shapes::<RangeInclusive<u8>>("RangeInclusive", &["start", "end"]);
    assert_range_shapes::<RangeTo<u8>>("RangeTo", &["end"]);
}

#[test]
fn maps_bound_as_an_externally_tagged_enum() {
    let serialize = SerializeShapeGraph::for_type::<Bound<u8>>();
    let SerializeDefinitionKind::Enum(shape) = &serialize.root_definition().unwrap().kind else {
        panic!("bound serialization shape should be an enum");
    };
    assert_eq!(shape.repr, Tagging::External);
    assert_eq!(shape.variants[0].name, "Unbounded");
    assert_eq!(shape.variants[0].style, FieldsStyle::Unit);
    assert_eq!(shape.variants[1].name, "Included");
    assert_eq!(shape.variants[1].style, FieldsStyle::Newtype);

    let deserialize = DeserializeShapeGraph::for_type::<Bound<u8>>();
    let DeserializeDefinitionKind::Enum(shape) = &deserialize.root_definition().unwrap().kind
    else {
        panic!("bound deserialization shape should be an enum");
    };
    assert_eq!(shape.variants[2].name, "Excluded");
    let crate::DeserializeVariantContent::Fields(fields) = &shape.variants[2].content else {
        panic!("Excluded should contain one reflected field");
    };
    assert_eq!(fields[0].wire_shape, FieldWireShape::Value(ShapeRef::U8));
}

fn assert_range_shapes<T>(type_name: &str, field_names: &[&str])
where
    T: SerializeShape + DeserializeShape,
{
    let serialize = T::serialize_shape();
    let serialize_definition = serialize.root_definition().unwrap();
    assert_eq!(serialize_definition.type_name.name, type_name);
    let SerializeDefinitionKind::Struct(shape) = &serialize_definition.kind else {
        panic!("range serialization shape should be a struct");
    };
    assert_eq!(
        shape
            .fields
            .iter()
            .map(|field| field.name)
            .collect::<Vec<_>>(),
        field_names
    );

    let deserialize = T::deserialize_shape();
    let deserialize_definition = deserialize.root_definition().unwrap();
    assert_eq!(deserialize_definition.type_name.name, type_name);
    let DeserializeDefinitionKind::Struct(shape) = &deserialize_definition.kind else {
        panic!("range deserialization shape should be a struct");
    };
    assert!(shape.attributes.deny_unknown_fields);
    assert_eq!(
        shape
            .fields
            .iter()
            .map(|field| field.name)
            .collect::<Vec<_>>(),
        field_names
    );
}

#[test]
fn supports_serde_tuple_arity() {
    type Tuple16 = (
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
    );

    let graph = SerializeShapeGraph::for_type::<Tuple16>();
    let ShapeRef::Tuple(items) = graph.root() else {
        panic!("16-element tuple should produce a tuple shape");
    };
    assert_eq!(items, &vec![ShapeRef::U8; 16]);
}

#[test]
fn maps_common_core_and_alloc_shapes() {
    assert_eq!(
        DeserializeShapeGraph::for_type::<Cow<'static, str>>().root(),
        &ShapeRef::String
    );
    assert_eq!(
        SerializeShapeGraph::for_type::<Cell<u8>>().root(),
        &ShapeRef::U8
    );
    assert_eq!(
        DeserializeShapeGraph::for_type::<Wrapping<i16>>().root(),
        &ShapeRef::I16
    );
    assert_eq!(
        SerializeShapeGraph::for_type::<Saturating<String>>().root(),
        &ShapeRef::String
    );
    assert_eq!(
        DeserializeShapeGraph::for_type::<Saturating<u16>>().root(),
        &ShapeRef::U16
    );
    assert_eq!(
        SerializeShapeGraph::for_type::<Reverse<u32>>().root(),
        &ShapeRef::U32
    );
    assert_eq!(
        DeserializeShapeGraph::for_type::<VecDeque<u8>>().root(),
        &ShapeRef::Seq(Box::new(ShapeRef::U8))
    );
    assert_eq!(
        SerializeShapeGraph::for_type::<LinkedList<i32>>().root(),
        &ShapeRef::Seq(Box::new(ShapeRef::I32))
    );
    assert_eq!(
        DeserializeShapeGraph::for_type::<BinaryHeap<u16>>().root(),
        &ShapeRef::Seq(Box::new(ShapeRef::U16))
    );
    assert_eq!(
        SerializeShapeGraph::for_type::<CString>().root(),
        &ShapeRef::Bytes
    );
    assert_eq!(
        DeserializeShapeGraph::for_type::<CString>().root(),
        &ShapeRef::Bytes
    );
    assert_eq!(
        <Box<CStr> as DeserializeShape>::deserialize_shape().root(),
        &ShapeRef::Bytes
    );
}

#[test]
fn follows_shared_pointer_serde_shapes() {
    assert_eq!(
        <Rc<str> as SerializeShape>::serialize_shape().root(),
        &ShapeRef::String
    );
    assert_eq!(
        <Rc<[u8]> as DeserializeShape>::deserialize_shape().root(),
        &ShapeRef::Seq(Box::new(ShapeRef::U8))
    );
    assert_eq!(
        SerializeShapeGraph::for_type::<RcWeak<u16>>().root(),
        &ShapeRef::Option(Box::new(ShapeRef::U16))
    );
    #[cfg(target_has_atomic = "ptr")]
    assert_eq!(
        <Arc<[u8]> as DeserializeShape>::deserialize_shape().root(),
        &ShapeRef::Seq(Box::new(ShapeRef::U8))
    );
}

#[test]
fn accepts_serde_serializable_collection_and_wrapper_bounds() {
    assert_eq!(
        SerializeShapeGraph::for_type::<BinaryHeap<BorrowedShape>>().root(),
        &ShapeRef::Seq(Box::new(ShapeRef::U8))
    );
    assert_eq!(
        <RefCell<str> as SerializeShape>::serialize_shape().root(),
        &ShapeRef::String
    );
}

#[test]
fn follows_cow_directional_serde_bounds() {
    assert_eq!(
        SerializeShapeGraph::for_type::<Cow<'static, BorrowedShape>>().root(),
        &ShapeRef::U8
    );
    assert_eq!(
        DeserializeShapeGraph::for_type::<Cow<'static, BorrowedShape>>().root(),
        &ShapeRef::String
    );
}

#[cfg(feature = "std")]
#[test]
fn maps_common_std_shapes() {
    assert_eq!(
        SerializeShapeGraph::for_type::<std::path::Path>().root(),
        &ShapeRef::String
    );
    assert_eq!(
        SerializeShapeGraph::for_type::<std::path::PathBuf>().root(),
        &ShapeRef::String
    );
    assert_eq!(
        DeserializeShapeGraph::for_type::<std::path::PathBuf>().root(),
        &ShapeRef::String
    );
    assert_eq!(
        <Box<std::path::Path> as DeserializeShape>::deserialize_shape().root(),
        &ShapeRef::String
    );
    assert_eq!(
        <&std::path::Path as DeserializeShape>::deserialize_shape().root(),
        &ShapeRef::String
    );
    assert_eq!(
        <std::sync::Mutex<str> as SerializeShape>::serialize_shape().root(),
        &ShapeRef::String
    );
    assert_eq!(
        <std::sync::RwLock<str> as SerializeShape>::serialize_shape().root(),
        &ShapeRef::String
    );

    let system_time = DeserializeShapeGraph::for_type::<std::time::SystemTime>();
    let DeserializeDefinitionKind::Struct(shape) = &system_time.root_definition().unwrap().kind
    else {
        panic!("system time shape should be a struct");
    };
    assert_eq!(shape.fields[0].name, "secs_since_epoch");
    assert_eq!(shape.fields[1].name, "nanos_since_epoch");
    assert!(shape.attributes.deny_unknown_fields);
}

#[test]
fn maps_network_shapes_without_std() {
    let ipv4_binary = ShapeRef::Array {
        item: Box::new(ShapeRef::U8),
        len: 4,
    };
    assert_eq!(
        SerializeShapeGraph::for_type::<core::net::Ipv4Addr>().root(),
        &ShapeRef::union([ShapeRef::String, ipv4_binary.clone()])
    );

    let socket = DeserializeShapeGraph::for_type::<core::net::SocketAddr>();
    let ShapeRef::Union(root) = socket.root() else {
        panic!("socket address should reflect human-readable and binary shapes");
    };
    assert!(root.alternatives().contains(&ShapeRef::String));
    let definition_id = root
        .alternatives()
        .iter()
        .find_map(|shape| match shape {
            ShapeRef::Definition(id) => Some(*id),
            _ => None,
        })
        .expect("binary socket shape should be a named enum");
    let DeserializeDefinitionKind::Enum(shape) = &socket.definition(definition_id).unwrap().kind
    else {
        panic!("binary socket shape should be an enum");
    };
    assert_eq!(shape.repr, Tagging::External);
    assert_eq!(shape.variants[0].name, "V4");
    let crate::DeserializeVariantContent::Fields(fields) = &shape.variants[0].content else {
        panic!("V4 should contain its binary socket tuple");
    };
    assert_eq!(
        fields[0].wire_shape,
        FieldWireShape::Value(ShapeRef::Tuple(vec![ipv4_binary, ShapeRef::U16]))
    );
}
