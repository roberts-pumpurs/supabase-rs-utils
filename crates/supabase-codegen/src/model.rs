//! Versioned, portable `PostgreSQL` schema metadata. Snapshots contain no credentials or row data.

use serde::{Deserialize, Serialize};

/// Current on-disk snapshot format.
pub const SNAPSHOT_VERSION: u32 = 3;

/// Metadata that can be checked into source control for offline builds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub version: u32,
    pub schemas: Vec<Schema>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Schema {
    pub name: String,
    pub enums: Vec<Enum>,
    pub composites: Vec<Composite>,
    pub tables: Vec<Table>,
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Enum {
    pub name: String,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Composite {
    pub name: String,
    pub fields: Vec<Column>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Table {
    pub name: String,
    pub kind: TableKind,
    pub columns: Vec<Column>,
    #[serde(deserialize_with = "Deserialize::deserialize")]
    pub primary_key: Option<Vec<String>>,
    pub unique_keys: Vec<Vec<String>>,
    pub foreign_keys: Vec<ForeignKey>,
    pub is_partition: bool,
}

/// Qualified relation identity, including targets outside the selected schemas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationRef {
    pub schema: String,
    pub name: String,
}

/// A direct catalog foreign key, with both column lists in matching constraint order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForeignKey {
    pub name: String,
    pub columns: Vec<String>,
    pub referenced_relation: RelationRef,
    pub referenced_columns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableKind {
    Table,
    View,
    MaterializedView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Column {
    pub name: String,
    pub ty: PgType,
    pub nullable: bool,
    pub has_default: bool,
    pub generated: bool,
    pub identity: Identity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Identity {
    None,
    Always,
    ByDefault,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum PgType {
    Builtin(String),
    Named {
        schema: String,
        name: String,
    },
    Array(Box<Self>),
    Domain {
        schema: String,
        name: String,
        base: Box<Self>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Function {
    pub name: String,
    pub arguments: Vec<Argument>,
    pub returns: ReturnType,
    pub returns_set: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Argument {
    pub name: String,
    pub ty: PgType,
    pub has_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ReturnType {
    Type(PgType),
    Record(Vec<Column>),
}

pub(crate) fn set_not_null(columns: &mut [Column], fields: &[String], target: &str) -> Result<(), crate::Error> {
    for field in fields {
        let column = columns.iter_mut().find(|column| &column.name == field)
            .ok_or_else(|| crate::Error::Invalid(format!("unknown @not_null field {target}.{field}")))?;
        column.nullable = false;
    }
    Ok(())
}

pub(crate) fn annotation(comment: Option<&str>, tag: &str) -> Result<Vec<String>, crate::Error> {
    let mut fields = Vec::new();
    for line in comment.unwrap_or_default().lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(tag) {
            if !rest.starts_with(char::is_whitespace) {
                return Err(crate::Error::Invalid(format!("invalid {tag} annotation")));
            }
            let names: Vec<_> = rest.split([',', ' ', '\t']).filter(|name| !name.is_empty()).map(str::to_owned).collect();
            if names.is_empty() {
                return Err(crate::Error::Invalid(format!("empty {tag} annotation")));
            }
            fields.extend(names);
        }
    }
    Ok(fields)
}

pub(crate) fn apply_not_null(snapshot: &mut Snapshot, targets: &std::collections::BTreeMap<String, Vec<String>>) -> Result<(), crate::Error> {
    for (target, fields) in targets {
        let mut found = false;
        for schema in &mut snapshot.schemas {
            for table in &mut schema.tables {
                if target == &format!("{}.tables.{}.Row", schema.name, table.name) && table.kind != TableKind::Table {
                    set_not_null(&mut table.columns, fields, target)?;
                    found = true;
                }
            }
            for composite in &mut schema.composites {
                use heck::ToUpperCamelCase as _;
                if target == &format!("{}.composites.{}", schema.name, composite.name.to_upper_camel_case()) {
                    set_not_null(&mut composite.fields, fields, target)?;
                    found = true;
                }
            }
            for function in &mut schema.functions {
                if target == &format!("{}.functions.{}.Record", schema.name, function.name) {
                    if let ReturnType::Record(columns) = &mut function.returns {
                        set_not_null(columns, fields, target)?;
                        found = true;
                    }
                }
            }
        }
        if !found {
            return Err(crate::Error::Invalid(format!("unknown not_null target {target:?}")));
        }
    }
    Ok(())
}

/// Only direct projections from one base relation qualify. Joins, CTEs,
/// grouping and expressions deliberately provide no inference.
pub(crate) fn view_columns(sql: &str) -> Option<(Vec<String>, Vec<Option<String>>)> {
    use sqlparser::{ast::{Expr, GroupByExpr, SelectItem, SetExpr, Statement, TableFactor}, dialect::PostgreSqlDialect, parser::Parser};
    let statements = Parser::parse_sql(&PostgreSqlDialect {}, sql).ok()?;
    let [Statement::Query(query)] = statements.as_slice() else { return None };
    if query.with.is_some() { return None; }
    let SetExpr::Select(select) = query.body.as_ref() else { return None };
    if !matches!(&select.group_by, GroupByExpr::Expressions(expressions, modifiers) if expressions.is_empty() && modifiers.is_empty()) {
        return None;
    }
    let [from] = select.from.as_slice() else { return None };
    if !from.joins.is_empty() { return None; }
    let TableFactor::Table { name, alias, args: None, .. } = &from.relation else { return None };
    let relation: Vec<String> = name.0.iter().map(|part| part.as_ident().map(|name| name.value.clone())).collect::<Option<_>>()?;
    let qualifier = alias.as_ref().map(|alias| alias.name.value.as_str()).or_else(|| relation.last().map(String::as_str))?;
    let columns = select.projection.iter().map(|item| {
        let expression = match item {
            SelectItem::UnnamedExpr(expression) | SelectItem::ExprWithAlias { expr: expression, .. } => expression,
            _ => return None,
        };
        match expression {
            Expr::Identifier(name) => Some(name.value.clone()),
            Expr::CompoundIdentifier(names) if names.len() == 2 && names.first().is_some_and(|name| name.value == qualifier) => names.last().map(|name| name.value.clone()),
            _ => None,
        }
    }).collect();
    Some((relation, columns))
}

#[cfg(test)]
mod contract_tests {
    use super::*;

    #[test]
    fn view_inference_rejects_nullable_sql_shapes() {
        assert_eq!(view_columns("SELECT t.id, t.id + 1 AS computed FROM public.t t"), Some((vec!["public".into(), "t".into()], vec![Some("id".into()), None])));
        for sql in [
            "SELECT a.id FROM a LEFT JOIN b ON a.id = b.id",
            "SELECT id FROM a UNION SELECT id FROM b",
            "SELECT id FROM a GROUP BY ROLLUP(id)",
            "WITH a AS (SELECT NULL AS id) SELECT id FROM a",
        ] {
            assert!(view_columns(sql).is_none());
        }
    }

    #[test]
    fn annotations_reject_empty_contracts() {
        assert_eq!(annotation(Some("@not_null id, name"), "@not_null").unwrap(), ["id", "name"]);
        assert!(annotation(Some("@not_null"), "@not_null").is_err());
    }

    #[test]
    fn check_membership_accepts_only_literal_exact_forms() {
        let labels = Some(vec!["organization".into(), "user".into()]);
        assert_eq!(check_values("owner_type IN ('user', 'organization')", "owner_type"), labels);
        assert_eq!(check_values("((owner_type = ANY (ARRAY['user'::text, 'organization'::text])))", "owner_type"), labels);
        for expression in ["owner_type NOT IN ('user')", "owner_type IN ('user', other)", "owner_type IN ('user') OR owner_type IS NULL", "lower(owner_type) IN ('user')", "owner_type::uuid IN ('user')"] {
            assert!(check_values(expression, "owner_type").is_none());
        }
    }
}

/// Recognize only a single string-valued column membership test. A compound
/// CHECK, negation, nonliteral item, or arbitrary cast never becomes an enum.
pub(crate) fn check_values(sql: &str, column: &str) -> Option<Vec<String>> {
    use sqlparser::{ast::{BinaryOperator, Expr, SelectItem, SetExpr, Statement, Value}, dialect::PostgreSqlDialect, parser::Parser};
    fn unnest(expression: &Expr) -> Option<&Expr> {
        match expression {
            Expr::Nested(inner) => unnest(inner),
            Expr::Cast { expr, data_type, .. } if matches!(data_type.to_string().as_str(), "TEXT" | "VARCHAR" | "CHAR" | "TEXT[]" | "VARCHAR[]" | "pg_catalog.text" | "pg_catalog.text[]") => unnest(expr),
            Expr::Cast { .. } => None,
            other => Some(other),
        }
    }
    let statements = Parser::parse_sql(&PostgreSqlDialect {}, &format!("SELECT {sql}")).ok()?;
    let [Statement::Query(query)] = statements.as_slice() else { return None };
    let SetExpr::Select(select) = query.body.as_ref() else { return None };
    let [SelectItem::UnnamedExpr(expression)] = select.projection.as_slice() else { return None };
    let (left, values) = match unnest(expression)? {
        Expr::InList { expr, list, negated: false } => (expr.as_ref(), list.as_slice()),
        Expr::AnyOp { left, compare_op: BinaryOperator::Eq, right, .. } => {
            let Expr::Array(array) = unnest(right)? else { return None };
            (left.as_ref(), array.elem.as_slice())
        }
        _ => return None,
    };
    if !matches!(unnest(left)?, Expr::Identifier(name) if name.value == column) || values.is_empty() { return None; }
    let mut labels = values.iter().map(|value| match unnest(value)? {
        Expr::Value(value) => match &value.value { Value::SingleQuotedString(label) => Some(label.clone()), _ => None },
        _ => None,
    }).collect::<Option<Vec<_>>>()?;
    labels.sort();
    labels.dedup();
    Some(labels)
}
