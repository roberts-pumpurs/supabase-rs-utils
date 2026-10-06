#![expect(
    clippy::tests_outside_test_module,
    reason = "Integration tests exercise public interfaces."
)]
#![expect(clippy::unwrap_used, reason = "Assertions retain failure details.")]

use rp_supabase_client::{
    rp_postgrest::reqwest,
    schema::params::{self, Op, QueryPair},
};

fn request(pairs: &[QueryPair]) -> reqwest::Request {
    reqwest::Client::new()
        .get("https://example.supabase.co/rest/v1/runtime_table")
        .query(pairs)
        .build()
        .unwrap()
}

#[test]
fn runtime_comparisons_and_nested_groups_serialize_once() {
    let pair = params::filter("score", Op::Gt, 10_i32);
    assert_eq!(
        request(core::slice::from_ref(&pair)).url().query().unwrap(),
        "score=gt.10"
    );
    let nested = params::and(&[pair, params::filter("status", Op::Eq, "active")]).unwrap();
    let group = params::or(&[nested, params::filter("score", Op::Lte, 0_i32)]).unwrap();
    assert_eq!(
        group.1,
        r#"(and(score.gt."10",status.eq."active"),score.lte."0")"#
    );
    assert_eq!(
        request(&[group]).url().query().unwrap(),
        "or=%28and%28score.gt.%2210%22%2Cstatus.eq.%22active%22%29%2Cscore.lte.%220%22%29"
    );
}

#[test]
fn grammar_punctuation_stays_inside_literal_identifiers_and_values() {
    let column = "comma,name.period(quote)\"slash\\";
    let value = "x),or(admin.eq.true),y.z\"\\";
    let scalar = params::filter(column, Op::Eq, value);
    assert_eq!(scalar.0, r#""comma,name.period(quote)\"slash\\""#);
    assert_eq!(scalar.1, format!("eq.{value}"));
    let group = params::and(&[scalar]).unwrap();
    assert_eq!(
        group.1,
        r#"("comma,name.period(quote)\"slash\\".eq."x),or(admin.eq.true),y.z\"\\")"#
    );
    let serialized = request(core::slice::from_ref(&group));
    assert_eq!(
        serialized.url().query_pairs().collect::<Vec<_>>(),
        vec![(group.0.clone(), group.1.clone())]
    );
    assert!(serialized.url().query().unwrap().contains("%5C%22"));
}

#[test]
fn composition_rejects_non_filters_and_unchecked_nested_literals() {
    params::or(&[]).unwrap_err();
    params::and(&[params::limit(5)]).unwrap_err();
    params::or(&[("or".into(), "(name.eq.x,admin.eq.true)".into())]).unwrap_err();
    params::or(&[("or".into(), "(name.eq.\"x\")trailing".into())]).unwrap_err();
    params::or(&[("name,admin".into(), "eq.x".into())]).unwrap_err();
}

#[test]
fn is_predicates_use_unquoted_tokens_in_nested_groups() {
    let null = params::or(&[("deleted_at".into(), "is.null".into())]).unwrap();
    let nested = params::and(&[null, ("enabled".into(), "is.true".into())]).unwrap();
    assert_eq!(nested.1, "(or(deleted_at.is.null),enabled.is.true)");
    assert_eq!(
        request(&[nested]).url().query().unwrap(),
        "and=%28or%28deleted_at.is.null%29%2Cenabled.is.true%29"
    );
    for literal in ["false", "unknown"] {
        let pair = params::or(&[("enabled".into(), format!("is.{literal}").into())]).unwrap();
        assert_eq!(pair.1, format!("(enabled.is.{literal})"));
    }
    params::or(&[("deleted_at".into(), "is.\"null\"".into())]).unwrap_err();
    params::or(&[("or".into(), "(deleted_at.is.\"null\")".into())]).unwrap_err();
    params::or(&[("enabled".into(), "is.true,admin.eq.true".into())]).unwrap_err();
}

#[test]
fn composite_cursor_uses_ascending_lexicographic_comparisons() {
    let cursor = params::after(&[("org_id", 7_i32), ("user_id", 42_i32)]).unwrap();
    assert_eq!(
        cursor.1,
        r#"(org_id.gt."7",and(org_id.eq."7",user_id.gt."42"))"#
    );
    assert_eq!(
        request(core::slice::from_ref(&cursor))
            .url()
            .query()
            .unwrap(),
        "or=%28org_id.gt.%227%22%2Cand%28org_id.eq.%227%22%2Cuser_id.gt.%2242%22%29%29"
    );
    assert!(params::after::<i32>(&[]).is_none());
    let three = params::after(&[("a", 1_i32), ("b", 2_i32), ("c", 3_i32)]).unwrap();
    assert_eq!(
        three.1,
        r#"(a.gt."1",and(a.eq."1",b.gt."2"),and(a.eq."1",b.eq."2",c.gt."3"))"#
    );
}

#[test]
fn cursor_escapes_literal_columns_and_values() {
    let cursor = params::after(&[("org.id", "a,b"), ("user(id)", "q\"\\")]).unwrap();
    assert_eq!(
        cursor.1,
        r#"("org.id".gt."a,b",and("org.id".eq."a,b","user(id)".gt."q\"\\"))"#
    );
    let serialized = request(core::slice::from_ref(&cursor));
    assert_eq!(
        serialized.url().query_pairs().collect::<Vec<_>>(),
        vec![(cursor.0.clone(), cursor.1.clone())]
    );
}
