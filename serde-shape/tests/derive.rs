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

#![cfg(feature = "derive")]
#![allow(dead_code)]

use serde_shape::SerdeShape;

#[derive(SerdeShape)]
#[serde(
    rename_all = "kebab-case",
    deny_unknown_fields,
    default,
    expecting = "config object"
)]
struct Config {
    http_port: u16,
    #[serde(alias = "endpoint")]
    api_url: Option<String>,
    #[serde(default = "default_retries")]
    retries: u8,
    #[serde(flatten)]
    storage: Storage,
    #[serde(skip)]
    skipped: NotShape,
    #[serde(deserialize_with = "custom_secret")]
    secret: NotShape,
}

#[derive(SerdeShape)]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case"
)]
#[non_exhaustive]
enum Storage {
    #[serde(alias = "s3-compatible")]
    S3 {
        bucket_name: String,
    },
    AzBlob {
        container_name: String,
    },
    #[serde(other)]
    Other,
}

#[derive(SerdeShape)]
#[serde(transparent)]
struct UserId(u64);

#[derive(SerdeShape)]
#[serde(from = "String")]
struct FromString(String);

#[derive(SerdeShape)]
struct SkipsGeneric<T> {
    #[serde(skip)]
    value: T,
}

#[derive(SerdeShape)]
struct Marker<T> {
    marker: std::marker::PhantomData<T>,
}

#[derive(SerdeShape)]
struct Recursive {
    child: Option<Box<Recursive>>,
}

struct NotShape;

fn default_retries() -> u8 {
    3
}

#[test]
fn snapshots_struct_shape_from_container_and_field_attrs() {
    insta::assert_debug_snapshot!(Config::shape());
}

#[test]
fn snapshots_internally_tagged_enum_shape_from_variant_attrs() {
    insta::assert_debug_snapshot!(Storage::shape());
}

#[test]
fn snapshots_transparent_struct_shape() {
    insta::assert_debug_snapshot!(UserId::shape());
}

#[test]
fn snapshots_conversion_based_opaque_shape() {
    insta::assert_debug_snapshot!(FromString::shape());
}

#[test]
fn snapshots_skipped_generic_field_without_shape_bound() {
    insta::assert_debug_snapshot!(SkipsGeneric::<NotShape>::shape());
}

#[test]
fn snapshots_phantom_data_generic_field_without_shape_bound() {
    insta::assert_debug_snapshot!(Marker::<NotShape>::shape());
}

#[test]
fn snapshots_recursive_type_reusing_the_same_definition() {
    insta::assert_debug_snapshot!(Recursive::shape());
}
