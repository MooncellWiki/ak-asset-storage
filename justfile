_default:
  @just --list -u

init: init-env
    cargo binstall cargo-release git-cliff
    cargo install sqlx-cli rustfs-cli

# one-time dev environment prep: directories and the upload bucket
init-env:
    #!/usr/bin/env bash
    set -euo pipefail
    # k3s's kubectl can use /etc/rancher/k3s/k3s.yaml implicitly; kube-rs cannot.
    # Keep an explicit KUBECONFIG or an existing standard kubeconfig untouched.
    if [[ -z "${KUBECONFIG:-}" && ! -e "$HOME/.kube/config" && ! -L "$HOME/.kube/config" ]]; then
        mkdir -p "$HOME/.kube"
        kubeconfig_tmp="$(mktemp "$HOME/.kube/config.XXXXXX")"
        trap 'rm -f "$kubeconfig_tmp"' EXIT
        kubectl config view --raw --minify --flatten > "$kubeconfig_tmp"
        mv "$kubeconfig_tmp" "$HOME/.kube/config"
    fi
    mkdir -p tmp/asset/asset/gamedata tmp/asset/asset/raw
    # the bucket must exist before the first upload; the code never creates it
    # (RustFS runs in the local k3s via deploy/k3s/dev, NodePort 31000)
    if command -v rc >/dev/null 2>&1 && curl -sf -m 3 http://127.0.0.1:31000/health >/dev/null; then
        rc alias set rustfs http://127.0.0.1:31000 torappu torappu123 >/dev/null
        rc bucket create --ignore-existing rustfs/arknights-assets
    else
        echo "note: rustfs not reachable; after 'kubectl apply -k deploy/k3s/dev' run:"
        echo "  rc alias set rustfs http://127.0.0.1:31000 torappu torappu123"
        echo "  rc bucket create --ignore-existing rustfs/arknights-assets"
    fi

up:
    sqlx migrate run

pre-release version:
    git cliff -o CHANGELOG.md --tag {{version}} && git add CHANGELOG.md

e2e:
    cargo test --test e2e -- --ignored --nocapture --test-threads=1
