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

`serde-shape` reflects the data model that a Rust type emits through Serde serialization or accepts through Serde deserialization. It builds an inspectable graph from type information and `#[serde(...)]` attributes without serializing or deserializing a value.

Libraries and tools can inspect Rust and Serde names, fields, enum tagging, defaults, aliases, skipped values, format-dependent alternatives, and custom serializer or deserializer boundaries.

## Getting started

Enable the `derive` feature when you want `#[derive(SerializeShape)]` and `#[derive(DeserializeShape)]`:

```toml
[dependencies]
serde-shape = { version = "0.1.0", features = ["derive"] }
```

Enable `std` when your reflected types use shapes provided only by the Rust standard library:

```toml
[dependencies]
serde-shape = { version = "0.1.0", features = ["derive", "std"] }
```

The shape derives are independent of Serde's `Serialize` and `Deserialize` derives: they neither implement nor require those traits. Derive both sets when a type must also perform actual serialization or deserialization. Likewise, `serde_shape` attributes describe reflection metadata only and do not change Serde's runtime behavior.

## Choosing a direction

Serde permits a type to emit and accept different representations. Names, skipped fields, custom functions, and conversion types can all differ by direction, so `serde-shape` exposes two independent APIs:

| Representation                     | Derive             | Graph entry point        |
| ---------------------------------- | ------------------ | ------------------------ |
| Values emitted by serialization    | `SerializeShape`   | `T::serialize_shape()`   |
| Values accepted by deserialization | `DeserializeShape` | `T::deserialize_shape()` |

Derive only the direction a consumer needs. Derive both when a tool compares the emitted and accepted representations.

## When to use serde-shape

Use `serde-shape` when Serde already defines the contract you care about, but you also need to inspect that contract as data.

Typical use cases:

- generating configuration reference docs from config structs;
- deriving environment variable names and value kinds from nested config;
- documenting API or file-format shapes without handwritten schemas;
- checking how a serialized or deserialized shape changes across releases;
- building schema exporters that start from Serde metadata.

`serde-shape` is intentionally not a validation schema. It does not infer value ranges, regular expressions, business rules, or runtime behavior hidden inside custom serializer or deserializer functions.

Field shapes expose `wire_shape` as the source of truth for regular values, flattened fields, inline transparent fields, and omitted fields. Use `FieldWireShape::shape()` when a graph walker needs the contributed shape without distinguishing those wire positions. Custom serializer/deserializer boundaries are represented by `ShapeRef::Opaque`, including when they are flattened or inline.

