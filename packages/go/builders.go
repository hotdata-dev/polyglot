package polyglot

import (
	"encoding/json"
	"fmt"
	"sort"
	"strings"

	"github.com/tobilg/polyglot/packages/go/internal/ffi"
)

// Expression is an immutable SQL builder plan. It is evaluated by the shared
// Rust builder engine only when Build or BuildSQL is called.
type Expression struct {
	base        map[string]any
	operations  []map[string]any
	readDialect string
	err         error
}

type ClauseOptions struct {
	Append bool
}

var AppendClauses = ClauseOptions{Append: true}

type LateralViewOptions struct {
	Outer bool
}

type CTASOptions struct {
	Replace   bool
	Temporary bool
}

func SQLExpr(sql string) Expression { return expression(map[string]any{"kind": "sql", "sql": sql}) }
func Column(name string) Expression {
	return expression(map[string]any{"kind": "column", "name": name})
}
func Table(name string) Expression    { return expression(map[string]any{"kind": "table", "name": name}) }
func Star() Expression                { return expression(map[string]any{"kind": "star"}) }
func Lit(value any) Expression        { return fromValue(value, false) }
func Condition(sql string) Expression { return SQLExpr(sql) }

func Func(name string, args ...any) Expression {
	nodes, err := nodes(args, true)
	return withError(map[string]any{"kind": "function", "name": name, "args": nodes}, err)
}

func builtin(name string, args ...any) Expression {
	nodes, err := nodes(args, true)
	return withError(map[string]any{"kind": "builtin", "function": name, "args": nodes}, err)
}

func Count(value any) Expression          { return builtin("count", value) }
func CountStar() Expression               { return builtin("count_star") }
func CountDistinct(value any) Expression  { return builtin("count_distinct", value) }
func Sum(value any) Expression            { return builtin("sum", value) }
func Avg(value any) Expression            { return builtin("avg", value) }
func Min(value any) Expression            { return builtin("min", value) }
func Max(value any) Expression            { return builtin("max", value) }
func ApproxDistinct(value any) Expression { return builtin("approx_distinct", value) }
func Upper(value any) Expression          { return builtin("upper", value) }
func Lower(value any) Expression          { return builtin("lower", value) }
func Length(value any) Expression         { return builtin("length", value) }
func Trim(value any) Expression           { return builtin("trim", value) }
func LTrim(value any) Expression          { return builtin("ltrim", value) }
func RTrim(value any) Expression          { return builtin("rtrim", value) }
func Reverse(value any) Expression        { return builtin("reverse", value) }
func InitCap(value any) Expression        { return builtin("initcap", value) }
func Substring(value, start any, length ...any) Expression {
	args := []any{value, start}
	args = append(args, length...)
	return builtin("substring", args...)
}
func Replace(value, old, replacement any) Expression {
	return builtin("replace", value, old, replacement)
}
func ConcatWS(separator any, values ...any) Expression {
	return builtin("concat_ws", append([]any{separator}, values...)...)
}
func Coalesce(values ...any) Expression     { return builtin("coalesce", values...) }
func NullIf(left, right any) Expression     { return builtin("null_if", left, right) }
func IfNull(value, fallback any) Expression { return builtin("if_null", value, fallback) }
func Abs(value any) Expression              { return builtin("abs", value) }
func Round(value any, decimals ...any) Expression {
	return builtin("round", append([]any{value}, decimals...)...)
}
func Floor(value any) Expression          { return builtin("floor", value) }
func Ceil(value any) Expression           { return builtin("ceil", value) }
func Power(base, exponent any) Expression { return builtin("power", base, exponent) }
func Sqrt(value any) Expression           { return builtin("sqrt", value) }
func Ln(value any) Expression             { return builtin("ln", value) }
func Exp(value any) Expression            { return builtin("exp", value) }
func Sign(value any) Expression           { return builtin("sign", value) }
func Greatest(values ...any) Expression   { return builtin("greatest", values...) }
func Least(values ...any) Expression      { return builtin("least", values...) }
func CurrentDate() Expression             { return builtin("current_date") }
func CurrentTime() Expression             { return builtin("current_time") }
func CurrentTimestamp() Expression        { return builtin("current_timestamp") }
func Extract(field string, value any) Expression {
	node, err := nodeFor(value, true)
	return withError(map[string]any{"kind": "extract", "field": field, "expression": node}, err)
}
func RowNumber() Expression { return builtin("row_number") }
func Rank() Expression      { return builtin("rank") }
func DenseRank() Expression { return builtin("dense_rank") }

