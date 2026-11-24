#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
REPOSITORY='Coooolfan/mcp-cli'

usage() {
    echo "Usage: $0 <release-tag|--select> [output-directory]"
    echo 'Use --select to choose from the latest 100 published releases (including prereleases).'
    echo 'Upload the binaries to <download-base-url>/<version>/ separately.'
    echo 'Downloads six native binaries and prints MCP_CLI_VERSION.'
    echo 'Requires gh. Does not compile or publish anything.'
}
if [[ "${1:-}" == --help || "${1:-}" == -h ]]; then usage; exit 0; fi
if [[ $# -lt 1 || $# -gt 2 ]]; then usage >&2; exit 2; fi
command -v gh >/dev/null 2>&1 || { echo "Required command unavailable: gh" >&2; exit 1; }

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
output_directory="${2:-${ROOT_DIR}/dist/${release_version}}"
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
echo "Upload the binaries to <download-base-url>/${release_version}/, then configure MCP_CLI_BASE_URL in /admin/system-config."
printf 'MCP_CLI_VERSION=%s\n' "$release_version"
