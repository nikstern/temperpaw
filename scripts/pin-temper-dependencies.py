#!/usr/bin/env python3
"""Pin every Temper git dependency to one repository and immutable revision."""

from __future__ import annotations

import argparse
import re
from pathlib import Path


TEMPER_PIN = re.compile(
    r'git = "https://github\.com/[^/"]+/temper\.git", '
    r'(?:rev = "[0-9a-f]{7,40}"|branch = "[^"]+"|tag = "[^"]+")'
)
TEMPER_LOCK_PIN = re.compile(
    r'source = "git\+https://github\.com/[^/"]+/temper\.git\?rev=[0-9a-f]{7,40}#[0-9a-f]{40}"'
)
IGNORED_DIRECTORIES = {".git", "node_modules", "target"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("repository", help="GitHub owner/repository, for example nikstern/temper")
    parser.add_argument("revision", help="exact 40-character lowercase Git commit")
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify the requested pin without changing manifests",
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="repository root (defaults to the parent of this script directory)",
    )
    return parser.parse_args()


def tracked_candidates(root: Path, filename: str) -> list[Path]:
    return sorted(
        path
        for path in root.rglob(filename)
        if not any(part in IGNORED_DIRECTORIES for part in path.relative_to(root).parts)
    )


def main() -> int:
    args = parse_args()
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/temper", args.repository):
        raise SystemExit("repository must have the form <github-owner>/temper")
    if not re.fullmatch(r"[0-9a-f]{40}", args.revision):
        raise SystemExit("revision must be an exact 40-character lowercase Git commit")

    root = args.root.resolve()
    manifest_revision = args.revision[:8]
    expected = (
        f'git = "https://github.com/{args.repository}.git", '
        f'rev = "{manifest_revision}"'
    )
    matched = 0
    changed: list[Path] = []
    stale: list[Path] = []
    for manifest in tracked_candidates(root, "Cargo.toml"):
        source = manifest.read_text()
        count = len(TEMPER_PIN.findall(source))
        if count == 0:
            continue
        matched += count
        updated = TEMPER_PIN.sub(expected, source)
        if updated != source:
            stale.append(manifest.relative_to(root))
            if not args.check:
                manifest.write_text(updated)
                changed.append(manifest.relative_to(root))

    expected_lock = (
        f'source = "git+https://github.com/{args.repository}.git?rev={manifest_revision}'
        f'#{args.revision}"'
    )
    lock_matches = 0
    changed_locks: list[Path] = []
    for lockfile in tracked_candidates(root, "Cargo.lock"):
        source = lockfile.read_text()
        count = len(TEMPER_LOCK_PIN.findall(source))
        lock_matches += count
        updated = TEMPER_LOCK_PIN.sub(expected_lock, source)
        if updated != source:
            stale.append(lockfile.relative_to(root))
            if not args.check:
                lockfile.write_text(updated)
                changed_locks.append(lockfile.relative_to(root))

    if matched == 0:
        raise SystemExit("no Temper git dependency pins were found")
    if args.check and stale:
        rendered = "\n".join(f"- {path}" for path in stale)
        raise SystemExit(f"Temper dependency pin is stale in:\n{rendered}")

    verb = "verified" if args.check else "updated"
    print(
        f"{verb} {matched} manifest pins and {lock_matches} lockfile entries "
        f"({len(changed)} manifests, {len(changed_locks)} lockfiles changed)"
    )
    for path in changed + changed_locks:
        print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
