import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
from release_version import apply_version, version_from_tag


class ReleaseVersionTests(unittest.TestCase):
    def test_validates_tags(self):
        for tag in ("0.1.1", "v0.1.1"):
            self.assertEqual(version_from_tag(tag), "0.1.1")
        for tag in ("v01.1.1", "v0.1", "v0.1.1-rc1", "v0.1.1/unsafe", ""):
            with self.assertRaises(ValueError):
                version_from_tag(tag)

    def test_updates_only_reportstudio_version(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            (root / "Cargo.toml").write_text('[package]\nname = "reportstudio"\nversion = "0.1.0"\n\n[dependencies]\nserde = "1"\n')
            (root / "Cargo.lock").write_text('[[package]]\nname = "reportstudio"\nversion = "0.1.0"\n\n[[package]]\nname = "other"\nversion = "0.1.0"\n')
            apply_version(root, "0.1.1")
            self.assertIn('version = "0.1.1"', (root / "Cargo.toml").read_text())
            lock = (root / "Cargo.lock").read_text()
            self.assertIn('name = "reportstudio"\nversion = "0.1.1"', lock)
            self.assertIn('name = "other"\nversion = "0.1.0"', lock)

    def test_invalid_lock_does_not_change_manifest(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            text = '[package]\nname = "reportstudio"\nversion = "0.1.0"\n'
            (root / "Cargo.toml").write_text(text)
            (root / "Cargo.lock").write_text('[[package]]\nname = "other"\nversion = "0.1.0"\n')
            with self.assertRaises(ValueError):
                apply_version(root, "v0.1.1")
            self.assertEqual((root / "Cargo.toml").read_text(), text)


if __name__ == "__main__":
    unittest.main()
