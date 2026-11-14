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
- `src/commands/` - CLI entrypoints for `server`, `worker`, `seed`, and manifest imports

The frontend lives in `app/` (Vue 3 + TypeScript + [PRTS Design](https://mooncellwiki.github.io/prts-design/), file-based routing).

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
- `launcher` (optional PC official-launcher archive)
- `s3`
- `sentry`
- `mcp`
- `metrics`
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

### PC launcher archive

PC tracking is opt-in. Add the following section to the worker's TOML config
after applying the database migrations:

```toml
[launcher]
poll_interval_seconds = 300
concurrency = 4
retry_interval_seconds = 60
max_retries = 3
download_timeout_seconds = 900
```

The defaults target the PC official channel. The worker polls the launcher's
`batch_proxy` API independently of Android, checks the encrypted `game_files`
MD5, decrypts AES-256-CBC/PKCS7 JSONL, and atomically saves the publication and
its entire manifest. A publication is identified by package version and
manifest MD5, so a same-version reissue is retained. No PC publication starts
a Torappu extraction Job.

Each `(MD5, size)` content pair is downloaded once across versions and paths.
Downloads stream into anonymous temporary files while checking size and MD5,
then upload raw bytes to `launcher/blobs/<first 2>/<next 2>/<remaining 60>`
in the existing bucket, using the file's hex SHA-256.
These records are separate from Android's ZIP-backed content hashes.
Allow enough temporary disk for concurrent downloads of the largest file;
`TMPDIR` selects that filesystem. Set a bucket lifecycle rule to expire
incomplete multipart uploads left by process termination.

Discovery continues while archival is running. A worker claims a content pair
with a short lease instead of an open transaction, so no database connection
is held during transfers; concurrent workers skip leased content, and a claim
interrupted by a restart is retried once its lease (twice
`download_timeout_seconds`) expires. Failures are recorded per content pair
and retried with exponential backoff: `retry_interval_seconds` after the first
failure, doubling each time (1, 2 and 4 minutes by default, never more than one
day). After `max_retries` retries the worker stops requesting that content.
On startup it requeues exhausted content with a fresh retry budget. A newly
imported publication also requeues content it lists that is in backoff or
has exhausted its retries. An in-flight retry then loses its lease, so the
worker discards its failure. If a source is unavailable or fails verification,
the worker tries the other known paths for that same content, newest package
version first, whatever order the manifests were imported in.
Successful objects remain available across restarts. A publication becomes
ready only when every entry references a verified, uploaded object.

#### Import historical manifests

After applying migrations, pass one or more launcher metadata files in import
order.

```sh
ak-asset-storage import-launcher-manifest -c config.toml \
  76.0.0.meta.json 77.0.0.meta.json 78.0.0.meta.json
```

The command accepts the `.meta.json` format written by
`launcher-rev/scripts/fetch_manifest.py`. It reads `version`,
`client_version`, `file_path` (the release CDN base), and `game_files_md5`.
By default it fetches `<file_path>/game_files`. To use saved ciphertext, add
`"game_files_path": "game_files.encrypted"` to the metadata. Relative paths
are resolved from the metadata file's directory.

Each release is verified and committed separately. If an import fails, correct
the input and rerun the same command. Previously imported releases are skipped.
The command uses the configured launcher AES settings or the official defaults.
The worker with `[launcher]` enabled archives missing file contents in the
background.

The **PC 归档** page at `/launcher` shows file and byte progress, searches historical
paths, compares publications, and downloads finished files even while other
files are pending. Renames appear as a deleted path and an added path sharing
the same content. Public read-only API endpoints:

| Endpoint                                      | Result                                              |
| --------------------------------------------- | --------------------------------------------------- |
| `GET /api/v1/launcher/releases`               | Releases, newest version first; `before`, `limit`   |
| `GET /api/v1/launcher/releases/{id}`          | Release metadata and progress                       |
| `GET /api/v1/launcher/releases/{id}/files`    | Full manifest, paginated with `after`, `q`, `limit` |
| `GET /api/v1/launcher/diff?from=ID&to=ID`     | Added/deleted/modified paths; `after`, `limit`      |
| `GET /api/v1/launcher/releases/{id}/manifest` | Original encrypted `game_files`                     |

Page sizes default to 100 and are limited to 200. Use `nextBefore` or
`nextAfter` as the next request's `before` or `after` with the same filters.
The page downloads sharded objects through the same `/storage/` route as
Android bundles. Route `/storage/launcher/blobs/...` to `launcher/blobs/...`
in the archive bucket. The original encrypted `game_files` is stored in the
PostgreSQL `launcher_releases.encrypted_manifest` column. API schemas are
included in OpenAPI.

Protocol parameters were cross-checked against the
[official-channel request](https://github.com/Well2333/ArkSwitcher/blob/main/src/official_source.rs)
and the live official API/manifest. The upstream response's `version` is the
package version; `client_version` is the game content version. Endpoint,
request JSON, and hex AES key/IV can be overridden in the launcher section.

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

- `list_versions` / `get_version` report `gamedataStatus`: `ready` (marker present),
  `unmarked` (directory without marker: still extracting, or extracted before markers
  existed) or `missing`.
- `list_files` / `search_files` file entries carry a public `url` built from
  `server.host` (`/assets/...` for `raw/`, `/gamedata/...` for `gamedata/`).
  `search_files` takes `path_prefix` to search one subtree; `gamedata/latest/` resolves
  to the version the alias links to.
- MCP never returns file contents: clients download a file's `url` and extract what they
  need themselves (gamedata tables are several MB each).

### Prometheus metrics

Enable metrics separately from MCP in the server config.

```toml
[metrics]
enable = true
```

Prometheus scrapes `GET /metrics` on the existing server port. The endpoint
returns 404 by default. With metrics enabled and MCP disabled, the endpoint
returns a valid empty scrape. Restrict `/metrics` to Prometheus at the gateway
or network layer. When running multiple replicas, scrape each replica directly
so Prometheus can track each process's counters.

### Story resource explorer

`GET /api/v1/story-resources/explore?type=character&page=1&pageSize=60`
returns one page with `total`, `matchedTotal`, `counts` (all resource types),
`categoryCounts`, and the dataset's `updatedAt`. It does not require the client
to collect the entire resource list. The existing cursor-based list and usage
interfaces remain compatible.

- `q`: case-insensitive AND keywords, literal substring matching; also accepts
  pasted script commands, wiki filenames, and exact expression aliases. Up to
  2048 bytes; only the first 32 distinct keywords are used.
- `groupBy=base|resource` (default `base`): characters sharing a character.json
  base are grouped **before** counting, sorting and paging. Group `count` is
  the distinct union of scripts across all variants; each member keeps its own
  `scriptCount`. Any matching member includes the group, represented by the
  first matching member in natural order.
- `category=operator|npc|other`: filters characters only. Type counts ignore
  the selected type; category counts also ignore the selected category.
- `sort=id|scriptCount`, `order=asc|desc`: natural ID sorting compares numeric
  runs by value and orders `_` < `#` < `$` < digits < letters, with the original
  ID as the deterministic tie-breaker. Script count ties use ascending natural
  ID order.
- `page` is 1-based and clamps to the last page; `pageSize` is 1–500 (default 60).
- `focus`: an exact listing ID for a shared detail. Returns `selection` and
  locates its page if it matches the filters; otherwise the detail remains
  available separately. `focusIndex` is zero-based, or null when not matched.

Responses carry `Cache-Control: private, max-age=60`, so browsers may reuse them
briefly while shared caches never store them.

`story_resource_catalog` is derived data: the migration backfills it from the
current usages, and subsequent story imports rebuild it in the **same
transaction** as usages and `story_dataset.updated_at`. Re-imports also include
unused face aliases from character.json; the migration alone can recover only
aliases already in the usage snapshot. Deploy this backend and migration before
switching prts-widgets to the paginated interface.

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
