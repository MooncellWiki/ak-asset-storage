#!/usr/bin/env bash
# Check that a debug file exported from the Docker build can symbolicate the
# binary inside a runtime image.
#
# Usage: check-sentry-debug-file.sh <image> <debug-file>
set -euo pipefail

image=$1
debug_file=$2

binary=$(mktemp)
trap 'rm -f "$binary"' EXIT
container=$(docker create --quiet "$image" /app/ak-asset-storage)
docker cp "$container:/app/ak-asset-storage" "$binary"
docker rm "$container" >/dev/null

binary_info=$(sentry-cli debug-files check --json "$binary")
debug_info=$(sentry-cli debug-files check --json "$debug_file")

# The SDK reports the executable to Sentry only when it has a GNU build ID.
if [[ -z "$(jq -r '.variants[0].code_id // empty' <<<"$binary_info")" ]]; then
  echo "::error::The binary in $image has no GNU build ID, so Sentry cannot match debug files to it"
  exit 1
fi

binary_id=$(jq -r '.variants[0].debug_id' <<<"$binary_info")
debug_id=$(jq -r '.variants[0].debug_id' <<<"$debug_info")
if [[ "$binary_id" != "$debug_id" ]]; then
  echo "::error::$debug_file has debug ID $debug_id, but the binary in $image has $binary_id"
  exit 1
fi

if ! jq -e '.features | split(", ") | index("debug")' <<<"$debug_info" >/dev/null; then
  echo "::error::$debug_file contains no DWARF debug information"
  exit 1
fi

echo "Debug ID $debug_id matches the binary in $image ($(jq -r .features <<<"$debug_info"))"
