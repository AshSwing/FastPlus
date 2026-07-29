# FastPlus - Alpha Expression Language, based on WorldQuant's Fast Expression

基于 WorldQuant Fast Expression 开发的因子表达式语法解析器.

## 平台规则

- 关键字参数全部都可以使用字符串类型
- 数值标量会自动广播, 可以视作 MATRIX

## 当前操作符支持范围

当前实现面向 RegularAlpha，仅收录 RegularAlpha 可用的 108 个操作符。原始操作符资料中属于 SuperAlpha 的操作符暂不支持，包括 Combo、Reduce、Special 等类别中的专用操作符。

当前与平台行为保持一致的限制：

- `subtract` 目前只支持 `nary = 2`，即两个位置参数；平台的无限参数版本尚未实现。
- `add`、`multiply`、`max`、`min` 等 `nary = -1` 操作符只在签名中声明一个可变参数类型，实际使用时至少需要两个位置参数，由 op-check 负责校验。
- `jump_decay` 的位置参数为 `x` 和 `d`，关键字参数为 `stddev=false`、`sensitivity=0.5`、`force=0.1`。
- `ts_rank_gmean_amean_diff` 要求使用 `lookback` 关键字参数，以保持表达式语法的完整性；虽然平台允许将 `d` 作为位置参数传入，FastPlus 当前不采用这种写法。
- 除 `ts_rank_gmean_amean_diff` 外，包含 lookback 语义的时间序列操作符将 lookback 作为最后一个位置参数，例如 `ts_backfill(x, d)`、`ts_rank(x, d)`。
