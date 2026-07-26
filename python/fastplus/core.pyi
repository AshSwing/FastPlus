class BinaryOp:
    NotEqual: BinaryOp
    Equal: BinaryOp
    GreaterEqual: BinaryOp
    LessEqual: BinaryOp
    Greater: BinaryOp
    Less: BinaryOp
    Add: BinaryOp
    Subtract: BinaryOp
    Multiply: BinaryOp
    Divide: BinaryOp
    Or: BinaryOp
    And: BinaryOp

class UnaryOp:
    Positive: UnaryOp
    Negative: UnaryOp
    Not: UnaryOp

class Driver:
    Gaussian: Driver
    Uniform: Driver
    Cauchy: Driver

class Expression: ...

class TernaryExpr(Expression):
    cond: Expression
    then_branch: Expression
    else_branch: Expression

class BinaryExpr(Expression):
    left: Expression
    op: BinaryOp
    right: Expression

class UnaryExpr(Expression):
    op: UnaryOp
    expr: Expression

class OperationExpr(Expression):
    op: str
    pos_args: list[Expression]
    kw_args: list[KwArg]

class Constant(Expression): ...

class FloatConstant(Constant):
    value: float

class NonNegativeFloatConstant(Constant):
    value: float

class PositiveFloatConstant(Constant):
    value: float

class RatioConstant(Constant):
    value: float

class IntegerConstant(Constant):
    value: int

class NonNegativeIntegerConstant(Constant):
    value: int

class PositiveIntegerConstant(Constant):
    value: int

class EnumIntegerConstant(Constant):
    value_from: int
    value_to: int

class BooleanConstant(Constant):
    value: bool

class NaNConstant(Constant):
    value: float

class DriverConstant(Constant):
    value: Driver

class RangeConstant(Constant):
    value: float

class ArrayConstant(Constant):
    value: list[float]

class StringConstant(Constant):
    value: str

class PlaceholderExpr(Expression):
    name: str

class VariableExpr(Expression):
    name: str

class GroupExpr(Expression):
    expr: Expression

class Assignment:
    variable: str
    value: Expression

class KwArg:
    name: str
    value: Constant

class Alpha:
    assignments: list[Assignment]
    signal: Expression

def parse(input: str) -> Alpha: ...
