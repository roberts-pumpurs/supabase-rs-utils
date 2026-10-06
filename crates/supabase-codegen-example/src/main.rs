//! Offline by default. Set SUPABASE_CODEGEN_API_URL to a PostgREST base URL
//! (including /rest/v1 for Supabase) to run finite live CRUD/RPC and relationship scenarios.
//! Apply smoke.sql to a disposable database first. For live code generation set
//! SUPABASE_CODEGEN_DATABASE_URL in the build environment. Optional API credentials
//! come only from SUPABASE_CODEGEN_API_KEY and SUPABASE_CODEGEN_ACCESS_TOKEN.

use rp_supabase_client::postgrest::{Builder, Postgrest};
use rp_supabase_client::schema::{Array, Field, rpc};
use serde::de::DeserializeOwned;
use serde_json::json;

rp_supabase_client::include_schema!("database.rs");

#[allow(dead_code)]
mod relationship_projections;
mod relationships;

use public::functions::echo_message;
use public::tables::messages::{self, Insert, Update};

rp_supabase_client::projection! {
    #[derive(Debug)]
    struct Message for public::tables::messages { id, body, note, amount }
}

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
    let row: Message = serde_json::from_value(json!({
        "id": 1, "body": "offline example", "note": null,
        "amount": serde_json::from_str::<serde_json::Number>("123456789012345678901234567890.123456789")?
    }))?;
    assert_eq!(row.note, None);
    assert_eq!(row.body, "offline example");
    assert_eq!(
        row.amount.to_string(),
        "123456789012345678901234567890.123456789"
    );
    println!(
        "Offline generated bindings: {}",
        serde_json::to_string(&request)?
    );
    relationship_projections::offline()?;
    Ok(())
}

async fn decode<T: DeserializeOwned>(request: Builder) -> Result<T, Box<dyn std::error::Error>> {
    let response = request.execute().await?;
    Ok(rp_supabase_client::PostgerstResponse::<T>::new(response)
        .json()
        .await??)
}

async fn live(client: Postgrest) -> Result<(), Box<dyn std::error::Error>> {
    relationships::live(client.clone()).await?;
    let rows = messages::query(client.clone())
        .select::<Message>()
        .insert(&insert("live example"))
        .fetch()
        .await?;
    let row = rows.first().ok_or("insert returned no row")?;
    let id = row.id;
    // Attempt cleanup after insertion, including recoverable scenario errors.
    let scenario = async {
        let rows = messages::query(client.clone())
            .select::<Message>()
            .eq(messages::columns::id, &id)
            .fetch()
            .await?;
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
        let rows = messages::query(client.clone())
            .select::<Message>()
            .eq(messages::columns::id, &id)
            .update(&update)
            .fetch()
            .await?;
        assert_eq!(rows[0].note.as_deref(), Some("updated"));
        let views = public::tables::message_summaries::query(client.clone())
            .eq(public::tables::message_summaries::columns::id, &id)
            .fetch()
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
    let cleanup = messages::query(client)
        .eq(messages::columns::id, &id)
        .delete()
        .fetch()
        .await;
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
