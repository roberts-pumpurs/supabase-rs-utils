use crate::public::tables::{adapters, skills};
use rp_supabase_client::{
    rp_postgrest::{Count, Postgrest},
    schema::{Order, params},
};

rp_supabase_client::projection! {
    #[derive(Debug, PartialEq)]
    struct Artifact for [crate::public::tables::skills, crate::public::tables::adapters] { id, name, owner_id }
}

pub async fn live(client: Postgrest) -> Result<(), Box<dyn std::error::Error>> {
    let id = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_micros(),
    )?;
    let name = format!("gap,{id}\"\\");
    let scenario = async {
        skills::query(client.clone())
            .insert(&skills::Insert {
                id,
                name: name.clone(),
                owner_id: id,
                manifest: serde_json::json!({"fingerprint":name}),
            })
            .execute()
            .await?;
        adapters::query(client.clone())
            .insert(&adapters::Insert {
                id,
                name: name.clone(),
                owner_id: id,
                manifest: serde_json::json!({"fingerprint":name}),
            })
            .execute()
            .await?;
        let skill = skills::query(client.clone())
            .select::<Artifact>()
            .in_(skills::columns::name, [name.as_str()])
            .json_text_eq(skills::columns::manifest, &["fingerprint"], &name)?
            .order(skills::columns::id, Order::Desc)
            .range(0, 0)
            .fetch_with_count(Count::Exact)
            .await?;
        assert_eq!(skill.count, 1);
        assert_eq!(skill.data.len(), 1);
        let adapter: Vec<Artifact> = adapters::query(client.clone())
            .select::<Artifact>()
            .eq(adapters::columns::id, &id)
            .into_raw()
            .fetch()
            .await?;
        assert_eq!(skill.data, adapter);
        assert_eq!(
            adapters::query(client.clone())
                .eq(adapters::columns::owner_id, &id)
                .limit(1)
                .count(Count::Exact)
                .await?,
            1
        );
        // Pure pairs can be fed to any reqwest client. Use the owned builder here
        // to retain the live runner's API credentials while proving literal grammar.
        let mut raw = client.from("skills");
        for (key, value) in [
            params::projection::<skills::Row, Artifact>(),
            params::eq(skills::columns::id, &id),
        ] {
            raw.append_query(key, value);
        }
        assert_eq!(raw.fetch::<Vec<Artifact>>().await?, adapter);
        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;
    // Attempt both cleanup operations even if insertion or a read fails.
    let skill_cleanup = skills::query(client.clone())
        .eq(skills::columns::id, &id)
        .delete()
        .execute_with_count(Count::Exact)
        .await;
    let adapter_cleanup = adapters::query(client)
        .eq(adapters::columns::id, &id)
        .delete()
        .execute()
        .await;
    scenario?;
    assert_eq!(skill_cleanup?, 1);
    adapter_cleanup?;
    println!("Live shared DTO/order/IN/JSON/pagination/count/minimal/raw scenario passed");
    Ok(())
}
