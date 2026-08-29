# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Breaking changes

* Make shape graph roots and definition lists read-only. Use `root()`, `definitions()`, and `definition(id)` instead of accessing fields directly.
* Make the `ShapeId` tuple field private. Use `ShapeId::index()` when the graph-local numeric index is needed.
* Add `description` to definition, field, and variant metadata. Manual struct literals must initialize the new field.
* Remove `OpaqueReason::{FromType, TryFromType, IntoType}` because Serde conversion attributes now use the conversion type's shape instead of an opaque boundary.

### New features

* Add `#[serde_shape(with = "Type")]`, `serialize_as`, and `deserialize_as` overrides for custom Serde functions and foreign representations.
* Preserve Rust doc comments on derived containers, variants, and fields as user-facing descriptions.

### Bug fixes

* Reflect the proxy type used by Serde `from`, `try_from`, and `into` container attributes.
* Follow Serde's directional bounds for `Cow`: serialization reflects the borrowed type and deserialization reflects the owned type.

### Improvements

* Document runtime graph construction, format-dependent unions, graph-local identifiers, built-in coverage, and custom representation boundaries.
* Replace broad debug snapshots with focused behavior assertions and remove the snapshot-testing dependency.
