# FastPlus

FastPlus 是一个基于 Rust 和 PyO3 实现的 WorldQuant Fast Expression 解析器。它把 Alpha 表达式解析为可访问的 Python 对象，并在解析已知操作符时进行参数数量、参数类型和部分操作符专属规则检查。

当前实现面向 RegularAlpha，收录了 Expert 权限范围内的 108 个操作符。

## 安装

FastPlus 需要 Python 3.12 及以上版本。

从 PyPI 安装：

```bash
pip install py-fastplus
```

如果需要从源码开发：

```bash
uv sync
source .venv/bin/activate
maturin develop --skip-install
```

安装后可以在 Python 中导入：

```python
import fastplus
```

## Python 用法

使用 `fastplus.parse` 解析一个 Alpha 表达式：

```python
import fastplus

alpha = fastplus.parse("a=ts_delay(close, 5); group_rank(a, industry)")

print(alpha.fields)
print(alpha.operators)
```

输出：

```text
{'matrix': ['close'], 'vector': [], 'group': ['industry']}
['ts_delay', 'group_rank']
```

`Alpha` 提供以下属性：

- `assignments`：赋值语句列表，每个元素包含 `variable` 和 `value`。
- `signal`：最终信号表达式。
- `fields`：按 `matrix`、`vector`、`group` 分类的数据字段名称。
- `operators`：表达式中使用的操作符名称列表。

解析失败会抛出 `ValueError`。例如：

最终 `signal` 不能是 `Group` 或 `Vector`：

```python
fastplus.parse("bucket(rank(x), range='0, 1, 0.1')")
```

```text
 --> 1:1
  |
1 | bucket(rank(x), range='0, 1, 0.1')
  | ^--------------------------------^
  |
  = signal must be Matrix-compatible, but got Group
```

`bucket` 不能同时提供有效的 `range` 和 `buckets`：

```python
fastplus.parse("bucket(rank(x), range='0, 1, 0.1', buckets='0, 0.5, 1')")
```

```text
 --> 1:1
  |
1 | bucket(rank(x), range='0, 1, 0.1', buckets='0, 0.5, 1')
  | ^-----------------------------------------------------^
  |
  = operator `bucket` check failed: InvalidExpression("bucket requires exactly one of range and buckets to be valid")
```

`vec_*` 操作符要求 `Vector`，不能把其他返回 `Matrix` 的操作嵌进去：

```python
fastplus.parse("vec_avg(rank(x))")
```

```text
 --> 1:1
  |
1 | vec_avg(rank(x))
  | ^--------------^
  |
  = operator `vec_avg` check failed: InvalidArgumentType { expected: Vector, actual: Matrix }
```

异常信息会同时指出错误位置、操作符名称、错误类型、期望类型和实际类型，便于研究员和大模型定位问题并自动修复表达式。

## FastPlus 语法

### Alpha 和赋值

一个 Alpha 由零个或多个赋值语句和一个最终信号组成：

```text
[variable = expression;] expression [;]
```

例如：

```text
adv = ts_mean(volume, 20);
signal = rank(adv);
signal
```

赋值左侧只能是普通变量，不能使用占位符。最后一个分号可选。

### 变量、字段和占位符

普通变量由字母开头，只能包含 ASCII 字母、数字和下划线：

```text
close
industry
my_signal_1
```

占位符必须使用完整形式：

```text
<close_field/>
```

目前不支持常数占位符，例如 `<1/>` 或 `<0.5/>`。占位符会统一作为数据字段处理，并在 `Alpha.fields` 中保留完整的 `<`、`/>` 标记。

已知操作符会根据参数签名把变量和占位符分类为 `matrix`、`vector` 或 `group`。未知变量在无法确定类型时作为通配类型处理；赋值左侧定义的变量不会被计入 `fields`。

### 操作符调用

操作符调用格式为：

```text
operator(positional_arg, ..., keyword=value, ...)
```

位置参数必须写在关键字参数之前，关键字参数只能使用常量：

```text
add(close, volume, filter=true)
clamp(x, lower=-1, upper=1, inverse=false, mask='mean')
```

已知操作符会检查：

- 位置参数数量；
- 位置参数类型；
- 关键字名称和类型；
- 必填关键字参数；
- 部分操作符专属约束，例如 `k <= d`、`clamp` 的上下界关系，以及 `bucket` 的参数互斥和数组顺序。

未知操作符会保留在表达式中，但跳过操作符检查。

当前实现中的几个平台差异：

- `subtract` 目前只支持两个位置参数。
- `add`、`multiply`、`max`、`min` 等可变参数操作符至少需要两个位置参数。
- `jump_decay` 的位置参数是 `x`、`d`，关键字参数是 `stddev`、`sensitivity`、`force`。
- `ts_rank_gmean_amean_diff` 使用必填的 `lookback` 关键字参数。
- 其他包含 lookback 语义的时间序列操作符通常把 lookback 作为最后一个位置参数，例如 `ts_backfill(x, d)` 和 `ts_rank(x, d)`。
- 关键字参数只能使用常量，不能传入变量、占位符或复杂表达式。
- 操作符至少需要一个位置参数；位置参数和关键字参数不能交错。

### 表达式和运算符

支持括号、前缀运算、二元运算和三元表达式：

```text
-(x + 1)
x > 0 && y < 1
x ? y : z
```

支持的二元运算符包括：

```text
||  &&  !=  ==  >=  <=  >  <  +  -  *  /
```

支持的前缀运算符包括：

```text
!  +  -
```

三元表达式的三个分支都必须是完整表达式。

### 常量和关键字参数

支持整数、浮点数、布尔值和字符串常量。数值常量会根据值归类为整数、正整数、比例、浮点数等类型；数值标量可以按平台规则广播为 `Matrix`。

字符串关键字会经过统一解析，例如：

```text
driver='gaussian'
mask='nearest_bound'
mask='mean'
ignore='NaN'
```

还支持范围、数组和集合形式，例如：

```text
range='0, 1, 0.1'
buckets='0, 0.2, 0.5, 1'
ignore='NaN 0 1'
```

### 注释和空白

空格、制表符和换行会被忽略。支持以下注释：

```text
// 行注释
# 行注释
/* 块注释 */
```

## 当前限制和待开发功能

### 操作符权限范围

当前只有 Expert 权限，因此只能完整检查 Expert 范围内的操作符。未知操作符会跳过检查，以便后续扩展；欢迎提交 Pull Request 补充操作符签名和规则。

### 当前不会检查的规则

以下表达式按平台规则应该失败，但当前还没有检测：

- 定义了但没有使用的变量；
- 用户定义变量与数据字段重名。
- 字段分类的交叉校验：如果同一个字段同时出现在 `matrix` 和 `vector`（或其他类别）中，说明至少有一个位置的类型推断错误，目前尚未检测。

### 待开发功能

- 模板展开：根据占位符自动生成具体表达式。
- 参数遍历：遍历参数空间并生成参数组合。
- AST 结构化输出：提供 `.format()` 方法。
- 表达式编译：提供 `.compile()` 方法，自动合并同类项并优化因子表达式结构。
- 补充更多权限级别的操作符和操作符专属校验。

## 开发验证

Rust 测试和检查：

```bash
cargo fmt -- --check
cargo test --lib
cargo clippy --all-targets --all-features -- -D warnings
```

Python 测试和类型检查：

```bash
uv run pytest
uv run ty check
```
