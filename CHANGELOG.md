# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Breaking changes

* Represent Serde field and variant identifier enums with dedicated `Tagging` variants instead of incorrectly describing their input as externally tagged enums.
* Gate atomic shape implementations behind the `std` feature, matching Serde's own atomic implementations instead of advertising them in `no_std` builds where Serde cannot use them.
* Remove `DeserializeShape` from the unsized `str`, `[u8]`, and `Path` types, which do not implement Serde `Deserialize`. Their supported borrowed and owned forms retain explicit shapes.
* Replace the identical `SerializeTypeName` and `DeserializeTypeName` structures with one direction-neutral `TypeName`. Manual named definitions can use `TypeName::of::<T>(serde_name)` instead of repeating `core::any::type_name::<T>()`.
* Remove the redundant `transparent` field from container attributes. Transparent containers remain observable through their field's `FieldWireShape::Inline` position.
* Remove the blanket `DeserializeShape` implementations for `&T` and `&mut T`, which claimed support that Serde does not provide. Borrowed `&str`, `&[u8]`, and `&Path` inputs retain explicit implementations; custom borrowed types can now provide their own local implementation.
* Remove the redundant `tagging` and `has_flatten` fields from container attributes. Read enum tagging from `SerializeEnumShape::repr` or `DeserializeEnumShape::repr`, and identify flattened fields through `FieldWireShape::Flatten`.
* Make shape graph roots and definition lists read-only. Use `root()`, `definitions()`, and `definition(id)` instead of accessing fields directly.
* Make the `ShapeId` tuple field private. Use `ShapeId::index()` when the graph-local numeric index is needed.
* Add `description` to definition, field, and variant metadata. Manual struct literals must initialize the new field.
* Remove `OpaqueReason::{FromType, TryFromType, IntoType}` because Serde conversion attributes now use the conversion type's shape instead of an opaque boundary.
* Restrict the blanket `Box<T>` deserialization shape to sized `T`. Serde-supported owned DSTs have explicit implementations; shape-only custom DSTs now need a local newtype.

### New features

* Reflect serialized `core::fmt::Arguments` as a string, matching Serde's formatting implementation.
* Add target-specific `OsStr` and `OsString` enum shapes on Unix and Windows, including owned `Box<OsStr>` input.
* Add `SerializeShapeGraph::from_fn` and `DeserializeShapeGraph::from_fn` so custom shape functions can describe foreign graph roots without a dummy wrapper type.
* Add `#[serde_shape(serialize_with = "path", deserialize_with = "path")]` hooks for custom Serde functions and foreign representations.
* Allow custom shape hooks on enum variants so known custom variant content does not have to remain opaque.
* Reflect Serde's byte-buffer representation for `CStr`, `CString`, and owned `Box<CStr>` input.
* Add `Rc`, `Arc`, and weak-pointer shapes, preserving Serde's owned input and optional weak-pointer representations.
* Reflect `Saturating<T>` with Serde's generic serialization and primitive-only deserialization support.
* Add named struct shapes for Serde's `Range`, `RangeFrom`, `RangeInclusive`, and `RangeTo` representations.
* Reflect `Bound<T>` as Serde's externally tagged `Unbounded`, `Included`, and `Excluded` enum.
* Add the `std`-only `SystemTime` struct shape with Serde's epoch field names.
* Add directional `#[serde_shape(bound(...))]` overrides for generic custom shape hooks.
* Add `SerializeShapeGraph::root_definition` and `DeserializeShapeGraph::root_definition` for directly inspecting named root types.
* Preserve Rust doc comments on derived containers, variants, and fields as user-facing descriptions.

### Bug fixes

* Recognize only Serde's private borrowing helpers when recovering `Cow<str>` and `Cow<[u8]>` shapes, leaving similarly named user deserializers opaque.
* Match Serde's deserialization bounds for tree and hash collections so a shape implementation is exposed only when the corresponding collection can actually deserialize.
* Preserve the known string and byte shapes of `#[serde(borrow)]` fields using `Cow<str>` or `Cow<[u8]>` instead of treating Serde's generated borrowing helpers as custom opaque deserializers.
* Distinguish borrowed byte input from owned boxed slices: `&[u8]` reflects bytes while `Box<[u8]>` reflects a sequence, matching Serde.
* Match Serde's serialization bounds for `BinaryHeap`, `RefCell`, `Mutex`, and `RwLock`, including unsized wrapper contents.
* Reflect the proxy type used by Serde `from`, `try_from`, and `into` container attributes.
* Follow Serde's directional bounds for `Cow`: serialization reflects the borrowed type and deserialization reflects the owned type.
* Make IP and socket address shapes available in `no_std` builds through `core::net`.
* Preserve qualified Serde default paths without token-rendering spaces.

### Improvements

* Add `definition_for` to both graph types so walkers can resolve a `ShapeRef::Definition` without repeating a match and id lookup.
* Verify the packaged main crate against the packaged derive implementation that will be released with it, rather than accidentally compiling the previously published same-version macro crate from crates.io.
* Clarify that shape graphs are normalized semantic models rather than exact traces of Serde serializer or deserializer method dispatch.
* Add `FieldWireShape::shape()` so graph walkers can follow any present field without repeating a match over every wire position.
* Replace the integration test's embedded TOML/environment editor with a focused consumer-boundary test of the shape metadata. Configuration policy remains downstream rather than becoming a second implementation maintained by this crate.
* Align the derive macro with Serde's current `syn` 3 parser stack so applications deriving both Serde and shape metadata do not compile two `syn` major versions.
* Verify both publishable crate archives from their normalized manifests in CI.
* Implement `Default` for container attributes, `Tagging`, and `DefaultShape` so manual shape implementations can initialize ordinary Serde metadata concisely.
* Allow unsized types such as `str` and slices to use the `SerializeShape::serialize_shape` and `DeserializeShape::deserialize_shape` convenience methods directly.
* Document runtime graph construction, format-dependent unions, graph-local identifiers, built-in coverage, and custom representation boundaries.
* Replace broad debug snapshots with focused behavior assertions and remove the snapshot-testing dependency.
