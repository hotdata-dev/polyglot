//! DataFusion target lowerings: Snowflake ARRAY_CONTAINS argument order,
//! JSON arrow operators, and native GENERATE_SERIES.
use polyglot_sql::{transpile, DialectType};

#[test]
fn snowflake_array_contains_swaps_arguments_and_drops_variant_cast() {
    let out = transpile(
        "SELECT ARRAY_CONTAINS(2::VARIANT, a) AS v FROM t",
        DialectType::Snowflake,
        DialectType::DataFusion,
    )
    .unwrap();
    assert_eq!(out, vec!["SELECT array_contains(a, 2) AS v FROM t"]);
}

#[test]
fn postgres_json_arrows_are_kept_for_datafusion() {
    let out = transpile(
        "SELECT j -> 'k' ->> 'n' AS v FROM t",
        DialectType::PostgreSQL,
        DialectType::DataFusion,
    )
    .unwrap();
    assert_eq!(out, vec!["SELECT j -> 'k' ->> 'n' AS v FROM t"]);
}

#[test]
fn postgres_generate_series_is_kept_native_for_datafusion() {
    let out = transpile(
        "SELECT * FROM generate_series(1, 3)",
        DialectType::PostgreSQL,
        DialectType::DataFusion,
    )
    .unwrap();
    assert_eq!(out, vec!["SELECT * FROM generate_series(1, 3)"]);
    let duck = transpile(
        "SELECT * FROM generate_series(1, 3)",
        DialectType::PostgreSQL,
        DialectType::DuckDB,
    )
    .unwrap();
    assert_eq!(duck, vec!["SELECT * FROM UNNEST(GENERATE_SERIES(1, 3))"]);
}
