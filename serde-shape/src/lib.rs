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

//! Reflect the Serde deserialization shape of Rust types.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

use std::collections::BTreeMap;

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use serde_shape_derive::SerdeShape;

/// A type that can describe the shape accepted by its Serde deserializer.
pub trait SerdeShape {
    /// Build this type's shape inside the provided context.
    fn shape_in(context: &mut ShapeContext) -> ShapeRef;

    /// Build a complete shape graph rooted at this type.
    fn shape() -> Shape
    where
        Self: Sized,
    {
        Shape::for_type::<Self>()
    }
}

/// A complete shape graph rooted at one type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Shape {
    /// The root shape reference.
    pub root: ShapeRef,
    /// Named type definitions reachable from the root.
    pub definitions: Vec<DefinitionShape>,
}

impl Shape {
    /// Build a complete shape graph rooted at `T`.
    pub fn for_type<T>() -> Self
    where
        T: SerdeShape + ?Sized,
    {
        let mut context = ShapeContext::default();
        let root = T::shape_in(&mut context);
        Self {
            root,
            definitions: context.finish(),
        }
    }

    /// Return a definition by id.
    pub fn definition(&self, id: ShapeId) -> Option<&DefinitionShape> {
        self.definitions.get(id.0)
    }
}

/// Accumulates named definitions while a shape graph is built.
#[derive(Debug, Default)]
pub struct ShapeContext {
    definitions: Vec<Option<DefinitionShape>>,
    definitions_by_rust_name: BTreeMap<&'static str, ShapeId>,
}

impl ShapeContext {
    /// Define a named type once and return a reference to its definition.
    pub fn define_named_type<F>(&mut self, type_name: TypeName, build: F) -> ShapeRef
    where
        F: FnOnce(&mut Self) -> DefinitionKind,
    {
        if let Some(id) = self.definitions_by_rust_name.get(type_name.rust_name) {
            return ShapeRef::Definition(*id);
        }

        let id = ShapeId(self.definitions.len());
        self.definitions_by_rust_name
            .insert(type_name.rust_name, id);
        self.definitions.push(None);

        let kind = build(self);
        self.definitions[id.0] = Some(DefinitionShape {
            id,
            type_name,
            kind,
        });
        ShapeRef::Definition(id)
    }

    fn finish(self) -> Vec<DefinitionShape> {
        self.definitions
            .into_iter()
            .map(|definition| definition.expect("shape definition was reserved but not filled"))
            .collect()
    }
}

/// Identifies a named shape definition.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShapeId(pub usize);

/// Names associated with a Rust type and its Serde container.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeName {
    /// The fully qualified Rust type name, including generic arguments.
    pub rust_name: &'static str,
    /// The Serde deserialize name after container rename rules are applied.
    pub serde_name: &'static str,
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
    /// Signed integer shape.
    Signed(IntegerWidth),
    /// Unsigned integer shape.
    Unsigned(IntegerWidth),
    /// Floating point shape.
    Float(FloatWidth),
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

/// Integer bit width.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntegerWidth {
    /// Pointer-sized integer.
    Size,
    /// 8-bit integer.
    W8,
    /// 16-bit integer.
    W16,
    /// 32-bit integer.
    W32,
    /// 64-bit integer.
    W64,
    /// 128-bit integer.
    W128,
}

/// Floating point bit width.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FloatWidth {
    /// 32-bit floating point number.
    W32,
    /// 64-bit floating point number.
    W64,
}

/// A named type definition in a shape graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefinitionShape {
    /// The stable id of this definition inside its graph.
    pub id: ShapeId,
    /// The Rust and Serde names for this definition.
    pub type_name: TypeName,
    /// The definition body.
    pub kind: DefinitionKind,
}

/// The body of a named type definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DefinitionKind {
    /// Struct-like Serde input.
    Struct(StructShape),
    /// Enum-like Serde input.
    Enum(EnumShape),
    /// Input shape that cannot be inferred faithfully.
    Opaque(OpaqueShape),
}

