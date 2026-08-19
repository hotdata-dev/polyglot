/**
 * Backwards-compatible TypeScript builder facade.
 *
 * The classes in this module preserve the original mutable chaining API while
 * adapting every operation to the immutable, binding-neutral builder plan in
 * `compat.ts`. There are no independent WASM builder objects or AST semantics.
 */

import * as compat from './compat';

export type ExprInput = Expr | string | number | boolean | null;

function input(value: ExprInput): compat.Expression {
  if (value instanceof Expr) return value._expression;
  if (typeof value === 'string') return compat.col(value);
  return compat.lit(value);
}

function inputs(values: ExprInput[]): compat.Expression[] {
  return values.map(input);
}

export class Expr {
  /** @internal */
  readonly _expression: compat.Expression;

  /** @internal */
  constructor(expression: compat.Expression) {
    this._expression = expression;
  }

  eq(other: ExprInput): Expr {
    return new Expr(this._expression.eq(input(other)));
  }
  neq(other: ExprInput): Expr {
    return new Expr(this._expression.neq(input(other)));
  }
  lt(other: ExprInput): Expr {
    return new Expr(this._expression.lt(input(other)));
  }
  lte(other: ExprInput): Expr {
    return new Expr(this._expression.lte(input(other)));
  }
  gt(other: ExprInput): Expr {
    return new Expr(this._expression.gt(input(other)));
  }
  gte(other: ExprInput): Expr {
    return new Expr(this._expression.gte(input(other)));
  }
  and(other: ExprInput): Expr {
    return new Expr(this._expression.and_(input(other)));
  }
  or(other: ExprInput): Expr {
    return new Expr(this._expression.or_(input(other)));
  }
  xor(other: ExprInput): Expr {
    return new Expr(this._expression.xor(input(other)));
  }
  not(): Expr {
    return new Expr(this._expression.not_());
  }
  add(other: ExprInput): Expr {
    return new Expr(this._expression.add(input(other)));
  }
  sub(other: ExprInput): Expr {
    return new Expr(this._expression.sub(input(other)));
  }
  mul(other: ExprInput): Expr {
    return new Expr(this._expression.mul(input(other)));
  }
  div(other: ExprInput): Expr {
    return new Expr(this._expression.div(input(other)));
  }
  mod(other: ExprInput): Expr {
    return new Expr(this._expression.mod(input(other)));
  }
  neg(): Expr {
    return new Expr(this._expression.neg());
  }
  is(other: ExprInput): Expr {
    return new Expr(this._expression.is(input(other)));
  }
  like(pattern: ExprInput): Expr {
    return new Expr(this._expression.like(input(pattern)));
  }
  ilike(pattern: ExprInput): Expr {
    return new Expr(this._expression.ilike(input(pattern)));
  }
  rlike(pattern: ExprInput): Expr {
    return new Expr(this._expression.rlike(input(pattern)));
  }
  isNull(): Expr {
    return new Expr(this._expression.isNull());
  }
  isNotNull(): Expr {
    return new Expr(this._expression.isNotNull());
  }
  between(low: ExprInput, high: ExprInput): Expr {
    return new Expr(this._expression.between(input(low), input(high)));
  }
  inList(...values: ExprInput[]): Expr {
    return new Expr(this._expression.isin(...inputs(values)));
  }
  notIn(...values: ExprInput[]): Expr {
    return new Expr(this._expression.notIn(...inputs(values)));
  }
  alias(name: string): Expr {
    return new Expr(this._expression.as_(name));
  }
  as(name: string): Expr {
    return this.alias(name);
  }
  cast(to: string): Expr {
    return new Expr(this._expression.cast(to));
  }
  asc(): Expr {
    return new Expr(this._expression.asc());
  }
  desc(): Expr {
    return new Expr(this._expression.desc());
  }
  toSql(dialect = 'generic'): string {
    return this._expression.sql(dialect);
  }
  toJSON(): unknown {
    return this._expression.build();
  }
  free(): void {}
}

export class WindowDefBuilder {
  /** @internal */
  _partitionBy: compat.Input[] = [];
  /** @internal */
  _orderBy: compat.Input[] = [];

  partitionBy(...expressions: ExprInput[]): this {
    this._partitionBy = inputs(expressions);
    return this;
  }
  orderBy(...expressions: ExprInput[]): this {
    this._orderBy = inputs(expressions);
    return this;
  }
  free(): void {}
}

