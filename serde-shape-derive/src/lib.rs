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
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use quote::quote;
use serde_derive_internals::Ctxt;
use serde_derive_internals::Derive;
use serde_derive_internals::ast;
use serde_derive_internals::attr;
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

/// Derive `serde_shape::SerializeShape` from Serde serialize metadata.
#[proc_macro_derive(SerializeShape, attributes(serde))]
pub fn derive_serialize_shape(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand_serialize_shape(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive `serde_shape::DeserializeShape` from Serde deserialize metadata.
#[proc_macro_derive(DeserializeShape, attributes(serde))]
pub fn derive_deserialize_shape(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand_deserialize_shape(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_serialize_shape(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let container = parse_container(input, Derive::Serialize)?;
    let ident = &input.ident;
    let mut generics = input.generics.clone();
    add_serialize_shape_bounds(&mut generics, &container);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let body = serialize_shape_body(&container);

    Ok(quote! {
        impl #impl_generics ::serde_shape::SerializeShape for #ident #ty_generics #where_clause {
            fn serialize_shape_in(
                context: &mut ::serde_shape::SerializeShapeContext,
            ) -> ::serde_shape::ShapeRef {
                #body
            }
        }
    })
}

fn expand_deserialize_shape(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let container = parse_container(input, Derive::Deserialize)?;
    let ident = &input.ident;
    let mut generics = input.generics.clone();
    add_deserialize_shape_bounds(&mut generics, &container);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let body = deserialize_shape_body(&container);

    Ok(quote! {
        impl #impl_generics ::serde_shape::DeserializeShape for #ident #ty_generics #where_clause {
            fn deserialize_shape_in(
                context: &mut ::serde_shape::DeserializeShapeContext,
            ) -> ::serde_shape::ShapeRef {
                #body
            }
        }
    })
}

fn parse_container<'a>(input: &'a DeriveInput, derive: Derive) -> syn::Result<ast::Container<'a>> {
    let cx = Ctxt::new();
    let Some(container) = ast::Container::from_ast(&cx, input, derive) else {
        cx.check()?;
        return Err(syn::Error::new_spanned(
            input,
            "serde-shape could not parse this item",
        ));
    };
    cx.check()?;
    Ok(container)
}

