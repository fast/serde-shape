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

//! Reflect the shapes used by Serde serialization and deserialization.
//!
//! `serde-shape` builds a lightweight graph that describes what a Rust type emits through Serde
//! serialization and accepts through Serde deserialization. It does not run Serde, and it is not a
//! full validation schema. Instead, it gives tools access to the same structural information that
//! Serde derives from Rust types and `#[serde(...)]` attributes, including union value shapes.
//!
//! Common uses are generating configuration reference docs, deriving environment-variable maps
//! from config structs, documenting wire formats, and checking whether two versions of a type
//! expose compatible Serde shapes.
//!
//! # Install
//!
//! Enable the `derive` feature when you want `#[derive(SerializeShape)]` and
//! `#[derive(DeserializeShape)]`:
//!
//! ```toml
//! [dependencies]
//! serde-shape = { version = "0.1.0", features = ["derive"] }
//! ```
//!
//! Enable `std` when the reflected types use shapes provided only by the Rust standard library:
//!
//! ```toml
//! [dependencies]
//! serde-shape = { version = "0.1.0", features = ["derive", "std"] }
//! ```
//!
//! The crate is `no_std` by default and requires `alloc`.
//!
//! # Quick start
//!
//! Derive [`trait@DeserializeShape`] for the type you want to inspect, then build a
//! [`DeserializeShapeGraph`]:
//!
//! ```rust
//! # #[cfg(feature = "derive")]
//! # {
//! use serde_shape::DeserializeDefinitionKind;
//! use serde_shape::DeserializeShape;
//! use serde_shape::FieldsStyle;
//!
//! #[derive(DeserializeShape)]
//! #[serde(rename_all = "kebab-case", deny_unknown_fields)]
//! struct Config {
//!     http_port: u16,
//!     peers: Vec<String>,
//!     tls: Option<TlsConfig>,
//! }
//!
//! #[derive(DeserializeShape)]
//! #[serde(rename_all = "kebab-case")]
//! struct TlsConfig {
//!     cert_path: String,
//!     key_path: String,
//! }
//!
//! let graph = Config::deserialize_shape();
//! let config = graph.root_definition().unwrap();
//!
//! let DeserializeDefinitionKind::Struct(shape) = &config.kind else {
//!     panic!("Config should produce a struct shape");
//! };
//!
//! assert_eq!(config.type_name.name, "Config");
//! assert_eq!(shape.style, FieldsStyle::Struct);
//! assert!(shape.attributes.deny_unknown_fields);
//! assert_eq!(shape.fields[0].name, "http-port");
//! assert_eq!(shape.fields[1].name, "peers");
//! assert_eq!(shape.fields[2].name, "tls");
//! # }
//! ```
//!
//! Serialization and deserialization are reflected separately because Serde lets the two
//! directions differ:
//!
//! ```rust
//! # #[cfg(feature = "derive")]
//! # {
//! use serde_shape::DeserializeDefinitionKind;
//! use serde_shape::DeserializeShape;
//! use serde_shape::SerializeDefinitionKind;
//! use serde_shape::SerializeShape;
//!
//! #[derive(SerializeShape, DeserializeShape)]
//! #[serde(rename(serialize = "wire-output", deserialize = "wire-input"))]
//! struct Message {
//!     #[serde(rename(serialize = "out-id", deserialize = "in-id"))]
//!     id: u64,
//! }
//!
//! let serialize_graph = Message::serialize_shape();
//! let deserialize_graph = Message::deserialize_shape();
//! let serialize_definition = serialize_graph.root_definition().unwrap();
//! let deserialize_definition = deserialize_graph.root_definition().unwrap();
//!
//! assert_eq!(serialize_definition.type_name.name, "wire-output");
//! assert_eq!(deserialize_definition.type_name.name, "wire-input");
//!
//! let SerializeDefinitionKind::Struct(serialize_shape) = &serialize_definition.kind else {
//!     panic!("Message should produce a struct serialization shape");
//! };
//! let DeserializeDefinitionKind::Struct(deserialize_shape) = &deserialize_definition.kind else {
//!     panic!("Message should produce a struct deserialization shape");
//! };
//!
//! assert_eq!(serialize_shape.fields[0].name, "out-id");
//! assert_eq!(deserialize_shape.fields[0].name, "in-id");
//! # }
//! ```
//!
//! # Shape graphs
//!
//! A shape graph has a [`ShapeRef`] root and a list of named definitions. Flat primitive and
//! compound values are represented directly as [`ShapeRef`] values. Structs and enums are
//! stored as named definitions and referenced by [`ShapeId`].
//!
//! Definition IDs are local to one graph. They contain an index but no graph identity, so callers
//! must keep each ID paired with the graph that produced it. Use
//! [`SerializeShapeGraph::definition`] or [`DeserializeShapeGraph::definition`] to resolve them.
//! Definition ordering and debug output are not stable persistence formats.
//! Definitions may be recursive, so graph walkers must detect repeated [`ShapeId`] values before
//! following definition references.
//!
//! Types that branch on Serde's human-readable mode may expose a union of their known
//! representations. Shape graphs describe possible semantic shapes across formats rather than
//! specializing themselves for one serializer.
//!
//! [`ShapeRef`] is not a trace of exact serializer or deserializer method calls. It deliberately
//! preserves useful Rust distinctions such as fixed arrays and pointer-width integers even when
//! Serde dispatches them through tuple or fixed-width integer methods.
//!
//! # Derive behavior
//!
//! The derive macros read Serde container, variant, and field attributes, so the resulting shape
//! follows the metadata Serde derives for each direction.
//!
//! A custom serializer or deserializer has no inferable inner shape, so the affected field or
//! variant content is represented by an opaque boundary. Whole-container conversion attributes
//! use the conversion type's shape. Serde remote derives expose the helper definition's declared
//! wire shape.
//! Field-level [`FieldWireShape`] distinguishes ordinary values from flattened fields, inline
//! transparent fields, and omitted fields. Custom serializer/deserializer boundaries use
//! [`ShapeRef::Opaque`] and remain composable with those field positions.
//!
//! Use `#[serde_shape(serialize_with = "path")]` or
//! `#[serde_shape(deserialize_with = "path")]` to declare the representation of a container,
//! variant, or field that cannot be inferred. Each function receives the current graph context and
//! returns a [`ShapeRef`], so it can delegate to another type or build a custom shape directly.
//!
//! Rust doc comments on derived containers, variants, and fields are preserved in their
//! `description` fields for documentation and diagnostic consumers.
//!
//! # Manual implementations
//!
//! Implement [`trait@SerializeShape`] or [`trait@DeserializeShape`] manually when a type's Serde
//! representation is known but cannot be derived. This is common for wrappers that deserialize
//! from a string or another primitive representation:
//!
//! ```rust
//! use serde_shape::DeserializeShape;
//! use serde_shape::DeserializeShapeContext;
//! use serde_shape::ShapeRef;
//!
//! struct ByteSize(u64);
//!
//! impl DeserializeShape for ByteSize {
//!     fn deserialize_shape_in(_context: &mut DeserializeShapeContext) -> ShapeRef {
//!         ShapeRef::union([ShapeRef::String, ShapeRef::U64])
//!     }
//! }
//!
//! assert_eq!(
//!     ByteSize::deserialize_shape().root(),
//!     &ShapeRef::union([ShapeRef::String, ShapeRef::U64])
//! );
//! ```
//!
//! For recursive or shared named types, use [`SerializeShapeContext::define_named_type`] or
//! [`DeserializeShapeContext::define_named_type`] so the graph contains one definition and
//! all recursive edges point back to it.

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

