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
use syn::Expr;
use syn::ExprPath;
use syn::Lit;
use syn::LitStr;
use syn::meta::ParseNestedMeta;
use syn::spanned::Spanned;

#[derive(Default)]
pub struct ShapeAttrs {
    serialize_with: Option<(ExprPath, Span)>,
    deserialize_with: Option<(ExprPath, Span)>,
}

impl ShapeAttrs {
    pub fn parse(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut parsed = Self::default();

        for attr in attrs {
            if !attr.path().is_ident("serde_shape") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("serialize_with") {
                    set_once(
                        &mut parsed.serialize_with,
                        parse_path(&meta)?,
                        meta.path.span(),
                    )
                } else if meta.path.is_ident("deserialize_with") {
                    set_once(
                        &mut parsed.deserialize_with,
                        parse_path(&meta)?,
                        meta.path.span(),
                    )
                } else {
                    Err(meta.error(
                        "unknown serde_shape attribute; expected `serialize_with` or `deserialize_with`",
                    ))
                }
            })?;
        }

        Ok(parsed)
    }

    pub fn serialize_with(&self) -> Option<&ExprPath> {
        self.serialize_with.as_ref().map(|(path, _)| path)
    }

    pub fn deserialize_with(&self) -> Option<&ExprPath> {
        self.deserialize_with.as_ref().map(|(path, _)| path)
    }

    pub fn is_empty(&self) -> bool {
        self.serialize_with.is_none() && self.deserialize_with.is_none()
    }
}

pub fn description(attrs: &[Attribute]) -> Option<String> {
    let mut lines = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| match &attr.meta {
            syn::Meta::NameValue(meta) => match &meta.value {
                Expr::Lit(expr) => match &expr.lit {
                    Lit::Str(line) => {
                        let line = line.value();
                        Some(
                            line.strip_prefix(' ')
                                .unwrap_or(&line)
                                .trim_end()
                                .to_owned(),
                        )
                    }
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>();

    while lines.first().is_some_and(String::is_empty) {
        lines.remove(0);
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }

    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn parse_path(meta: &ParseNestedMeta<'_>) -> syn::Result<ExprPath> {
    let value = meta.value()?;
    let value: LitStr = value.parse()?;
    value.parse()
}

fn set_once<T>(slot: &mut Option<(T, Span)>, value: T, span: Span) -> syn::Result<()> {
    if slot.is_some() {
        return Err(syn::Error::new(span, "duplicate serde_shape attribute"));
    }
    *slot = Some((value, span));
    Ok(())
}
