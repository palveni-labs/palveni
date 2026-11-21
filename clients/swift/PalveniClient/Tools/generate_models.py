"""Pipe the JSON from `palveni openapi` to this script to refresh Swift wire types."""

import argparse
import json
from pathlib import Path
import sys


def swift_type(schema, schemas):
    if "$ref" in schema:
        name = schema["$ref"].removeprefix("#/components/schemas/")
        target = schemas[name]
        if target["type"] == "object" or "enum" in target:
            return name
        return swift_type(target, schemas)
    primitive = schema["type"]
    if primitive == "array":
        return "[" + swift_type(schema["items"], schemas) + "]"
    nullable = isinstance(primitive, list) and "null" in primitive
    if nullable:
        primitive, = (kind for kind in primitive if kind != "null")
    types = {
        ("integer", "int64"): "Int64",
        ("integer", "int32"): "Int32",
        ("string", "date-time"): "String",
        ("string", "uuid"): "UUID",
        ("string", None): "String",
        ("boolean", None): "Bool",
    }
    return types[primitive, schema.get("format")] + ("?" if nullable else "")


def generate(document):
    schemas = document["components"]["schemas"]
    lines = ["// Generated from the Palveni OpenAPI document by Tools/generate_models.py.",
             "import Foundation"]
    lines += ["", "public enum ItemKind: String, Decodable, Sendable {"]
    lines.extend(f"    case {value}" for value in schemas["ItemKind"]["enum"])
    lines.append("}")
    models = [
        ("MediaState", "public ", "Decodable"),
        ("PersonalItem", "public ", "Decodable"),
        ("PersonalItemPage", "public ", "Decodable"),
        ("PlaybackUpdate", "", "Encodable"),
        ("FavoriteUpdate", "", "Encodable"),
    ]
    for name, access, protocol in models:
        schema = schemas[name]
        lines += ["", f"{access}struct {name}: {protocol}, Equatable, Sendable {{"]
        for key in schema["required"]:
            lines.append(f"    {access}let {key}: {swift_type(schema['properties'][key], schemas)}")
        lines.append("}")
    return "\n".join(lines) + "\n"


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    target = Path(__file__).resolve().parents[1] / "Sources/PalveniClient/Models.generated.swift"
    output = generate(json.load(sys.stdin))
    if args.check:
        if target.read_text() != output:
            sys.exit("Regenerate the Swift models from the current OpenAPI document.")
    else:
        target.write_text(output)
