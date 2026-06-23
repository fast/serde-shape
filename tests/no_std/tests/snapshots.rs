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

use serde_shape::DeserializeShape;
use serde_shape::SerializeShape;
use serde_shape_test_no_std::NoStdConfig;

#[test]
fn snapshots_no_std_config_deserialize_shape() {
    insta::assert_debug_snapshot!(NoStdConfig::deserialize_shape());
}

#[test]
fn snapshots_no_std_config_serialize_shape() {
    insta::assert_debug_snapshot!(NoStdConfig::serialize_shape());
}
