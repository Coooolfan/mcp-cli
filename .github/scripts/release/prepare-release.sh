#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=.github/scripts/release/release-state.sh
source "$(dirname "$0")/release-state.sh"

semver='^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-(alpha|beta|rc)\.(0|[1-9][0-9]*))?$'
if [[ ! "$INPUT_VERSION" =~ $semver ]]; then
  echo '::error::版本号必须为 1.2.3 或 1.2.3-alpha.1 / beta.1 / rc.1，不带 v 前缀'
  exit 1
fi
case "$INPUT_LATEST" in
  true | false) ;;
  *) echo '::error::latest 必须是 true 或 false'; exit 1 ;;
esac
prerelease=false
[[ "$INPUT_VERSION" != *-* ]] || prerelease=true
if [[ "$prerelease" == true && "$INPUT_LATEST" == true ]]; then
  echo '::error::预发布版本不能标记为 Latest'
  exit 1
fi
if [[ ! -s "docs/release_notes/${INPUT_VERSION}.md" ]]; then
  echo "::error::缺少非空发布说明 docs/release_notes/${INPUT_VERSION}.md"
  exit 1
fi

git fetch --force --tags origin
target_sha="$(git rev-parse --verify HEAD)"
existing_sha="$(git rev-parse --verify --quiet "refs/tags/${INPUT_VERSION}^{commit}" || true)"
if [[ -n "$existing_sha" && "$existing_sha" != "$target_sha" ]]; then
  echo "::error::tag ${INPUT_VERSION} 已指向其他提交 ${existing_sha}"
  exit 1
fi
release="$(release_state "$INPUT_VERSION")"
if [[ -n "$release" ]]; then
  if [[ "$(jq -r .draft <<< "$release")" != true ]]; then
    echo "::error::Release ${INPUT_VERSION} 已公开，不能覆盖"
    exit 1
  fi
  if [[ "$(jq -r .prerelease <<< "$release")" != "$prerelease" ]]; then
    echo '::error::现有草稿的预发布类型与输入不一致'
    exit 1
  fi
  if [[ "$existing_sha" != "$target_sha" ]]; then
    echo '::error::现有草稿必须有指向本次目标提交的 tag'
    exit 1
  fi
fi
if [[ "$INPUT_LATEST" == true ]]; then
  git fetch origin '+refs/heads/main:refs/remotes/origin/main'
  if [[ "$target_sha" != "$(git rev-parse --verify origin/main)" ]]; then
    echo '::error::Latest 只能从 origin/main 当前 HEAD 发布'
    exit 1
  fi
fi
{
  echo "version=$INPUT_VERSION"
  echo "target_sha=$target_sha"
  echo "prerelease=$prerelease"
  echo "publish_latest=$INPUT_LATEST"
} >> "$GITHUB_OUTPUT"
{
  echo '## mcp-cli Release'
  echo "- Version: \`$INPUT_VERSION\`"
  echo "- Target SHA: \`$target_sha\`"
  echo "- Prerelease: \`$prerelease\`"
  echo "- Latest: \`$INPUT_LATEST\`"
} >> "$GITHUB_STEP_SUMMARY"
