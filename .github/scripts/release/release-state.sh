#!/usr/bin/env bash
# Source this file from a release job with GH_TOKEN and GITHUB_REPOSITORY set.
# Listing releases distinguishes a missing release from an API/authentication error.
release_state() {
  gh api --paginate "repos/${GITHUB_REPOSITORY}/releases" \
    --jq '.[] | {tag_name, draft, prerelease}' \
    | jq -sc --arg tag "$1" 'map(select(.tag_name == $tag)) | .[0] // empty'
}
