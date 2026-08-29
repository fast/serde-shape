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
[license-url]: https://www.apache.org/licenses/LICENSE-2.0
[actions-badge]: https://github.com/fast/serde-shape/workflows/CI/badge.svg
[actions-url]: https://github.com/fast/serde-shape/actions?query=workflow%3ACI

`serde-shape` builds inspectable graphs of Serde serialization and deserialization shapes. Derive macros generate the metadata code at compile time; calling `serialize_shape()` or `deserialize_shape()` constructs the graph at runtime without serializing or deserializing a value.

It gives libraries and tools a lightweight graph of the Rust types, Serde names, field metadata, enum tagging, defaults, aliases, union value alternatives, skips, and custom serializer/deserializer boundaries that make up a type's wire shape.

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

`serde-shape` is intentionally not a full validation schema. It reflects the Serde data model shape and relevant Serde attributes; it does not infer value ranges, regexes, business rules, or runtime behavior hidden inside custom serializer/deserializer functions. Use `ShapeRef::union` for format-native alternatives that do not fit one Rust shape. Union alternatives may overlap; they are flattened, deduplicated, and stored in canonical order.

Field shapes expose `wire_shape` as the source of truth for regular values, flattened fields, inline transparent fields, and omitted fields. Use `FieldWireShape::shape()` when a graph walker needs the contributed shape without distinguishing those wire positions. Custom serializer/deserializer boundaries are represented by `ShapeRef::Opaque`, including when they are flattened or inline.

