//! ClickHouse query parameters, `{name:Type}`, whose name is a reserved word.

use polyglot_sql::expressions::Expression;
use polyglot_sql::{parse, transpile, DialectType, ExpressionWalk};

fn parameters(sql: &str) -> Vec<(String, String)> {
    let statements = parse(sql, DialectType::ClickHouse)
        .unwrap_or_else(|e| panic!("failed to parse {sql:?}: {e}"));
    let mut found = Vec::new();
    for statement in &statements {
        for node in statement.dfs() {
            if let Expression::Parameter(p) = node {
                found.push((
                    p.name.clone().unwrap_or_default(),
                    p.expression.clone().unwrap_or_default(),
                ));
            }
        }
    }
    found
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(n, t)| (n.to_string(), t.to_string()))
        .collect()
}

#[test]
fn a_reserved_word_is_a_parameter_name() {
    assert_eq!(
        parameters("SELECT a FROM t WHERE ts >= {from:DateTime} AND ts < {to:DateTime}"),
        pairs(&[("from", "DateTime"), ("to", "DateTime")])
    );
    assert_eq!(
        parameters(
            "SELECT {select:UInt8}, {where:String}, {order:Array(String)}, {group:Nullable(Int64)} \
             FROM t LIMIT {limit:UInt32}"
        ),
        pairs(&[
            ("select", "UInt8"),
            ("where", "String"),
            ("order", "Array(String)"),
            ("group", "Nullable(Int64)"),
            ("limit", "UInt32"),
        ])
    );
}

#[test]
fn a_reserved_name_round_trips() {
    let sql = "SELECT a FROM t WHERE ts >= {from: DateTime} AND b = {name: String}";
    let out = transpile(sql, DialectType::ClickHouse, DialectType::ClickHouse).unwrap();
    assert_eq!(out, vec![sql.to_string()]);
}

#[test]
fn ordinary_names_and_map_literals_are_unchanged() {
    assert_eq!(
        parameters("SELECT {n:UInt32}, {db:Identifier}.t FROM t"),
        pairs(&[("n", "UInt32"), ("db", "Identifier")])
    );
    // A map literal with string keys is still a map.
    assert!(parameters("SELECT {'from': 1, 'to': 2}").is_empty());
}
