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
use alloc::ffi::CString;
use core::ffi::CStr;

use crate::DeserializeShape;
use crate::DeserializeShapeContext;
use crate::SerializeShape;
use crate::SerializeShapeContext;
use crate::ShapeRef;

impl SerializeShape for CStr {
    fn serialize_shape_in(_context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Bytes
    }
}

impl SerializeShape for CString {
    fn serialize_shape_in(_context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Bytes
    }
}

impl DeserializeShape for CString {
    fn deserialize_shape_in(_context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Bytes
    }
}

impl DeserializeShape for Box<CStr> {
    fn deserialize_shape_in(_context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Bytes
    }
}
