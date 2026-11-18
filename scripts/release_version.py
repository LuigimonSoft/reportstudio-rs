"""Set the crate version in an isolated release checkout from its Git tag."""
import pathlib
import re
import sys

TAG = re.compile(r"v?((?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*))\Z")
MANIFEST = re.compile(r'(\[package\][\s\S]*?^version\s*=\s*")[^"]+("\s*$)', re.M)
LOCK = re.compile(r'(\[\[package\]\]\s*\nname = "reportstudio"\s*\nversion = ")[^"]+("\s*$)', re.M)


def version_from_tag(tag):
    match = TAG.fullmatch(tag)
    if not match:
        raise ValueError(f"Expected a stable MAJOR.MINOR.PATCH tag, with an optional v prefix: {tag!r}")
    return match.group(1)


def apply_version(root, tag):
    version = version_from_tag(tag)
    manifest_path = pathlib.Path(root) / "Cargo.toml"
    lock_path = pathlib.Path(root) / "Cargo.lock"
    manifest = manifest_path.read_text(encoding="utf-8")
    lock = lock_path.read_text(encoding="utf-8")
    manifest, manifest_count = MANIFEST.subn(lambda m: m.group(1) + version + m.group(2), manifest, count=1)
    lock, lock_count = LOCK.subn(lambda m: m.group(1) + version + m.group(2), lock)
    if manifest_count != 1 or lock_count != 1:
        raise ValueError("Expected a package version in Cargo.toml and exactly one reportstudio package in Cargo.lock")
    manifest_path.write_text(manifest, encoding="utf-8")
    lock_path.write_text(lock, encoding="utf-8")
    return version


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("Usage: python3 release_version.py <release-tag>")
    print(f"Release crate version: {apply_version(pathlib.Path.cwd(), sys.argv[1])}")
