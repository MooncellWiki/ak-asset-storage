_default:
  @just --list -u

init: init-env
    cargo binstall cargo-release git-cliff
    cargo install sqlx-cli rustfs-cli

# one-time dev environment prep: directories, ownership, backup exclusion, bucket
init-env:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p tmp/asset/gamedata tmp/asset/raw
    mkdir -p tmp/rustfs-data
    # the rustfs container writes as uid 10001 and bind mounts do not copy
    # ownership from the image, so the host directory must be prepared
    sudo chown -R 10001:10001 tmp/rustfs-data
    if command -v getenforce >/dev/null 2>&1 && [ "$(getenforce)" = "Enforcing" ]; then
        sudo chcon -Rt container_file_t tmp/rustfs-data
    fi
    # the bucket must exist before the first upload; the code never creates it
    if command -v rc >/dev/null 2>&1 && curl -sf -m 3 http://127.0.0.1:9000/health >/dev/null; then
        rc alias set rustfs http://127.0.0.1:9000 torappu torappu123 >/dev/null
        rc bucket create --ignore-existing rustfs/arknights-assets
    else
        echo "note: rustfs not reachable; after 'docker compose up -d' run 'rc bucket create --ignore-existing rustfs/arknights-assets'"
    fi

up:
    sqlx migrate run

pre-release version:
    git cliff -o CHANGELOG.md --tag {{version}} && git add CHANGELOG.md

e2e:
    cargo test --test e2e -- --ignored --nocapture --test-threads=1
