#!/usr/bin/env python3
"""project_params.py — read the project's declared parameters.

Authority: constitution, Governance > "Project Parameters Live In One
Declaration". `.specify/memory/project.yml` is the sole home of project-specific values;
this module is how Python tooling reads them, and it is also a CLI so shell scripts can
read them without a second copy of any value.

WHY. A project-specific value with two homes has no home: the two copies disagree
silently after the first rename.

THE ROOT IS RESOLVED, NEVER STORED. It comes from this file's own location. The
declaration holds repository-relative paths only, and a declared value that is absolute is
refused rather than used, because a stored absolute path is what fails when a project moves.

Module use:
    from project_params import params, path, value
    value("structure.literature")         # 'literature'
    path("structure.literature")          # absolute
    params()["domain"]["solvers"]

CLI use (for shell scripts):
    python3 project_params.py branches.integration         # the declared value
    python3 project_params.py --path structure.literature  # resolved to absolute
    python3 project_params.py --root                       # the repository root
    python3 project_params.py --json                       # the whole declaration
"""
from __future__ import annotations

import argparse
import json
import os
import sys
from functools import lru_cache

DECLARATION_RELATIVE = os.path.join(".specify", "memory", "project.yml")


class DeclarationError(RuntimeError):
    """The declaration is missing, unreadable, or does not hold what was asked for."""


def repo_root() -> str:
    """Resolve the repository root from THIS file's location, never from a literal.

    This file lives at <root>/.specify/scripts/python/project_params.py, so the root is
    three levels up. The resolution is verified rather than assumed.
    """
    here = os.path.dirname(os.path.abspath(__file__))
    root = os.path.abspath(os.path.join(here, "..", "..", ".."))
    if not os.path.isdir(os.path.join(root, ".specify")):
        raise DeclarationError(
            f"Resolved repository root {root} has no .specify directory. This module "
            "assumes it lives at <root>/.specify/scripts/python/; if it moved, fix the "
            "resolution rather than adding an absolute literal."
        )
    return root


@lru_cache(maxsize=1)
def params() -> dict:
    """The whole declaration, parsed. Cached for the process."""
    declaration = os.path.join(repo_root(), DECLARATION_RELATIVE)
    if not os.path.isfile(declaration):
        raise DeclarationError(
            f"The project declaration is absent: {declaration}\n"
            "Constitution v1.30.0 makes it the sole home of project-specific values, so "
            "there is no fallback to hard-coded defaults by design."
        )
    try:
        import yaml  # noqa: PLC0415  (an optional dependency, reported precisely below)
    except ImportError as exc:  # pragma: no cover - environment-dependent
        raise DeclarationError(
            "Reading the project declaration needs PyYAML (pip install pyyaml). The "
            "declaration is the single source of project-specific values, so guessing a "
            "default here would defeat its purpose."
        ) from exc
    with open(declaration, "r", encoding="utf-8") as fh:
        loaded = yaml.safe_load(fh)
    if not isinstance(loaded, dict):
        raise DeclarationError(f"{declaration} did not parse to a mapping.")
    return loaded


def value(key: str):
    """Look up a dotted key. Raises on an undeclared key rather than returning a default."""
    node = params()
    for part in key.split("."):
        if not isinstance(node, dict) or part not in node:
            raise DeclarationError(
                f'"{key}" is not declared in {DECLARATION_RELATIVE} (failed at "{part}"). '
                "Add it to the declaration rather than hard-coding the value."
            )
        node = node[part]
    return node


def path(key: str, *extra: str) -> str:
    """Resolve a declared repository-relative path to an absolute path."""
    declared = value(key)
    if not isinstance(declared, str) or not declared:
        raise DeclarationError(
            f'Declared value "{key}" is not a path (got {type(declared).__name__}).'
        )
    if os.path.isabs(declared):
        raise DeclarationError(
            f'Declared value "{key}" is an ABSOLUTE path ({declared}). The declaration '
            "must hold repository-relative paths only."
        )
    return os.path.join(repo_root(), declared, *extra)


def main(argv=None) -> int:
    p = argparse.ArgumentParser(description=__doc__,
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("key", nargs="?", help="dotted key into the declaration")
    p.add_argument("--path", action="store_true", help="resolve the value to an absolute path")
    p.add_argument("--root", action="store_true", help="print the resolved repository root")
    p.add_argument("--json", action="store_true", help="print the whole declaration as JSON")
    a = p.parse_args(argv)
    try:
        if a.root:
            print(repo_root())
        elif a.json:
            print(json.dumps(params(), indent=2, sort_keys=True))
        elif a.key:
            print(path(a.key) if a.path else value(a.key))
        else:
            p.print_usage(sys.stderr)
            return 2
    except DeclarationError as exc:
        print(f"project_params: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
