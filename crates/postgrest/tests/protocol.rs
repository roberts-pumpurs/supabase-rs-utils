#![expect(
    clippy::tests_outside_test_module,
    reason = "Integration tests exercise public interfaces."
)]

use rp_postgrest::{
    ConfigError, Error, Postgrest,
    reqwest::{Method, Request},
};

#[expect(
    clippy::unwrap_used,
    reason = "Valid protocol fixtures must build requests"
)]
fn request(builder: rp_postgrest::Builder) -> Request {
    builder.build().unwrap().build().unwrap()
}
#[expect(clippy::unwrap_used, reason = "The fixed test URL is valid")]
fn client() -> Postgrest {
    Postgrest::new("http://example.test/rest/v1/").unwrap()
}

#[expect(
    clippy::non_ascii_literal,
    reason = "Literal Unicode verifies UTF-8 resource encoding"
)]
#[test]
fn literal_resource_names_encode_once_in_table_and_rpc_paths() {
    for (name, encoded) in [
        ("a#b", "a%23b"),
        ("rpc?echo", "rpc%3Fecho"),
        ("typed.probe", "typed%2Eprobe"),
        ("a/b", "a%2Fb"),
        ("%2E", "%252E"),
        ("é \\", "%C3%A9%20%5C"),
    ] {
        assert_eq!(
            request(client().from(name)).url().path(),
            format!("/rest/v1/{encoded}")
        );
        assert_eq!(
            request(client().rpc(name, "{}")).url().path(),
            format!("/rest/v1/rpc/{encoded}")
        );
    }
    for name in [".", ".."] {
        assert!(matches!(
            client().from(name).build(),
            Err(Error::Configuration(ConfigError::DotOnlyResource))
        ));
        assert!(matches!(
            client().rpc(name, "{}").build(),
            Err(Error::Configuration(ConfigError::DotOnlyResource))
        ));
    }
}

#[expect(
    clippy::unwrap_used,
    reason = "Assertions describe preference and RPC header contracts"
)]
#[test]
fn preferences_compose_in_both_helper_orders_and_last_same_key_wins() {
    for builder in [
        client().from("items").exact_count().upsert("{}"),
        client().from("items").upsert("{}").exact_count(),
    ] {
        let request = request(
            builder.insert_header("prefer", "return=minimal,count=planned,handling=strict"),
        );
        let value = request.headers()["prefer"].to_str().unwrap();
        let mut directives: Vec<_> = value.split(',').collect();
        directives.sort_unstable();
        assert_eq!(
            directives,
            [
                "count=planned",
                "handling=strict",
                "resolution=merge-duplicates",
                "return=minimal"
            ]
        );
        assert!(!request.headers().contains_key("range"));
    }
    let inherited = client()
        .insert_header("prefer", "tx=rollback,count=exact")
        .unwrap();
    let mutation = request(
        inherited
            .from("items")
            .insert_header("prefer", "count=estimated")
            .insert("{}"),
    );
    assert_eq!(
        mutation.headers()["prefer"],
        "tx=rollback,count=estimated,return=representation"
    );
    let rpc = request(client().rpc_json("test", &()));
    assert!(!rpc.headers().contains_key("prefer"));
    assert_eq!(rpc.headers()["accept"], "application/json");
    assert!(rpc.url().query().is_none());
}

