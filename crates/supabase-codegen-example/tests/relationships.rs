#[allow(dead_code)]
mod database {
    rp_supabase_client::include_schema!("database.rs");
}
use database::public;
use public::tables;
use rp_supabase_client::{rp_postgrest::Postgrest, schema::Projection};
use serde_json::json;

#[allow(dead_code)]
#[path = "../src/relationship_projections.rs"]
mod relationship_projections;
use relationship_projections::*;

rp_supabase_client::projection! {
    #[derive(Debug)]
    struct Aliases for crate::public::tables::orders {
        id,
        r#select: embed(crate::public::tables::orders::relationships::orders_billing, AddressSummary),
        r#type: embed(crate::public::tables::orders::relationships::orders_shipping, AddressSummary, inner),
    }
}
rp_supabase_client::projection! {
    #[derive(Debug)]
    struct OrdersWithPredicates for crate::public::tables::customers {
        id,
        orders: embed(crate::public::tables::customers::relationships::orders_orders_customer, OrderPredicates),
    }
}

#[test]
fn selections_append_nested_children_and_preserve_aliases() {
    let address = "id,label,country:countries!address_country(id,name)";
    let order = format!(
        "id,label,billing:addresses!orders_billing({address}),shipping:addresses!orders_shipping({address}),detail:order_details!details_order(order_id,note)"
    );
    assert_eq!(OrderSummary::selection(), order);
    assert_eq!(
        Aliases::selection(),
        format!(
            "id,\"select\":addresses!orders_billing({address}),type:addresses!orders_shipping!inner({address})"
        )
    );
    assert_eq!(
        CustomerPredicates::selection(),
        "id,matching_orders:orders!orders_customer()"
    );
    for (len, selection) in [
        (OrderSummary::SELECT_LEN, OrderSummary::selection()),
        (CustomerSummary::SELECT_LEN, CustomerSummary::selection()),
        (Aliases::SELECT_LEN, Aliases::selection()),
        (
            CustomerPredicates::SELECT_LEN,
            CustomerPredicates::selection(),
        ),
    ] {
        assert_eq!(len, selection.len());
    }
    let mut output = String::from("prefix:");
    CustomerSummary::write_selection(&mut output);
    assert_eq!(output, format!("prefix:{}", CustomerSummary::selection()));
}

#[test]
fn conservative_cardinality_and_missing_child_are_distinct() {
    let customer: CustomerSummary = serde_json::from_value(json!({
        "id": 1, "name": "customer", "orders": [], "preference": null,
    }))
    .unwrap();
    let _: Vec<OrderSummary> = customer.orders;
    let _: Option<PreferenceSummary> = customer.preference;
    let order: OrderSummary = serde_json::from_value(json!({
        "id": 2, "label": "order", "billing": null, "shipping": null, "detail": null,
    }))
    .unwrap();
    let _: Option<AddressSummary> = order.billing;
    let _: Option<DetailSummary> = order.detail;
    let inner: OrderInner = serde_json::from_value(json!({"id": 2, "billing": null})).unwrap();
    let _: Option<AddressSummary> = inner.billing;
    for invalid in [
        r#"{"id":2,"label":"order","shipping":null,"detail":null}"#,
        r#"{"id":2,"label":"order","billing":null,"shipping":null}"#,
        r#"{"id":2,"label":"order","billing":[],"shipping":null,"detail":null}"#,
        r#"{"id":2,"label":"order","billing":null,"billing":null,"shipping":null,"detail":null}"#,
        r#"{"id":2,"label":"order","billing":{"id":3,"label":"address"},"shipping":null,"detail":null}"#,
    ] {
        assert!(
            serde_json::from_str::<OrderSummary>(invalid).is_err(),
            "accepted {invalid}"
        );
    }
    for invalid in [
        json!({"id":1,"name":"customer","orders":null,"preference":null}),
        json!({"id":1,"name":"customer","orders":{},"preference":null}),
        json!({"id":1,"name":"customer","orders":[],"preference":[]}),
        json!({"id":1,"name":"customer","orders":[]}),
    ] {
        assert!(serde_json::from_value::<CustomerSummary>(invalid).is_err());
    }
    let direct: OrderCustomer = serde_json::from_value(json!({"id": 2, "customer": null})).unwrap();
    let _: Option<CustomerIdentity> = direct.customer;
    assert!(serde_json::from_value::<OrderCustomer>(json!({"id": 2})).is_err());
}

