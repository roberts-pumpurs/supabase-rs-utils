//! Offline by default. Set SUPABASE_CODEGEN_API_URL to a PostgREST base URL
//! (including /rest/v1 for Supabase) to run a finite live CRUD/RPC smoke scenario.
//! Apply smoke.sql to a disposable database first. For live code generation set
//! SUPABASE_CODEGEN_DATABASE_URL in the build environment. Optional API credentials
//! come only from SUPABASE_CODEGEN_API_KEY and SUPABASE_CODEGEN_ACCESS_TOKEN.

use rp_supabase_client::postgrest::{Builder, Postgrest};
use rp_supabase_client::schema::{Array, Field, from, rpc};
use serde::de::DeserializeOwned;
use serde_json::json;

rp_supabase_client::include_schema!("database.rs");

use public::functions::echo_message;
use public::tables::messages::{Insert, Row, Update};

fn insert(body: &str) -> Insert {
    // The custom TypedBuilder derive is applied only to this generated struct.
    Insert::builder()
        .body(body.to_owned())
        .note(Field::Value(None))
        .created_at(Field::Omit)
        .mood(Field::Omit)
        .amount(Field::Value(
            serde_json::from_str("123456789012345678901234567890.123456789")
                .expect("valid numeric fixture"),
        ))
        .metadata(Field::Value(json!({"source": "codegen-example"})))
        .tags(Field::Value(Array::Elements(vec![
            Some("generated".into()),
            None,
        ])))
        .build()
}

fn offline() -> Result<(), Box<dyn std::error::Error>> {
    let request = serde_json::to_value(insert("offline example"))?;
    assert_eq!(request["note"], json!(null));
    assert!(request.get("created_at").is_none());
    assert!(request.get("id").is_none());
    let row: Row = serde_json::from_value(json!({
        "id": 1, "body": "offline example", "note": null,
        "created_at": "2026-01-01T00:00:00Z", "mood": "needs-review",
        "metadata": {"source": "snapshot"}, "tags": [["a", null], ["b", "c"]],
        "amount": serde_json::from_str::<serde_json::Number>("123456789012345678901234567890.123456789")?
    }))?;
    assert_eq!(
        serde_json::to_value(&row)?["tags"],
        json!([["a", null], ["b", "c"]])
    );
    assert_eq!(
        row.amount.to_string(),
        "123456789012345678901234567890.123456789"
    );
    println!(
        "Offline generated bindings: {}",
        serde_json::to_string(&request)?
    );
    Ok(())
}

async fn decode<T: DeserializeOwned>(request: Builder) -> Result<T, Box<dyn std::error::Error>> {
    let response = request.execute().await?;
    Ok(rp_supabase_client::PostgerstResponse::<T>::new(response)
        .json()
        .await??)
}

async fn live(client: Postgrest) -> Result<(), Box<dyn std::error::Error>> {
    let rows: Vec<Row> =
        decode(from::<Row>(client.clone()).insert(serde_json::to_string(&insert("live example"))?))
            .await?;
    let row = rows.first().ok_or("insert returned no row")?;
    let id = row.id.to_string();
    // Attempt cleanup after insertion, including recoverable scenario errors.
    let scenario = async {
        let rows: Vec<Row> = decode(from::<Row>(client.clone()).select("*").eq("id", &id)).await?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].note, None);
        let update = Update {
            note: Field::Value(Some("updated".into())),
            ..Update::default()
        };
        assert_eq!(
            rows[0].amount.to_string(),
            "123456789012345678901234567890.123456789"
        );
        let rows: Vec<Row> = decode(
            from::<Row>(client.clone())
                .eq("id", &id)
                .update(serde_json::to_string(&update)?),
        )
        .await?;
        assert_eq!(rows[0].note.as_deref(), Some("updated"));
        let views: Vec<public::tables::message_summaries::Row> = decode(
            from::<public::tables::message_summaries::Row>(client.clone())
                .select("*")
                .eq("id", &id),
        )
        .await?;
        assert_eq!(views.len(), 1);
        let args = echo_message::Args {
            message: Some("rpc example".into()),
        };
        let echoed: <echo_message::Function as rp_supabase_client::schema::Function>::Returns =
            decode(rpc::<echo_message::Function>(client.clone(), &args)?).await?;
        assert_eq!(echoed.as_deref(), Some("rpc example"));
        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;
    let cleanup: Result<Vec<Row>, _> = decode(from::<Row>(client).eq("id", &id).delete()).await;
    scenario?;
    assert_eq!(cleanup?.len(), 1);
    println!("Live insert/select/update/view/RPC/delete scenario passed");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    offline()?;
    if let Ok(url) = std::env::var("SUPABASE_CODEGEN_API_URL") {
        let mut client = Postgrest::new(url.trim_end_matches('/'));
        if let Ok(key) = std::env::var("SUPABASE_CODEGEN_API_KEY") {
            client = client.insert_header("apikey", key);
        }
        if let Ok(token) = std::env::var("SUPABASE_CODEGEN_ACCESS_TOKEN") {
            client = client.auth(token);
        }
        tokio::time::timeout(std::time::Duration::from_secs(30), live(client)).await??;
    }
    Ok(())
}
