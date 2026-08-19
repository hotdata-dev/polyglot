use crate::errors::map_parse_error;
use crate::expr::PyExpression;
use crate::expr_types::wrap_expression;
use polyglot_sql::builder::plan::{
    evaluate, BinaryOperator, BuildNode, BuildOperation, BuilderAssignment, BuilderPlan,
    BuilderValue, BuiltinFunction, JoinType, UnaryOperator,
};
use pyo3::exceptions::{PyNotImplementedError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBool, PyDict, PyString, PyTuple};

fn ensure_copy(copy: bool) -> PyResult<()> {
    if copy {
        Ok(())
    } else {
        Err(PyNotImplementedError::new_err(
            "copy=False is not supported; Polyglot builder expressions are immutable",
        ))
    }
}

fn dialect_name(dialect: Option<&str>) -> &str {
    dialect.unwrap_or("generic")
}

pub(crate) fn binary_operator(value: &str) -> PyResult<BinaryOperator> {
    Ok(match value {
        "eq" => BinaryOperator::Eq,
        "neq" => BinaryOperator::Neq,
        "lt" => BinaryOperator::Lt,
        "lte" => BinaryOperator::Lte,
        "gt" => BinaryOperator::Gt,
        "gte" => BinaryOperator::Gte,
        "and" => BinaryOperator::And,
        "or" => BinaryOperator::Or,
        "xor" => BinaryOperator::Xor,
        "add" => BinaryOperator::Add,
        "sub" => BinaryOperator::Sub,
        "mul" => BinaryOperator::Mul,
        "div" => BinaryOperator::Div,
        "mod" => BinaryOperator::Mod,
        "like" => BinaryOperator::Like,
        "ilike" => BinaryOperator::Ilike,
        "rlike" => BinaryOperator::Rlike,
        "is" => BinaryOperator::Is,
        _ => {
            return Err(PyValueError::new_err(format!(
                "unsupported binary operator: {value}"
            )))
        }
    })
}

pub(crate) fn unary_operator(value: &str) -> PyResult<UnaryOperator> {
    Ok(match value {
        "not" => UnaryOperator::Not,
        "neg" => UnaryOperator::Neg,
        "is_null" => UnaryOperator::IsNull,
        "is_not_null" => UnaryOperator::IsNotNull,
        _ => {
            return Err(PyValueError::new_err(format!(
                "unsupported unary operator: {value}"
            )))
        }
    })
}

pub(crate) fn join_type(value: &str) -> PyResult<JoinType> {
    let normalized = value.trim().to_ascii_lowercase();
    Ok(match normalized.as_str() {
        "" | "join" | "inner" | "inner join" => JoinType::Inner,
        "left" | "left join" | "left outer" | "left outer join" => JoinType::Left,
        "right" | "right join" | "right outer" | "right outer join" => JoinType::Right,
        "full" | "full join" | "full outer" | "full outer join" => JoinType::Full,
        "cross" | "cross join" => JoinType::Cross,
        _ => {
            return Err(PyValueError::new_err(format!(
                "unsupported join type: {value}"
            )))
        }
    })
}

fn builtin_function(value: &str) -> PyResult<BuiltinFunction> {
    Ok(match value {
        "count" => BuiltinFunction::Count,
        "count_star" => BuiltinFunction::CountStar,
        "count_distinct" => BuiltinFunction::CountDistinct,
        "sum" => BuiltinFunction::Sum,
        "avg" => BuiltinFunction::Avg,
        "min" => BuiltinFunction::Min,
        "max" => BuiltinFunction::Max,
        "approx_distinct" => BuiltinFunction::ApproxDistinct,
        "upper" => BuiltinFunction::Upper,
        "lower" => BuiltinFunction::Lower,
        "length" => BuiltinFunction::Length,
        "trim" => BuiltinFunction::Trim,
        "ltrim" => BuiltinFunction::Ltrim,
        "rtrim" => BuiltinFunction::Rtrim,
        "reverse" => BuiltinFunction::Reverse,
        "initcap" => BuiltinFunction::Initcap,
        "substring" => BuiltinFunction::Substring,
        "replace" => BuiltinFunction::Replace,
        "concat_ws" => BuiltinFunction::ConcatWs,
        "coalesce" => BuiltinFunction::Coalesce,
        "null_if" => BuiltinFunction::NullIf,
        "if_null" => BuiltinFunction::IfNull,
        "abs" => BuiltinFunction::Abs,
        "round" => BuiltinFunction::Round,
        "floor" => BuiltinFunction::Floor,
        "ceil" => BuiltinFunction::Ceil,
        "power" => BuiltinFunction::Power,
        "sqrt" => BuiltinFunction::Sqrt,
        "ln" => BuiltinFunction::Ln,
        "exp" => BuiltinFunction::Exp,
        "sign" => BuiltinFunction::Sign,
        "greatest" => BuiltinFunction::Greatest,
        "least" => BuiltinFunction::Least,
        "current_date" => BuiltinFunction::CurrentDate,
        "current_time" => BuiltinFunction::CurrentTime,
        "current_timestamp" => BuiltinFunction::CurrentTimestamp,
        "row_number" => BuiltinFunction::RowNumber,
        "rank" => BuiltinFunction::Rank,
        "dense_rank" => BuiltinFunction::DenseRank,
        _ => {
            return Err(PyValueError::new_err(format!(
                "unknown builder function: {value}"
            )))
        }
    })
}

