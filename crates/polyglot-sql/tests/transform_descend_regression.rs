//! `transform_recursive` must descend into the children of single-child typed
//! nodes (`ArraySize`, `LastDay`) so the inner expression is lowered for the
//! target before the outer node is rewritten.
use polyglot_sql::{transpile, DialectType};

#[test]
fn array_size_argument_is_lowered_for_the_target() {
    let out = transpile(
        "SELECT ARRAY_SIZE(ARRAY_CONSTRUCT(1, 2, 3)) AS v",
        DialectType::Snowflake,
        DialectType::DuckDB,
    )
    .unwrap();
    assert_eq!(out, vec!["SELECT ARRAY_LENGTH([1, 2, 3]) AS v"]);
}

#[test]
fn last_day_argument_is_lowered_for_the_target() {
    let out = transpile(
        "SELECT LAST_DAY(IFF(c, d1, d2)) AS v FROM t",
        DialectType::Snowflake,
        DialectType::DuckDB,
    )
    .unwrap();
    assert_eq!(
        out,
        vec!["SELECT LAST_DAY(CASE WHEN c THEN d1 ELSE d2 END) AS v FROM t"]
    );
}
