_default:
  @just --list -u

init:
    cargo binstall cargo-release git-cliff
    cargo install sqlx-cli rustfs-cli

up:
    sqlx migrate run

pre-release version:
    git cliff -o CHANGELOG.md --tag {{version}} && git add CHANGELOG.md

e2e:
    cargo test --test e2e -- --ignored --nocapture --test-threads=1