export function col(name: string): Expr {
  return new Expr(compat.col(name));
}
export function lit(value: string | number | boolean | null): Expr {
  return new Expr(compat.lit(value));
}
export function star(): Expr {
  return new Expr(compat.condition('*'));
}
export function sqlNull(): Expr {
  return new Expr(compat.lit(null));
}
export function boolean(value: boolean): Expr {
  return new Expr(compat.lit(value));
}
export function table(name: string): Expr {
  return new Expr(compat.table_(name));
}
export function sqlExpr(sql: string): Expr {
  return new Expr(compat.sqlExpr(sql));
}
export function condition(sql: string): Expr {
  return sqlExpr(sql);
}
export function func(name: string, ...args: ExprInput[]): Expr {
  return new Expr(compat.func(name, ...inputs(args)));
}
export function not(expression: ExprInput): Expr {
  return new Expr(compat.not_(input(expression)));
}
export function cast(expression: ExprInput, to: string): Expr {
  return new Expr(input(expression).cast(to));
}
export function alias(expression: ExprInput, name: string): Expr {
  return new Expr(input(expression).as_(name));
}
export function subquery(query: SelectBuilder, name: string): Expr {
  return new Expr(query._current().subquery(name));
}
export function and(...conditions: ExprInput[]): Expr {
  if (conditions.length === 0) return boolean(true);
  if (conditions.length === 1 && conditions[0] instanceof Expr)
    return conditions[0];
  return new Expr(compat.and_(...inputs(conditions)));
}
export function or(...conditions: ExprInput[]): Expr {
  if (conditions.length === 0) return boolean(false);
  if (conditions.length === 1 && conditions[0] instanceof Expr)
    return conditions[0];
  return new Expr(compat.or_(...inputs(conditions)));
}

export const count = (expression?: ExprInput) =>
  new Expr(
    expression === undefined
      ? compat.countStar()
      : compat.count(input(expression)),
  );
export const countDistinct = (expression: ExprInput) =>
  new Expr(compat.countDistinct(input(expression)));
export const approxDistinct = (expression: ExprInput) =>
  new Expr(compat.approxDistinct(input(expression)));
export const sum = (expression: ExprInput) =>
  new Expr(compat.sum(input(expression)));
export const avg = (expression: ExprInput) =>
  new Expr(compat.avg(input(expression)));
export const min = (expression: ExprInput) =>
  new Expr(compat.min(input(expression)));
export const max = (expression: ExprInput) =>
  new Expr(compat.max(input(expression)));
export const upper = (expression: ExprInput) =>
  new Expr(compat.upper(input(expression)));
export const lower = (expression: ExprInput) =>
  new Expr(compat.lower(input(expression)));
export const length = (expression: ExprInput) =>
  new Expr(compat.length(input(expression)));
export const trim = (expression: ExprInput) =>
  new Expr(compat.trim(input(expression)));
export const ltrim = (expression: ExprInput) =>
  new Expr(compat.ltrim(input(expression)));
export const rtrim = (expression: ExprInput) =>
  new Expr(compat.rtrim(input(expression)));
export const reverse = (expression: ExprInput) =>
  new Expr(compat.reverse(input(expression)));
export const initcap = (expression: ExprInput) =>
  new Expr(compat.initcap(input(expression)));
export function substring(
  expression: ExprInput,
  start: ExprInput,
  length?: ExprInput,
): Expr {
  return new Expr(
    length === undefined
      ? compat.substring(input(expression), input(start))
      : compat.substring(input(expression), input(start), input(length)),
  );
}
export const replace = (
  expression: ExprInput,
  from: ExprInput,
  to: ExprInput,
) => new Expr(compat.replace(input(expression), input(from), input(to)));
export const concatWs = (separator: ExprInput, ...expressions: ExprInput[]) =>
  new Expr(compat.concatWs(input(separator), ...inputs(expressions)));
export const coalesce = (...expressions: ExprInput[]) =>
  new Expr(compat.coalesce(...inputs(expressions)));
export const nullIf = (left: ExprInput, right: ExprInput) =>
  new Expr(compat.nullIf(input(left), input(right)));
export const ifNull = (expression: ExprInput, fallback: ExprInput) =>
  new Expr(compat.ifNull(input(expression), input(fallback)));
export const abs = (expression: ExprInput) =>
  new Expr(compat.abs(input(expression)));
