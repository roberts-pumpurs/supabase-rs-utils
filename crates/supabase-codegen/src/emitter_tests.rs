#![expect(
    clippy::indexing_slicing,
    reason = "These fixtures construct exactly one schema and the indexed table before access."
)]
#![expect(
    clippy::unwrap_used,
    reason = "Unexpected generation failures must print their diagnostic in tests."
)]

use super::*;

fn snapshot() -> Snapshot {
    Snapshot {
        version: SNAPSHOT_VERSION,
        schemas: vec![Schema {
            name: "public".into(),
            enums: vec![],
            composites: vec![],
            tables: vec![],
            functions: vec![],
        }],
    }
}

fn column(name: &str) -> Column {
    Column {
        name: name.into(),
        ty: PgType::Builtin("text".into()),
        nullable: false,
        has_default: false,
        generated: false,
        identity: Identity::None,
    }
}

#[test]
fn normalized_name_collisions_and_unknown_attribute_targets_fail() {
    let mut metadata = snapshot();
    metadata.schemas[0].tables.push(Table {
        name: "messages".into(),
        kind: TableKind::Table,
        columns: vec![column("self"), column("Self")],
    });
    let generator = crate::Generator::new();
    assert!(matches!(
        generate(&metadata, &generator.config),
        Err(Error::Invalid(_))
    ));
    metadata.schemas[0].tables[0].columns.pop();
    let generator = generator.type_attribute("public.tables.missing.Insert", "#[allow(dead_code)]");
    assert!(matches!(
        generate(&metadata, &generator.config),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn output_is_stable_across_metadata_object_order() {
    let mut metadata = snapshot();
    for name in ["zebra", "alpha"] {
        metadata.schemas[0].tables.push(Table {
            name: name.into(),
            kind: TableKind::Table,
            columns: vec![column("value")],
        });
    }
    let generator = crate::Generator::new();
    let first = generate(&metadata, &generator.config).unwrap();
    metadata.schemas[0].tables.reverse();
    assert_eq!(first, generate(&metadata, &generator.config).unwrap());
}

#[test]
fn unsupported_types_fail_without_a_mapping() {
    let mut metadata = snapshot();
    let mut value = column("value");
    value.ty = PgType::Named {
        schema: "extensions".into(),
        name: "opaque".into(),
    };
    metadata.schemas[0].tables.push(Table {
        name: "messages".into(),
        kind: TableKind::Table,
        columns: vec![value],
    });
    assert!(matches!(
        crate::Generator::new().from_metadata(metadata),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn invalid_custom_rust_is_rejected_before_emission() {
    let generators = [
        crate::Generator::new().prelude("use broken::;"),
        crate::Generator::new().derive("not a path"),
        crate::Generator::new().attribute("#[broken("),
        crate::Generator::new().type_override("pg_catalog.text", "Vec<"),
        crate::Generator::new().runtime_path("not a path"),
    ];
    for generator in generators {
        assert!(matches!(
            generator.from_metadata(snapshot()),
            Err(Error::Invalid(_))
        ));
    }
}