/// Serde attributes that apply to a whole container.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContainerAttributes {
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

/// Struct-like shape metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructShape {
    /// The struct field style.
    pub style: FieldsStyle,
    /// The accepted deserialization fields.
    pub fields: Vec<FieldShape>,
    /// Container-level Serde attributes.
    pub attributes: ContainerAttributes,
}

/// Enum-like shape metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumShape {
    /// The enum representation.
    pub repr: Tagging,
    /// The accepted deserialization variants.
    pub variants: Vec<VariantShape>,
    /// Container-level Serde attributes.
    pub attributes: ContainerAttributes,
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

/// Field-level shape metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldShape {
    /// The original Rust field member.
    pub member: FieldMember,
    /// The primary Serde deserialize name.
    pub deserialize_name: &'static str,
    /// All accepted Serde deserialize names, including the primary name.
    pub deserialize_aliases: Vec<&'static str>,
    /// The field input shape, or `None` when the field has no inferred input.
    pub shape: Option<ShapeRef>,
    /// The default used if this field is missing.
    pub default: DefaultShape,
    /// Whether the field is flattened into the containing map.
    pub flatten: bool,
    /// Whether Serde skips this field during deserialization.
    pub skip_deserializing: bool,
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

/// Variant-level shape metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariantShape {
    /// The original Rust variant name.
    pub rust_name: &'static str,
    /// The primary Serde deserialize name.
    pub deserialize_name: &'static str,
    /// All accepted Serde deserialize names, including the primary name.
    pub deserialize_aliases: Vec<&'static str>,
    /// The variant field style.
    pub style: FieldsStyle,
    /// The variant fields, if their input shape can be inferred.
    pub fields: Vec<FieldShape>,
    /// Whether Serde skips this variant during deserialization.
    pub skip_deserializing: bool,
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
    /// The type uses `#[serde(remote = "...")]`.
    Remote,
    /// A custom deserializer controls the input.
    CustomDeserializer,
    /// The type has no built-in shape implementation.
    Unsupported,
}

macro_rules! primitive_shape {
    ($ty:ty, $shape:expr) => {
        impl SerdeShape for $ty {
            fn shape_in(_context: &mut ShapeContext) -> ShapeRef {
                $shape
            }
        }
    };
}

primitive_shape!((), ShapeRef::Unit);
primitive_shape!(bool, ShapeRef::Bool);
primitive_shape!(char, ShapeRef::Char);
primitive_shape!(i8, ShapeRef::Signed(IntegerWidth::W8));
primitive_shape!(i16, ShapeRef::Signed(IntegerWidth::W16));
primitive_shape!(i32, ShapeRef::Signed(IntegerWidth::W32));
primitive_shape!(i64, ShapeRef::Signed(IntegerWidth::W64));
primitive_shape!(i128, ShapeRef::Signed(IntegerWidth::W128));
primitive_shape!(isize, ShapeRef::Signed(IntegerWidth::Size));
primitive_shape!(u8, ShapeRef::Unsigned(IntegerWidth::W8));
primitive_shape!(u16, ShapeRef::Unsigned(IntegerWidth::W16));
primitive_shape!(u32, ShapeRef::Unsigned(IntegerWidth::W32));
primitive_shape!(u64, ShapeRef::Unsigned(IntegerWidth::W64));
primitive_shape!(u128, ShapeRef::Unsigned(IntegerWidth::W128));
primitive_shape!(usize, ShapeRef::Unsigned(IntegerWidth::Size));
primitive_shape!(f32, ShapeRef::Float(FloatWidth::W32));
primitive_shape!(f64, ShapeRef::Float(FloatWidth::W64));
primitive_shape!(String, ShapeRef::String);

