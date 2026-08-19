/**
 * Immutable compatibility builder API.
 *
 * Plans are evaluated by the same Rust builder engine used by Python and Go.
 * The mutable builders in `builders.ts` are adapters over this API.
 */
import { wasm_compat_build } from '../wasm/polyglot_sql_wasm.js';
import type { BinaryOperator } from './generated/BinaryOperator.js';
import type { BuilderAssignment } from './generated/BuilderAssignment.js';
import type { BuilderOutput } from './generated/BuilderOutput.js';
import type { BuilderPlan } from './generated/BuilderPlan.js';
import type { BuildNode as PlanNode } from './generated/BuildNode.js';
import type { BuildOperation as PlanOperation } from './generated/BuildOperation.js';
import type { BuildRequest } from './generated/BuildRequest.js';
import type { BuiltinFunction } from './generated/BuiltinFunction.js';
import type { JoinType as ProtocolJoinType } from './generated/JoinType.js';
import type { UnaryOperator } from './generated/UnaryOperator.js';

export type Input = Expression | string | number | boolean | null;

export interface ListOptions {
  append?: boolean;
}

export type JoinType = ProtocolJoinType;

export class Expression {
  readonly #plan: BuilderPlan;
  readonly #readDialect: string;

  private constructor(plan: BuilderPlan, readDialect = 'generic') {
    this.#plan = plan;
    this.#readDialect = readDialect;
  }

  static fromNode(node: PlanNode, readDialect = 'generic'): Expression {
    return new Expression({ base: node, operations: [] }, readDialect);
  }