export function round(expression: ExprInput, decimals?: ExprInput): Expr {
  return new Expr(
    decimals === undefined
      ? compat.round(input(expression))
      : compat.round(input(expression), input(decimals)),
  );
}
export const floor = (expression: ExprInput) =>
  new Expr(compat.floor(input(expression)));
export const ceil = (expression: ExprInput) =>
  new Expr(compat.ceil(input(expression)));
export const power = (base: ExprInput, exponent: ExprInput) =>
  new Expr(compat.power(input(base), input(exponent)));
export const sqrt = (expression: ExprInput) =>
  new Expr(compat.sqrt(input(expression)));
export const ln = (expression: ExprInput) =>
  new Expr(compat.ln(input(expression)));
export const exp = (expression: ExprInput) =>
  new Expr(compat.exp(input(expression)));
export const sign = (expression: ExprInput) =>
  new Expr(compat.sign(input(expression)));
export const greatest = (...expressions: ExprInput[]) =>
  new Expr(compat.greatest(...inputs(expressions)));
export const least = (...expressions: ExprInput[]) =>
  new Expr(compat.least(...inputs(expressions)));
export const currentDate = () => func('CURRENT_DATE');
export const currentTime = () => func('CURRENT_TIME');
export const currentTimestamp = () => func('CURRENT_TIMESTAMP');
export const extract = (unit: string, from: ExprInput) =>
  new Expr(compat.extract(unit, input(from)));
export const rowNumber = () => new Expr(compat.rowNumber());
export const rank = () => new Expr(compat.rank());
export const denseRank = () => new Expr(compat.denseRank());

abstract class StatefulBuilder {
  #consumed = false;

  protected ensureActive(): void {
    if (this.#consumed) throw new Error('Builder already consumed');
  }
  protected consume(): void {
    this.ensureActive();
    this.#consumed = true;
  }
  free(): void {
    this.#consumed = true;
  }
}

export class SelectBuilder extends StatefulBuilder {
  #expression: compat.Expression;

  constructor() {
    super();
    this.#expression = compat.select();
  }
  /** @internal */
  _current(): compat.Expression {
    this.ensureActive();
    return this.#expression;
  }
  select(...columns: (ExprInput | '*')[]): this {
    this.ensureActive();
    const values = columns.map((column) =>
      column === '*' ? compat.condition('*') : input(column),
    );
    this.#expression = this.#expression.select(...values);
    return this;
  }
  from(tableOrExpression: string | Expr): this {
    this.ensureActive();
    this.#expression = this.#expression.from_(
      typeof tableOrExpression === 'string'
        ? compat.table_(tableOrExpression)
        : tableOrExpression._expression,
    );
    return this;
  }
  join(tableName: string, on: ExprInput): this {
    this.#expression = this._current().join(tableName, {
      on: input(on),
      joinType: 'inner',
    });
    return this;
  }
  leftJoin(tableName: string, on: ExprInput): this {
    this.#expression = this._current().join(tableName, {
      on: input(on),
      joinType: 'left',
    });
    return this;
  }
  rightJoin(tableName: string, on: ExprInput): this {
    this.#expression = this._current().join(tableName, {
      on: input(on),
      joinType: 'right',
    });
    return this;
  }
  fullJoin(tableName: string, on: ExprInput): this {
    this.#expression = this._current().join(tableName, {
      on: input(on),
      joinType: 'full',
    });
    return this;
  }
  crossJoin(tableName: string): this {
    this.#expression = this._current().join(tableName, { joinType: 'cross' });
    return this;
  }
  where(condition: ExprInput): this {
    this.#expression = this._current().where(
      typeof condition === 'string' ? condition : input(condition),
    );
    return this;
  }
  groupBy(...expressions: ExprInput[]): this {
    this.#expression = this._current().groupBy(...inputs(expressions));
    return this;
  }
  having(condition: ExprInput): this {
    this.#expression = this._current().having(input(condition));
    return this;
  }
  orderBy(...expressions: ExprInput[]): this {
    this.#expression = this._current().orderBy(...inputs(expressions));
    return this;
  }
  sortBy(...expressions: ExprInput[]): this {
    this.#expression = this._current().sortBy(...inputs(expressions));
    return this;
  }
  limit(value: number): this {
    this.#expression = this._current().limit(value);
    return this;
  }
  offset(value: number): this {
    this.#expression = this._current().offset(value);
    return this;
  }
  distinct(): this {
    this.#expression = this._current().distinct();
    return this;
  }
  qualify(condition: ExprInput): this {
    this.#expression = this._current().qualify(input(condition));
    return this;
  }
  window(name: string, definition: WindowDefBuilder): this {
    this.#expression = this._current().window(name, {
      partitionBy: definition._partitionBy,
      orderBy: definition._orderBy,
    });
    return this;
  }
  lateral(
    functionExpression: Expr,
    tableAlias: string,
    columnAliases: string[],
    options: { outer?: boolean } = {},
  ): this {
    this.#expression = this._current().lateralView(
      functionExpression._expression,
      { tableAlias, columnAliases, outer: options.outer },
    );
    return this;
  }
  hint(text: string): this {
    this.#expression = this._current().hint(text);
    return this;
  }
  forUpdate(): this {
    this.#expression = this._current().forUpdate();
    return this;
  }
  forShare(): this {
    this.#expression = this._current().forShare();
    return this;
  }
  ctas(
    tableName: string,
    options: { replace?: boolean; temporary?: boolean } = {},
  ): unknown {
    const result = this._current().ctas(tableName, options).build();
    this.consume();
    return result;
  }
  ctasSql(
    tableName: string,
    dialect = 'generic',
    options: { replace?: boolean; temporary?: boolean } = {},
  ): string {
    const result = this._current().ctas(tableName, options).sql(dialect);
    this.consume();
    return result;
  }
  union(other: SelectBuilder): SetOpBuilder {
    return new SetOpBuilder(this._current().union(other._current()));
  }
  unionAll(other: SelectBuilder): SetOpBuilder {
    return new SetOpBuilder(this._current().union(other._current(), false));
  }
  intersect(other: SelectBuilder): SetOpBuilder {
    return new SetOpBuilder(this._current().intersect(other._current()));
  }
  intersectAll(other: SelectBuilder): SetOpBuilder {
    return new SetOpBuilder(this._current().intersect(other._current(), false));
  }
  except(other: SelectBuilder): SetOpBuilder {
    return new SetOpBuilder(this._current().except_(other._current()));
  }
  exceptAll(other: SelectBuilder): SetOpBuilder {
    return new SetOpBuilder(this._current().except_(other._current(), false));
  }
  toSql(dialect = 'generic'): string {
    const result = this._current().sql(dialect);
    this.consume();
    return result;
  }
  build(): unknown {
    const result = this._current().build();
    this.consume();
    return result;
  }
}

