#![expect(
    clippy::min_ident_chars,
    reason = "Short closure bindings keep nested metadata traversals and quoted Rust readable."
)]

use alloc::collections::{BTreeMap, BTreeSet};

use heck::{ToSnakeCase as _, ToUpperCamelCase as _};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::parse::Parser as _;

use crate::{Config, Error, model::*};

fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}

fn ident(name: &str, camel: bool) -> Result<Ident, Error> {
    let mut normalized = if camel {
        name.to_upper_camel_case()
    } else {
        name.to_snake_case()
    };
    if normalized.is_empty() {
        return Err(invalid(format!(
            "identifier {name:?} has no Rust identifier characters"
        )));
    }
    if normalized.starts_with(|c: char| c.is_ascii_digit()) {
        normalized.insert(0, '_');
    }
    if matches!(normalized.as_str(), "self" | "Self" | "super" | "crate") {
        normalized.push('_');
    }
    syn::parse_str::<Ident>(&normalized)
        .or_else(|_| syn::parse_str::<Ident>(&format!("r#{normalized}")))
        .map_err(|_error| {
            invalid(format!(
                "cannot convert PostgreSQL identifier {name:?} to Rust"
            ))
        })
}

fn unique<'a>(
    names: impl IntoIterator<Item = &'a str>,
    camel: bool,
    context: &str,
) -> Result<(), Error> {
    let mut seen = BTreeMap::new();
    for name in names {
        let rust = ident(name, camel)?.to_string();
        if let Some(previous) = seen.insert(rust.clone(), name) {
            return Err(invalid(format!(
                "{context}: identifiers {previous:?} and {name:?} collide as {rust}"
            )));
        }
    }
    Ok(())
}

fn identifiers(tokens: TokenStream, names: &mut BTreeSet<String>) {
    for token in tokens {
        match token {
            proc_macro2::TokenTree::Ident(name) => {
                names.insert(name.to_string());
            }
            proc_macro2::TokenTree::Group(group) => identifiers(group.stream(), names),
            proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => {}
        }
    }
}

