"""Guard the Rust CI profile against debug-artifact disk exhaustion."""

from pathlib import Path
import re
import unittest


class CiRustProfileTests(unittest.TestCase):
    def test_ci_disables_debug_info_for_dev_and_test_profiles(self) -> None:
        workflow = Path(__file__).resolve().parents[2] / ".github" / "workflows" / "ci.yml"
        text = workflow.read_text(encoding="utf-8")
        for key in ("CARGO_PROFILE_DEV_DEBUG", "CARGO_PROFILE_TEST_DEBUG"):
            with self.subTest(key=key):
                self.assertRegex(text, rf'(?m)^\s+{key}:\s*["\']?0["\']?\s*$')

    def test_fast_test_job_bounds_parallel_linking(self) -> None:
        workflow = Path(__file__).resolve().parents[2] / ".github" / "workflows" / "ci.yml"
        text = workflow.read_text(encoding="utf-8")
        check_job = text.split("  check:\n", 1)[1].split("\n  # The desktop application", 1)[0]
        self.assertRegex(check_job, r'(?m)^\s+CARGO_BUILD_JOBS:\s*["\']?2["\']?\s*$')
        self.assertIn("cargo test --workspace --exclude island_ui", check_job)

    def test_ci_keeps_incremental_compilation_disabled(self) -> None:
        workflow = Path(__file__).resolve().parents[2] / ".github" / "workflows" / "ci.yml"
        text = workflow.read_text(encoding="utf-8")
        self.assertRegex(text, r'(?m)^\s+CARGO_INCREMENTAL:\s*["\']?0["\']?\s*$')


if __name__ == "__main__":
    unittest.main()
