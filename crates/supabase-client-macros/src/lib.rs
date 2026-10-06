//! Constructor-checked query-local records and lossless schema keys.
use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;
use quote::{quote, quote_spanned};
use syn::{
    Ident, Path, Token, Type, braced,
    ext::IdentExt as _,
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

struct Input {
    runtime: Path,
    root: Type,
    fields: Vec<Field>,
}
struct Field {
    alias: Ident,
    kind: Kind,
}
enum Kind {
    Scalar,
    Nested(Ident, Vec<Field>, bool),
    Shared(Ident, Type, bool),
    Empty(Ident, bool),
}
fn fields(input: ParseStream) -> syn::Result<Vec<Field>> {
    let body;
    braced!(body in input);
    let mut out = Vec::<Field>::new();
    while !body.is_empty() {
        let alias: Ident = body.parse()?;
        let normalized = alias.to_string().trim_start_matches("r#").to_owned();
        if out
            .iter()
            .any(|field| field.alias.to_string().trim_start_matches("r#") == normalized)
        {
            return Err(syn::Error::new(alias.span(), "duplicate selected alias"));
        }
        let kind = if body.peek(Token![:]) {
            body.parse::<Token![:]>()?;
            let edge: Ident = body.parse()?;
            if edge == "embed" || edge == "empty" || edge == "inner" {
                let args;
                parenthesized!(args in body);
                let key = args.parse()?;
                if edge == "inner" {
                    if !args.is_empty() {
                        return Err(args.error("expected inner(relationship)"));
                    }
                    Kind::Nested(key, fields(&body)?, true)
                } else {
                    let dto = if edge == "embed" {
                        args.parse::<Token![,]>()?;
                        Some(args.parse::<Type>()?)
                    } else {
                        None
                    };
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
                    match dto {
                        Some(dto) => Kind::Shared(key, dto, inner),
                        None => Kind::Empty(key, inner),
                    }
                }
            } else {
                Kind::Nested(edge, fields(&body)?, false)
            }
        } else {
            Kind::Scalar
        };
        out.push(Field { alias, kind });
        if body.is_empty() {
            break;
        }
        body.parse::<Token![,]>()?;
    }
    if out.is_empty() {
        return Err(body.error("select at least one field"));
    }
    Ok(out)
}
impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let runtime = if input.peek(Ident) && input.peek2(Token![=]) && !input.peek2(Token![=>]) {
            let keyword: Ident = input.parse()?;
            if keyword != "runtime" {
                return Err(syn::Error::new(
                    keyword.span(),
                    "expected runtime = crate_path;",
                ));
            }
            input.parse::<Token![=]>()?;
            let path = input.parse()?;
            input.parse::<Token![;]>()?;
            path
        } else {
            syn::parse_quote!(::rp_supabase_client)
        };
        let root = input.parse()?;
        input.parse::<Token![=>]>()?;
        let fields = fields(input)?;
        Ok(Self {
            runtime,
            root,
            fields,
        })
    }
}
/// Construct a typed query-local selection from a relation and selected fields.
#[proc_macro]
pub fn select(input: TokenStream) -> TokenStream {
    let names = Names::new(input.clone().into());
    let Input {
        runtime,
        root,
        fields,
    } = parse_macro_input!(input as Input);
    node(
        &names,
        &runtime,
        &fields,
        &quote!(#runtime::schema::selection::RelationToken::<#root>::new()),
    )
    .into()
}
// Every generated spelling is outside the entire caller token identifier set.
// Growing a prefix (rather than hashing) also covers nested paths and generic arguments.
struct Names {
    prefix: String,
}

impl Names {
    fn new(tokens: Tokens) -> Self {
        fn collect(tokens: Tokens, names: &mut Vec<String>) {
            for token in tokens {
                match token {
                    proc_macro2::TokenTree::Ident(ident) => names.push(ident.unraw().to_string()),
                    proc_macro2::TokenTree::Group(group) => collect(group.stream(), names),
                    proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => {}
                }
            }
        }
        let mut identifiers = Vec::new();
        collect(tokens, &mut identifiers);
        let mut prefix = String::from("__selection_");
        while identifiers.iter().any(|name| name.starts_with(&prefix)) {
            prefix.push('_');
        }
        Self { prefix }
    }

    fn ident(&self, suffix: &str) -> Ident {
        Ident::new(
            &format!("{}{suffix}", self.prefix),
            proc_macro2::Span::mixed_site(),
        )
    }
}

struct EmissionField<'a> {
    field: &'a Field,
    index: usize,
    ty: Ident,
    argument: Ident,
    slot: Ident,
}

