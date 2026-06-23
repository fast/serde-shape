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
//! `serde-shape` builds a lightweight graph that describes what a Rust type
//! emits through Serde serialization and accepts through Serde deserialization.
//! It does not run Serde and it is not a full validation schema. Instead, it
//! gives tools access to the same structural information that Serde derives
//! from Rust types and `#[serde(...)]` attributes.
//!
//! Common uses are generating configuration reference docs, deriving
//! environment-variable maps from config structs, documenting wire formats,
//! and checking whether two versions of a type expose compatible Serde shapes.
//!
//! # Install
//!
//! Enable the `derive` feature when you want `#[derive(SerializeShape)]` and
//! `#[derive(DeserializeShape)]`:
//!
//! ```toml
//! [dependencies]
//! serde-shape = { version = "0.0.1", features = ["derive"] }
//! ```
//!
//! Enable `std` when the reflected types use shapes provided only by the Rust
//! standard library:
//!
//! ```toml
//! [dependencies]
//! serde-shape = { version = "0.0.1", features = ["derive", "std"] }
//! ```
//!
//! The crate is `no_std` by default and requires `alloc`.
//!
//! # Quick start
//!
//! Derive [`trait@DeserializeShape`] for the type you want to inspect, then
//! build a [`DeserializeShapeGraph`]:
//!
//! ```rust
//! # #[cfg(feature = "derive")]
//! # {
//! use serde_shape::DeserializeDefinitionKind;
//! use serde_shape::DeserializeShape;
//! use serde_shape::FieldsStyle;
//! use serde_shape::ShapeRef;
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
//! let ShapeRef::Definition(config_id) = graph.root else {
//!     panic!("Config should produce a named definition");
//! };
//! let config = graph.definition(config_id).unwrap();
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
//! Serialization and deserialization are reflected separately because Serde
//! lets the two directions differ:
//!
//! ```rust
//! # #[cfg(feature = "derive")]
//! # {
//! use serde_shape::DeserializeDefinitionKind;
//! use serde_shape::DeserializeShape;
//! use serde_shape::SerializeDefinitionKind;
//! use serde_shape::SerializeShape;
//! use serde_shape::ShapeRef;
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
//!
//! let ShapeRef::Definition(serialize_id) = serialize_graph.root else {
//!     panic!("Message should produce a named serialization definition");
//! };
//! let ShapeRef::Definition(deserialize_id) = deserialize_graph.root else {
//!     panic!("Message should produce a named deserialization definition");
//! };
//!
//! let serialize_definition = serialize_graph.definition(serialize_id).unwrap();
//! let deserialize_definition = deserialize_graph.definition(deserialize_id).unwrap();
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
//! A shape graph has a [`ShapeRef`] root and a list of named definitions. Flat
//! primitive and compound values are represented directly as [`ShapeRef`]
//! values. Structs and enums are stored as named definitions and referenced by
//! [`ShapeId`].
//!
//! Definition IDs are local to one graph. Use [`SerializeShapeGraph::definition`]
//! or [`DeserializeShapeGraph::definition`] to resolve them.
//!
//! # Derive behavior
//!
//! The derive macros read Serde container, variant, and field attributes, so the resulting shape
//! follows the metadata Serde derives for each direction.
//!
//! A custom serializer or deserializer has no inferable inner shape, so the
//! affected field or variant is marked as custom and its nested shape is omitted.
//! Whole-container conversion and remote-derive attributes are represented as
//! opaque definitions.
//!
//! # Manual implementations
//!
//! Implement [`trait@SerializeShape`] or [`trait@DeserializeShape`] manually
//! when a type's Serde representation is known but cannot be derived. This is
//! common for wrappers that deserialize from a string or another primitive
//! representation:
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
//!         ShapeRef::String
//!     }
//! }
//!
//! assert_eq!(ByteSize::deserialize_shape().root, ShapeRef::String);
//! ```
//!
//! For recursive or shared named types, use
//! [`SerializeShapeContext::define_named_type`] or
//! [`DeserializeShapeContext::define_named_type`] so the graph contains one
//! definition and all recursive edges point back to it.

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

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
/// The macro understands the `#[serde(...)]` metadata that Serde's deserialize
/// derive exposes.
///
/// # Example
///
/// ```rust
/// # #[cfg(feature = "derive")]
/// # {
/// use serde_shape::DefaultShape;
/// use serde_shape::DeserializeDefinitionKind;
/// use serde_shape::DeserializeShape;
/// use serde_shape::ShapeRef;
/// use serde_shape::Tagging;
///
/// #[derive(DeserializeShape)]
/// #[serde(
///     tag = "kind",
///     rename_all = "snake_case",
///     rename_all_fields = "snake_case"
/// )]
/// enum Storage {
///     Local {
///         #[serde(default)]
///         data_dir: String,
///     },
///     S3 {
///         bucket_name: String,
///     },
/// }
///
/// let graph = Storage::deserialize_shape();
/// let ShapeRef::Definition(id) = graph.root else {
///     panic!("Storage should produce a named definition");
/// };
/// let definition = graph.definition(id).unwrap();
///
/// let DeserializeDefinitionKind::Enum(shape) = &definition.kind else {
///     panic!("Storage should produce an enum shape");
/// };
///
/// assert_eq!(shape.repr, Tagging::Internal { tag: "kind" });
/// assert_eq!(shape.variants[0].name, "local");
/// assert_eq!(shape.variants[0].fields[0].name, "data_dir");
/// assert_eq!(shape.variants[0].fields[0].default, DefaultShape::Default);
/// assert_eq!(shape.variants[1].fields[0].name, "bucket_name");
/// # }
/// ```
pub use serde_shape_derive::DeserializeShape;

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
/// Derive [`trait@SerializeShape`] from Serde serialization metadata.
///
/// The macro reflects the serialized field and variant names, container tagging
/// mode, skipped output, skip predicates, flattening, transparent containers,
/// and custom serializers that determine a type's Serde output shape.
///
/// # Example
///
/// ```rust
/// # #[cfg(feature = "derive")]
/// # {
/// use serde_shape::SerializeDefinitionKind;
/// use serde_shape::SerializeShape;
/// use serde_shape::ShapeRef;
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
/// let ShapeRef::Definition(id) = graph.root else {
///     panic!("Response should produce a named definition");
/// };
/// let definition = graph.definition(id).unwrap();
///
/// let SerializeDefinitionKind::Struct(shape) = &definition.kind else {
///     panic!("Response should produce a struct shape");
/// };
///
/// assert_eq!(definition.type_name.name, "api-response");
/// assert_eq!(shape.fields[0].name, "requestId");
/// assert_eq!(shape.fields[1].name, "nextPage");
/// assert_eq!(shape.fields[1].skip_if, Some("Option::is_none"));
/// # }
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
    fn serialize_shape() -> SerializeShapeGraph
    where
        Self: Sized,
    {
        SerializeShapeGraph::for_type::<Self>()
    }
}

