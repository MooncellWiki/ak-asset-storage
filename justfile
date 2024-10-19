_default:
  @just --list -u

init:
    cargo install sea-orm-cli
up:
    sea-orm-cli migrate up -d ./migration/

entity:
    sea-orm-cli generate entity --with-serde both --output-dir src/models/_entities/