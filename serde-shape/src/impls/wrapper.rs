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
use alloc::rc::Rc;
use alloc::rc::Weak as RcWeak;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Weak as ArcWeak;
use core::cell::Cell;
use core::cell::RefCell;
use core::cmp::Reverse;
use core::marker::PhantomData;
use core::num::Saturating;
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
    (T) Box<T>
    where
        serialize { T: SerializeShape + ?Sized }
        deserialize { T: DeserializeShape }
    => T;

    (T) Cell<T>
    where
        serialize { T: Copy + SerializeShape }
        deserialize { T: Copy + DeserializeShape }
    => T;

    (T) RefCell<T>
    where
        serialize { T: SerializeShape + ?Sized }
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

impl<T> SerializeShape for &T
where
    T: SerializeShape + ?Sized,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        T::serialize_shape_in(context)
    }
}

impl<T> SerializeShape for &mut T
where
    T: SerializeShape + ?Sized,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        T::serialize_shape_in(context)
    }
}

impl DeserializeShape for &str {
    fn deserialize_shape_in(_context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::String
    }
}

impl DeserializeShape for &[u8] {
    fn deserialize_shape_in(_context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Bytes
    }
}

impl<T> SerializeShape for Saturating<T>
where
    T: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        T::serialize_shape_in(context)
    }
}

macro_rules! saturating_deserialize_shape {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl DeserializeShape for Saturating<$ty> {
                fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
                    <$ty as DeserializeShape>::deserialize_shape_in(context)
                }
            }
        )+
    };
}

saturating_deserialize_shape!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize,
);

impl<T> DeserializeShape for Box<[T]>
where
    T: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Seq(Box::new(T::deserialize_shape_in(context)))
    }
}

impl DeserializeShape for Box<str> {
    fn deserialize_shape_in(_context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::String
    }
}

macro_rules! shared_pointer_shape {
    ($strong:ident, $weak:ident) => {
        impl<T> SerializeShape for $strong<T>
        where
            T: SerializeShape + ?Sized,
        {
            fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
                T::serialize_shape_in(context)
            }
        }

        impl<T> DeserializeShape for $strong<T>
        where
            T: ?Sized,
            Box<T>: DeserializeShape,
        {
            fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
                <Box<T> as DeserializeShape>::deserialize_shape_in(context)
            }
        }

        impl<T> SerializeShape for $weak<T>
        where
            T: SerializeShape + ?Sized,
        {
            fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
                ShapeRef::Option(Box::new(T::serialize_shape_in(context)))
            }
        }

        impl<T> DeserializeShape for $weak<T>
        where
            T: DeserializeShape,
        {
            fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
                ShapeRef::Option(Box::new(T::deserialize_shape_in(context)))
            }
        }
    };
}

shared_pointer_shape!(Rc, RcWeak);

#[cfg(target_has_atomic = "ptr")]
shared_pointer_shape!(Arc, ArcWeak);

#[cfg(feature = "std")]
impl DeserializeShape for Box<std::path::Path> {
    fn deserialize_shape_in(_context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::String
    }
}

#[cfg(feature = "std")]
impl DeserializeShape for &std::path::Path {
    fn deserialize_shape_in(_context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::String
    }
}

#[cfg(feature = "std")]
transparent_shape! {
    (T) std::sync::Mutex<T>
    where
        serialize { T: SerializeShape + ?Sized }
        deserialize { T: DeserializeShape }
    => T;

    (T) std::sync::RwLock<T>
    where
        serialize { T: SerializeShape + ?Sized }
        deserialize { T: DeserializeShape }
    => T;
}

impl<T> SerializeShape for Cow<'_, T>
where
    T: ToOwned + SerializeShape + ?Sized,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        T::serialize_shape_in(context)
    }
}

impl<T> DeserializeShape for Cow<'_, T>
where
    T: ToOwned + ?Sized,
    T::Owned: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        T::Owned::deserialize_shape_in(context)
    }
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
