# Contributing to Arknights Asset Storage

Thank you for your interest in contributing to the Arknights Asset Storage project!

## Prerequisites

- Rust (latest stable)
- Node.js (v20 or higher) and pnpm
- `just`
- A local single-node k3s with `kubectl` (dev PostgreSQL and RustFS run there)

What runs where, the generated config and manifests, the extraction Job, and the
dev/e2e isolation model are documented in [DEVELOPMENT.md](DEVELOPMENT.md).

## Setup

```bash
cargo install cargo-binstall && cargo binstall just -y   # or install just your way
just init          # dev Cargo tools + init-env: generates tmp/config.toml,
                   # prepares tmp/rustfs-data and the upload bucket
pnpm install
just k3s-apply     # PostgreSQL (deploy/k3s/dev) + RustFS (generated manifest)
just up            # sqlx migrate run
```

## Daily Workflow

```bash
cargo run --bin ak-asset-storage -- server -c tmp/config.toml   # API on :5150
cargo run --bin ak-asset-storage -- worker -c tmp/config.toml
pnpm dev                                                         # frontend on :25173, proxies /api → :5150
```

More day-to-day commands (debug binaries, worker concurrency, e2e runs) are in
DEVELOPMENT.md under 常用命令.

## Verification

Pre-commit runs fmt/clippy/eslint on staged files via lint-staged. The full set
CI enforces:

```bash
cargo fmt --all -- --check
cargo clippy --all-features -- -D warnings
cargo test --all-features -- --nocapture
pnpm typecheck
pnpm lint
```

`pnpm build` is required before cargo build/clippy/test — rust-embed embeds the
frontend build output into the binary.

## Database

Migrations live in `migrations/`:

```bash
sqlx migrate add <name>   # create a new migration
sqlx migrate run          # apply (same as `just up`)
```

- Always use the `sqlx::query!` / `query_as!` / `query_scalar!` macros, never
  the non-macro `sqlx::query` / `query_as` functions — the macros verify queries
  at compile time.
- Never edit the `.sqlx/` directory.
- Do not use `SQLX_OFFLINE=true` for local `cargo check` / `cargo build`; run
  `sqlx migrate run` first and let sqlx verify against the live database. If it
  cannot connect, fix the database instead of falling back to offline mode.

## Project Structure

See [README.md](README.md) for the module layout of the Rust backend and the
frontend location.

## Code Style

- Follow existing Rust code style (rustfmt)
- Follow eslint in frontend
- Write tests for new features
- Update documentation as needed