extern crate alloc;
extern crate self as serde_shape;
#[cfg(feature = "std")]
extern crate std;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::any::TypeId;
use core::fmt;

/// Private exports used by generated derive code.
#[doc(hidden)]
#[allow(missing_docs)]
pub mod __private {
    pub use alloc::vec;
}

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
/// Derive [`trait@DeserializeShape`] from Serde deserialization metadata.
///
/// Use this macro when a type's accepted input shape should be reflected from the same
/// metadata that Serde uses for deserialization. The generated implementation records the
/// deserialization-side names, shape graph, and Serde field/container metadata.
///
/// Use `#[serde_shape(deserialize_with = "path")]` on a container, variant, or field to
/// override an opaque or foreign representation. The function must accept `&mut
/// DeserializeShapeContext` and return a [`ShapeRef`]. Generic hooks can replace inferred
/// bounds with `#[serde_shape(bound(deserialize = "T: DeserializeShape"))]` on the container.
///
/// # Example
///
/// ```rust
/// use serde_shape::DefaultShape;
/// use serde_shape::DeserializeDefinitionKind;
/// use serde_shape::DeserializeShape;
///
/// #[derive(DeserializeShape)]
/// #[serde(rename_all = "kebab-case")]
/// struct Config {
///     listen_addr: String,
///     #[serde(default)]
///     worker_count: u16,
/// }
///
/// let graph = Config::deserialize_shape();
/// let definition = graph.root_definition().unwrap();
///
/// let DeserializeDefinitionKind::Struct(shape) = &definition.kind else {
///     panic!("Config should produce a struct shape");
/// };
///
/// assert_eq!(shape.fields[0].name, "listen-addr");
/// assert_eq!(shape.fields[1].name, "worker-count");
/// assert_eq!(shape.fields[1].default, DefaultShape::Default);
/// ```
pub use serde_shape_derive::DeserializeShape;
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
/// Derive [`trait@SerializeShape`] from Serde serialization metadata.
///
/// Use this macro when a type's emitted output shape should be reflected from the same
/// metadata that Serde uses for serialization. The generated implementation records the
/// serialization-side names, shape graph, and Serde field/container metadata.
///
/// Use `#[serde_shape(serialize_with = "path")]` on a container, variant, or field to override
/// an opaque or foreign representation. The function must accept `&mut SerializeShapeContext`
/// and return a [`ShapeRef`]. Generic hooks can replace inferred bounds with
/// `#[serde_shape(bound(serialize = "T: SerializeShape"))]` on the container.
///
/// # Example
///
/// ```rust
/// use serde_shape::SerializeDefinitionKind;
/// use serde_shape::SerializeShape;
///
/// #[derive(SerializeShape)]
/// #[serde(rename = "api-response", rename_all = "camelCase")]
/// struct Response {
///     request_id: u64,
///     #[serde(skip_serializing_if = "Option::is_none")]
///     next_page: Option<String>,
/// }
///
/// let graph = Response::serialize_shape();
/// let definition = graph.root_definition().unwrap();
///
/// let SerializeDefinitionKind::Struct(shape) = &definition.kind else {
///     panic!("Response should produce a struct shape");
/// };
///
/// assert_eq!(definition.type_name.name, "api-response");
/// assert_eq!(shape.fields[0].name, "requestId");
/// assert_eq!(shape.fields[1].name, "nextPage");
/// assert!(shape.fields[1].skip_if.is_some());
/// ```
pub use serde_shape_derive::SerializeShape;

