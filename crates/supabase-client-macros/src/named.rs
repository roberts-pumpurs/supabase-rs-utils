//! Resolve the established named projection grammar into shared projection compilation.
use proc_macro2::TokenStream as Tokens;
use quote::{quote, quote_spanned};
use syn::{
    Attribute, Ident, Path, Token, Type, Visibility, braced, bracketed,
    ext::IdentExt as _,
    parenthesized,
    parse::{Parse, ParseStream},
};

use crate::{Names, projection};

pub struct Input {
    runtime: Tokens,
    attributes: Vec<Attribute>,
    visibility: Visibility,
    name: Ident,
    first: Path,
    others: Vec<Path>,
    fields: Vec<Field>,
}
struct Field {
    name: Ident,
    kind: Kind,
}
enum Kind {
    Scalar,
    Embed(Path, Tokens, bool),
    Empty(Path),
}

impl Field {
    fn public_type(&self, rt: &Tokens, relation: &Path) -> Option<Tokens> {
        match &self.kind {
            Kind::Scalar => {
                let field = &self.name;
                Some(quote!(<#relation::columns::#field as #rt::schema::Column>::Value))
            }
            Kind::Embed(edge, child, _) => Some(quote!(
                <<#edge as #rt::schema::Relationship>::Cardinality as #rt::schema::Cardinality>::Output<#child>
            )),
            Kind::Empty(_) => None,
        }
    }
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let runtime;
        bracketed!(runtime in input);
        let runtime = runtime.parse()?;
        let attributes = input.call(Attribute::parse_outer)?;
        let visibility = input.parse()?;
        input.parse::<Token![struct]>()?;
        let name = input.parse()?;
        input.parse::<Token![for]>()?;
        let shared = input.peek(syn::token::Bracket);
        let (first, others) = if shared {
            let relations;
            bracketed!(relations in input);
            let paths = relations.parse_terminated(relation_path, Token![,])?;
            let mut paths = paths.into_iter();
            let first = paths
                .next()
                .ok_or_else(|| relations.error("expected at least one relation"))?;
            (first, paths.collect())
        } else {
            (relation_path(input)?, Vec::new())
        };
        let body;
        braced!(body in input);
        let mut fields = Vec::new();
        while !body.is_empty() {
            // The named grammar has always permitted empty comma-separated entries.
            if !shared && body.peek(Token![,]) {
                body.parse::<Token![,]>()?;
                continue;
            }
            let name = body.parse()?;
            let kind = if body.peek(Token![:]) && !shared {
                body.parse::<Token![:]>()?;
                let keyword: Ident = body.parse()?;
                let args;
                parenthesized!(args in body);
                let edge = args.parse()?;
                if keyword == "embed" {
                    args.parse::<Token![,]>()?;
                    let child: Type = args.parse()?;
                    let inner = if args.peek(Token![,]) {
                        args.parse::<Token![,]>()?;
                        let flag: Ident = args.parse()?;
                        if flag != "inner" {
                            return Err(syn::Error::new(flag.span(), "expected inner"));
                        }
                        true
                    } else {
                        false
                    };
                    if !args.is_empty() {
                        return Err(args.error("unexpected embed arguments"));
                    }
                    Kind::Embed(edge, quote!(#child), inner)
                } else if keyword == "empty" {
                    if !args.is_empty() {
                        return Err(args.error("expected empty(relationship)"));
                    }
                    Kind::Empty(edge)
                } else {
                    return Err(syn::Error::new(keyword.span(), "expected embed or empty"));
                }
            } else {
                Kind::Scalar
            };
            fields.push(Field { name, kind });
            if !body.is_empty() {
                body.parse::<Token![,]>()?;
            }
        }
        Ok(Self {
            runtime,
            attributes,
            visibility,
            name,
            first,
            others,
            fields,
        })
    }
}

fn relation_path(input: ParseStream) -> syn::Result<Path> {
    if input.peek(Token![::]) {
        return Err(input.error("expected a relation module path starting with an identifier"));
    }
    input.call(Path::parse_mod_style)
}

pub fn expand(names: &Names, input: &Input) -> Tokens {
    let rt = &input.runtime;
    let name = &input.name;
    let attributes = &input.attributes;
    let visibility = &input.visibility;
    let first = &input.first;
    let relation = quote!(#first::Row);
    let mut decoded = Vec::new();
    let mut selected = Vec::new();
    let mut aliases = Vec::new();
    let mut handles = Vec::new();
    let mut checks = Vec::new();
    for (index, field) in input.fields.iter().enumerate() {
        let alias = &field.name;
        let descriptor = match &field.kind {
            Kind::Scalar => {
                quote_spanned!(alias.span()=> #rt::schema::selection::Scalar<#first::columns::#alias>)
            }
            Kind::Embed(edge, child, inner) => {
                checks.push(
                    quote!(#rt::schema::__private::check_embed::<#relation, #edge, #child>();),
                );
                handles.push(quote! {
                #[allow(non_upper_case_globals)]
                pub const #alias: #rt::schema::Embed<#name, #child, #edge> = #rt::schema::Embed::new(#rt::schema::__private::alias(::core::stringify!(#alias)));
            });
                child_descriptor(
                    rt,
                    &names.ident(&format!("Alias{index}")),
                    alias,
                    edge,
                    child,
                    *inner,
                    &mut aliases,
                )
            }
            Kind::Empty(edge) => {
                checks.push(quote!(#rt::schema::__private::check_empty::<#relation, #edge>();));
                handles.push(quote! {
                #[allow(non_upper_case_globals)]
                pub const #alias: #rt::schema::Embed<#name, #rt::schema::EmptySelection<<#edge as #rt::schema::Relationship>::Target>, #edge> = #rt::schema::Embed::new(#rt::schema::__private::alias(::core::stringify!(#alias)));
            });
                child_descriptor(
                    rt,
                    &names.ident(&format!("Alias{index}")),
                    alias,
                    edge,
                    &quote!(#rt::schema::selection::PredicateRecord),
                    false,
                    &mut aliases,
                )
            }
        };
        if !matches!(field.kind, Kind::Empty(..)) {
            decoded.push(projection::DecodedField {
            name: alias.clone(),
            ty: quote!(<#descriptor as #rt::schema::selection::SelectionField<#relation>>::Value),
            key: quote!(<#descriptor as #rt::schema::selection::SelectionField<#relation>>::KEY),
            slot: names.ident(&format!("slot{index}")),
        });
        }
        selected.push(descriptor);
    }
    let generics = syn::Generics::default();
    let compiled = projection::decoder(
        names,
        rt,
        name,
        &generics,
        &decoded,
        None,
        (
            &format!("projection {}", name.unraw()),
            "a projection field name",
        ),
    );
    let rendering = projection::rendering(rt, name, &relation, &generics, &selected);
    let field_names = decoded.iter().map(|field| &field.name);
    // Descriptor alias types stay private inside the implementation block. Public
    // DTO fields expose only the original column/cardinality value types.
    let public_types = input
        .fields
        .iter()
        .filter_map(|field| field.public_type(rt, first));
    let shared = input
        .others
        .iter()
        .map(|next| shared_projection(input, next));
    quote! {
        #(#attributes)*
        #visibility struct #name { #(pub #field_names: #public_types,)* }
        const _: () = {
            #(#aliases)*
            #(#checks)*
            #rt::schema::__private::assert_distinct(&[#(<#selected as #rt::schema::selection::SelectionField<#relation>>::KEY),*]);
            impl #name { #(#handles)* }
            #compiled
            #rendering
            #(#shared)*
        };
    }
}

fn child_descriptor(
    rt: &Tokens,
    alias: &Ident,
    name: &Ident,
    edge: &Path,
    child: &Tokens,
    inner: bool,
    aliases: &mut Vec<Tokens>,
) -> Tokens {
    let key = name.unraw().to_string();
    aliases.push(quote! {
        struct #alias;
        impl #rt::schema::selection::Alias for #alias { const NAME: &'static ::core::primitive::str = #key; }
    });
    quote!(#rt::schema::selection::Child<#edge, #rt::schema::selection::Named<<#edge as #rt::schema::Relationship>::Target, #child>, #alias, #inner>)
}

fn shared_projection(input: &Input, next: &Path) -> Tokens {
    let rt = &input.runtime;
    let name = &input.name;
    let first = &input.first;
    let fields = input.fields.iter().map(|field| &field.name);
    quote! {
        #(#rt::schema::__private::assert_same_column::<#first::columns::#fields, #next::columns::#fields>();)*
        impl #rt::schema::Projection<#next::Row> for #name {
            const SELECT_LEN: ::core::primitive::usize = <Self as #rt::schema::Projection<#first::Row>>::SELECT_LEN;
            fn write_selection(output: &mut ::std::string::String) {
                <Self as #rt::schema::Projection<#first::Row>>::write_selection(output);
            }
        }
    }
}
