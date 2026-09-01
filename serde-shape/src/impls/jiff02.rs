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

use jiff02::SignedDuration;
use jiff02::Span;
use jiff02::Timestamp;
use jiff02::Zoned;
use jiff02::civil::Date;
use jiff02::civil::DateTime;
use jiff02::civil::ISOWeekDate;
use jiff02::civil::Time;

use crate::DeserializeShape;
use crate::DeserializeShapeContext;
use crate::SerializeShape;
use crate::SerializeShapeContext;
use crate::ShapeRef;

macro_rules! string_shape {
    ($($ty:ty),* $(,)?) => {
        $(
            impl SerializeShape for $ty {
                fn serialize_shape_in(_context: &mut SerializeShapeContext) -> ShapeRef {
                    ShapeRef::String
                }
            }

            impl DeserializeShape for $ty {
                fn deserialize_shape_in(_context: &mut DeserializeShapeContext) -> ShapeRef {
                    ShapeRef::String
                }
            }
        )*
    };
}

string_shape!(
    Date,
    DateTime,
    ISOWeekDate,
    SignedDuration,
    Span,
    Time,
    Timestamp,
    Zoned,
);