pub(crate) fn node_from_any(value: &Bound<'_, PyAny>, parse_string: bool) -> PyResult<BuildNode> {
    if let Ok(expression) = value.extract::<PyRef<'_, PyExpression>>() {
        return Ok(BuildNode::Ast {
            expression: Box::new(expression.inner.clone()),
        });
    }
    if value.is_none() {
        return Ok(BuildNode::Literal {
            value: BuilderValue::Null,
        });
    }
    if value.is_instance_of::<PyBool>() {
        return Ok(BuildNode::Literal {
            value: BuilderValue::Bool(value.extract::<bool>()?),
        });
    }
    if value.is_instance_of::<PyString>() {
        let string = value.extract::<String>()?;
        return Ok(if parse_string {
            BuildNode::Sql { sql: string }
        } else {
            BuildNode::Literal {
                value: BuilderValue::String(string),
            }
        });
    }
    if let Ok(integer) = value.extract::<i64>() {
        return Ok(BuildNode::Literal {
            value: BuilderValue::Integer(integer),
        });
    }
    if let Ok(float) = value.extract::<f64>() {
        return Ok(BuildNode::Literal {
            value: BuilderValue::Float(float),
        });
    }
    Err(PyTypeError::new_err(format!(
        "unsupported builder expression input: {}",
        value.get_type().name()?
    )))
}

pub(crate) fn nodes_from_tuple(
    values: &Bound<'_, PyTuple>,
    parse_strings: bool,
) -> PyResult<Vec<BuildNode>> {
    values
        .iter()
        .map(|value| node_from_any(&value, parse_strings))
        .collect()
}

fn evaluate_and_wrap(
    py: Python<'_>,
    node: BuildNode,
    dialect: Option<&str>,
) -> PyResult<Py<PyAny>> {
    let expression =
        evaluate(&BuilderPlan::new(node), dialect_name(dialect)).map_err(map_parse_error)?;
    wrap_expression(py, expression)
}

pub(crate) fn apply_and_wrap(
    py: Python<'_>,
    expression: &PyExpression,
    operation: BuildOperation,
    dialect: Option<&str>,
    copy: bool,
) -> PyResult<Py<PyAny>> {
    ensure_copy(copy)?;
    evaluate_and_wrap(
        py,
        BuilderPlan::new(BuildNode::Ast {
            expression: Box::new(expression.inner.clone()),
        })
        .apply(operation)
        .into_node(),
        dialect,
    )
}

#[pyfunction(name = "select", signature = (*expressions, dialect = None, copy = true))]
fn py_select(
    py: Python<'_>,
    expressions: &Bound<'_, PyTuple>,
    dialect: Option<&str>,
    copy: bool,
) -> PyResult<Py<PyAny>> {
    ensure_copy(copy)?;
    evaluate_and_wrap(
        py,
        BuildNode::Select {
            expressions: nodes_from_tuple(expressions, true)?,
        },
        dialect,
    )
}

#[pyfunction(name = "from_", signature = (expression, dialect = None, copy = true))]
fn py_from(
    py: Python<'_>,
    expression: &Bound<'_, PyAny>,
    dialect: Option<&str>,
    copy: bool,
) -> PyResult<Py<PyAny>> {
    ensure_copy(copy)?;
    let base = BuildNode::Select {
        expressions: Vec::new(),
    };
    evaluate_and_wrap(
        py,
        BuilderPlan::new(base)
            .apply(BuildOperation::From {
                source: node_from_any(expression, true)?,
            })
            .into_node(),
        dialect,
    )
}

