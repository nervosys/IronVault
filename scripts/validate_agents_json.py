#!/usr/bin/env python3
"""Validate .well-known/agents.json against the schema it declares.

The manifest has carried a `$schema` pointer since it was written. Until now
nothing checked it, and for most of that time it pointed at a domain the
project does not own -- so the pointer was decorative twice over.

This makes it load-bearing: the schema is published at
website/public/schemas/, the manifest's `$schema` must name it, and the
manifest must actually validate against it.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / ".well-known" / "agents.json"
SCHEMA = ROOT / "website" / "public" / "schemas" / "agents-discovery-v1.json"


def main() -> int:
    try:
        from jsonschema import Draft202012Validator, FormatChecker
    except ImportError:
        print("jsonschema is not installed; run `pip install jsonschema`", file=sys.stderr)
        return 2

    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))

    Draft202012Validator.check_schema(schema)

    declared = manifest.get("$schema")
    if declared != schema.get("$id"):
        print(
            f"agents.json declares $schema {declared!r} but the schema's $id is "
            f"{schema.get('$id')!r}. A manifest that names a schema it is not "
            f"validated against is worse than one that names none.",
            file=sys.stderr,
        )
        return 1

    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = sorted(validator.iter_errors(manifest), key=lambda e: list(e.path))
    if errors:
        print(f"{len(errors)} validation error(s) in {MANIFEST.name}:", file=sys.stderr)
        for err in errors:
            where = "/".join(str(p) for p in err.path) or "<root>"
            print(f"  {where}: {err.message}", file=sys.stderr)
        return 1

    # The discovery block promises paths exist. Check that too: this is the
    # class of defect that put a 404 logo in ai-plugin.json.
    missing = [
        f"{key} -> {rel}"
        for key, rel in manifest.get("discovery", {}).items()
        if not key.endswith("_version") and not (ROOT / rel).exists()
    ]
    if missing:
        print("discovery paths that do not exist:", file=sys.stderr)
        for m in missing:
            print(f"  {m}", file=sys.stderr)
        return 1

    print(
        f"agents.json validates against {SCHEMA.relative_to(ROOT).as_posix()} "
        f"and every discovery path exists"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
