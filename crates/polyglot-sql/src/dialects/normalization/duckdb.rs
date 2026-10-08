//! Preserve DuckDB's type-dependent `//` semantics before target rewrites.

use super::scalar::{expression_numeric_kind, NumericKind};
use super::*;

fn combine(left: NumericKind, right: NumericKind) -> NumericKind {
    use NumericKind::*;
    match (left, right) {
        (Unknown, _) | (_, Unknown) => Unknown,
        (Float, _) | (_, Float) => Float,
        (Decimal, _) | (_, Decimal) => Decimal,
        (Integer, Integer) => Integer,
    }
}

fn numeric_kind(expr: &Expression) -> NumericKind {
    match expr {
        Expression::Paren(p) => numeric_kind(&p.this),
        Expression::Neg(n) => numeric_kind(&n.this),
        Expression::Alias(a) => numeric_kind(&a.this),
        Expression::Add(b) | Expression::Sub(b) | Expression::Mul(b) | Expression::Mod(b) => {
            combine(numeric_kind(&b.left), numeric_kind(&b.right))
        }
        // The complete bottom-up source pass has already lowered fractional
        // children to Div. Remaining IntDiv children have integer operands.
        Expression::IntDiv(_) => NumericKind::Integer,
        Expression::Div(_) => NumericKind::Float,
        Expression::Function(f) if f.name.eq_ignore_ascii_case("NULLIF") && f.args.len() == 2 => {
            numeric_kind(&f.args[0])
        }
        Expression::NullIf(f) => numeric_kind(&f.this),
        _ => expression_numeric_kind(expr),
    }
}

/// The divisor shape this pass generates: `NULLIF(b, 0)`. A `Div` with that
/// divisor is a lowered `//`; any other `Div` is the source's own `/`.
fn is_lowered_divisor(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::Function(f)
            if f.name.eq_ignore_ascii_case("NULLIF")
                && f.args.len() == 2
                && matches!(&f.args[1], Expression::Literal(lit)
                    if matches!(lit.as_ref(), Literal::Number(n) if n == "0"))
    )
}

/// DataFusion's `/` keeps the operands' own types, so a lowered integer `//`
/// (`a / NULLIF(b, 0)`) is still an integer there and is classified from its
/// operands. The source's own `/` is DOUBLE in DuckDB whatever its operands,
/// so it stays Float, as in `numeric_kind`.
fn datafusion_numeric_kind(expr: &Expression) -> NumericKind {
    match expr {
        Expression::Paren(p) => datafusion_numeric_kind(&p.this),
        Expression::Neg(n) => datafusion_numeric_kind(&n.this),
        Expression::Alias(a) => datafusion_numeric_kind(&a.this),
        Expression::Add(b) | Expression::Sub(b) | Expression::Mul(b) | Expression::Mod(b) => {
            combine(
                datafusion_numeric_kind(&b.left),
                datafusion_numeric_kind(&b.right),
            )
        }
        Expression::Div(b) if is_lowered_divisor(&b.right) => combine(
            datafusion_numeric_kind(&b.left),
            datafusion_numeric_kind(&b.right),
        ),
        Expression::Div(_) => NumericKind::Float,
        Expression::IntDiv(_) => NumericKind::Integer,
        Expression::Function(f) if f.name.eq_ignore_ascii_case("NULLIF") && f.args.len() == 2 => {
            datafusion_numeric_kind(&f.args[0])
        }
        Expression::NullIf(f) => datafusion_numeric_kind(&f.this),
        _ => expression_numeric_kind(expr),
    }
}

/// DuckDB's `/` is always DOUBLE, but DataFusion's `/` on two integers
/// truncates. Inside a `//` operand that difference changes the result
/// (`(7 / 2) // 2` is 1.75 in DuckDB), so cast the dividend of any such
/// source division to DOUBLE. Only arithmetic nodes are walked.
fn float_source_divisions(expr: Expression) -> Expression {
    match expr {
        Expression::Paren(mut p) => {
            p.this = float_source_divisions(p.this);
            Expression::Paren(p)
        }
        Expression::Neg(mut n) => {
            n.this = float_source_divisions(n.this);
            Expression::Neg(n)
        }
        Expression::Alias(mut a) => {
            a.this = float_source_divisions(a.this);
            Expression::Alias(a)
        }
        Expression::Add(mut b) => {
            b.left = float_source_divisions(b.left);
            b.right = float_source_divisions(b.right);
            Expression::Add(b)
        }
        Expression::Sub(mut b) => {
            b.left = float_source_divisions(b.left);
            b.right = float_source_divisions(b.right);
            Expression::Sub(b)
        }
        Expression::Mul(mut b) => {
            b.left = float_source_divisions(b.left);
            b.right = float_source_divisions(b.right);
            Expression::Mul(b)
        }
        Expression::Mod(mut b) => {
            b.left = float_source_divisions(b.left);
            b.right = float_source_divisions(b.right);
            Expression::Mod(b)
        }
        Expression::Div(mut b) => {
            b.left = float_source_divisions(b.left);
            b.right = float_source_divisions(b.right);
            if !is_lowered_divisor(&b.right)
                && datafusion_numeric_kind(&b.left) == NumericKind::Integer
                && datafusion_numeric_kind(&b.right) == NumericKind::Integer
            {
                b.left = super::operators::cast_expr(
                    b.left,
                    DataType::Double {
                        precision: None,
                        scale: None,
                    },
                );
            }
            Expression::Div(b)
        }
        other => other,
    }
}

