from importlib.metadata import PackageNotFoundError, version

try:
    __version__ = version("py-fastplus")
except PackageNotFoundError:
    __version__ = "0+unknown"