func Select(expressions ...any) Expression {
	nodes, err := nodes(expressions, true)
	return withError(map[string]any{"kind": "select", "expressions": nodes}, err)
}

func From(source any) Expression { return Select().From(source) }

func Case(operand ...any) Expression {
	var value any
	var err error
	if len(operand) > 1 {
		err = fmt.Errorf("polyglot builder: Case accepts at most one operand")
	} else if len(operand) == 1 {
		value, err = nodeFor(operand[0], true)
	}
	return withError(map[string]any{"kind": "case", "operand": value}, err)
}

func Update(table string, assignments map[string]any) Expression {
	values, err := assignmentNodes(assignments)
	return withError(map[string]any{"kind": "update", "table": table, "assignments": values}, err)
}

func Delete(table string) Expression {
	return expression(map[string]any{"kind": "delete", "table": table})
}

func Insert(source any, into string, columns ...string) Expression {
	node, err := nodeFor(source, true)
	return withError(map[string]any{"kind": "insert", "into": into, "expression": node, "columns": columns}, err)
}
func InsertInto(into string) Expression {
	return expression(map[string]any{"kind": "insert", "into": into, "expression": nil, "columns": []string{}})
}

func MergeInto(target string) Expression {
	return expression(map[string]any{"kind": "merge", "target": target})
}

func And(expressions ...any) Expression { return combine("and", expressions) }
func Or(expressions ...any) Expression  { return combine("or", expressions) }
func Not(value any) Expression          { return unary("not", value, true) }
func Alias(value any, alias string) Expression {
	node, err := nodeFor(value, true)
	return withError(map[string]any{"kind": "alias", "expression": node, "alias": alias}, err)
}

func (e Expression) ReadDialect(dialect string) Expression {
	e.readDialect = dialect
	return e
}

func (e Expression) Eq(other any) Expression    { return e.binary("eq", other, false) }
func (e Expression) Neq(other any) Expression   { return e.binary("neq", other, false) }
func (e Expression) LT(other any) Expression    { return e.binary("lt", other, false) }
func (e Expression) LTE(other any) Expression   { return e.binary("lte", other, false) }
func (e Expression) GT(other any) Expression    { return e.binary("gt", other, false) }
func (e Expression) GTE(other any) Expression   { return e.binary("gte", other, false) }
func (e Expression) Add(other any) Expression   { return e.binary("add", other, false) }
func (e Expression) Sub(other any) Expression   { return e.binary("sub", other, false) }
func (e Expression) Mul(other any) Expression   { return e.binary("mul", other, false) }
func (e Expression) Div(other any) Expression   { return e.binary("div", other, false) }
func (e Expression) Mod(other any) Expression   { return e.binary("mod", other, false) }
func (e Expression) Is(other any) Expression    { return e.binary("is", other, false) }
func (e Expression) And(other any) Expression   { return e.binary("and", other, true) }
func (e Expression) Or(other any) Expression    { return e.binary("or", other, true) }
func (e Expression) Xor(other any) Expression   { return e.binary("xor", other, true) }
func (e Expression) Like(other any) Expression  { return e.binary("like", other, false) }
func (e Expression) ILike(other any) Expression { return e.binary("ilike", other, false) }
func (e Expression) RLike(other any) Expression { return e.binary("rlike", other, false) }
func (e Expression) Not() Expression            { return e.unary("not") }
func (e Expression) Neg() Expression            { return e.unary("neg") }
func (e Expression) IsNull() Expression         { return e.unary("is_null") }
func (e Expression) IsNotNull() Expression      { return e.unary("is_not_null") }

func (e Expression) As(alias string) Expression {
	return e.wrap(map[string]any{"kind": "alias", "alias": alias}, "expression")
}

func (e Expression) Cast(dataType string) Expression {
	return e.wrap(map[string]any{"kind": "cast", "to": dataType}, "expression")
}