mod impls;
#[cfg(test)]
mod tests;

/// A type that can describe the shape emitted by its Serde serializer.
pub trait SerializeShape {
    /// Build this type's serialization shape inside the provided context.
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef;

    /// Build a complete serialization shape graph rooted at this type.
    fn serialize_shape() -> SerializeShapeGraph {
        SerializeShapeGraph::for_type::<Self>()
    }
}

/// A type that can describe the shape accepted by its Serde deserializer.
pub trait DeserializeShape {
    /// Build this type's deserialization shape inside the provided context.
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef;

    /// Build a complete deserialization shape graph rooted at this type.
    fn deserialize_shape() -> DeserializeShapeGraph {
        DeserializeShapeGraph::for_type::<Self>()
    }
}

/// A complete serialization shape graph rooted at one type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SerializeShapeGraph {
    /// The root shape reference.
    root: ShapeRef,
    /// Named type definitions reachable from the root.
    definitions: Vec<SerializeDefinitionShape>,
}

impl SerializeShapeGraph {
    /// Build a serialization graph from a function that returns its root shape.
    ///
    /// This is useful when the root type is foreign or when no Rust type corresponds to the
    /// complete wire shape. Use [`Self::for_type`] when the root implements [`SerializeShape`].
    pub fn from_fn<F>(build_root: F) -> Self
    where
        F: FnOnce(&mut SerializeShapeContext) -> ShapeRef,
    {
        let mut context = SerializeShapeContext::default();
        let root = build_root(&mut context);
        Self {
            root,
            definitions: context.finish(),
        }
    }

    /// Build a complete serialization shape graph rooted at `T`.
    pub fn for_type<T>() -> Self
    where
        T: SerializeShape + ?Sized,
    {
        Self::from_fn(T::serialize_shape_in)
    }

    /// Return the root shape reference.
    pub fn root(&self) -> &ShapeRef {
        &self.root
    }

    /// Return the root definition when the graph root is a named type.
    pub fn root_definition(&self) -> Option<&SerializeDefinitionShape> {
        self.definition_for(self.root())
    }

    /// Return the named definitions reachable from the root.
    pub fn definitions(&self) -> &[SerializeDefinitionShape] {
        &self.definitions
    }

    /// Return a definition at this id's graph-local index.
    ///
    /// A [`ShapeId`] does not encode graph ownership. The caller must pass an id produced by this
    /// graph; an id from another graph with an in-bounds index cannot be distinguished here.
    pub fn definition(&self, id: ShapeId) -> Option<&SerializeDefinitionShape> {
        self.definitions.get(id.0)
    }

    /// Return the definition directly referenced by `shape`.
    ///
    /// Returns `None` for non-definition shapes and out-of-bounds definition indexes.
    pub fn definition_for(&self, shape: &ShapeRef) -> Option<&SerializeDefinitionShape> {
        let ShapeRef::Definition(id) = shape else {
            return None;
        };
        self.definition(*id)
    }
}

