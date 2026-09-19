"""The interpreter the tools in this directory need, and the one reason for it.

Measured over this directory, and held by `test_python_floor.py` rather than by this
docstring: `toolchains.py` reads a manifest's TOML with `tomllib`, which arrives in 3.11,
and every other feature these modules use is older than that — each one parses at 3.8 and
imports nothing else that is version-gated. So the floor is one number with one cause, and
the rule derives both directions of it: a module that needs more than the floor fails by
name, and a floor raised without a cause fails too.

A module that reaches a gated feature says so by name rather than dying as an ImportError:
the two that do import this, so an older interpreter is told the floor and the reason
instead of `No module named 'tomllib'`. Which modules those are is derived from the import
graph rather than listed, and so is which CI jobs owe that interpreter.
"""
import sys

FLOOR = (3, 11)
WHY = "toolchains.py reads manifests with tomllib, which is 3.11"


def refuse(version) -> str | None:
    """What to say about `version`, or None where it is not below the floor."""
    if tuple(version)[:2] < FLOOR:
        return (f"planning/integrity needs Python {FLOOR[0]}.{FLOOR[1]}+: {WHY}; "
                f"this interpreter is {version[0]}.{version[1]}")
    return None


def require() -> None:
    """Refuse by name where the running interpreter is older than the floor."""
    said = refuse(sys.version_info)
    if said is not None:
        raise SystemExit(said)


require()