#[test]
fn pagination_counts_ordering_and_duplicate_raw_pairs_keep_protocol_semantics() {
    let mut builder = client()
        .from("items")
        .range(4, 9)
        .exact_count()
        .limit(0)
        .foreign_table_limit(2, "cities")
        .order("id.desc")
        .order_with_options("name", None::<String>, true, false)
        .order_with_options("id", Some("cities"), false, true)
        .order_with_options("name", Some("cities"), true, false);
    builder
        .append_query("id", "eq.1")
        .append_query("id", "neq.2");
    assert_eq!(
        builder
            .query_pairs()
            .filter(|(key, _)| *key == "id")
            .collect::<Vec<_>>(),
        [("id", "eq.1"), ("id", "neq.2")]
    );
    let request = request(builder);
    let pairs: Vec<_> = request.url().query_pairs().collect();
    assert!(
        pairs
            .iter()
            .any(|(key, value)| key == "limit" && value == "0")
    );
    assert!(
        pairs
            .iter()
            .any(|(key, value)| key == "cities.limit" && value == "2")
    );
    assert_eq!(pairs.iter().filter(|(key, _)| key == "order").count(), 1);
    assert!(
        pairs
            .iter()
            .any(|(key, value)| key == "order" && value == "id.desc,name.asc.nullslast")
    );
    assert!(
        pairs.iter().any(|(key, value)| key == "cities.order"
            && value == "id.desc.nullsfirst,name.asc.nullslast")
    );
    assert_eq!(request.headers()["range"], "4-9");
    assert_eq!(request.headers()["prefer"], "count=exact");
}

#[expect(
    clippy::unwrap_used,
    reason = "Assertions describe method, schema and raw body contracts"
)]
#[test]
fn method_profile_customization_and_raw_bodies_survive_build() {
    let base = client().schema("inherited");
    let get = request(base.from("items").schema("local").method(Method::HEAD));
    assert_eq!(get.method(), Method::HEAD);
    assert_eq!(get.headers()["accept-profile"], "local");
    for builder in [
        base.from("items").insert(" raw JSON "),
        base.from("items").upsert(" raw JSON "),
        base.from("items").update(" raw JSON "),
        base.rpc("f", " raw JSON "),
    ] {
        let request = builder
            .schema("local")
            .build()
            .unwrap()
            .header("x-custom", "preserved")
            .build()
            .unwrap();
        assert_eq!(request.headers()["content-profile"], "local");
        assert_eq!(request.headers()["x-custom"], "preserved");
        assert_eq!(request.body().unwrap().as_bytes().unwrap(), b" raw JSON ");
    }
    let delete = request(base.from("items").delete().single());
    assert_eq!(delete.method(), Method::DELETE);
    assert_eq!(
        delete.headers()["accept"],
        "application/vnd.pgrst.object+json"
    );
}

struct Fails(&'static str);
impl serde::Serialize for Fails {
    fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom(self.0))
    }
}
#[expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "Assertions and source matches enforce sticky serialization failures"
)]
#[test]
fn errors_are_fallible_and_first_serialization_failure_is_sticky() {
    for url in [
        "not a URL",
        "ftp://example.test",
        "http://user:password@example.test",
        "http://example.test?query",
        "http://example.test#fragment",
    ] {
        assert!(matches!(Postgrest::new(url), Err(Error::Configuration(_))));
    }
    assert!(matches!(
        client().auth("bad\nvalue"),
        Err(Error::Configuration(ConfigError::HeaderValue(_)))
    ));
    assert!(matches!(
        client().insert_header("bad name", "value"),
        Err(Error::Configuration(ConfigError::HeaderName(_)))
    ));
    assert!(matches!(
        client().insert_header("x-test", "bad\nvalue"),
        Err(Error::Configuration(ConfigError::HeaderValue(_)))
    ));
    assert!(matches!(
        client().from("items").schema("bad\nvalue").build(),
        Err(Error::Configuration(ConfigError::HeaderValue(_)))
    ));
    assert!(matches!(
        client().from("items").auth("bad\nvalue").build(),
        Err(Error::Configuration(ConfigError::HeaderValue(_)))
    ));
    assert!(matches!(
        client()
            .from("items")
            .insert_header("bad name", "value")
            .build(),
        Err(Error::Configuration(ConfigError::HeaderName(_)))
    ));
    let builder = client()
        .from("items")
        .insert_json(&Fails("first"))
        .update_json(&Fails("second"))
        .insert("{}");
    let error = builder.build().unwrap_err();
    let Error::Serialization(source) = error else {
        panic!("unexpected error: {error:?}");
    };
    assert_eq!(source.to_string(), "first");
    assert!(matches!(
        client().rpc_json("f", &Fails("rpc")).build(),
        Err(Error::Serialization(_))
    ));
}
