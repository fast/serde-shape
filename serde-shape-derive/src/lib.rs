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

//! Derive macros for `serde-shape`.

use std::collections::BTreeSet;

use proc_macro::TokenStream;
use proc_macro_crate::FoundCrate;
use proc_macro_crate::crate_name;
use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use quote::quote;
use serde_derive_internals::Ctxt;
use serde_derive_internals::Derive;
use serde_derive_internals::ast;
use serde_derive_internals::attr;
use serde_derive_internals::name::Name;
use serde_derive_internals::ungroup;
use syn::DeriveInput;
use syn::GenericArgument;
use syn::LitStr;
use syn::Member;
use syn::PathArguments;
use syn::ReturnType;
use syn::Type;
use syn::TypeParamBound;
use syn::parse_macro_input;
use syn::parse_quote;

mod shape_attr;

use shape_attr::ShapeAttrs;
use shape_attr::description;

/// Derive `serde_shape::SerializeShape` from Serde serialize metadata.
#[proc_macro_derive(SerializeShape, attributes(serde, serde_shape))]
pub fn derive_serialize_shape(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand_serialize_shape(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive `serde_shape::DeserializeShape` from Serde deserialize metadata.
#[proc_macro_derive(DeserializeShape, attributes(serde, serde_shape))]
pub fn derive_deserialize_shape(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand_deserialize_shape(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_serialize_shape(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let serde_shape = serde_shape_crate()?;
    let container = parse_container(input, Derive::Serialize)?;
    let shape_attrs = ShapeAttrs::parse(&input.attrs)?;
    validate_shape_attrs(&container)?;
    let ident = &input.ident;
    let mut generics = input.generics.clone();
    add_serialize_shape_bounds(&mut generics, &container, &shape_attrs)?;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let body = serialize_shape_body(&container, &shape_attrs)?;

    Ok(quote! {
        const _: () = {
            use #serde_shape as __serde_shape;

            impl #impl_generics __serde_shape::SerializeShape for #ident #ty_generics #where_clause {
                fn serialize_shape_in(
                    context: &mut __serde_shape::SerializeShapeContext,
                ) -> __serde_shape::ShapeRef {
                    #body
                }
            }
        };
    })
}

fn expand_deserialize_shape(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let serde_shape = serde_shape_crate()?;
    let container = parse_container(input, Derive::Deserialize)?;
    let shape_attrs = ShapeAttrs::parse(&input.attrs)?;
    validate_shape_attrs(&container)?;
    let ident = &input.ident;
    let mut generics = input.generics.clone();
    add_deserialize_shape_bounds(&mut generics, &container, &shape_attrs)?;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let body = deserialize_shape_body(&container, &shape_attrs)?;

    Ok(quote! {
        const _: () = {
            use #serde_shape as __serde_shape;

            impl #impl_generics __serde_shape::DeserializeShape for #ident #ty_generics #where_clause {
                fn deserialize_shape_in(
                    context: &mut __serde_shape::DeserializeShapeContext,
                ) -> __serde_shape::ShapeRef {
                    #body
                }
            }
        };
    })
}

fn serde_shape_crate() -> syn::Result<TokenStream2> {
    match crate_name("serde-shape") {
        Ok(FoundCrate::Itself) => Ok(quote!(::serde_shape)),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name.replace('-', "_"), Span::call_site());
            Ok(quote!(::#ident))
        }
        Err(err) => Err(syn::Error::new(
            Span::call_site(),
            format!("serde-shape derive could not resolve the serde-shape crate: {err}"),
        )),
    }
}

fn parse_container<'a>(input: &'a DeriveInput, derive: Derive) -> syn::Result<ast::Container<'a>> {
    let cx = Ctxt::new();
    let private = Ident::new("__private", Span::call_site());
    let Some(container) = ast::Container::from_ast(&cx, input, derive, &private) else {
        cx.check()?;
        return Err(syn::Error::new_spanned(
            input,
            "serde-shape could not parse this item",
        ));
    };
    cx.check()?;

    if matches!(derive, Derive::Serialize) {
        let message = match container.attrs.identifier() {
            attr::Identifier::No => None,
            attr::Identifier::Field => Some("field identifiers cannot be serialized"),
            attr::Identifier::Variant => Some("variant identifiers cannot be serialized"),
        };
        if let Some(message) = message {
            return Err(syn::Error::new_spanned(input, message));
        }
    }

    Ok(container)
}