impl SerdeShape for str {
    fn shape_in(_context: &mut ShapeContext) -> ShapeRef {
        ShapeRef::String
    }
}

impl SerdeShape for [u8] {
    fn shape_in(_context: &mut ShapeContext) -> ShapeRef {
        ShapeRef::Bytes
    }
}

impl<T> SerdeShape for &T
where
    T: SerdeShape + ?Sized,
{
    fn shape_in(context: &mut ShapeContext) -> ShapeRef {
        T::shape_in(context)
    }
}

impl<T> SerdeShape for &mut T
where
    T: SerdeShape + ?Sized,
{
    fn shape_in(context: &mut ShapeContext) -> ShapeRef {
        T::shape_in(context)
    }
}

impl<T> SerdeShape for Box<T>
where
    T: SerdeShape + ?Sized,
{
    fn shape_in(context: &mut ShapeContext) -> ShapeRef {
        T::shape_in(context)
    }
}

impl<T> SerdeShape for Option<T>
where
    T: SerdeShape,
{
    fn shape_in(context: &mut ShapeContext) -> ShapeRef {
        ShapeRef::Option(Box::new(T::shape_in(context)))
    }
}

impl<T> SerdeShape for Vec<T>
where
    T: SerdeShape,
{
    fn shape_in(context: &mut ShapeContext) -> ShapeRef {
        ShapeRef::Seq(Box::new(T::shape_in(context)))
    }
}

impl<T, const N: usize> SerdeShape for [T; N]
where
    T: SerdeShape,
{
    fn shape_in(context: &mut ShapeContext) -> ShapeRef {
        ShapeRef::Array {
            item: Box::new(T::shape_in(context)),
            len: N,
        }
    }
}

impl<K, V> SerdeShape for BTreeMap<K, V>
where
    K: SerdeShape,
    V: SerdeShape,
{
    fn shape_in(context: &mut ShapeContext) -> ShapeRef {
        ShapeRef::Map {
            key: Box::new(K::shape_in(context)),
            value: Box::new(V::shape_in(context)),
        }
    }
}

impl<K, V, S> SerdeShape for std::collections::HashMap<K, V, S>
where
    K: SerdeShape,
    V: SerdeShape,
{
    fn shape_in(context: &mut ShapeContext) -> ShapeRef {
        ShapeRef::Map {
            key: Box::new(K::shape_in(context)),
            value: Box::new(V::shape_in(context)),
        }
    }
}

impl<T> SerdeShape for std::collections::BTreeSet<T>
where
    T: SerdeShape,
{
    fn shape_in(context: &mut ShapeContext) -> ShapeRef {
        ShapeRef::Seq(Box::new(T::shape_in(context)))
    }
}

impl<T, S> SerdeShape for std::collections::HashSet<T, S>
where
    T: SerdeShape,
{
    fn shape_in(context: &mut ShapeContext) -> ShapeRef {
        ShapeRef::Seq(Box::new(T::shape_in(context)))
    }
}

impl<T> SerdeShape for std::marker::PhantomData<T> {
    fn shape_in(_context: &mut ShapeContext) -> ShapeRef {
        ShapeRef::Unit
    }
}

macro_rules! tuple_shape {
    ($($name:ident),+ $(,)?) => {
        impl<$($name),+> SerdeShape for ($($name,)+)
        where
            $($name: SerdeShape,)+
        {
            fn shape_in(context: &mut ShapeContext) -> ShapeRef {
                ShapeRef::Tuple(vec![$($name::shape_in(context),)+])
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
        let shape = Shape::for_type::<BTreeMap<String, Option<u16>>>();

        assert_eq!(
            shape.root,
            ShapeRef::Map {
                key: Box::new(ShapeRef::String),
                value: Box::new(ShapeRef::Option(Box::new(ShapeRef::Unsigned(
                    IntegerWidth::W16
                )))),
            }
        );
        assert!(shape.definitions.is_empty());
    }
}