  /** @internal */
  _planNode(): PlanNode {
    return this.#plan.operations.length === 0
      ? this.#plan.base
      : { kind: 'plan', plan: this.#plan };
  }

  /** Use this source dialect when parsing string fragments in the plan. */
  readDialect(dialect: string): Expression {
    return new Expression(this.#plan, dialect || 'generic');
  }

  build(): unknown {
    return wasm_compat_build(this.#request({ kind: 'ast' }));
  }

  sql(dialect = 'generic'): string {
    return wasm_compat_build(
      this.#request({ kind: 'sql', dialect: dialect || 'generic' }),
    ) as string;
  }

  eq(other: Input): Expression {
    return this.#binary('eq', other);
  }
  neq(other: Input): Expression {
    return this.#binary('neq', other);
  }
  lt(other: Input): Expression {
    return this.#binary('lt', other);
  }
  lte(other: Input): Expression {
    return this.#binary('lte', other);
  }
  gt(other: Input): Expression {
    return this.#binary('gt', other);
  }
  gte(other: Input): Expression {
    return this.#binary('gte', other);
  }
  add(other: Input): Expression {
    return this.#binary('add', other);
  }
  sub(other: Input): Expression {
    return this.#binary('sub', other);
  }
  mul(other: Input): Expression {
    return this.#binary('mul', other);
  }
  div(other: Input): Expression {
    return this.#binary('div', other);
  }
  mod(other: Input): Expression {
    return this.#binary('mod', other);
  }
  is(other: Input): Expression {
    return this.#binary('is', other);
  }
  and_(other: Input): Expression {
    return this.#binary('and', other, true);
  }
  or_(other: Input): Expression {
    return this.#binary('or', other, true);
  }
  xor(other: Input): Expression {
    return this.#binary('xor', other, true);
  }
  like(other: Input): Expression {
    return this.#binary('like', other);
  }
  ilike(other: Input): Expression {
    return this.#binary('ilike', other);
  }
  rlike(other: Input): Expression {
    return this.#binary('rlike', other);
  }

  not_(): Expression {
    return this.#unary('not');
  }
  neg(): Expression {
    return this.#unary('neg');
  }
  isNull(): Expression {
    return this.#unary('is_null');
  }
  isNotNull(): Expression {
    return this.#unary('is_not_null');
  }

  as_(alias: string): Expression {
    return this.#derive({ kind: 'alias', expression: this._planNode(), alias });
  }

  cast(to: string): Expression {
    return this.#derive({ kind: 'cast', expression: this._planNode(), to });
  }

  asc(): Expression {
    return this.#derive({
      kind: 'ordered',
      expression: this._planNode(),
      desc: false,
    });
  }

  desc(): Expression {
    return this.#derive({
      kind: 'ordered',
      expression: this._planNode(),
      desc: true,
    });
  }

  between(low: Input, high: Input): Expression {
    return this.#derive({
      kind: 'between',
      expression: this._planNode(),
      low: nodeFor(low, false),
      high: nodeFor(high, false),
    });
  }

  isin(...values: Input[]): Expression {
    return this.#derive({
      kind: 'in_list',
      expression: this._planNode(),
      values: values.map((value) => nodeFor(value, false)),
      negated: false,
    });
  }

  notIn(...values: Input[]): Expression {
    return this.#derive({
      kind: 'in_list',
      expression: this._planNode(),
      values: values.map((value) => nodeFor(value, false)),
      negated: true,
    });
  }

  select(...expressions: Input[]): Expression;
  select(options: ListOptions, ...expressions: Input[]): Expression;
  select(...values: (Input | ListOptions)[]): Expression {
    const [options, expressions] = listArguments(values);
    return this.#listOperation('select', expressions, options.append ?? true);
  }

  from_(source: Input): Expression {
    return this.#apply({ kind: 'from', source: nodeFor(source, true) });
  }

  join(
    source: Input,
    options: { on?: Input; joinType?: JoinType } = {},
  ): Expression {
    return this.#apply({
      kind: 'join',
      source: nodeFor(source, true),
      on: options.on === undefined ? null : nodeFor(options.on, true),
      join_type: options.joinType ?? 'inner',
    });
  }

  where(...expressions: Input[]): Expression;
  where(options: ListOptions, ...expressions: Input[]): Expression;
  where(...values: (Input | ListOptions)[]): Expression {
    const [options, expressions] = listArguments(values);
    return this.#listOperation('where', expressions, options.append ?? true);
  }

  groupBy(...expressions: Input[]): Expression;
  groupBy(options: ListOptions, ...expressions: Input[]): Expression;
  groupBy(...values: (Input | ListOptions)[]): Expression {
    const [options, expressions] = listArguments(values);
    return this.#listOperation('group_by', expressions, options.append ?? true);
  }
  having(...expressions: Input[]): Expression;
  having(options: ListOptions, ...expressions: Input[]): Expression;
  having(...values: (Input | ListOptions)[]): Expression {
    const [options, expressions] = listArguments(values);
    return this.#listOperation('having', expressions, options.append ?? true);
  }
  orderBy(...expressions: Input[]): Expression;
  orderBy(options: ListOptions, ...expressions: Input[]): Expression;
  orderBy(...values: (Input | ListOptions)[]): Expression {
    const [options, expressions] = listArguments(values);
    return this.#listOperation('order_by', expressions, options.append ?? true);
  }
  sortBy(...expressions: Input[]): Expression;
  sortBy(options: ListOptions, ...expressions: Input[]): Expression;
  sortBy(...values: (Input | ListOptions)[]): Expression {
    const [options, expressions] = listArguments(values);
    return this.#listOperation('sort_by', expressions, options.append ?? true);
  }
  qualify(...expressions: Input[]): Expression;
  qualify(options: ListOptions, ...expressions: Input[]): Expression;
  qualify(...values: (Input | ListOptions)[]): Expression {
    const [options, expressions] = listArguments(values);
    return this.#listOperation('qualify', expressions, options.append ?? true);
  }

  lateralView(
    expression: Input,
    options: {
      tableAlias?: string;
      columnAliases?: string[];
      outer?: boolean;
    } = {},
  ): Expression {
    return this.#apply({
      kind: 'lateral_view',
      expression: nodeFor(expression, true),
      table_alias: options.tableAlias ?? null,
      column_aliases: options.columnAliases ?? [],
      outer: options.outer ?? false,
    });
  }

  window(
    name: string,
    options: { partitionBy?: Input[]; orderBy?: Input[] } = {},
  ): Expression {
    return this.#apply({
      kind: 'window',
      name,
      partition_by: (options.partitionBy ?? []).map((value) =>
        nodeFor(value, true),
      ),
      order_by: (options.orderBy ?? []).map((value) => nodeFor(value, true)),
    });
  }

  forUpdate(): Expression {
    return this.#apply({ kind: 'lock', lock_type: 'update' });
  }

  forShare(): Expression {
    return this.#apply({ kind: 'lock', lock_type: 'share' });
  }

  hint(text: string): Expression {
    return this.#apply({ kind: 'hint', text });
  }

  ctas(
    table: string,
    options: { replace?: boolean; temporary?: boolean } = {},
  ): Expression {
    return this.#apply({
      kind: 'ctas',
      table,
      replace: options.replace ?? false,
      temporary: options.temporary ?? false,
    });
  }

  limit(expression: Input): Expression {
    return this.#apply({
      kind: 'limit',
      expression: nodeFor(expression, false),
    });
  }

  offset(expression: Input): Expression {
    return this.#apply({
      kind: 'offset',
      expression: nodeFor(expression, false),
    });
  }

  distinct(enabled = true): Expression {
    return this.#apply({ kind: 'distinct', enabled });
  }

  subquery(alias?: string): Expression {
    return this.#apply({ kind: 'subquery', alias: alias ?? null });
  }

  union(other: Expression, distinct = true): Expression {
    return this.#apply({ kind: 'union', other: other._planNode(), distinct });
  }

  intersect(other: Expression, distinct = true): Expression {
    return this.#apply({
      kind: 'intersect',
      other: other._planNode(),
      distinct,
    });
  }

  except_(other: Expression, distinct = true): Expression {
    return this.#apply({ kind: 'except', other: other._planNode(), distinct });
  }

  when(condition: Input, result: Input): Expression {
    return this.#apply({
      kind: 'when',
      condition: nodeFor(condition, true),
      result: nodeFor(result, true),
    });
  }

  else_(result: Input): Expression {
    return this.#apply({ kind: 'else', result: nodeFor(result, true) });
  }

  set_(assignments: Record<string, Input>): Expression {
    return this.#apply({
      kind: 'set',
      assignments: assignmentNodes(assignments),
    });
  }

  insertColumns(columns: string[]): Expression {
    return this.#apply({ kind: 'insert_columns', columns });
  }

  values(...rows: Input[][]): Expression {
    return this.#apply({
      kind: 'values',
      rows: rows.map((row) => row.map((value) => nodeFor(value, false))),
      append: true,
    });
  }

  query(query: Expression): Expression {
    return this.#apply({ kind: 'query', query: query._planNode() });
  }

  mergeUsing(source: Input, on: Input): Expression {
    return this.#apply({
      kind: 'merge_using',
      source: nodeFor(source, true),
      on: nodeFor(on, true),
    });
  }

  whenMatchedUpdate(
    assignments: Record<string, Input>,
    condition?: Input,
  ): Expression {
    return this.#apply({
      kind: 'when_matched_update',
      assignments: assignmentNodes(assignments),
      condition: condition === undefined ? null : nodeFor(condition, true),
    });
  }

  whenMatchedDelete(condition?: Input): Expression {
    return this.#apply({
      kind: 'when_matched_delete',
      condition: condition === undefined ? null : nodeFor(condition, true),
    });
  }

  whenNotMatchedInsert(
    columns: string[],
    values: Input[],
    condition?: Input,
  ): Expression {
    return this.#apply({
      kind: 'when_not_matched_insert',
      columns,
      values: values.map((value) => nodeFor(value, false)),
      condition: condition === undefined ? null : nodeFor(condition, true),
    });
  }

  #binary(op: BinaryOperator, other: Input, parseString = false): Expression {
    return this.#derive({
      kind: 'binary',
      op,
      left: this._planNode(),
      right: nodeFor(other, parseString),
    });
  }

  #unary(op: UnaryOperator): Expression {
    return this.#derive({ kind: 'unary', op, expression: this._planNode() });
  }

  #listOperation(
    kind:
      | 'select'
      | 'where'
      | 'group_by'
      | 'having'
      | 'order_by'
      | 'sort_by'
      | 'qualify',
    expressions: Input[],
    append = true,
  ): Expression {
    return this.#apply({
      kind,
      expressions: expressions.map((value) => nodeFor(value, true)),
      append,
    });
  }

  #apply(operation: PlanOperation): Expression {
    return new Expression(
      {
        base: this.#plan.base,
        operations: [...this.#plan.operations, operation],
      },
      this.#readDialect,
    );
  }

  #derive(node: PlanNode): Expression {
    return Expression.fromNode(node, this.#readDialect);
  }

  #request(output: BuilderOutput): BuildRequest {
    return {
      version: 1,
      read_dialect: this.#readDialect,
      plan: this.#plan,
      output,
    };
  }
}