#[test]
fn empty_embeds_have_no_required_response_field_and_raw_aliases_decode_exactly() {
    for value in [
        json!({"id": 1}),
        json!({"id": 1, "matching_orders": []}),
        json!({"id": 1, "matching_orders": null}),
    ] {
        assert_eq!(
            serde_json::from_value::<CustomerPredicates>(value)
                .unwrap()
                .id,
            1
        );
    }
    let aliases: Aliases =
        serde_json::from_value(json!({"id":1,"select":null,"type":null})).unwrap();
    assert!(aliases.r#select.is_none() && aliases.r#type.is_none());
    assert!(
        serde_json::from_value::<Aliases>(json!({"id":1,"r#select":null,"type":null})).is_err()
    );
    assert!(
        serde_json::from_str::<Aliases>(r#"{"id":1,"select":null,"select":null,"type":null}"#)
            .is_err()
    );
}

#[tokio::test]
async fn nested_scoped_filters_preserve_literal_values_and_alias_paths() {
    let mut server = mockito::Server::new_async().await;
    let literal = "a,b.c:(d)*\"e\\f café";
    let request = server
        .mock("GET", "/customers")
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded(
                "select".into(),
                CustomerSummary::selection().into_owned(),
            ),
            mockito::Matcher::UrlEncoded("id".into(), "eq.1".into()),
            mockito::Matcher::UrlEncoded(
                "orders.billing.country.name".into(),
                format!("eq.{literal}"),
            ),
            mockito::Matcher::UrlEncoded("orders.shipping.label".into(), format!("eq.{literal}")),
            mockito::Matcher::UrlEncoded("orders.billing".into(), "not.is.null".into()),
            mockito::Matcher::UrlEncoded("orders.detail".into(), "is.null".into()),
        ]))
        .with_body(r#"[{"id":1,"name":"customer","orders":[],"preference":null}]"#)
        .create_async()
        .await;
    let rows = tables::customers::query(Postgrest::new(server.url()).unwrap())
        .select(rp_supabase_client::schema::named::<_, CustomerSummary>())
        .eq(tables::customers::columns::id, &1)
        .embedded(
            CustomerSummary::orders
                .then(OrderSummary::billing)
                .then(AddressSummary::country),
            |country| {
                country.eq(tables::countries::columns::name, literal);
            },
        )
        .embedded(CustomerSummary::orders, |orders| {
            orders.embedded(OrderSummary::shipping, |address| {
                address.eq(tables::addresses::columns::label, literal);
            });
            orders
                .exists(OrderSummary::billing)
                .not_exists(OrderSummary::detail);
        })
        .fetch()
        .await
        .unwrap();
    assert_eq!(rows[0].id, 1);
    request.assert_async().await;
}

#[tokio::test]
async fn reserved_alias_filters_are_encoded_once_and_independent() {
    let mut server = mockito::Server::new_async().await;
    let request = server
        .mock("GET", "/orders")
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("select".into(), Aliases::selection().into_owned()),
            mockito::Matcher::UrlEncoded("\"select\".id".into(), "eq.10".into()),
            mockito::Matcher::UrlEncoded("type.id".into(), "eq.20".into()),
            mockito::Matcher::UrlEncoded("\"select\"".into(), "not.is.null".into()),
        ]))
        .with_body(r#"[{"id":1,"select":null,"type":null}]"#)
        .create_async()
        .await;
    let rows = tables::orders::query(Postgrest::new(server.url()).unwrap())
        .select(rp_supabase_client::schema::named::<_, Aliases>())
        .embedded(Aliases::r#select, |address| {
            address.eq(tables::addresses::columns::id, &10);
        })
        .embedded(Aliases::r#type, |address| {
            address.eq(tables::addresses::columns::id, &20);
        })
        .exists(Aliases::r#select)
        .fetch()
        .await
        .unwrap();
    assert_eq!(rows[0].id, 1);
    request.assert_async().await;
}

#[tokio::test]
async fn empty_exists_and_anti_exists_work_at_root_and_in_children() {
    let mut server = mockito::Server::new_async().await;
    for (predicate, expected) in [(true, "not.is.null"), (false, "is.null")] {
        let request = server
            .mock("GET", "/customers")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded(
                    "select".into(),
                    CustomerPredicates::selection().into_owned(),
                ),
                mockito::Matcher::UrlEncoded("matching_orders.id".into(), "eq.7".into()),
                mockito::Matcher::UrlEncoded("matching_orders".into(), expected.into()),
            ]))
            .with_body(r#"[{"id":1}]"#)
            .create_async()
            .await;
        let query = tables::customers::query(Postgrest::new(server.url()).unwrap())
            .select(rp_supabase_client::schema::named::<_, CustomerPredicates>())
            .embedded(CustomerPredicates::matching_orders, |orders| {
                orders.eq(tables::orders::columns::id, &7);
            });
        let rows = if predicate {
            query.exists(CustomerPredicates::matching_orders)
        } else {
            query.not_exists(CustomerPredicates::matching_orders)
        }
        .fetch()
        .await
        .unwrap();
        assert_eq!(rows[0].id, 1);
        request.assert_async().await;
    }
    let request = server
        .mock("GET", "/customers")
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded(
                "select".into(),
                OrdersWithPredicates::selection().into_owned(),
            ),
            mockito::Matcher::UrlEncoded("orders.matching_details".into(), "is.null".into()),
        ]))
        .with_body(r#"[{"id":1,"orders":[{"id":7}]}]"#)
        .create_async()
        .await;
    let rows = tables::customers::query(Postgrest::new(server.url()).unwrap())
        .select(rp_supabase_client::schema::named::<_, OrdersWithPredicates>())
        .embedded(OrdersWithPredicates::orders, |orders| {
            orders.not_exists(OrderPredicates::matching_details);
        })
        .fetch()
        .await
        .unwrap();
    assert_eq!(rows[0].orders[0].id, 7);
    request.assert_async().await;
}

