from .core import Alpha
from .core import (
    parse as core_parse,
)


def parse(input: str) -> Alpha:
    return core_parse(input)
