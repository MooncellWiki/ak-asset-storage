#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
REPO_DIR="${ROOT_DIR}"
VERSIONS=(
  "26-05-20-12-59-09_e8f456"
  "26-05-27-13-32-37_d44f28"
)

FILES=(
  "refs_fx_texture_overseas_char_1037_amiya3_sale__13.dat"
  "refs_fx_texture_overseas_live2d_dyn_char_1037_amiya3_sale__13.dat"
  "arts_avg_shader_profile.dat"
)

for VERSION in "${VERSIONS[@]}"; do
  BASE_URL="https://ak.hycdn.cn/assetbundle/official/Android/assets/${VERSION}"
  TARGET_DIR="${REPO_DIR}/e2e/fixtures/upstream/${VERSION}"
  mkdir -p "${TARGET_DIR}"

  for FILE in "${FILES[@]}"; do
    curl -fsSL "${BASE_URL}/${FILE}" -o "${TARGET_DIR}/${FILE}"
  done
done
