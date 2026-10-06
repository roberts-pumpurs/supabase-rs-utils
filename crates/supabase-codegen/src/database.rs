//! Direct catalog extraction; never reads user rows or embeds connection diagnostics.

#![expect(
    clippy::min_ident_chars,
    reason = "Short closure bindings keep catalog joins and ordering expressions readable."
)]

use alloc::collections::{BTreeMap, BTreeSet};

use postgres::{IsolationLevel, Row, Transaction};
use postgres_native_tls::MakeTlsConnector;

use crate::Error;
use crate::model::{
    Argument, Column, Composite, Enum, ForeignKey, Function, Identity, PgType, RelationRef,
    ReturnType, SNAPSHOT_VERSION, Schema, Snapshot, Table, TableKind,
};

const CATALOG: &str = include_str!("catalog.sql");

// Driver and server diagnostics can contain passwords, connection strings, or SQL values.
// Deliberately retain only the operation, never the underlying error's Display/source.
fn database_error(operation: &str) -> Error {
    Error::Database(format!(
        "PostgreSQL {operation} failed; check connection settings, certificates, and catalog permissions"
    ))
}

fn query(
    tx: &mut Transaction<'_>,
    section: &str,
    schemas: Option<&[String]>,
) -> Result<Vec<Row>, Error> {
    let sql = CATALOG
        .split("-- query: ")
        .skip(1)
        .find_map(|part| {
            let (name, sql) = part.split_once('\n')?;
            (name == section).then_some(sql)
        })
        .ok_or_else(|| Error::Invalid(format!("missing catalog query {section}")))?;
    match schemas {
        Some(schemas) => tx.query(sql, &[&schemas]),
        None => tx.query(sql, &[]),
    }
    .map_err(|_error| database_error("catalog query"))
}

#[derive(Clone)]
struct TypeInfo {
    schema: String,
    name: String,
    kind: String,
    base: i64,
    element: i64,
    array: i64,
    relation: i64,
    not_null: bool,
    has_default: bool,
}

#[derive(Clone)]
struct Attribute {
    name: String,
    oid: i64,
    not_null: bool,
    has_default: bool,
    generated: bool,
    identity: Identity,
}

struct Catalog {
    types: BTreeMap<i64, TypeInfo>,
    attributes: BTreeMap<i64, Vec<Attribute>>,
    enums: BTreeMap<i64, Vec<String>>,
    schemas: BTreeMap<String, Schema>,
    emitted: BTreeSet<i64>,
}

impl Catalog {
    fn schema(&mut self, name: &str) -> &mut Schema {
        self.schemas
            .entry(name.to_owned())
            .or_insert_with(|| Schema {
                name: name.to_owned(),
                enums: Vec::new(),
                composites: Vec::new(),
                tables: Vec::new(),
                functions: Vec::new(),
            })
    }

    fn info(&self, oid: i64) -> Result<&TypeInfo, Error> {
        self.types.get(&oid).ok_or_else(|| {
            Error::Invalid(format!(
                "catalog references missing PostgreSQL type OID {oid}"
            ))
        })
    }

    fn resolve(&mut self, oid: i64, domains: &mut BTreeSet<i64>) -> Result<PgType, Error> {
        let info = self.info(oid)?.clone();
        if info.kind == "d" {
            if !domains.insert(oid) {
                return Err(Error::Invalid(format!(
                    "cyclic domain {}.{}",
                    info.schema, info.name
                )));
            }
            let base = self.resolve(info.base, domains)?;
            domains.remove(&oid);
            return Ok(PgType::Domain {
                schema: info.schema,
                name: info.name,
                base: Box::new(base),
            });
        }
        // typelem alone also identifies non-array types such as int2vector. Verify
        // the inverse typarray relationship rather than guessing from type names.
        if info.element != 0 && self.info(info.element)?.array == oid {
            return Ok(PgType::Array(Box::new(
                self.resolve(info.element, domains)?,
            )));
        }
        if info.kind == "p" && info.name != "void" {
            return Err(Error::Invalid(format!(
                "unsupported PostgreSQL pseudotype {}.{}",
                info.schema, info.name
            )));
        }
        if info.kind == "e" && self.emitted.insert(oid) {
            let variants = self.enums.get(&oid).cloned().unwrap_or_default();
            self.schema(&info.schema).enums.push(Enum {
                name: info.name.clone(),
                variants,
            });
        }
        if info.kind == "c" && self.emitted.insert(oid) {
            let attributes = self
                .attributes
                .get(&info.relation)
                .cloned()
                .unwrap_or_default();
            let mut fields = Vec::with_capacity(attributes.len());
            for attribute in attributes {
                fields.push(self.column(&attribute, true)?);
            }
            self.schema(&info.schema).composites.push(Composite {
                name: info.name.clone(),
                fields,
            });
        }
        if info.schema == "pg_catalog" && info.kind != "e" && info.kind != "c" {
            Ok(PgType::Builtin(info.name))
        } else {
            // Extension/base/range types preserve their identity for explicit overrides.
            Ok(PgType::Named {
                schema: info.schema,
                name: info.name,
            })
        }
    }

