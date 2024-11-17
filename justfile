_default:
  @just --list -u

init:
    cargo install sea-orm-cli cargo-release git-cliff
up:
    sea-orm-cli migrate up -d ./migration/

entity:
    sea-orm-cli generate entity --model-extra-derives utoipa::ToSchema --with-serde both --output-dir src/models/_entities/