// Bind override syntax in the prelude's scope, before nested generated names can shadow it.
fn override_aliases(
    prelude: &syn::File,
    types: BTreeMap<String, syn::Type>,
) -> (TokenStream, BTreeMap<String, Ident>) {
    let mut occupied = BTreeSet::new();
    identifiers(quote!(#prelude), &mut occupied);
    for ty in types.values() {
        identifiers(quote!(#ty), &mut occupied);
    }
    let mut aliases = TokenStream::new();
    let mut overrides = BTreeMap::new();
    let mut serial = 0_usize;
    for (key, ty) in types {
        let name = loop {
            let name = format!("__SupabaseCodegenType{serial}");
            serial = serial.saturating_add(1_usize);
            if occupied.insert(name.clone()) {
                break Ident::new(&name, Span::call_site());
            }
        };
        aliases.extend(quote!(type #name = #ty;));
        overrides.insert(key, name);
    }
    (aliases, overrides)
}

struct Emitter<'a> {
    config: &'a Config,
    runtime: syn::Path,
    derives: Vec<syn::Path>,
    attributes: Vec<syn::Attribute>,
    targeted: BTreeMap<String, Vec<syn::Attribute>>,
    used_targets: BTreeSet<String>,
    overrides: BTreeMap<String, Ident>,
    named: BTreeMap<(String, String), bool>, // true for composite, false for enum
}

fn attributes(values: &[String]) -> Result<Vec<syn::Attribute>, Error> {
    let mut result = Vec::new();
    for value in values {
        let parsed = syn::Attribute::parse_outer
            .parse_str(value)
            .map_err(|e| invalid(format!("invalid type attribute {value:?}: {e}")))?;
        if parsed.is_empty() {
            return Err(invalid("type attribute cannot be empty"));
        }
        result.extend(parsed);
    }
    Ok(result)
}

impl Emitter<'_> {
    fn decoration(&mut self, target: &str, deserialize: bool, default: bool) -> TokenStream {
        self.used_targets.insert(target.to_owned());
        let attrs = &self.attributes;
        let targeted = self.targeted.get(target).cloned().unwrap_or_default();
        let mut derives: Vec<syn::Path> = vec![
            syn::parse_quote!(::serde::Serialize),
            syn::parse_quote!(Debug),
            syn::parse_quote!(Clone),
        ];
        if deserialize {
            derives.push(syn::parse_quote!(::serde::Deserialize));
        }
        if default {
            derives.push(syn::parse_quote!(Default));
        }
        for derive in &self.derives {
            if !derives
                .iter()
                .any(|p| quote!(#p).to_string() == quote!(#derive).to_string())
            {
                derives.push(derive.clone());
            }
        }
        quote!(#(#attrs)* #(#targeted)* #[derive(#(#derives),*)])
    }

    #[expect(
        clippy::too_many_lines,
        reason = "One SQL wire-type mapping keeps scalar choices visible together."
    )]
    fn ty(&self, ty: &PgType, depth: usize) -> Result<TokenStream, Error> {
        let key = match ty {
            PgType::Builtin(name) => Some(if name.contains('.') {
                name.clone()
            } else {
                format!("pg_catalog.{name}")
            }),
            PgType::Named { schema, name } | PgType::Domain { schema, name, .. } => {
                Some(format!("{schema}.{name}"))
            }
            PgType::Array(_) => None,
        };
        if let Some(overridden) = key.as_ref().and_then(|key| self.overrides.get(key)) {
            let parents = core::iter::repeat_with(|| quote!(super::)).take(depth);
            return Ok(quote!(#(#parents)* #overridden));
        }
        match ty {
            PgType::Domain { base, .. } => self.ty(base, depth),
            PgType::Array(inner) => {
                let inner = self.ty(inner, depth)?;
                let runtime = &self.runtime;
                Ok(quote!(#runtime::Array<#inner>))
            }
            PgType::Named { schema, name } => {
                let composite = self.named.get(&(schema.clone(), name.clone())).ok_or_else(|| invalid(format!("unresolved PostgreSQL type {schema}.{name}; include its schema metadata or supply type_override")))?;
                let schema = ident(schema, false)?;
                let name = ident(name, true)?;
                let scope = if *composite {
                    quote!(composites)
                } else {
                    quote!(enums)
                };
                let parents = core::iter::repeat_with(|| quote!(super::)).take(depth);
                Ok(quote!(#(#parents)* #schema::#scope::#name))
            }
            PgType::Builtin(name) => {
                let name = name.strip_prefix("pg_catalog.").unwrap_or(name);
                Ok(match name {
                    "bool" | "boolean" => quote!(::core::primitive::bool),
                    "int2" | "smallint" => quote!(::core::primitive::i16),
                    "int4" | "integer" => quote!(::core::primitive::i32),
                    "int8" | "bigint" => quote!(::core::primitive::i64),
                    "oid" | "xid" | "cid" => quote!(::core::primitive::u32),
                    "float4" | "real" => quote!(::core::primitive::f32),
                    "float8" | "double precision" => quote!(::core::primitive::f64),
                    "numeric" | "decimal" => quote!(::serde_json::Number),
                    "json" | "jsonb" => quote!(::serde_json::Value),
                    "uuid" => quote!(::uuid::Uuid),
                    "date" => quote!(::chrono::NaiveDate),
                    "time" | "time without time zone" => quote!(::chrono::NaiveTime),
                    "timestamp" | "timestamp without time zone" => quote!(::chrono::NaiveDateTime),
                    "timestamptz" | "timestamp with time zone" => {
                        quote!(::chrono::DateTime<::chrono::FixedOffset>)
                    }
                    "void" => quote!(()),
                    "text"
                    | "varchar"
                    | "character varying"
                    | "bpchar"
                    | "character"
                    | "char"
                    | "name"
                    | "bytea"
                    | "inet"
                    | "cidr"
                    | "macaddr"
                    | "macaddr8"
                    | "interval"
                    | "timetz"
                    | "time with time zone"
                    | "point"
                    | "line"
                    | "lseg"
                    | "box"
                    | "path"
                    | "polygon"
                    | "circle"
                    | "int4range"
                    | "int8range"
                    | "numrange"
                    | "tsrange"
                    | "tstzrange"
                    | "daterange"
                    | "int4multirange"
                    | "int8multirange"
                    | "nummultirange"
                    | "tsmultirange"
                    | "tstzmultirange"
                    | "datemultirange"
                    | "tsvector"
                    | "tsquery"
                    | "bit"
                    | "varbit"
                    | "xml"
                    | "pg_lsn"
                    | "money" => quote!(::std::string::String),
                    _ => {
                        return Err(invalid(format!(
                            "unsupported PostgreSQL type pg_catalog.{name}; supply an explicit type_override"
                        )));
                    }
                })
            }
        }
    }

    fn fields(
        &self,
        columns: &[Column],
        depth: usize,
        mode: &str,
        context: &str,
    ) -> Result<TokenStream, Error> {
        unique(columns.iter().map(|c| c.name.as_str()), false, context)?;
        let mut fields = TokenStream::new();
        for column in columns {
            if matches!(mode, "insert" | "update")
                && (column.generated || column.identity == Identity::Always)
            {
                continue;
            }
            let name = ident(&column.name, false)?;
            let wire = &column.name;
            let base = self.ty(&column.ty, depth)?;
            let nullable = column.nullable || matches!(mode, "composite" | "view" | "record");
            let ty = if nullable {
                quote!(::std::option::Option<#base>)
            } else {
                base
            };
            let optional = mode == "update"
                || mode == "insert"
                    && (nullable || column.has_default || column.identity == Identity::ByDefault);
            if optional {
                let runtime = &self.runtime;
                let skip = format!(
                    "{}::Field::is_omit",
                    quote!(#runtime).to_string().replace(' ', "")
                );
                fields.extend(quote!(#[serde(rename = #wire, skip_serializing_if = #skip)] pub #name: #runtime::Field<#ty>,));
            } else {
                fields.extend(quote!(#[serde(rename = #wire)] pub #name: #ty,));
            }
        }
        Ok(fields)
    }

    fn is_record(&self, ty: &PgType) -> bool {
        match ty {
            PgType::Named { schema, name } => self
                .named
                .get(&(schema.clone(), name.clone()))
                .copied()
                .unwrap_or(false),
            PgType::Domain { base, .. } => self.is_record(base),
            PgType::Builtin(_) | PgType::Array(_) => false,
        }
    }

    fn function_returns(
        &mut self,
        function: &Function,
        target: &str,
    ) -> Result<(TokenStream, TokenStream), Error> {
        let mut record = TokenStream::new();
        let returns = match &function.returns {
            ReturnType::Record(columns) => {
                let decoration = self.decoration(&format!("{target}.Record"), true, false);
                let fields = self.fields(columns, 3, "record", target)?;
                record.extend(quote!(#decoration pub struct Record { #fields }));
                if function.returns_set {
                    quote!(::std::vec::Vec<Record>)
                } else {
                    quote!(Record)
                }
            }
            ReturnType::Type(ty) => {
                let base = self.ty(ty, 3)?;
                if matches!(ty, PgType::Builtin(name) if name == "void" || name == "pg_catalog.void")
                {
                    quote!(())
                } else if self.is_record(ty) {
                    if function.returns_set {
                        quote!(::std::vec::Vec<#base>)
                    } else {
                        base
                    }
                } else if function.returns_set {
                    quote!(::std::vec::Vec<::std::option::Option<#base>>)
                } else {
                    quote!(::std::option::Option<#base>)
                }
            }
        };
        Ok((record, returns))
    }
}

fn references(ty: &PgType, config: &Config, output: &mut BTreeSet<String>) {
    match ty {
        PgType::Named { schema, name } => {
            if !config
                .type_overrides
                .contains_key(&format!("{schema}.{name}"))
            {
                output.insert(schema.clone());
            }
        }
        PgType::Domain { schema, name, base } => {
            if !config
                .type_overrides
                .contains_key(&format!("{schema}.{name}"))
            {
                references(base, config, output);
            }
        }
        PgType::Array(base) => references(base, config, output),
        PgType::Builtin(_) => {}
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "One schema traversal keeps emitted nesting and relative type paths together."
)]
pub fn generate(snapshot: &Snapshot, config: &Config) -> Result<String, Error> {
    if snapshot.version != SNAPSHOT_VERSION {
        return Err(invalid(format!(
            "unsupported snapshot version {}",
            snapshot.version
        )));
    }
    let runtime = syn::parse_str::<syn::Path>(&config.runtime_path)
        .map_err(|e| invalid(format!("invalid runtime path: {e}")))?;
    let prelude =
        syn::parse_file(&config.prelude).map_err(|e| invalid(format!("invalid prelude: {e}")))?;
    let derives = config
        .derives
        .iter()
        .map(|d| {
            syn::parse_str::<syn::Path>(d)
                .map_err(|e| invalid(format!("invalid derive {d:?}: {e}")))
        })
        .collect::<Result<_, _>>()?;
    let override_types: BTreeMap<String, syn::Type> = config
        .type_overrides
        .iter()
        .map(|(key, value)| {
            Ok((
                key.clone(),
                syn::parse_str::<syn::Type>(value)
                    .map_err(|e| invalid(format!("invalid override for {key}: {e}")))?,
            ))
        })
        .collect::<Result<_, Error>>()?;
    let (aliases, overrides) = override_aliases(&prelude, override_types);
    let targeted = config
        .type_attributes
        .iter()
        .map(|(key, value)| Ok((key.clone(), attributes(value)?)))
        .collect::<Result<_, Error>>()?;
    unique(
        snapshot.schemas.iter().map(|s| s.name.as_str()),
        false,
        "schemas",
    )?;
    let schemas: BTreeMap<_, _> = snapshot
        .schemas
        .iter()
        .map(|s| (s.name.clone(), s))
        .collect();
    let mut included: BTreeSet<_> = config.schemas.iter().cloned().collect();
    if included.is_empty() {
        return Err(invalid("at least one schema must be selected"));
    }
    loop {
        let before = included.len();
        for name in included.clone() {
            let schema = schemas
                .get(&name)
                .ok_or_else(|| invalid(format!("schema {name:?} is absent from snapshot")))?;
            for ty in schema
                .tables
                .iter()
                .flat_map(|t| t.columns.iter().map(|c| &c.ty))
                .chain(
                    schema
                        .composites
                        .iter()
                        .flat_map(|c| c.fields.iter().map(|c| &c.ty)),
                )
                .chain(
                    schema
                        .functions
                        .iter()
                        .flat_map(|f| f.arguments.iter().map(|a| &a.ty)),
                )
            {
                references(ty, config, &mut included);
            }
            for function in &schema.functions {
                match &function.returns {
                    ReturnType::Type(ty) => references(ty, config, &mut included),
                    ReturnType::Record(fields) => {
                        for field in fields {
                            references(&field.ty, config, &mut included);
                        }
                    }
                }
            }
        }
        if included.len() == before {
            break;
        }
    }
    let mut emitter = Emitter {
        config,
        runtime,
        derives,
        attributes: attributes(&config.attributes)?,
        targeted,
        used_targets: BTreeSet::new(),
        overrides,
        named: BTreeMap::new(),
    };
    for name in &included {
        let schema = schemas
            .get(name)
            .ok_or_else(|| invalid(format!("schema {name:?} is absent from snapshot")))?;
        unique(
            schema.enums.iter().map(|e| e.name.as_str()),
            true,
            &format!("{name}.enums"),
        )?;
        unique(
            schema.composites.iter().map(|e| e.name.as_str()),
            true,
            &format!("{name}.composites"),
        )?;
        for (type_name, composite) in schema
            .enums
            .iter()
            .map(|e| (&e.name, false))
            .chain(schema.composites.iter().map(|c| (&c.name, true)))
        {
            if emitter
                .named
                .insert((name.clone(), type_name.clone()), composite)
                .is_some()
            {
                return Err(invalid(format!("duplicate named type {name}.{type_name}")));
            }
        }
    }
    let mut modules = TokenStream::new();
    for schema_name in included {
        let schema = schemas
            .get(&schema_name)
            .ok_or_else(|| invalid(format!("schema {schema_name:?} is absent from snapshot")))?;
        let schema_ident = ident(&schema_name, false)?;
        let mut enums = TokenStream::new();
        let mut sorted: Vec<_> = schema.enums.iter().collect();
        sorted.sort_by_key(|e| &e.name);
        for enumeration in sorted {
            unique(
                enumeration.variants.iter().map(String::as_str),
                true,
                &format!("enum {schema_name}.{}", enumeration.name),
            )?;
            let name = ident(&enumeration.name, true)?;
            let target = format!("{schema_ident}.enums.{name}");
            let decoration = emitter.decoration(&target, true, false);
            let mut variants = TokenStream::new();
            for label in &enumeration.variants {
                let variant = ident(label, true)?;
                variants.extend(quote!(#[serde(rename = #label)] #variant,));
            }
            enums.extend(quote!(#decoration pub enum #name { #variants }));
        }
        let mut composites = TokenStream::new();
        let mut sorted: Vec<_> = schema.composites.iter().collect();
        sorted.sort_by_key(|c| &c.name);
        for composite in sorted {
            let name = ident(&composite.name, true)?;
            let target = format!("{schema_ident}.composites.{name}");
            let decoration = emitter.decoration(&target, true, false);
            let fields = emitter.fields(&composite.fields, 2, "composite", &target)?;
            composites.extend(quote!(#decoration pub struct #name { #fields }));
        }
        unique(
            schema.tables.iter().map(|t| t.name.as_str()),
            false,
            &format!("{schema_name}.tables"),
        )?;
        let mut tables = TokenStream::new();
        let mut sorted: Vec<_> = schema.tables.iter().collect();
        sorted.sort_by_key(|t| &t.name);
        for table in sorted {
            let name = ident(&table.name, false)?;
            let target = format!("{schema_ident}.tables.{name}");
            let decoration = emitter.decoration(&format!("{target}.Row"), true, false);
            let fields = emitter.fields(
                &table.columns,
                3,
                if table.kind == TableKind::Table {
                    "row"
                } else {
                    "view"
                },
                &target,
            )?;
            let runtime = emitter.runtime.clone();
            let wire_name = &table.name;
            let mut writes = TokenStream::new();
            if table.kind == TableKind::Table {
                for (rust_name, mode) in [("Insert", "insert"), ("Update", "update")] {
                    let default = mode == "update"
                        || table
                            .columns
                            .iter()
                            .filter(|c| !c.generated && c.identity != Identity::Always)
                            .all(|c| {
                                c.nullable || c.has_default || c.identity == Identity::ByDefault
                            });
                    let decoration =
                        emitter.decoration(&format!("{target}.{rust_name}"), false, default);
                    let fields = emitter.fields(&table.columns, 3, mode, &target)?;
                    let rust_name = Ident::new(rust_name, Span::call_site());
                    writes.extend(quote!(#decoration pub struct #rust_name { #fields }));
                }
            }
            tables.extend(quote!(pub mod #name { #[allow(unused_imports)] use super::*; #decoration pub struct Row { #fields } impl #runtime::Relation for Row { const SCHEMA: &'static str = #schema_name; const NAME: &'static str = #wire_name; } #writes }));
        }
        let mut functions = TokenStream::new();
        let mut groups: BTreeMap<&str, Vec<&Function>> = BTreeMap::new();
        for function in &schema.functions {
            groups.entry(&function.name).or_default().push(function);
        }
        let mut function_names = BTreeSet::new();
        for (wire_name, mut overloads) in groups {
            overloads.sort_by_key(|f| {
                f.arguments
                    .iter()
                    .map(|a| format!("{}:{:?}", a.name, a.ty))
                    .collect::<Vec<_>>()
            });
            let count = overloads.len();
            let mut signatures = BTreeSet::new();
            for (index, function) in overloads.into_iter().enumerate() {
                let signature = function
                    .arguments
                    .iter()
                    .map(|a| format!("{}:{:?}", a.name, a.ty))
                    .collect::<Vec<_>>();
                if !signatures.insert(signature) {
                    return Err(invalid(format!(
                        "duplicate RPC signature {schema_name}.{wire_name}"
                    )));
                }
                let base = ident(wire_name, false)?.to_string();
                let generated = if count == 1 {
                    base
                } else {
                    format!("{}_{index}", base.strip_prefix("r#").unwrap_or(&base))
                };
                if !function_names.insert(generated.clone()) {
                    return Err(invalid(format!(
                        "RPC module name collision {schema_name}.{generated}"
                    )));
                }
                let name =
                    syn::parse_str::<Ident>(&generated).map_err(|e| invalid(e.to_string()))?;
                let target = format!("{schema_ident}.functions.{name}");
                unique(
                    function.arguments.iter().map(|a| a.name.as_str()),
                    false,
                    &target,
                )?;
                let decoration = emitter.decoration(
                    &format!("{target}.Args"),
                    false,
                    function.arguments.iter().all(|a| a.has_default),
                );
                let mut args = TokenStream::new();
                for argument in &function.arguments {
                    let field = ident(&argument.name, false)?;
                    let wire = &argument.name;
                    let base = emitter.ty(&argument.ty, 3)?;
                    if argument.has_default {
                        let runtime = &emitter.runtime;
                        let skip = format!(
                            "{}::Field::is_omit",
                            quote!(#runtime).to_string().replace(' ', "")
                        );
                        args.extend(quote!(#[serde(rename = #wire, skip_serializing_if = #skip)] pub #field: #runtime::Field<::std::option::Option<#base>>,));
                    } else {
                        args.extend(quote!(#[serde(rename = #wire)] pub #field: ::std::option::Option<#base>,));
                    }
                }
                let (record, returns) = emitter.function_returns(function, &target)?;
                let runtime = &emitter.runtime;
                functions.extend(quote!(pub mod #name { #[allow(unused_imports)] use super::*; #decoration pub struct Args { #args } #record pub type Returns = #returns; pub struct Function; impl #runtime::Function for Function { type Args = Args; type Returns = Returns; const SCHEMA: &'static str = #schema_name; const NAME: &'static str = #wire_name; } }));
            }
        }
        modules.extend(quote!(pub mod #schema_ident { #[allow(unused_imports)] use super::*; pub mod enums { #[allow(unused_imports)] use super::*; #enums } pub mod composites { #[allow(unused_imports)] use super::*; #composites } pub mod tables { #[allow(unused_imports)] use super::*; #tables } pub mod functions { #[allow(unused_imports)] use super::*; #functions } }));
    }
    for target in emitter.config.type_attributes.keys() {
        if !emitter.used_targets.contains(target) {
            return Err(invalid(format!(
                "unknown generated type attribute target {target:?}"
            )));
        }
    }
    let file = syn::parse2::<syn::File>(quote!(#prelude #aliases #modules))
        .map_err(|e| invalid(format!("generated Rust is invalid: {e}")))?;
    Ok(prettyplease::unparse(&file))
}

#[cfg(test)]
#[path = "emitter_tests.rs"]
mod tests;
