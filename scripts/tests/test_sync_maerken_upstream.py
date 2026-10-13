"""Tests for scripts/sync_maerken_upstream.py, on throwaway git repositories.

    python3 -m unittest discover -s scripts/tests
"""

from __future__ import annotations

import contextlib
import io
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import sync_maerken_upstream as sync  # noqa: E402


def run(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(repo), *args], capture_output=True, text=True, check=True
    ).stdout.strip()


def commit_all(repo: Path, message: str) -> str:
    run(repo, "add", "-A")
    run(repo, "-c", "user.name=t", "-c", "user.email=t@example.invalid", "commit", "-qm", message)
    return run(repo, "rev-parse", "HEAD")


def write(root: Path, path: str, text: str) -> None:
    p = root / path
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text)


class SyncTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        base = Path(self.tmp.name)
        self.up, self.island = base / "upstream", base / "island"
        for repo in (self.up, self.island):
            repo.mkdir()
            run(repo, "init", "-q")
        # Upstream at the pin: a crate directory and two loose files.
        write(self.up, "crates/core/src/lib.rs", "pub fn a() {}\n")
        write(self.up, "crates/core/src/old.rs", "// old\n")
        write(self.up, "apps/ui/assets/model.glb", "GLB1")
        write(self.up, "docs/canon/STATUS.md", "status 1\n")
        self.pin = commit_all(self.up, "pin")
        # The island imported them, renaming one file's path, and added a
        # file of its own inside the imported crate.
        write(self.island, "crates/core/src/lib.rs", "pub fn a() {}\n")
        write(self.island, "crates/core/src/old.rs", "// old\n")
        write(self.island, "crates/core/src/island_only.rs", "// ours\n")
        write(self.island, "assets/model.glb", "GLB1")
        write(self.island, "docs/canon/STATUS.md", "status 1\n")
        write(
            self.island,
            sync.MANIFEST,
            json.dumps(
                {
                    "repository": "test/upstream",
                    "imports": [
                        {
                            "name": "core",
                            "pin": self.pin,
                            "directories": [["crates/core", "crates/core"]],
                            "files": [
                                ["apps/ui/assets/model.glb", "assets/model.glb"],
                                ["docs/canon/STATUS.md", "docs/canon/STATUS.md"],
                            ],
                        }
                    ],
                }
            ),
        )
        commit_all(self.island, "import")

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def main(self, *args: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = sync.main([*args, "--island", str(self.island)])
        return code, out.getvalue(), err.getvalue()

    def move_upstream_on(self) -> str:
        write(self.up, "crates/core/src/lib.rs", "pub fn a() {}\npub fn b() {}\n")
        (self.up / "crates/core/src/old.rs").unlink()
        write(self.up, "crates/core/src/new.rs", "// new\n")
        write(self.up, "apps/ui/assets/model.glb", "GLB2")
        write(self.up, "unrelated.txt", "not imported\n")
        return commit_all(self.up, "later")

    def test_unchanged_upstream_reports_nothing(self) -> None:
        write(self.up, "unrelated.txt", "not imported\n")
        commit_all(self.up, "unrelated")
        code, out, _ = self.main("--check", str(self.up))
        self.assertEqual(code, 0)
        self.assertIn("unchanged", out)

    def test_check_names_every_changed_imported_path_and_writes_nothing(self) -> None:
        self.move_upstream_on()
        code, out, _ = self.main("--check", str(self.up))
        self.assertEqual(code, 1)
        self.assertIn("modified       crates/core/src/lib.rs", out)
        self.assertIn("deleted        crates/core/src/old.rs", out)
        self.assertIn("added          crates/core/src/new.rs", out)
        self.assertIn("modified       apps/ui/assets/model.glb -> assets/model.glb", out)
        self.assertNotIn("unrelated", out)
        self.assertEqual(run(self.island, "status", "--porcelain"), "")

    def test_a_manifest_path_missing_at_its_pin_is_reported(self) -> None:
        manifest = json.loads((self.island / sync.MANIFEST).read_text())
        manifest["imports"][0]["files"].append(["docs/canon/GONE.md", "docs/canon/GONE.md"])
        (self.island / sync.MANIFEST).write_text(json.dumps(manifest))
        code, out, _ = self.main("--check", str(self.up))
        self.assertEqual(code, 1)
        self.assertIn("missing at pin docs/canon/GONE.md", out)

    def test_apply_brings_across_exactly_the_import_set(self) -> None:
        later = self.move_upstream_on()
        code, out, _ = self.main("--apply", str(self.up), "--commit", later)
        self.assertEqual(code, 0, out)
        self.assertEqual(
            (self.island / "crates/core/src/lib.rs").read_text(), "pub fn a() {}\npub fn b() {}\n"
        )
        self.assertFalse((self.island / "crates/core/src/old.rs").exists())
        self.assertEqual((self.island / "crates/core/src/new.rs").read_text(), "// new\n")
        self.assertEqual((self.island / "assets/model.glb").read_text(), "GLB2")
        self.assertEqual((self.island / "crates/core/src/island_only.rs").read_text(), "// ours\n")
        self.assertFalse((self.island / "unrelated.txt").exists())
        manifest = json.loads((self.island / sync.MANIFEST).read_text())
        self.assertEqual(manifest["imports"][0]["pin"], later)
        # And now nothing differs from the new pin.
        commit_all(self.island, "synced")
        self.assertEqual(self.main("--check", str(self.up))[0], 0)

    def test_apply_refuses_a_dirty_worktree(self) -> None:
        later = self.move_upstream_on()
        write(self.island, "crates/core/src/lib.rs", "work in progress\n")
        code, _, err = self.main("--apply", str(self.up), "--commit", later)
        self.assertEqual(code, 2)
        self.assertIn("uncommitted changes", err)
        self.assertEqual((self.island / "crates/core/src/lib.rs").read_text(), "work in progress\n")

    def test_apply_refuses_a_commit_upstream_does_not_have(self) -> None:
        code, _, err = self.main("--apply", str(self.up), "--commit", "0" * 40)
        self.assertEqual(code, 2)
        self.assertIn("has no commit", err)
        self.assertEqual(run(self.island, "status", "--porcelain"), "")

    def test_apply_needs_an_explicit_commit(self) -> None:
        with contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit):
                sync.main(["--apply", str(self.up), "--island", str(self.island)])


class ManifestTest(unittest.TestCase):
    def test_the_repositorys_manifest_names_files_that_exist_here(self) -> None:
        root = Path(__file__).resolve().parents[2]
        manifest = json.loads((root / sync.MANIFEST).read_text())
        for group in manifest["imports"]:
            self.assertEqual(len(group["pin"]), 40, group["name"])
            for _, local in group.get("directories", []):
                self.assertTrue((root / local).is_dir(), local)
            for _, local in group.get("files", []):
                self.assertTrue((root / local).is_file(), local)


if __name__ == "__main__":
    unittest.main()
