//! Check real consumer snippets in an isolated Cargo project.
//! A positive control detects compiler and dependency setup failures.
use std::{fs, path::PathBuf, process::Command};

#[test]
fn typed_contract_rejects_invalid_consumers() {
    let directory =
        std::env::temp_dir().join(format!("supabase-typed-consumers-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let client = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../supabase-client");
    let manifest = format!(
        r#"[package]
name = "supabase-typed-consumers"
version = "0.0.0"
edition = "2024"
[workspace]
[lib]
path = "consumer.rs"
[dependencies]
rp-supabase-client = {{ path = {client:?} }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = {{ version = "1", features = ["arbitrary_precision"] }}
chrono = {{ version = "0.4", default-features = false, features = ["serde", "clock", "std"] }}
typed-builder = "0.21"
"#
    );
    fs::write(directory.join("Cargo.toml"), manifest).unwrap();
    fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock"),
        directory.join("Cargo.lock"),
    )
    .unwrap();
    let header = format!(
        "#![allow(dead_code)]\nmod bindings {{ include!({:?}); }}\nmod database {{ include!({:?}); }}\nuse bindings::public::tables;\nuse database::public;\nuse database::public::tables as rel;\nuse rp_supabase_client::postgrest::Postgrest;\n#[path = {:?}] mod relationship_projections;\nuse relationship_projections::*;\nrp_supabase_client::projection! {{ struct Id for bindings::public::tables::a_b {{ id }} }}\nrp_supabase_client::projection! {{ struct OtherOrder for database::public::tables::orders {{ id, billing: embed(database::public::tables::orders::relationships::orders_billing, AddressSummary) }} }}\n",
        PathBuf::from(env!("OUT_DIR")).join("contract_bindings.rs"),
        PathBuf::from(env!("OUT_DIR")).join("database.rs"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/relationship_projections.rs"),
    );
    let cases = [
        (
            "valid",
            "fn consumer(c: Postgrest) { let _ = tables::a_b::query(c).select::<Id>().eq(tables::a_b::columns::id, &7); }",
            true,
        ),
        (
            "unknown_column",
            "fn consumer(c: Postgrest) { let _ = tables::a_b::query(c).eq(tables::a_b::columns::missing, &7); }",
            false,
        ),
        (
            "wrong_relation_column",
            "fn consumer(c: Postgrest) { let _ = tables::a_b::query(c).eq(tables::bool::columns::value, &true); }",
            false,
        ),
        (
            "wrong_value",
            "fn consumer(c: Postgrest) { let _ = tables::a_b::query(c).eq(tables::a_b::columns::id, &\"seven\"); }",
            false,
        ),
        (
            "wrong_projection",
            "fn consumer(c: Postgrest) { let _ = tables::bool::query(c).select::<Id>(); }",
            false,
        ),
        (
            "wrong_payload",
            "fn consumer(c: Postgrest) { let _ = tables::a_b::query(c).insert(&tables::bool::Insert { value: true }); }",
            false,
        ),
        (
            "raw_payload",
            "fn consumer(c: Postgrest) { let _ = tables::a_b::query(c).update(&serde_json::json!({\"id\":7})); }",
            false,
        ),
        (
            "unselected_field",
            "fn consumer(row: Id) { let _ = row.body; }",
            false,
        ),
        (
            "nonnull_null",
            "fn consumer(c: Postgrest) { let _ = tables::a_b::query(c).is_null(tables::a_b::columns::id); }",
            false,
        ),
        (
            "nullable_value",
            "fn consumer(c: Postgrest) { let _ = tables::typed_probe::query(c).eq(tables::typed_probe::columns::display_name, &Some(String::from(\"name\"))); }",
            false,
        ),
        (
            "invalid_projection_column",
            "rp_supabase_client::projection! { struct Invalid for bindings::public::tables::a_b { missing } }",
            false,
        ),
        (
            "view_write",
            "fn consumer(c: Postgrest) { let _ = database::public::tables::message_summaries::query(c).delete(); }",
            false,
        ),
        (
            "second_mutation",
            "fn consumer(c: Postgrest) { let _ = tables::a_b::query(c).delete().delete(); }",
            false,
        ),
        (
            "valid_nested_locked_write",
            "fn consumer(c: Postgrest) { let _ = rel::customers::query(c).select::<CustomerSummary>().embedded(CustomerSummary::orders.then(OrderSummary::billing).then(AddressSummary::country), |child| { child.eq(rel::countries::columns::name, \"literal\"); }).exists(CustomerSummary::orders).delete().eq(rel::customers::columns::id, &7); }",
            true,
        ),
        (
            "valid_empty_scoped_and_write_first",
            "fn consumer(c: Postgrest) { let _ = rel::customers::query(c).select::<CustomerPredicates>().delete().embedded(CustomerPredicates::matching_orders, |child| { child.eq(rel::orders::columns::id, &7); }).not_exists(CustomerPredicates::matching_orders); }",
            true,
        ),
        (
            "wrong_embed_source",
            "rp_supabase_client::projection! { struct WrongSource for database::public::tables::customers { id, address: embed(database::public::tables::orders::relationships::orders_billing, AddressSummary) } }",
            false,
        ),
        (
            "wrong_embed_target",
            "rp_supabase_client::projection! { struct WrongTarget for database::public::tables::orders { id, address: embed(database::public::tables::orders::relationships::orders_billing, CountrySummary) } }",
            false,
        ),
        (
            "wrong_root_handle",
            "fn consumer(c: Postgrest) { let _ = rel::customers::query(c).select::<CustomerSummary>().embedded(OrderSummary::billing, |_| {}); }",
            false,
        ),
        (
            "unselected_child_projection_handle",
            "fn consumer(c: Postgrest) { let _ = rel::customers::query(c).select::<CustomerSummary>().embedded(CustomerSummary::orders.then(OtherOrder::billing), |_| {}); }",
            false,
        ),
        (
            "unselected_handle_at_root",
            "fn consumer(c: Postgrest) { let _ = rel::orders::query(c).embedded(OrderSummary::billing, |_| {}); }",
            false,
        ),
        (
            "wrong_scoped_column",
            "fn consumer(c: Postgrest) { let _ = rel::orders::query(c).select::<OrderSummary>().embedded(OrderSummary::billing, |child| { child.eq(rel::countries::columns::id, &7); }); }",
            false,
        ),
        (
            "wrong_scoped_value",
            "fn consumer(c: Postgrest) { let _ = rel::orders::query(c).select::<OrderSummary>().embedded(OrderSummary::billing, |child| { child.eq(rel::addresses::columns::id, \"seven\"); }); }",
            false,
        ),
        (
            "select_after_embedded",
            "fn consumer(c: Postgrest) { let _ = rel::orders::query(c).select::<OrderSummary>().embedded(OrderSummary::billing, |_| {}).select::<OrderInner>(); }",
            false,
        ),
        (
            "select_after_exists",
            "fn consumer(c: Postgrest) { let _ = rel::customers::query(c).select::<CustomerPredicates>().exists(CustomerPredicates::matching_orders).select::<CustomerSummary>(); }",
            false,
        ),
        (
            "select_after_not_exists",
            "fn consumer(c: Postgrest) { let _ = rel::customers::query(c).select::<CustomerPredicates>().not_exists(CustomerPredicates::matching_orders).select::<CustomerSummary>(); }",
            false,
        ),
        (
            "select_after_locked_write",
            "fn consumer(c: Postgrest) { let _ = rel::orders::query(c).select::<OrderSummary>().embedded(OrderSummary::billing, |_| {}).delete().select::<OrderInner>(); }",
            false,
        ),
        (
            "wrong_exists_handle",
            "fn consumer(c: Postgrest) { let _ = rel::customers::query(c).select::<CustomerPredicates>().exists(OrderSummary::billing); }",
            false,
        ),
        (
            "empty_has_no_decoded_field",
            "fn consumer(row: CustomerPredicates) { let _ = row.matching_orders; }",
            false,
        ),
        (
            "empty_has_no_selected_child_handle",
            "fn consumer(c: Postgrest) { let _ = rel::customers::query(c).select::<CustomerPredicates>().embedded(CustomerPredicates::matching_orders.then(OrderSummary::billing), |_| {}); }",
            false,
        ),
        (
            "root_rejects_child_column",
            "fn consumer(c: Postgrest) { let _ = rel::orders::query(c).select::<OrderSummary>().eq(rel::addresses::columns::id, &7); }",
            false,
        ),
        (
            "duplicate_scalar_embed_alias",
            "rp_supabase_client::projection! { struct Duplicate for database::public::tables::orders { id, id: embed(database::public::tables::orders::relationships::orders_billing, AddressSummary) } }",
            false,
        ),
        (
            "duplicate_embed_alias",
            "rp_supabase_client::projection! { struct Duplicate for database::public::tables::orders { billing: embed(database::public::tables::orders::relationships::orders_billing, AddressSummary), billing: empty(database::public::tables::orders::relationships::orders_shipping) } }",
            false,
        ),
        (
            "to_one_is_not_unconditional",
            "fn consumer(row: OrderInner) { let _: AddressSummary = row.billing; }",
            false,
        ),
        (
            "to_many_is_not_optional",
            "fn consumer(row: CustomerSummary) { let _: Option<OrderSummary> = row.orders; }",
            false,
        ),
    ];
    for (name, source, succeeds) in cases {
        fs::write(
            directory.join("consumer.rs"),
            format!("{header}\n{source}\n"),
        )
        .unwrap();
        let mut command = Command::new(env!("CARGO"));
        command
            .current_dir(&directory)
            .args(["check", "--offline", "--quiet"]);
        if let Some(toolchain) = option_env!("RUSTUP_TOOLCHAIN") {
            command.env("RUSTUP_TOOLCHAIN", toolchain);
        }
        let output = command.output().unwrap();
        assert_eq!(
            output.status.success(),
            succeeds,
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fs::remove_dir_all(directory).unwrap();
}