func (e Expression) Asc() Expression {
	return e.wrap(map[string]any{"kind": "ordered", "desc": false}, "expression")
}
func (e Expression) Desc() Expression {
	return e.wrap(map[string]any{"kind": "ordered", "desc": true}, "expression")
}

func (e Expression) Between(low, high any) Expression {
	lowNode, lowErr := nodeFor(low, false)
	highNode, highErr := nodeFor(high, false)
	return e.derived(map[string]any{"kind": "between", "expression": e.planNode(), "low": lowNode, "high": highNode}, firstError(e.err, lowErr, highErr))
}

func (e Expression) In(values ...any) Expression    { return e.in(false, values) }
func (e Expression) NotIn(values ...any) Expression { return e.in(true, values) }

func (e Expression) Select(values ...any) Expression {
	return e.listOperation("select", values, true, true)
}
func (e Expression) From(source any) Expression {
	node, err := nodeFor(source, true)
	return e.apply(map[string]any{"kind": "from", "source": node}, err)
}
func (e Expression) Join(source any, on any, joinType ...string) Expression {
	sourceNode, sourceErr := nodeFor(source, true)
	var onNode any
	var onErr error
	if on != nil {
		onNode, onErr = nodeFor(on, true)
	}
	kind := ""
	if len(joinType) > 0 {
		kind = joinType[0]
	}
	kind, kindErr := normalizeJoinType(kind)
	return e.apply(map[string]any{"kind": "join", "source": sourceNode, "on": onNode, "join_type": kind}, firstError(sourceErr, onErr, kindErr))
}
func (e Expression) LeftJoin(source, on any) Expression  { return e.Join(source, on, "left") }
func (e Expression) RightJoin(source, on any) Expression { return e.Join(source, on, "right") }
func (e Expression) FullJoin(source, on any) Expression  { return e.Join(source, on, "full") }
func (e Expression) CrossJoin(source any) Expression     { return e.Join(source, nil, "cross") }
func (e Expression) Where(values ...any) Expression {
	return e.listOperation("where", values, true, true)
}
func (e Expression) WhereWithOptions(options ClauseOptions, values ...any) Expression {
	return e.listOperation("where", values, options.Append, true)
}
func (e Expression) GroupBy(values ...any) Expression {
	return e.listOperation("group_by", values, true, true)
}
func (e Expression) GroupByWithOptions(options ClauseOptions, values ...any) Expression {
	return e.listOperation("group_by", values, options.Append, true)
}
func (e Expression) Having(values ...any) Expression {
	return e.listOperation("having", values, true, true)
}
func (e Expression) HavingWithOptions(options ClauseOptions, values ...any) Expression {
	return e.listOperation("having", values, options.Append, true)
}
func (e Expression) OrderBy(values ...any) Expression {
	return e.listOperation("order_by", values, true, true)
}
func (e Expression) OrderByWithOptions(options ClauseOptions, values ...any) Expression {
	return e.listOperation("order_by", values, options.Append, true)
}
func (e Expression) SortBy(values ...any) Expression {
	return e.listOperation("sort_by", values, true, true)
}
func (e Expression) SortByWithOptions(options ClauseOptions, values ...any) Expression {
	return e.listOperation("sort_by", values, options.Append, true)
}
func (e Expression) Qualify(values ...any) Expression {
	return e.listOperation("qualify", values, true, true)
}
func (e Expression) QualifyWithOptions(options ClauseOptions, values ...any) Expression {
	return e.listOperation("qualify", values, options.Append, true)
}
func (e Expression) SelectWithOptions(options ClauseOptions, values ...any) Expression {
	return e.listOperation("select", values, options.Append, true)
}
func (e Expression) LateralView(value any, tableAlias string, columnAliases ...string) Expression {
	return e.LateralViewWithOptions(LateralViewOptions{}, value, tableAlias, columnAliases...)
}
func (e Expression) LateralViewWithOptions(options LateralViewOptions, value any, tableAlias string, columnAliases ...string) Expression {
	node, err := nodeFor(value, true)
	return e.apply(map[string]any{"kind": "lateral_view", "expression": node, "table_alias": tableAlias, "column_aliases": columnAliases, "outer": options.Outer}, err)
}
func (e Expression) Window(name string, partitionBy, orderBy []any) Expression {
	partitions, partitionErr := nodes(partitionBy, true)
	orders, orderErr := nodes(orderBy, true)
	return e.apply(map[string]any{"kind": "window", "name": name, "partition_by": partitions, "order_by": orders}, firstError(partitionErr, orderErr))
}
func (e Expression) ForUpdate() Expression {
	return e.apply(map[string]any{"kind": "lock", "lock_type": "update"}, nil)
}
func (e Expression) ForShare() Expression {
	return e.apply(map[string]any{"kind": "lock", "lock_type": "share"}, nil)
}
func (e Expression) Hint(text string) Expression {
	return e.apply(map[string]any{"kind": "hint", "text": text}, nil)
}
func (e Expression) CTAS(table string) Expression {
	return e.CTASWithOptions(CTASOptions{}, table)
}
func (e Expression) CTASWithOptions(options CTASOptions, table string) Expression {
	return e.apply(map[string]any{"kind": "ctas", "table": table, "replace": options.Replace, "temporary": options.Temporary}, nil)
}
func (e Expression) Limit(value any) Expression {
	return e.singleOperation("limit", "expression", value, false)
}
func (e Expression) Offset(value any) Expression {
	return e.singleOperation("offset", "expression", value, false)
}
func (e Expression) Distinct(enabled ...bool) Expression {
	value := true
	if len(enabled) > 0 {
		value = enabled[0]
	}
	return e.apply(map[string]any{"kind": "distinct", "enabled": value}, nil)
}
func (e Expression) Subquery(alias ...string) Expression {
	var name any
	if len(alias) > 0 && alias[0] != "" {
		name = alias[0]
	}
	return e.apply(map[string]any{"kind": "subquery", "alias": name}, nil)
}
func (e Expression) Union(other Expression, distinct ...bool) Expression {
	return e.setOperation("union", other, distinct)
}
func (e Expression) Intersect(other Expression, distinct ...bool) Expression {
	return e.setOperation("intersect", other, distinct)
}
func (e Expression) Except(other Expression, distinct ...bool) Expression {
	return e.setOperation("except", other, distinct)
}
func (e Expression) When(condition, result any) Expression {
	conditionNode, conditionErr := nodeFor(condition, true)
	resultNode, resultErr := nodeFor(result, true)
	return e.apply(map[string]any{"kind": "when", "condition": conditionNode, "result": resultNode}, firstError(conditionErr, resultErr))
}
func (e Expression) Else(result any) Expression {
	return e.singleOperation("else", "result", result, true)
}
func (e Expression) Set(assignments map[string]any) Expression {
	values, err := assignmentNodes(assignments)
	return e.apply(map[string]any{"kind": "set", "assignments": values}, err)
}
func (e Expression) InsertColumns(columns ...string) Expression {
	return e.apply(map[string]any{"kind": "insert_columns", "columns": columns}, nil)
}
func (e Expression) Values(rows ...[]any) Expression {
	result := make([][]map[string]any, 0, len(rows))
	for _, row := range rows {
		values, err := nodes(row, false)
		if err != nil {
			return e.derived(e.planNode(), firstError(e.err, err))
		}
		result = append(result, values)
	}
	return e.apply(map[string]any{"kind": "values", "rows": result, "append": true}, nil)
}
func (e Expression) Query(query Expression) Expression {
	return e.apply(map[string]any{"kind": "query", "query": query.planNode()}, query.err)
}
func (e Expression) MergeUsing(source, on any) Expression {
	sourceNode, sourceErr := nodeFor(source, true)
	onNode, onErr := nodeFor(on, true)
	return e.apply(map[string]any{"kind": "merge_using", "source": sourceNode, "on": onNode}, firstError(sourceErr, onErr))
}
func (e Expression) WhenMatchedUpdate(assignments map[string]any, condition ...any) Expression {
	values, assignmentErr := assignmentNodes(assignments)
	conditionNode, conditionErr := optionalNode(condition, true)
	return e.apply(map[string]any{"kind": "when_matched_update", "assignments": values, "condition": conditionNode}, firstError(assignmentErr, conditionErr))
}
func (e Expression) WhenMatchedDelete(condition ...any) Expression {
	conditionNode, err := optionalNode(condition, true)
	return e.apply(map[string]any{"kind": "when_matched_delete", "condition": conditionNode}, err)
}
func (e Expression) WhenNotMatchedInsert(columns []string, values []any, condition ...any) Expression {
	valueNodes, valueErr := nodes(values, false)
	conditionNode, conditionErr := optionalNode(condition, true)
	return e.apply(map[string]any{"kind": "when_not_matched_insert", "columns": columns, "values": valueNodes, "condition": conditionNode}, firstError(valueErr, conditionErr))
}

