#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=.github/scripts/release/release-state.sh
source "$(dirname "$0")/release-state.sh"

git fetch --force --tags origin
existing_sha="$(git rev-parse --verify --quiet "refs/tags/${VERSION}^{commit}" || true)"
if [[ -n "$existing_sha" && "$existing_sha" != "$TARGET_SHA" ]]; then
  echo "::error::tag ${VERSION} 已指向其他提交 ${existing_sha}"
  exit 1
fi
release="$(release_state "$VERSION")"
if [[ -n "$release" ]]; then
  if [[ "$(jq -r .draft <<< "$release")" != true || "$(jq -r .prerelease <<< "$release")" != "$PRERELEASE" ]]; then
    echo '::error::仅能复用预发布类型一致的草稿 Release'
    exit 1
  fi
  if [[ "$existing_sha" != "$TARGET_SHA" ]]; then
    echo '::error::现有草稿的 tag 必须指向本次目标提交'
    exit 1
  fi
else
  if [[ -z "$existing_sha" ]]; then
    git config user.name 'github-actions[bot]'
    git config user.email '41898282+github-actions[bot]@users.noreply.github.com'
    git tag -a "$VERSION" "$TARGET_SHA" -m "Release $VERSION"
    git push origin "refs/tags/$VERSION"
  fi
  args=(--draft --verify-tag --target "$TARGET_SHA" --title "$VERSION"
    --notes-file "docs/release_notes/${VERSION}.md")
  [[ "$PRERELEASE" != true ]] || args+=(--prerelease)
  gh release create "$VERSION" "${args[@]}"
fi
# Refresh notes when retrying a draft after a transient build/upload failure.
gh release edit "$VERSION" --notes-file "docs/release_notes/${VERSION}.md"