fn validate_shape_attrs(container: &ast::Container<'_>) -> syn::Result<()> {
    match &container.data {
        ast::Data::Enum(variants) => {
            for variant in variants {
                validate_variant_shape_attrs(variant)?;
                for field in &variant.fields {
                    validate_field_shape_attrs(field)?;
                }
            }
        }
        ast::Data::Struct(_, fields) => {
            for field in fields {
                validate_field_shape_attrs(field)?;
            }
        }
    }
    Ok(())
}

fn validate_variant_shape_attrs(variant: &ast::Variant<'_>) -> syn::Result<()> {
    let attrs = ShapeAttrs::parse(&variant.original.attrs)?;
    if attrs.has_bound() {
        return Err(syn::Error::new_spanned(
            variant.original,
            "serde_shape bounds are supported on containers, not variants",
        ));
    }
    Ok(())
}

fn validate_field_shape_attrs(field: &ast::Field<'_>) -> syn::Result<()> {
    let attrs = ShapeAttrs::parse(&field.original.attrs)?;
    if attrs.has_bound() {
        return Err(syn::Error::new_spanned(
            field.original,
            "serde_shape bounds are supported on containers, not fields",
        ));
    }
    Ok(())
}

fn add_serialize_shape_bounds(
    generics: &mut syn::Generics,
    container: &ast::Container<'_>,
    shape_attrs: &ShapeAttrs,
) -> syn::Result<()> {
    if let Some(predicates) = shape_attrs.serialize_bound() {
        generics
            .make_where_clause()
            .predicates
            .extend(predicates.iter().cloned());
        return Ok(());
    }
    let type_params: BTreeSet<_> = generics
        .type_params()
        .map(|param| param.ident.to_string())
        .collect();
    if shape_attrs.serialize_with().is_some() {
        return Ok(());
    }
    if container.attrs.remote().is_some() {
        return Ok(());
    }
    if let Some(ty) = container.attrs.type_into() {
        if type_uses_params(ty, &type_params) {
            generics
                .make_where_clause()
                .predicates
                .push(parse_quote!(#ty: __serde_shape::SerializeShape));
        }
        return Ok(());
    }

    let mut field_bound_types = Vec::new();

    match &container.data {
        ast::Data::Struct(_, fields) => {
            collect_serialize_field_bound_types(fields, &type_params, &mut field_bound_types)?;
        }
        ast::Data::Enum(variants) => {
            for variant in variants {
                let variant_shape_attrs = ShapeAttrs::parse(&variant.original.attrs)?;
                if variant.attrs.skip_serializing()
                    || variant.attrs.serialize_with().is_some()
                    || variant_shape_attrs.serialize_with().is_some()
                {
                    continue;
                }
                collect_serialize_field_bound_types(
                    &variant.fields,
                    &type_params,
                    &mut field_bound_types,
                )?;
            }
        }
    }

    for ty in field_bound_types {
        generics
            .make_where_clause()
            .predicates
            .push(parse_quote!(#ty: __serde_shape::SerializeShape));
    }
    Ok(())
}

fn add_deserialize_shape_bounds(
    generics: &mut syn::Generics,
    container: &ast::Container<'_>,
    shape_attrs: &ShapeAttrs,
) -> syn::Result<()> {
    if let Some(predicates) = shape_attrs.deserialize_bound() {
        generics
            .make_where_clause()
            .predicates
            .extend(predicates.iter().cloned());
        return Ok(());
    }
    let type_params: BTreeSet<_> = generics
        .type_params()
        .map(|param| param.ident.to_string())
        .collect();
    if shape_attrs.deserialize_with().is_some() {
        return Ok(());
    }
    if container.attrs.remote().is_some() {
        return Ok(());
    }
    if let Some(ty) = container
        .attrs
        .type_from()
        .or_else(|| container.attrs.type_try_from())
    {
        if type_uses_params(ty, &type_params) {
            generics
                .make_where_clause()
                .predicates
                .push(parse_quote!(#ty: __serde_shape::DeserializeShape));
        }
        return Ok(());
    }

    let mut field_bound_types = Vec::new();

    match &container.data {
        ast::Data::Struct(_, fields) => {
            collect_deserialize_field_bound_types(fields, &type_params, &mut field_bound_types)?;
        }
        ast::Data::Enum(variants) => {
            for variant in variants {
                let variant_shape_attrs = ShapeAttrs::parse(&variant.original.attrs)?;
                if variant.attrs.skip_deserializing()
                    || variant.attrs.deserialize_with().is_some()
                    || variant_shape_attrs.deserialize_with().is_some()
                {
                    continue;
                }
                collect_deserialize_field_bound_types(
                    &variant.fields,
                    &type_params,
                    &mut field_bound_types,
                )?;
            }
        }
    }

    for ty in field_bound_types {
        generics
            .make_where_clause()
            .predicates
            .push(parse_quote!(#ty: __serde_shape::DeserializeShape));
    }
    Ok(())
}

fn collect_serialize_field_bound_types(
    fields: &[ast::Field<'_>],
    type_params: &BTreeSet<String>,
    field_bound_types: &mut Vec<Type>,
) -> syn::Result<()> {
    for field in fields {
        let shape_attrs = ShapeAttrs::parse(&field.original.attrs)?;
        if field.attrs.skip_serializing() {
            continue;
        }
        if shape_attrs.serialize_with().is_none() && field.attrs.serialize_with().is_none() {
            collect_shape_bound_types(field.ty, type_params, field_bound_types);
        }
    }
    Ok(())
}

fn collect_deserialize_field_bound_types(
    fields: &[ast::Field<'_>],
    type_params: &BTreeSet<String>,
    field_bound_types: &mut Vec<Type>,
) -> syn::Result<()> {
    for field in fields {
        let shape_attrs = ShapeAttrs::parse(&field.original.attrs)?;
        if field.attrs.skip_deserializing() {
            continue;
        }
        if shape_attrs.deserialize_with().is_none() && field.attrs.deserialize_with().is_none() {
            collect_shape_bound_types(field.ty, type_params, field_bound_types);
        }
    }
    Ok(())
}

fn collect_shape_bound_types(
    ty: &Type,
    type_params: &BTreeSet<String>,
    field_bound_types: &mut Vec<Type>,
) {
    match ty {
        Type::Array(ty) => collect_shape_bound_types(&ty.elem, type_params, field_bound_types),
        Type::FnPtr(ty) => {
            for input in &ty.inputs {
                collect_shape_bound_types(&input.ty, type_params, field_bound_types);
            }
            collect_return_type_params(&ty.output, type_params, field_bound_types);
        }
        Type::Group(ty) => collect_shape_bound_types(&ty.elem, type_params, field_bound_types),
        Type::ImplTrait(ty) => {
            collect_type_param_bounds(&ty.bounds, type_params, field_bound_types);
        }
        Type::Paren(ty) => collect_shape_bound_types(&ty.elem, type_params, field_bound_types),
        Type::Path(ty) => {
            if ty
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "PhantomData")
            {
                return;
            }

            let is_associated_type = ty.qself.as_ref().is_some_and(|qself| {
                let mut qself_bounds = Vec::new();
                collect_shape_bound_types(&qself.ty, type_params, &mut qself_bounds);
                !qself_bounds.is_empty()
            }) || (ty.path.leading_colon.is_none()
                && ty.path.segments.len() > 1
                && ty
                    .path
                    .segments
                    .first()
                    .is_some_and(|segment| type_params.contains(&segment.ident.to_string())));

            if is_associated_type {
                push_bound_type(field_bound_types, Type::Path(ty.clone()));
                return;
            }

            if ty.qself.is_none()
                && ty.path.leading_colon.is_none()
                && ty.path.segments.len() == 1
                && ty
                    .path
                    .segments
                    .first()
                    .is_some_and(|segment| type_params.contains(&segment.ident.to_string()))
            {
                push_bound_type(field_bound_types, Type::Path(ty.clone()));
                return;
            }

            if let Some(qself) = &ty.qself {
                collect_shape_bound_types(&qself.ty, type_params, field_bound_types);
            }

            for segment in &ty.path.segments {
                collect_path_arguments(&segment.arguments, type_params, field_bound_types);
            }
        }
        Type::Ptr(ty) => collect_shape_bound_types(&ty.elem, type_params, field_bound_types),
        Type::Reference(ty) => collect_shape_bound_types(&ty.elem, type_params, field_bound_types),
        Type::Slice(ty) => collect_shape_bound_types(&ty.elem, type_params, field_bound_types),
        Type::TraitObject(ty) => {
            collect_type_param_bounds(&ty.bounds, type_params, field_bound_types);
        }
        Type::Tuple(ty) => {
            for elem in &ty.elems {
                collect_shape_bound_types(elem, type_params, field_bound_types);
            }
        }
        Type::Infer(_) | Type::Macro(_) | Type::Never(_) | Type::Verbatim(_) => {}
        _ => {}
    }
}

fn type_uses_params(ty: &Type, type_params: &BTreeSet<String>) -> bool {
    let mut bound_types = Vec::new();
    collect_shape_bound_types(ty, type_params, &mut bound_types);
    !bound_types.is_empty()
}

fn collect_path_arguments(
    arguments: &PathArguments,
    type_params: &BTreeSet<String>,
    field_bound_types: &mut Vec<Type>,
) {
    match arguments {
        PathArguments::None => {}
        PathArguments::AngleBracketed(arguments) => {
            for argument in &arguments.args {
                match argument {
                    GenericArgument::Type(ty) => {
                        collect_shape_bound_types(ty, type_params, field_bound_types);
                    }
                    GenericArgument::AssocType(assoc) => {
                        collect_shape_bound_types(&assoc.ty, type_params, field_bound_types);
                    }
                    GenericArgument::Constraint(constraint) => {
                        collect_type_param_bounds(
                            &constraint.bounds,
                            type_params,
                            field_bound_types,
                        );
                    }
                    GenericArgument::Lifetime(_)
                    | GenericArgument::Const(_)
                    | GenericArgument::AssocConst(_) => {}
                    _ => {}
                }
            }
        }
        PathArguments::Parenthesized(arguments) => {
            for input in &arguments.inputs {
                collect_shape_bound_types(&input.ty, type_params, field_bound_types);
            }
            collect_return_type_params(&arguments.output, type_params, field_bound_types);
        }
    }
}

fn collect_type_param_bounds(
    bounds: &syn::punctuated::Punctuated<TypeParamBound, syn::Token![+]>,
    type_params: &BTreeSet<String>,
    field_bound_types: &mut Vec<Type>,
) {
    for bound in bounds {
        if let TypeParamBound::Trait(bound) = bound {
            for segment in &bound.path.segments {
                collect_path_arguments(&segment.arguments, type_params, field_bound_types);
            }
        }
    }
}

fn collect_return_type_params(
    return_type: &ReturnType,
    type_params: &BTreeSet<String>,
    field_bound_types: &mut Vec<Type>,
) {
    if let ReturnType::Type(_, ty) = return_type {
        collect_shape_bound_types(ty, type_params, field_bound_types);
    }
}

fn push_bound_type(field_bound_types: &mut Vec<Type>, ty: Type) {
    let tokens = ty.to_token_stream().to_string();
    if field_bound_types
        .iter()
        .all(|existing| existing.to_token_stream().to_string() != tokens)
    {
        field_bound_types.push(ty);
    }
}

fn serialize_shape_body(
    container: &ast::Container<'_>,
    shape_attrs: &ShapeAttrs,
) -> syn::Result<TokenStream2> {
    if let Some(function) = shape_attrs.serialize_with() {
        return Ok(quote!(#function(context)));
    }
    if let Some(ty) = container.attrs.type_into() {
        return Ok(quote!(<#ty as __serde_shape::SerializeShape>::serialize_shape_in(context)));
    }

    let name = lit_name(container.attrs.name().serialize_name());
    let description = description(&container.original.attrs);
    let description = option_lit(description.as_deref());
    let kind = serialize_definition_kind(container)?;

    Ok(quote! {
        context.define_named_type_with_description(
            __serde_shape::TypeName::of::<Self>(#name),
            #description,
            |context| {
                #kind
            },
        )
    })
}

fn deserialize_shape_body(
    container: &ast::Container<'_>,
    shape_attrs: &ShapeAttrs,
) -> syn::Result<TokenStream2> {
    if let Some(function) = shape_attrs.deserialize_with() {
        return Ok(quote!(#function(context)));
    }
    if let Some(ty) = container
        .attrs
        .type_from()
        .or_else(|| container.attrs.type_try_from())
    {
        return Ok(quote!(<#ty as __serde_shape::DeserializeShape>::deserialize_shape_in(context)));
    }

    let name = lit_name(container.attrs.name().deserialize_name());
    let description = description(&container.original.attrs);
    let description = option_lit(description.as_deref());
    let kind = deserialize_definition_kind(container)?;

    Ok(quote! {
        context.define_named_type_with_description(
            __serde_shape::TypeName::of::<Self>(#name),
            #description,
            |context| {
                #kind
            },
        )
    })
}

fn serialize_definition_kind(container: &ast::Container<'_>) -> syn::Result<TokenStream2> {
    if let Some(path) = container.attrs.remote() {
        let opaque = remote_opaque_shape(path);
        return Ok(quote!(__serde_shape::SerializeDefinitionKind::Opaque(#opaque)));
    }

    let attributes = serialize_container_attributes(&container.attrs);
    Ok(match &container.data {
        ast::Data::Struct(style, fields) => {
            let style = fields_style(*style);
            let fields = fields
                .iter()
                .map(serialize_field_shape)
                .collect::<syn::Result<Vec<_>>>()?;
            quote! {
                __serde_shape::SerializeDefinitionKind::Struct(__serde_shape::SerializeStructShape {
                    style: #style,
                    fields: __serde_shape::__private::vec![#(#fields),*],
                    attributes: #attributes,
                })
            }
        }
        ast::Data::Enum(variants) => {
            let repr = tagging(container.attrs.tag());
            let variants = variants
                .iter()
                .map(serialize_variant_shape)
                .collect::<syn::Result<Vec<_>>>()?;
            quote! {
                __serde_shape::SerializeDefinitionKind::Enum(__serde_shape::SerializeEnumShape {
                    repr: #repr,
                    variants: __serde_shape::__private::vec![#(#variants),*],
                    attributes: #attributes,
                })
            }
        }
    })
}

fn deserialize_definition_kind(container: &ast::Container<'_>) -> syn::Result<TokenStream2> {
    if let Some(path) = container.attrs.remote() {
        let opaque = remote_opaque_shape(path);
        return Ok(quote!(__serde_shape::DeserializeDefinitionKind::Opaque(#opaque)));
    }

    let attributes = deserialize_container_attributes(&container.attrs);
    Ok(match &container.data {
        ast::Data::Struct(style, fields) => {
            let style = fields_style(*style);
            let fields = fields
                .iter()
                .map(deserialize_field_shape)
                .collect::<syn::Result<Vec<_>>>()?;
            quote! {
                __serde_shape::DeserializeDefinitionKind::Struct(__serde_shape::DeserializeStructShape {
                    style: #style,
                    fields: __serde_shape::__private::vec![#(#fields),*],
                    attributes: #attributes,
                })
            }
        }
        ast::Data::Enum(variants) => {
            let repr = deserialize_tagging(&container.attrs);
            let variants = variants
                .iter()
                .map(deserialize_variant_shape)
                .collect::<syn::Result<Vec<_>>>()?;
            quote! {
                __serde_shape::DeserializeDefinitionKind::Enum(__serde_shape::DeserializeEnumShape {
                    repr: #repr,
                    variants: __serde_shape::__private::vec![#(#variants),*],
                    attributes: #attributes,
                })
            }
        }
    })
}

fn remote_opaque_shape<T>(detail: T) -> TokenStream2
where
    T: ToTokens,
{
    let detail = lit(detail.to_token_stream().to_string());

    quote! {
        __serde_shape::OpaqueShape {
            type_name: ::core::any::type_name::<Self>(),
            reason: __serde_shape::OpaqueReason::Remote,
            detail: ::core::option::Option::Some(#detail),
        }
    }
}

fn serialize_container_attributes(attrs: &attr::Container) -> TokenStream2 {
    let non_exhaustive = attrs.non_exhaustive();

    quote! {
        __serde_shape::SerializeContainerAttributes {
            non_exhaustive: #non_exhaustive,
        }
    }
}

fn deserialize_container_attributes(attrs: &attr::Container) -> TokenStream2 {
    let deny_unknown_fields = attrs.deny_unknown_fields();
    let default = default_shape(attrs.default());
    let expecting = option_lit(attrs.expecting());
    let non_exhaustive = attrs.non_exhaustive();

    quote! {
        __serde_shape::DeserializeContainerAttributes {
            deny_unknown_fields: #deny_unknown_fields,
            default: #default,
            expecting: #expecting,
            non_exhaustive: #non_exhaustive,
        }
    }
}

fn serialize_variant_shape(variant: &ast::Variant<'_>) -> syn::Result<TokenStream2> {
    let shape_attrs = ShapeAttrs::parse(&variant.original.attrs)?;
    let rust_name = lit(variant.ident.to_string());
    let name = lit_name(variant.attrs.name().serialize_name());
    let description = description(&variant.original.attrs);
    let description = option_lit(description.as_deref());
    let style = fields_style(variant.style);
    let skip = variant.attrs.skip_serializing();
    let untagged = variant.attrs.untagged();
    let content = if skip {
        quote!(__serde_shape::SerializeVariantContent::Omitted)
    } else if let Some(function) = shape_attrs.serialize_with() {
        quote!(__serde_shape::SerializeVariantContent::Shape(#function(context)))
    } else if let Some(custom_serializer) = variant.attrs.serialize_with() {
        let detail = option_path(Some(custom_serializer));
        quote! {
            __serde_shape::SerializeVariantContent::Custom(__serde_shape::OpaqueShape {
                type_name: ::core::any::type_name::<Self>(),
                reason: __serde_shape::OpaqueReason::CustomSerializer,
                detail: #detail,
            })
        }
    } else {
        let fields = variant
            .fields
            .iter()
            .map(serialize_field_shape)
            .collect::<syn::Result<Vec<_>>>()?;
        quote! {
            __serde_shape::SerializeVariantContent::Fields(
                __serde_shape::__private::vec![#(#fields),*],
            )
        }
    };

    Ok(quote! {
        __serde_shape::SerializeVariantShape {
            rust_name: #rust_name,
            name: #name,
            description: #description,
            style: #style,
            content: #content,
            untagged: #untagged,
        }
    })
}

fn deserialize_variant_shape(variant: &ast::Variant<'_>) -> syn::Result<TokenStream2> {
    let shape_attrs = ShapeAttrs::parse(&variant.original.attrs)?;
    let rust_name = lit(variant.ident.to_string());
    let name = lit_name(variant.attrs.name().deserialize_name());
    let aliases = aliases(variant.attrs.aliases());
    let description = description(&variant.original.attrs);
    let description = option_lit(description.as_deref());
    let style = fields_style(variant.style);
    let skip = variant.attrs.skip_deserializing();
    let other = variant.attrs.other();
    let untagged = variant.attrs.untagged();
    let content = if skip {
        quote!(__serde_shape::DeserializeVariantContent::Omitted)
    } else if let Some(function) = shape_attrs.deserialize_with() {
        quote!(__serde_shape::DeserializeVariantContent::Shape(#function(context)))
    } else if let Some(custom_deserializer) = variant.attrs.deserialize_with() {
        let detail = option_path(Some(custom_deserializer));
        quote! {
            __serde_shape::DeserializeVariantContent::Custom(__serde_shape::OpaqueShape {
                type_name: ::core::any::type_name::<Self>(),
                reason: __serde_shape::OpaqueReason::CustomDeserializer,
                detail: #detail,
            })
        }
    } else {
        let fields = variant
            .fields
            .iter()
            .map(deserialize_field_shape)
            .collect::<syn::Result<Vec<_>>>()?;
        quote! {
            __serde_shape::DeserializeVariantContent::Fields(
                __serde_shape::__private::vec![#(#fields),*],
            )
        }
    };

    Ok(quote! {
        __serde_shape::DeserializeVariantShape {
            rust_name: #rust_name,
            name: #name,
            aliases: #aliases,
            description: #description,
            style: #style,
            content: #content,
            other: #other,
            untagged: #untagged,
        }
    })
}

fn serialize_field_shape(field: &ast::Field<'_>) -> syn::Result<TokenStream2> {
    let shape_attrs = ShapeAttrs::parse(&field.original.attrs)?;
    let member = field_member(&field.member);
    let name = lit_name(field.attrs.name().serialize_name());
    let description = description(&field.original.attrs);
    let description = option_lit(description.as_deref());
    let skip = field.attrs.skip_serializing();
    let skip_if = option_path(field.attrs.skip_serializing_if());
    let flatten = field.attrs.flatten();
    let transparent = field.attrs.transparent();
    let ty = field.ty;
    let wire_shape = if skip {
        quote!(__serde_shape::FieldWireShape::Omitted)
    } else {
        let value_shape = if let Some(function) = shape_attrs.serialize_with() {
            quote!(#function(context))
        } else if let Some(custom_serializer) = field.attrs.serialize_with() {
            let detail = option_path(Some(custom_serializer));
            quote! {
                __serde_shape::ShapeRef::Opaque(__serde_shape::OpaqueShape {
                    type_name: ::core::any::type_name::<#ty>(),
                    reason: __serde_shape::OpaqueReason::CustomSerializer,
                    detail: #detail,
                })
            }
        } else {
            quote!(<#ty as __serde_shape::SerializeShape>::serialize_shape_in(context))
        };

        if transparent {
            quote!(__serde_shape::FieldWireShape::Inline(#value_shape))
        } else if flatten {
            quote!(__serde_shape::FieldWireShape::Flatten(#value_shape))
        } else {
            quote!(__serde_shape::FieldWireShape::Value(#value_shape))
        }
    };

    Ok(quote! {
        __serde_shape::SerializeFieldShape {
            member: #member,
            name: #name,
            description: #description,
            wire_shape: #wire_shape,
            skip_if: #skip_if,
        }
    })
}

fn deserialize_field_shape(field: &ast::Field<'_>) -> syn::Result<TokenStream2> {
    let shape_attrs = ShapeAttrs::parse(&field.original.attrs)?;
    let borrowed_cow_shape = borrowed_cow_shape(field)?;
    let member = field_member(&field.member);
    let name = lit_name(field.attrs.name().deserialize_name());
    let aliases = aliases(field.attrs.aliases());
    let description = description(&field.original.attrs);
    let description = option_lit(description.as_deref());
    let skip = field.attrs.skip_deserializing();
    let default = default_shape(field.attrs.default());
    let flatten = field.attrs.flatten();
    let transparent = field.attrs.transparent();
    let ty = field.ty;
    let wire_shape = if skip {
        quote!(__serde_shape::FieldWireShape::Omitted)
    } else {
        let value_shape = if let Some(function) = shape_attrs.deserialize_with() {
            quote!(#function(context))
        } else if let Some(shape) = borrowed_cow_shape {
            shape
        } else if let Some(custom_deserializer) = field.attrs.deserialize_with() {
            let detail = option_path(Some(custom_deserializer));
            quote! {
                __serde_shape::ShapeRef::Opaque(__serde_shape::OpaqueShape {
                    type_name: ::core::any::type_name::<#ty>(),
                    reason: __serde_shape::OpaqueReason::CustomDeserializer,
                    detail: #detail,
                })
            }
        } else {
            quote!(<#ty as __serde_shape::DeserializeShape>::deserialize_shape_in(context))
        };

        if transparent {
            quote!(__serde_shape::FieldWireShape::Inline(#value_shape))
        } else if flatten {
            quote!(__serde_shape::FieldWireShape::Flatten(#value_shape))
        } else {
            quote!(__serde_shape::FieldWireShape::Value(#value_shape))
        }
    };

    Ok(quote! {
        __serde_shape::DeserializeFieldShape {
            member: #member,
            name: #name,
            aliases: #aliases,
            description: #description,
            wire_shape: #wire_shape,
            default: #default,
        }
    })
}

fn field_member(member: &Member) -> TokenStream2 {
    match member {
        Member::Named(ident) => {
            let ident = lit(ident.to_string());
            quote!(__serde_shape::FieldMember::Named(#ident))
        }
        Member::Unnamed(index) => {
            let index = index.index as usize;
            quote!(__serde_shape::FieldMember::Unnamed(#index))
        }
    }
}

fn fields_style(style: ast::Style) -> TokenStream2 {
    match style {
        ast::Style::Struct => quote!(__serde_shape::FieldsStyle::Struct),
        ast::Style::Tuple => quote!(__serde_shape::FieldsStyle::Tuple),
        ast::Style::Newtype => quote!(__serde_shape::FieldsStyle::Newtype),
        ast::Style::Unit => quote!(__serde_shape::FieldsStyle::Unit),
    }
}

fn tagging(tag: &attr::TagType) -> TokenStream2 {
    match tag {
        attr::TagType::External => quote!(__serde_shape::Tagging::External),
        attr::TagType::Internal { tag } => {
            let tag = lit(tag);
            quote!(__serde_shape::Tagging::Internal { tag: #tag })
        }
        attr::TagType::Adjacent { tag, content } => {
            let tag = lit(tag);
            let content = lit(content);
            quote!(__serde_shape::Tagging::Adjacent {
                tag: #tag,
                content: #content,
            })
        }
        attr::TagType::None => quote!(__serde_shape::Tagging::Untagged),
    }
}

fn deserialize_tagging(attrs: &attr::Container) -> TokenStream2 {
    match attrs.identifier() {
        attr::Identifier::No => tagging(attrs.tag()),
        attr::Identifier::Field => quote!(__serde_shape::Tagging::FieldIdentifier),
        attr::Identifier::Variant => quote!(__serde_shape::Tagging::VariantIdentifier),
    }
}

fn default_shape(default: &attr::Default) -> TokenStream2 {
    match default {
        attr::Default::None => quote!(__serde_shape::DefaultShape::None),
        attr::Default::Default => quote!(__serde_shape::DefaultShape::Default),
        attr::Default::Path(path) => {
            let path = lit(path.to_token_stream().to_string().replace(' ', ""));
            quote!(__serde_shape::DefaultShape::Path(#path))
        }
    }
}

fn aliases(aliases: &BTreeSet<Name>) -> TokenStream2 {
    let aliases = aliases.iter().map(lit_name);
    quote!(__serde_shape::__private::vec![#(#aliases),*])
}

fn borrowed_cow_shape(field: &ast::Field<'_>) -> syn::Result<Option<TokenStream2>> {
    // serde_derive_internals models borrowed Cow fields as custom deserializers internally. Read
    // the source-level contract instead, so this derive does not depend on Serde's private helper
    // path. An explicit user deserializer still takes precedence over the built-in Cow behavior.
    if field.attrs.borrowed_lifetimes().is_empty()
        || has_explicit_serde_deserializer(&field.original.attrs)?
    {
        return Ok(None);
    }

    let Some(element) = cow_element_type(field.ty) else {
        return Ok(None);
    };
    if is_primitive_type(element, "str") {
        Ok(Some(quote!(__serde_shape::ShapeRef::String)))
    } else if is_byte_slice(element) {
        Ok(Some(quote!(__serde_shape::ShapeRef::Bytes)))
    } else {
        Ok(None)
    }
}

fn has_explicit_serde_deserializer(attrs: &[syn::Attribute]) -> syn::Result<bool> {
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("serde")) {
        let metas = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        )?;
        if metas
            .iter()
            .any(|meta| meta.path().is_ident("deserialize_with") || meta.path().is_ident("with"))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn cow_element_type(ty: &Type) -> Option<&Type> {
    let Type::Path(ty) = ungroup(ty) else {
        return None;
    };
    let segment = ty.path.segments.last()?;
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let mut arguments = arguments.args.iter();
    match (arguments.next(), arguments.next(), arguments.next()) {
        (Some(GenericArgument::Lifetime(_)), Some(GenericArgument::Type(element)), None)
            if segment.ident == "Cow" =>
        {
            Some(element)
        }
        _ => None,
    }
}

fn is_byte_slice(ty: &Type) -> bool {
    match ungroup(ty) {
        Type::Slice(slice) => is_primitive_type(&slice.elem, "u8"),
        _ => false,
    }
}

fn is_primitive_type(ty: &Type, name: &str) -> bool {
    let Type::Path(ty) = ungroup(ty) else {
        return false;
    };
    ty.qself.is_none()
        && ty.path.leading_colon.is_none()
        && ty.path.segments.len() == 1
        && ty.path.segments[0].ident == name
        && ty.path.segments[0].arguments.is_empty()
}

fn lit_name(value: &Name) -> LitStr {
    LitStr::new(&value.value, value.span)
}

fn option_lit(value: Option<&str>) -> TokenStream2 {
    match value {
        Some(value) => {
            let value = lit(value);
            quote!(::core::option::Option::Some(#value))
        }
        None => quote!(::core::option::Option::None),
    }
}

fn option_path(value: Option<&syn::ExprPath>) -> TokenStream2 {
    match value {
        Some(value) => {
            let value = lit(value.to_token_stream().to_string().replace(' ', ""));
            quote!(::core::option::Option::Some(#value))
        }
        None => quote!(::core::option::Option::None),
    }
}

fn lit(value: impl AsRef<str>) -> LitStr {
    LitStr::new(value.as_ref(), proc_macro2::Span::call_site())
}
