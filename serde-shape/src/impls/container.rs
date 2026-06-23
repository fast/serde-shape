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

    (T) std::collections::VecDeque<T>
    where
        serialize { T: SerializeShape }
        deserialize { T: DeserializeShape }
    => T;

    (T) std::collections::LinkedList<T>
    where
        serialize { T: SerializeShape }
        deserialize { T: DeserializeShape }
    => T;

    (T) std::collections::BinaryHeap<T>
    where
        serialize { T: Ord + SerializeShape }
        deserialize { T: Ord + DeserializeShape }
    => T;

    (T) std::collections::BTreeSet<T>
    where
        serialize { T: SerializeShape }
        deserialize { T: DeserializeShape }
    => T;

    (T, S) std::collections::HashSet<T, S>
    where
        serialize { T: SerializeShape }
        deserialize { T: DeserializeShape }
    => T;
}

impl<T, const N: usize> SerializeShape for [T; N]
where
    T: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Array {
            item: Box::new(T::serialize_shape_in(context)),
            len: N,
        }
    }
}

impl<T, const N: usize> DeserializeShape for [T; N]
where
    T: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Array {
            item: Box::new(T::deserialize_shape_in(context)),
            len: N,
        }
    }
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
    (K, V) std::collections::BTreeMap<K, V>
    where
        serialize {
            K: SerializeShape,
            V: SerializeShape
        }
        deserialize {
            K: DeserializeShape,
            V: DeserializeShape
        }
    => (K, V);

    (K, V, S) std::collections::HashMap<K, V, S>
    where
        serialize {
            K: SerializeShape,
            V: SerializeShape
        }
        deserialize {
            K: DeserializeShape,
            V: DeserializeShape
        }
    => (K, V);
}
