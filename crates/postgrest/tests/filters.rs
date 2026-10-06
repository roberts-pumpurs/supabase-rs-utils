#![expect(
    clippy::tests_outside_test_module,
    reason = "Integration tests exercise public interfaces."
)]

use rp_postgrest::{Builder, Postgrest};

#[expect(clippy::unwrap_used, reason = "The fixed test URL is valid")]
fn query() -> Builder {
    Postgrest::new("https://example.com/rest/v1/")
        .unwrap()
        .from("records")
}

#[expect(
    clippy::unwrap_used,
    reason = "Valid filter fixtures must build requests"
)]
fn pairs(builder: Builder) -> Vec<(String, String)> {
    builder
        .build()
        .unwrap()
        .build()
        .unwrap()
        .url()
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect()
}

#[expect(
    clippy::non_ascii_literal,
    reason = "Literal Unicode verifies scalar filter transport encoding"
)]
#[test]
fn scalar_literals_survive_transport_encoding_and_repeated_columns() {
    let literal = " a,b.(c):\"d\"\\e%&+?#/雪 ";
    let actual = pairs(
        query()
            .eq("child.alias->>value", literal)
            .neq("child.alias->>value", literal)
            .gt("gt", literal)
            .gte("gte", literal)
            .lt("lt", literal)
            .lte("lte", literal)
            .like("like", literal)
            .ilike("ilike", literal)
            .is("is", "null")
            .not("eq", "negated", literal),
    );
    let expected = [
        ("child.alias->>value", "eq"),
        ("child.alias->>value", "neq"),
        ("gt", "gt"),
        ("gte", "gte"),
        ("lt", "lt"),
        ("lte", "lte"),
        ("like", "like"),
        ("ilike", "ilike"),
    ]
    .into_iter()
    .map(|(key, operator)| (key.to_owned(), format!("{operator}.{literal}")))
    .chain([
        ("is".to_owned(), "is.null".to_owned()),
        ("negated".to_owned(), format!("not.eq.{literal}")),
    ])
    .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

#[test]
fn boolean_and_raw_list_fragments_keep_their_grammar() {
    let grammar = r#"name.eq."a,b",or(id.gte.1,capital.is.null)"#;
    assert_eq!(
        pairs(
            query()
                .and(grammar)
                .or(grammar)
                .in_("name", [r#""Paris,France""#, r#""a\"b\\c""#, "null"])
                .in_("empty", core::iter::empty::<&str>())
                .not("in", "name", r#"("x,y",null)"#),
        ),
        vec![
            ("and".to_owned(), format!("({grammar})")),
            ("or".to_owned(), format!("({grammar})")),
            (
                "name".to_owned(),
                r#"in.("Paris,France","a\"b\\c",null)"#.to_owned()
            ),
            ("empty".to_owned(), "in.()".to_owned()),
            ("name".to_owned(), r#"not.in.("x,y",null)"#.to_owned()),
        ],
    );
}

#[expect(
    clippy::non_ascii_literal,
    reason = "Literal Unicode verifies quoted list encoding without changing its grammar"
)]
#[test]
fn literal_list_quotes_elements_without_changing_raw_list_behavior() {
    assert_eq!(
        pairs(
            query()
                .in_values(
                    "name",
                    ["a,b", "(x)", "a.b:c", "", "null", "a\"b\\c", "雪%&+"]
                )
                .in_values("empty", core::iter::empty::<&str>()),
        ),
        vec![
            (
                "name".to_owned(),
                r#"in.("a,b","(x)","a.b:c","","null","a\"b\\c","雪%&+")"#.to_owned(),
            ),
            ("empty".to_owned(), "in.()".to_owned()),
        ],
    );
}

#[test]
fn containment_and_range_operators_keep_raw_boundaries() {
    assert_eq!(
        pairs(
            query()
                .cs("json", r#"{"name":"a,b","nested":[1,2]}"#)
                .cd(String::from("array"), r#"{"a,b","c"}"#)
                .ov("range", "[10,20)")
                .sl("range", (-10, 20))
                .sr("range", (10, 20))
                .nxl("range", (10, 20))
                .nxr("range", (10, 20))
                .adj("range", (i64::MIN, i64::MAX))
                .not("ov", "range", "(10,20]"),
        ),
        vec![
            (
                "json".to_owned(),
                r#"cs.{"name":"a,b","nested":[1,2]}"#.to_owned()
            ),
            ("array".to_owned(), r#"cd.{"a,b","c"}"#.to_owned()),
            ("range".to_owned(), "ov.[10,20)".to_owned()),
            ("range".to_owned(), "sl.(-10,20)".to_owned()),
            ("range".to_owned(), "sr.(10,20)".to_owned()),
            ("range".to_owned(), "nxl.(10,20)".to_owned()),
            ("range".to_owned(), "nxr.(10,20)".to_owned()),
            (
                "range".to_owned(),
                format!("adj.({},{})", i64::MIN, i64::MAX)
            ),
            ("range".to_owned(), "not.ov.(10,20]".to_owned()),
        ],
    );
}

#[expect(
    clippy::non_ascii_literal,
    reason = "Literal Unicode verifies full text query transport encoding"
)]
#[test]
fn every_full_text_operator_preserves_query_and_optional_configuration() {
    let text = "'fat' & ('cat':* | '雪')\\ + %";
    let config = "custom.schema_config";
    let actual = pairs(
        query()
            .fts("document", text, None)
            .fts("document", text, Some(config))
            .plfts("document", text, None)
            .plfts("document", text, Some(config))
            .phfts("document", text, None)
            .phfts("document", text, Some(config))
            .wfts("document", text, None)
            .wfts("document", text, Some(config))
            .fts("document", text, Some("")),
    );
    let mut expected = Vec::new();
    for operator in ["fts", "plfts", "phfts", "wfts"] {
        expected.push(("document".to_owned(), format!("{operator}.{text}")));
        expected.push((
            "document".to_owned(),
            format!("{operator}({config}).{text}"),
        ));
    }
    expected.push(("document".to_owned(), format!("fts().{text}")));
    assert_eq!(actual, expected);
}