If the consumer needs JSON Schema, [`schemars`](https://docs.rs/schemars) directly targets that format. `serde-shape` instead keeps serialization and deserialization shapes separate and leaves format-specific export and validation to downstream tools.

## Inspecting a graph

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

## Derive behavior

The derive macros use Serde's derive metadata for the selected direction. They reflect the following wire-relevant behavior:

| Scope                       | Reflected behavior                                                                                                                                                                              |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Container                   | Directional names, rename rules, enum tagging, transparent fields, defaults, unknown-field policy, expectation text, identifier enums, and `#[non_exhaustive]`                                  |
| Variant                     | Directional names and skips, deserialization aliases and `other`, per-variant `untagged`, field style, and custom-function boundaries                                                           |
| Field                       | Directional names and skips, deserialization aliases and defaults, `flatten`, `skip_serializing_if`, transparent placement, borrowed string or byte `Cow` input, and custom-function boundaries |
| Conversion and remote types | `into`, `from`, and `try_from` use the conversion type's shape; remote helpers expose their declared fields and Serde metadata                                                                  |

Serde attributes that affect generated code without changing the wire model are not copied into the graph. For example, Serde trait bounds and crate paths remain concerns of Serde's derive, while shape trait bounds are inferred independently.

### Custom representations

`serde_shape` has three direction-specific reflection extensions:

| Attribute                                       | Purpose                                                  | Allowed on                       |
| ----------------------------------------------- | -------------------------------------------------------- | -------------------------------- |
| `serialize_with = "path"`                       | Supplies a serialization `ShapeRef`                      | Containers, variants, and fields |
| `deserialize_with = "path"`                     | Supplies a deserialization `ShapeRef`                    | Containers, variants, and fields |
| `bound(serialize = "...", deserialize = "...")` | Replaces inferred shape bounds in the selected direction | Containers                       |

These extensions cannot rename, tag, skip, flatten, alias, or default a Serde item. Serde remains the source of truth for wire behavior; `serde_shape` only supplies reflection information that cannot be inferred.

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

Types declared with `#[serde(remote = "...")]` expose the wire shape described by the remote helper's fields and attributes. A field that uses the helper through `#[serde(with = "...")]` still needs a `serde_shape` hook that delegates to the helper, because an arbitrary `with` module does not identify its representation type.

## Model boundaries

Shape graphs are an inspection API, not a stable interchange format. `ShapeId` values are local to one graph and carry only a definition index, not graph identity. Keep an ID paired with the graph that produced it; a lookup cannot detect an ID from another graph when its index is in bounds. Definition ordering and `Debug` output are not persistence contracts.

Definitions may be recursive. A `ShapeRef::Definition` is a graph edge, so walkers must detect repeated `ShapeId` values instead of expanding definitions indefinitely.

Types that branch on `Serializer::is_human_readable()` or `Deserializer::is_human_readable()` may expose a union of their known representations. The graph describes the possible semantic shapes across formats; it is not specialized for one serializer format.

`ShapeRef` is a normalized semantic model, not a trace of exact `Serializer` or `Deserializer` method calls. For example, it preserves fixed arrays as `ShapeRef::Array` and pointer-width integers as `Isize` or `Usize`, even though Serde formats receive those values through tuple and fixed-width integer APIs. Use a recording serializer or deserializer when exact method dispatch is the contract being tested.

## Feature flags

`serde-shape` enables no features by default.

- `derive`: enables `#[derive(SerializeShape)]` and `#[derive(DeserializeShape)]`.
- `jiff02`: enables string shapes for Jiff 0.2 types with direct Serde implementations.
- `std`: enables shape implementations for standard-library-only types.

## Built-in shapes

The built-in implementations follow Serde's semantic representations in each direction, including known human-readable and compact alternatives.

| Group         | Supported types                                                                                                                                                                    |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Scalars       | Rust primitives, `String`, serialized `str` and `fmt::Arguments`, and non-zero integers                                                                                            |
| Containers    | `Option`, `Result`, arrays through length 32, slices for serialization, tuples through arity 16, `Vec`, `VecDeque`, `LinkedList`, `BinaryHeap`, `BTreeSet`, and `BTreeMap`         |
| Wrappers      | Serialized references, borrowed string/byte/path inputs, `Box`, `Rc`, `Arc`, their weak pointers, `Cow`, `Cell`, `RefCell`, `Wrapping`, `Saturating`, `Reverse`, and `PhantomData` |
| FFI           | `CStr` and `CString` byte representations; on Unix and Windows, serialized `OsStr`, `OsString`, and owned `Box<OsStr>` input                                                       |
| Ranges        | `Range`, `RangeFrom`, `RangeInclusive`, `RangeTo`, and `Bound`                                                                                                                     |
| Time          | `core::time::Duration` and, with `std`, `SystemTime`                                                                                                                               |
| Network       | `core::net` IP and socket address types                                                                                                                                            |
| `std` feature | Atomics available on the target, `HashMap`, `HashSet`, `Path`, `PathBuf`, `Mutex`, and `RwLock`                                                                                    |

With the `jiff02` feature, `Date`, `DateTime`, `ISOWeekDate`, `SignedDuration`, `Span`, `Time`, `Timestamp`, and `Zoned` are string shapes in both directions, matching Jiff's direct Serde implementations.

Network address shapes are unions of their human-readable string representation and their compact Serde representation. A serialized byte slice and an owned `Box<[u8]>` input are sequences, while borrowed byte deserialization uses `ShapeRef::Bytes`.

OS string shapes preserve Serde's target-specific externally tagged representation: `Unix` contains a byte sequence, while `Windows` contains a `u16` sequence. Deserialization advertises only the variant accepted on the current target.

Serde's `rc` feature is still required to serialize or deserialize `Rc`, `Arc`, and their weak pointers; the shape implementations do not enable Serde features.

Serialization follows Serde's blanket support for `&T` and `&mut T`. Deserialization only provides reference shapes for Serde's borrowable `&str`, `&[u8]`, and `&Path` inputs; arbitrary shared and mutable references do not have a Serde deserializer.

The unsized `str`, `[u8]`, and `Path` types themselves do not implement `DeserializeShape`, matching Serde. Their borrowed and owned input forms have explicit shape implementations.

For an unsupported foreign type, use a Serde remote definition, a local newtype, or a `serde_shape` custom hook. Custom Serde functions remain opaque by default because their wire behavior cannot be inferred.

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