/// A type that can describe the shape accepted by its Serde deserializer.
pub trait DeserializeShape {
    /// Build this type's deserialization shape inside the provided context.
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef;

    /// Build a complete deserialization shape graph rooted at this type.
    fn deserialize_shape() -> DeserializeShapeGraph
    where
        Self: Sized,
    {
        DeserializeShapeGraph::for_type::<Self>()
    }
}

/// A complete serialization shape graph rooted at one type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SerializeShapeGraph {
    /// The root shape reference.
    pub root: ShapeRef,
    /// Named type definitions reachable from the root.
    pub definitions: Vec<SerializeDefinitionShape>,
}

impl SerializeShapeGraph {
    /// Build a complete serialization shape graph rooted at `T`.
    pub fn for_type<T>() -> Self
    where
        T: SerializeShape + ?Sized,
    {
        let mut context = SerializeShapeContext::default();
        let root = T::serialize_shape_in(&mut context);
        Self {
            root,
            definitions: context.finish(),
        }
    }

    /// Return a definition by id.
    pub fn definition(&self, id: ShapeId) -> Option<&SerializeDefinitionShape> {
        self.definitions.get(id.0)
    }
}

/// A complete deserialization shape graph rooted at one type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeserializeShapeGraph {
    /// The root shape reference.
    pub root: ShapeRef,
    /// Named type definitions reachable from the root.
    pub definitions: Vec<DeserializeDefinitionShape>,
}

impl DeserializeShapeGraph {
    /// Build a complete deserialization shape graph rooted at `T`.
    pub fn for_type<T>() -> Self
    where
        T: DeserializeShape + ?Sized,
    {
        let mut context = DeserializeShapeContext::default();
        let root = T::deserialize_shape_in(&mut context);
        Self {
            root,
            definitions: context.finish(),
        }
    }

    /// Return a definition by id.
    pub fn definition(&self, id: ShapeId) -> Option<&DeserializeDefinitionShape> {
        self.definitions.get(id.0)
    }
}

