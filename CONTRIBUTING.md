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

Run `just dev` to build and start the API and frontend in two tmux panes.
The worker is optional; start it separately with the command below when needed.
Detach with `Ctrl-b d`, reconnect with `just dev`, and stop the session's services
with `just dev-stop`. This requires tmux and the setup above.

Alternatively, run each service separately:

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

The e2e job checks SQL macros against a fresh PostgreSQL service after applying
the migrations. The other CI jobs and the Docker build compile with
`SQLX_OFFLINE=true` against the checked-in `.sqlx/` cache, so regenerate and
commit it whenever queries or the schema change:

```bash
sqlx migrate run
cargo sqlx prepare -- --locked --all-targets --all-features
```

Launcher tests cover protocol decoding, publication identity, manifest
transactions, diffs, content reuse, and interrupted claims:

```bash
cargo test --locked --test launcher
# Uses the local development S3 bucket plus an isolated SQLx test database:
cargo test --locked --test launcher archives_raw_bytes -- --ignored --nocapture
# Optional: reads only the live official API and game_files, not game binaries:
cargo test --locked --test launcher validates_live -- --ignored --nocapture
```

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
