import fastplus
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
