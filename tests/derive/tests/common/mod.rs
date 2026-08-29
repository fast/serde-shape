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

use renamed_shape::DeserializeDefinitionShape;
use renamed_shape::DeserializeShape;
use renamed_shape::SerializeDefinitionShape;
use renamed_shape::SerializeShape;

pub(super) fn deserialize_root_definition<T>() -> DeserializeDefinitionShape
where
    T: DeserializeShape,
{
    let graph = T::deserialize_shape();
    graph
        .root_definition()
        .expect("deserialization root definition should exist")
        .clone()
}

pub(super) fn serialize_root_definition<T>() -> SerializeDefinitionShape
where
    T: SerializeShape,
{
    let graph = T::serialize_shape();
    graph
        .root_definition()
        .expect("serialization root definition should exist")
        .clone()
}