#[pyfunction(name = "column", signature = (name, table = None))]
fn py_column(py: Python<'_>, name: &str, table: Option<&str>) -> PyResult<Py<PyAny>> {
    let name = table
        .map(|table| format!("{table}.{name}"))
        .unwrap_or_else(|| name.to_string());
    evaluate_and_wrap(py, BuildNode::Column { name }, None)
}

#[pyfunction(name = "col", signature = (name, table = None))]
fn py_col(py: Python<'_>, name: &str, table: Option<&str>) -> PyResult<Py<PyAny>> {
    py_column(py, name, table)
}

#[pyfunction(name = "table_", signature = (name))]
fn py_table(py: Python<'_>, name: &str) -> PyResult<Py<PyAny>> {
    evaluate_and_wrap(
        py,
        BuildNode::Table {
            name: name.to_string(),
        },
        None,
    )
}

#[pyfunction(name = "convert", signature = (value, copy = false))]
fn py_convert(py: Python<'_>, value: &Bound<'_, PyAny>, copy: bool) -> PyResult<Py<PyAny>> {
    if copy {
        // Expressions are cloned by node_from_any; scalar values are immutable.
    }
    evaluate_and_wrap(py, node_from_any(value, false)?, None)
}

#[pyfunction(name = "lit", signature = (value))]
fn py_lit(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    py_convert(py, value, false)
}

#[pyfunction(name = "condition", signature = (expression, dialect = None, copy = true))]
fn py_condition(
    py: Python<'_>,
    expression: &Bound<'_, PyAny>,
    dialect: Option<&str>,
    copy: bool,
) -> PyResult<Py<PyAny>> {
    ensure_copy(copy)?;
    evaluate_and_wrap(py, node_from_any(expression, true)?, dialect)
}

fn combine(
    py: Python<'_>,
    expressions: &Bound<'_, PyTuple>,
    op: &str,
    dialect: Option<&str>,
    copy: bool,
) -> PyResult<Py<PyAny>> {
    ensure_copy(copy)?;
    let mut expressions = nodes_from_tuple(expressions, true)?.into_iter();
    let mut node = expressions
        .next()
        .ok_or_else(|| PyValueError::new_err("at least one expression is required"))?;
    for right in expressions {
        node = BuildNode::Binary {
            op: binary_operator(op)?,
            left: Box::new(node),
            right: Box::new(right),
        };
    }
    evaluate_and_wrap(py, node, dialect)
}

#[pyfunction(name = "and_", signature = (*expressions, dialect = None, copy = true, wrap = true))]
fn py_and(
    py: Python<'_>,
    expressions: &Bound<'_, PyTuple>,
    dialect: Option<&str>,
    copy: bool,
    wrap: bool,
) -> PyResult<Py<PyAny>> {
    let _ = wrap;
    combine(py, expressions, "and", dialect, copy)
}

#[pyfunction(name = "or_", signature = (*expressions, dialect = None, copy = true, wrap = true))]
fn py_or(
    py: Python<'_>,
    expressions: &Bound<'_, PyTuple>,
    dialect: Option<&str>,
    copy: bool,
    wrap: bool,
) -> PyResult<Py<PyAny>> {
    let _ = wrap;
    combine(py, expressions, "or", dialect, copy)
}

#[pyfunction(name = "not_", signature = (expression, dialect = None, copy = true))]
fn py_not(
    py: Python<'_>,
    expression: &Bound<'_, PyAny>,
    dialect: Option<&str>,
    copy: bool,
) -> PyResult<Py<PyAny>> {
    ensure_copy(copy)?;
    evaluate_and_wrap(
        py,
        BuildNode::Unary {
            op: UnaryOperator::Not,
            expression: Box::new(node_from_any(expression, true)?),
        },
        dialect,
    )
}

#[pyfunction(name = "alias_", signature = (expression, alias, dialect = None, copy = true))]
fn py_alias(
    py: Python<'_>,
    expression: &Bound<'_, PyAny>,
    alias: &str,
    dialect: Option<&str>,
    copy: bool,
) -> PyResult<Py<PyAny>> {
    ensure_copy(copy)?;
    evaluate_and_wrap(
        py,
        BuildNode::Alias {
            expression: Box::new(node_from_any(expression, true)?),
            alias: alias.to_string(),
        },
        dialect,
    )
}

