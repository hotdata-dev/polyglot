//! Schema-aware transpilation: column types from a `Schema` are inferred onto
//! the AST before the target transform, so type-dependent rewrites fire on
//! bare column references.
use polyglot_sql::expressions::DataType;
use polyglot_sql::schema::{MappingSchema, Schema};
use polyglot_sql::{transpile, transpile_with_schema_by_name, DialectType, TranspileOptions};

fn schema() -> MappingSchema {
    let mut schema = MappingSchema::new();
    schema
        .add_table(
            "t",
            &[
                (
                    "x".to_string(),
                    DataType::Double {
                        precision: None,
                        scale: None,
                    },
                ),
                ("s".to_string(), DataType::Text),
            ],
            None,
        )
        .unwrap();
    schema
}

fn with_schema(sql: &str) -> Vec<String> {
    transpile_with_schema_by_name(
        sql,
        "postgres",
        "tsql",
        &schema(),
        &TranspileOptions::default(),
    )
    .unwrap()
}

#[test]
fn float_column_cast_to_integer_is_rounded_when_the_type_is_known() {
    // Without a schema the column's type is unknown, so no rounding is applied.
    let plain = transpile(
        "SELECT CAST(t.x AS INT) AS v FROM t",
        DialectType::PostgreSQL,
        DialectType::TSQL,
    )
    .unwrap();
    assert_eq!(plain, vec!["SELECT CAST(t.x AS INTEGER) AS v FROM t"]);
    assert_eq!(
        with_schema("SELECT CAST(t.x AS INT) AS v FROM t"),
        vec!["SELECT CAST(ROUND(t.x, 0) AS INTEGER) AS v FROM t"]
    );
}

#[test]
fn unqualified_columns_resolve_through_the_schema() {
    assert_eq!(
        with_schema("SELECT CAST(x AS INT) AS v FROM t"),
        vec!["SELECT CAST(ROUND(x, 0) AS INTEGER) AS v FROM t"]
    );
}

#[test]
fn non_float_columns_are_left_alone() {
    assert_eq!(
        with_schema("SELECT CAST(t.s AS INT) AS v FROM t"),
        vec!["SELECT CAST(t.s AS INTEGER) AS v FROM t"]
    );
}

#[test]
fn unknown_dialect_names_are_reported() {
    let err = transpile_with_schema_by_name(
        "SELECT 1",
        "nope",
        "tsql",
        &schema(),
        &TranspileOptions::default(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("Unknown dialect"), "got: {err}");
}
