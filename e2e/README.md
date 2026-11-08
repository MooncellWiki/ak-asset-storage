This directory contains fixtures and runtime files for the end-to-end tests in
`tests/e2e/`. The suite runs real server/worker/CLI processes against PostgreSQL
and RustFS. AK responses and Kubernetes Job creation use in-process fake APIs;
no game server credentials or real extraction Jobs are needed.

- `fixtures/` stores checked-in upstream samples served by the fake AK API.
- `runtime/` is generated locally while running the E2E test and is gitignored.

## Running locally

Build the frontend before compiling Rust (`pnpm build`). Install `rustfs-cli`
(provides `rc`) and `plocate` (provides `plocate` and `updatedb`).

The default dependency mode is `k3s`: the harness uses `kubectl` and `just` to
apply the dev PostgreSQL deployment and generated RustFS manifest. Run
`just init-env` once to prepare the RustFS hostPath directory and kubeconfig.
Only a local Kubernetes API endpoint is accepted.

```bash
sqlx migrate run
cargo test --locked --all-features --test e2e -- --ignored --nocapture --test-threads=1
```

Keep `--test-threads=1`: every test reuses the same ports and runtime directory,
and resets the `ak_asset_storage_e2e` database and `ak-asset-storage-e2e` bucket.
The development database and bucket are separate and are not reset.

## Using existing services / CI

Set `AK_E2E_DEPENDENCIES=external` to skip all `kubectl`/`just` operations. This
mode still waits for services, recreates the test database, applies migrations,
and recreates the test bucket. It expects these fixed local endpoints:

| Dependency | Endpoint                 | Credentials                                                            |
| ---------- | ------------------------ | ---------------------------------------------------------------------- |
| PostgreSQL | `localhost:32432`        | `ak` / `ak` (can create/drop databases and terminate test connections) |
| RustFS     | `http://127.0.0.1:31000` | `torappu` / `torappu123` (path-style S3)                               |

```bash
AK_E2E_DEPENDENCIES=external cargo test --locked --all-features --test e2e -- \
  --ignored --nocapture --test-threads=1
```

The `e2e` job in `.github/workflows/ci.yaml` starts isolated service containers,
downloads the frontend build, and installs the test tools. It pins the RustFS
image by version/digest and verifies the pinned CLI download's SHA-256.
The job migrates `ak_asset_storage_next` before compilation, sets `DATABASE_URL`
to that database and `SQLX_OFFLINE=false`, then runs the suite serially. The
harness resets only `ak_asset_storage_e2e`, leaving compile-time checks isolated
from test resets. Renovate updates the RustFS image in both the workflow and
`just gen-rustfs`, so local runs use the same version; bump the CLI pin by hand.

Test output and the service container logs (printed by the runner's
"Stop containers" step) are all in the job log. Tests kill and reap the
server/worker processes before the next test reuses their ports, and
`kill_on_drop` stops them when a test panics.

Spawned workers rescan watched files every second with a two-second debounce
(`--watch-interval-seconds` / `--watch-debounce-seconds`); the production
defaults (10 s / 30 s) would dominate the watcher tests' runtime.
