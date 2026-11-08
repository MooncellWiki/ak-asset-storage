#!/usr/bin/env bash
# Upload an exported frontend build's assets to OSS.
#
# Usage: upload-frontend.sh <dist dir> <CDN base URL> <oss://bucket/prefix/>
# ossutil reads OSS_ACCESS_KEY_ID, OSS_ACCESS_KEY_SECRET, OSS_REGION and
# OSS_ENDPOINT from the environment.
set -euo pipefail

dist=$1
cdn=$2
dest=$3

: "${OSS_ACCESS_KEY_ID:?Set the OSS_ACCESS_KEY_ID repository secret}"
: "${OSS_ACCESS_KEY_SECRET:?Set the OSS_ACCESS_KEY_SECRET repository secret}"
: "${OSS_REGION:?Set the OSS_REGION repository secret}"
# ossutil takes a region ID (cn-hangzhou); ali-oss style values add "oss-".
export OSS_REGION=${OSS_REGION#oss-}

# Existing objects are skipped and everything is cached for a year, which is
# only safe when each file name carries a content hash.
unhashed=$(find "$dist/fe" -type f | grep -vE -- '-[A-Za-z0-9_-]{8}\.[A-Za-z0-9]+$' || true)
if [[ -n $unhashed ]]; then
  echo "::error::Build assets without a content hash in their name:"
  echo "$unhashed"
  exit 1
fi

entry=$(grep -oE '<script type="module"[^>]* src="[^"]+"' "$dist/index.html" \
  | sed -E 's/.* src="([^"]+)"$/\1/' | head -n1)
if [[ $entry != "$cdn"* ]] || grep -q '"/fe/' "$dist/index.html"; then
  echo "::error::index.html does not load its assets from $cdn; was AK_FE_CDN set for the build?"
  exit 1
fi

# Nothing is deleted: tabs opened on earlier releases still load their chunks.
ossutil cp "$dist/fe/" "$dest" --recursive --ignore-existing \
  --cache-control "public, max-age=31536000, immutable" --job 16 --no-progress
