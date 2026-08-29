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

use proc_macro2::Span;
use syn::Attribute;
use syn::LitStr;
use syn::Type;
use syn::meta::ParseNestedMeta;
use syn::spanned::Spanned;

#[derive(Default)]
pub struct ShapeAttrs {
    serialize_as: Option<(Type, Span)>,
    deserialize_as: Option<(Type, Span)>,
    with: Option<(Type, Span)>,
}

impl ShapeAttrs {
    pub fn parse(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut parsed = Self::default();

        for attr in attrs {
            if !attr.path().is_ident("serde_shape") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("with") {
                    set_once(&mut parsed.with, parse_type(&meta)?, meta.path.span())
                } else if meta.path.is_ident("serialize_as") {
                    set_once(
                        &mut parsed.serialize_as,
                        parse_type(&meta)?,
                        meta.path.span(),
                    )
                } else if meta.path.is_ident("deserialize_as") {
                    set_once(
                        &mut parsed.deserialize_as,
                        parse_type(&meta)?,
                        meta.path.span(),
                    )
                } else {
                    Err(meta.error(
                        "unknown serde_shape attribute; expected `with`, `serialize_as`, or `deserialize_as`",
                    ))
                }
            })?;
        }

        if let Some((_, span)) = &parsed.with {
            if parsed.serialize_as.is_some() || parsed.deserialize_as.is_some() {
                return Err(syn::Error::new(
                    *span,
                    "`with` cannot be combined with `serialize_as` or `deserialize_as`",
                ));
            }
        }

        Ok(parsed)
    }

    pub fn serialize_as(&self) -> Option<&Type> {
        self.serialize_as
            .as_ref()
            .or(self.with.as_ref())
            .map(|(ty, _)| ty)
    }

    pub fn deserialize_as(&self) -> Option<&Type> {
        self.deserialize_as
            .as_ref()
            .or(self.with.as_ref())
            .map(|(ty, _)| ty)
    }

    pub fn is_empty(&self) -> bool {
        self.serialize_as.is_none() && self.deserialize_as.is_none() && self.with.is_none()
    }
}

fn parse_type(meta: &ParseNestedMeta<'_>) -> syn::Result<Type> {
    let value = meta.value()?;
    let value: LitStr = value.parse()?;
    value.parse()
}

fn set_once(slot: &mut Option<(Type, Span)>, ty: Type, span: Span) -> syn::Result<()> {
    if slot.is_some() {
        return Err(syn::Error::new(span, "duplicate serde_shape attribute"));
    }
    *slot = Some((ty, span));
    Ok(())
}
