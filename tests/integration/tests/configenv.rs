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

#![allow(dead_code)]

use serde_shape::DeserializeDefinitionKind;
use serde_shape::DeserializeDefinitionShape;
use serde_shape::DeserializeShape;
use serde_shape::DeserializeShapeGraph;
use serde_shape::DeserializeVariantContent;
use serde_shape::FieldWireShape;
use serde_shape::FieldsStyle;
use serde_shape::ShapeRef;
use serde_shape::Tagging;

#[derive(DeserializeShape)]
struct ClientConfig {
    #[serde(flatten)]
    common: CommonConfig,
    transport: Transport,
}

#[derive(DeserializeShape)]
struct CommonConfig {
    retries: u8,
}

#[derive(DeserializeShape)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Transport {
    Tcp(TcpTransport),
}

#[derive(DeserializeShape)]
struct TcpTransport {
    host: String,
    port: u16,
    #[serde(rename = "tls.version")]
    tls_version: String,
}

#[test]
fn exposes_the_structure_needed_by_config_consumers() {
    let graph = ClientConfig::deserialize_shape();
    let root = graph
        .root_definition()
        .expect("config definition should exist");
    let DeserializeDefinitionKind::Struct(config) = &root.kind else {
        panic!("config should be a struct");
    };
    let [common, transport] = config.fields.as_slice() else {
        panic!("config should expose both fields");
    };

    assert!(matches!(common.wire_shape, FieldWireShape::Flatten(_)));
    let common = definition_for_wire_shape(&graph, &common.wire_shape);
    let DeserializeDefinitionKind::Struct(common) = &common.kind else {
        panic!("flattened config should be a struct");
    };
    assert_eq!(common.fields[0].name, "retries");

    let transport = definition_for_wire_shape(&graph, &transport.wire_shape);
    let DeserializeDefinitionKind::Enum(transport) = &transport.kind else {
        panic!("transport should be an enum");
    };
    assert_eq!(transport.repr, Tagging::Internal { tag: "kind" });

    let tcp = &transport.variants[0];
    assert_eq!(tcp.name, "tcp");
    assert_eq!(tcp.style, FieldsStyle::Newtype);
    let DeserializeVariantContent::Fields(fields) = &tcp.content else {
        panic!("newtype variant should expose fields");
    };
    let [payload] = fields.as_slice() else {
        panic!("newtype variant should expose its payload field");
    };

    let tcp = definition_for_wire_shape(&graph, &payload.wire_shape);
    let DeserializeDefinitionKind::Struct(tcp) = &tcp.kind else {
        panic!("TCP payload should be a struct");
    };
    assert_eq!(
        tcp.fields
            .iter()
            .map(|field| field.name)
            .collect::<Vec<_>>(),
        ["host", "port", "tls.version"]
    );
}

fn definition_for_wire_shape<'a>(
    graph: &'a DeserializeShapeGraph,
    wire_shape: &FieldWireShape,
) -> &'a DeserializeDefinitionShape {
    let shape = wire_shape.shape().expect("field should be present");
    let ShapeRef::Definition(id) = shape else {
        panic!("field should reference a named definition");
    };
    graph.definition(*id).expect("definition should exist")
}