#[pyfunction(name = "func", signature = (name, *args, dialect = None, copy = true))]
fn py_func(
    py: Python<'_>,
    name: &str,
    args: &Bound<'_, PyTuple>,
    dialect: Option<&str>,
    copy: bool,
) -> PyResult<Py<PyAny>> {
    ensure_copy(copy)?;
    evaluate_and_wrap(
        py,
        BuildNode::Function {
            name: name.to_string(),
            args: nodes_from_tuple(args, true)?,
        },
        dialect,
    )
}

#[pyfunction(name = "_builder_builtin", signature = (name, *args))]
fn py_builder_builtin(
    py: Python<'_>,
    name: &str,
    args: &Bound<'_, PyTuple>,
) -> PyResult<Py<PyAny>> {
    evaluate_and_wrap(
        py,
        BuildNode::Builtin {
            function: builtin_function(name)?,
            args: nodes_from_tuple(args, true)?,
        },
        None,
    )
}

#[pyfunction(name = "extract", signature = (field, expression))]
fn py_extract(py: Python<'_>, field: &str, expression: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    evaluate_and_wrap(
        py,
        BuildNode::Extract {
            field: field.to_string(),
            expression: Box::new(node_from_any(expression, true)?),
        },
        None,
    )
}

#[pyfunction(name = "case", signature = (expression = None, dialect = None, copy = true))]
fn py_case(
    py: Python<'_>,
    expression: Option<&Bound<'_, PyAny>>,
    dialect: Option<&str>,
    copy: bool,
) -> PyResult<Py<PyAny>> {
    ensure_copy(copy)?;
    evaluate_and_wrap(
        py,
        BuildNode::Case {
            operand: expression
                .map(|value| node_from_any(value, true).map(Box::new))
                .transpose()?,
        },
        dialect,
    )
}

fn assignments_from_dict(values: Option<&Bound<'_, PyDict>>) -> PyResult<Vec<BuilderAssignment>> {
    let Some(values) = values else {
        return Ok(Vec::new());
    };
    values
        .iter()
        .map(|(key, value)| {
            Ok(BuilderAssignment {
                column: key.extract::<String>()?,
                value: node_from_any(&value, false)?,
            })
        })
        .collect()
}

#[pyfunction(name = "update", signature = (table, properties = None, r#where = None, from_ = None, dialect = None, copy = true))]
fn py_update(
    py: Python<'_>,
    table: &str,
    properties: Option<&Bound<'_, PyDict>>,
    r#where: Option<&Bound<'_, PyAny>>,
    from_: Option<&str>,
    dialect: Option<&str>,
    copy: bool,
) -> PyResult<Py<PyAny>> {
    ensure_copy(copy)?;
    evaluate_and_wrap(
        py,
        BuildNode::Update {
            table: table.to_string(),
            assignments: assignments_from_dict(properties)?,
            where_clause: r#where
                .map(|value| node_from_any(value, true).map(Box::new))
                .transpose()?,
            from: from_.map(str::to_string),
        },
        dialect,
    )
}

#[pyfunction(name = "delete", signature = (table, r#where = None, dialect = None, copy = true))]
fn py_delete(
    py: Python<'_>,
    table: &str,
    r#where: Option<&Bound<'_, PyAny>>,
    dialect: Option<&str>,
    copy: bool,
) -> PyResult<Py<PyAny>> {
    ensure_copy(copy)?;
    evaluate_and_wrap(
        py,
        BuildNode::Delete {
            table: table.to_string(),
            where_clause: r#where
                .map(|value| node_from_any(value, true).map(Box::new))
                .transpose()?,
        },
        dialect,
    )
}

#[pyfunction(name = "insert", signature = (expression, into, columns = None, dialect = None, copy = true))]
fn py_insert(
    py: Python<'_>,
    expression: &Bound<'_, PyAny>,
    into: &str,
    columns: Option<Vec<String>>,
    dialect: Option<&str>,
    copy: bool,
) -> PyResult<Py<PyAny>> {
    ensure_copy(copy)?;
    evaluate_and_wrap(
        py,
        BuildNode::Insert {
            into: into.to_string(),
            expression: Some(Box::new(node_from_any(expression, true)?)),
            columns: columns.unwrap_or_default(),
        },
        dialect,
    )
}