/// Accumulates named serialization definitions while a shape graph is built.
#[derive(Debug, Default)]
pub struct SerializeShapeContext {
    definitions: Vec<Option<SerializeDefinitionShape>>,
    definitions_by_rust_name: BTreeMap<&'static str, ShapeId>,
}

impl SerializeShapeContext {
    /// Define a named type once and return a reference to its definition.
    pub fn define_named_type<F>(&mut self, type_name: SerializeTypeName, build: F) -> ShapeRef
    where
        F: FnOnce(&mut Self) -> SerializeDefinitionKind,
    {
        if let Some(id) = self.definitions_by_rust_name.get(type_name.rust_name) {
            return ShapeRef::Definition(*id);
        }

        let id = ShapeId(self.definitions.len());
        self.definitions_by_rust_name
            .insert(type_name.rust_name, id);
        self.definitions.push(None);

        let kind = build(self);
        self.definitions[id.0] = Some(SerializeDefinitionShape {
            id,
            type_name,
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
    definitions_by_rust_name: BTreeMap<&'static str, ShapeId>,
}

impl DeserializeShapeContext {
    /// Define a named type once and return a reference to its definition.
    pub fn define_named_type<F>(&mut self, type_name: DeserializeTypeName, build: F) -> ShapeRef
    where
        F: FnOnce(&mut Self) -> DeserializeDefinitionKind,
    {
        if let Some(id) = self.definitions_by_rust_name.get(type_name.rust_name) {
            return ShapeRef::Definition(*id);
        }

        let id = ShapeId(self.definitions.len());
        self.definitions_by_rust_name
            .insert(type_name.rust_name, id);
        self.definitions.push(None);

        let kind = build(self);
        self.definitions[id.0] = Some(DeserializeDefinitionShape {
            id,
            type_name,
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

/// Identifies a named shape definition.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShapeId(pub usize);

/// Names associated with a Rust type and its Serde serializer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SerializeTypeName {
    /// The fully qualified Rust type name, including generic arguments.
    pub rust_name: &'static str,
    /// The Serde serialize name after container rename rules are applied.
    pub name: &'static str,
}

/// Names associated with a Rust type and its Serde deserializer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeserializeTypeName {
    /// The fully qualified Rust type name, including generic arguments.
    pub rust_name: &'static str,
    /// The Serde deserialize name after container rename rules are applied.
    pub name: &'static str,
}

/// A reference to a shape node.
#[derive(Clone, Debug, Eq, PartialEq)]
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
    /// Byte buffer shape.
    Bytes,
    /// Optional value shape.
    Option(Box<ShapeRef>),
    /// Sequence shape.
    Seq(Box<ShapeRef>),
    /// Fixed-size array shape.
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
    /// Named type definition reference.
    Definition(ShapeId),
    /// Shape intentionally left opaque.
    Opaque(OpaqueShape),
}

impl ShapeRef {
    /// Return whether this is a signed integer shape.
    pub fn is_signed_integer(&self) -> bool {
        matches!(
            self,
            Self::I8 | Self::I16 | Self::I32 | Self::I64 | Self::I128 | Self::Isize
        )
    }

    /// Return whether this is an unsigned integer shape.
    pub fn is_unsigned_integer(&self) -> bool {
        matches!(
            self,
            Self::U8 | Self::U16 | Self::U32 | Self::U64 | Self::U128 | Self::Usize
        )
    }

    /// Return whether this is any integer shape.
    pub fn is_integer(&self) -> bool {
        self.is_signed_integer() || self.is_unsigned_integer()
    }

    /// Return whether this is a floating point shape.
    pub fn is_float(&self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }

    /// Return whether this is any numeric shape.
    pub fn is_number(&self) -> bool {
        self.is_integer() || self.is_float()
    }
}

/// A named type definition in a serialization shape graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SerializeDefinitionShape {
    /// The stable id of this definition inside its graph.
    pub id: ShapeId,
    /// The Rust and Serde names for this definition.
    pub type_name: SerializeTypeName,
    /// The definition body.
    pub kind: SerializeDefinitionKind,
}

/// A named type definition in a deserialization shape graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeserializeDefinitionShape {
    /// The stable id of this definition inside its graph.
    pub id: ShapeId,
    /// The Rust and Serde names for this definition.
    pub type_name: DeserializeTypeName,
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SerializeContainerAttributes {
    /// The container tagging representation.
    pub tagging: Tagging,
    /// Whether any field is flattened.
    pub has_flatten: bool,
    /// Whether the container uses `#[serde(transparent)]`.
    pub transparent: bool,
    /// Whether the Rust item is marked `#[non_exhaustive]`.
    pub non_exhaustive: bool,
}

/// Serde attributes that apply to a whole deserialized container.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeserializeContainerAttributes {
    /// The container tagging representation.
    pub tagging: Tagging,
    /// Whether unknown fields are rejected.
    pub deny_unknown_fields: bool,
    /// The default used for missing fields.
    pub default: DefaultShape,
    /// Whether any field is flattened.
    pub has_flatten: bool,
    /// Whether the container uses `#[serde(transparent)]`.
    pub transparent: bool,
    /// Custom Serde expectation text, if present.
    pub expecting: Option<&'static str>,
    /// Whether the Rust item is marked `#[non_exhaustive]`.
    pub non_exhaustive: bool,
}

/// Serde container or enum tagging representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Tagging {
    /// The default externally tagged representation.
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
    /// The field output shape, or `None` when the field has no inferred output.
    pub value_shape: Option<ShapeRef>,
    /// Whether the field is flattened into the containing map.
    pub flatten: bool,
    /// Whether Serde skips this field during serialization.
    pub skip: bool,
    /// The predicate used to skip this field during serialization.
    pub skip_if: Option<&'static str>,
    /// Whether this field uses a custom serializer.
    pub custom_serializer: bool,
    /// Whether this is the transparent field of a transparent container.
    pub transparent: bool,
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
    /// The field input shape, or `None` when the field has no inferred input.
    pub value_shape: Option<ShapeRef>,
    /// The default used if this field is missing.
    pub default: DefaultShape,
    /// Whether the field is flattened into the containing map.
    pub flatten: bool,
    /// Whether Serde skips this field during deserialization.
    pub skip: bool,
    /// Whether this field uses a custom deserializer.
    pub custom_deserializer: bool,
    /// Whether this is the transparent field of a transparent container.
    pub transparent: bool,
}

/// The Rust member represented by a field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldMember {
    /// A named Rust field.
    Named(&'static str),
    /// An unnamed tuple field index.
    Unnamed(usize),
}

/// Variant-level serialization metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SerializeVariantShape {
    /// The original Rust variant name.
    pub rust_name: &'static str,
    /// The primary Serde serialize name.
    pub name: &'static str,
    /// The variant field style.
    pub style: FieldsStyle,
    /// The variant fields, if their output shape can be inferred.
    pub fields: Vec<SerializeFieldShape>,
    /// Whether Serde skips this variant during serialization.
    pub skip: bool,
    /// Whether this variant uses a custom serializer.
    pub custom_serializer: bool,
    /// Whether this variant is individually marked untagged.
    pub untagged: bool,
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
    /// The variant field style.
    pub style: FieldsStyle,
    /// The variant fields, if their input shape can be inferred.
    pub fields: Vec<DeserializeFieldShape>,
    /// Whether Serde skips this variant during deserialization.
    pub skip: bool,
    /// Whether this variant uses a custom deserializer.
    pub custom_deserializer: bool,
    /// Whether this is a Serde `other` catch-all variant.
    pub other: bool,
    /// Whether this variant is individually marked untagged.
    pub untagged: bool,
}

/// A Serde default marker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DefaultShape {
    /// No default is configured.
    None,
    /// `Default::default()` is used.
    Default,
    /// A custom default function path is used.
    Path(&'static str),
}

impl DefaultShape {
    /// Return whether this value represents no default.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

/// Shape intentionally left opaque.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpaqueShape {
    /// The Rust type or Serde item that is opaque.
    pub type_name: &'static str,
    /// Why the shape is opaque.
    pub reason: OpaqueReason,
    /// Additional human-readable detail.
    pub detail: Option<&'static str>,
}

/// Reason a shape cannot be represented precisely.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpaqueReason {
    /// The type uses `#[serde(from = "...")]`.
    FromType,
    /// The type uses `#[serde(try_from = "...")]`.
    TryFromType,
    /// The type uses `#[serde(into = "...")]`.
    IntoType,
    /// The type uses `#[serde(remote = "...")]`.
    Remote,
    /// A custom serializer controls the output.
    CustomSerializer,
    /// A custom deserializer controls the input.
    CustomDeserializer,
    /// The type has no built-in shape implementation.
    Unsupported,
}
