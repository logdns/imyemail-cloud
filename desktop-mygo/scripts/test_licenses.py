import json
import pathlib
import subprocess
import sys
import tempfile
import unittest


class LicenseTests(unittest.TestCase):
    def test_unicode_metadata_and_notice(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            package = root / "dependency"
            package.mkdir()
            contents = "MIT License\nFixture copyright\n".encode("utf-8")
            (package / "LICENSE").write_bytes(contents)
            destination = root / "licenses"
            destination.mkdir()
            metadata = {
                "packages": [
                    {"id": "cli", "name": "chck-cli", "source": None},
                    {
                        "id": "fixture",
                        "name": "fixture",
                        "version": "1.0.0",
                        "source": "registry+fixture",
                        "manifest_path": str(package / "Cargo.toml"),
                        "license": "MIT",
                        "repository": "https://example.invalid/许可证",
                    },
                ],
                "resolve": {
                    "nodes": [
                        {"id": "cli", "dependencies": ["fixture"]},
                        {"id": "fixture", "dependencies": []},
                    ]
                },
            }
            source = root / "metadata.json"
            source.write_text(json.dumps(metadata, ensure_ascii=False), encoding="utf-8")
            subprocess.run(
                [sys.executable, str(pathlib.Path(__file__).with_name("licenses.py")), str(source), str(destination)],
                check=True,
                timeout=10,
            )
            self.assertEqual((destination / "fixture-1.0.0-LICENSE").read_bytes(), contents)
            self.assertIn("许可证", (destination / "rust-dependencies.txt").read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
