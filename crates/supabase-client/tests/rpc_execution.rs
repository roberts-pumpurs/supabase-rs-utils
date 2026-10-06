#![expect(
    clippy::tests_outside_test_module,
    reason = "Integration tests exercise generated runtime contracts."
)]
#![expect(clippy::unwrap_used, reason = "Test setup retains failure details.")]

use rp_supabase_client::{
    Error, Postgrest,
    schema::{Field, Function, rpc},
};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Row {
    id: i32,
}

struct Rows;
impl Function for Rows {
    type Args = Field<String>;
    type Returns = Vec<Row>;
    const SCHEMA: &'static str = "private";
    const NAME: &'static str = "rows";
}

struct JsonObject;
impl Function for JsonObject {
    type Args = ();
    type Returns = serde_json::Value;
    const SCHEMA: &'static str = "private";
    const NAME: &'static str = "json_object";
}

#[tokio::test]
async fn single_requests_server_cardinality_and_decodes_selected_row() {
    let mut server = mockito::Server::new_async().await;
    let response = server
        .mock("POST", "/rpc/rows")
        .match_header("accept", "application/vnd.pgrst.object+json")
        .match_header("content-profile", "private")
        .match_body(r#""argument""#)
        .with_body(r#"{"id":7}"#)
        .create_async()
        .await;
    let row = rpc::<Rows>(
        Postgrest::new(server.url()).unwrap(),
        &Field::Value("argument".into()),
    )
    .single()
    .fetch_as::<Row>()
    .await
    .unwrap();
    assert_eq!(row, Row { id: 7 });
    response.assert_async().await;
}

#[tokio::test]
async fn single_preserves_generated_returns_and_cardinality_error() {
    let mut server = mockito::Server::new_async().await;
    let response = server
        .mock("POST", "/rpc/rows")
        .match_header("accept", "application/vnd.pgrst.object+json")
        .match_header("content-profile", "private")
        .with_status(406)
        .with_body(r#"{"code":"PGRST116","message":"Cannot coerce the result to a single JSON object","details":"The result contains 0 rows","hint":null}"#)
        .create_async()
        .await;
    // fetch still uses the generated Vec<Row>, even after single().
    let result: Result<Vec<Row>, Error> = rpc::<Rows>(
        Postgrest::new(server.url()).unwrap(),
        &Field::Value("argument".into()),
    )
    .single()
    .fetch()
    .await;
    let error = result.unwrap_err();
    assert!(matches!(error, Error::Postgrest { .. }));
    assert_eq!(error.postgrest_body().unwrap().code.as_str(), "PGRST116");
    response.assert_async().await;
}

#[tokio::test]
async fn jsonb_can_decode_as_an_explicit_type_and_reports_shape_mismatch() {
    let mut server = mockito::Server::new_async().await;
    let response = server
        .mock("POST", "/rpc/json_object")
        .match_header("content-profile", "private")
        .match_body("null")
        .with_body(r#"{"id":7}"#)
        .expect(3)
        .create_async()
        .await;
    let client = Postgrest::new(server.url()).unwrap();
    let generated = rpc::<JsonObject>(client.clone(), &())
        .fetch()
        .await
        .unwrap();
    assert_eq!(generated, serde_json::json!({"id": 7_i32}));
    let row = rpc::<JsonObject>(client.clone(), &())
        .fetch_as::<Row>()
        .await
        .unwrap();
    assert_eq!(row, Row { id: 7 });
    let error = rpc::<JsonObject>(client, &())
        .fetch_as::<Vec<Row>>()
        .await
        .unwrap_err();
    assert!(matches!(error, Error::ResponseDecode { .. }));
    response.assert_async().await;
}

#[tokio::test]
async fn execute_accepts_empty_and_non_json_success_bodies() {
    let mut server = mockito::Server::new_async().await;
    let empty = server
        .mock("POST", "/rpc/rows")
        .match_header("content-profile", "private")
        .with_status(204)
        .create_async()
        .await;
    let non_json = server
        .mock("POST", "/rpc/json_object")
        .match_header("content-profile", "private")
        .with_status(200)
        .with_body("not JSON")
        .create_async()
        .await;
    let client = Postgrest::new(server.url()).unwrap();
    rpc::<Rows>(client.clone(), &Field::Value("argument".into()))
        .execute()
        .await
        .unwrap();
    rpc::<JsonObject>(client, &()).execute().await.unwrap();
    empty.assert_async().await;
    non_json.assert_async().await;
}

#[tokio::test]
async fn execute_preserves_structured_failure() {
    let mut server = mockito::Server::new_async().await;
    let response = server
        .mock("POST", "/rpc/json_object")
        .match_header("content-profile", "private")
        .with_status(403)
        .with_body(r#"{"code":"42501","message":"permission denied","details":null,"hint":null}"#)
        .create_async()
        .await;
    let error = rpc::<JsonObject>(Postgrest::new(server.url()).unwrap(), &())
        .execute()
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Postgrest { .. }));
    assert_eq!(error.postgrest_body().unwrap().code.as_str(), "42501");
    response.assert_async().await;
}

#[tokio::test]
async fn rpc_modes_preserve_deferred_serialization_errors() {
    let mut server = mockito::Server::new_async().await;
    let response = server
        .mock("POST", "/rpc/rows")
        .expect(0)
        .create_async()
        .await;
    let client = Postgrest::new(server.url()).unwrap();
    assert!(matches!(
        rpc::<Rows>(client.clone(), &Field::Omit)
            .single()
            .fetch()
            .await,
        Err(Error::Serialization(_))
    ));
    assert!(matches!(
        rpc::<Rows>(client.clone(), &Field::Omit)
            .fetch_as::<Row>()
            .await,
        Err(Error::Serialization(_))
    ));
    assert!(matches!(
        rpc::<Rows>(client, &Field::Omit).execute().await,
        Err(Error::Serialization(_))
    ));
    response.assert_async().await;
}
