#!/usr/bin/env python3
"""Independent, standard-library CCJZ v1 reader and verifier."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import pathlib
import posixpath
import sys
import zipfile

MIMETYPE = "application/vnd.chemsema.document+zip"
SCHEMA = "chemsema.container.v1"


def _safe_name(name: str) -> bool:
    return bool(name) and "\\" not in name and ":" not in name and not name.startswith("/") and all(
        part not in ("", ".", "..") for part in name.split("/")
    ) and posixpath.normpath(name) == name


def _verified(archive: zipfile.ZipFile, descriptor: dict) -> bytes:
    path = descriptor["path"]
    if not _safe_name(path):
        raise ValueError(f"unsafe CCJZ entry name: {path}")
    data = archive.read(path)
    if len(data) != descriptor["size"]:
        raise ValueError(f"CCJZ entry size mismatch: {path}")
    if hashlib.sha256(data).hexdigest() != descriptor["sha256"]:
        raise ValueError(f"CCJZ entry SHA-256 mismatch: {path}")
    return data


def read_ccjz(path: pathlib.Path) -> dict:
    raw_prefix = path.read_bytes()[:2]
    if raw_prefix == b"\x1f\x8b":
        with gzip.open(path, "rt", encoding="utf-8") as source:
            return json.load(source)
    with zipfile.ZipFile(path, "r") as archive:
        infos = archive.infolist()
        if not infos or infos[0].filename != "mimetype":
            raise ValueError("CCJZ mimetype must be the first ZIP entry")
        names = [entry.filename for entry in infos]
        if any(not _safe_name(name) for name in names):
            raise ValueError("CCJZ contains an unsafe entry name")
        if len(names) != len(set(names)) or len(names) != len({name.lower() for name in names}):
            raise ValueError("CCJZ contains duplicate entry names")
        if any(entry.compress_type != zipfile.ZIP_STORED for entry in infos):
            raise ValueError("CCJZ v1 entries must use stored compression")
        if archive.read("mimetype").decode("ascii") != MIMETYPE:
            raise ValueError("CCJZ mimetype is not canonical")
        manifest = json.loads(archive.read("manifest.json"))
        if (
            manifest.get("schema") != SCHEMA
            or manifest.get("mediaType") != MIMETYPE
            or manifest.get("documentFormat") != "chemsema/0.2"
        ):
            raise ValueError("unsupported CCJZ manifest header")
        descriptors = [manifest["root"], *manifest.get("sceneChunks", [])]
        descriptors.extend(manifest.get("resources", {}).values())
        descriptors.extend(manifest.get("attachments", {}).values())
        declared = {"mimetype", "manifest.json", *(item["path"] for item in descriptors)}
        if set(names) != declared:
            extra = sorted(set(names) - declared)
            missing = sorted(declared - set(names))
            raise ValueError(f"CCJZ manifest directory mismatch: extra={extra}, missing={missing}")
        root = json.loads(_verified(archive, manifest["root"]))
        scene = root["entities"]["scene"]
        if scene:
            raise ValueError("CCJZ root scene must be empty")
        first = 0
        for chunk in manifest.get("sceneChunks", []):
            if chunk["firstRecord"] != first:
                raise ValueError("CCJZ scene chunks are not contiguous")
            records = [json.loads(line) for line in _verified(archive, chunk).splitlines() if line]
            if len(records) != chunk["recordCount"]:
                raise ValueError("CCJZ scene chunk record count mismatch")
            scene.extend(records)
            first += len(records)
        resources = root["resources"]
        for resource_id, descriptor in sorted(manifest.get("resources", {}).items()):
            if resource_id in resources:
                raise ValueError(f"duplicate CCJZ resource: {resource_id}")
            resources[resource_id] = json.loads(_verified(archive, descriptor))
        for resource_id, descriptor in manifest.get("attachments", {}).items():
            data = resources.get(resource_id, {}).get("data", {})
            if (
                data.get("storage") != "ccjz-attachment"
                or data.get("mediaType") != descriptor["mediaType"]
                or data.get("byteLength") != descriptor["size"]
                or data.get("sha256") != descriptor["sha256"]
            ):
                raise ValueError(f"CCJZ attachment descriptor mismatch: {resource_id}")
            _verified(archive, descriptor)
        return root


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=pathlib.Path)
    parser.add_argument("--pretty", action="store_true")
    arguments = parser.parse_args()
    document = read_ccjz(arguments.input)
    json.dump(
        document,
        sys.stdout,
        ensure_ascii=False,
        sort_keys=True,
        indent=2 if arguments.pretty else None,
        separators=None if arguments.pretty else (",", ":"),
    )
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