#[pyfunction(name = "merge_into", signature = (target, dialect = None, copy = true))]
fn py_merge_into(
    py: Python<'_>,
    target: &str,
    dialect: Option<&str>,
    copy: bool,
) -> PyResult<Py<PyAny>> {
    ensure_copy(copy)?;
    evaluate_and_wrap(
        py,
        BuildNode::Merge {
            target: target.to_string(),
        },
        dialect,
    )
}

fn set_operation(
    py: Python<'_>,
    left: &Bound<'_, PyAny>,
    right: &Bound<'_, PyAny>,
    kind: &str,
    distinct: bool,
    dialect: Option<&str>,
    copy: bool,
) -> PyResult<Py<PyAny>> {
    ensure_copy(copy)?;
    evaluate_and_wrap(
        py,
        BuilderPlan::new(node_from_any(left, true)?)
            .apply(match kind {
                "union" => BuildOperation::Union {
                    other: node_from_any(right, true)?,
                    distinct,
                },
                "intersect" => BuildOperation::Intersect {
                    other: node_from_any(right, true)?,
                    distinct,
                },
                _ => BuildOperation::Except {
                    other: node_from_any(right, true)?,
                    distinct,
                },
            })
            .into_node(),
        dialect,
    )
}

#[pyfunction(name = "union", signature = (left, right, distinct = true, dialect = None, copy = true))]
fn py_union(
    py: Python<'_>,
    left: &Bound<'_, PyAny>,
    right: &Bound<'_, PyAny>,
    distinct: bool,
    dialect: Option<&str>,
    copy: bool,
) -> PyResult<Py<PyAny>> {
    set_operation(py, left, right, "union", distinct, dialect, copy)
}

#[pyfunction(name = "intersect", signature = (left, right, distinct = true, dialect = None, copy = true))]
fn py_intersect(
    py: Python<'_>,
    left: &Bound<'_, PyAny>,
    right: &Bound<'_, PyAny>,
    distinct: bool,
    dialect: Option<&str>,
    copy: bool,
) -> PyResult<Py<PyAny>> {
    set_operation(py, left, right, "intersect", distinct, dialect, copy)
}

#[pyfunction(name = "except_", signature = (left, right, distinct = true, dialect = None, copy = true))]
fn py_except(
    py: Python<'_>,
    left: &Bound<'_, PyAny>,
    right: &Bound<'_, PyAny>,
    distinct: bool,
    dialect: Option<&str>,
    copy: bool,
) -> PyResult<Py<PyAny>> {
    set_operation(py, left, right, "except", distinct, dialect, copy)
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_select, m)?)?;
    m.add_function(wrap_pyfunction!(py_from, m)?)?;
    m.add_function(wrap_pyfunction!(py_column, m)?)?;
    m.add_function(wrap_pyfunction!(py_col, m)?)?;
    m.add_function(wrap_pyfunction!(py_table, m)?)?;
    m.add_function(wrap_pyfunction!(py_convert, m)?)?;
    m.add_function(wrap_pyfunction!(py_lit, m)?)?;
    m.add_function(wrap_pyfunction!(py_condition, m)?)?;
    m.add_function(wrap_pyfunction!(py_and, m)?)?;
    m.add_function(wrap_pyfunction!(py_or, m)?)?;
    m.add_function(wrap_pyfunction!(py_not, m)?)?;
    m.add_function(wrap_pyfunction!(py_alias, m)?)?;
    m.add_function(wrap_pyfunction!(py_func, m)?)?;
    m.add_function(wrap_pyfunction!(py_builder_builtin, m)?)?;
    m.add_function(wrap_pyfunction!(py_extract, m)?)?;
    m.add_function(wrap_pyfunction!(py_case, m)?)?;
    m.add_function(wrap_pyfunction!(py_update, m)?)?;
    m.add_function(wrap_pyfunction!(py_delete, m)?)?;
    m.add_function(wrap_pyfunction!(py_insert, m)?)?;
    m.add_function(wrap_pyfunction!(py_merge_into, m)?)?;
    m.add_function(wrap_pyfunction!(py_union, m)?)?;
    m.add_function(wrap_pyfunction!(py_intersect, m)?)?;
    m.add_function(wrap_pyfunction!(py_except, m)?)?;
    Ok(())
}