export function sqlExpr(sql: string): Expression {
  return Expression.fromNode({ kind: 'sql', sql });
}

export function column(name: string, table?: string): Expression {
  return Expression.fromNode({
    kind: 'column',
    name: table ? `${table}.${name}` : name,
  });
}

export const col = column;

export function table_(name: string): Expression {
  return Expression.fromNode({ kind: 'table', name });
}

export function convert(value: Input): Expression {
  return Expression.fromNode(nodeFor(value, false));
}

export const lit = convert;

export function condition(value: Input): Expression {
  return Expression.fromNode(nodeFor(value, true));
}

export function func(name: string, ...args: Input[]): Expression {
  return Expression.fromNode({
    kind: 'function',
    name,
    args: args.map((value) => nodeFor(value, true)),
  });
}

function builtin(functionName: BuiltinFunction, ...args: Input[]): Expression {
  return Expression.fromNode({
    kind: 'builtin',
    function: functionName,
    args: args.map((value) => nodeFor(value, true)),
  });
}

export const count = (expression: Input) => builtin('count', expression);
export const countStar = () => builtin('count_star');
export const countDistinct = (expression: Input) =>
  builtin('count_distinct', expression);
export const sum = (expression: Input) => builtin('sum', expression);
export const avg = (expression: Input) => builtin('avg', expression);
export const min = (expression: Input) => builtin('min', expression);
export const max = (expression: Input) => builtin('max', expression);
export const approxDistinct = (expression: Input) =>
  builtin('approx_distinct', expression);
