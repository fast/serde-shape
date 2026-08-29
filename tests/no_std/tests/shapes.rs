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

use serde_shape::DeserializeDefinitionKind;
use serde_shape::DeserializeShape;
use serde_shape::FieldWireShape;
use serde_shape::SerializeDefinitionKind;
use serde_shape::SerializeShape;
use serde_shape::ShapeRef;
use serde_shape_test_no_std::NoStdConfig;

#[test]
fn reflects_no_std_deserialization() {
    let graph = NoStdConfig::deserialize_shape();
    let ShapeRef::Definition(root_id) = graph.root() else {
        panic!("root shape should be a definition");
    };
    let root_id = *root_id;
    assert_eq!(root_id.index(), 0);
    assert_eq!(graph.definitions().len(), 2);

    let DeserializeDefinitionKind::Struct(shape) = &graph.definition(root_id).unwrap().kind else {
        panic!("root definition should be a struct");
    };
    assert_eq!(shape.fields.len(), 4);
    assert_eq!(
        shape.fields[0].wire_shape,
        FieldWireShape::Value(ShapeRef::String)
    );
    assert_eq!(
        shape.fields[1].wire_shape,
        FieldWireShape::Value(ShapeRef::Seq(Box::new(ShapeRef::Option(Box::new(
            ShapeRef::U16,
        )))))
    );
    assert_eq!(
        shape.fields[2].wire_shape,
        FieldWireShape::Value(ShapeRef::Option(Box::new(ShapeRef::Definition(root_id))))
    );
    assert!(matches!(
        shape.fields[3].wire_shape,
        FieldWireShape::Value(ShapeRef::Union(_))
    ));
}

#[test]
fn reflects_no_std_serialization() {
    let graph = NoStdConfig::serialize_shape();
    let ShapeRef::Definition(root_id) = graph.root() else {
        panic!("root shape should be a definition");
    };
    let root_id = *root_id;
    assert_eq!(graph.definitions().len(), 2);

    let SerializeDefinitionKind::Struct(shape) = &graph.definition(root_id).unwrap().kind else {
        panic!("root definition should be a struct");
    };
    assert_eq!(shape.fields.len(), 4);
    assert_eq!(
        shape.fields[0].wire_shape,
        FieldWireShape::Value(ShapeRef::String)
    );
    assert_eq!(
        shape.fields[1].wire_shape,
        FieldWireShape::Value(ShapeRef::Seq(Box::new(ShapeRef::Option(Box::new(
            ShapeRef::U16,
        )))))
    );
    assert_eq!(
        shape.fields[2].wire_shape,
        FieldWireShape::Value(ShapeRef::Option(Box::new(ShapeRef::Definition(root_id))))
    );
    assert!(matches!(
        shape.fields[3].wire_shape,
        FieldWireShape::Value(ShapeRef::Union(_))
    ));
}
