from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest


WORKFLOW = Path(__file__).resolve().parents[1] / "workflows" / "_close-jira.yaml"


class FindJiraKeyTest(unittest.TestCase):
    def run_lookup(self, branch, comments="", gh_exit=0):
        lines = WORKFLOW.read_text().splitlines()
        step = lines.index("      - name: Find Jira key")
        start = lines.index("        run: |", step) + 1
        end = start
        while end < len(lines) and lines[end].startswith("          "):
            end += 1
        script = textwrap.dedent("\n".join(lines[start:end]))

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            gh = root / "gh"
            gh.write_text('#!/bin/bash\nprintf "%s\\n" "$COMMENTS"\nexit "$GH_EXIT"\n')
            gh.chmod(0o755)
            output = root / "output"
            env = {
                "PATH": f"{root}:/usr/bin:/bin",
                "HOME": directory,
                "REF": branch,
                "URL": "https://github.com/example/repo/pull/1",
                "GITHUB_OUTPUT": str(output),
                "COMMENTS": comments,
                "GH_EXIT": str(gh_exit),
            }
            result = subprocess.run(
                ["/bin/bash", "-e", "-c", script],
                env=env,
                capture_output=True,
                text=True,
            )
            return result, output.read_text() if output.exists() else ""

    def test_branch_without_comments(self):
        result, output = self.run_lookup("CLOUDP-457446", gh_exit=1)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "key=CLOUDP-457446\n")

    def test_branch_with_description(self):
        result, output = self.run_lookup("CLOUDP-457446-update-commands")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "key=CLOUDP-457446\n")

    def test_tracking_comment_fallback(self):
        comments = (
            "Unrelated CLOUDP-1\n"
            "The ticket [CLOUDP-454829](https://jira.mongodb.org/browse/CLOUDP-454829) "
            "was created for internal tracking.\n"
            "The ticket CLOUDP-2 was created for internal tracking."
        )
        result, output = self.run_lookup("dependabot/go_modules/update", comments)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "key=CLOUDP-454829\n")

    def test_missing_key_fails_before_emitting_output(self):
        result, output = self.run_lookup("update-commands", "Unrelated CLOUDP-1")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("::error::No Jira key found", result.stderr)
        self.assertEqual(output, "")

    def test_github_failure_is_not_swallowed(self):
        result, output = self.run_lookup("dependabot/update", gh_exit=7)
        self.assertEqual(result.returncode, 7)
        self.assertEqual(output, "")


if __name__ == "__main__":
    unittest.main()
