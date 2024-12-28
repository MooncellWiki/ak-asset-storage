_default:
  @just --list -u

init:
    cargo install cargo-release git-cliff sqlx-cli

up:
    sqlx migrate run

pre-release version:
    git cliff -o CHANGELOG.md --tag {{version}} && git add CHANGELOG.md
