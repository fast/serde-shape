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

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

use std::collections::BTreeMap;

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use serde_shape_derive::DeserializeShape;
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use serde_shape_derive::SerializeShape;

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

macro_rules! primitive_shape {
    ($($ty:ty => $shape:expr;)+) => {
        $(
            impl SerializeShape for $ty {
                fn serialize_shape_in(_context: &mut SerializeShapeContext) -> ShapeRef {
                    $shape
                }
            }

            impl DeserializeShape for $ty {
                fn deserialize_shape_in(_context: &mut DeserializeShapeContext) -> ShapeRef {
                    $shape
                }
            }
        )+
    };
}

primitive_shape! {
    () => ShapeRef::Unit;
    bool => ShapeRef::Bool;
    char => ShapeRef::Char;
    i8 => ShapeRef::I8;
    i16 => ShapeRef::I16;
    i32 => ShapeRef::I32;
    i64 => ShapeRef::I64;
    i128 => ShapeRef::I128;
    isize => ShapeRef::Isize;
    u8 => ShapeRef::U8;
    u16 => ShapeRef::U16;
    u32 => ShapeRef::U32;
    u64 => ShapeRef::U64;
    u128 => ShapeRef::U128;
    usize => ShapeRef::Usize;
    f32 => ShapeRef::F32;
    f64 => ShapeRef::F64;
    str => ShapeRef::String;
    String => ShapeRef::String;
    std::path::Path => ShapeRef::String;
    std::path::PathBuf => ShapeRef::String;
    std::net::IpAddr => ShapeRef::String;
    std::net::Ipv4Addr => ShapeRef::String;
    std::net::Ipv6Addr => ShapeRef::String;
    std::net::SocketAddr => ShapeRef::String;
    std::net::SocketAddrV4 => ShapeRef::String;
    std::net::SocketAddrV6 => ShapeRef::String;
    std::num::NonZeroI8 => ShapeRef::I8;
    std::num::NonZeroI16 => ShapeRef::I16;
    std::num::NonZeroI32 => ShapeRef::I32;
    std::num::NonZeroI64 => ShapeRef::I64;
    std::num::NonZeroI128 => ShapeRef::I128;
    std::num::NonZeroIsize => ShapeRef::Isize;
    std::num::NonZeroU8 => ShapeRef::U8;
    std::num::NonZeroU16 => ShapeRef::U16;
    std::num::NonZeroU32 => ShapeRef::U32;
    std::num::NonZeroU64 => ShapeRef::U64;
    std::num::NonZeroU128 => ShapeRef::U128;
    std::num::NonZeroUsize => ShapeRef::Usize;
}

#[cfg(target_has_atomic = "8")]
primitive_shape! {
    std::sync::atomic::AtomicBool => ShapeRef::Bool;
    std::sync::atomic::AtomicI8 => ShapeRef::I8;
    std::sync::atomic::AtomicU8 => ShapeRef::U8;
}

#[cfg(target_has_atomic = "16")]
primitive_shape! {
    std::sync::atomic::AtomicI16 => ShapeRef::I16;
    std::sync::atomic::AtomicU16 => ShapeRef::U16;
}

#[cfg(target_has_atomic = "32")]
primitive_shape! {
    std::sync::atomic::AtomicI32 => ShapeRef::I32;
    std::sync::atomic::AtomicU32 => ShapeRef::U32;
}

#[cfg(target_has_atomic = "64")]
primitive_shape! {
    std::sync::atomic::AtomicI64 => ShapeRef::I64;
    std::sync::atomic::AtomicU64 => ShapeRef::U64;
}

#[cfg(target_has_atomic = "ptr")]
primitive_shape! {
    std::sync::atomic::AtomicIsize => ShapeRef::Isize;
    std::sync::atomic::AtomicUsize => ShapeRef::Usize;
}

impl SerializeShape for [u8] {
    fn serialize_shape_in(_context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Bytes
    }
}

