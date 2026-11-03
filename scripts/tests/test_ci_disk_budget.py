"""Protect Rust fast-test CI from exhausting GitHub-hosted runner storage."""
from pathlib import Path
import unittest

WORKFLOW = Path(__file__).resolve().parents[2] / ".github/workflows/ci.yml"


class FastTestResourceBudget(unittest.TestCase):
    def test_rust_check_job_has_bounded_artifacts(self):
        workflow = WORKFLOW.read_text()
        check = workflow.split("\n  check:\n", 1)[1].split("\n  desktop:\n", 1)[0]
        self.assertIn('CARGO_PROFILE_TEST_DEBUG: "0"', check)
        self.assertIn('CARGO_PROFILE_DEV_DEBUG: "0"', check)
        self.assertIn('CARGO_BUILD_JOBS: "2"', check)
        self.assertIn("cargo test --workspace --exclude island_ui", check)


if __name__ == "__main__":
    unittest.main()