struct Emission<'a> {
    names: &'a Names,
    runtime: &'a Path,
    fields: Vec<EmissionField<'a>>,
    marker: Ident,
}

impl<'a> Emission<'a> {
    fn new(names: &'a Names, runtime: &'a Path, fields: &'a [Field]) -> Self {
        let marker = names.ident("marker");
        let fields = fields
            .iter()
            .enumerate()
            .map(|(index, field)| EmissionField {
                field,
                index,
                ty: names.ident(&format!("F{index}")),
                argument: names.ident(&format!("arg{index}")),
                slot: names.ident(&format!("slot{index}")),
            })
            .collect();
        Self {
            names,
            runtime,
            fields,
            marker,
        }
    }

    fn types(&self) -> impl Iterator<Item = &Ident> {
        self.fields.iter().map(|field| &field.ty)
    }

    fn decoded(&self) -> impl Iterator<Item = &EmissionField<'a>> {
        self.fields
            .iter()
            .filter(|field| !matches!(field.field.kind, Kind::Empty(..)))
    }

    fn embedded(&self) -> impl Iterator<Item = &EmissionField<'a>> {
        self.fields
            .iter()
            .filter(|field| !matches!(field.field.kind, Kind::Scalar))
    }

    fn bound(&self) -> Tokens {
        let r_ident = self.names.ident("R");
        let rt = self.runtime;
        let types = self.types();
        quote!(#r_ident: #rt::schema::Relation + #rt::schema::Projection<#r_ident>, #(#types: #rt::schema::selection::SelectionField<#r_ident>),*)
    }
}

fn node(names: &Names, rt: &Path, fields: &[Field], relation: &Tokens) -> Tokens {
    let construct_ident = names.ident("construct");
    let relation_ident = names.ident("relation");
    let emission = Emission::new(names, rt, fields);
    let record = record(&emission);
    let keys = decoder_keys(&emission);
    let decoder = decoder(&emission);
    let projection = projection(&emission);
    let descriptor = descriptor(&emission);
    let values = emission
        .fields
        .iter()
        .map(|field| field_value(names, rt, field));
    quote!({
        #record
        #keys
        #decoder
        #projection
        #descriptor
        let #relation_ident = #relation;
        #construct_ident(#relation_ident, #(#values),*)
    })
}

