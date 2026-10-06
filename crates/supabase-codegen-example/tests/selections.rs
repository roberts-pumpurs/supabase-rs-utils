#[allow(dead_code)]
mod bindings {
    rp_supabase_client::include_schema!("contract_bindings.rs");
}
#[allow(dead_code)]
mod database {
    rp_supabase_client::include_schema!("database.rs");
}
use database::public;
use rp_supabase_client::schema::Selection;
#[expect(
    dead_code,
    reason = "the shared example also contains its offline entry point"
)]
#[path = "../src/relationship_projections.rs"]
mod relationship_projections;

#[test]
fn local_records_are_strict_and_preserve_sql_names() {
    use bindings::public::tables::typed_probe::Row as Root;
    use rp_supabase_client::{key, select};
    let selected = select!(Root => { id, display_name });
    assert_eq!(core::mem::size_of_val(&selected), 0);
    let rows = selected
        .decode(r#"[{"id":7,"display.name":null,"unknown":true}]"#)
        .unwrap();
    assert_eq!(rows[0].id, 7);
    assert_eq!(rows[0].display_name, None);
    for invalid in [
        r#"[{"id":7}]"#,
        r#"[{"display.name":null}]"#,
        r#"[{"id":7,"display.name":null,"display.name":null}]"#,
        r#"[{"id":7,"id":8,"display.name":null}]"#,
    ] {
        assert!(selected.decode(invalid).is_err(), "{invalid}");
    }
    let _ = selected
        .query(rp_supabase_client::rp_postgrest::Postgrest::new("http://localhost").unwrap())
        .eq(selected.column(key!(id)), &7)
        .into_raw();
}

#[test]
fn exact_schema_identity_raw_and_unicode_keys() {
    use bindings::shared::tables::typed_probe::Row as Root;
    use rp_supabase_client::{key, select};
    let selected = select!(Root => { id, r#type, café });
    let rows = selected
        .decode(r#"[{"id":"text-id","type":"raw","café":"unicode"}]"#)
        .unwrap();
    let _: &String = &rows[0].id;
    assert_eq!(rows[0].r#type, "raw");
    assert_eq!(rows[0].café, "unicode");
    let _ = selected.column(key!(r#type));
    let _ = selected.column(key!(café));
}

#[test]
fn named_descendants_inline_inner_and_predicate_only_decode() {
    use database::public::tables::orders::Row as Root;
    use relationship_projections::AddressSummary;
    use rp_supabase_client::{key, select};
    let selected = select!(Root => {
        id,
        billing: embed(orders_billing, AddressSummary),
        shipping: inner(orders_shipping) { label },
        matched: empty(order_details_details_order, inner)
    });
    let rows = selected.decode(r#"[{"id":7,"billing":{"id":8,"label":"bill","country":{"id":1,"name":"France"}},"shipping":{"label":"ship"}}]"#).unwrap();
    assert_eq!(
        rows[0]
            .billing
            .as_ref()
            .unwrap()
            .country
            .as_ref()
            .unwrap()
            .name,
        "France"
    );
    assert_eq!(rows[0].shipping.as_ref().unwrap().label, "ship");
    let client = rp_supabase_client::rp_postgrest::Postgrest::new("http://localhost").unwrap();
    let _ = selected
        .query(client)
        .embedded(selected.billing.then(AddressSummary::country), |country| {
            country.eq(database::public::tables::countries::columns::name, "France");
        })
        .embedded(selected.billing, |address| {
            address.embedded(AddressSummary::country, |country| {
                country.eq(database::public::tables::countries::columns::name, "France");
            });
        })
        .embedded(selected.shipping, |address| {
            address.eq(database::public::tables::addresses::columns::id, &8);
        })
        .exists(selected.matched)
        .eq(selected.column(key!(id)), &7)
        .into_raw();
}

#[test]
#[allow(non_camel_case_types)]
fn adversarial_consumer_names_preserve_owned_records_and_strict_embeds() {
    use database::public::tables::orders::Row as __Record;
    use rp_supabase_client as __Descriptor;
    type __Alias0 = relationship_projections::AddressSummary;
    struct str;
    struct usize;
    let _ = (str, usize);

    let selected = __Descriptor::select!(runtime = __Descriptor; __Record => {
        id,
        billing: embed(orders_billing, __Alias0),
        shipping: orders_shipping { label }
    });
    let rows = {
        let input = ::std::string::String::from(
            r#"[{"id":7,"billing":{"id":8,"label":"owned bill","country":null},"shipping":{"label":"owned ship"}}]"#,
        );
        selected.decode(&input).unwrap()
    };
    let _: &::std::string::String = &rows[0].billing.as_ref().unwrap().label;
    let _: &::std::string::String = &rows[0].shipping.as_ref().unwrap().label;
    assert_eq!(rows[0].id, 7);
    assert_eq!(rows[0].billing.as_ref().unwrap().label, "owned bill");
    assert_eq!(rows[0].shipping.as_ref().unwrap().label, "owned ship");
    assert!(rows[0].billing.as_ref().unwrap().country.is_none());

    let nullable = selected
        .decode(r#"[{"id":7,"billing":null,"shipping":null}]"#)
        .unwrap();
    assert!(nullable[0].billing.is_none());
    assert!(nullable[0].shipping.is_none());
    for invalid in [
        r#"[{"id":7,"shipping":null}]"#,
        r#"[{"id":7,"billing":null}]"#,
        r#"[{"id":7,"billing":null,"billing":null,"shipping":null}]"#,
        r#"[{"id":7,"billing":null,"shipping":null,"shipping":null}]"#,
        r#"[{"id":7,"billing":null,"shipping":{}}]"#,
        r#"[{"id":7,"billing":null,"shipping":{"label":"first","label":"second"}}]"#,
    ] {
        assert!(selected.decode(invalid).is_err(), "{invalid}");
    }
}

#[tokio::test]
async fn query_first_filters_do_not_overfetch_sql_named_columns() {
    use bindings::public::tables::typed_probe::Row as Root;
    use rp_supabase_client::{key, select};
    let selected = select!(Root => { id });
    let mut server = mockito::Server::new_async().await;
    let request = server
        .mock("GET", "/typed%2Eprobe")
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("select".into(), "id".into()),
            mockito::Matcher::UrlEncoded("\"display.name\"".into(), "eq.visible".into()),
        ]))
        .with_body(r#"[{"id":7}]"#)
        .create_async()
        .await;
    let rows = selected
        .query(rp_supabase_client::rp_postgrest::Postgrest::new(server.url()).unwrap())
        .eq(selected.column(key!(display_name)), "visible")
        .eq(selected.column(key!(id)), &7)
        .fetch()
        .await
        .unwrap();
    assert_eq!(rows[0].id, 7);
    request.assert_async().await;
}
