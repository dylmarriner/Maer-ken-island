#!/usr/bin/env python3
"""Compare, or bring across, what this repository imported from Maer-Ken.

The import set is ``scripts/upstream_imports.json``: each group names the
upstream commit it is pinned to, whole directories imported under the same
path, and single files with their upstream and local paths. ``UPSTREAM.md``
says why each was imported and lists every island divergence since.

    sync_maerken_upstream.py --check <Maer-Ken checkout> [--to <rev>]

reports every imported path that changed upstream between its pin and
``--to`` (the checkout's HEAD by default). It never writes anything. It
exits 0 when nothing changed, 1 when something did, 2 on an error.

    sync_maerken_upstream.py --apply <Maer-Ken checkout> --commit <sha>

copies the import set as it is at ``<sha>`` over this repository's copies,
removes files upstream removed, and moves every pin to ``<sha>``. It
refuses on a dirty worktree, so the result is exactly ``git diff``: island
divergences show there as reverted lines and are put back by hand, with
``UPSTREAM.md`` as the list. Files the island added inside an imported
directory are never touched.

Standard library only.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
DEFAULT_ISLAND = HERE.parent
MANIFEST = "scripts/upstream_imports.json"


class SyncError(Exception):
    """Something that stops a sync before it changes anything."""


def git_bytes(repo: Path, *args: str) -> bytes:
    result = subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        raise SyncError(
            f"git {' '.join(args)} in {repo}: {result.stderr.decode(errors='replace').strip()}"
        )
    return result.stdout


def git(repo: Path, *args: str) -> str:
    return git_bytes(repo, *args).decode()


def resolve(upstream: Path, rev: str) -> str:
    try:
        return git(upstream, "rev-parse", "--verify", f"{rev}^{{commit}}").strip()
    except SyncError:
        raise SyncError(
            f"{upstream} has no commit {rev}; fetch it there first"
        ) from None


def load_manifest(island: Path) -> dict:
    path = island / MANIFEST
    try:
        return json.loads(path.read_text())
    except FileNotFoundError:
        raise SyncError(f"no import manifest at {path}") from None


def changes(upstream: Path, group: dict, target: str) -> list[tuple[str, str, str]]:
    """(status, upstream path, local path) for every imported path that
    differs between the group's pin and `target`."""
    pin = resolve(upstream, group["pin"])
    dirs = group.get("directories", [])
    files = dict(group.get("files", []))
    found = []
    if files:
        present = set(
            git(upstream, "ls-tree", "-r", "--name-only", pin, "--", *files).splitlines()
        )
        for up_path in sorted(set(files) - present):
            found.append(("missing at pin", up_path, files[up_path]))
    paths = [d for d, _ in dirs] + list(files)
    if not paths:
        return found
    status = {"A": "added", "M": "modified", "D": "deleted", "T": "modified"}
    diff = git(upstream, "diff", "--no-renames", "--name-status", pin, target, "--", *paths)
    for line in diff.splitlines():
        code, path = line.split("\t", 1)
        if path in files:
            local = files[path]
        else:
            up_dir, local_dir = next((d, l) for d, l in dirs if path.startswith(d + "/"))
            local = local_dir + path[len(up_dir):]
        found.append((status.get(code[0], "modified"), path, local))
    return found


def check(island: Path, upstream: Path, to: str) -> int:
    manifest = load_manifest(island)
    target = resolve(upstream, to)
    total = 0
    for group in manifest["imports"]:
        found = changes(upstream, group, target)
        total += len(found)
        print(f"{group['name']}: pinned at {group['pin'][:12]}, compared with {target[:12]}")
        for status, up_path, local in found:
            arrow = "" if up_path == local else f" -> {local}"
            print(f"  {status:<14} {up_path}{arrow}")
        if not found:
            print("  unchanged")
    return 1 if total else 0


def apply(island: Path, upstream: Path, commit: str) -> int:
    if git(island, "status", "--porcelain").strip():
        raise SyncError(
            f"{island} has uncommitted changes; a sync must land on a clean worktree "
            "so that `git diff` is exactly what came from upstream"
        )
    manifest = load_manifest(island)
    target = resolve(upstream, commit)
    # Work out everything first: nothing is written unless all of it can be.
    writes: list[tuple[Path, bytes]] = []
    removals: list[Path] = []
    for group in manifest["imports"]:
        for status, up_path, local in changes(upstream, group, target):
            if status == "missing at pin":
                raise SyncError(
                    f"{up_path} is in the manifest but not at its pin {group['pin'][:12]}"
                )
            if status == "deleted":
                removals.append(island / local)
            else:
                writes.append((island / local, git_bytes(upstream, "show", f"{target}:{up_path}")))
    for path, content in writes:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
    for path in removals:
        if path.exists():
            path.unlink()
    for group in manifest["imports"]:
        group["pin"] = target
    (island / MANIFEST).write_text(json.dumps(manifest, indent=2) + "\n")
    print(
        f"{len(writes)} written, {len(removals)} removed; every pin is now {target[:12]}.\n"
        "Review `git diff`: island divergences (UPSTREAM.md) appear as reverted lines "
        "and are put back by hand before committing."
    )
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n\n")[0])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", metavar="MAER_KEN", type=Path)
    mode.add_argument("--apply", metavar="MAER_KEN", type=Path)
    parser.add_argument("--to", default="HEAD", help="upstream revision to compare with (--check)")
    parser.add_argument("--commit", help="upstream commit to bring across (--apply)")
    parser.add_argument("--island", type=Path, default=DEFAULT_ISLAND, help=argparse.SUPPRESS)
    args = parser.parse_args(argv)
    try:
        if args.check:
            return check(args.island, args.check, args.to)
        if not args.commit:
            parser.error("--apply needs --commit <sha>: a sync always names what it brings across")
        return apply(args.island, args.apply, args.commit)
    except SyncError as err:
        print(f"error: {err}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
