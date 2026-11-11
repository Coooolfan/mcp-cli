"""Exercise release guards against an isolated local git remote; never contact GitHub."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / ".github/scripts/release"


class ReleaseTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="mcp-release-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.env = dict(os.environ, PATH=str(self.bin) + os.pathsep + os.environ["PATH"],
                        INPUT_VERSION="0.3.0", INPUT_LATEST="false",
                        GITHUB_OUTPUT=str(self.root / "output"),
                        GITHUB_STEP_SUMMARY=str(self.root / "summary"),
                        GITHUB_REPOSITORY="example/mcp-cli", GH_RESPONSE="",
                        GH_LOG=str(self.root / "gh.log"), GH_FAIL="false",
                        GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM="1",
                        GIT_TERMINAL_PROMPT="0")
        self.git("init", "--bare", str(self.root / "remote"))
        self.git("init", "-b", "main")
        self.git("config", "user.name", "Release test")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        self.git("config", "tag.gpgsign", "false")
        notes = self.repo / "docs/release_notes"
        notes.mkdir(parents=True)
        (notes / "0.3.0.md").write_text("## feat\n* Test release\n")
        self.git("add", ".")
        self.git("commit", "-m", "test release")
        self.git("remote", "add", "origin", str(self.root / "remote"))
        self.git("push", "origin", "main")
        gh = self.bin / "gh"
        gh.write_text("""#!/usr/bin/env python3
import json, os, sys
if sys.argv[1] == 'api':
    if os.environ['GH_FAIL'] == 'true':
        sys.exit(1)
    print(os.environ['GH_RESPONSE'])
else:
    with open(os.environ['GH_LOG'], 'a') as log:
        log.write(json.dumps(sys.argv[1:]) + '\\n')