#[tokio::test]
async fn locked_selection_retains_embedded_filters_when_choosing_a_write() {
    let mut server = mockito::Server::new_async().await;
    let request = server
        .mock("DELETE", "/orders")
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("select".into(), OrderSummary::selection().into_owned()),
            mockito::Matcher::UrlEncoded("id".into(), "eq.7".into()),
            mockito::Matcher::UrlEncoded("billing.id".into(), "eq.8".into()),
        ]))
        .with_body(r#"[{"id":7,"label":"deleted","billing":null,"shipping":null,"detail":null}]"#)
        .create_async()
        .await;
    let rows = tables::orders::query(Postgrest::new(server.url()).unwrap())
        .select(rp_supabase_client::schema::named::<_, OrderSummary>())
        .embedded(OrderSummary::billing, |address| {
            address.eq(tables::addresses::columns::id, &8);
        })
        .delete()
        .eq(tables::orders::columns::id, &7)
        .fetch()
        .await
        .unwrap();
    assert_eq!(rows[0].id, 7);
    request.assert_async().await;
}

#[test]
fn composite_forward_and_reverse_results_decode_with_declared_cardinality() {
    let child: CompositeChildSummary = serde_json::from_value(json!({"id":1,
        "parent":{"tenant_id":2,"id":3,"label":"parent"}}))
    .unwrap();
    assert_eq!(child.parent.unwrap().tenant_id, 2);
    let parent: CompositeParentChildren =
        serde_json::from_value(json!({"id":3,"children":[{"id":1}]})).unwrap();
    assert_eq!(parent.children[0].id, 1);
}

#[tokio::test]
async fn fetch_rejects_missing_optional_and_duplicate_children() {
    let mut server = mockito::Server::new_async().await;
    for body in [
        r#"[{"id":7,"label":"order","shipping":null,"detail":null}]"#,
        r#"[{"id":7,"label":"order","billing":null,"billing":null,"shipping":null,"detail":null}]"#,
        r#"[{"id":7,"label":"order","billing":[],"shipping":null,"detail":null}]"#,
    ] {
        let request = server
            .mock("GET", "/orders")
            .match_query(mockito::Matcher::UrlEncoded(
                "select".into(),
                OrderSummary::selection().into_owned(),
            ))
            .with_body(body)
            .create_async()
            .await;
        let error = tables::orders::query(Postgrest::new(server.url()).unwrap())
            .select(rp_supabase_client::schema::named::<_, OrderSummary>())
            .fetch()
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            rp_supabase_client::rp_postgrest::Error::ResponseDecode { .. }
        ));
        request.assert_async().await;
    }
}

#[tokio::test]
async fn insert_and_update_preserve_selection_lock_and_child_predicates() {
    use rp_supabase_client::schema::Field;
    let mut server = mockito::Server::new_async().await;
    let body = r#"[{"id":7,"label":"changed","billing":null,"shipping":null,"detail":null}]"#;
    let patch = server
        .mock("PATCH", "/orders")
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("select".into(), OrderSummary::selection().into_owned()),
            mockito::Matcher::UrlEncoded("billing.id".into(), "eq.8".into()),
            mockito::Matcher::UrlEncoded("shipping".into(), "is.null".into()),
        ]))
        .match_body(mockito::Matcher::Json(json!({"label":"changed"})))
        .with_body(body)
        .create_async()
        .await;
    let rows = tables::orders::query(Postgrest::new(server.url()).unwrap())
        .select(rp_supabase_client::schema::named::<_, OrderSummary>())
        .embedded(OrderSummary::billing, |address| {
            address.eq(tables::addresses::columns::id, &8);
        })
        .update(&tables::orders::Update {
            label: Field::Value("changed".into()),
            ..Default::default()
        })
        .not_exists(OrderSummary::shipping)
        .fetch()
        .await
        .unwrap();
    assert_eq!(rows[0].id, 7);
    patch.assert_async().await;
    let post = server
        .mock("POST", "/orders")
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("select".into(), OrderSummary::selection().into_owned()),
            mockito::Matcher::UrlEncoded("billing".into(), "not.is.null".into()),
        ]))
        .match_body(mockito::Matcher::Json(json!({
            "id":7,"customer_id":1,"billing_id":8,"shipping_id":null,"label":"changed",
        })))
        .with_status(201)
        .with_body(body)
        .create_async()
        .await;
    let rows = tables::orders::query(Postgrest::new(server.url()).unwrap())
        .select(rp_supabase_client::schema::named::<_, OrderSummary>())
        .exists(OrderSummary::billing)
        .insert(&tables::orders::Insert {
            id: 7,
            customer_id: 1,
            billing_id: Field::Value(Some(8)),
            shipping_id: Field::Value(None),
            label: "changed".into(),
        })
        .fetch()
        .await
        .unwrap();
    assert_eq!(rows[0].id, 7);
    post.assert_async().await;
}
