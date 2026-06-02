# AGENTS.md

## Commands

```bash
# Lint/format checks (pre-commit runs these via lint-staged)
cargo fmt --all -- --check
cargo clippy --all-features -- -D warnings   # CI fails on warnings
pnpm lint                                     # ESLint for frontend

# Typecheck
pnpm typecheck                                # vue-tsc --noEmit

# Test
cargo test --all-features -- --nocapture      # CI uses this exact invocation

# Build frontend (required before cargo build/clippy/test for rust-embed)
pnpm build
```

After editing `.rs` files, run `cargo check`. For full verification: `cargo fmt`, `cargo clippy`, then `cargo test`.

## Architecture

Rust workspace (edition 2024, toolchain 1.92.0) with 4 crates:

- `crates/application` — entities, ports (traits), services, DTOs. Pure domain logic, no infrastructure deps.
- `crates/infrastructure` — PostgreSQL repos, S3, SMTP, HTTP clients. Depends on `application`.
- `crates/web` — Axum HTTP handlers/routes, embeds frontend via `rust-embed`. Depends on `application` + `infrastructure`.
- `crates/cli` — binary entrypoint (`ak-asset-storage`). Commands: `server`, `worker`, `seed`, `import-manifest`.

Dependency direction: `cli → web → application ← infrastructure`. The web crate should not directly depend on infrastructure; uses DI.

Config is TOML-based (see `example.toml`), not env-only.

## Frontend

Vue 3 + TypeScript + Naive UI. Lives in `app/` at repo root (not `src/`).

- File-based routing: `app/pages/` → routes via `unplugin-vue-router`
- Path alias: `~/` → `app/`
- UnoCSS, unplugin-icons, auto-imported Naive UI components
- Dev server on port 25173, proxies `/api` → `localhost:5150`
- `pnpm api` regenerates `app/common/schema.d.ts` from OpenAPI endpoint (requires running backend)

## Build & Deploy

Docker builds frontend first (`pnpm build` → `dist/`), then Rust binary with `SQLX_OFFLINE=true`. Release is tag-triggered → Docker image to GHCR.

## Database

PostgreSQL via `sqlx`. Migrations in `migrations/`.

```bash
docker-compose up -d      # PostgreSQL on :25432, RustFS (S3) on :9000/:9001
sqlx migrate run           # Run migrations
sqlx migrate add <name>    # Create new migration
```

**Connection:**

- URL: `postgres://ak:ak@localhost:25432/ak_asset_storage_next`
- Docker container: `ak-asset-storage-db-1`

**Direct SQL via docker exec:**

```bash
# Connect interactively
docker exec -it ak-asset-storage-db-1 psql -U ak -d ak_asset_storage_next

# Run a query
docker exec -i ak-asset-storage-db-1 psql -U ak -d ak_asset_storage_next -c "SELECT * FROM versions LIMIT 5;"
```

**Common tables:**

- `versions` — game resource versions
- `asset_to_bundle_mappings` — manifest entries (asset → bundle path, dir, node_type)

**Important rules:**

- **Never edit the `.sqlx/` directory.** Leave it as-is.
- **Do not use `SQLX_OFFLINE=true`** when running `cargo check` or `cargo build`. Instead, run `sqlx migrate run` first to ensure migrations are applied to the running database, then run `cargo check` or `cargo build` directly so sqlx can verify queries against the live database.
- If `cargo check` or `cargo build` fails with a database connection error, **stop and ask the user to resolve it** — do not fall back to `SQLX_OFFLINE=true`.

## Clippy

Workspace uses pedantic + nursery lints with select allows (`missing_errors_doc`, `missing_panics_doc`, `module_name_repetitions`). Run with `--all-features`.

## Testing

All tests are in `crates/application/tests/`:

- `unit/` — mocked repos and services
- `integration/` — workflow tests with mocked externals
- `common/mocks/` — shared mock implementations and test data
