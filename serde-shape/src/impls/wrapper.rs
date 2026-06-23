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

macro_rules! transparent_shape {
    (
        $(
            ($($generics:tt)*) $ty:ty
            where
                serialize { $($serialize_bounds:tt)* }
                deserialize { $($deserialize_bounds:tt)* }
            => $inner:ty;
        )+
    ) => {
        $(
            impl<$($generics)*> SerializeShape for $ty
            where
                $($serialize_bounds)*
            {
                fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
                    <$inner as SerializeShape>::serialize_shape_in(context)
                }
            }

            impl<$($generics)*> DeserializeShape for $ty
            where
                $($deserialize_bounds)*
            {
                fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
                    <$inner as DeserializeShape>::deserialize_shape_in(context)
                }
            }
        )+
    };
}

transparent_shape! {
    (T) &T
    where
        serialize { T: SerializeShape + ?Sized }
        deserialize { T: DeserializeShape + ?Sized }
    => T;

    (T) &mut T
    where
        serialize { T: SerializeShape + ?Sized }
        deserialize { T: DeserializeShape + ?Sized }
    => T;

    (T) Box<T>
    where
        serialize { T: SerializeShape + ?Sized }
        deserialize { T: DeserializeShape + ?Sized }
    => T;

    ('a, T) std::borrow::Cow<'a, T>
    where
        serialize {
            T: std::borrow::ToOwned + ?Sized,
            <T as std::borrow::ToOwned>::Owned: SerializeShape
        }
        deserialize {
            T: std::borrow::ToOwned + ?Sized,
            <T as std::borrow::ToOwned>::Owned: DeserializeShape
        }
    => <T as std::borrow::ToOwned>::Owned;

    (T) std::cell::Cell<T>
    where
        serialize { T: Copy + SerializeShape }
        deserialize { T: Copy + DeserializeShape }
    => T;

    (T) std::cell::RefCell<T>
    where
        serialize { T: SerializeShape }
        deserialize { T: DeserializeShape }
    => T;

    (T) std::sync::Mutex<T>
    where
        serialize { T: SerializeShape }
        deserialize { T: DeserializeShape }
    => T;

    (T) std::sync::RwLock<T>
    where
        serialize { T: SerializeShape }
        deserialize { T: DeserializeShape }
    => T;

    (T) std::num::Wrapping<T>
    where
        serialize { T: SerializeShape }
        deserialize { T: DeserializeShape }
    => T;

    (T) std::cmp::Reverse<T>
    where
        serialize { T: SerializeShape }
        deserialize { T: DeserializeShape }
    => T;
}

impl<T> SerializeShape for std::marker::PhantomData<T> {
    fn serialize_shape_in(_context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Unit
    }
}

impl<T> DeserializeShape for std::marker::PhantomData<T> {
    fn deserialize_shape_in(_context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Unit
    }
}
