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

use alloc::vec;

use crate::DeserializeShape;
use crate::DeserializeShapeContext;
use crate::SerializeShape;
use crate::SerializeShapeContext;
use crate::ShapeRef;

macro_rules! tuple_shape {
    ($($($name:ident),+;)+) => {
        $(
            impl<$($name),+> SerializeShape for ($($name,)+)
            where
                $($name: SerializeShape,)+
            {
                fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
                    ShapeRef::Tuple(vec![$($name::serialize_shape_in(context),)+])
                }
            }

            impl<$($name),+> DeserializeShape for ($($name,)+)
            where
                $($name: DeserializeShape,)+
            {
                fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
                    ShapeRef::Tuple(vec![$($name::deserialize_shape_in(context),)+])
                }
            }
        )+
    };
}

tuple_shape! {
    T0;
    T0, T1;
    T0, T1, T2;
    T0, T1, T2, T3;
    T0, T1, T2, T3, T4;
    T0, T1, T2, T3, T4, T5;
    T0, T1, T2, T3, T4, T5, T6;
    T0, T1, T2, T3, T4, T5, T6, T7;
    T0, T1, T2, T3, T4, T5, T6, T7, T8;
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9;
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10;
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11;
}
