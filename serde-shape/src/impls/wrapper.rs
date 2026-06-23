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
use core::cell::Cell;
use core::cell::RefCell;
use core::cmp::Reverse;
use core::marker::PhantomData;
use core::num::Wrapping;

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

    ('a, T) Cow<'a, T>
    where
        serialize {
            T: ToOwned + ?Sized,
            <T as ToOwned>::Owned: SerializeShape
        }
        deserialize {
            T: ToOwned + ?Sized,
            <T as ToOwned>::Owned: DeserializeShape
        }
    => <T as ToOwned>::Owned;

    (T) Cell<T>
    where
        serialize { T: Copy + SerializeShape }
        deserialize { T: Copy + DeserializeShape }
    => T;

    (T) RefCell<T>
    where
        serialize { T: SerializeShape }
        deserialize { T: DeserializeShape }
    => T;

    (T) Wrapping<T>
    where
        serialize { T: SerializeShape }
        deserialize { T: DeserializeShape }
    => T;

    (T) Reverse<T>
    where
        serialize { T: SerializeShape }
        deserialize { T: DeserializeShape }
    => T;
}

#[cfg(feature = "std")]
transparent_shape! {
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
}

impl<T> SerializeShape for PhantomData<T> {
    fn serialize_shape_in(_context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Unit
    }
}

impl<T> DeserializeShape for PhantomData<T> {
    fn deserialize_shape_in(_context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Unit
    }
}