If the consumer needs JSON Schema, [`schemars`](https://docs.rs/schemars) directly targets that format. `serde-shape` instead keeps serialization and deserialization shapes separate and leaves format-specific export and validation to downstream tools.

## Example

The following example shows how to inspect a nested config type.

```rust
use serde_shape::{DeserializeDefinitionKind, DeserializeShape, FieldsStyle};

#[derive(DeserializeShape)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
/// Application configuration.
struct Config {
    /// Port used by the HTTP server.
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
let definition = graph.root_definition().unwrap();

let DeserializeDefinitionKind::Struct(shape) = &definition.kind else {
    panic!("Config should produce a struct shape");
};

assert_eq!(definition.type_name.name, "Config");
assert_eq!(definition.description, Some("Application configuration."));
assert_eq!(shape.style, FieldsStyle::Struct);
assert!(shape.attributes.deny_unknown_fields);
assert_eq!(shape.fields[0].name, "http-port");
assert_eq!(shape.fields[0].description, Some("Port used by the HTTP server."));
assert_eq!(shape.fields[1].name, "peers");
assert_eq!(shape.fields[2].name, "tls");
```

See the [crate documentation][docs-url] for the full shape graph model, derive behavior, and manual implementation examples.

Rust doc comments on derived containers, variants, and fields are preserved as descriptions. Consumers can use the same comments for generated configuration references, CLI help, or diagnostics.

## Custom representations

Custom Serde functions and foreign types do not expose enough information for `serde-shape` to infer their wire representation. Provide functions that build the serialization and deserialization shapes explicitly:

```rust
use serde_shape::{
    DeserializeShape, DeserializeShapeContext, SerializeShape, SerializeShapeContext, ShapeRef,
};

struct ForeignDuration;

fn serialize_duration(context: &mut SerializeShapeContext) -> ShapeRef {
    String::serialize_shape_in(context)
}

fn deserialize_duration(_context: &mut DeserializeShapeContext) -> ShapeRef {
    ShapeRef::union([ShapeRef::String, ShapeRef::U64])
}

#[derive(SerializeShape, DeserializeShape)]
struct Config {
    #[serde(with = "duration_format")]
    #[serde_shape(
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    timeout: ForeignDuration,
}
```

Each function receives the current graph context and returns a `ShapeRef`. It may delegate to another type's shape implementation or construct a custom shape directly. A custom shape function is an assertion about the Serde behavior; `serde-shape` cannot verify that the declared shape matches the serializer or deserializer implementation.

The same `serde_shape` hooks can be placed on enum variants whose content is controlled by a variant-level Serde custom function. A variant hook describes the content inside the enum's tagging representation; the enum's `repr` still describes the tag. Without an explicit hook, custom variant content remains opaque.

For a generic custom hook, container-level `#[serde_shape(bound(serialize = "...", deserialize = "..."))]` replaces the automatically inferred bounds in the corresponding direction, following Serde's bound-override convention.

## Model boundaries

Shape graphs are an inspection API, not a stable interchange format. `ShapeId` values are local to one graph, and definition ordering and `Debug` output are not persistence contracts.

Definitions may be recursive. A `ShapeRef::Definition` is a graph edge, so walkers must detect repeated `ShapeId` values instead of expanding definitions indefinitely.

Types that branch on `Serializer::is_human_readable()` or `Deserializer::is_human_readable()` may expose a union of their known representations. The graph describes the possible semantic shapes across formats; it is not specialized for one serializer format.

`ShapeRef` is a normalized semantic model, not a trace of exact `Serializer` or `Deserializer` method calls. For example, it preserves fixed arrays as `ShapeRef::Array` and pointer-width integers as `Isize` or `Usize`, even though Serde formats receive those values through tuple and fixed-width integer APIs. Use a recording serializer or deserializer when exact method dispatch is the contract being tested.

## Feature flags

`serde-shape` enables no features by default.

- `derive`: enables `#[derive(SerializeShape)]` and `#[derive(DeserializeShape)]`.
- `std`: enables shape implementations for standard-library-only types.

## Built-in shapes

The built-in implementations follow Serde's semantic representations in each direction, including known human-readable and compact alternatives.

| Group | Supported types |
| --- | --- |
| Scalars | Rust primitives, `String`, `str`, non-zero integers, and atomics available on the target |
| Containers | `Option`, `Result`, arrays, slices for serialization, tuples through arity 16, `Vec`, `VecDeque`, `LinkedList`, `BinaryHeap`, `BTreeSet`, and `BTreeMap` |
| Wrappers | References, `Box`, `Rc`, `Arc`, their weak pointers, `Cow`, `Cell`, `RefCell`, `Wrapping`, `Saturating`, `Reverse`, and `PhantomData` |
| FFI | `CStr` and `CString` byte representations, including owned `Box<CStr>` input |
| Ranges | `Range`, `RangeFrom`, `RangeInclusive`, `RangeTo`, and `Bound` |
| Time | `core::time::Duration` and, with `std`, `SystemTime` |
| Network | `core::net` IP and socket address types |
| `std` feature | `HashMap`, `HashSet`, `Path`, `PathBuf`, `Mutex`, and `RwLock` |

Network address shapes are unions of their human-readable string representation and their compact Serde representation. A serialized byte slice and an owned `Box<[u8]>` input are sequences, while borrowed byte deserialization uses `ShapeRef::Bytes`.

Serde's `rc` feature is still required to serialize or deserialize `Rc`, `Arc`, and their weak pointers; the shape implementations do not enable Serde features.

For an unsupported foreign type, use a local newtype and implement `SerializeShape` or `DeserializeShape` manually. Custom Serde functions remain opaque by default because their wire behavior cannot be inferred; use a `serde_shape` custom hook when the representation is known.

## `no_std` support

`serde-shape` is `no_std` by default and requires `alloc`.

Enable the `std` feature explicitly when your shapes use standard-library-only types.

## Minimum Rust version policy

This crate's minimum supported `rustc` version is `1.85.0`.

The current policy is that the minimum Rust version required to use this crate can be increased in minor version updates. For example, if `crate 1.0` requires Rust 1.85.0, then `crate 1.0.z` for all values of `z` will also require Rust 1.85.0 or newer. However, `crate 1.y` for `y > 0` may require a newer minimum version of Rust.

## Contributing

See the [contributor guide](CONTRIBUTING.md) for the development workflow and test conventions.

## License

This project is licensed under the [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0).
