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

Secret fields can also be provided through environment variables; a non-empty
variable overrides the TOML value, so credentials can be injected (Docker /
Kubernetes secrets) without rewriting the config file:

| Config field           | Environment variable   |
| ---------------------- | ---------------------- |
| `database.uri`         | `DATABASE_URI`         |
| `s3.access_key_id`     | `S3_ACCESS_KEY_ID`     |
| `s3.secret_access_key` | `S3_SECRET_ACCESS_KEY` |
| `mailer.auth.password` | `MAILER_AUTH_PASSWORD` |
| `torappu.token`        | `TORAPPU_TOKEN`        |
| `torappu.github.token` | `TORAPPU_GITHUB_TOKEN` |

These fields may be omitted from TOML when their environment variables are set.
The optional mailer and GitHub integrations still require their non-secret TOML
configuration.

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

### MCP

Set `[mcp] enable = true` to expose the read-only query tools at `/mcp` (Streamable HTTP).

- `read_gamedata` reads one file of a version's extracted gamedata. JSON files are parsed:
  `keys_only` outlines the children at a JSON Pointer (key, pointer, type, size) and
  `pointer` returns one part, refused with a hint when it exceeds `max_bytes` (default
  20 000, at most 100 000). Other files are returned as UTF-8 text pages. Without a
  version it reads the newest version whose `.gamedata-ready.json` marker exists.
  `gamedata_read_concurrency` (default 2) caps parallel reads; excess calls fail fast.
- `list_versions` / `get_version` report `gamedataStatus`: `ready` (marker present),
  `unmarked` (directory without marker: still extracting, or extracted before markers
  existed) or `missing`.
- `list_files` / `search_files` file entries carry a public `url` built from
  `server.host` (`/assets/...` for `raw/`, `/gamedata/...` for `gamedata/`).
  `search_files` takes `path_prefix` to search one subtree; `gamedata/latest/` resolves
  to the version the alias links to.

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
# The Job itself uses the separate ak-asset-extractor account without an API
# token, disables privilege escalation, drops all capabilities and uses
# RuntimeDefault seccomp. RBAC alone cannot constrain Job contents: install
# deployment-specific admission rules as explained in deploy/k3s/rbac.yaml.
# Jobs require a non-root image USER; upgrade Torappu and prepare volume
# ownership before deploying this launcher (see DEVELOPMENT.md).
[torappu.kubernetes]
# Image pulls and retries are handled by kubelet. Configure any registry
# credentials on the Kubernetes nodes.
image_url = "your-registry/your-image:latest"
namespace = "ak-asset-storage"
# Fixed Job name; doubles as the single-flight lock, a launch is rejected
# while a Job with this name is still running.
job_name = "ak-asset-job"
service_account_name = "ak-asset-extractor"
# Optional UID/GID overrides for writable output volumes; otherwise the
# image's USER applies (Torappu: 10001:10001). Both must be nonzero if set.
# run_as_user = 10001
# run_as_group = 10001
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

`deploy/k3s/rbac.yaml` is a standalone deployment example; it is not included
by `deploy/k3s/dev/kustomization.yaml`. Adapt the Job name and namespace to
your configuration, and install admission rules for the permitted images,
volume paths and security context in your cluster.

Create `service_account_name` in the configured namespace before launching
Jobs, without a RoleBinding and with token automount disabled. A missing
account leaves the Job with `FailedCreate` events and blocks subsequent
launches until its deadline (6 hours by default); inspect `kubectl describe
job` when a launch reports `already running` without a Pod.
