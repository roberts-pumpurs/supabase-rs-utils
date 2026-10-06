//! Versioned, portable `PostgreSQL` schema metadata. Snapshots contain no credentials or row data.

use serde::{Deserialize, Serialize};

/// Current on-disk snapshot format.
pub const SNAPSHOT_VERSION: u32 = 2;

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
