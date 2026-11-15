"""Protect Rust fast-test CI from exhausting GitHub-hosted runner storage."""
from pathlib import Path
import unittest

WORKFLOW = Path(__file__).resolve().parents[2] / ".github/workflows/ci.yml"


class FastTestResourceBudget(unittest.TestCase):
    def test_rust_check_job_has_bounded_artifacts(self):
        workflow = WORKFLOW.read_text()
        check = workflow.split("\n  check:\n", 1)[1].split("\n  desktop:\n", 1)[0]
        # Modern CI defines these for every job; inheriting a global setting
        # protects the fast-test job as effectively as repeating it locally.
        global_env = workflow.split("\nenv:\n", 1)[1].split("\nconcurrency:\n", 1)[0]
        for key in ("CARGO_PROFILE_TEST_DEBUG", "CARGO_PROFILE_DEV_DEBUG"):
            self.assertTrue(
                f'{key}: "0"' in global_env or f'{key}: "0"' in check,
                f"{key} must disable debug artifacts globally or in the fast-test job",
            )
        self.assertIn('CARGO_BUILD_JOBS: "2"', check)
        self.assertIn("cargo test --workspace --exclude island_ui", check)


if __name__ == "__main__":
    unittest.main()
