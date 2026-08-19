import pytest

import polyglot_sql


def test_select_builder_matches_sqlglot_common_surface():
    query = (
        polyglot_sql.select("customer_id", "COUNT(*) AS orders")
        .from_("orders")
        .where("status = 'complete'")
        .group_by("customer_id")
        .order_by("orders DESC")
        .limit(10)
    )

    assert isinstance(query, polyglot_sql.Select)
    assert query.sql() == (
        "SELECT customer_id, COUNT(*) AS orders FROM orders "
        "WHERE status = 'complete' GROUP BY customer_id ORDER BY orders DESC LIMIT 10"
    )


def test_builder_is_immutable_and_appends_conditions():
    base = polyglot_sql.select("x").from_("tbl").where("x > 0")
    extended = base.where("x < 9")

    assert base.sql() == "SELECT x FROM tbl WHERE x > 0"
    assert extended.sql() == "SELECT x FROM tbl WHERE x > 0 AND x < 9"
    assert base.where("x < 9", append=False).sql() == "SELECT x FROM tbl WHERE x < 9"


def test_explicit_columns_and_scalar_strings_have_sqlglot_coercion():
    condition = polyglot_sql.column("status").eq("active")

    assert isinstance(condition, polyglot_sql.Eq)
    assert condition.sql() == "status = 'active'"
    assert polyglot_sql.func("COALESCE", "status", 1).sql() == "COALESCE(status, 1)"


def test_expression_operators_case_and_set_operations():
    expression = (polyglot_sql.column("price") * 2).as_("total")
    case = polyglot_sql.case().when("x = 1", "x").else_("fallback")
    union = polyglot_sql.select("x").from_("a").union(
        polyglot_sql.select("x").from_("b"), distinct=False
    )

    assert expression.sql() == "price * 2 AS total"
    assert polyglot_sql.column("a").xor("b").sql() == "a XOR b"
    assert (polyglot_sql.column("a") ^ polyglot_sql.column("b")).sql() == "a XOR b"
    assert case.sql() == "CASE WHEN x = 1 THEN x ELSE fallback END"
    assert union.sql() == "SELECT x FROM a UNION ALL SELECT x FROM b"
    assert polyglot_sql.union("SELECT x FROM a", "SELECT x FROM b").sql() == (
        "SELECT x FROM a UNION SELECT x FROM b"
    )


def test_basic_dml_builders():
    update = polyglot_sql.update("users", {"name": "Bob"}, where="id = 1")
    delete = polyglot_sql.delete("users", where="id = 1")
    insert = polyglot_sql.insert("SELECT id, name FROM staging", "users", ["id", "name"])

    assert update.sql() == "UPDATE users SET name = 'Bob' WHERE id = 1"
    assert delete.sql() == "DELETE FROM users WHERE id = 1"
    assert insert.sql() == "INSERT INTO users (id, name) SELECT id, name FROM staging"


def test_copy_false_is_rejected_without_mutation():
    query = polyglot_sql.select("x")

    with pytest.raises(NotImplementedError, match="immutable"):
        query.from_("tbl", copy=False)

    assert query.sql() == "SELECT x"


def test_builder_parse_errors_use_parse_error():
    with pytest.raises(polyglot_sql.ParseError):
        polyglot_sql.select("(")


def test_named_helpers_and_advanced_query_clauses_share_the_builder_engine():
    query = (
        polyglot_sql.select(
            "department",
            polyglot_sql.count(polyglot_sql.column("id")).as_("employees"),
        )
        .from_("employees")
        .join(
            "departments",
            on="employees.department_id = departments.id",
            join_type="full",
        )
        .window(
            "w",
            partition_by=("department",),
            order_by=("salary DESC",),
        )
        .for_share()
        .hint("FULL(employees)")
        .ctas("department_summary")
    )

    sql = query.sql()
    assert "CREATE TABLE department_summary AS SELECT" in sql
    assert "FULL JOIN departments" in sql
    assert "WINDOW w AS" in sql
    assert "FOR SHARE" in sql


def test_insert_rows_and_conditional_merge_actions():
    insert = polyglot_sql.insert("VALUES (1, 'Ada')", "users", ["id", "name"])
    insert = insert.values(2, "Grace")
    assert insert.sql() == "INSERT INTO users (id, name) VALUES (1, 'Ada'), (2, 'Grace')"

    merge = (
        polyglot_sql.merge_into("target")
        .merge_using("source", "target.id = source.id")
        .when_matched_update(
            {"name": polyglot_sql.column("source.name")},
            condition="source.active",
        )
        .when_matched_delete(condition="source.deleted")
        .when_not_matched_insert(
            ["id", "name"],
            (polyglot_sql.column("source.id"), polyglot_sql.column("source.name")),
            condition="source.active",
        )
    )
    sql = merge.sql()
    assert "WHEN MATCHED AND source.active THEN UPDATE SET name = source.name" in sql
    assert "WHEN MATCHED AND source.deleted THEN DELETE" in sql
    assert "WHEN NOT MATCHED AND source.active THEN INSERT" in sql
