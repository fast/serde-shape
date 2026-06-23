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

use alloc::string::String;

use crate::DeserializeShape;
use crate::DeserializeShapeContext;
use crate::SerializeShape;
use crate::SerializeShapeContext;
use crate::ShapeRef;

macro_rules! primitive_shape {
    ($($ty:ty => $shape:expr;)+) => {
        $(
            impl SerializeShape for $ty {
                fn serialize_shape_in(_context: &mut SerializeShapeContext) -> ShapeRef {
                    $shape
                }
            }

            impl DeserializeShape for $ty {
                fn deserialize_shape_in(_context: &mut DeserializeShapeContext) -> ShapeRef {
                    $shape
                }
            }
        )+
    };
}

primitive_shape! {
    () => ShapeRef::Unit;
    bool => ShapeRef::Bool;
    char => ShapeRef::Char;
    i8 => ShapeRef::I8;
    i16 => ShapeRef::I16;
    i32 => ShapeRef::I32;
    i64 => ShapeRef::I64;
    i128 => ShapeRef::I128;
    isize => ShapeRef::Isize;
    u8 => ShapeRef::U8;
    u16 => ShapeRef::U16;
    u32 => ShapeRef::U32;
    u64 => ShapeRef::U64;
    u128 => ShapeRef::U128;
    usize => ShapeRef::Usize;
    f32 => ShapeRef::F32;
    f64 => ShapeRef::F64;
    str => ShapeRef::String;
    [u8] => ShapeRef::Bytes;
    String => ShapeRef::String;
    core::num::NonZeroI8 => ShapeRef::I8;
    core::num::NonZeroI16 => ShapeRef::I16;
    core::num::NonZeroI32 => ShapeRef::I32;
    core::num::NonZeroI64 => ShapeRef::I64;
    core::num::NonZeroI128 => ShapeRef::I128;
    core::num::NonZeroIsize => ShapeRef::Isize;
    core::num::NonZeroU8 => ShapeRef::U8;
    core::num::NonZeroU16 => ShapeRef::U16;
    core::num::NonZeroU32 => ShapeRef::U32;
    core::num::NonZeroU64 => ShapeRef::U64;
    core::num::NonZeroU128 => ShapeRef::U128;
    core::num::NonZeroUsize => ShapeRef::Usize;
}

#[cfg(feature = "std")]
primitive_shape! {
    std::path::Path => ShapeRef::String;
    std::path::PathBuf => ShapeRef::String;
    std::net::IpAddr => ShapeRef::String;
    std::net::Ipv4Addr => ShapeRef::String;
    std::net::Ipv6Addr => ShapeRef::String;
    std::net::SocketAddr => ShapeRef::String;
    std::net::SocketAddrV4 => ShapeRef::String;
    std::net::SocketAddrV6 => ShapeRef::String;
}

#[cfg(target_has_atomic = "8")]
primitive_shape! {
    core::sync::atomic::AtomicBool => ShapeRef::Bool;
    core::sync::atomic::AtomicI8 => ShapeRef::I8;
    core::sync::atomic::AtomicU8 => ShapeRef::U8;
}

#[cfg(target_has_atomic = "16")]
primitive_shape! {
    core::sync::atomic::AtomicI16 => ShapeRef::I16;
    core::sync::atomic::AtomicU16 => ShapeRef::U16;
}

#[cfg(target_has_atomic = "32")]
primitive_shape! {
    core::sync::atomic::AtomicI32 => ShapeRef::I32;
    core::sync::atomic::AtomicU32 => ShapeRef::U32;
}

#[cfg(target_has_atomic = "64")]
primitive_shape! {
    core::sync::atomic::AtomicI64 => ShapeRef::I64;
    core::sync::atomic::AtomicU64 => ShapeRef::U64;
}

#[cfg(target_has_atomic = "ptr")]
primitive_shape! {
    core::sync::atomic::AtomicIsize => ShapeRef::Isize;
    core::sync::atomic::AtomicUsize => ShapeRef::Usize;
}