func (c *Client) Build(expression Expression) (json.RawMessage, error) {
	request, err := expression.request(map[string]any{"kind": "ast"})
	if err != nil {
		return nil, err
	}
	return c.callRaw("build", func(lib *ffi.Library) ffi.Result { return lib.Build(request) })
}

func (c *Client) BuildSQL(expression Expression, dialect string) (string, error) {
	request, err := expression.request(map[string]any{"kind": "sql", "dialect": defaultDialect(dialect)})
	if err != nil {
		return "", err
	}
	if err := rejectNUL(request); err != nil {
		return "", err
	}
	return c.callPayload("build", func(lib *ffi.Library) ffi.Result { return lib.Build(request) })
}

func expression(node map[string]any) Expression {
	return Expression{base: node, readDialect: "generic"}
}
func withError(node map[string]any, err error) Expression {
	value := expression(node)
	value.err = err
	return value
}

func fromValue(value any, parseString bool) Expression {
	node, err := nodeFor(value, parseString)
	return withError(node, err)
}

func nodeFor(value any, parseString bool) (map[string]any, error) {
	if expression, ok := value.(Expression); ok {
		return expression.planNode(), expression.err
	}
	if parseString {
		if sql, ok := value.(string); ok {
			return map[string]any{"kind": "sql", "sql": sql}, nil
		}
	}
	switch value := value.(type) {
	case nil:
		return map[string]any{"kind": "literal", "value": map[string]any{"kind": "null"}}, nil
	case string:
		return map[string]any{"kind": "literal", "value": map[string]any{"kind": "string", "value": value}}, nil
	case bool:
		return map[string]any{"kind": "literal", "value": map[string]any{"kind": "bool", "value": value}}, nil
	case int:
		return integerNode(int64(value)), nil
	case int8:
		return integerNode(int64(value)), nil
	case int16:
		return integerNode(int64(value)), nil
	case int32:
		return integerNode(int64(value)), nil
	case int64:
		return integerNode(value), nil
	case uint:
		if uint64(value) > uint64(^uint64(0)>>1) {
			return nil, fmt.Errorf("polyglot builder: integer is out of range")
		}
		return integerNode(int64(value)), nil
	case float32:
		return floatNode(float64(value)), nil
	case float64:
		return floatNode(value), nil
	default:
		return nil, fmt.Errorf("polyglot builder: unsupported expression input %T", value)
	}
}