/// A complete deserialization shape graph rooted at one type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeserializeShapeGraph {
    /// The root shape reference.
    root: ShapeRef,
    /// Named type definitions reachable from the root.
    definitions: Vec<DeserializeDefinitionShape>,
}

impl DeserializeShapeGraph {
    /// Build a deserialization graph from a function that returns its root shape.
    ///
    /// This lets a custom shape function describe a foreign root without introducing a wrapper
    /// type solely to implement [`DeserializeShape`].
    ///
    /// ```rust
    /// use serde_shape::DeserializeShapeContext;
    /// use serde_shape::DeserializeShapeGraph;
    /// use serde_shape::ShapeRef;
    ///
    /// fn duration_input(_context: &mut DeserializeShapeContext) -> ShapeRef {
    ///     ShapeRef::union([ShapeRef::String, ShapeRef::U64])
    /// }
    ///
    /// let graph = DeserializeShapeGraph::from_fn(duration_input);
    /// assert_eq!(
    ///     graph.root(),
    ///     &ShapeRef::union([ShapeRef::String, ShapeRef::U64]),
    /// );
    /// ```
    pub fn from_fn<F>(build_root: F) -> Self
    where
        F: FnOnce(&mut DeserializeShapeContext) -> ShapeRef,
    {
        let mut context = DeserializeShapeContext::default();
        let root = build_root(&mut context);
        Self {
            root,
            definitions: context.finish(),
        }
    }

    /// Build a complete deserialization shape graph rooted at `T`.
    pub fn for_type<T>() -> Self
    where
        T: DeserializeShape + ?Sized,
    {
        Self::from_fn(T::deserialize_shape_in)
    }

    /// Return the root shape reference.
    pub fn root(&self) -> &ShapeRef {
        &self.root
    }

    /// Return the root definition when the graph root is a named type.
    pub fn root_definition(&self) -> Option<&DeserializeDefinitionShape> {
        self.definition_for(self.root())
    }

    /// Return the named definitions reachable from the root.
    pub fn definitions(&self) -> &[DeserializeDefinitionShape] {
        &self.definitions
    }

    /// Return a definition at this id's graph-local index.
    ///
    /// A [`ShapeId`] does not encode graph ownership. The caller must pass an id produced by this
    /// graph; an id from another graph with an in-bounds index cannot be distinguished here.
    pub fn definition(&self, id: ShapeId) -> Option<&DeserializeDefinitionShape> {
        self.definitions.get(id.0)
    }

    /// Return the definition directly referenced by `shape`.
    ///
    /// Returns `None` for non-definition shapes and out-of-bounds definition indexes.
    pub fn definition_for(&self, shape: &ShapeRef) -> Option<&DeserializeDefinitionShape> {
        let ShapeRef::Definition(id) = shape else {
            return None;
        };
        self.definition(*id)
    }
}

/// Accumulates named serialization definitions while a shape graph is built.
#[derive(Debug, Default)]
pub struct SerializeShapeContext {
    definitions: Vec<Option<SerializeDefinitionShape>>,
    definitions_by_identity: BTreeMap<(TypeId, &'static str), ShapeId>,
}

impl SerializeShapeContext {
    /// Define a named type once and return a reference to its definition.
    ///
    /// The concrete builder type and diagnostic Rust name form the graph-local identity. Call this
    /// method from one stable closure expression for every occurrence of the same named type.
    pub fn define_named_type<F>(&mut self, type_name: TypeName, build: F) -> ShapeRef
    where
        F: FnOnce(&mut Self) -> SerializeDefinitionKind + 'static,
    {
        self.define_named_type_with_description(type_name, None, build)
    }

    /// Define a named type with user-facing documentation.
    ///
    /// This behaves like [`Self::define_named_type`] and stores `description` on the resulting
    /// definition.
    pub fn define_named_type_with_description<F>(
        &mut self,
        type_name: TypeName,
        description: Option<&'static str>,
        build: F,
    ) -> ShapeRef
    where
        F: FnOnce(&mut Self) -> SerializeDefinitionKind + 'static,
    {
        let identity = (TypeId::of::<F>(), type_name.rust_name);
        if let Some(id) = self.definitions_by_identity.get(&identity) {
            return ShapeRef::Definition(*id);
        }

        let id = ShapeId(self.definitions.len());
        self.definitions_by_identity.insert(identity, id);
        self.definitions.push(None);

        let kind = build(self);
        self.definitions[id.0] = Some(SerializeDefinitionShape {
            id,
            type_name,
            description,
            kind,
        });
        ShapeRef::Definition(id)
    }