export function select(...columns: (ExprInput | '*')[]): SelectBuilder {
  return new SelectBuilder().select(...columns);
}

export class InsertBuilder extends StatefulBuilder {
  #expression: compat.Expression;
  constructor(tableName: string) {
    super();
    this.#expression = compat.insertInto(tableName);
  }
  columns(...columns: string[]): this {
    this.ensureActive();
    this.#expression = this.#expression.insertColumns(columns);
    return this;
  }
  values(...values: ExprInput[]): this {
    this.ensureActive();
    this.#expression = this.#expression.values(inputs(values));
    return this;
  }
  query(query: SelectBuilder): this {
    this.ensureActive();
    this.#expression = this.#expression.query(query._current());
    return this;
  }
  toSql(dialect = 'generic'): string {
    this.consume();
    return this.#expression.sql(dialect);
  }
  build(): unknown {
    this.consume();
    return this.#expression.build();
  }
}
export const insertInto = (tableName: string) => new InsertBuilder(tableName);
export const insert = insertInto;

export class UpdateBuilder extends StatefulBuilder {
  #expression: compat.Expression;
  constructor(tableName: string) {
    super();
    this.#expression = compat.update(tableName);
  }
  set(column: string, value: ExprInput): this {
    this.ensureActive();
    this.#expression = this.#expression.set_({ [column]: input(value) });
    return this;
  }
  where(condition: ExprInput): this {
    this.ensureActive();
    this.#expression = this.#expression.where(input(condition));
    return this;
  }
  from(tableName: string): this {
    this.ensureActive();
    this.#expression = this.#expression.from_(tableName);
    return this;
  }
  toSql(dialect = 'generic'): string {
    this.consume();
    return this.#expression.sql(dialect);
  }
  build(): unknown {
    this.consume();
    return this.#expression.build();
  }
}
export const update = (tableName: string) => new UpdateBuilder(tableName);