fn field_value(names: &Names, rt: &Path, field: &EmissionField<'_>) -> Tokens {
    let edge_ident = names.ident("edge");
    let relation_ident = names.ident("relation");
    let alias = &field.field.alias;
    match &field.field.kind {
        Kind::Scalar => {
            let key = key_type(rt, alias);
            quote_spanned!(alias.span()=> #rt::schema::selection::column(#relation_ident, #rt::schema::Key::<#key>::new()))
        }
        Kind::Nested(edge, child, inner) => {
            let child = node(names, rt, child, &quote!(#edge_ident.target()));
            embedded_value(names, rt, field, edge, *inner, &child)
        }
        Kind::Shared(edge, dto, inner) => {
            let child = quote_spanned!(alias.span()=> #rt::schema::selection::shared::<_, #dto>(#edge_ident));
            embedded_value(names, rt, field, edge, *inner, &child)
        }
        Kind::Empty(edge, inner) => {
            let child = quote!(#rt::schema::selection::empty(#edge_ident));
            embedded_value(names, rt, field, edge, *inner, &child)
        }
    }
}

fn embedded_value(
    names: &Names,
    rt: &Path,
    field: &EmissionField<'_>,
    edge: &Ident,
    inner: bool,
    child: &Tokens,
) -> Tokens {
    let child_ident = names.ident("child");
    let edge_ident = names.ident("edge");
    let relation_ident = names.ident("relation");
    let alias = &field.field.alias;
    let index = field.index;
    let alias_ty = names.ident(&format!("Alias{index}"));
    let alias_name = alias.unraw().to_string();
    let key = key_type(rt, edge);
    let lookup = quote_spanned!(edge.span()=> #rt::schema::selection::edge(#relation_ident, #rt::schema::Key::<#key>::new()));
    quote!({
        struct #alias_ty;
        impl #rt::schema::selection::Alias for #alias_ty { const NAME: &'static ::core::primitive::str = #alias_name; }
        let #edge_ident = #lookup;
        let #child_ident = #child;
        #rt::schema::selection::embed::<_, _, _, #inner>(#edge_ident, #child_ident, #alias_ty)
    })
}

// Generic items know only resolved Field descriptors; lookup constraints never enter decoder/rendering.
#[expect(
    clippy::cognitive_complexity,
    reason = "quote emits record trait implementations; the emitter has no branches"
)]
fn record(emission: &Emission<'_>) -> Tokens {
    let r_ident = emission.names.ident("R");
    let record_ident = emission.names.ident("Record");
    let s_ident = emission.names.ident("S");
    let rt = emission.runtime;
    let fs: Vec<_> = emission.types().collect();
    let bound = emission.bound();
    let marker = &emission.marker;
    let dfs: Vec<_> = emission.decoded().map(|field| &field.ty).collect();
    let dnames: Vec<_> = emission.decoded().map(|field| &field.field.alias).collect();
    let count = dfs.len();
    quote! {
        struct #record_ident<#r_ident,#(#fs),*> where #bound {
            #(pub #dnames: <#dfs as #rt::schema::selection::SelectionField<#r_ident>>::Value,)*
            #marker: ::core::marker::PhantomData<fn() -> (#r_ident,#(#fs),*)>,
        }
        impl<#r_ident,#(#fs),*> ::core::fmt::Debug for #record_ident<#r_ident,#(#fs),*> where #bound, #(<#dfs as #rt::schema::selection::SelectionField<#r_ident>>::Value: ::core::fmt::Debug,)* {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                let mut d = f.debug_struct("Record"); #(d.field(::core::stringify!(#dnames), &self.#dnames);)* d.finish()
            }
        }
        impl<#r_ident,#(#fs),*> #rt::schema::__private::serde::Serialize for #record_ident<#r_ident,#(#fs),*> where #bound, #(<#dfs as #rt::schema::selection::SelectionField<#r_ident>>::Value: #rt::schema::__private::serde::Serialize,)* {
            fn serialize<#s_ident: #rt::schema::__private::serde::Serializer>(&self, s: #s_ident) -> ::core::result::Result<#s_ident::Ok,#s_ident::Error> {
                let mut m = s.serialize_map(::core::option::Option::Some(#count))?;
                #(#rt::schema::__private::serde::ser::SerializeMap::serialize_entry(&mut m, <#dfs as #rt::schema::selection::SelectionField<#r_ident>>::KEY, &self.#dnames)?;)* #rt::schema::__private::serde::ser::SerializeMap::end(m)
            }
        }
    }
}

#[expect(
    clippy::cognitive_complexity,
    reason = "quote emits identifier visitor branches, not emitter control flow"
)]
fn decoder_keys(emission: &Emission<'_>) -> Tokens {
    let de_lifetime = syn::Lifetime::new(
        &format!("'{}", emission.names.ident("de")),
        proc_macro2::Span::mixed_site(),
    );
    let d_ident = emission.names.ident("D");
    let e_ident = emission.names.ident("E");
    let key_ident = emission.names.ident("Key");
    let keyvisitor_ident = emission.names.ident("KeyVisitor");
    let r_ident = emission.names.ident("R");
    let rt = emission.runtime;
    let fs: Vec<_> = emission.types().collect();
    let bound = emission.bound();
    let dfs: Vec<_> = emission.decoded().map(|field| &field.ty).collect();
    let indices: Vec<_> = emission.decoded().map(|field| field.index).collect();
    quote! {
        struct #key_ident<#r_ident,#(#fs),*>(::core::option::Option<::core::primitive::usize>, ::core::marker::PhantomData<fn() -> (#r_ident,#(#fs),*)>);
        struct #keyvisitor_ident<#r_ident,#(#fs),*>(::core::marker::PhantomData<fn() -> (#r_ident,#(#fs),*)>);
        impl<#de_lifetime,#r_ident,#(#fs),*> #rt::schema::__private::serde::de::Visitor<#de_lifetime> for #keyvisitor_ident<#r_ident,#(#fs),*> where #bound {
            type Value = #key_ident<#r_ident,#(#fs),*>;
            fn expecting(&self,f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result { f.write_str("a selected object field") }
            fn visit_str<#e_ident: #rt::schema::__private::serde::de::Error>(self,key: &::core::primitive::str) -> ::core::result::Result<Self::Value,#e_ident> {
                #(if key == <#dfs as #rt::schema::selection::SelectionField<#r_ident>>::KEY { return ::core::result::Result::Ok(#key_ident(::core::option::Option::Some(#indices), ::core::marker::PhantomData)); })*
                ::core::result::Result::Ok(#key_ident(::core::option::Option::None, ::core::marker::PhantomData))
            }
        }
        impl<#de_lifetime,#r_ident,#(#fs),*> #rt::schema::__private::serde::Deserialize<#de_lifetime> for #key_ident<#r_ident,#(#fs),*> where #bound {
            fn deserialize<#d_ident: #rt::schema::__private::serde::Deserializer<#de_lifetime>>(d: #d_ident) -> ::core::result::Result<Self,#d_ident::Error> { d.deserialize_identifier(#keyvisitor_ident::<#r_ident,#(#fs),*>(::core::marker::PhantomData)) }
        }
    }
}