    fn column(&mut self, attribute: &Attribute, conservative: bool) -> Result<Column, Error> {
        let mut oid = attribute.oid;
        let mut not_null = attribute.not_null;
        let mut has_default = attribute.has_default;
        let mut visited = BTreeSet::new();
        loop {
            let info = self.info(oid)?;
            if info.kind != "d" {
                break;
            }
            if !visited.insert(oid) {
                return Err(Error::Invalid(format!(
                    "cyclic domain {}.{}",
                    info.schema, info.name
                )));
            }
            not_null |= info.not_null;
            has_default |= info.has_default;
            oid = info.base;
        }
        Ok(Column {
            name: attribute.name.clone(),
            ty: self.resolve(attribute.oid, &mut BTreeSet::new())?,
            nullable: conservative || !not_null,
            has_default: !conservative && has_default,
            generated: !conservative && attribute.generated,
            identity: if conservative {
                Identity::None
            } else {
                attribute.identity
            },
        })
    }

    fn is_pseudo(&self, oid: i64) -> Result<bool, Error> {
        let info = self.info(oid)?;
        if info.kind == "d" {
            return self.is_pseudo(info.base);
        }
        Ok(info.kind == "p" && info.name != "void")
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "Catalog extraction remains one ordered read-only transaction over related metadata."
)]
pub fn introspect(url: &str, schemas: &[String]) -> Result<Snapshot, Error> {
    let mut config: postgres::Config = url
        .parse()
        .map_err(|_error| database_error("connection configuration"))?;
    // Never permit the driver's default Prefer mode to fall back to plaintext.
    // Explicit sslmode=disable remains available for local databases.
    if config.get_ssl_mode() == postgres::config::SslMode::Prefer {
        config.ssl_mode(postgres::config::SslMode::Require);
    }
    // native-tls defaults verify both the certificate chain and server hostname.
    let tls = native_tls::TlsConnector::builder()
        .build()
        .map_err(|_error| database_error("TLS initialization"))?;
    let mut client = config
        .connect(MakeTlsConnector::new(tls))
        .map_err(|_error| database_error("connection"))?;
    let mut tx = client
        .build_transaction()
        .isolation_level(IsolationLevel::RepeatableRead)
        .read_only(true)
        .start()
        .map_err(|_error| database_error("snapshot transaction"))?;
    let mut catalog = Catalog {
        types: BTreeMap::new(),
        attributes: BTreeMap::new(),
        enums: BTreeMap::new(),
        schemas: BTreeMap::new(),
        emitted: BTreeSet::new(),
    };
    for row in query(&mut tx, "schemas", Some(schemas))? {
        catalog.schema(row.get::<_, &str>("name"));
    }
    for schema in schemas {
        if !catalog.schemas.contains_key(schema) {
            return Err(Error::Invalid(format!(
                "requested PostgreSQL schema {schema:?} does not exist"
            )));
        }
    }
    for row in query(&mut tx, "types", None)? {
        catalog.types.insert(
            row.get("oid"),
            TypeInfo {
                schema: row.get("schema"),
                name: row.get("name"),
                kind: row.get("kind"),
                base: row.get("base"),
                element: row.get("element"),
                array: row.get("array"),
                relation: row.get("relation"),
                not_null: row.get("not_null"),
                has_default: row.get("has_default"),
            },
        );
    }
    for row in query(&mut tx, "enums", None)? {
        catalog
            .enums
            .entry(row.get("oid"))
            .or_default()
            .push(row.get("label"));
    }
    for row in query(&mut tx, "attributes", None)? {
        let identity: &str = row.get("identity");
        catalog
            .attributes
            .entry(row.get("relation"))
            .or_default()
            .push(Attribute {
                name: row.get("name"),
                oid: row.get("type_oid"),
                not_null: row.get("not_null"),
                has_default: row.get("has_default"),
                generated: !row.get::<_, &str>("generated").is_empty(),
                identity: match identity {
                    "a" => Identity::Always,
                    "d" => Identity::ByDefault,
                    _ => Identity::None,
                },
            });
    }
    // Include enums and composites in requested schemas, including nullable
    // relation-row composites, even if currently unreferenced.
    let roots: Vec<i64> = catalog
        .types
        .iter()
        .filter_map(|(&oid, info)| {
            (schemas.contains(&info.schema) && matches!(info.kind.as_str(), "e" | "c"))
                .then_some(oid)
        })
        .collect();
    for oid in roots {
        catalog.resolve(oid, &mut BTreeSet::new())?;
    }
    let mut constraints: BTreeMap<i64, Vec<Row>> = BTreeMap::new();
    for row in query(&mut tx, "constraints", Some(schemas))? {
        constraints
            .entry(row.get("relation"))
            .or_default()
            .push(row);
    }
    for row in query(&mut tx, "relations", Some(schemas))? {
        let schema: String = row.get("schema");
        let kind = match row.get::<_, &str>("kind") {
            "v" => TableKind::View,
            "m" => TableKind::MaterializedView,
            _ => TableKind::Table,
        };
        let attributes = catalog
            .attributes
            .get(&row.get::<_, i64>("oid"))
            .cloned()
            .unwrap_or_default();
        let mut columns = Vec::with_capacity(attributes.len());
        for attribute in attributes {
            columns.push(catalog.column(&attribute, kind != TableKind::Table)?);
        }
        let mut primary_key = None;
        let mut unique_keys = Vec::new();
        let mut foreign_keys = Vec::new();
        for constraint in constraints
            .remove(&row.get::<_, i64>("oid"))
            .unwrap_or_default()
        {
            match constraint.get::<_, &str>("kind") {
                "p" => primary_key = Some(constraint.get("columns")),
                "u" => unique_keys.push(constraint.get("columns")),
                "f" => foreign_keys.push(ForeignKey {
                    name: constraint.get("name"),
                    columns: constraint.get("columns"),
                    referenced_relation: RelationRef {
                        schema: constraint.get("referenced_schema"),
                        name: constraint.get("referenced_name"),
                    },
                    referenced_columns: constraint.get("referenced_columns"),
                }),
                _ => return Err(database_error("unsupported constraint kind")),
            }
        }
        catalog.schema(&schema).tables.push(Table {
            name: row.get("name"),
            kind,
            columns,
            primary_key,
            unique_keys,
            foreign_keys,
            is_partition: row.get("is_partition"),
        });
    }
    for row in query(&mut tx, "functions", Some(schemas))? {
        let types: Vec<i64> = row.get("types");
        let modes: Option<Vec<String>> = row.get("modes");
        let names: Option<Vec<String>> = row.get("names");
        let input_count: i32 = row.get("input_count");
        let defaults: i32 = row.get("defaults");
        let return_oid: i64 = row.get("return_oid");
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut eligible = true;
        let mut input_index = 0_i32;
        for (index, &oid) in types.iter().enumerate() {
            let mode = modes
                .as_ref()
                .and_then(|m| m.get(index))
                .map_or("i", String::as_str);
            let name = names
                .as_ref()
                .and_then(|n| n.get(index))
                .cloned()
                .unwrap_or_default();
            if matches!(mode, "i" | "b" | "v") {
                if name.is_empty() || catalog.is_pseudo(oid)? {
                    eligible = false;
                }
                inputs.push((
                    name.clone(),
                    oid,
                    input_index >= input_count.saturating_sub(defaults),
                ));
                input_index = input_index.saturating_add(1_i32);
            }
            if matches!(mode, "o" | "b" | "t") {
                outputs.push((name, oid));
            }
        }
        // Named-object RPC bindings intentionally exclude unnamed inputs and dynamic/
        // polymorphic pseudotypes. OUT records are eligible only with known named fields.
        if catalog.is_pseudo(return_oid)?
            && (catalog.info(return_oid)?.name != "record" || outputs.is_empty())
        {
            eligible = false;
        }
        for (name, oid) in &outputs {
            if name.is_empty() || catalog.is_pseudo(*oid)? {
                eligible = false;
            }
        }
        if !eligible {
            continue;
        }
        let mut arguments = Vec::with_capacity(inputs.len());
        for (name, oid, has_default) in inputs {
            arguments.push(Argument {
                name,
                ty: catalog.resolve(oid, &mut BTreeSet::new())?,
                has_default,
            });
        }
        let returns = if outputs.is_empty() {
            ReturnType::Type(catalog.resolve(return_oid, &mut BTreeSet::new())?)
        } else {
            let mut fields = Vec::with_capacity(outputs.len());
            for (name, oid) in outputs {
                fields.push(Column {
                    name,
                    ty: catalog.resolve(oid, &mut BTreeSet::new())?,
                    nullable: true,
                    has_default: false,
                    generated: false,
                    identity: Identity::None,
                });
            }
            ReturnType::Record(fields)
        };
        let schema: String = row.get("schema");
        catalog.schema(&schema).functions.push(Function {
            name: row.get("name"),
            arguments,
            returns,
            returns_set: row.get("returns_set"),
        });
    }
    tx.commit()
        .map_err(|_error| database_error("snapshot commit"))?;
    for schema in catalog.schemas.values_mut() {
        schema.enums.sort_by(|a, b| a.name.cmp(&b.name));
        schema.composites.sort_by(|a, b| a.name.cmp(&b.name));
    }
    Ok(Snapshot {
        version: SNAPSHOT_VERSION,
        schemas: catalog.schemas.into_values().collect(),
    })
}
