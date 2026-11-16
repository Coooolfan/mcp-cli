"""Run package.sh against fake GitHub releases; do not download or execute binaries."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
ASSETS = {
    "darwin-amd64": "mcp-cli-darwin-x64",
    "darwin-arm64": "mcp-cli-darwin-arm64",
    "linux-amd64": "mcp-cli-linux-x64",
    "linux-arm64": "mcp-cli-linux-arm64",
    "windows-amd64": "mcp-cli-win-x64.exe",
    "windows-arm64": "mcp-cli-win-arm64.exe",
}


class PackageTest(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="mcp-package-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.script = self.root / "package.sh"
        shutil.copy2(ROOT / "package.sh", self.script)
        tools = self.root / "bin"
        tools.mkdir()
        (tools / "gh").write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
args = sys.argv[1:]
with open(os.environ['GH_LOG'], 'a') as log:
    log.write(json.dumps(args) + '\\n')
if args[:2] == ['release', 'list']:
    print(os.environ.get('GH_CANDIDATES', '0.3.0-rc.1\\nv0.2.0'))
elif args[:2] == ['release', 'view']:
    print(os.environ.get('GH_DRAFT', 'false'))
elif args[:2] == ['release', 'download']:
    folder = pathlib.Path(args[args.index('--dir') + 1])
    for index, arg in enumerate(args):
        if arg == '--pattern':
            name = args[index + 1]
            if name == os.environ.get('GH_MISSING'): continue
            content = b'' if name == os.environ.get('GH_EMPTY') else ('native fixture ' + name).encode()
            (folder / name).write_bytes(content)
    if os.environ.get('GH_FAIL') == 'true': sys.exit(1)
else:
    sys.exit('Unexpected GitHub command')
''')
        (tools / "gh").chmod(0o755)
        self.env = dict(os.environ, PATH=str(tools) + os.pathsep + os.environ["PATH"],
                        GH_LOG=str(self.root / "gh.log"))
        for key in ["GH_MISSING", "GH_EMPTY", "GH_FAIL", "GH_DRAFT", "GH_CANDIDATES"]:
            self.env.pop(key, None)
        self.output = self.root / "output directory"

    def run_package(self, *args, input="", **env):
        return subprocess.run(["bash", str(self.script), *args], cwd=self.root,
                              env=dict(self.env, **env), input=input, capture_output=True, text=True)

    def test_downloads_six_native_assets_and_generates_matching_manifest(self):
        result = self.run_package("0.3.0-rc.1", str(self.output))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        manifest = json.loads((self.output / "manifest.json").read_text())
        self.assertEqual(manifest["version"], "0.3.0-rc.1")
        self.assertEqual(set(manifest["artifacts"]), set(ASSETS))
        for platform, name in ASSETS.items():
            self.assertEqual(manifest["artifacts"][platform], {
                "url": f"https://github.com/Coooolfan/mcp-cli/releases/download/0.3.0-rc.1/{name}",
                "sha256": hashlib.sha256((self.output / name).read_bytes()).hexdigest(),
            })
        self.assertEqual(len((self.output / "checksums.txt").read_text().splitlines()), 6)
        self.assertFalse(list(self.root.glob(".mcp-package.*")))

    def test_selects_from_release_candidates_and_preserves_legacy_tag_in_urls(self):
        result = self.run_package(input="9\n2\n")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        manifest = json.loads((self.root / "dist/v0.2.0/manifest.json").read_text())
        self.assertEqual(manifest["version"], "0.2.0")
        self.assertIn("/v0.2.0/", manifest["artifacts"]["windows-amd64"]["url"])
        self.assertIn("Invalid selection", result.stderr)

    def test_incomplete_empty_or_failed_download_leaves_no_package(self):
        for override in [{"GH_MISSING": "mcp-cli-win-x64.exe"},
                         {"GH_EMPTY": "mcp-cli-linux-x64"}, {"GH_FAIL": "true"}]:
            with self.subTest(override=override):
                result = self.run_package("0.3.0", str(self.output), **override)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(self.output.exists())
                self.assertFalse(list(self.root.glob(".mcp-package.*")))

    def test_does_not_overwrite_existing_directory(self):
        self.output.mkdir()
        marker = self.output / "manifest.json"
        marker.write_text("existing")
        result = self.run_package("0.3.0", str(self.output))
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(marker.read_text(), "existing")
        self.assertNotIn('"download"', Path(self.env["GH_LOG"]).read_text())

    def test_rejects_invalid_tags_and_drafts(self):
        for tag in ["../0.3.0", "0.3.0;touch injected", "01.2.3"]:
            self.assertNotEqual(self.run_package(tag, str(self.output)).returncode, 0)
        self.assertFalse(Path(self.env["GH_LOG"]).exists())
        self.assertNotEqual(self.run_package("0.3.0", str(self.output), GH_DRAFT="true").returncode, 0)
        self.assertFalse(self.output.exists())
        self.assertFalse((self.root / "injected").exists())

    def test_no_candidates_or_no_selection_does_not_default_to_latest(self):
        self.assertNotEqual(self.run_package(GH_CANDIDATES="").returncode, 0)
        self.assertNotEqual(self.run_package(input="").returncode, 0)
        self.assertFalse((self.root / "dist").exists())


if __name__ == "__main__":
    unittest.main()
