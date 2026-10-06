use rp_supabase_codegen::Generator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=schema.json");
    println!("cargo:rerun-if-changed=tests/schema.json");
    println!("cargo:rerun-if-env-changed=SUPABASE_CODEGEN_DATABASE_URL");
    let generator = Generator::new()
        .schema("public")
        .prelude("use ::std::string::String as DatabaseText;")
        .derive("PartialEq")
        .attribute("#[allow(dead_code)]")
        .type_override("pg_catalog.text", "DatabaseText")
        .type_attribute(
            "public.tables.messages.Insert",
            "#[derive(::typed_builder::TypedBuilder)]",
        )
        .type_attribute(
            "public.tables.messages.Insert",
            "#[builder(field_defaults(setter(into)))]",
        );
    // Live metadata is an explicit opt-in. Invalid credentials never fall back to the snapshot.
    let bindings = if std::env::var_os("SUPABASE_CODEGEN_DATABASE_URL").is_some() {
        generator.from_database_env("SUPABASE_CODEGEN_DATABASE_URL")?
    } else {
        generator.from_snapshot("schema.json")?
    };
    bindings.write_to_out_dir("database.rs")?;
    // Compile consumer regressions using an alias that deliberately shares Row's name.
    Generator::new()
        .prelude("use ::std::string::String as Row;")
        .type_override("pg_catalog.text", "Row")
        .from_snapshot("tests/schema.json")?
        .write_to_out_dir("contract_bindings.rs")?;
    Ok(())
}
