//! `MOD(a, b)` lowered to the infix `%` must keep the call's grouping when it
//! is an operand of another operator.
use polyglot_sql::{transpile, DialectType};

#[test]
fn mod_lowered_to_percent_keeps_its_grouping_under_multiplication() {
    for target in [
        DialectType::PostgreSQL,
        DialectType::DuckDB,
        DialectType::DataFusion,
        DialectType::MySQL,
    ] {
        let out = transpile("SELECT 2 * MOD(5, 3) AS v", DialectType::Snowflake, target).unwrap();
        assert_eq!(out, vec!["SELECT 2 * (5 % 3) AS v"], "target {target:?}");
    }
}

#[test]
fn mod_lowered_to_percent_as_a_left_operand_needs_no_parentheses() {
    // `*` and `%` share precedence and associate left, so `7 % 4 * 2` already
    // groups as `(7 % 4) * 2`.
    let out = transpile(
        "SELECT MOD(7, 4) * 2 AS v",
        DialectType::Snowflake,
        DialectType::PostgreSQL,
    )
    .unwrap();
    assert_eq!(out, vec!["SELECT 7 % 4 * 2 AS v"]);
}

#[test]
fn top_level_mod_is_not_parenthesized() {
    let out = transpile(
        "SELECT MOD(a, 7) AS m FROM t",
        DialectType::PostgreSQL,
        DialectType::TSQL,
    )
    .unwrap();
    assert_eq!(out, vec!["SELECT a % 7 AS m FROM t"]);
}

#[test]
fn dialects_that_keep_the_function_form_are_unchanged() {
    let out = transpile(
        "SELECT 2 * MOD(5, 3) AS v",
        DialectType::Snowflake,
        DialectType::Oracle,
    )
    .unwrap();
    assert_eq!(out, vec!["SELECT 2 * MOD(5, 3) AS v"]);
}
