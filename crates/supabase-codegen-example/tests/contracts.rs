#[allow(dead_code)]
mod bindings {
    rp_supabase_client::include_schema!("contract_bindings.rs");
}

use bindings::public::{functions, tables};

#[test]
fn generated_names_do_not_shadow_primitive_or_prelude_types() {
    let row: tables::a_b::Row =
        serde_json::from_str(r#"{"id":7,"body":"text alias","active":true}"#).unwrap();
    assert_eq!(row.body, "text alias");
    assert!(row.active);
    assert_eq!(serde_json::to_value(row).unwrap()["body"], "text alias");
}

#[test]
fn composite_and_output_rpc_results_match_json_cardinality() {
    let composite: functions::payload_rpc::Returns =
        serde_json::from_str(r#"{"label":"value","state":null}"#).unwrap();
    assert_eq!(composite.label.as_deref(), Some("value"));
    let table: functions::table_rpc::Returns =
        serde_json::from_str(r#"[{"value":"first","state":null},{"value":"second","state":null}]"#)
            .unwrap();
    assert_eq!(table[0].value.as_deref(), Some("first"));
    assert_eq!(table[1].value.as_deref(), Some("second"));
    let out: functions::single_out::Returns = serde_json::from_str(r#"{"plus":6}"#).unwrap();
    assert_eq!(out.plus, Some(6));
    let inout: functions::single_inout::Returns = serde_json::from_str(r#"{"num":9}"#).unwrap();
    assert_eq!(inout.num, Some(9));
    let named_record: functions::enum_record::Returns = serde_json::from_str(r#""ok""#).unwrap();
    assert!(matches!(
        named_record,
        Some(bindings::public::enums::Record::Ok)
    ));
}

#[tokio::test]
async fn quoted_relation_and_function_names_reach_the_correct_endpoint() {
    let mut server = mockito::Server::new_async().await;
    let relation = server
        .mock("GET", "/a%23b")
        .match_query(mockito::Matcher::UrlEncoded(
            "select".to_owned(),
            "*".to_owned(),
        ))
        .with_body(r#"[{"id":7,"body":"quoted","active":true}]"#)
        .create_async()
        .await;
    let client = rp_supabase_client::postgrest::Postgrest::new(server.url());
    let response = rp_supabase_client::schema::from::<tables::a_b::Row>(client)
        .select("*")
        .execute()
        .await
        .unwrap();
    let rows = rp_supabase_client::PostgerstResponse::<Vec<tables::a_b::Row>>::new(response)
        .json()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(rows[0].body, "quoted");
    relation.assert_async().await;
    let function = server
        .mock("POST", "/rpc/rpc%3Fecho")
        .with_body(r#""quoted RPC""#)
        .create_async()
        .await;
    let client = rp_supabase_client::postgrest::Postgrest::new(server.url());
    let request = rp_supabase_client::schema::rpc::<functions::rpc_echo::Function>(
        client,
        &functions::rpc_echo::Args {
            message: Some("input".to_owned()),
        },
    )
    .unwrap();
    let response = request.execute().await.unwrap();
    let returned =
        rp_supabase_client::PostgerstResponse::<functions::rpc_echo::Returns>::new(response)
            .json()
            .await
            .unwrap()
            .unwrap();
    assert_eq!(returned.as_deref(), Some("quoted RPC"));
    function.assert_async().await;
}