""")
        gh.chmod(0o755)

    def git(self, *args):
        return subprocess.run(["git", *args], cwd=self.repo, env=self.env,
                              check=True, capture_output=True, text=True).stdout.strip()

    def run_script(self, name, **env):
        return subprocess.run(["bash", str(SCRIPTS / name)], cwd=self.repo,
                              env=dict(self.env, **env), capture_output=True, text=True)

    def tag(self, name="0.3.0"):
        self.git("tag", name)
        self.git("push", "origin", "refs/tags/" + name)

    def state(self, draft, prerelease=False):
        self.env["GH_RESPONSE"] = json.dumps({"tag_name": "0.3.0", "draft": draft,
                                              "prerelease": prerelease})

    def test_stable_latest_outputs_exact_commit(self):
        result = self.run_script("prepare-release.sh", INPUT_LATEST="true")
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        output = Path(self.env["GITHUB_OUTPUT"]).read_text()
        self.assertIn("target_sha=" + self.git("rev-parse", "HEAD"), output)
        self.assertIn("publish_latest=true", output)
        self.assertIn("prerelease=false", output)

    def test_rejects_invalid_and_injected_versions(self):
        for version in ["v0.3.0", "01.2.3", "0.3.0-rc.01", "0.3.0-rc-1", "0.3.0; touch injected"]:
            with self.subTest(version=version):
                self.assertNotEqual(self.run_script("prepare-release.sh", INPUT_VERSION=version).returncode, 0)
        self.assertFalse((self.repo / "injected").exists())

    def test_prerelease_cannot_be_latest(self):
        result = self.run_script("prepare-release.sh", INPUT_VERSION="0.3.0-rc.1", INPUT_LATEST="true")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Latest", result.stdout)

    def test_missing_notes_block_release(self):
        result = self.run_script("prepare-release.sh", INPUT_VERSION="0.4.0")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("docs/release_notes/0.4.0.md", result.stdout)

    def test_published_release_cannot_be_overwritten(self):
        self.tag()
        self.state(draft=False)
        self.assertNotEqual(self.run_script("prepare-release.sh").returncode, 0)
        self.assertNotEqual(self.run_script("create-draft-release.sh", VERSION="0.3.0",
                                          TARGET_SHA=self.git("rev-parse", "HEAD"), PRERELEASE="false").returncode, 0)
        self.assertFalse(Path(self.env["GH_LOG"]).exists())

    def test_draft_retry_requires_same_commit(self):
        self.tag()
        self.state(draft=True)
        self.assertEqual(self.run_script("prepare-release.sh").returncode, 0)
        self.git("commit", "--allow-empty", "-m", "different commit")
        self.assertNotEqual(self.run_script("prepare-release.sh").returncode, 0)

    def test_latest_requires_main_head(self):
        self.git("checkout", "-b", "patch")
        self.git("commit", "--allow-empty", "-m", "patch")
        self.assertNotEqual(self.run_script("prepare-release.sh", INPUT_LATEST="true").returncode, 0)
        self.assertEqual(self.run_script("prepare-release.sh").returncode, 0)

    def test_api_errors_abort_preflight(self):
        self.assertNotEqual(self.run_script("prepare-release.sh", GH_FAIL="true").returncode, 0)
        self.assertFalse(Path(self.env["GITHUB_OUTPUT"]).exists())

    def test_creates_tag_on_exact_commit_and_draft_with_notes(self):
        sha = self.git("rev-parse", "HEAD")
        result = self.run_script("create-draft-release.sh", VERSION="0.3.0", TARGET_SHA=sha, PRERELEASE="false")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.git("rev-parse", "0.3.0^{commit}"), sha)
        operations = [json.loads(line) for line in Path(self.env["GH_LOG"]).read_text().splitlines()]
        self.assertEqual(operations[0][:3], ["release", "create", "0.3.0"])
        self.assertIn("--draft", operations[0])
        self.assertIn("docs/release_notes/0.3.0.md", operations[0])

    def test_note_skill_paginates_and_recognizes_legacy_tags(self):
        for name in ["v0.2.0", "0.3.0-rc.1", "0.3.0-beta.2"]:
            self.git("tag", name)
        script = ROOT / ".agents/skills/release-note-creator/scripts/git-tags.sh"
        def page(offset):
            result = subprocess.run(["bash", str(script), "--pagesize", "2", "--offset", str(offset)],
                                    cwd=self.repo, env=self.env, check=True, capture_output=True, text=True)
            return [line.split()[0] for line in result.stdout.splitlines()[1:]]
        self.assertEqual(page(0), ["0.3.0-rc.1", "0.3.0-beta.2"])
        self.assertEqual(page(2), ["v0.2.0"])

    def test_version_stamp_keeps_dependency_lock_and_cargo_agreement(self):
        spec = importlib.util.spec_from_file_location("release_version", SCRIPTS / "set-version.py")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        for filename in ["Cargo.toml", "Cargo.lock"]:
            (self.repo / filename).write_bytes((ROOT / filename).read_bytes())
        before = tomllib.loads((self.repo / "Cargo.lock").read_text())
        module.set_version(self.repo, "0.3.0-rc.1")
        manifest = tomllib.loads((self.repo / "Cargo.toml").read_text())
        after = tomllib.loads((self.repo / "Cargo.lock").read_text())
        self.assertEqual(manifest["package"]["version"], "0.3.0-rc.1")
        self.assertEqual([p for p in before["package"] if p["name"] != "mcp-cli"],
                         [p for p in after["package"] if p["name"] != "mcp-cli"])
        src = self.repo / "src"
        src.mkdir()
        (src / "lib.rs").write_text("")
        (src / "main.rs").write_text("fn main() {}")
        metadata = subprocess.run(["cargo", "metadata", "--locked", "--offline", "--no-deps", "--format-version=1"],
                                  cwd=self.repo, capture_output=True, text=True, check=True)
        self.assertEqual(json.loads(metadata.stdout)["packages"][0]["version"], "0.3.0-rc.1")
        # Invalid versions cannot partially rewrite either file.
        files = [(self.repo / name).read_bytes() for name in ["Cargo.toml", "Cargo.lock"]]
        with self.assertRaises(ValueError):
            module.set_version(self.repo, "v0.4.0")
        self.assertEqual(files, [(self.repo / name).read_bytes() for name in ["Cargo.toml", "Cargo.lock"]])


if __name__ == "__main__":
    unittest.main()