// The complexity is inside the emitted decoder, not the emitter's control flow.
#[expect(
    clippy::cognitive_complexity,
    reason = "quote emits the map visitor's field decoding branches"
)]
fn decoder(emission: &Emission<'_>) -> Tokens {
    let de_lifetime = syn::Lifetime::new(
        &format!("'{}", emission.names.ident("de")),
        proc_macro2::Span::mixed_site(),
    );
    let d_ident = emission.names.ident("D");
    let key_type_ident = emission.names.ident("Key");
    let m_ident = emission.names.ident("M");
    let r_ident = emission.names.ident("R");
    let record_ident = emission.names.ident("Record");
    let visitor_ident = emission.names.ident("Visitor");
    let key_value_ident = emission.names.ident("key");
    let map_ident = emission.names.ident("map");
    let rt = emission.runtime;
    let fs: Vec<_> = emission.types().collect();
    let bound = emission.bound();
    let marker = &emission.marker;
    let dfs: Vec<_> = emission.decoded().map(|field| &field.ty).collect();
    let dnames: Vec<_> = emission.decoded().map(|field| &field.field.alias).collect();
    let dslots: Vec<_> = emission.decoded().map(|field| &field.slot).collect();
    let indices: Vec<_> = emission.decoded().map(|field| field.index).collect();
    quote! {
        struct #visitor_ident<#r_ident,#(#fs),*>(::core::marker::PhantomData<fn() -> (#r_ident,#(#fs),*)>);
        impl<#de_lifetime,#r_ident,#(#fs),*> #rt::schema::__private::serde::de::Visitor<#de_lifetime> for #visitor_ident<#r_ident,#(#fs),*> where #bound {
            type Value = #record_ident<#r_ident,#(#fs),*>;
            fn expecting(&self,f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result { f.write_str("a selected object") }
            fn visit_map<#m_ident: #rt::schema::__private::serde::de::MapAccess<#de_lifetime>>(self,mut #map_ident: #m_ident) -> ::core::result::Result<Self::Value,#m_ident::Error> {
                #(let mut #dslots: ::core::option::Option<<#dfs as #rt::schema::selection::SelectionField<#r_ident>>::Value> = ::core::option::Option::None;)*
                while let ::core::option::Option::Some(#key_value_ident) = #map_ident.next_key::<#key_type_ident<#r_ident,#(#fs),*>>()? {
                    match #key_value_ident.0 {
                        #(::core::option::Option::Some(#indices) => {
                            if #dslots.is_some() { return ::core::result::Result::Err(<#m_ident::Error as #rt::schema::__private::serde::de::Error>::duplicate_field(<#dfs as #rt::schema::selection::SelectionField<#r_ident>>::KEY)); }
                            #dslots = ::core::option::Option::Some(#map_ident.next_value()?);
                        })*
                        _ => { let _: #rt::schema::__private::serde::de::IgnoredAny = #map_ident.next_value()?; }
                    }
                }
                ::core::result::Result::Ok(#record_ident { #(#dnames: #dslots.ok_or_else(|| <#m_ident::Error as #rt::schema::__private::serde::de::Error>::missing_field(<#dfs as #rt::schema::selection::SelectionField<#r_ident>>::KEY))?,)* #marker: ::core::marker::PhantomData })
            }
        }
        impl<#de_lifetime,#r_ident,#(#fs),*> #rt::schema::__private::serde::Deserialize<#de_lifetime> for #record_ident<#r_ident,#(#fs),*> where #bound {
            fn deserialize<#d_ident: #rt::schema::__private::serde::Deserializer<#de_lifetime>>(d: #d_ident) -> ::core::result::Result<Self,#d_ident::Error> { d.deserialize_map(#visitor_ident::<#r_ident,#(#fs),*>(::core::marker::PhantomData)) }
        }
    }
}