export const upper = (expression: Input) => builtin('upper', expression);
export const lower = (expression: Input) => builtin('lower', expression);
export const length = (expression: Input) => builtin('length', expression);
export const trim = (expression: Input) => builtin('trim', expression);
export const ltrim = (expression: Input) => builtin('ltrim', expression);
export const rtrim = (expression: Input) => builtin('rtrim', expression);
export const reverse = (expression: Input) => builtin('reverse', expression);
export const initcap = (expression: Input) => builtin('initcap', expression);
export const substring = (expression: Input, start: Input, length?: Input) =>
  length === undefined
    ? builtin('substring', expression, start)
    : builtin('substring', expression, start, length);
export const replace = (expression: Input, old: Input, replacement: Input) =>
  builtin('replace', expression, old, replacement);
export const concatWs = (separator: Input, ...expressions: Input[]) =>
  builtin('concat_ws', separator, ...expressions);
export const coalesce = (...expressions: Input[]) =>
  builtin('coalesce', ...expressions);
export const nullIf = (left: Input, right: Input) =>
  builtin('null_if', left, right);
export const ifNull = (expression: Input, fallback: Input) =>
  builtin('if_null', expression, fallback);
export const abs = (expression: Input) => builtin('abs', expression);
export const round = (expression: Input, decimals?: Input) =>
  decimals === undefined
    ? builtin('round', expression)
    : builtin('round', expression, decimals);
export const floor = (expression: Input) => builtin('floor', expression);
export const ceil = (expression: Input) => builtin('ceil', expression);
export const power = (base: Input, exponent: Input) =>
  builtin('power', base, exponent);
export const sqrt = (expression: Input) => builtin('sqrt', expression);
export const ln = (expression: Input) => builtin('ln', expression);
export const exp = (expression: Input) => builtin('exp', expression);
export const sign = (expression: Input) => builtin('sign', expression);
export const greatest = (...expressions: Input[]) =>
  builtin('greatest', ...expressions);
export const least = (...expressions: Input[]) =>
  builtin('least', ...expressions);
export const currentDate = () => builtin('current_date');
export const currentTime = () => builtin('current_time');
export const currentTimestamp = () => builtin('current_timestamp');
export const extract = (field: string, expression: Input) =>
  Expression.fromNode({
    kind: 'extract',
    field,
    expression: nodeFor(expression, true),
  });