impl DeserializeShape for [u8] {
    fn deserialize_shape_in(_context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Bytes
    }
}

impl<T> SerializeShape for &T
where
    T: SerializeShape + ?Sized,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        T::serialize_shape_in(context)
    }
}

impl<T> DeserializeShape for &T
where
    T: DeserializeShape + ?Sized,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        T::deserialize_shape_in(context)
    }
}

impl<T> SerializeShape for &mut T
where
    T: SerializeShape + ?Sized,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        T::serialize_shape_in(context)
    }
}

impl<T> DeserializeShape for &mut T
where
    T: DeserializeShape + ?Sized,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        T::deserialize_shape_in(context)
    }
}

impl<T> SerializeShape for Box<T>
where
    T: SerializeShape + ?Sized,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        T::serialize_shape_in(context)
    }
}

impl<T> DeserializeShape for Box<T>
where
    T: DeserializeShape + ?Sized,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        T::deserialize_shape_in(context)
    }
}

impl<'a, T> SerializeShape for std::borrow::Cow<'a, T>
where
    T: ToOwned + ?Sized,
    T::Owned: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        T::Owned::serialize_shape_in(context)
    }
}

impl<'a, T> DeserializeShape for std::borrow::Cow<'a, T>
where
    T: ToOwned + ?Sized,
    T::Owned: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        T::Owned::deserialize_shape_in(context)
    }
}

impl<T> SerializeShape for std::cell::Cell<T>
where
    T: Copy + SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        T::serialize_shape_in(context)
    }
}

impl<T> DeserializeShape for std::cell::Cell<T>
where
    T: Copy + DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        T::deserialize_shape_in(context)
    }
}

impl<T> SerializeShape for std::cell::RefCell<T>
where
    T: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        T::serialize_shape_in(context)
    }
}

impl<T> DeserializeShape for std::cell::RefCell<T>
where
    T: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        T::deserialize_shape_in(context)
    }
}

impl<T> SerializeShape for std::sync::Mutex<T>
where
    T: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        T::serialize_shape_in(context)
    }
}

impl<T> DeserializeShape for std::sync::Mutex<T>
where
    T: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        T::deserialize_shape_in(context)
    }
}

impl<T> SerializeShape for std::sync::RwLock<T>
where
    T: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        T::serialize_shape_in(context)
    }
}

impl<T> DeserializeShape for std::sync::RwLock<T>
where
    T: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        T::deserialize_shape_in(context)
    }
}

impl<T> SerializeShape for std::num::Wrapping<T>
where
    T: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        T::serialize_shape_in(context)
    }
}

impl<T> DeserializeShape for std::num::Wrapping<T>
where
    T: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        T::deserialize_shape_in(context)
    }
}

impl<T> SerializeShape for std::cmp::Reverse<T>
where
    T: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        T::serialize_shape_in(context)
    }
}

impl<T> DeserializeShape for std::cmp::Reverse<T>
where
    T: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        T::deserialize_shape_in(context)
    }
}

impl<T> SerializeShape for Option<T>
where
    T: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Option(Box::new(T::serialize_shape_in(context)))
    }
}

impl<T> DeserializeShape for Option<T>
where
    T: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Option(Box::new(T::deserialize_shape_in(context)))
    }
}

impl<T> SerializeShape for Vec<T>
where
    T: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Seq(Box::new(T::serialize_shape_in(context)))
    }
}

impl<T> DeserializeShape for Vec<T>
where
    T: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Seq(Box::new(T::deserialize_shape_in(context)))
    }
}

impl<T> SerializeShape for std::collections::VecDeque<T>
where
    T: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Seq(Box::new(T::serialize_shape_in(context)))
    }
}

impl<T> DeserializeShape for std::collections::VecDeque<T>
where
    T: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Seq(Box::new(T::deserialize_shape_in(context)))
    }
}