    fn finish(self) -> Vec<SerializeDefinitionShape> {
        self.definitions
            .into_iter()
            .map(|definition| definition.expect("shape definition was reserved but not filled"))
            .collect()
    }
}

/// Accumulates named deserialization definitions while a shape graph is built.
#[derive(Debug, Default)]
pub struct DeserializeShapeContext {
    definitions: Vec<Option<DeserializeDefinitionShape>>,
    definitions_by_identity: BTreeMap<(TypeId, &'static str), ShapeId>,
}

impl DeserializeShapeContext {
    /// Define a named type once and return a reference to its definition.
    ///
    /// The concrete builder type and diagnostic Rust name form the graph-local identity. Call this
    /// method from one stable closure expression for every occurrence of the same named type.
    pub fn define_named_type<F>(&mut self, type_name: TypeName, build: F) -> ShapeRef
    where
        F: FnOnce(&mut Self) -> DeserializeDefinitionKind + 'static,
    {
        self.define_named_type_with_description(type_name, None, build)
    }

    /// Define a named type with user-facing documentation.
    ///
    /// This behaves like [`Self::define_named_type`] and stores `description` on the resulting
    /// definition.
    pub fn define_named_type_with_description<F>(
        &mut self,
        type_name: TypeName,
        description: Option<&'static str>,
        build: F,
    ) -> ShapeRef
    where
        F: FnOnce(&mut Self) -> DeserializeDefinitionKind + 'static,
    {
        let identity = (TypeId::of::<F>(), type_name.rust_name);
        if let Some(id) = self.definitions_by_identity.get(&identity) {
            return ShapeRef::Definition(*id);
        }

        let id = ShapeId(self.definitions.len());
        self.definitions_by_identity.insert(identity, id);
        self.definitions.push(None);

        let kind = build(self);
        self.definitions[id.0] = Some(DeserializeDefinitionShape {
            id,
            type_name,
            description,
            kind,
        });
        ShapeRef::Definition(id)
    }

    fn finish(self) -> Vec<DeserializeDefinitionShape> {
        self.definitions
            .into_iter()
            .map(|definition| definition.expect("shape definition was reserved but not filled"))
            .collect()
    }
}

/// Identifies a named shape definition by its graph-local index.
///
/// An id does not carry the identity of its originating graph. Keep it paired with the graph that
/// produced it.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShapeId(usize);

impl ShapeId {
    /// Return this id's graph-local definition index.
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Names associated with a Rust type and one direction of its Serde representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeName {
    /// The fully qualified Rust type name, including generic arguments.
    pub rust_name: &'static str,
    /// The direction-specific Serde name after container rename rules are applied.
    pub name: &'static str,
}

impl TypeName {
    /// Build names for `T` and one direction-specific Serde container name.
    pub fn of<T>(name: &'static str) -> Self
    where
        T: ?Sized,
    {
        Self {
            rust_name: core::any::type_name::<T>(),
            name,
        }
    }
}

/// A reference to a shape node.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ShapeRef {
    /// Unit shape.
    Unit,
    /// Boolean shape.
    Bool,
    /// Character shape.
    Char,
    /// `i8` shape.
    I8,
    /// `i16` shape.
    I16,
    /// `i32` shape.
    I32,
    /// `i64` shape.
    I64,
    /// `i128` shape.
    I128,
    /// `isize` shape.
    Isize,
    /// `u8` shape.
    U8,
    /// `u16` shape.
    U16,
    /// `u32` shape.
    U32,
    /// `u64` shape.
    U64,
    /// `u128` shape.
    U128,
    /// `usize` shape.
    Usize,
    /// `f32` shape.
    F32,
    /// `f64` shape.
    F64,
    /// UTF-8 string shape.
    String,
    /// Serde byte-buffer data-model shape.
    Bytes,
    /// Optional value shape.
    Option(Box<ShapeRef>),
    /// Sequence shape.
    Seq(Box<ShapeRef>),
    /// Fixed-size array shape.
    ///
    /// Built-in array implementations follow Serde's supported lengths of 0 through 32. Manual
    /// implementations may construct other lengths for custom representations.
    Array {
        /// The array item shape.
        item: Box<ShapeRef>,
        /// The array length.
        len: usize,
    },
    /// Map shape.
    Map {
        /// The map key shape.
        key: Box<ShapeRef>,
        /// The map value shape.
        value: Box<ShapeRef>,
    },
    /// Tuple shape.
    Tuple(Vec<ShapeRef>),
    /// A normalized union of two or more possible value shapes.
    ///
    /// Construct unions with [`ShapeRef::union`] or [`ShapeRef::try_union`].
    Union(UnionShape),
    /// Named type definition reference.
    Definition(ShapeId),
    /// Shape intentionally left opaque.
    Opaque(OpaqueShape),
}

impl ShapeRef {
    /// Build a normalized union from one or more possible value shapes.
    ///
    /// Nested unions are flattened, duplicate alternatives are removed, and alternatives are
    /// sorted into a canonical order. A single distinct alternative is returned directly.
    ///
    /// # Panics
    ///
    /// Panics when `alternatives` is empty. Use [`ShapeRef::try_union`] when the input may be
    /// empty.
    pub fn union<I>(alternatives: I) -> Self
    where
        I: IntoIterator<Item = Self>,
    {
        Self::try_union(alternatives).expect("shape union requires at least one alternative")
    }

