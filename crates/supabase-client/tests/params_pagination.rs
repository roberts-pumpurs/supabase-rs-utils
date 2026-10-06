#![expect(
    clippy::tests_outside_test_module,
    reason = "Integration tests exercise public interfaces."
)]
#![expect(
    clippy::unwrap_used,
    reason = "Test setup and assertions retain failure details."
)]

use rp_supabase_client::{
    rp_postgrest::reqwest,
    schema::params::{self, Nulls, Order, QueryPair, RangeError},
};

fn request(pairs: &[QueryPair]) -> reqwest::Request {
    reqwest::Client::new()
        .get("https://example.supabase.co/rest/v1/runtime_table")
        .query(pairs)
        .build()
        .unwrap()
}

#[test]
fn runtime_table_parameters_are_serialized_once() {
    let mut pairs = vec![
        params::select("*,tasks(id,name)"),
        params::order_by_with_nulls("created_at", Order::Desc, Nulls::Last),
    ];
    pairs.extend(params::range(5, 9).unwrap());
    let request = request(&pairs);
    assert_eq!(
        request.url().query().unwrap(),
        "select=*%2Ctasks%28id%2Cname%29&order=created_at.desc.nullslast&offset=5&limit=5"
    );
    assert_eq!(
        request.url().query_pairs().collect::<Vec<_>>(),
        vec![
            ("select".into(), "*,tasks(id,name)".into()),
            ("order".into(), "created_at.desc.nullslast".into()),
            ("offset".into(), "5".into()),
            ("limit".into(), "5".into()),
        ]
    );
    assert_eq!(params::select("*").1, "*");
}

#[test]
fn pagination_inclusive_and_usize_boundaries() {
    let pairs = [params::limit(0), params::offset(0)];
    assert_eq!(request(&pairs).url().query().unwrap(), "limit=0&offset=0");
    assert_eq!(
        params::range(0, 0).unwrap(),
        [params::offset(0), params::limit(1)]
    );
    assert_eq!(
        params::range(usize::MAX, usize::MAX).unwrap(),
        [params::offset(usize::MAX), params::limit(1)]
    );
    assert_eq!(
        params::range(1, usize::MAX).unwrap(),
        [params::offset(1), params::limit(usize::MAX)]
    );
    assert_eq!(params::range(1, 0), Err(RangeError::Reversed));
    assert_eq!(params::range(0, usize::MAX), Err(RangeError::Overflow));
    let maximum = request(&[params::limit(usize::MAX), params::offset(usize::MAX)]);
    assert_eq!(
        maximum.url().query_pairs().collect::<Vec<_>>(),
        vec![
            ("limit".into(), usize::MAX.to_string().into()),
            ("offset".into(), usize::MAX.to_string().into()),
        ]
    );
}

#[test]
fn runtime_identifiers_and_relation_paths_are_literal() {
    let column = "comma,name.percent%\"slash\\";
    let pair = params::order_by(column, Order::Asc);
    assert_eq!(pair.1, r#""comma,name.percent%\"slash\\".asc"#);
    let scoped = params::scope(&["order", "child.alias"], pair);
    assert_eq!(scoped.0, r#""order"."child.alias".order"#);
    let serialized = request(core::slice::from_ref(&scoped));
    let decoded = serialized.url().query_pairs().collect::<Vec<_>>();
    assert_eq!(decoded, vec![(scoped.0.clone(), scoped.1.clone())]);
    assert!(serialized.url().query().unwrap().contains("%25"));
    assert!(!serialized.url().query().unwrap().contains("%2525"));
    assert_eq!(params::scope(&[], params::limit(2)), params::limit(2));
    let scoped_range = params::range(0, 2)
        .unwrap()
        .map(|pair| params::scope(&["tasks"], pair));
    assert_eq!(
        request(&scoped_range).url().query().unwrap(),
        "tasks.offset=0&tasks.limit=3"
    );
    assert_eq!(params::order_by("name", Order::Asc).1, "name.asc");
    assert_eq!(
        params::order_by_with_nulls("order", Order::Asc, Nulls::First).1,
        r#""order".asc.nullsfirst"#
    );
}
