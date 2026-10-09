# Arknights Asset Storage

AK asset monitoring, storage, and torappu orchestration service.

## Rust Layout

Rust backend is a single crate organized by module:

- `src/api/` - Axum HTTP handlers, router, and API request/response types
- `src/database/` - PostgreSQL-only SQLx access behind `Database { pool: PgPool }`
- `src/external/` - concrete integrations for AK API, S3, SMTP, Kubernetes Jobs, GitHub, and torappu assets
- `src/service/` - shared workflows reused by server and worker
- `src/worker/` - polling loop and manifest watcher
- `src/commands/` - CLI entrypoints for `server`, `worker`, `seed`, and `import-manifest`

The frontend lives in `app/`.

## Development

### Prerequisites

- Rust stable
- Node.js 20+
- pnpm
- A local single-node k3s (`kubectl` on PATH; dev manifests bind NodePorts to 127.0.0.1)

### Setup

```bash
pnpm install
kubectl apply -k deploy/k3s/dev
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

# Optional: launch the asset-extraction image as a Kubernetes Job on new
# version detection. Cluster credentials resolve like kube-rs Config::infer
# ($KUBECONFIG / ~/.kube/config first, then the in-cluster service account,
# so keep kubeconfigs out of the server/worker pod); apply
# deploy/k3s/rbac.yaml and set `serviceAccountName: ak-asset-storage` on the
# server/worker pod when running inside k3s.
[torappu.kubernetes]
image_url = "your-registry/your-image:latest"
namespace = "ak-asset-storage"
# Fixed Job name; doubles as the single-flight lock, a launch is rejected
# while a Job with this name is still running.
job_name = "ak-asset-job"
# imagePullSecret for private registries (pulls and retries are kubelet's job)
image_pull_secret = "ak-registry-cred"
env_vars = [ "TZ=Asia/Shanghai" ]
# Fails a Job that has not finished in time (default 6h), including one stuck
# pulling its image, so it cannot block later launches forever.
# active_deadline_seconds = 21600

# One block per mounted volume; set exactly one of pvc / host_path.
[[torappu.kubernetes.volume_mounts]]
pvc = "ak-asset-data"
mount_path = "/app/data"

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
