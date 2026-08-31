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
use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::collections::BinaryHeap;
use alloc::collections::LinkedList;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
#[cfg(feature = "std")]
use core::hash::BuildHasher;
#[cfg(feature = "std")]
use core::hash::Hash;

use crate::DeserializeShape;
use crate::DeserializeShapeContext;
use crate::SerializeShape;
use crate::SerializeShapeContext;
use crate::ShapeRef;

impl<T> SerializeShape for Option<T>
where
    T: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Option(Box::new(T::serialize_shape_in(context)))
    }
}

impl<T> DeserializeShape for Option<T>
where
    T: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Option(Box::new(T::deserialize_shape_in(context)))
    }
}

macro_rules! seq_shape {
    (
        $(
            ($($generics:tt)*) $ty:ty
            where
                serialize { $($serialize_bounds:tt)* }
                deserialize { $($deserialize_bounds:tt)* }
            => $item:ty;
        )+
    ) => {
        $(
            impl<$($generics)*> SerializeShape for $ty
            where
                $($serialize_bounds)*
            {
                fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
                    ShapeRef::Seq(Box::new(<$item as SerializeShape>::serialize_shape_in(context)))
                }
            }

            impl<$($generics)*> DeserializeShape for $ty
            where
                $($deserialize_bounds)*
            {
                fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
                    ShapeRef::Seq(Box::new(<$item as DeserializeShape>::deserialize_shape_in(context)))
                }
            }
        )+
    };
}

seq_shape! {
    (T) Vec<T>
    where
        serialize { T: SerializeShape }
        deserialize { T: DeserializeShape }
    => T;

    (T) VecDeque<T>
    where
        serialize { T: SerializeShape }
        deserialize { T: DeserializeShape }
    => T;

    (T) LinkedList<T>
    where
        serialize { T: SerializeShape }
        deserialize { T: DeserializeShape }
    => T;

    (T) BinaryHeap<T>
    where
        serialize { T: SerializeShape }
        deserialize { T: Ord + DeserializeShape }
    => T;

    (T) BTreeSet<T>
    where
        serialize { T: SerializeShape }
        deserialize { T: DeserializeShape + Ord }
    => T;

}

impl<T> SerializeShape for [T]
where
    T: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Seq(Box::new(T::serialize_shape_in(context)))
    }
}

#[cfg(feature = "std")]
seq_shape! {
    (T, S) std::collections::HashSet<T, S>
    where
        serialize { T: SerializeShape }
        deserialize {
            T: DeserializeShape + Eq + Hash,
            S: BuildHasher + Default
        }
    => T;
}

impl<T> SerializeShape for [T; 0] {
    fn serialize_shape_in(_context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Array {
            item: Box::new(unobserved_empty_array_item::<T>()),
            len: 0,
        }
    }
}

impl<T> DeserializeShape for [T; 0] {
    fn deserialize_shape_in(_context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Array {
            item: Box::new(unobserved_empty_array_item::<T>()),
            len: 0,
        }
    }
}

fn unobserved_empty_array_item<T>() -> ShapeRef {
    ShapeRef::Opaque(crate::OpaqueShape {
        type_name: core::any::type_name::<T>(),
        reason: crate::OpaqueReason::Unobserved,
        detail: Some("zero-length array has no elements"),
    })
}

macro_rules! array_shape {
    ($($len:literal)+) => {
        $(
            impl<T> SerializeShape for [T; $len]
            where
                T: SerializeShape,
            {
                fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
                    ShapeRef::Array {
                        item: Box::new(T::serialize_shape_in(context)),
                        len: $len,
                    }
                }
            }

            impl<T> DeserializeShape for [T; $len]
            where
                T: DeserializeShape,
            {
                fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
                    ShapeRef::Array {
                        item: Box::new(T::deserialize_shape_in(context)),
                        len: $len,
                    }
                }
            }
        )+
    };
}

array_shape! {
    1 2 3 4 5 6 7 8 9 10
    11 12 13 14 15 16 17 18 19 20
    21 22 23 24 25 26 27 28 29 30
    31 32
}

macro_rules! map_shape {
    (
        $(
            ($($generics:tt)*) $ty:ty
            where
                serialize { $($serialize_bounds:tt)* }
                deserialize { $($deserialize_bounds:tt)* }
            => ($key:ty, $value:ty);
        )+
    ) => {
        $(
            impl<$($generics)*> SerializeShape for $ty
            where
                $($serialize_bounds)*
            {
                fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
                    ShapeRef::Map {
                        key: Box::new(<$key as SerializeShape>::serialize_shape_in(context)),
                        value: Box::new(<$value as SerializeShape>::serialize_shape_in(context)),
                    }
                }
            }

            impl<$($generics)*> DeserializeShape for $ty
            where
                $($deserialize_bounds)*
            {
                fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
                    ShapeRef::Map {
                        key: Box::new(<$key as DeserializeShape>::deserialize_shape_in(context)),
                        value: Box::new(<$value as DeserializeShape>::deserialize_shape_in(context)),
                    }
                }
            }
        )+
    };
}

map_shape! {
    (K, V) BTreeMap<K, V>
    where
        serialize {
            K: SerializeShape,
            V: SerializeShape
        }
        deserialize {
            K: DeserializeShape + Ord,
            V: DeserializeShape
        }
    => (K, V);

}

#[cfg(feature = "std")]
map_shape! {
    (K, V, S) std::collections::HashMap<K, V, S>
    where
        serialize {
            K: SerializeShape,
            V: SerializeShape
        }
        deserialize {
            K: DeserializeShape + Eq + Hash,
            V: DeserializeShape,
            S: BuildHasher + Default
        }
    => (K, V);
}
