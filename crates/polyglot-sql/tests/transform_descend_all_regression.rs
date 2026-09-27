//! `transform_recursive` must descend into the children of every typed
//! function node, so an inner expression is lowered for the target before the
//! outer node is rewritten. Each case below produced SQL the target rejects
//! when the child was left in the source dialect's form.
use polyglot_sql::{transpile, DialectType};

fn t(sql: &str, from: DialectType, to: DialectType) -> String {
    transpile(sql, from, to).unwrap().join("; ")
}

#[test]
fn unary_function_argument_is_lowered() {
    // DataFusion has no ARRAY_CONSTRUCT; the inner call must become a list literal.
    assert_eq!(
        t(
            "SELECT CARDINALITY(ARRAY_CONSTRUCT(1, 2)) AS v",
            DialectType::Snowflake,
            DialectType::DataFusion
        ),
        "SELECT CARDINALITY([1, 2]) AS v"
    );
    assert_eq!(
        t(
            "SELECT array_length(list_value(1, 2)) AS v",
            DialectType::DuckDB,
            DialectType::PostgreSQL
        ),
        "SELECT ARRAY_LENGTH(ARRAY[1, 2], 1) AS v"
    );
    assert_eq!(
        t(
            "SELECT year(if(c, d1, d2)) AS v FROM t",
            DialectType::DuckDB,
            DialectType::Snowflake
        ),
        "SELECT YEAR(CASE WHEN c THEN d1 ELSE d2 END) AS v FROM t"
    );
}

#[test]
fn binary_function_argument_is_lowered() {
    assert_eq!(
        t(
            "SELECT ARRAY_APPEND(ARRAY_CONSTRUCT(1, 2), 3) AS v",
            DialectType::Snowflake,
            DialectType::DuckDB
        ),
        "SELECT LIST_APPEND([1, 2], 3) AS v"
    );
    assert_eq!(
        t(
            "SELECT ARRAY_APPEND(ARRAY_CONSTRUCT(1, 2), 3) AS v",
            DialectType::Snowflake,
            DialectType::DataFusion
        ),
        "SELECT ARRAY_APPEND([1, 2], 3) AS v"
    );
}

#[test]
fn multi_child_function_argument_is_lowered() {
    assert_eq!(
        t(
            "SELECT NVL2(ARRAY_CONSTRUCT(1), 1, 0) AS v",
            DialectType::Snowflake,
            DialectType::DataFusion
        ),
        "SELECT CASE WHEN NOT [1] IS NULL THEN 1 ELSE 0 END AS v"
    );
}
