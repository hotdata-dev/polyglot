//! DuckDB `ORDER BY ALL`: round-trips unquoted for dialects with native
//! support, expands to positional `ORDER BY 1..n` for dialects without it,
//! and is rejected when the column count can't be known statically (`*` or
//! `COLUMNS(...)`). Also covers that the bare `ALL` keyword never leaks into
//! or out of ordinary identifier handling, that the positional expansion
//! survives the `DISTINCT ON` window-function rewrite, and that NULL
//! ordering is preserved.
use polyglot_sql::{generate, parse_one, rename_columns, DialectType};

fn transpile(sql: &str, source: DialectType, target: DialectType) -> String {
    polyglot_sql::transpile(sql, source, target)
        .unwrap()
        .remove(0)
}

#[test]
fn order_by_all_round_trips_unquoted_in_duckdb() {
    assert_eq!(
        transpile(
            "SELECT a, b FROM t ORDER BY ALL",
            DialectType::DuckDB,
            DialectType::DuckDB
        ),
        "SELECT a, b FROM t ORDER BY ALL"
    );
}

#[test]
fn order_by_all_passes_through_for_native_targets() {
    // Snowflake, ClickHouse, and Databricks natively support ORDER BY ALL, so
    // the keyword is kept rather than expanded to positions -- including over
    // `*`, where expansion would have been impossible. DuckDB's ALL sorts
    // NULLs last in both directions; the NULLS clause is spelled out only
    // where the target's own default differs (Databricks ASC defaults to
    // NULLs first; Snowflake DESC does too; ClickHouse already matches).
    let cases = [
        (DialectType::Snowflake, "ALL", "ALL DESC NULLS LAST"),
        (DialectType::ClickHouse, "ALL", "ALL DESC"),
        (DialectType::Databricks, "ALL NULLS LAST", "ALL DESC"),
    ];
    for (target, asc, desc) in cases {
        assert_eq!(
            transpile(
                "SELECT a, b FROM t ORDER BY ALL",
                DialectType::DuckDB,
                target
            ),
            format!("SELECT a, b FROM t ORDER BY {asc}"),
            "target {target:?}"
        );
        assert_eq!(
            transpile(
                "SELECT a, b FROM t ORDER BY ALL DESC",
                DialectType::DuckDB,
                target
            ),
            format!("SELECT a, b FROM t ORDER BY {desc}"),
            "target {target:?}"
        );
        assert_eq!(
            transpile("SELECT * FROM t ORDER BY ALL", DialectType::DuckDB, target),
            format!("SELECT * FROM t ORDER BY {asc}"),
            "target {target:?}"
        );
    }
}

#[test]
fn order_by_all_expands_from_any_native_source() {
    // The marker must be expanded whenever the *source* has native ORDER BY
    // ALL and the target doesn't -- not only for DuckDB -- otherwise the bare
    // keyword leaks out as a nonexistent `"ALL"` column. Each source's own
    // NULL default carries over (Databricks sorts NULLs first for ASC, which
    // PostgreSQL doesn't).
    let cases = [
        (DialectType::Snowflake, "1, 2"),
        (DialectType::ClickHouse, "1, 2"),
        (DialectType::Databricks, "1 NULLS FIRST, 2 NULLS FIRST"),
    ];
    for (source, expected) in cases {
        assert_eq!(
            transpile(
                "SELECT a, b FROM t ORDER BY ALL",
                source,
                DialectType::PostgreSQL
            ),
            format!("SELECT a, b FROM t ORDER BY {expected}"),
            "source {source:?}"
        );
    }
}

#[test]
fn order_by_all_expands_to_positional_for_targets_without_native_support() {
    // PostgreSQL's own ASC default is also NULLS LAST, so (like DataFusion,
    // covered separately below) the explicit ordering DuckDB's ALL implies
    // is elided rather than printed redundantly.
    assert_eq!(
        transpile(
            "SELECT a, b FROM t ORDER BY ALL",
            DialectType::DuckDB,
            DialectType::PostgreSQL
        ),
        "SELECT a, b FROM t ORDER BY 1, 2"
    );
}

#[test]
fn order_by_all_expansion_keeps_the_direction() {
    // DuckDB sorts NULLs last for DESC too, whereas PostgreSQL defaults DESC to
    // NULLS FIRST, so the expansion carries the explicit null order across.
    assert_eq!(
        transpile(
            "SELECT a, b FROM t ORDER BY ALL DESC",
            DialectType::DuckDB,
            DialectType::PostgreSQL
        ),
        "SELECT a, b FROM t ORDER BY 1 DESC NULLS LAST, 2 DESC NULLS LAST"
    );
}

#[test]
fn order_by_all_reports_unsupported_when_mysql_cant_preserve_null_order() {
    // DuckDB's implicit default is NULLs last in both directions. MySQL has
    // no NULLS FIRST/LAST syntax, and its own ASC default is NULLs *first*
    // (NULL sorts as the smallest value) -- the opposite of what DuckDB's
    // ORDER BY ALL needs here. Silently emitting plain `ORDER BY 1` would
    // change which rows a LIMIT selects, so this must be reported as
    // unsupported rather than transpiled incorrectly.
    let err = polyglot_sql::transpile(
        "SELECT a FROM t ORDER BY ALL",
        DialectType::DuckDB,
        DialectType::MySQL,
    )
    .unwrap_err();
    assert!(err.to_string().contains("NULL"), "got: {err}");
}

