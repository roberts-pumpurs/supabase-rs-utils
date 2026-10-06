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
    let client = rp_supabase_client::rp_postgrest::Postgrest::new(server.url()).unwrap();
    let rows = tables::a_b::query(client).fetch().await.unwrap();
    assert_eq!(rows[0].body, "quoted");
    relation.assert_async().await;
    let function = server
        .mock("POST", "/rpc/rpc%3Fecho")
        .with_body(r#""quoted RPC""#)
        .create_async()
        .await;
    let client = rp_supabase_client::rp_postgrest::Postgrest::new(server.url()).unwrap();
    let returned = rp_supabase_client::schema::rpc::<functions::rpc_echo::Function>(
        client,
        &functions::rpc_echo::Args {
            message: Some("input".to_owned()),
        },
    )
    .fetch()
    .await
    .unwrap();
    assert_eq!(returned.as_deref(), Some("quoted RPC"));
    function.assert_async().await;
}

#[tokio::test]
async fn typed_rpc_infers_composite_set_and_void_results_without_relation_semantics() {
    use rp_supabase_client::{
        rp_postgrest::Postgrest,
        schema::{Field, rpc},
    };
    let mut server = mockito::Server::new_async().await;
    let composite = server
        .mock("POST", "/rpc/payload_rpc")
        .match_query(mockito::Matcher::Missing)
        .match_header("content-profile", "public")
        .match_body(mockito::Matcher::Json(serde_json::json!({"label":"input"})))
        .with_body(r#"{"label":"returned","state":null}"#)
        .create_async()
        .await;
    let row = rpc::<functions::payload_rpc::Function>(
        Postgrest::new(server.url()).unwrap(),
        &functions::payload_rpc::Args {
            label: Some("input".into()),
        },
    )
    .fetch()
    .await
    .unwrap();
    assert_eq!(row.label.as_deref(), Some("returned"));
    assert!(row.state.is_none());
    composite.assert_async().await;

    for (label, expected_body) in [
        (Field::Omit, serde_json::json!({})),
        (Field::Value(None), serde_json::json!({"label":null})),
        (
            Field::Value(Some("literal".into())),
            serde_json::json!({"label":"literal"}),
        ),
    ] {
        let set = server
            .mock("POST", "/rpc/table_rpc")
            .match_query(mockito::Matcher::Missing)
            .match_body(mockito::Matcher::Json(expected_body))
            .with_body(r#"[{"value":"first","state":null},{"value":"second","state":null}]"#)
            .create_async()
            .await;
        let rows = rpc::<functions::table_rpc::Function>(
            Postgrest::new(server.url()).unwrap(),
            &functions::table_rpc::Args { label },
        )
        .fetch()
        .await
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].value.as_deref(), Some("first"));
        assert_eq!(rows[1].value.as_deref(), Some("second"));
        set.assert_async().await;
    }

    let void = server
        .mock("POST", "/rpc/void_action")
        .match_query(mockito::Matcher::Missing)
        .match_body(mockito::Matcher::Json(serde_json::json!({})))
        .with_status(204)
        .create_async()
        .await;
    let () = rpc::<functions::void_action::Function>(
        Postgrest::new(server.url()).unwrap(),
        &functions::void_action::Args {},
    )
    .fetch()
    .await
    .unwrap();
    void.assert_async().await;
}

#[tokio::test]
async fn typed_rpc_reports_flat_errors_and_rejects_wrong_return_cardinality() {
    use rp_supabase_client::{
        rp_postgrest::{Error, Postgrest},
        schema::rpc,
    };
    let mut server = mockito::Server::new_async().await;
    for (status, body) in [
        (
            403,
            r#"{"code":"42501","message":"permission denied","details":null,"hint":null}"#,
        ),
        (502, "not an error envelope"),
        (200, r#"["wrong scalar cardinality"]"#),
        (200, "not JSON"),
    ] {
        let request = server
            .mock("POST", "/rpc/rpc%3Fecho")
            .with_status(status)
            .with_body(body)
            .create_async()
            .await;
        let error = rpc::<functions::rpc_echo::Function>(
            Postgrest::new(server.url()).unwrap(),
            &functions::rpc_echo::Args {
                message: Some("input".into()),
            },
        )
        .fetch()
        .await
        .unwrap_err();
        assert_eq!(error.status().unwrap().as_u16(), status as u16);
        match (status, error) {
            (403, Error::Postgrest { .. }) | (502, Error::Decode { .. }) => {}
            (200, Error::ResponseDecode { source, .. }) => {
                assert_eq!(
                    source.classify(),
                    if body == "not JSON" {
                        serde_json::error::Category::Syntax
                    } else {
                        serde_json::error::Category::Data
                    }
                );
            }
            (_, error) => panic!("unexpected RPC error: {error:?}"),
        }
        request.assert_async().await;
    }
    let raw = rpc::<functions::rpc_echo::Function>(
        Postgrest::new(server.url()).unwrap(),
        &functions::rpc_echo::Args { message: None },
    )
    .into_raw();
    let request = raw.build().unwrap().build().unwrap();
    assert_eq!(request.url().path(), "/rpc/rpc%3Fecho");
    assert!(request.url().query().is_none());
}

#[tokio::test]
async fn typed_rpc_rejects_wrong_composite_set_and_void_shapes() {
    use rp_supabase_client::{
        rp_postgrest::{Error, Postgrest},
        schema::{Field, rpc},
    };
    let mut server = mockito::Server::new_async().await;
    let composite = server
        .mock("POST", "/rpc/payload_rpc")
        .with_body(r#"[{"label":"not a single composite","state":null}]"#)
        .create_async()
        .await;
    let error = rpc::<functions::payload_rpc::Function>(
        Postgrest::new(server.url()).unwrap(),
        &functions::payload_rpc::Args { label: None },
    )
    .fetch()
    .await
    .err()
    .unwrap();
    assert!(matches!(error, Error::ResponseDecode { .. }));
    composite.assert_async().await;

    let set = server
        .mock("POST", "/rpc/table_rpc")
        .with_body(r#"{"value":"not a set","state":null}"#)
        .create_async()
        .await;
    let error = rpc::<functions::table_rpc::Function>(
        Postgrest::new(server.url()).unwrap(),
        &functions::table_rpc::Args { label: Field::Omit },
    )
    .fetch()
    .await
    .err()
    .unwrap();
    assert!(matches!(error, Error::ResponseDecode { .. }));
    set.assert_async().await;

    for body in ["", r#"{"not":"void"}"#] {
        let void = server
            .mock("POST", "/rpc/void_action")
            .with_body(body)
            .create_async()
            .await;
        let error = rpc::<functions::void_action::Function>(
            Postgrest::new(server.url()).unwrap(),
            &functions::void_action::Args {},
        )
        .fetch()
        .await
        .unwrap_err();
        assert!(matches!(error, Error::ResponseDecode { .. }));
        void.assert_async().await;
    }
}
