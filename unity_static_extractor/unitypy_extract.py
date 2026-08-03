#!/usr/bin/env python3
"""Best-effort UnityPy scanner used alongside the AssetsTools.NET scanner.

It is deliberately read-only: its only output is a JSON list of strings.  The
native scanner remains the fallback when Python/UnityPy is unavailable.
"""

import json
import re
import sys
from pathlib import Path


def looks_like_text(value):
    if not isinstance(value, str):
        return False
    value = value.strip()
    if len(value) < 2 or len(value) > 2_000 or sum(char.isalpha() for char in value) < 2:
        return False
    if value.startswith("\ufeff"):
        return False
    if value.startswith(("UnityEngine.", "com.unity.", "<Keyboard>", "line:")):
        return False
    if re.fullmatch(r"[0-9a-fA-F]{32}", value):
        return False
    if re.fullmatch(r"[0-9a-fA-F-]{36}", value):
        return False
    if "://" in value or value.endswith((".asset", ".prefab")):
        return False
    if value.startswith("color_name,hex,r,g,b"):
        return False
    if value.startswith('{"') or (value.startswith("{") and '"name"' in value):
        return False
    if not re.search(r"\s", value):
        # Class names, enum names and serialized identifiers make up the bulk of
        # typetree noise. Keep short words such as "Yes" and "No".
        if re.fullmatch(r"[a-z]+[A-Z][a-zA-Z]+", value):
            return False
        if re.fullmatch(r"[A-Z][a-zA-Z]+[A-Z][a-zA-Z]+", value):
            return False
        if re.fullmatch(r"(?:[A-Za-z0-9]+\.)+[A-Za-z0-9]+", value):
            return False
        if "_" in value or "&" in value:
            return False
    return True


DISPLAY_TEXT_FIELDS = {
    "m_text", "text", "m_displaytext", "m_translatedtext",
    "m_originaltext", "m_localizedtext", "m_tooltip", "m_description",
}
TEXT_ASSET_JSON_FIELDS = {
    "text", "value", "localized", "translation", "translated",
    "dialogue", "line",
}


def text_fields_in(value, seen=None, depth=0):
    """Yield only known player-facing fields from a MonoBehaviour typetree."""
    if depth > 12:
        return
    if seen is None:
        seen = set()
    if not isinstance(value, (dict, list, tuple)):
        return
    ident = id(value)
    if ident in seen:
        return
    seen.add(ident)
    if isinstance(value, dict):
        for key, item in value.items():
            if str(key).lower() in DISPLAY_TEXT_FIELDS and isinstance(item, str):
                if looks_like_text(item):
                    yield item.strip()
            else:
                yield from text_fields_in(item, seen, depth + 1)
    else:
        for item in value:
            yield from text_fields_in(item, seen, depth + 1)


def textasset_lines(tree):
    """UnityPy documents TextAsset.m_Script as the text payload."""
    if not isinstance(tree, dict):
        return
    script = tree.get("m_Script")
    if not isinstance(script, str):
        return
    try:
        payload = json.loads(script)
    except (TypeError, ValueError):
        # Generic CSV/bytes TextAssets can be color/font/configuration data.
        # Raw lines are safe only for explicitly text-like asset names.
        name = str(tree.get("m_Name", "")).lower()
        if not any(token in name for token in (
            "dialog", "localiz", "string", "subtitle", "yarn", "story",
            "language", "locale", "text",
        )):
            return
        for line in script.splitlines():
            if not line.lstrip().startswith('"') and looks_like_text(line):
                yield line.strip()
        return

    def json_values(value, accepted=False):
        if isinstance(value, str):
            if accepted and looks_like_text(value):
                yield value.strip()
        elif isinstance(value, dict):
            for key, item in value.items():
                yield from json_values(item, str(key).lower() in TEXT_ASSET_JSON_FIELDS)
        elif isinstance(value, list):
            for item in value:
                yield from json_values(item, accepted)

    yield from json_values(payload)


def candidate_files(data_dir):
    bundle_extensions = {".bundle", ".unity3d", ".ab", ".assetbundle", ".data"}
    for path in data_dir.rglob("*"):
        if not path.is_file() or path.name.endswith((".resS", ".resource", ".bak", ".temp")):
            continue
        lower = path.name.lower()
        is_serialized = lower.endswith(".assets") or lower.startswith(("level", "sharedassets"))
        is_bundle = path.suffix.lower() in bundle_extensions
        # Addressables frequently store bundles as hash-named extensionless files.
        is_addressable = "streamingassets/aa/" in path.as_posix().lower() and not path.suffix
        if is_serialized or is_bundle or is_addressable:
            yield path


def main():
    if len(sys.argv) != 3:
        print("usage: unitypy_extract.py <data-folder> <output.json>", file=sys.stderr)
        return 2
    try:
        import UnityPy
    except ImportError as error:
        print(f"UnityPy/dependency unavailable ({error})", file=sys.stderr)
        return 3

    data_dir, output = map(Path, sys.argv[1:])
    results, scanned, failed = set(), 0, 0
    for path in candidate_files(data_dir):
        try:
            env = UnityPy.load(str(path))
            scanned += 1
            for obj in env.objects:
                # UnityPy parses objects lazily. Read just the two kinds that can
                # carry UI/dialogue text; never scan MonoScript metadata.
                if obj.type.name not in {"TextAsset", "MonoBehaviour"}:
                    continue
                try:
                    tree = obj.parse_as_dict()
                except Exception:
                    try:
                        tree = obj.read_typetree()
                    except Exception:
                        continue
                if obj.type.name == "TextAsset":
                    results.update(textasset_lines(tree))
                else:
                    results.update(text_fields_in(tree))
        except Exception:
            failed += 1

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(sorted(results), ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"[UnityPy] scanned={scanned} unreadable={failed} texts={len(results)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
