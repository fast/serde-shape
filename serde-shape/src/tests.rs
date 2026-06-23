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
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::collections::BinaryHeap;
use alloc::collections::LinkedList;
use alloc::collections::VecDeque;
use alloc::string::String;
use core::cell::Cell;
use core::cmp::Reverse;
use core::num::Wrapping;

use crate::DeserializeShapeGraph;
use crate::SerializeShapeGraph;
use crate::ShapeRef;

#[test]
fn builds_map_shape() {
    let serialize_shape = SerializeShapeGraph::for_type::<BTreeMap<String, Option<u16>>>();
    let deserialize_shape = DeserializeShapeGraph::for_type::<BTreeMap<String, Option<u16>>>();
    let expected = ShapeRef::Map {
        key: Box::new(ShapeRef::String),
        value: Box::new(ShapeRef::Option(Box::new(ShapeRef::U16))),
    };

    assert_eq!(serialize_shape.root, expected);
    assert!(serialize_shape.definitions.is_empty());
    assert_eq!(deserialize_shape.root, expected);
    assert!(deserialize_shape.definitions.is_empty());
}

#[test]
fn maps_common_core_and_alloc_shapes() {
    assert_eq!(
        DeserializeShapeGraph::for_type::<Cow<'static, str>>().root,
        ShapeRef::String
    );
    assert_eq!(
        SerializeShapeGraph::for_type::<Cell<u8>>().root,
        ShapeRef::U8
    );
    assert_eq!(
        DeserializeShapeGraph::for_type::<Wrapping<i16>>().root,
        ShapeRef::I16
    );
    assert_eq!(
        SerializeShapeGraph::for_type::<Reverse<u32>>().root,
        ShapeRef::U32
    );
    assert_eq!(
        DeserializeShapeGraph::for_type::<VecDeque<u8>>().root,
        ShapeRef::Seq(Box::new(ShapeRef::U8))
    );
    assert_eq!(
        SerializeShapeGraph::for_type::<LinkedList<i32>>().root,
        ShapeRef::Seq(Box::new(ShapeRef::I32))
    );
    assert_eq!(
        DeserializeShapeGraph::for_type::<BinaryHeap<u16>>().root,
        ShapeRef::Seq(Box::new(ShapeRef::U16))
    );
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

#[cfg(target_has_atomic = "ptr")]
#[test]
fn maps_atomic_shapes() {
    assert_eq!(
        SerializeShapeGraph::for_type::<core::sync::atomic::AtomicUsize>().root,
        ShapeRef::Usize
    );
    assert_eq!(
        DeserializeShapeGraph::for_type::<core::sync::atomic::AtomicUsize>().root,
        ShapeRef::Usize
    );
}