fn add_serialize_shape_bounds(generics: &mut syn::Generics, container: &ast::Container<'_>) {
    if container.attrs.type_into().is_some() || container.attrs.remote().is_some() {
        return;
    }

    let type_params: BTreeSet<_> = generics
        .type_params()
        .map(|param| param.ident.to_string())
        .collect();
    let mut field_bound_types = Vec::new();

    match &container.data {
        ast::Data::Struct(_, fields) => {
            collect_serialize_field_bound_types(fields, &type_params, &mut field_bound_types);
        }
        ast::Data::Enum(variants) => {
            for variant in variants {
                if variant.attrs.skip_serializing() || variant.attrs.serialize_with().is_some() {
                    continue;
                }
                collect_serialize_field_bound_types(
                    &variant.fields,
                    &type_params,
                    &mut field_bound_types,
                );
            }
        }
    }

    for ty in field_bound_types {
        generics
            .make_where_clause()
            .predicates
            .push(parse_quote!(#ty: ::serde_shape::SerializeShape));
    }
}

fn add_deserialize_shape_bounds(generics: &mut syn::Generics, container: &ast::Container<'_>) {
    if container.attrs.type_from().is_some()
        || container.attrs.type_try_from().is_some()
        || container.attrs.remote().is_some()
    {
        return;
    }

    let type_params: BTreeSet<_> = generics
        .type_params()
        .map(|param| param.ident.to_string())
        .collect();
    let mut field_bound_types = Vec::new();

    match &container.data {
        ast::Data::Struct(_, fields) => {
            collect_deserialize_field_bound_types(fields, &type_params, &mut field_bound_types);
        }
        ast::Data::Enum(variants) => {
            for variant in variants {
                if variant.attrs.skip_deserializing() || variant.attrs.deserialize_with().is_some()
                {
                    continue;
                }
                collect_deserialize_field_bound_types(
                    &variant.fields,
                    &type_params,
                    &mut field_bound_types,
                );
            }
        }
    }

    for ty in field_bound_types {
        generics
            .make_where_clause()
            .predicates
            .push(parse_quote!(#ty: ::serde_shape::DeserializeShape));
    }
}

fn collect_serialize_field_bound_types(
    fields: &[ast::Field<'_>],
    type_params: &BTreeSet<String>,
    field_bound_types: &mut Vec<Type>,
) {
    for field in fields {
        if field.attrs.skip_serializing() || field.attrs.serialize_with().is_some() {
            continue;
        }
        collect_field_bound_type(field, type_params, field_bound_types);
    }
}

fn collect_deserialize_field_bound_types(
    fields: &[ast::Field<'_>],
    type_params: &BTreeSet<String>,
    field_bound_types: &mut Vec<Type>,
) {
    for field in fields {
        if field.attrs.skip_deserializing() || field.attrs.deserialize_with().is_some() {
            continue;
        }
        collect_field_bound_type(field, type_params, field_bound_types);
    }
}

fn collect_field_bound_type(
    field: &ast::Field<'_>,
    type_params: &BTreeSet<String>,
    field_bound_types: &mut Vec<Type>,
) {
    let mut used_type_params = BTreeSet::new();
    collect_type_params(field.ty, type_params, &mut used_type_params);
    if !used_type_params.is_empty() {
        field_bound_types.push((*field.ty).clone());
    }
}

fn collect_type_params(
    ty: &Type,
    type_params: &BTreeSet<String>,
    used_type_params: &mut BTreeSet<String>,
) {
    match ty {
        Type::Array(ty) => collect_type_params(&ty.elem, type_params, used_type_params),
        Type::BareFn(ty) => {
            for input in &ty.inputs {
                collect_type_params(&input.ty, type_params, used_type_params);
            }
            collect_return_type_params(&ty.output, type_params, used_type_params);
        }
        Type::Group(ty) => collect_type_params(&ty.elem, type_params, used_type_params),
        Type::ImplTrait(ty) => collect_type_param_bounds(&ty.bounds, type_params, used_type_params),
        Type::Paren(ty) => collect_type_params(&ty.elem, type_params, used_type_params),
        Type::Path(ty) => {
            if let Some(qself) = &ty.qself {
                collect_type_params(&qself.ty, type_params, used_type_params);
            }
            for segment in &ty.path.segments {
                let ident = segment.ident.to_string();
                if type_params.contains(&ident) {
                    used_type_params.insert(ident);
                }
                collect_path_arguments(&segment.arguments, type_params, used_type_params);
            }
        }
        Type::Ptr(ty) => collect_type_params(&ty.elem, type_params, used_type_params),
        Type::Reference(ty) => collect_type_params(&ty.elem, type_params, used_type_params),
        Type::Slice(ty) => collect_type_params(&ty.elem, type_params, used_type_params),
        Type::TraitObject(ty) => {
            collect_type_param_bounds(&ty.bounds, type_params, used_type_params);
        }
        Type::Tuple(ty) => {
            for elem in &ty.elems {
                collect_type_params(elem, type_params, used_type_params);
            }
        }
        Type::Infer(_) | Type::Macro(_) | Type::Never(_) | Type::Verbatim(_) => {}
        _ => {}
    }
}

fn collect_path_arguments(
    arguments: &PathArguments,
    type_params: &BTreeSet<String>,
    used_type_params: &mut BTreeSet<String>,
) {
    match arguments {
        PathArguments::None => {}
        PathArguments::AngleBracketed(arguments) => {
            for argument in &arguments.args {
                match argument {
                    GenericArgument::Type(ty) => {
                        collect_type_params(ty, type_params, used_type_params);
                    }
                    GenericArgument::AssocType(assoc) => {
                        collect_type_params(&assoc.ty, type_params, used_type_params);
                    }
                    GenericArgument::Constraint(constraint) => {
                        collect_type_param_bounds(
                            &constraint.bounds,
                            type_params,
                            used_type_params,
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
                collect_type_params(input, type_params, used_type_params);
            }
            collect_return_type_params(&arguments.output, type_params, used_type_params);
        }
    }
}

fn collect_type_param_bounds(
    bounds: &syn::punctuated::Punctuated<TypeParamBound, syn::Token![+]>,
    type_params: &BTreeSet<String>,
    used_type_params: &mut BTreeSet<String>,
) {
    for bound in bounds {
        if let TypeParamBound::Trait(bound) = bound {
            for segment in &bound.path.segments {
                collect_path_arguments(&segment.arguments, type_params, used_type_params);
            }
        }
    }
}

fn collect_return_type_params(
    return_type: &ReturnType,
    type_params: &BTreeSet<String>,
    used_type_params: &mut BTreeSet<String>,
) {
    if let ReturnType::Type(_, ty) = return_type {
        collect_type_params(ty, type_params, used_type_params);
    }
}

fn serialize_shape_body(container: &ast::Container<'_>) -> TokenStream2 {
    let name = lit(container.attrs.name().serialize_name());
    let kind = serialize_definition_kind(container);

    quote! {
        context.define_named_type(
            ::serde_shape::SerializeTypeName {
                rust_name: ::core::any::type_name::<Self>(),
                name: #name,
            },
            |context| {
                #kind
            },
        )
    }
}

fn deserialize_shape_body(container: &ast::Container<'_>) -> TokenStream2 {
    let name = lit(container.attrs.name().deserialize_name());
    let kind = deserialize_definition_kind(container);

    quote! {
        context.define_named_type(
            ::serde_shape::DeserializeTypeName {
                rust_name: ::core::any::type_name::<Self>(),
                name: #name,
            },
            |context| {
                #kind
            },
        )
    }
}

fn serialize_definition_kind(container: &ast::Container<'_>) -> TokenStream2 {
    if let Some(ty) = container.attrs.type_into() {
        return serialize_opaque_definition("IntoType", ty);
    }
    if let Some(path) = container.attrs.remote() {
        return serialize_opaque_definition("Remote", path);
    }

    let attributes = serialize_container_attributes(&container.attrs);
    match &container.data {
        ast::Data::Struct(style, fields) => {
            let style = fields_style(*style);
            let fields = fields.iter().map(serialize_field_shape);
            quote! {
                ::serde_shape::SerializeDefinitionKind::Struct(::serde_shape::SerializeStructShape {
                    style: #style,
                    fields: ::serde_shape::__private::vec![#(#fields),*],
                    attributes: #attributes,
                })
            }
        }
        ast::Data::Enum(variants) => {
            let repr = tagging(container.attrs.tag());
            let variants = variants.iter().map(serialize_variant_shape);
            quote! {
                ::serde_shape::SerializeDefinitionKind::Enum(::serde_shape::SerializeEnumShape {
                    repr: #repr,
                    variants: ::serde_shape::__private::vec![#(#variants),*],
                    attributes: #attributes,
                })
            }
        }
    }
}

fn deserialize_definition_kind(container: &ast::Container<'_>) -> TokenStream2 {
    if let Some(ty) = container.attrs.type_from() {
        return deserialize_opaque_definition("FromType", ty);
    }
    if let Some(ty) = container.attrs.type_try_from() {
        return deserialize_opaque_definition("TryFromType", ty);
    }
    if let Some(path) = container.attrs.remote() {
        return deserialize_opaque_definition("Remote", path);
    }

    let attributes = deserialize_container_attributes(&container.attrs);
    match &container.data {
        ast::Data::Struct(style, fields) => {
            let style = fields_style(*style);
            let fields = fields.iter().map(deserialize_field_shape);
            quote! {
                ::serde_shape::DeserializeDefinitionKind::Struct(::serde_shape::DeserializeStructShape {
                    style: #style,
                    fields: ::serde_shape::__private::vec![#(#fields),*],
                    attributes: #attributes,
                })
            }
        }
        ast::Data::Enum(variants) => {
            let repr = tagging(container.attrs.tag());
            let variants = variants.iter().map(deserialize_variant_shape);
            quote! {
                ::serde_shape::DeserializeDefinitionKind::Enum(::serde_shape::DeserializeEnumShape {
                    repr: #repr,
                    variants: ::serde_shape::__private::vec![#(#variants),*],
                    attributes: #attributes,
                })
            }
        }
    }
}

fn serialize_opaque_definition<T>(reason: &str, detail: T) -> TokenStream2
where
    T: ToTokens,
{
    let reason = opaque_reason(reason);
    let detail = lit(detail.to_token_stream().to_string());

    quote! {
        ::serde_shape::SerializeDefinitionKind::Opaque(::serde_shape::OpaqueShape {
            type_name: ::core::any::type_name::<Self>(),
            reason: #reason,
            detail: ::core::option::Option::Some(#detail),
        })
    }
}

fn deserialize_opaque_definition<T>(reason: &str, detail: T) -> TokenStream2
where
    T: ToTokens,
{
    let reason = opaque_reason(reason);
    let detail = lit(detail.to_token_stream().to_string());

    quote! {
        ::serde_shape::DeserializeDefinitionKind::Opaque(::serde_shape::OpaqueShape {
            type_name: ::core::any::type_name::<Self>(),
            reason: #reason,
            detail: ::core::option::Option::Some(#detail),
        })
    }
}

fn serialize_container_attributes(attrs: &attr::Container) -> TokenStream2 {
    let tagging = tagging(attrs.tag());
    let has_flatten = attrs.has_flatten();
    let transparent = attrs.transparent();
    let non_exhaustive = attrs.non_exhaustive();

    quote! {
        ::serde_shape::SerializeContainerAttributes {
            tagging: #tagging,
            has_flatten: #has_flatten,
            transparent: #transparent,
            non_exhaustive: #non_exhaustive,
        }
    }
}

fn deserialize_container_attributes(attrs: &attr::Container) -> TokenStream2 {
    let tagging = tagging(attrs.tag());
    let deny_unknown_fields = attrs.deny_unknown_fields();
    let default = default_shape(attrs.default());
    let has_flatten = attrs.has_flatten();
    let transparent = attrs.transparent();
    let expecting = option_lit(attrs.expecting());
    let non_exhaustive = attrs.non_exhaustive();

    quote! {
        ::serde_shape::DeserializeContainerAttributes {
            tagging: #tagging,
            deny_unknown_fields: #deny_unknown_fields,
            default: #default,
            has_flatten: #has_flatten,
            transparent: #transparent,
            expecting: #expecting,
            non_exhaustive: #non_exhaustive,
        }
    }
}

fn serialize_variant_shape(variant: &ast::Variant<'_>) -> TokenStream2 {
    let rust_name = lit(variant.ident.to_string());
    let name = lit(variant.attrs.name().serialize_name());
    let style = fields_style(variant.style);
    let skip = variant.attrs.skip_serializing();
    let custom_serializer = variant.attrs.serialize_with().is_some();
    let untagged = variant.attrs.untagged();
    let fields: Vec<_> = if skip || custom_serializer {
        Vec::new()
    } else {
        variant.fields.iter().map(serialize_field_shape).collect()
    };

    quote! {
        ::serde_shape::SerializeVariantShape {
            rust_name: #rust_name,
            name: #name,
            style: #style,
            fields: ::serde_shape::__private::vec![#(#fields),*],
            skip: #skip,
            custom_serializer: #custom_serializer,
            untagged: #untagged,
        }
    }
}

fn deserialize_variant_shape(variant: &ast::Variant<'_>) -> TokenStream2 {
    let rust_name = lit(variant.ident.to_string());
    let name = lit(variant.attrs.name().deserialize_name());
    let aliases = aliases(variant.attrs.aliases());
    let style = fields_style(variant.style);
    let skip = variant.attrs.skip_deserializing();
    let custom_deserializer = variant.attrs.deserialize_with().is_some();
    let other = variant.attrs.other();
    let untagged = variant.attrs.untagged();
    let fields: Vec<_> = if skip || custom_deserializer {
        Vec::new()
    } else {
        variant.fields.iter().map(deserialize_field_shape).collect()
    };

    quote! {
        ::serde_shape::DeserializeVariantShape {
            rust_name: #rust_name,
            name: #name,
            aliases: #aliases,
            style: #style,
            fields: ::serde_shape::__private::vec![#(#fields),*],
            skip: #skip,
            custom_deserializer: #custom_deserializer,
            other: #other,
            untagged: #untagged,
        }
    }
}

fn serialize_field_shape(field: &ast::Field<'_>) -> TokenStream2 {
    let member = field_member(&field.member);
    let name = lit(field.attrs.name().serialize_name());
    let skip = field.attrs.skip_serializing();
    let skip_if = option_path(field.attrs.skip_serializing_if());
    let custom_serializer = field.attrs.serialize_with().is_some();
    let flatten = field.attrs.flatten();
    let transparent = field.attrs.transparent();
    let ty = field.ty;
    let value_shape = if skip || custom_serializer {
        quote!(::core::option::Option::None)
    } else {
        quote!(::core::option::Option::Some(<#ty as ::serde_shape::SerializeShape>::serialize_shape_in(context)))
    };

    quote! {
        ::serde_shape::SerializeFieldShape {
            member: #member,
            name: #name,
            value_shape: #value_shape,
            flatten: #flatten,
            skip: #skip,
            skip_if: #skip_if,
            custom_serializer: #custom_serializer,
            transparent: #transparent,
        }
    }
}

fn deserialize_field_shape(field: &ast::Field<'_>) -> TokenStream2 {
    let member = field_member(&field.member);
    let name = lit(field.attrs.name().deserialize_name());
    let aliases = aliases(field.attrs.aliases());
    let skip = field.attrs.skip_deserializing();
    let custom_deserializer = field.attrs.deserialize_with().is_some();
    let default = default_shape(field.attrs.default());
    let flatten = field.attrs.flatten();
    let transparent = field.attrs.transparent();
    let ty = field.ty;
    let value_shape = if skip || custom_deserializer {
        quote!(::core::option::Option::None)
    } else {
        quote!(::core::option::Option::Some(<#ty as ::serde_shape::DeserializeShape>::deserialize_shape_in(context)))
    };

    quote! {
        ::serde_shape::DeserializeFieldShape {
            member: #member,
            name: #name,
            aliases: #aliases,
            value_shape: #value_shape,
            default: #default,
            flatten: #flatten,
            skip: #skip,
            custom_deserializer: #custom_deserializer,
            transparent: #transparent,
        }
    }
}

fn field_member(member: &Member) -> TokenStream2 {
    match member {
        Member::Named(ident) => {
            let ident = lit(ident.to_string());
            quote!(::serde_shape::FieldMember::Named(#ident))
        }
        Member::Unnamed(index) => {
            let index = index.index as usize;
            quote!(::serde_shape::FieldMember::Unnamed(#index))
        }
    }
}

fn fields_style(style: ast::Style) -> TokenStream2 {
    match style {
        ast::Style::Struct => quote!(::serde_shape::FieldsStyle::Struct),
        ast::Style::Tuple => quote!(::serde_shape::FieldsStyle::Tuple),
        ast::Style::Newtype => quote!(::serde_shape::FieldsStyle::Newtype),
        ast::Style::Unit => quote!(::serde_shape::FieldsStyle::Unit),
    }
}

fn tagging(tag: &attr::TagType) -> TokenStream2 {
    match tag {
        attr::TagType::External => quote!(::serde_shape::Tagging::External),
        attr::TagType::Internal { tag } => {
            let tag = lit(tag);
            quote!(::serde_shape::Tagging::Internal { tag: #tag })
        }
        attr::TagType::Adjacent { tag, content } => {
            let tag = lit(tag);
            let content = lit(content);
            quote!(::serde_shape::Tagging::Adjacent {
                tag: #tag,
                content: #content,
            })
        }
        attr::TagType::None => quote!(::serde_shape::Tagging::Untagged),
    }
}

fn default_shape(default: &attr::Default) -> TokenStream2 {
    match default {
        attr::Default::None => quote!(::serde_shape::DefaultShape::None),
        attr::Default::Default => quote!(::serde_shape::DefaultShape::Default),
        attr::Default::Path(path) => {
            let path = lit(path.to_token_stream().to_string());
            quote!(::serde_shape::DefaultShape::Path(#path))
        }
    }
}

fn opaque_reason(reason: &str) -> TokenStream2 {
    match reason {
        "FromType" => quote!(::serde_shape::OpaqueReason::FromType),
        "TryFromType" => quote!(::serde_shape::OpaqueReason::TryFromType),
        "IntoType" => quote!(::serde_shape::OpaqueReason::IntoType),
        "Remote" => quote!(::serde_shape::OpaqueReason::Remote),
        _ => quote!(::serde_shape::OpaqueReason::Unsupported),
    }
}

fn aliases(aliases: &BTreeSet<String>) -> TokenStream2 {
    let aliases = aliases.iter().map(lit);
    quote!(::serde_shape::__private::vec![#(#aliases),*])
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