    /// Try to build a normalized union from possible value shapes.
    ///
    /// Returns `None` when `alternatives` is empty. Nested unions are flattened, duplicate
    /// alternatives are removed, and alternatives are sorted into a canonical order. A single
    /// distinct alternative is returned directly.
    pub fn try_union<I>(alternatives: I) -> Option<Self>
    where
        I: IntoIterator<Item = Self>,
    {
        let mut normalized = Vec::new();
        for alternative in alternatives {
            match alternative {
                Self::Union(union) => normalized.extend(union.alternatives),
                alternative => normalized.push(alternative),
            }
        }
        let mut alternatives = normalized;
        alternatives.sort();
        alternatives.dedup();

        match alternatives.len() {
            0 => None,
            1 => alternatives.pop(),
            _ => Some(Self::Union(UnionShape { alternatives })),
        }
    }

    /// Return whether this is a signed integer shape.
    pub fn is_signed_integer(&self) -> bool {
        match self {
            Self::I8 | Self::I16 | Self::I32 | Self::I64 | Self::I128 | Self::Isize => true,
            Self::Union(union) => union.alternatives.iter().all(Self::is_signed_integer),
            _ => false,
        }
    }

    /// Return whether this is an unsigned integer shape.
    pub fn is_unsigned_integer(&self) -> bool {
        match self {
            Self::U8 | Self::U16 | Self::U32 | Self::U64 | Self::U128 | Self::Usize => true,
            Self::Union(union) => union.alternatives.iter().all(Self::is_unsigned_integer),
            _ => false,
        }
    }

    /// Return whether this is any integer shape.
    pub fn is_integer(&self) -> bool {
        match self {
            Self::Union(union) => union.alternatives.iter().all(Self::is_integer),
            _ => self.is_signed_integer() || self.is_unsigned_integer(),
        }
    }

    /// Return whether this is a floating point shape.
    pub fn is_float(&self) -> bool {
        match self {
            Self::F32 | Self::F64 => true,
            Self::Union(union) => union.alternatives.iter().all(Self::is_float),
            _ => false,
        }
    }

    /// Return whether this is any numeric shape.
    pub fn is_number(&self) -> bool {
        match self {
            Self::Union(union) => union.alternatives.iter().all(Self::is_number),
            _ => self.is_integer() || self.is_float(),
        }
    }
}

/// The normalized alternatives contained by [`ShapeRef::Union`].
///
/// A union always contains at least two distinct alternatives in canonical order. Use
/// [`ShapeRef::union`] or [`ShapeRef::try_union`] to construct one. Alternatives may overlap; a
/// union means that any alternative is possible, not that exactly one alternative must match.
#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct UnionShape {
    alternatives: Vec<ShapeRef>,
}

impl UnionShape {
    /// Return the canonical union alternatives.
    pub fn alternatives(&self) -> &[ShapeRef] {
        &self.alternatives
    }
}

impl fmt::Debug for UnionShape {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.alternatives.fmt(formatter)
    }
}

/// A named type definition in a serialization shape graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SerializeDefinitionShape {
    /// The stable id of this definition inside its graph.
    pub id: ShapeId,
    /// The Rust and Serde names for this definition.
    pub type_name: TypeName,
    /// User-facing documentation for this definition, if available.
    pub description: Option<&'static str>,
    /// The definition body.
    pub kind: SerializeDefinitionKind,
}

/// A named type definition in a deserialization shape graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeserializeDefinitionShape {
    /// The stable id of this definition inside its graph.
    pub id: ShapeId,
    /// The Rust and Serde names for this definition.
    pub type_name: TypeName,
    /// User-facing documentation for this definition, if available.
    pub description: Option<&'static str>,
    /// The definition body.
    pub kind: DeserializeDefinitionKind,
}

