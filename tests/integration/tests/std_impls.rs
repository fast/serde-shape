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

use serde_shape::DeserializeShapeGraph;
use serde_shape::SerializeShapeGraph;
use serde_shape::ShapeRef;

#[test]
fn maps_common_std_shapes() {
    assert_eq!(
        SerializeShapeGraph::for_type::<std::path::Path>().root,
        ShapeRef::String
    );
    assert_eq!(
        DeserializeShapeGraph::for_type::<std::path::Path>().root,
        ShapeRef::String
    );
    assert_eq!(
        SerializeShapeGraph::for_type::<std::path::PathBuf>().root,
        ShapeRef::String
    );
    assert_eq!(
        DeserializeShapeGraph::for_type::<std::net::SocketAddr>().root,
        ShapeRef::String
    );
}