export class DeleteBuilder extends StatefulBuilder {
  #expression: compat.Expression;
  constructor(tableName: string) {
    super();
    this.#expression = compat.delete(tableName);
  }
  where(condition: ExprInput): this {
    this.ensureActive();
    this.#expression = this.#expression.where(input(condition));
    return this;
  }
  toSql(dialect = 'generic'): string {
    this.consume();
    return this.#expression.sql(dialect);
  }
  build(): unknown {
    this.consume();
    return this.#expression.build();
  }
}
export const deleteFrom = (tableName: string) => new DeleteBuilder(tableName);
export const del = deleteFrom;

export class MergeBuilder extends StatefulBuilder {
  #expression: compat.Expression;
  constructor(target: string) {
    super();
    this.#expression = compat.mergeInto(target);
  }
  using(source: string, on: ExprInput): this {
    this.ensureActive();
    this.#expression = this.#expression.mergeUsing(source, input(on));
    return this;
  }
  whenMatchedUpdate(
    assignments: Record<string, ExprInput>,
    condition?: ExprInput,
  ): this {
    this.ensureActive();
    this.#expression = this.#expression.whenMatchedUpdate(
      Object.fromEntries(
        Object.entries(assignments).map(([column, value]) => [
          column,
          input(value),
        ]),
      ),
      condition === undefined ? undefined : input(condition),
    );
    return this;
  }
  whenMatchedDelete(condition?: ExprInput): this {
    this.ensureActive();
    this.#expression = this.#expression.whenMatchedDelete(
      condition === undefined ? undefined : input(condition),
    );
    return this;
  }
  whenNotMatchedInsert(
    columns: string[],
    values: ExprInput[],
    condition?: ExprInput,
  ): this {
    this.ensureActive();
    this.#expression = this.#expression.whenNotMatchedInsert(
      columns,
      inputs(values),
      condition === undefined ? undefined : input(condition),
    );
    return this;
  }
  toSql(dialect = 'generic'): string {
    this.consume();
    return this.#expression.sql(dialect);
  }
  build(): unknown {
    this.consume();
    return this.#expression.build();
  }
}
export const mergeInto = (target: string) => new MergeBuilder(target);

export class CaseBuilder {
  #expression: compat.Expression;
  constructor(expression: compat.Expression = compat.case()) {
    this.#expression = expression;
  }
  when(condition: ExprInput, result: ExprInput): this {
    this.#expression = this.#expression.when(input(condition), input(result));
    return this;
  }
  else_(result: ExprInput): this {
    this.#expression = this.#expression.else_(input(result));
    return this;
  }
  build(): Expr {
    return new Expr(this.#expression);
  }
  toSql(dialect = 'generic'): string {
    return this.#expression.sql(dialect);
  }
}
export const caseWhen = () => new CaseBuilder();
export const caseOf = (operand: ExprInput) =>
  new CaseBuilder(compat.case(input(operand)));

export class SetOpBuilder extends StatefulBuilder {
  #expression: compat.Expression;
  constructor(expression: compat.Expression) {
    super();
    this.#expression = expression;
  }
  orderBy(...expressions: ExprInput[]): this {
    this.ensureActive();
    this.#expression = this.#expression.orderBy(...inputs(expressions));
    return this;
  }
  limit(value: number): this {
    this.ensureActive();
    this.#expression = this.#expression.limit(value);
    return this;
  }
  offset(value: number): this {
    this.ensureActive();
    this.#expression = this.#expression.offset(value);
    return this;
  }
  toSql(dialect = 'generic'): string {
    this.consume();
    return this.#expression.sql(dialect);
  }
  build(): unknown {
    this.consume();
    return this.#expression.build();
  }
}

export const union = (left: SelectBuilder, right: SelectBuilder) =>
  new SetOpBuilder(left._current().union(right._current()));
export const unionAll = (left: SelectBuilder, right: SelectBuilder) =>
  new SetOpBuilder(left._current().union(right._current(), false));
export const intersect = (left: SelectBuilder, right: SelectBuilder) =>
  new SetOpBuilder(left._current().intersect(right._current()));
export const intersectAll = (left: SelectBuilder, right: SelectBuilder) =>
  new SetOpBuilder(left._current().intersect(right._current(), false));
export const except = (left: SelectBuilder, right: SelectBuilder) =>
  new SetOpBuilder(left._current().except_(right._current()));
export const exceptAll = (left: SelectBuilder, right: SelectBuilder) =>
  new SetOpBuilder(left._current().except_(right._current(), false));