pub(in crate::dialects) fn prepare_integer_division(
    expr: Expression,
    source: DialectType,
    target: DialectType,
) -> Result<Expression> {
    if source != DialectType::DuckDB || target == DialectType::DuckDB {
        return Ok(expr);
    }

    // Visit every physical child, including typed function arguments and both
    // operands of nested IntDiv nodes, before lowering the enclosing operator.
    crate::traversal::transform_all(expr, &|expr| {
        let Expression::IntDiv(mut division) = expr else {
            return Ok(expr);
        };
        let left_kind = numeric_kind(&division.this);
        let right_kind = numeric_kind(&division.expression);
        let kind = combine(left_kind, right_kind);

        // DataFusion's `/` is type-dependent in the same way as DuckDB's `//`:
        // integer operands truncate toward zero and a floating operand makes
        // it float division. So `a / NULLIF(b, 0)` is exact without resolving
        // operand types (an untyped column that turns out to be DECIMAL keeps
        // decimal arithmetic where DuckDB gives DOUBLE; the value is the same).
        // A known DECIMAL operand is cast to DOUBLE whenever one is involved
        // on either side, unless a float operand already decides the type.
        if target == DialectType::DataFusion {
            division.this = float_source_divisions(division.this);
            division.expression = float_source_divisions(division.expression);
            let left = datafusion_numeric_kind(&division.this);
            let right = datafusion_numeric_kind(&division.expression);
            let decimal_involved = left == NumericKind::Decimal || right == NumericKind::Decimal;
            let float_involved = left == NumericKind::Float || right == NumericKind::Float;
            if decimal_involved && !float_involved {
                division.this = super::operators::cast_expr(
                    division.this,
                    DataType::Double {
                        precision: None,
                        scale: None,
                    },
                );
            }
            let divisor = Expression::Function(Box::new(Function::new(
                "NULLIF".to_string(),
                vec![division.expression, Expression::number(0)],
            )));
            return Ok(Expression::Div(Box::new(BinaryOp::new(
                division.this,
                divisor,
            ))));
        }

        if kind == NumericKind::Unknown {
            return Err(crate::error::Error::unsupported(
                "DuckDB // with unresolved operand types; use explicit numeric casts",
                target.to_string(),
            ));
        }

        // NULLIF evaluates the divisor once and preserves NULL on zero for
        // literals, casts, computed expressions, and values only known at runtime.
        division.expression = Expression::Function(Box::new(Function::new(
            "NULLIF".to_string(),
            vec![division.expression, Expression::number(0)],
        )));

        if kind != NumericKind::Integer {
            // DuckDB decimal division produces DOUBLE, whereas some targets
            // keep decimal arithmetic (and SQLite may treat DECIMAL as INTEGER).
            // An explicitly floating operand already establishes float division.
            if kind == NumericKind::Decimal {
                division.this = super::operators::cast_expr(
                    division.this,
                    DataType::Double {
                        precision: None,
                        scale: None,
                    },
                );
            }
            return Ok(Expression::Div(Box::new(BinaryOp::new(
                division.this,
                division.expression,
            ))));
        }

        // Only use IntDiv where generation has a native or exact lowering.
        // A generic DIV(...) call is invalid in several other targets.
        if !matches!(
            target,
            DialectType::PostgreSQL
                | DialectType::BigQuery
                | DialectType::ClickHouse
                | DialectType::SQLite
                | DialectType::Vertica
                | DialectType::Hive
                | DialectType::Spark
                | DialectType::Databricks
        ) {
            return Err(crate::error::Error::unsupported(
                "DuckDB integer // conversion",
                target.to_string(),
            ));
        }
        Ok(Expression::IntDiv(division))
    })
}