fn projection(emission: &Emission<'_>) -> Tokens {
    let r_ident = emission.names.ident("R");
    let record_ident = emission.names.ident("Record");
    let rt = emission.runtime;
    let fs: Vec<_> = emission.types().collect();
    let bound = emission.bound();
    quote! {
        impl<#r_ident,#(#fs),*> #rt::schema::Projection<#r_ident> for #record_ident<#r_ident,#(#fs),*> where #bound {
            const SELECT_LEN: ::core::primitive::usize = {
                #rt::schema::__private::assert_distinct(&[#(<#fs as #rt::schema::selection::SelectionField<#r_ident>>::KEY),*]);
                (0usize #(+ <#fs as #rt::schema::selection::SelectionField<#r_ident>>::LEN + 1)*).saturating_sub(1)
            };
            fn write_selection(out: &mut ::std::string::String) {
                let start = out.len(); #(if out.len() != start { out.push(','); } <#fs as #rt::schema::selection::SelectionField<#r_ident>>::write(out);)*
            }
        }
    }
}

#[expect(
    clippy::cognitive_complexity,
    reason = "quote emits descriptor trait bounds; the emitter has no branches"
)]
fn descriptor(emission: &Emission<'_>) -> Tokens {
    let descriptor_ident = emission.names.ident("Descriptor");
    let r_ident = emission.names.ident("R");
    let record_ident = emission.names.ident("Record");
    let construct_ident = emission.names.ident("construct");
    let rt = emission.runtime;
    let fs: Vec<_> = emission.types().collect();
    let args: Vec<_> = emission
        .fields
        .iter()
        .map(|field| &field.argument)
        .collect();
    let bound = emission.bound();
    let marker = &emission.marker;
    let ebounds: Vec<_> = emission
        .embedded()
        .map(|field| {
            let ty = &field.ty;
            quote!(#ty: #rt::schema::selection::Embedded<#r_ident>)
        })
        .collect();
    let descfields: Vec<_> = emission.embedded().map(|field| {
        let ty = &field.ty;
        let alias = &field.field.alias;
        quote!(pub #alias: #rt::schema::selection::Handle<#record_ident<#r_ident,#(#fs),*>, <<#ty as #rt::schema::selection::Embedded<#r_ident>>::Child as #rt::schema::selection::Selection>::Record, <#ty as #rt::schema::selection::Embedded<#r_ident>>::Edge, <#ty as #rt::schema::selection::Embedded<#r_ident>>::Alias, <#ty as #rt::schema::selection::Embedded<#r_ident>>::Child>)
    }).collect();
    let descinit: Vec<_> = emission.embedded().map(|field| {
        let argument = &field.argument;
        let alias = &field.field.alias;
        quote!(#alias: #rt::schema::selection::Handle::new(#rt::schema::selection::Embedded::child(#argument)))
    }).collect();
    quote! {
        struct #descriptor_ident<#r_ident,#(#fs),*> where #bound, #(#ebounds,)* {
            #(#descfields,)*
            #marker: ::core::marker::PhantomData<fn() -> (#r_ident,#(#fs),*)>,
        }
        impl<#r_ident,#(#fs),*> ::core::marker::Copy for #descriptor_ident<#r_ident,#(#fs),*> where #bound, #(#ebounds,)* {}
        impl<#r_ident,#(#fs),*> ::core::clone::Clone for #descriptor_ident<#r_ident,#(#fs),*> where #bound, #(#ebounds,)* { fn clone(&self) -> Self { *self } }
        impl<#r_ident,#(#fs),*> #rt::schema::selection::Selection for #descriptor_ident<#r_ident,#(#fs),*> where #bound, #(#ebounds,)* {
            type Relation = #r_ident;
            type Record = #record_ident<#r_ident,#(#fs),*>;
        }
        fn #construct_ident<#r_ident,#(#fs),*>(_: #rt::schema::selection::RelationToken<#r_ident>, #(#args: #fs),*) -> #descriptor_ident<#r_ident,#(#fs),*> where #bound, #(#ebounds,)* {
            #descriptor_ident { #(#descinit,)* #marker: ::core::marker::PhantomData }
        }
    }
}

