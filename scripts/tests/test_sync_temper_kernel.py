#!/usr/bin/env python3
"""Contract tests for the repository-owned Temper kernel pin synchronizer."""

from __future__ import annotations

import os
from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
SYNCHRONIZER = REPOSITORY_ROOT / "scripts" / "sync-temper-kernel"
OLD_REVISION = "1" * 40
NEW_REVISION = "2" * 40


class SyncTemperKernelTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        self.write(
            ".temper-kernel.toml",
            f'''repository = "https://github.com/nikstern/temper.git"
revision = "{OLD_REVISION}"
manifest_patterns = ["Cargo.toml", "crates/**/Cargo.toml", "os-apps/**/Cargo.toml"]
lockfile_patterns = ["Cargo.lock", "crates/**/Cargo.lock", "os-apps/**/Cargo.lock"]
''',
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def write(self, relative_path: str, content: str) -> Path:
        path = self.root / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(textwrap.dedent(content))
        return path

    def run_sync(self, *arguments: str, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [str(SYNCHRONIZER), *arguments, "--root", str(self.root)],
            text=True,
            capture_output=True,
            env=env,
            check=False,
        )

    def test_check_reports_moving_and_mixed_repository_pins_with_exact_paths(self) -> None:
        self.write(
            "os-apps/example/wasm/new_module/Cargo.toml",
            '''
            [package]
            name = "new-module"
            version = "0.1.0"

            [dependencies]
            temper-wasm-sdk = { git = "https://github.com/nerdsane/temper.git", branch = "main" }
            ''',
        )
        self.write(
            "reference-projects/intentionally-moving/Cargo.toml",
            '''
            [dependencies]
            temper-wasm-sdk = { git = "https://github.com/nerdsane/temper.git", branch = "main" }
            ''',
        )
        self.write(
            "os-apps/example/target/generated/Cargo.toml",
            '''
            [dependencies]
            temper-wasm-sdk = { git = "https://github.com/nerdsane/temper.git", branch = "main" }
            ''',
        )

        result = self.run_sync("--check")

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("os-apps/example/wasm/new_module/Cargo.toml", result.stderr)
        self.assertIn("moving branch", result.stderr)
        self.assertIn("mixed repository", result.stderr)
        self.assertNotIn("reference-projects", result.stderr)
        self.assertNotIn("target/generated", result.stderr)

    def test_check_identifies_a_malformed_manifest_revision(self) -> None:
        self.write(
            "crates/example/Cargo.toml",
            '''
            [dependencies]
            temper-runtime = { git = "https://github.com/nikstern/temper.git", rev = "deadbeef" }
            ''',
        )

        result = self.run_sync("--check")

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("crates/example/Cargo.toml", result.stderr)
        self.assertIn("malformed revision", result.stderr)
        self.assertIn("40-character lowercase", result.stderr)

    def test_check_reports_a_stale_lockfile_with_its_exact_path(self) -> None:
        self.write(
            "Cargo.toml",
            f'''
            [package]
            name = "root"
            version = "0.1.0"

            [dependencies]
            temper-runtime = {{ git = "https://github.com/nikstern/temper.git", rev = "{OLD_REVISION}" }}
            ''',
        )
        self.write(
            "Cargo.lock",
            f'''
            version = 4

            [[package]]
            name = "temper-runtime"
            version = "0.1.0"
            source = "git+https://github.com/nikstern/temper.git?rev=deadbeef#{'d' * 40}"
            ''',
        )

        result = self.run_sync("--check")

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Cargo.lock", result.stderr)
        self.assertIn("selector `rev=deadbeef`", result.stderr)
        self.assertIn(f"resolves `{'d' * 40}`", result.stderr)

    def test_sync_updates_manifests_and_delegates_lockfile_refresh_to_cargo(self) -> None:
        manifest = self.write(
            "os-apps/example/wasm/new_module/Cargo.toml",
            f'''
            [package]
            name = "new-module"
            version = "0.1.0"

            [workspace]

            [dependencies]
            temper-wasm-sdk = {{ git = "https://github.com/nikstern/temper.git", rev = "{OLD_REVISION[:8]}" }}
            ''',
        )
        lockfile = self.write(
            "os-apps/example/wasm/new_module/Cargo.lock",
            f'''
            version = 4

            [[package]]
            name = "temper-wasm-sdk"
            version = "0.1.0"
            source = "git+https://github.com/nikstern/temper.git?rev={OLD_REVISION[:8]}#{OLD_REVISION}"
            ''',
        )
        fake_bin = self.root / "fake-bin"
        fake_bin.mkdir()
        fake_cargo = fake_bin / "cargo"
        fake_cargo.write_text(
            textwrap.dedent(
                f'''\
                #!/usr/bin/env python3
                import os
                from pathlib import Path
                import sys

                args = sys.argv[1:]
                Path(os.environ["FAKE_CARGO_LOG"]).write_text(" ".join(args))
                manifest = Path(args[args.index("--manifest-path") + 1])
                lockfile = manifest.with_name("Cargo.lock")
                lockfile.write_text(lockfile.read_text().replace("{OLD_REVISION[:8]}#{OLD_REVISION}", "{NEW_REVISION}#{NEW_REVISION}"))
                '''
            )
        )
        fake_cargo.chmod(0o755)
        cargo_log = self.root / "cargo.log"
        environment = os.environ.copy()
        environment["PATH"] = f"{fake_bin}{os.pathsep}{environment['PATH']}"
        environment["FAKE_CARGO_LOG"] = str(cargo_log)

        result = self.run_sync(NEW_REVISION, env=environment)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(f'rev = "{NEW_REVISION}"', manifest.read_text())
        self.assertIn(f'revision = "{NEW_REVISION}"', (self.root / ".temper-kernel.toml").read_text())
        self.assertIn(f"?rev={NEW_REVISION}#{NEW_REVISION}", lockfile.read_text())
        self.assertIn("update --manifest-path", cargo_log.read_text())
        self.assertIn("-p temper-wasm-sdk", cargo_log.read_text())
        self.assertIn(f"--precise {NEW_REVISION}", cargo_log.read_text())
        self.assertIn("1 manifest", result.stdout)
        self.assertIn("1 lockfile", result.stdout)

        check = self.run_sync("--check")
        self.assertEqual(check.returncode, 0, check.stderr)

    def test_sync_rejects_a_non_full_revision_without_modifying_the_pin(self) -> None:
        before = (self.root / ".temper-kernel.toml").read_text()

        result = self.run_sync("deadbeef")

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("40-character lowercase", result.stderr)
        self.assertEqual((self.root / ".temper-kernel.toml").read_text(), before)

    def test_sync_handles_multiline_inline_and_expanded_dependency_tables(self) -> None:
        manifest = self.write(
            "crates/example/Cargo.toml",
            f'''
            [package]
            name = "example"
            version = "0.1.0"

            [dependencies]
            temper-runtime = {{
                git = "https://github.com/nerdsane/temper.git",
                branch = "main",
                features = ["observe"],
            }}

            [dev-dependencies.temper-spec]
            git = "https://github.com/nerdsane/temper.git"
            tag = "moving"
            ''',
        )

        result = self.run_sync(NEW_REVISION)

        self.assertEqual(result.returncode, 0, result.stderr)
        source = manifest.read_text()
        self.assertEqual(source.count("https://github.com/nikstern/temper.git"), 2)
        self.assertEqual(source.count(f'rev = "{NEW_REVISION}"'), 2)
        self.assertNotIn("branch =", source)
        self.assertNotIn("tag =", source)


if __name__ == "__main__":
    unittest.main()
