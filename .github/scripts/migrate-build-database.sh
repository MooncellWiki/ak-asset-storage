#!/usr/bin/env bash
# Only for a fresh, disposable build database. Deployed databases use SQLx's
# migration runner (and its migration history) instead.
set -euo pipefail
for migration in migrations/*.up.sql; do
  psql "$DATABASE_URL" -X -v ON_ERROR_STOP=1 -f "$migration"
done
