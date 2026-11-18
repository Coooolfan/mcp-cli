#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
REPOSITORY='Coooolfan/mcp-cli'

usage() {
    echo "Usage: $0 <release-tag|--select> <download-base-url> [output-directory]"
    echo 'Use --select to choose from the latest 100 published releases (including prereleases).'
    echo 'Upload the binaries to <download-base-url>/<version>/ separately.'
    echo 'Downloads six native binaries and prints MCP_CLI_VERSION and MCP_CLI_BASE_URL.'
    echo 'Requires gh and python3. Does not compile or publish anything.'
}
if [[ "${1:-}" == --help || "${1:-}" == -h ]]; then usage; exit 0; fi
if [[ $# -lt 2 || $# -gt 3 ]]; then usage >&2; exit 2; fi
for tool in gh python3; do
    command -v "$tool" >/dev/null 2>&1 || { echo "Required command unavailable: $tool" >&2; exit 1; }
done

download_base_url="$(python3 - "$2" <<'PYBASE'
import sys
from urllib.parse import urlsplit
value = sys.argv[1]
try:
    url = urlsplit(value)
    url.port  # Validate an explicitly supplied port
    valid = (value.startswith("https://") and url.hostname and url.username is None and url.password is None
             and "?" not in value and "#" not in value
             and not any(character.isspace() or ord(character) < 32 for character in value)
             and not any(character in value for character in "\\\"'"))
except ValueError:
    valid = False
if not valid:
    raise SystemExit("Download base URL must be HTTPS without credentials, query, fragment, spaces or quotes")
print(value.rstrip("/"))
PYBASE
)"
release_tag="$1"
if [[ "$release_tag" == --select ]]; then
    release_tag=''
    candidates="$(gh release list --repo "$REPOSITORY" --exclude-drafts --limit 100 --json tagName --jq '.[].tagName')"
    if [[ -z "$candidates" ]]; then echo 'No published releases found' >&2; exit 1; fi
    tags=()
    while IFS= read -r candidate; do tags+=("$candidate"); done <<< "$candidates"
    PS3='Select a release number: '
    select candidate in "${tags[@]}"; do
        if [[ -n "$candidate" ]]; then release_tag="$candidate"; break; fi
        echo 'Invalid selection' >&2
    done
    if [[ -z "$release_tag" ]]; then echo 'No release selected' >&2; exit 1; fi
fi
semver='^v?(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-(alpha|beta|rc)\.(0|[1-9][0-9]*))?$'
if [[ ! "$release_tag" =~ $semver ]]; then echo 'Invalid release tag' >&2; exit 1; fi
is_draft="$(gh release view "$release_tag" --repo "$REPOSITORY" --json isDraft --jq '.isDraft')"
if [[ "$is_draft" != false ]]; then echo 'Only published releases can be packaged' >&2; exit 1; fi

release_version="${release_tag#v}"
output_directory="${3:-${ROOT_DIR}/dist/${release_version}}"
if [[ -e "$output_directory" || -L "$output_directory" ]]; then
    echo "Output already exists; choose a new directory: $output_directory" >&2
    exit 1
fi
mkdir -p -- "$(dirname -- "$output_directory")"
staging="$(mktemp -d "$(dirname -- "$output_directory")/.mcp-package.XXXXXX")"
trap 'rm -rf -- "$staging"' EXIT
trap 'exit 1' HUP INT TERM
assets=(mcp-cli-darwin-x64 mcp-cli-darwin-arm64 mcp-cli-linux-x64 mcp-cli-linux-arm64
    mcp-cli-win-x64.exe mcp-cli-win-arm64.exe)
patterns=()
for asset in "${assets[@]}"; do patterns+=(--pattern "$asset"); done
gh release download "$release_tag" --repo "$REPOSITORY" --dir "$staging" "${patterns[@]}"

for asset in "${assets[@]}"; do
    if [[ ! -f "$staging/$asset" || ! -s "$staging/$asset" ]]; then
        echo "Missing or empty release binary: $asset" >&2
        exit 1
    fi
done
mv -- "$staging" "$output_directory"
trap - EXIT HUP INT TERM
echo "Packaged release ${release_tag}: ${output_directory}"
echo "Upload the binaries to ${download_base_url}/${release_version}/, then set these values in /admin/system-config:"
printf 'MCP_CLI_VERSION=%s\nMCP_CLI_BASE_URL=%s\n' "$release_version" "$download_base_url"