impl<T> SerializeShape for std::collections::LinkedList<T>
where
    T: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Seq(Box::new(T::serialize_shape_in(context)))
    }
}

impl<T> DeserializeShape for std::collections::LinkedList<T>
where
    T: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Seq(Box::new(T::deserialize_shape_in(context)))
    }
}

impl<T> SerializeShape for std::collections::BinaryHeap<T>
where
    T: Ord + SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Seq(Box::new(T::serialize_shape_in(context)))
    }
}

impl<T> DeserializeShape for std::collections::BinaryHeap<T>
where
    T: Ord + DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Seq(Box::new(T::deserialize_shape_in(context)))
    }
}

impl<T, const N: usize> SerializeShape for [T; N]
where
    T: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Array {
            item: Box::new(T::serialize_shape_in(context)),
            len: N,
        }
    }
}

impl<T, const N: usize> DeserializeShape for [T; N]
where
    T: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Array {
            item: Box::new(T::deserialize_shape_in(context)),
            len: N,
        }
    }
}

impl<K, V> SerializeShape for BTreeMap<K, V>
where
    K: SerializeShape,
    V: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Map {
            key: Box::new(K::serialize_shape_in(context)),
            value: Box::new(V::serialize_shape_in(context)),
        }
    }
}

impl<K, V> DeserializeShape for BTreeMap<K, V>
where
    K: DeserializeShape,
    V: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Map {
            key: Box::new(K::deserialize_shape_in(context)),
            value: Box::new(V::deserialize_shape_in(context)),
        }
    }
}

impl<K, V, S> SerializeShape for std::collections::HashMap<K, V, S>
where
    K: SerializeShape,
    V: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Map {
            key: Box::new(K::serialize_shape_in(context)),
            value: Box::new(V::serialize_shape_in(context)),
        }
    }
}

impl<K, V, S> DeserializeShape for std::collections::HashMap<K, V, S>
where
    K: DeserializeShape,
    V: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Map {
            key: Box::new(K::deserialize_shape_in(context)),
            value: Box::new(V::deserialize_shape_in(context)),
        }
    }
}

impl<T> SerializeShape for std::collections::BTreeSet<T>
where
    T: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Seq(Box::new(T::serialize_shape_in(context)))
    }
}

impl<T> DeserializeShape for std::collections::BTreeSet<T>
where
    T: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Seq(Box::new(T::deserialize_shape_in(context)))
    }
}

impl<T, S> SerializeShape for std::collections::HashSet<T, S>
where
    T: SerializeShape,
{
    fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Seq(Box::new(T::serialize_shape_in(context)))
    }
}

impl<T, S> DeserializeShape for std::collections::HashSet<T, S>
where
    T: DeserializeShape,
{
    fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Seq(Box::new(T::deserialize_shape_in(context)))
    }
}

impl<T> SerializeShape for std::marker::PhantomData<T> {
    fn serialize_shape_in(_context: &mut SerializeShapeContext) -> ShapeRef {
        ShapeRef::Unit
    }
}

impl<T> DeserializeShape for std::marker::PhantomData<T> {
    fn deserialize_shape_in(_context: &mut DeserializeShapeContext) -> ShapeRef {
        ShapeRef::Unit
    }
}

macro_rules! tuple_shape {
    ($($name:ident),+ $(,)?) => {
        impl<$($name),+> SerializeShape for ($($name,)+)
        where
            $($name: SerializeShape,)+
        {
            fn serialize_shape_in(context: &mut SerializeShapeContext) -> ShapeRef {
                ShapeRef::Tuple(vec![$($name::serialize_shape_in(context),)+])
            }
        }

        impl<$($name),+> DeserializeShape for ($($name,)+)
        where
            $($name: DeserializeShape,)+
        {
            fn deserialize_shape_in(context: &mut DeserializeShapeContext) -> ShapeRef {
                ShapeRef::Tuple(vec![$($name::deserialize_shape_in(context),)+])
            }
        }
    };
}