#[test]
fn order_by_all_expansion_omits_nulls_clause_for_datafusion() {
    // The cross-dialect NULL-ordering pass models DataFusion, like DuckDB, as
    // sorting NULLs last in both directions, so no clause is needed.
    assert_eq!(
        transpile(
            "SELECT a, b FROM t ORDER BY ALL",
            DialectType::DuckDB,
            DialectType::DataFusion
        ),
        "SELECT a, b FROM t ORDER BY 1, 2"
    );
}

#[test]
fn order_by_all_expands_inside_subqueries() {
    assert_eq!(
        transpile(
            "SELECT a FROM t WHERE a IN (SELECT x, y FROM u ORDER BY ALL LIMIT 1)",
            DialectType::DuckDB,
            DialectType::PostgreSQL
        ),
        "SELECT a FROM t WHERE a IN (SELECT x, y FROM u ORDER BY 1, 2 LIMIT 1)"
    );
}

#[test]
fn order_by_all_over_star_is_unsupported_for_other_targets() {
    let err = polyglot_sql::transpile(
        "SELECT * FROM t ORDER BY ALL",
        DialectType::DuckDB,
        DialectType::PostgreSQL,
    )
    .unwrap_err();
    assert!(err.to_string().contains("ORDER BY ALL"), "got: {err}");
}

#[test]
fn order_by_all_over_columns_macro_is_unsupported_for_other_targets() {
    // COLUMNS(...) can expand to any number of output columns, so (like `*`)
    // the positional count can't be derived statically.
    let err = polyglot_sql::transpile(
        "SELECT COLUMNS('a|b') FROM t ORDER BY ALL",
        DialectType::DuckDB,
        DialectType::PostgreSQL,
    )
    .unwrap_err();
    assert!(err.to_string().contains("ORDER BY ALL"), "got: {err}");
}

#[test]
fn a_quoted_column_named_all_is_not_expanded() {
    assert_eq!(
        transpile(
            "SELECT a, b FROM t ORDER BY \"all\"",
            DialectType::DuckDB,
            DialectType::PostgreSQL
        ),
        "SELECT a, b FROM t ORDER BY \"all\""
    );
}

#[test]
fn an_ordinary_column_renamed_to_all_is_still_quoted() {
    // Renaming a column to the literal name "all" must still round-trip as a
    // properly quoted identifier in DuckDB output — it must not be emitted
    // bare, which DuckDB would then parse back as the ORDER BY ALL keyword.
    use std::collections::HashMap;
    let expr = parse_one("SELECT x FROM t", DialectType::DuckDB).unwrap();
    let mapping = HashMap::from([("x".to_string(), "all".to_string())]);
    let renamed = rename_columns(expr, &mapping);
    let out = generate(&renamed, DialectType::DuckDB).unwrap();
    assert_eq!(out, "SELECT \"all\" FROM t");
}

#[test]
fn order_by_all_on_union_expands_using_left_side_width() {
    assert_eq!(
        transpile(
            "SELECT a, b FROM t UNION ALL SELECT c, d FROM u ORDER BY ALL",
            DialectType::DuckDB,
            DialectType::PostgreSQL
        ),
        "SELECT a, b FROM t UNION ALL SELECT c, d FROM u ORDER BY 1, 2"
    );
}

#[test]
fn order_by_all_on_parenthesized_union_operands_expands() {
    // Parenthesized set-operation operands parse as subqueries; the width
    // lookup must see through them rather than treat the count as unknown.
    assert_eq!(
        transpile(
            "(SELECT a, b FROM t) UNION (SELECT c, d FROM u) ORDER BY ALL",
            DialectType::DuckDB,
            DialectType::PostgreSQL
        ),
        "(SELECT a, b FROM t) UNION (SELECT c, d FROM u) ORDER BY 1, 2"
    );
}

#[test]
fn order_by_all_on_union_passes_through_for_native_targets() {
    assert_eq!(
        transpile(
            "SELECT a, b FROM t UNION ALL SELECT c, d FROM u ORDER BY ALL",
            DialectType::DuckDB,
            DialectType::Snowflake
        ),
        "SELECT a, b FROM t UNION ALL SELECT c, d FROM u ORDER BY ALL"
    );
}

#[test]
fn order_by_all_on_union_over_star_is_unsupported() {
    let err = polyglot_sql::transpile(
        "SELECT * FROM t UNION SELECT * FROM u ORDER BY ALL",
        DialectType::DuckDB,
        DialectType::PostgreSQL,
    )
    .unwrap_err();
    assert!(err.to_string().contains("ORDER BY ALL"), "got: {err}");
}

#[test]
fn order_by_all_survives_distinct_on_rewrite() {
    // SQLite has no DISTINCT ON, so it's emulated with a ROW_NUMBER() window.
    // The positional ORDER BY produced by expanding ALL must be resolved to
    // the actual projected columns before landing inside that window's own
    // ORDER BY, where a bare "1"/"2" would mean a literal constant, not a
    // positional column reference, and silently change the result set.
    let out = transpile(
        "SELECT DISTINCT ON (a) a, b FROM t ORDER BY ALL",
        DialectType::DuckDB,
        DialectType::SQLite,
    );
    assert!(!out.contains("ORDER BY 1"), "got: {out}");
    assert!(out.contains("PARTITION BY a"), "got: {out}");
    assert!(out.contains("ORDER BY a"), "got: {out}");
}
