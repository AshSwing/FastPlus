import fastplus
import fastplus.core
import pytest


@pytest.mark.parametrize(
    "raw",
    [
        "returns",
        "a=1;returns",
        "x ? y : z",
        "foo(<x/>)",
        "-(1 + 2) * 3",
    ],
)
def test_parse_success(raw):
    assert fastplus.parse(input=raw) is not None


@pytest.mark.parametrize(
    "raw",
    [
        "a=1;",
        "a=1;b=2;c=foo(x, y, z=3)",
        "foo(z=1, x)",
        "1abc",
        "x ? y",
        "foo(,x)",
    ],
)
def test_parse_failure(raw):
    with pytest.raises(ValueError):
        fastplus.parse(input=raw)


def test_parse_tree_exposes_assignments_and_operation_arguments():
    alpha = fastplus.parse("a=1;add(a, 2, filter=true)")

    assert len(alpha.assignments) == 1
    assert alpha.assignments[0].variable == "a"
    assignment_value = alpha.assignments[0].value
    assert isinstance(assignment_value, fastplus.core.PositiveIntegerConstant)
    assert assignment_value.value == 1

    signal = alpha.signal
    assert isinstance(signal, fastplus.core.OperationExpr)
    assert signal.op == "add"
    assert len(signal.pos_args) == 2
    assert isinstance(signal.pos_args[0], fastplus.core.VariableExpr)
    assert signal.pos_args[0].name == "a"
    filter_value = signal.kw_args["filter"]
    assert isinstance(filter_value, fastplus.core.BooleanConstant)
    assert filter_value.value is True


def test_parse_tree_exposes_expression_and_constant_types():
    alpha = fastplus.parse(
        "-(x + 1) ? <value/> : clamp(x, lower=0, upper=1, inverse=false, mask='mean')"
    )

    assert isinstance(alpha.signal, fastplus.core.TernaryExpr)
    assert isinstance(alpha.signal.cond, fastplus.core.UnaryExpr)
    assert isinstance(alpha.signal.cond.expr, fastplus.core.GroupExpr)
    assert isinstance(alpha.signal.cond.expr.expr, fastplus.core.BinaryExpr)
    assert isinstance(alpha.signal.then_branch, fastplus.core.PlaceholderExpr)

    clamp = alpha.signal.else_branch
    assert isinstance(clamp, fastplus.core.OperationExpr)
    assert isinstance(clamp.kw_args["mask"], fastplus.core.MaskConstant)
    assert clamp.kw_args["mask"].value == fastplus.core.Mask.Mean


def test_python_parse_reports_operator_type_errors():
    with pytest.raises(ValueError, match="divide"):
        fastplus.parse("divide('not a matrix', 2)")
