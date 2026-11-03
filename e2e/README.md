This directory contains fixtures and runtime files for the manual end-to-end test flow.

- `fixtures/` stores checked-in upstream samples served by the fake AK API.
- `runtime/` is generated locally while running the E2E test and is gitignored.

`tests/http_e2e/` covers the real server binary over HTTP: bundle pagination,
literal filters and invalid cursors, MCP initialization and errors, paginated
file listings with working download URLs, and gamedata readiness filtering.
These tests run in CI and can also be run locally:

```sh
pnpm build
# DATABASE_URL must point to a development PostgreSQL instance. Its user
# needs CREATEDB: sqlx creates and migrates a separate database per test.
cargo test --locked --all-features --test http_e2e -- --ignored --nocapture
```

Install `plocate` (including `updatedb`) and export `DATABASE_URL` first, as for
the development setup. Each test starts its own server on a loopback port, uses
its own temporary asset directory, and shuts the server down before database
cleanup. The HTTP suite can run in parallel and needs no k3s, RustFS, or upstream
API. The full ingestion/worker suite remains in `tests/e2e/` and still runs
serially against the local k3s dependencies.
