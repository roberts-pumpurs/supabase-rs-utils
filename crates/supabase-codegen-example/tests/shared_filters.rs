#[allow(dead_code)]
mod database {
    rp_supabase_client::include_schema!("database.rs");
}
use database::public::tables::{adapters, skills};
use rp_supabase_client::rp_postgrest::Postgrest;
use rp_supabase_client::schema::{
    Column, FilterColumn, Projection, Query, Relation, SharedFilter, named, params,
};
use rp_supabase_client::{key, projection};

projection! {
    #[derive(Debug, PartialEq)]
    struct Artifact for [database::public::tables::skills, database::public::tables::adapters] {
        name,
    } filters {
        artifact_id: [id, owner_id],
        owner: [owner_id, id],
    }
}

fn pair<R: Relation>(id: i64) -> params::QueryPair
where
    Artifact: FilterColumn<key!(type artifact_id), R>,
    SharedFilter<Artifact, key!(type artifact_id), R>: Column<Relation = R, Filter = i64>,
{
    params::eq(Artifact::artifact_id::<R>(), &id)
}

fn filtered<R: Relation>(query: Query<R, Artifact>, id: i64) -> Query<R, Artifact>
where
    Artifact: Projection<R> + FilterColumn<key!(type artifact_id), R>,
    SharedFilter<Artifact, key!(type artifact_id), R>: Column<Relation = R, Filter = i64>,
{
    query.eq(Artifact::artifact_id::<R>(), &id)
}

#[test]
fn generic_shared_keys_map_columns_without_selecting_them() {
    assert_eq!(pair::<skills::Row>(7), ("id".into(), "eq.7".into()));
    assert_eq!(pair::<adapters::Row>(7), ("owner_id".into(), "eq.7".into()));
    assert_eq!(
        params::eq(Artifact::owner::<skills::Row>(), &8),
        ("owner_id".into(), "eq.8".into())
    );
    assert_eq!(
        params::eq(Artifact::owner::<adapters::Row>(), &8),
        ("id".into(), "eq.8".into())
    );
    assert_eq!(<Artifact as Projection<skills::Row>>::selection(), "name");
    assert_eq!(<Artifact as Projection<adapters::Row>>::selection(), "name");
    let record: Artifact = serde_json::from_str(r#"{"name":"search"}"#).unwrap();
    assert_eq!(record.name, "search");
    assert!(serde_json::from_str::<Artifact>(r#"{"id":7}"#).is_err());
}

#[tokio::test]
async fn generic_query_uses_each_relations_shared_column() {
    let mut server = mockito::Server::new_async().await;
    let skill = server
        .mock("GET", "/skills")
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("select".into(), "name".into()),
            mockito::Matcher::UrlEncoded("id".into(), "eq.7".into()),
        ]))
        .with_body(r#"[{"name":"skill"}]"#)
        .create_async()
        .await;
    let adapter = server
        .mock("GET", "/adapters")
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("select".into(), "name".into()),
            mockito::Matcher::UrlEncoded("owner_id".into(), "eq.7".into()),
        ]))
        .with_body(r#"[{"name":"adapter"}]"#)
        .create_async()
        .await;
    let client = Postgrest::new(server.url()).unwrap();
    let skill_rows = filtered(
        skills::query(client.clone()).select(named::<_, Artifact>()),
        7,
    )
    .fetch()
    .await
    .unwrap();
    let adapter_rows = filtered(adapters::query(client).select(named::<_, Artifact>()), 7)
        .fetch()
        .await
        .unwrap();
    assert_eq!(skill_rows[0].name, "skill");
    assert_eq!(adapter_rows[0].name, "adapter");
    skill.assert_async().await;
    adapter.assert_async().await;
}
