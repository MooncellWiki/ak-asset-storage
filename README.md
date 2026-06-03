# Arknights Asset Storage

AK asset monitoring, storage, and torappu orchestration service.

## Rust Layout

Rust backend is a single crate organized by module:

- `src/api/` - Axum HTTP handlers, router, and API request/response types
- `src/database/` - PostgreSQL-only SQLx access behind `Database { pool: PgPool }`
- `src/external/` - concrete integrations for AK API, S3, SMTP, Docker, GitHub, and torappu assets
- `src/service/` - shared workflows reused by server and worker
- `src/worker/` - polling loop and manifest watcher
- `src/commands/` - CLI entrypoints for `server`, `worker`, `seed`, and `import-manifest`

The frontend lives in `app/`.

## Development

### Prerequisites

- Rust stable
- Node.js 20+
- pnpm
- Docker / Docker Compose

### Setup

```bash
pnpm install
docker compose up -d
sqlx migrate run
```

### Run

Backend server:

```bash
cargo run --bin ak-asset-storage -- server -c config.toml
```

Worker:

```bash
cargo run --bin ak-asset-storage -- worker -c config.toml
```

Frontend dev server:

```bash
pnpm dev
```

## Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-features -- -D warnings
cargo test --all-features -- --nocapture
pnpm typecheck
pnpm lint
```

## Configuration

Configuration is TOML-based. See `example.toml` for a complete example.

Main sections:

- `logger`
- `server`
- `database`
- `mailer`
- `ak`
- `s3`
- `sentry`
- `torappu`

### Server

```toml
[server]
binding = "localhost"
port = 5150
host = "http://localhost"
```

### Database

```toml
[database]
uri = "postgres://user:password@localhost:5432/dbname"
max_connections = 10
connection_timeout_seconds = 30
```

### AK

```toml
[ak]
asset_url = "https://ak.hycdn.cn/assetbundle/official/Android/assets"
conf_url = "https://ak-conf.hypergryph.com/config/prod/official/Android"
```

### S3

```toml
[s3]
endpoint = "http://127.0.0.1:9000"
bucket_name = "bucket-name"
access_key_id = "access-key"
secret_access_key = "secret-key"
with_virtual_hosted_style_request = false
```

### Torappu

```toml
[torappu]
token = "your-torappu-token-here"
asset_base_path = "/assets"

[torappu.docker]
image_url = "your-docker-image:latest"
container_name = "ak-asset-container"
env_vars = [ "TZ=Asia/Shanghai" ]
volume_mapping = [ "./data:/app/data" ]
docker_host = "/var/run/docker.sock"
network = "boot_default"

[torappu.github]
owner = "your-username"
repo = "your-repo"
workflow_id = "workflow-file.yml"
ref = "main"
token = "github-token"
```

## Database Notes

- Migrations live in `migrations/`
- Do not edit `.sqlx/`
- Do not use `SQLX_OFFLINE=true` for local `cargo check` / `cargo build`
- If sqlx cannot connect to the database, fix the database first instead of falling back to offline mode