tuple_shape!(T0);
tuple_shape!(T0, T1);
tuple_shape!(T0, T1, T2);
tuple_shape!(T0, T1, T2, T3);
tuple_shape!(T0, T1, T2, T3, T4);
tuple_shape!(T0, T1, T2, T3, T4, T5);
tuple_shape!(T0, T1, T2, T3, T4, T5, T6);
tuple_shape!(T0, T1, T2, T3, T4, T5, T6, T7);
tuple_shape!(T0, T1, T2, T3, T4, T5, T6, T7, T8);
tuple_shape!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);
tuple_shape!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
tuple_shape!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_map_shape() {
        let serialize_shape = SerializeShapeGraph::for_type::<BTreeMap<String, Option<u16>>>();
        let deserialize_shape = DeserializeShapeGraph::for_type::<BTreeMap<String, Option<u16>>>();
        let expected = ShapeRef::Map {
            key: Box::new(ShapeRef::String),
            value: Box::new(ShapeRef::Option(Box::new(ShapeRef::U16))),
        };

        assert_eq!(serialize_shape.root, expected);
        assert!(serialize_shape.definitions.is_empty());
        assert_eq!(deserialize_shape.root, expected);
        assert!(deserialize_shape.definitions.is_empty());
    }

    #[test]
    fn maps_common_std_shapes() {
        assert_eq!(
            SerializeShapeGraph::for_type::<std::path::Path>().root,
            ShapeRef::String
        );
        assert_eq!(
            DeserializeShapeGraph::for_type::<std::path::Path>().root,
            ShapeRef::String
        );
        assert_eq!(
            SerializeShapeGraph::for_type::<std::path::PathBuf>().root,
            ShapeRef::String
        );
        assert_eq!(
            DeserializeShapeGraph::for_type::<std::borrow::Cow<'static, str>>().root,
            ShapeRef::String
        );
        assert_eq!(
            SerializeShapeGraph::for_type::<std::cell::Cell<u8>>().root,
            ShapeRef::U8
        );
        assert_eq!(
            DeserializeShapeGraph::for_type::<std::num::Wrapping<i16>>().root,
            ShapeRef::I16
        );
        assert_eq!(
            SerializeShapeGraph::for_type::<std::cmp::Reverse<u32>>().root,
            ShapeRef::U32
        );
        assert_eq!(
            DeserializeShapeGraph::for_type::<std::collections::VecDeque<u8>>().root,
            ShapeRef::Seq(Box::new(ShapeRef::U8))
        );
        assert_eq!(
            SerializeShapeGraph::for_type::<std::collections::LinkedList<i32>>().root,
            ShapeRef::Seq(Box::new(ShapeRef::I32))
        );
        assert_eq!(
            DeserializeShapeGraph::for_type::<std::collections::BinaryHeap<u16>>().root,
            ShapeRef::Seq(Box::new(ShapeRef::U16))
        );
    }

    #[test]
    fn classifies_flat_numeric_shapes() {
        assert!(ShapeRef::I8.is_signed_integer());
        assert!(ShapeRef::Usize.is_unsigned_integer());
        assert!(ShapeRef::I128.is_integer());
        assert!(ShapeRef::U64.is_integer());
        assert!(ShapeRef::F32.is_float());
        assert!(ShapeRef::F64.is_number());
        assert!(!ShapeRef::String.is_number());
    }

    #[cfg(target_has_atomic = "ptr")]
    #[test]
    fn maps_atomic_shapes() {
        assert_eq!(
            SerializeShapeGraph::for_type::<std::sync::atomic::AtomicUsize>().root,
            ShapeRef::Usize
        );
        assert_eq!(
            DeserializeShapeGraph::for_type::<std::sync::atomic::AtomicUsize>().root,
            ShapeRef::Usize
        );
    }
}
