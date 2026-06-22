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

//! Derive macro for `serde-shape`.

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

/// Derive `serde_shape::SerdeShape` from Serde derive metadata.
#[proc_macro_derive(SerdeShape, attributes(serde))]
pub fn derive_serde_shape(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand_serde_shape(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_serde_shape(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let cx = Ctxt::new();
    let Some(container) = ast::Container::from_ast(&cx, input, Derive::Deserialize) else {
        cx.check()?;
        return Err(syn::Error::new_spanned(
            input,
            "serde-shape could not parse this item",
        ));
    };
    cx.check()?;

    let ident = &input.ident;
    let mut generics = input.generics.clone();
    add_shape_bounds(&mut generics, &container);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let body = shape_body(&container);

    Ok(quote! {
        impl #impl_generics ::serde_shape::SerdeShape for #ident #ty_generics #where_clause {
            fn shape_in(context: &mut ::serde_shape::ShapeContext) -> ::serde_shape::ShapeRef {
                #body
            }
        }
    })
}

fn add_shape_bounds(generics: &mut syn::Generics, container: &ast::Container<'_>) {
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
            collect_field_bound_types(fields, &type_params, &mut field_bound_types);
        }
        ast::Data::Enum(variants) => {
            for variant in variants {
                if variant.attrs.skip_deserializing() || variant.attrs.deserialize_with().is_some()
                {
                    continue;
                }
                collect_field_bound_types(&variant.fields, &type_params, &mut field_bound_types);
            }
        }
    }

    for ty in field_bound_types {
        generics
            .make_where_clause()
            .predicates
            .push(parse_quote!(#ty: ::serde_shape::SerdeShape));
    }
}

fn collect_field_bound_types(
    fields: &[ast::Field<'_>],
    type_params: &BTreeSet<String>,
    field_bound_types: &mut Vec<Type>,
) {
    for field in fields {
        if field.attrs.skip_deserializing() || field.attrs.deserialize_with().is_some() {
            continue;
        }

        let mut used_type_params = BTreeSet::new();
        collect_type_params(field.ty, type_params, &mut used_type_params);
        if !used_type_params.is_empty() {
            field_bound_types.push((*field.ty).clone());
        }
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

fn shape_body(container: &ast::Container<'_>) -> TokenStream2 {
    let serde_name = lit(container.attrs.name().deserialize_name());
    let serialize_name = lit(container.attrs.name().serialize_name());
    let kind = definition_kind(container);

    quote! {
        context.define_named_type(
            ::serde_shape::TypeName {
                rust_name: ::std::any::type_name::<Self>(),
                serde_name: #serde_name,
                serialize_name: #serialize_name,
            },
            |context| {
                #kind
            },
        )
    }
}

fn definition_kind(container: &ast::Container<'_>) -> TokenStream2 {
    if let Some(ty) = container.attrs.type_from() {
        return opaque_definition("FromType", ty);
    }
    if let Some(ty) = container.attrs.type_try_from() {
        return opaque_definition("TryFromType", ty);
    }
    if let Some(path) = container.attrs.remote() {
        return opaque_definition("Remote", path);
    }

    let attributes = container_attributes(&container.attrs);
    match &container.data {
        ast::Data::Struct(style, fields) => {
            let style = fields_style(*style);
            let fields = fields.iter().map(field_shape);
            quote! {
                ::serde_shape::DefinitionKind::Struct(::serde_shape::StructShape {
                    style: #style,
                    fields: ::std::vec![#(#fields),*],
                    attributes: #attributes,
                })
            }
        }
        ast::Data::Enum(variants) => {
            let repr = tagging(container.attrs.tag());
            let variants = variants.iter().map(variant_shape);
            quote! {
                ::serde_shape::DefinitionKind::Enum(::serde_shape::EnumShape {
                    repr: #repr,
                    variants: ::std::vec![#(#variants),*],
                    attributes: #attributes,
                })
            }
        }
    }
}

fn opaque_definition<T>(reason: &str, detail: T) -> TokenStream2
where
    T: ToTokens,
{
    let reason = opaque_reason(reason);
    let detail = lit(detail.to_token_stream().to_string());

    quote! {
        ::serde_shape::DefinitionKind::Opaque(::serde_shape::OpaqueShape {
            type_name: ::std::any::type_name::<Self>(),
            reason: #reason,
            detail: ::std::option::Option::Some(#detail),
        })
    }
}

fn container_attributes(attrs: &attr::Container) -> TokenStream2 {
    let tagging = tagging(attrs.tag());
    let deny_unknown_fields = attrs.deny_unknown_fields();
    let default = default_shape(attrs.default());
    let has_flatten = attrs.has_flatten();
    let transparent = attrs.transparent();
    let expecting = option_lit(attrs.expecting());
    let non_exhaustive = attrs.non_exhaustive();

    quote! {
        ::serde_shape::ContainerAttributes {
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

fn variant_shape(variant: &ast::Variant<'_>) -> TokenStream2 {
    let rust_name = lit(variant.ident.to_string());
    let serialize_name = lit(variant.attrs.name().serialize_name());
    let deserialize_name = lit(variant.attrs.name().deserialize_name());
    let deserialize_aliases = aliases(variant.attrs.aliases());
    let style = fields_style(variant.style);
    let skip_serializing = variant.attrs.skip_serializing();
    let skip_deserializing = variant.attrs.skip_deserializing();
    let custom_serializer = variant.attrs.serialize_with().is_some();
    let custom_deserializer = variant.attrs.deserialize_with().is_some();
    let other = variant.attrs.other();
    let untagged = variant.attrs.untagged();
    let fields: Vec<_> = if skip_deserializing || custom_deserializer {
        Vec::new()
    } else {
        variant.fields.iter().map(field_shape).collect()
    };

    quote! {
        ::serde_shape::VariantShape {
            rust_name: #rust_name,
            serialize_name: #serialize_name,
            deserialize_name: #deserialize_name,
            deserialize_aliases: #deserialize_aliases,
            style: #style,
            fields: ::std::vec![#(#fields),*],
            skip_serializing: #skip_serializing,
            skip_deserializing: #skip_deserializing,
            custom_serializer: #custom_serializer,
            custom_deserializer: #custom_deserializer,
            other: #other,
            untagged: #untagged,
        }
    }
}

fn field_shape(field: &ast::Field<'_>) -> TokenStream2 {
    let member = field_member(&field.member);
    let serialize_name = lit(field.attrs.name().serialize_name());
    let deserialize_name = lit(field.attrs.name().deserialize_name());
    let deserialize_aliases = aliases(field.attrs.aliases());
    let skip_serializing = field.attrs.skip_serializing();
    let skip_serializing_if = option_path(field.attrs.skip_serializing_if());
    let skip_deserializing = field.attrs.skip_deserializing();
    let custom_serializer = field.attrs.serialize_with().is_some();
    let custom_deserializer = field.attrs.deserialize_with().is_some();
    let default = default_shape(field.attrs.default());
    let flatten = field.attrs.flatten();
    let transparent = field.attrs.transparent();
    let ty = field.ty;
    let shape = if skip_deserializing || custom_deserializer {
        quote!(::std::option::Option::None)
    } else {
        quote!(::std::option::Option::Some(<#ty as ::serde_shape::SerdeShape>::shape_in(context)))
    };

    quote! {
        ::serde_shape::FieldShape {
            member: #member,
            serialize_name: #serialize_name,
            deserialize_name: #deserialize_name,
            deserialize_aliases: #deserialize_aliases,
            shape: #shape,
            default: #default,
            flatten: #flatten,
            skip_serializing: #skip_serializing,
            skip_serializing_if: #skip_serializing_if,
            skip_deserializing: #skip_deserializing,
            custom_serializer: #custom_serializer,
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
        "Remote" => quote!(::serde_shape::OpaqueReason::Remote),
        _ => quote!(::serde_shape::OpaqueReason::Unsupported),
    }
}

fn aliases(aliases: &std::collections::BTreeSet<String>) -> TokenStream2 {
    let aliases = aliases.iter().map(lit);
    quote!(::std::vec![#(#aliases),*])
}

fn option_lit(value: Option<&str>) -> TokenStream2 {
    match value {
        Some(value) => {
            let value = lit(value);
            quote!(::std::option::Option::Some(#value))
        }
        None => quote!(::std::option::Option::None),
    }
}

fn option_path(value: Option<&syn::ExprPath>) -> TokenStream2 {
    match value {
        Some(value) => {
            let value = lit(value.to_token_stream().to_string().replace(' ', ""));
            quote!(::std::option::Option::Some(#value))
        }
        None => quote!(::std::option::Option::None),
    }
}

fn lit(value: impl AsRef<str>) -> LitStr {
    LitStr::new(value.as_ref(), proc_macro2::Span::call_site())
}
