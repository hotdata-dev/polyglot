//! DuckDB `//` integer division: tokenizes to a single operator (so `7 / / 2`
//! and separated/commented slashes are rejected, matching DuckDB), parses to
//! `IntDiv` with correct precedence, and round-trips or lowers per target --
//! reporting unsupported rather than silently changing results when a
//! float operand or a literal-zero divisor means the target's truncating
//! `DIV` can't reproduce DuckDB's actual behavior.
use polyglot_sql::{parse_one, transpile, DialectType};

#[test]
fn duckdb_int_div_round_trips() {
    let out = transpile(
        "SELECT 7 // 2 AS v",
        DialectType::DuckDB,
        DialectType::DuckDB,
    )
    .unwrap();
    assert_eq!(out, vec!["SELECT 7 // 2 AS v"]);
}

#[test]
fn duckdb_int_div_transpiles_like_other_integer_division() {
    // Same lowering the existing MySQL `DIV` operator gets for these targets.
    for target in [DialectType::PostgreSQL, DialectType::BigQuery] {
        let out = transpile("SELECT 7 // 2 AS v", DialectType::DuckDB, target).unwrap();
        assert_eq!(out, vec!["SELECT DIV(7, 2) AS v"], "target {target:?}");
    }
}

#[test]
fn duckdb_int_div_emulated_for_sqlite() {
    // SQLite has no DIV function, so // is emulated as truncating division.
    let out = transpile(
        "SELECT 7 // 2 AS v",
        DialectType::DuckDB,
        DialectType::SQLite,
    )
    .unwrap();
    assert_eq!(
        out,
        vec!["SELECT CAST(CAST(7 AS REAL) / 2 AS INTEGER) AS v"]
    );
}

#[test]
fn duckdb_int_div_binds_tighter_than_addition() {
    let out = transpile(
        "SELECT 1 + 7 // 2 AS v",
        DialectType::DuckDB,
        DialectType::DuckDB,
    )
    .unwrap();
    assert_eq!(out, vec!["SELECT 1 + 7 // 2 AS v"]);
}

#[test]
fn duckdb_int_div_precedence_is_unambiguous_in_the_ast() {
    // `1 + 7 // 2` round-trips the same either way `+`/`//` group (7), so it
    // doesn't actually prove precedence -- assert the AST shape directly,
    // and use `2 + 7 // 2` (5 if // binds tighter, 4 if + does) to show the
    // two groupings aren't interchangeable.
    use polyglot_sql::Expression;
    let expr = parse_one("SELECT 2 + 7 // 2 AS v", DialectType::DuckDB).unwrap();
    let Expression::Select(select) = &expr else {
        panic!("expected a SELECT");
    };
    let Expression::Alias(alias) = &select.expressions[0] else {
        panic!("expected an aliased projection");
    };
    let Expression::Add(add) = &alias.this else {
        panic!(
            "expected the top-level operator to be +, got {:?}",
            alias.this
        );
    };
    assert!(
        matches!(add.right, Expression::IntDiv(_)),
        "expected // to bind tighter than + (Add(2, IntDiv(7, 2))), got {add:?}"
    );
}

#[test]
fn duckdb_int_div_rejects_separated_slashes() {
    // DuckDB requires a contiguous `//`; `a / / b` and `a / /* c */ / b` are
    // invalid DuckDB SQL and must not be accepted as integer division.
    assert!(parse_one("SELECT 7 / / 2", DialectType::DuckDB).is_err());
    assert!(parse_one("SELECT 7 / /* c */ / 2", DialectType::DuckDB).is_err());
}

#[test]
fn duckdb_int_div_on_float_operand_is_unsupported() {
    // DuckDB's // falls back to ordinary float division when either operand
    // is non-integer (`7.0 // 2` is `3.5`, not `3`). Every other target's
    // integer division truncates unconditionally -- including Vertica's own
    // `//` and ClickHouse's intDiv -- so this must be reported, not silently
    // transpiled to a different result. Negated, parenthesized, and
    // exponent-form literals count too.
    for target in [
        DialectType::PostgreSQL,
        DialectType::BigQuery,
        DialectType::SQLite,
        DialectType::Vertica,
        DialectType::ClickHouse,
    ] {
        for sql in [
            "SELECT 7.0 // 2 AS v",
            "SELECT -7.0 // 2 AS v",
            "SELECT (7.0) // 2 AS v",
            "SELECT 7e0 // 2 AS v",
            "SELECT 7 // 2.0 AS v",
        ] {
            let err = transpile(sql, DialectType::DuckDB, target).unwrap_err();
            assert!(
                err.to_string().contains("float"),
                "{sql} -> {target:?}: got {err}"
            );
        }
    }
}

#[test]
fn duckdb_int_div_by_literal_zero_is_unsupported() {
    // DuckDB's 7 // 0 returns NULL; PostgreSQL's DIV(7, 0), BigQuery's
    // DIV(7, 0), and ClickHouse's intDiv(7, 0) all raise an error.
    for target in [
        DialectType::PostgreSQL,
        DialectType::BigQuery,
        DialectType::ClickHouse,
    ] {
        let err = transpile("SELECT 7 // 0 AS v", DialectType::DuckDB, target).unwrap_err();
        assert!(
            err.to_string().contains("zero"),
            "target {target:?}: got {err}"
        );
    }
}

#[test]
fn other_dialects_integer_division_is_unaffected() {
    // The float/zero guards are about DuckDB's // semantics specifically.
    // MySQL's and Vertica's integer division genuinely truncates float
    // operands, so their existing lowering must keep working.
    assert_eq!(
        transpile("SELECT 7.5 DIV 2", DialectType::MySQL, DialectType::MySQL).unwrap(),
        vec!["SELECT DIV(7.5, 2)"]
    );
    assert_eq!(
        transpile(
            "SELECT 7.5 // 2",
            DialectType::Vertica,
            DialectType::PostgreSQL
        )
        .unwrap(),
        vec!["SELECT DIV(7.5, 2)"]
    );
}

#[test]
fn plain_division_is_unchanged_in_duckdb_and_elsewhere() {
    let out = transpile(
        "SELECT 7 / 2 AS v",
        DialectType::DuckDB,
        DialectType::DuckDB,
    )
    .unwrap();
    assert_eq!(out, vec!["SELECT 7 / 2 AS v"]);
    let out = transpile(
        "SELECT 7 / 2 AS v",
        DialectType::PostgreSQL,
        DialectType::PostgreSQL,
    )
    .unwrap();
    assert_eq!(out, vec!["SELECT 7 / 2 AS v"]);
}
