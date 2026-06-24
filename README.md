# serde-shape

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![MSRV 1.85][msrv-badge]](https://www.whatrustisit.com)
[![Apache 2.0 licensed][license-badge]][license-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/serde-shape.svg
[crates-url]: https://crates.io/crates/serde-shape
[docs-badge]: https://img.shields.io/docsrs/serde-shape
[docs-url]: https://docs.rs/serde-shape
[msrv-badge]: https://img.shields.io/badge/MSRV-1.85-green?logo=rust
[license-badge]: https://img.shields.io/crates/l/serde-shape
[license-url]: LICENSE
[actions-badge]: https://github.com/fast/serde-shape/workflows/CI/badge.svg
[actions-url]: https://github.com/fast/serde-shape/actions?query=workflow%3ACI

`serde-shape` reflects the shape of Serde serialization and deserialization at compile time.

It gives libraries and tools a lightweight graph of the Rust types, Serde names, field metadata, enum tagging, defaults, aliases, union-like value alternatives, skips, and custom serializer/deserializer boundaries that make up a type's wire shape.

## Install

Enable the `derive` feature when you want `#[derive(SerializeShape)]` and `#[derive(DeserializeShape)]`:

```toml
[dependencies]
serde-shape = { version = "0.0.1", features = ["derive"] }
```

Enable `std` when your reflected types use shapes provided only by the Rust standard library:

```toml
[dependencies]
serde-shape = { version = "0.0.1", features = ["derive", "std"] }
```

## Motivation

Use `serde-shape` when Serde already defines the contract you care about, but you also need to inspect that contract as data.

Typical use cases:

- generating configuration reference docs from config structs;
- deriving environment variable names and value kinds from nested config;
- documenting API or file-format shapes without handwritten schemas;
- checking how a serialized or deserialized shape changes across releases;
- building schema exporters that start from Serde metadata.

`serde-shape` is intentionally not a full validation schema. It reflects the Serde data model shape and relevant Serde attributes; it does not infer value ranges, regexes, business rules, or runtime behavior hidden inside custom serializer/deserializer functions. Use `ShapeRef::OneOf` for format-native alternatives that do not fit one Rust shape.

Field shapes expose `wire_shape` as the source of truth for regular values, flattened fields, omitted fields, and custom serializer/deserializer boundaries.

You may use [`schemars`](https://docs.rs/schemars) for JSON Schema generation and validation. But `schemars` is not a general-purpose Serde shape reflection library, and it does not support all Serde attributes. `serde-shape` is designed to be a more complete and general-purpose reflection of Serde shapes.

## Example

The following example shows how to inspect a nested config type.

```rust
use serde_shape::{
    DeserializeDefinitionKind, DeserializeShape, FieldsStyle, ShapeRef,
};

#[derive(DeserializeShape)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct Config {
    http_port: u16,
    peers: Vec<String>,
    tls: Option<TlsConfig>,
}

#[derive(DeserializeShape)]
#[serde(rename_all = "kebab-case")]
struct TlsConfig {
    cert_path: String,
    key_path: String,
}

let graph = Config::deserialize_shape();
let ShapeRef::Definition(config_id) = graph.root else {
    panic!("Config should produce a named definition");
};
let definition = graph.definition(config_id).unwrap();

let DeserializeDefinitionKind::Struct(shape) = &definition.kind else {
    panic!("Config should produce a struct shape");
};

assert_eq!(definition.type_name.name, "Config");
assert_eq!(shape.style, FieldsStyle::Struct);
assert!(shape.attributes.deny_unknown_fields);
assert_eq!(shape.fields[0].name, "http-port");
assert_eq!(shape.fields[1].name, "peers");
assert_eq!(shape.fields[2].name, "tls");
```

See the [crate documentation][docs-url] for the full shape graph model, derive behavior, and manual implementation examples.

## Feature flags

`serde-shape` enables no features by default.

- `derive`: enables `#[derive(SerializeShape)]` and `#[derive(DeserializeShape)]`.
- `std`: enables shape implementations for standard-library-only types.

## `no_std` support

`serde-shape` is `no_std` by default and requires `alloc`.

Enable the `std` feature explicitly when your shapes use standard-library-only types.

## Minimum Rust version policy

This crate's minimum supported `rustc` version is `1.85.0`.

The current policy is that the minimum Rust version required to use this crate can be increased in minor version updates. For example, if `crate 1.0` requires Rust 1.85.0, then `crate 1.0.z` for all values of `z` will also require Rust 1.85.0 or newer. However, `crate 1.y` for `y > 0` may require a newer minimum version of Rust.

## License

This project is licensed under [Apache License, Version 2.0](LICENSE).
