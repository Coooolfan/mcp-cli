#!/usr/bin/env python3
"""Stamp the release version into this checkout without changing dependencies."""
import os
import re
import tomllib
from pathlib import Path


def set_version(root: Path, version: str) -> None:
    if not re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(-(alpha|beta|rc)\.(0|[1-9]\d*))?", version, re.ASCII):
        raise ValueError("Invalid release version")
    manifest = root / "Cargo.toml"
    lockfile = root / "Cargo.lock"
    manifest_text = manifest.read_text(encoding="utf-8")
    lock_text = lockfile.read_text(encoding="utf-8")
    package = tomllib.loads(manifest_text)["package"]
    if package["name"] != "mcp-cli":
        raise ValueError("Expected the mcp-cli package")
    old_version = package["version"]
    manifest_text, count = re.subn(
        r'(?m)^(version\s*=\s*")' + re.escape(old_version) + r'("\s*)$',
        lambda match: match[1] + version + match[2], manifest_text, count=1,
    )
    if count != 1:
        raise ValueError("Could not update package version")
    pattern = r'(\[\[package\]\]\s*name = "mcp-cli"\s*version = ")' + re.escape(old_version) + '"'
    lock_text, count = re.subn(pattern, lambda match: match[1] + version + '"', lock_text)
    if count != 1:
        raise ValueError("Cargo.lock must contain exactly one matching mcp-cli package")
    manifest.write_text(manifest_text, encoding="utf-8")
    lockfile.write_text(lock_text, encoding="utf-8")


if __name__ == "__main__":
    set_version(Path.cwd(), os.environ["VERSION"])