/// The body of a named serialization definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SerializeDefinitionKind {
    /// Struct-like Serde output.
    Struct(SerializeStructShape),
    /// Enum-like Serde output.
    Enum(SerializeEnumShape),
    /// Output shape that cannot be inferred faithfully.
    Opaque(OpaqueShape),
}

/// The body of a named deserialization definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeserializeDefinitionKind {
    /// Struct-like Serde input.
    Struct(DeserializeStructShape),
    /// Enum-like Serde input.
    Enum(DeserializeEnumShape),
    /// Input shape that cannot be inferred faithfully.
    Opaque(OpaqueShape),
}

/// Serde attributes that apply to a whole serialized container.
///
/// [`Default`] represents a container with every optional behavior disabled. Enum tagging is
/// recorded once in [`SerializeEnumShape::repr`], and flattened fields are identified by
/// [`FieldWireShape::Flatten`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SerializeContainerAttributes {
    /// Whether the Rust item is marked `#[non_exhaustive]`.
    pub non_exhaustive: bool,
}

/// Serde attributes that apply to a whole deserialized container.
///
/// [`Default`] represents a container with no default, expectation, or optional behavior
/// configured. Enum tagging is recorded once in [`DeserializeEnumShape::repr`], and flattened
/// fields are identified by [`FieldWireShape::Flatten`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DeserializeContainerAttributes {
    /// Whether unknown fields are rejected.
    pub deny_unknown_fields: bool,
    /// The default used for missing fields.
    pub default: DefaultShape,
    /// Custom Serde expectation text, if present.
    pub expecting: Option<&'static str>,
    /// Whether the Rust item is marked `#[non_exhaustive]`.
    pub non_exhaustive: bool,
}

/// Serde container or enum tagging representation.
///
/// [`Default`] is [`Tagging::External`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Tagging {
    /// The default externally tagged representation.
    #[default]
    External,
    /// `#[serde(tag = "...")]`.
    Internal {
        /// The tag field name.
        tag: &'static str,
    },
    /// `#[serde(tag = "...", content = "...")]`.
    Adjacent {
        /// The tag field name.
        tag: &'static str,
        /// The content field name.
        content: &'static str,
    },
    /// `#[serde(untagged)]`.
    Untagged,
    /// `#[serde(field_identifier)]`, accepted only during deserialization.
    FieldIdentifier,
    /// `#[serde(variant_identifier)]`, accepted only during deserialization.
    VariantIdentifier,
}

/// Struct-like serialization metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SerializeStructShape {
    /// The struct field style.
    pub style: FieldsStyle,
    /// The serialized fields.
    pub fields: Vec<SerializeFieldShape>,
    /// Container-level Serde serialization attributes.
    pub attributes: SerializeContainerAttributes,
}

/// Struct-like deserialization metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeserializeStructShape {
    /// The struct field style.
    pub style: FieldsStyle,
    /// The accepted deserialization fields.
    pub fields: Vec<DeserializeFieldShape>,
    /// Container-level Serde deserialization attributes.
    pub attributes: DeserializeContainerAttributes,
}

/// Enum-like serialization metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SerializeEnumShape {
    /// The enum representation.
    pub repr: Tagging,
    /// The serialized variants.
    pub variants: Vec<SerializeVariantShape>,
    /// Container-level Serde serialization attributes.
    pub attributes: SerializeContainerAttributes,
}

/// Enum-like deserialization metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeserializeEnumShape {
    /// The enum representation.
    pub repr: Tagging,
    /// The accepted deserialization variants.
    pub variants: Vec<DeserializeVariantShape>,
    /// Container-level Serde deserialization attributes.
    pub attributes: DeserializeContainerAttributes,
}

/// The style of a struct, variant, or tuple field list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldsStyle {
    /// Named fields.
    Struct,
    /// Multiple unnamed fields.
    Tuple,
    /// One unnamed field.
    Newtype,
    /// No fields.
    Unit,
}

/// Field-level serialization metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SerializeFieldShape {
    /// The original Rust field member.
    pub member: FieldMember,
    /// The primary Serde serialize name.
    pub name: &'static str,
    /// User-facing documentation for this field, if available.
    pub description: Option<&'static str>,
    /// How this field contributes to the serialized wire shape.
    pub wire_shape: FieldWireShape,
    /// The predicate used to skip this field during serialization, rendered as a parseable Rust
    /// path token stream. Whitespace is not normalized.
    pub skip_if: Option<&'static str>,
}

