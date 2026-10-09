# Arknights Asset Storage

AK asset monitoring, storage, and torappu orchestration service.

## Documentation

- [CONTRIBUTING.md](CONTRIBUTING.md) — development setup, daily workflow, verification, and code style
- [DEVELOPMENT.md](DEVELOPMENT.md) — the local dev environment: k3s dependencies, generated config and manifests, the extraction Job, dev/e2e isolation
- `example.toml` — complete annotated configuration file

## Rust Layout

Rust backend is a single crate organized by module:

- `src/api/` - Axum HTTP handlers, router, and API request/response types
- `src/database/` - PostgreSQL-only SQLx access behind `Database { pool: PgPool }`
- `src/external/` - concrete integrations for AK API, S3, SMTP, Kubernetes Jobs, GitHub, and torappu assets
- `src/service/` - shared workflows reused by server and worker
- `src/worker/` - polling loop and manifest watcher
- `src/commands/` - CLI entrypoints for `server`, `worker`, `seed`, and `import-manifest`

The frontend lives in `app/` (Vue 3 + TypeScript + Naive UI, file-based routing).

## Configuration

Configuration is TOML-based. See `example.toml` for a complete example.

Main sections:

- `logger`
- `server`
- `mailer`
- `database`
- `ak`
- `s3`
- `sentry`
- `mcp`
- `torappu` (with `plocate`, `kubernetes`, `github` subsections)

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
uri = "postgres://ak:ak@localhost:32432/ak_asset_storage_next"
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
endpoint = "http://127.0.0.1:31000"
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