func integerNode(value int64) map[string]any {
	return map[string]any{"kind": "literal", "value": map[string]any{"kind": "integer", "value": value}}
}
func floatNode(value float64) map[string]any {
	return map[string]any{"kind": "literal", "value": map[string]any{"kind": "float", "value": value}}
}

func nodes(values []any, parseStrings bool) ([]map[string]any, error) {
	result := make([]map[string]any, 0, len(values))
	for _, value := range values {
		node, err := nodeFor(value, parseStrings)
		if err != nil {
			return nil, err
		}
		result = append(result, node)
	}
	return result, nil
}

func optionalNode(values []any, parseString bool) (any, error) {
	if len(values) == 0 {
		return nil, nil
	}
	if len(values) > 1 {
		return nil, fmt.Errorf("polyglot builder: expected at most one optional expression")
	}
	return nodeFor(values[0], parseString)
}

func assignmentNodes(assignments map[string]any) ([]map[string]any, error) {
	keys := make([]string, 0, len(assignments))
	for key := range assignments {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	result := make([]map[string]any, 0, len(keys))
	for _, key := range keys {
		node, err := nodeFor(assignments[key], false)
		if err != nil {
			return nil, err
		}
		result = append(result, map[string]any{"column": key, "value": node})
	}
	return result, nil
}

func combine(op string, values []any) Expression {
	if len(values) == 0 {
		return withError(nil, fmt.Errorf("polyglot builder: %s requires at least one expression", strings.ToUpper(op)))
	}
	result := fromValue(values[0], true)
	for _, value := range values[1:] {
		result = result.binary(op, value, true)
	}
	return result
}

func unary(op string, value any, parseString bool) Expression {
	return fromValue(value, parseString).unary(op)
}

func (e Expression) binary(op string, other any, parseString bool) Expression {
	right, err := nodeFor(other, parseString)
	return e.derived(map[string]any{"kind": "binary", "op": op, "left": e.planNode(), "right": right}, firstError(e.err, err))
}
func (e Expression) unary(op string) Expression {
	return e.derived(map[string]any{"kind": "unary", "op": op, "expression": e.planNode()}, e.err)
}
func (e Expression) wrap(node map[string]any, field string) Expression {
	node[field] = e.planNode()
	return e.derived(node, e.err)
}
func (e Expression) derived(node map[string]any, err error) Expression {
	return Expression{base: node, readDialect: e.readDialect, err: err}
}
func (e Expression) apply(operation map[string]any, err error) Expression {
	operations := append([]map[string]any(nil), e.operations...)
	operations = append(operations, operation)
	return Expression{base: e.base, operations: operations, readDialect: e.readDialect, err: firstError(e.err, err)}
}
func (e Expression) listOperation(kind string, values []any, append, parseStrings bool) Expression {
	nodes, err := nodes(values, parseStrings)
	return e.apply(map[string]any{"kind": kind, "expressions": nodes, "append": append}, err)
}
func (e Expression) singleOperation(kind, field string, value any, parseString bool) Expression {
	node, err := nodeFor(value, parseString)
	op := map[string]any{"kind": kind, field: node}
	return e.apply(op, err)
}
func (e Expression) setOperation(kind string, other Expression, distinct []bool) Expression {
	value := true
	if len(distinct) > 0 {
		value = distinct[0]
	}
	return e.apply(map[string]any{"kind": kind, "other": other.planNode(), "distinct": value}, other.err)
}
func (e Expression) in(negated bool, values []any) Expression {
	nodes, err := nodes(values, false)
	return e.derived(map[string]any{"kind": "in_list", "expression": e.planNode(), "values": nodes, "negated": negated}, firstError(e.err, err))
}
func (e Expression) planNode() map[string]any {
	if len(e.operations) == 0 {
		return e.base
	}
	return map[string]any{
		"kind": "plan",
		"plan": map[string]any{
			"base":       e.base,
			"operations": e.operations,
		},
	}
}
func (e Expression) request(output map[string]any) (string, error) {
	if e.err != nil {
		return "", e.err
	}
	if e.base == nil {
		return "", fmt.Errorf("polyglot builder: expression is empty")
	}
	request := map[string]any{"version": 1, "read_dialect": defaultDialect(e.readDialect), "plan": map[string]any{"base": normalizePlanValue(e.base), "operations": normalizePlanValue(e.operations)}, "output": output}
	data, err := json.Marshal(request)
	if err != nil {
		return "", fmt.Errorf("polyglot builder: encode request: %w", err)
	}
	return string(data), nil
}

func normalizePlanValue(value any) any {
	switch value := value.(type) {
	case map[string]any:
		result := make(map[string]any, len(value))
		for key, child := range value {
			result[key] = normalizePlanValue(child)
		}
		return result
	case []map[string]any:
		result := make([]any, len(value))
		for index, child := range value {
			result[index] = normalizePlanValue(child)
		}
		return result
	case []any:
		result := make([]any, len(value))
		for index, child := range value {
			result[index] = normalizePlanValue(child)
		}
		return result
	default:
		return value
	}
}

func normalizeJoinType(value string) (string, error) {
	switch strings.ToLower(strings.TrimSpace(value)) {
	case "", "join", "inner", "inner join":
		return "inner", nil
	case "left", "left join", "left outer", "left outer join":
		return "left", nil
	case "right", "right join", "right outer", "right outer join":
		return "right", nil
	case "full", "full join", "full outer", "full outer join":
		return "full", nil
	case "cross", "cross join":
		return "cross", nil
	default:
		return "", fmt.Errorf("polyglot builder: unsupported join type %q", value)
	}
}
func firstError(errors ...error) error {
	for _, err := range errors {
		if err != nil {
			return err
		}
	}
	return nil
}