/// Field-level deserialization metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeserializeFieldShape {
    /// The original Rust field member.
    pub member: FieldMember,
    /// The primary Serde deserialize name.
    pub name: &'static str,
    /// All accepted Serde deserialize names, including the primary name.
    pub aliases: Vec<&'static str>,
    /// User-facing documentation for this field, if available.
    pub description: Option<&'static str>,
    /// How this field contributes to the deserialized wire shape.
    pub wire_shape: FieldWireShape,
    /// The default used if this field is missing.
    pub default: DefaultShape,
}

/// The Rust member represented by a field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldMember {
    /// A named Rust field.
    Named(&'static str),
    /// An unnamed tuple field index.
    Unnamed(usize),
}

/// How a field contributes to the wire representation in one Serde direction.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FieldWireShape {
    /// The field emits or accepts no value in this direction.
    Omitted,
    /// The field appears as a regular value at its Serde position.
    Value(ShapeRef),
    /// The field is flattened into the containing map.
    Flatten(ShapeRef),
    /// The field is serialized or deserialized directly at the containing type's position.
    Inline(ShapeRef),
}

impl FieldWireShape {
    /// Return the contributed value shape, or `None` when the field is omitted.
    ///
    /// This intentionally ignores whether the value is regular, flattened, or inline. Match on
    /// the enum directly when the field's wire position matters.
    pub fn shape(&self) -> Option<&ShapeRef> {
        match self {
            Self::Value(shape) | Self::Flatten(shape) | Self::Inline(shape) => Some(shape),
            Self::Omitted => None,
        }
    }
}

/// Variant-level serialization metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SerializeVariantShape {
    /// The original Rust variant name.
    pub rust_name: &'static str,
    /// The primary Serde serialize name.
    pub name: &'static str,
    /// User-facing documentation for this variant, if available.
    pub description: Option<&'static str>,
    /// The variant field style.
    pub style: FieldsStyle,
    /// How the variant contributes its serialized content.
    pub content: SerializeVariantContent,
    /// Whether this variant is individually marked untagged.
    pub untagged: bool,
}

/// The serialized content controlled by an enum variant.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SerializeVariantContent {
    /// The variant is omitted during serialization.
    Omitted,
    /// Serde derives the variant content from these fields.
    Fields(Vec<SerializeFieldShape>),
    /// A `serde_shape` hook supplies the content shape inside the enum's tagging representation.
    Shape(ShapeRef),
    /// A custom serializer controls the variant content.
    Custom(OpaqueShape),
}

/// Variant-level deserialization metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeserializeVariantShape {
    /// The original Rust variant name.
    pub rust_name: &'static str,
    /// The primary Serde deserialize name.
    pub name: &'static str,
    /// All accepted Serde deserialize names, including the primary name.
    pub aliases: Vec<&'static str>,
    /// User-facing documentation for this variant, if available.
    pub description: Option<&'static str>,
    /// The variant field style.
    pub style: FieldsStyle,
    /// How the variant contributes its deserialized content.
    pub content: DeserializeVariantContent,
    /// Whether this is a Serde `other` catch-all variant.
    pub other: bool,
    /// Whether this variant is individually marked untagged.
    pub untagged: bool,
}

/// The deserialized content controlled by an enum variant.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DeserializeVariantContent {
    /// The variant is omitted during deserialization.
    Omitted,
    /// Serde derives the variant content from these fields.
    Fields(Vec<DeserializeFieldShape>),
    /// A `serde_shape` hook supplies the content shape inside the enum's tagging representation.
    Shape(ShapeRef),
    /// A custom deserializer controls the variant content.
    Custom(OpaqueShape),
}

/// A Serde default marker.
///
/// [`Default`] is [`DefaultShape::None`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum DefaultShape {
    /// No default is configured.
    #[default]
    None,
    /// `Default::default()` is used.
    Default,
    /// A custom default function path is used. The value is a parseable Rust path token stream;
    /// whitespace is not normalized.
    Path(&'static str),
}

impl DefaultShape {
    /// Return whether this value represents no default.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

/// Shape intentionally left opaque.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OpaqueShape {
    /// The Rust type or Serde item that is opaque.
    pub type_name: &'static str,
    /// Why the shape is opaque.
    pub reason: OpaqueReason,
    /// Additional human-readable detail.
    pub detail: Option<&'static str>,
}

/// Reason a shape cannot be represented precisely.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OpaqueReason {
    /// A custom serializer controls the output. Derive-generated `detail` is a parseable Rust path
    /// token stream whose whitespace is not normalized.
    CustomSerializer,
    /// A custom deserializer controls the input. Derive-generated `detail` is a parseable Rust
    /// path token stream whose whitespace is not normalized.
    CustomDeserializer,
    /// The type has no built-in shape implementation.
    Unsupported,
    /// The surrounding representation contains no values of this type.
    Unobserved,
}