fn key_type(rt: &Path, name: &Ident) -> Tokens {
    let text = name.unraw().to_string();
    let characters = text.chars();
    quote!((#(#rt::schema::Character<#characters>,)*))
}
struct KeyInput {
    runtime: Path,
    name: Ident,
    ty: bool,
}
impl Parse for KeyInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let runtime = if input.peek(Ident) && input.peek2(Token![=]) {
            let word: Ident = input.parse()?;
            if word != "runtime" {
                return Err(syn::Error::new(word.span(), "expected runtime"));
            }
            input.parse::<Token![=]>()?;
            let path = input.parse()?;
            input.parse::<Token![;]>()?;
            path
        } else {
            syn::parse_quote!(::rp_supabase_client)
        };
        let ty = input.peek(Token![type]);
        if ty {
            input.parse::<Token![type]>()?;
        }
        let name = input.parse()?;
        Ok(Self { runtime, name, ty })
    }
}
/// Construct a lossless schema key, or its name type with `key!(type identifier)`.
#[proc_macro]
pub fn key(input: TokenStream) -> TokenStream {
    let KeyInput { runtime, name, ty } = parse_macro_input!(input as KeyInput);
    let key = key_type(&runtime, &name);
    if ty {
        key.into()
    } else {
        quote!(#runtime::schema::Key::<#key>::new()).into()
    }
}