export const rowNumber = () => builtin('row_number');
export const rank = () => builtin('rank');
export const denseRank = () => builtin('dense_rank');

export function select(...expressions: Input[]): Expression {
  return Expression.fromNode({
    kind: 'select',
    expressions: expressions.map((value) => nodeFor(value, true)),
  });
}

export function from_(source: Input): Expression {
  return select().from_(source);
}

function caseBuilder(operand?: Input): Expression {
  return Expression.fromNode({
    kind: 'case',
    operand: operand === undefined ? null : nodeFor(operand, true),
  });
}
export { caseBuilder as case };

function deleteBuilder(table: string, where?: Input): Expression {
  return Expression.fromNode({
    kind: 'delete',
    table,
    where_clause: where === undefined ? null : nodeFor(where, true),
  });
}
export { deleteBuilder as delete };

export function update(
  table: string,
  assignments: Record<string, Input> = {},
): Expression {
  return Expression.fromNode({
    kind: 'update',
    table,
    assignments: assignmentNodes(assignments),
    where_clause: null,
    from: null,
  });
}

export function insert(
  source: Input,
  into: string,
  columns: string[] = [],
): Expression {
  return Expression.fromNode({
    kind: 'insert',
    into,
    expression: nodeFor(source, true),
    columns,
  });
}

export function insertInto(into: string): Expression {
  return Expression.fromNode({
    kind: 'insert',
    into,
    expression: null,
    columns: [],
  });
}

export function mergeInto(target: string): Expression {
  return Expression.fromNode({ kind: 'merge', target });
}

export function and_(...expressions: Input[]): Expression {
  return combine('and', expressions);
}
export function or_(...expressions: Input[]): Expression {
  return combine('or', expressions);
}
export function not_(expression: Input): Expression {
  return condition(expression).not_();
}
export function alias_(expression: Input, alias: string): Expression {
  return condition(expression).as_(alias);
}
export function union(
  left: Expression,
  right: Expression,
  distinct = true,
): Expression {
  return left.union(right, distinct);
}
export function intersect(
  left: Expression,
  right: Expression,
  distinct = true,
): Expression {
  return left.intersect(right, distinct);
}
export function except_(
  left: Expression,
  right: Expression,
  distinct = true,
): Expression {
  return left.except_(right, distinct);
}

function nodeFor(value: Input, parseString: boolean): PlanNode {
  if (value instanceof Expression) {
    // A plan node remains encapsulated; build() is intentionally not called here.
    return expressionNode(value);
  }
  if (typeof value === 'string') {
    return parseString
      ? { kind: 'sql', sql: value }
      : { kind: 'literal', value: { kind: 'string', value } };
  }
  if (value === null) return { kind: 'literal', value: { kind: 'null' } };
  if (typeof value === 'boolean')
    return { kind: 'literal', value: { kind: 'bool', value } };
  if (typeof value === 'number') {
    return Number.isInteger(value)
      ? { kind: 'literal', value: { kind: 'integer', value } }
      : { kind: 'literal', value: { kind: 'float', value } };
  }
  throw new TypeError(`Unsupported builder input: ${String(value)}`);
}

function listArguments(
  values: (Input | ListOptions)[],
): [ListOptions, Input[]] {
  const first = values[0];
  if (
    typeof first === 'object' &&
    first !== null &&
    !(first instanceof Expression)
  ) {
    return [first as ListOptions, values.slice(1) as Input[]];
  }
  return [{}, values as Input[]];
}

function expressionNode(expression: Expression): PlanNode {
  return expression._planNode();
}

function assignmentNodes(values: Record<string, Input>): BuilderAssignment[] {
  return Object.entries(values).map(([column, value]) => ({
    column,
    value: nodeFor(value, false),
  }));
}

function combine(op: BinaryOperator, values: Input[]): Expression {
  if (values.length === 0)
    throw new TypeError(`${op.toUpperCase()} requires at least one expression`);
  return values.slice(1).reduce<Expression>(
    (left, right) =>
      Expression.fromNode({
        kind: 'binary',
        op,
        left: expressionNode(left),
        right: nodeFor(right, true),
      }),
    condition(values[0]),
  );
}
